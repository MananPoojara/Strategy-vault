//! YT strategies "BTST" (not from a book): the team's AmiBroker AFLs in `BTST/BTST_Intra day/`, main variant "Up-close"
//! (engine E1 of `BTST/BTST_AFL_Variant_Analysis.md`), built to `BTST/BTST_PHASE1_SPEC.md`.
//!
//! Data source parameter `src`, one family per run (never mixed):
//! * 1 = the team's IEOD_FO AmiBroker database, series family `BTST_5M`: start-stamped 5-minute bars `O/H/L/C/V<HHMM>`
//!   for every bar column present (0900 … 1930: day sessions incl. after-hours and the Muhurat evening sessions), and the day's own OHLC and volume `DO DH DL DC DV` compressed from ALL
//!   minutes of the day (after-hours and evening sessions included, as AmiBroker compresses them).
//! * 2 = the rolling front-month futures feed, family `BTST_F5M`: bars 0915 … 1525 (the lone 1530 print of that feed is
//!   stray and never read), no volume; daily bars are built from those bars (open = first open, high = highest high,
//!   low = lowest low, close = last close).
//!
//! The selected ticker only provides the trading calendar and the row indices of the trades; no cash price is read.
//! Conventions: long only, one position at a time, signals read bar by bar with data <= that bar. A day exists when its
//! daily close exists; a calendar day without data is skipped ("the previous day" is the previous day with data). A bar
//! is used only when its open, high, low and close are all present. At the entry bar of day D only days <= D-1 and the
//! bars of day D up to the entry bar are used (AFL `TimeFrameExpand`, expandLast). Weekly bars are calendar weeks
//! (Monday to Sunday) and monthly bars calendar months; only completed periods are visible. Entry at the close of the
//! bar stamped exactly at the entry time (AFL `timenow == entrytime`). Exits are checked on every bar after the entry bar
//! and, as the AFL allows same-bar exits, on the entry bar itself: profit booking first (fills at the target, or at the
//! open less 0.25% when the open gaps above it), then the hard stop (open less 0.25% when the open gaps below it, else
//! the stop), then the time exit at the close of the first bar of a later trading day from the exit time and before the
//! entry time (so a 09:55 session open before 2010 exits at 09:55).
#![allow(dead_code)]
use crate::books::ytmr::day_number;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_btst";
/// The default sub-variant (the team's verified Nifty list settings).
pub const ID: &str = "ytbtst_core";
pub const GROUP: &str = "YT Strategies · BTST · Up-close";
/// Coverage series of the two data sources (src 1, src 2); `/api/tickers` reports both.
pub const COVERAGE_SERIES: [&str; 2] = ["BTST_5M.DC", "BTST_F5M.C1525"];
const STOCK_ONLY: &str = "needs the F&O stock universe (Phase 2)";
const AFL_DIR: &str = "BTST/BTST_Intra day/";

const NOTES: &str = "YT strategy, not a book method; main variant Up-close (the team's AFL engine E1). \
Data source src: 1 = the team's IEOD_FO AmiBroker database (default; series BTST_5M: start-stamped 5-minute futures bars 0900 to 1930 with volume (after-hours and Muhurat evening-session bars are ordinary bars), and the day's OHLC and volume DO/DH/DL/DC/DV compressed from all its minutes incl. after-hours and evening sessions; NIFTY 2007-01-02 to 2023-02-07, BANKNIFTY 2007-05-29 to 2023-02-07); \
2 = the rolling front-month futures feed (series BTST_F5M, NIFTY only, 2019-02-11 to 2026-09-29: bars 0915 to 1525 are read, the stray 1530 print never; daily bars are built from those bars; no volume). One source per run, never mixed. \
The selected ticker only provides the trading calendar, so results are futures prices on cash-calendar days. Padded minutes are used exactly as stored (AmiBroker behaviour). \
A day exists when its daily close exists; a day without data is skipped. A bar with any of open, high, low or close missing is skipped; nothing is estimated. \
At the entry bar of day D only completed days (D-1 and earlier) and day D's bars up to the entry bar are used (AFL TimeFrameExpand, expandLast). \
Entry at the close P of the 5-minute bar stamped at the entry time. Exit, first event in time order: the bar's high is above entry x (1 + profit booking %), filled there or at the open less 0.25% on a gap above it (checked from the entry bar itself, AFL AllowSameBarExit); the bar's low is below entry x (1 - hard stop %), filled there or at the open less 0.25% on a gap below it (explicit stop price as in the '& Hard SL' AFL); \
or the close of the first bar of a later trading day from the exit time and before the entry time (a 09:55 session open before 2010 exits at 09:55). Profit booking 0 or hard stop 0 switch that exit off. \
Trend filter (Supertrend, SMA or off) on the last completed Daily, Weekly (Monday to Sunday) or Monthly bar; Supertrend: ATR = simple average of the 1-bar true range over the period, price mid point, close or pivot, bands ratcheted and flipped as in the AFL loop; SMA: the last completed close above the SMA of the bar before it. \
Deviations from the AFL: the Supertrend period starts at 1 (0 divides by zero); entry and exit times step by 5 minutes on the bar grid (other values give no trades); the AFL's pb level follows the latest raw buy signal even while a position is open, here the position's own entry. \
Penny Stock Price (penny): a buy needs the entry close above it (AFL buyflt C > pennyflt; 0 = off, the AFL default). Dropped from the AFL: ranking and position score (one instrument), turnover, circuit and dead-stock switches, commission and sizing (the engine applies its own costs). Results are simulated; no published figures exist for this strategy.";

const NOTES_E34: &str = "YT strategy, not a book method. Data source src: 1 = the team's IEOD_FO AmiBroker database (default; series BTST_5M, start-stamped 5-minute futures bars 0900 to 1930 incl. after-hours and Muhurat evening-session bars, daily bars from DO/DH/DL/DC compressed from all minutes; NIFTY 2007-01-02 to 2023-02-07, BANKNIFTY 2007-05-29 to 2023-02-07); 2 = the rolling futures feed (series BTST_F5M, NIFTY only, 2019-02-11 to 2026-09-29; bars 0915 to 1525 are candles, the stray 1530 print never; daily bars from those bars). One source per run. \
The candles of a day are its present bars in time order (padded minutes as stored); a day without data does not exist; the selected ticker only provides the calendar. tf 1 = 5-minute candles, 2 = 15-minute candles built from the 5-minute bars. \
A higher-timeframe value seen at a candle (AFL TimeFrameExpand, expandLast) is that of the latest daily, weekly (Monday to Sunday) or monthly period whose last candle is at or before it: inside a day the previous completed day, on the day's last candle that day itself. \
Supertrend = the AFL loop (simple-average ATR(1) over the period, bands ratcheted and re-seeded on a flip) on daily bars, compressed to weeks or months. Prices at fills: candle closes, targets as stated; costs are the engine's. Results are simulated; the team's own runs give counts and trade lists, no published figures.";

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Core,
    IbsRoc,
    Ath,
    AthOnly,
    HighN,
    Green,
    Volume,
    Athc,
    OldFixed,
    /// Stock-only AFL kept as a catalogue entry only (index filter on NSE500/NIFTY_F1): NotTestable.
    Stock,
    /// Universe sub-variants (F&O stock universe, cross-sectional portfolio): the up-close core plus a PositionScore or a
    /// NIFTY condition. Not runnable on a single instrument.
    RankPrice,
    RankIbs,
    RankLookback,
    RankRoc,
    RankIbsVolume,
    NiftyUp,
    NiftyInc,
    /// E3 "Second-last candle": no filter, monthly-Supertrend bull filter, bear filter.
    Slc,
    SlcBull,
    SlcBear,
    /// E4 "Candle-count" entry and exit.
    Candles,
}

impl Kind {
    fn e34(self) -> bool {
        matches!(self, Kind::Slc | Kind::SlcBull | Kind::SlcBear | Kind::Candles)
    }
    /// Runs only on the stock universe.
    fn universe_only(self) -> bool {
        matches!(self, Kind::RankPrice | Kind::RankIbs | Kind::RankLookback | Kind::RankRoc | Kind::RankIbsVolume | Kind::NiftyUp | Kind::NiftyInc)
    }
}

/// Status reason of the universe sub-variants on a single instrument.
pub const UNIVERSE_ONLY: &str = "runs on the F&O stock universe (universe run), not on a single instrument";
/// The universe name of the universe runs (AmiBroker watchlist).
pub const UNIVERSE: &str = "FO STOCKS C&P";

pub const GROUP_SLC: &str = "YT Strategies · BTST · Second-last candle";
pub const GROUP_CANDLES: &str = "YT Strategies · BTST · Candle-count";

fn group(kind: Kind) -> &'static str {
    match kind {
        Kind::Slc | Kind::SlcBull | Kind::SlcBear => GROUP_SLC,
        Kind::Candles => GROUP_CANDLES,
        _ => GROUP,
    }
}

/// One sub-variant: id, label, rule kind, the AFL it reads (relative to `BTST/BTST_Intra day/`, or the BTST root) and
/// what it adds or changes.
struct Variant {
    id: &'static str,
    label: &'static str,
    kind: Kind,
    afl: &'static str,
    note: &'static str,
}

const VARIANTS: [Variant; 21] = [
    Variant { id: ID, label: "Up-close core", kind: Kind::Core,
        afl: "BTST after 3 consecutive upclose_with profit booking/btst trad in intraday with flt and profit booking.afl (+ '& Hard SL.afl', the same rules with an explicit stop price)",
        note: "The last Lookback Days daily closes rose each day (2 = one rise) and the last Lookback Days - 2 daily changes each exceeded the one before (strict >); P is above the last completed close and P minus that close is larger than the last daily change. Defaults = the team's verified Nifty list: lookback 2, entry 14:50, exit 09:25, no profit booking, no stop, no filter." },
    Variant { id: "ytbtst_ibs_roc", label: "IBS and ROC thresholds", kind: Kind::IbsRoc,
        afl: "btst trad in intraday with flt and profit booking with IBS flt and ROC ranking .afl (BTST root, 2022-09-08) and 'BTST_with IBS + Threshold and ROC Ranking with Threshold/btst trad in intraday with flt and profit booking with IBS flt and ROC ranking.afl'",
        note: "Core rules plus IBS = (IH - P) / (IH - IL), IH and IL the highest high and lowest low of day D's bars from its first bar to the entry bar (IH = IL gives no trade), compared with the IBS threshold: ibs_dir 1 = IBS <= level (final AFL), 2 = IBS >= level (earlier AFL); and ROC = (P - base) / base x 100 >= the ROC threshold, base = Ref(TimeFrameExpand(Ref(C, -(ROC days - 2))), -5): the daily close ROC days - 2 days before the last completed day seen 5 bars before the entry bar (the AFL note: 3 ROC days is actually 2 days). Deviations: ROC days start at 2 (the AFL default 1 lies outside its own range 2..5 and reads a future close); the ROC threshold defaults to 1 (the AFL default 5 lies outside its own range 0.5..2) and its range starts at 0." },
    Variant { id: "ytbtst_ath", label: "All-time-high close", kind: Kind::Ath,
        afl: "BTST on ATH stocks with increamental close/btst trad in intraday with flt and profit booking of  incremental close and ath.afl",
        note: "Core rules plus P above the all-time-high daily close: HHV(C, BarCount) on the daily bars, expanded, i.e. the highest daily close of all loaded days up to the last completed day (the data starts 2007 for src 1, 2019 for src 2)." },
    Variant { id: "ytbtst_ath_only", label: "All-time-high only", kind: Kind::AthOnly,
        afl: "BTST on ATH stocks without increamental close/btst trad in intraday with flt and profit booking on ath stock.afl",
        note: "No up-close rules (no lookback): buy at the entry bar when P is above the highest daily close of all loaded days up to the last completed day, with the trend filter. Deviation: the AFL evaluates its filter array inside TimeFrameSet(inDaily), which misaligns an intraday array with daily bars; here the filter is read on the last completed bar as in the other AFLs (the default filter off is unaffected)." },
    Variant { id: "ytbtst_high_n", label: "Highest close of N days", kind: Kind::HighN,
        afl: "BTST with increamental up close & HIGHEST close of 'n' days/btst trad in intraday with flt and profit booking with highest clsoing of n days.afl",
        note: "Core rules plus P above HHV(C, N) of the daily closes, expanded: the highest close of the N completed days up to D-1 (strict >; fewer than N days gives no trade). Deviation: N defaults to 3 (the AFL default 2 lies below its own range 3..15, step 2)." },
    Variant { id: "ytbtst_green", label: "Green candle", kind: Kind::Green,
        afl: "BTST_with increamental up close + GREEN candle of prev 2 days/btst trad in intraday with flt and profit booking with green candle option.afl",
        note: "Core rules plus, with candle colour GREEN (1, the AFL default): P above the open of day D's first bar (today's intraday candle green) and the last Green candle days - 1 completed daily candles each closed above their open (strict); ANY (2) switches the condition off. Green candle days default 2 (one previous day), range 2..5 as the AFL's Optimize." },
    Variant { id: "ytbtst_volume", label: "Volume above average", kind: Kind::Volume,
        afl: "BTST_with increamental up close + Today's volume greater than 5 days avg/btst trad in intraday with flt and profit booking with volume condition.afl and …_WITH VOL. MULTI..afl",
        note: "Core rules plus today's volume so far > multiplier x MA(daily volume, N) of the N completed days up to D-1. Today's volume so far follows the AFL's avgvol loop: the 5-minute volumes are summed bar by bar and the sum is reset after every bar at or after the entry time, so at the entry bar it is the volume of day D's bars from its first bar up to and including the entry bar (plus earlier days' bars only when such a day had no bar at or after the entry time); padded minutes count, as stored. Daily volume = the day's DV (all minutes). Multiplier 1 = the plain AFL. src 1 only: the futures feed has no volume." },
    Variant { id: "ytbtst_athc", label: "ATHC swing filter", kind: Kind::Athc,
        afl: "BTST_with increamental up close_ATH-%swing as filter/btst trad in intraday with Profit booking AND ATHC FLT.afl",
        note: "Core rules with the Supertrend/SMA filter replaced by the AFL's ATHC-swing state on the last completed day: Hcl = running highest daily close, Up = Hcl, Dn = C - C x exit %; trend up when C > Up[-1], down when C < Dn[-1], bands ratcheted as in the AFL loop; BUYSIG turns on when C > Hcl[-1] (a new all-time-high close) and off when the trend is down. The AFL also computes SELLSIG (trend down) but never uses it: the sell line stays profit booking or the next-morning exit, so trades still end next morning. ATHC exit % default 10, range 5..25." },
    Variant { id: "ytbtst_old_fixed", label: "Old fixed 3-day AFL", kind: Kind::OldFixed,
        afl: "BTST_Intraday after 3 consecutive upclose_Without profit booking & SL/btst trad in intraday ver3_bar-count.afl",
        note: "ver3_bar-count.afl (ver2_updated with a candle-count entry and exit). Each day's candles are counted as the data has them, except a lone 15:30 candle (a 15:30 candle that is the day's last: the exchange's closing print). Buy at the close of the ENTRY CANDLE = the N-th counted candle back from the day's last (1 = last; default 3 = 15:15 on a 15:25 close) when P > C[-1] > C[-2] > C[-3] and P - C[-1] >= C[-1] - C[-2] >= C[-2] - C[-3], P = that candle's close, C[-n] = the close of the last counted candle of the n-th previous day (no lookahead); sell at the close of the EXIT CANDLE = the N-th counted candle of a later day (1 = first; default 2 = 09:20 on a 09:15 open, 09:05 on a 09:00 open, 10:00 on a 09:55 open). After-hours and Muhurat candles count. No profit booking, stop or filter." },
    Variant { id: "ytbtst_rank_price", label: "Price ranking", kind: Kind::RankPrice,
        afl: "BTST with increamental upclose_HIGH to LOW based ranking/ and BTST with increamental upclose_LOW to HIGH based ranking/",
        note: "Up-close core on every symbol; PositionScore = entry price (rank_dir 1, High to Low) or 1000000 - price (rank_dir 2, Low to High)." },
    Variant { id: "ytbtst_rank_ibs", label: "Ranking as per IBS value", kind: Kind::RankIbs,
        afl: "BTST_With increamental up close_Ranking as per IBS value (High to Low)/ and (Low to high)/",
        note: "Up-close core on every symbol; PositionScore = IBS of the day so far (rank_dir 1, High to Low) or 100 - IBS (rank_dir 2, Low to High); an undefined IBS (high = low) scores 0." },
    Variant { id: "ytbtst_rank_lookback", label: "Ranking by lookback days", kind: Kind::RankLookback,
        afl: "BTST with increamental up close with ranking VER 2 (Lower to higher lookback days)/ and VER 11 (Higher to lower lookback days)/",
        note: "Up-close core on every symbol; cond = how many of lookback days + 1 … + 4 also hold (same rising-close and increasing-change test); PositionScore = StaticVarGenerateRanks(cond, mode 1234) rank (ver 2: lower cond first) or 1000 - rank (ver 11: higher cond first). Equal cond: rank 1234 numbers ties in list order (alphabetical symbol order here), so ver 2 prefers the later symbol, ver 11 the earlier one. The team's own tie order among equal cond is a hidden per-symbol order that the rules do not determine (oracle finding); this documented rule is used." },
    Variant { id: "ytbtst_rank_roc", label: "ROC ranking", kind: Kind::RankRoc,
        afl: "BTST with increamental up close_ROC ranking/",
        note: "Up-close core on every symbol; PositionScore = 100 + ROC%, ROC against Ref(TimeFrameExpand(Ref(C, -(ROC days - 2))), -5) (5 bars before the entry bar); default ROC days 3 = a 2-day ROC as the team noted." },
    Variant { id: "ytbtst_rank_ibs_volume", label: "IBS ranking and volume multiplier", kind: Kind::RankIbsVolume,
        afl: "BTST_With increamental upclose+IBS ranking (High to Low)+Volume multiplier/",
        note: "Up-close core plus the volume condition (today's volume so far > multiplier x MA(daily volume, N), as in the Phase-1 volume sub-variant); PositionScore = IBS (High to Low)." },
    Variant { id: "ytbtst_nifty_upclose", label: "1 up-close of NIFTY", kind: Kind::NiftyUp,
        afl: "BTST with increamental upclose + 1 UPCLOSE of NIFTY/",
        note: "Up-close core plus: the NIFTY futures close at the entry bar (Foreign, aligned to the stock's bar timeline: the latest NIFTY bar at or before it) is above NIFTY's daily close of the stock's previous day (NIFTY compressed on the stock's timeline). No PositionScore: all signals tie, filled in ascending symbol order." },
    Variant { id: "ytbtst_nifty_incremental", label: "Up-close and incremental difference in NIFTY", kind: Kind::NiftyInc,
        afl: "BTST_With increamental upclose + Upclose & increamental difference in NIFTY/",
        note: "Up-close core plus the same rising-close / increasing-change test and entry-price test on NIFTY futures: NIFTY's own daily closes on the stock's days (the AFL sets the daily timeframe before SetForeign) and NIFTY's close at the stock's entry bar (the AFL's niftydays parameter is unused, the stock's lookback days apply). No PositionScore: ascending symbol order." },
    Variant { id: "ytbtst_index_filter", label: "INDEX filter", kind: Kind::Stock,
        afl: "BTST with INDEX filter/",
        note: "Supertrend or SMA on NSE500 or NIFTY_F1 (D/W/M) as an extra filter on a stock (needs NSE500 data and a stock universe)." },
    Variant { id: "ytbtst_slc", label: "Second-last candle", kind: Kind::Slc,
        afl: "BTST Intraday (Buy at 2nd last candle_Sell at next day opening)/BTST Intraday (Buy at 2nd last candle_Sell at next day opening).afl",
        note: "Every day: buy at the close of the day's second-last candle when flat (a day needs at least 2 candles) unless it is an up-circuit candle (open = close = high, compared at 2 decimals, AFL Prec); sell at the close of the first candle of the next day with data unless that candle is a low-circuit candle (open = close = low): then hold and try the first candle of the following day. The AFL's loop checks the buy before the sell, so when one candle is both a day's second-last and first candle (a 2-candle day) a buy there is cancelled by the same-bar sell (Buy[i] = 0): no trade. The title says 'sell at next day opening' but the code sells at the first candle's CLOSE; the Vault follows the code. Dead-stock, penny and turnover switches (defaults off) are not exposed." },
    Variant { id: "ytbtst_slc_bull", label: "Monthly bull filter", kind: Kind::SlcBull,
        afl: "BTST Intraday (Buy at 2nd last candle_Sell at next day opening)/BTST Intraday with monthly filter for Bull Market.afl",
        note: "The second-last-candle rules plus: the Supertrend trend (AFL loop on the filter timeframe, default monthly, period 5, factor 1, mid point) seen at the buy candle is up (+1) and its band is defined (AFL abc: TrendUp > 0 or TrendDown > 0, so no trade during the ATR warm-up). The value seen follows TimeFrameExpand (expandLast): the latest period whose last candle is at or before the candle; a second-last candle therefore sees the previous completed period. The AFL's TrendUp[i-1] = Null on a trend change touches a value that is already Null (no look-ahead). No trend exit." },
    Variant { id: "ytbtst_slc_bear", label: "Monthly bear filter", kind: Kind::SlcBear,
        afl: "BTST Intraday (Buy at 2nd last candle_Sell at next day opening)/BTST Intraday with monthly filter for Bear Market.afl",
        note: "Identical to the bull AFL except the trend must be down (-1)." },
    Variant { id: "ytbtst_candles", label: "Candle-count entry and exit", kind: Kind::Candles,
        afl: "BTST on the basis of no. of candles_Intraday FO/Candles based Entry Exit(BTST).afl",
        note: "Entry candle k (0 = the day's last candle, k = k candles before it): buy at its close when flat and the Supertrend trend seen there is up (filter on, as in the AFL; flt_type 3 switches it off, as in the team's WOF lists); target = close x (1 + PT%) when PT > 0. While holding, on every later candle (never on the buy candle): a counter adds 1 on each candle of a day after the buy day (it counts candles, not days); exit (1) high >= target: at the open less the selling buffer % if the open is above the target, else at the target; (2) else the trend seen is down: at the close; (3) else on a day after the buy day once the counter reaches the exit candle: at the close (exit candle 1 = the next day's first candle). The trend seen follows TimeFrameExpand (expandLast): on a day's last candle a daily filter already shows that day's own trend, so a trend exit can happen on the buy day's last candle; during the ATR warm-up the AFL loop's trend is up (Close > Null), and before the first period is complete AmiBroker shows the first period's own value (the team's lists buy on the first day of data with the filter on). Any fill price outside the candle's low..high is moved to the low or high, as AmiBroker's backtester does. Team list BANKNIFTY 15-min weekly 20,3 En 2 Ex 1: its exits imply PT 7% with buffer 0.25% (8550 -> 9148.5 = x 1.07). tf 2 builds 15-minute candles from the 5-minute bars (clock buckets :00/:15/:30/:45; open first, high max, low min, close last present bar)." },
];

fn variant(id: &str) -> Option<&'static Variant> {
    VARIANTS.iter().find(|v| v.id == id)
}

/// True for every sub-variant id of this family (placeholders included).
pub fn is_variant(id: &str) -> bool {
    variant(id).is_some()
}

/// True for the stock-only placeholders (NotTestable until the F&O stock universe exists).
pub fn is_placeholder(id: &str) -> bool {
    variant(id).is_some_and(|v| v.kind == Kind::Stock)
}

/// True for the universe sub-variants (rankings, NIFTY conditions): not runnable on a single instrument.
pub fn is_universe_only(id: &str) -> bool {
    variant(id).is_some_and(|v| v.kind.universe_only())
}

/// True for the sub-variants a universe run accepts: every Up-close sub-variant and the universe ones (not E3/E4, not the
/// held index filter).
pub fn universe_runnable(id: &str) -> bool {
    variant(id).is_some_and(|v| v.kind != Kind::Stock && !v.kind.e34())
}

/// The data source of a parameter set: 1 = IEOD (default), 2 = futures feed; anything else is returned as is.
pub fn src_of(p: &Params) -> i64 {
    p.get("src").map_or(1, |v| if v.is_finite() { v.round() as i64 } else { 0 })
}

/// Series family of a data source.
pub fn family(src: i64) -> &'static str {
    if src == 2 { "BTST_F5M" } else { "BTST_5M" }
}

/// The coverage series the run reads for a parameter set (readiness, blank-period default window).
pub fn required_series(p: &Params) -> &'static [&'static str] {
    if src_of(p) == 2 { &COVERAGE_SERIES[1..] } else { &COVERAGE_SERIES[..1] }
}

/// Parameter combinations this family cannot run, as (readiness status, note).
pub fn unsupported(id: &str, p: &Params) -> Option<(&'static str, String)> {
    let v = variant(id).filter(|v| v.kind != Kind::Stock && !v.kind.universe_only())?;
    let src = src_of(p);
    if !(1..=3).contains(&src) {
        return Some(("not_testable", "Data source src must be 1 (IEOD as stored), 2 (the 2019-2026 futures feed) or 3 (IEOD as the team's 2021 copy).".into()));
    }
    if v.kind.e34() && !matches!(p.get("tf").map_or(1.0, |t| t.round()), t if t == 1.0 || t == 2.0) {
        return Some(("not_testable", "Candle timeframe tf must be 1 (5-minute) or 2 (15-minute).".into()));
    }
    if v.kind == Kind::Volume && src == 2 {
        return Some(("needs_data", "The volume condition needs 5-minute and daily volume; the futures feed (src 2) carries no volume. Choose src 1 (the team's IEOD database).".into()));
    }
    None
}

/// Start-stamped 5-minute bar times (HHMM) the rules read for a data source: every bar column of the family that is
/// loaded (IEOD 0900 … 1930), for the feed only 0915 … 1525 (its 1530 print is stray).
pub fn bar_times(x: &Ctx, src: i64) -> Vec<u32> {
    let fam = family(src);
    (0..24u32)
        .flat_map(|h| (0..60u32).step_by(5).map(move |m| h * 100 + m))
        .filter(|&t| src != 2 || (915..=1525).contains(&t))
        .filter(|t| ["O", "H", "L", "C"].iter().any(|k| x.series(&format!("{fam}.{k}{t:04}")).is_some()))
        .collect()
}

/// Shared notes plus the sub-variant's source AFL and rule note, built once so the catalogue can hand out `&'static str`.
fn variant_notes(i: usize) -> &'static str {
    static NOTES_ALL: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    NOTES_ALL.get_or_init(|| {
        VARIANTS
            .iter()
            .map(|v| {
                let s = if v.kind.e34() {
                    format!("{NOTES_E34} Sub-variant: {} Source AFL: {AFL_DIR}{}.", v.note, v.afl)
                } else if v.kind.universe_only() {
                    format!("{NOTES} Universe sub-variant ({UNIVERSE}, {UNIVERSE_ONLY}): {} Portfolio (AmiBroker backtester as settled from the team lists): signals of all symbols at the entry bar ranked by PositionScore (higher first, equal scores in ascending symbol name), filled up to max_positions minus open positions; exits before entries on a bar; size = floor(pct% x previous-bar equity / price) whole shares (pct 0 = 100 / max_positions), equity compounding with open positions marked at the previous bar's close; when cash is short the last entries shrink to the shares cash allows (skipped below 1 share); a signal on a symbol's last data day is never entered; commission and slippage = the Vault's cost settings per side. Source AFL: {AFL_DIR}{}.", v.note, v.afl)
                } else if v.kind == Kind::Stock {
                    format!("YT strategy, not a book method. Stock-only AFL, not testable on an index: {STOCK_ONLY}. Source AFL: {AFL_DIR}{} {}", v.afl, v.note)
                } else {
                    let path = if v.kind == Kind::IbsRoc { format!("BTST/{}", v.afl) } else { format!("{AFL_DIR}{}", v.afl) };
                    format!("{NOTES} Sub-variant: {} Source AFL: {path}.", v.note)
                };
                &*Box::leak(s.into_boxed_str())
            })
            .collect()
    })[i]
}

// Rule lines, "AmiBroker style": `<sec>|[?<key><op><num>|]<text>` with sec F (conditions), E (entry), X (exit), an
// optional visibility guard (op `>`, `=`, `!=`) and placeholders `{key}`, `{key:t}` (HHMM as HH:MM) and
// `{key:opt:A/B/C}` (option by 1-based value) that the frontend fills with the current parameter values.
const R_RISE: &str = "F|Close(D−k) > Close(D−k−1) for k = 1 … {days}−1 (the last {days} daily closes rising)";
const R_INCR: &str = "F|?days>2|Close(D−k) − Close(D−k−1) > Close(D−k−1) − Close(D−k−2) for k = 1 … {days}−2 (each change larger than the one before)";
const R_ST: &str = "F|?flt_type=1|Supertrend({st_period}, {st_factor}, {st_point:opt:Mid/Close/Pivot}) on the last completed {flt_tf:opt:Daily/Weekly/Monthly} bar is up";
const R_SMA: &str = "F|?flt_type=2|Close > SMA({sma_period})[−1] on the last completed {flt_tf:opt:Daily/Weekly/Monthly} bar";
const R_OFF: &str = "F|?flt_type=3|No trend filter";
const R_P1: &str = "E|At {entry_time:t}: 5-min Close > Close(D−1)";
const R_P2: &str = "E|5-min Close − Close(D−1) > Close(D−1) − Close(D−2)";
const R_BUY: &str = "E|Buy on the {entry_time:t} bar close";
/// AFL `buyflt = C > pennyflt` (Penny Stock Price): the entry bar's close must be above it; 0 = off (any positive price).
const R_PENNY: &str = "E|?penny>0|5-min Close at {entry_time:t} > {penny} (penny stock filter)";
const R_PB: &str = "X|?pb_pct>0|Profit booking: High > Buy × (1 + {pb_pct}%); fill there, or at Open − 0.25% if Open gaps above";
const R_HS: &str = "X|?hardstop>0|Hard stop: Low < Buy × (1 − {hardstop}%); fill there, or at Open − 0.25% if Open gaps below";
const R_TIME: &str = "X|Sell on the close of the first bar from {exit_time:t} on the next trading day";
const R_U: &str = "F|Every symbol of the F&O stock universe (FO STOCKS C&P), as stored";
const R_SIZE: &str = "E|?pct=0|Size = floor(100 / {max_positions} % × previous-bar equity / price) shares";
const R_SIZE2: &str = "E|?pct>0|Size = floor({pct}% × previous-bar equity / price) shares";
const R_SLC_BUY: &str = "E|Buy on the close of the day's 2nd-last {tf:opt:5-min/15-min} candle";
const R_SLC_UC: &str = "E|No buy if Open = Close = High (up-circuit candle)";
const R_SLC_SELL: &str = "X|Sell on the close of the next trading day's first candle";
const R_SLC_LC: &str = "X|If that candle has Open = Close = Low (low-circuit): hold to the following day's first candle";

fn rules(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Core => &[R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2, R_PENNY, R_BUY, R_PB, R_HS, R_TIME],
        Kind::IbsRoc => &[R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2,
            "E|?ibs_dir=1|IBS = (High(D) − Close) / (High(D) − Low(D)) ≤ {ibs_thr} (day so far)",
            "E|?ibs_dir=2|IBS = (High(D) − Close) / (High(D) − Low(D)) ≥ {ibs_thr} (day so far)",
            "E|ROC = (Close − Close(D−{roc_days}+1)) / Close(D−{roc_days}+1) × 100 ≥ {roc_thr}%",
            R_PENNY, R_BUY, R_PB, R_HS, R_TIME],
        Kind::Ath => &[R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2, "E|5-min Close > HHV(daily Close, all completed days) (all-time-high close)", R_PENNY, R_BUY, R_PB, R_HS, R_TIME],
        Kind::AthOnly => &[R_ST, R_SMA, R_OFF, "E|At {entry_time:t}: 5-min Close > HHV(daily Close, all completed days) (all-time-high close)", R_PENNY, R_BUY, R_PB, R_HS, R_TIME],
        Kind::HighN => &[R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2, "E|5-min Close > HHV(Close, {hdays}) of the last {hdays} completed days", R_PENNY, R_BUY, R_PB, R_HS, R_TIME],
        Kind::Green => &[R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2,
            "E|?candle=1|5-min Close > today's first Open (today green)",
            "E|?candle=1|Close > Open on each of the last {green_days}−1 days",
            "E|?candle=2|Candle colour: any",
            R_PENNY, R_BUY, R_PB, R_HS, R_TIME],
        Kind::Volume => &[R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2, "E|Volume today up to {entry_time:t} > {vol_mult} × MA(daily Volume, {vol_days})", R_PENNY, R_BUY, R_PB, R_HS, R_TIME],
        Kind::Athc => &[R_RISE, R_INCR, "F|ATHC swing on: a new all-time-high Close not yet followed by Close < trailing stop (Close × (1 − {athc_pct}%), ratcheted up)", R_P1, R_P2, R_PENNY, R_BUY, R_PB, R_HS, R_TIME],
        Kind::OldFixed => &[
            "E|Entry candle = candle {entry_n} counted back from the day's last candle (a lone 15:30 record is not counted)",
            "E|Entry candle Close > Close(D−1) > Close(D−2) > Close(D−3)",
            "E|Close − Close(D−1) ≥ Close(D−1) − Close(D−2) ≥ Close(D−2) − Close(D−3)",
            "E|?penny>0|Entry candle Close > {penny} (penny stock filter)",
            "E|Buy on the entry candle close",
            "X|Sell on the close of candle {exit_n} counted from the market open of the next trading day",
        ],
        Kind::Stock => &["F|Stock-only rule (index condition across the F&O stock universe); not testable"],
        Kind::RankPrice => &[R_U, R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2, R_PENNY, R_BUY,
            "E|?rank_dir=1|PositionScore = entry price: highest first, top {max_positions}",
            "E|?rank_dir=2|PositionScore = 1000000 − entry price: lowest price first, top {max_positions}",
            R_SIZE, R_SIZE2, R_PB, R_HS, R_TIME],
        Kind::RankIbs => &[R_U, R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2, R_PENNY, R_BUY,
            "E|?rank_dir=1|PositionScore = IBS = (High(D) − Close) / (High(D) − Low(D)): highest first, top {max_positions}",
            "E|?rank_dir=2|PositionScore = 100 − IBS: lowest IBS first, top {max_positions}",
            R_SIZE, R_SIZE2, R_PB, R_HS, R_TIME],
        Kind::RankLookback => &[R_U, R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2, R_PENNY, R_BUY,
            "E|cond = number of lookbacks {days}+1 … {days}+4 that also hold",
            "E|?lb_version=2|PositionScore = rank of cond (ver 2): lowest cond first, top {max_positions}",
            "E|?lb_version=11|PositionScore = 1000 − rank of cond (ver 11): highest cond first, top {max_positions}",
            R_SIZE, R_SIZE2, R_PB, R_HS, R_TIME],
        Kind::RankRoc => &[R_U, R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2, R_PENNY, R_BUY,
            "E|PositionScore = 100 + ROC%, ROC = (Close − Close(D−{roc_days}+1)) / Close(D−{roc_days}+1) × 100: highest first, top {max_positions}",
            R_SIZE, R_SIZE2, R_PB, R_HS, R_TIME],
        Kind::RankIbsVolume => &[R_U, R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2,
            "E|Volume today up to {entry_time:t} > {vol_mult} × MA(daily Volume, {vol_days})", R_PENNY, R_BUY,
            "E|PositionScore = IBS: highest first, top {max_positions}",
            R_SIZE, R_SIZE2, R_PB, R_HS, R_TIME],
        Kind::NiftyUp => &[R_U, R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2,
            "E|NIFTY futures Close at {entry_time:t} > NIFTY Close(D−1)", R_PENNY, R_BUY,
            "E|No PositionScore: ascending symbol name, top {max_positions}",
            R_SIZE, R_SIZE2, R_PB, R_HS, R_TIME],
        Kind::NiftyInc => &[R_U, R_RISE, R_INCR, R_ST, R_SMA, R_OFF, R_P1, R_P2,
            "E|NIFTY futures: the last {days} daily closes rising, each change larger than the one before",
            "E|NIFTY Close at {entry_time:t} > NIFTY Close(D−1), and NIFTY Close − Close(D−1) > Close(D−1) − Close(D−2)", R_PENNY, R_BUY,
            "E|No PositionScore: ascending symbol name, top {max_positions}",
            R_SIZE, R_SIZE2, R_PB, R_HS, R_TIME],
        Kind::Slc => &[R_SLC_BUY, R_SLC_UC, R_SLC_SELL, R_SLC_LC],
        Kind::SlcBull => &["F|Supertrend({st_period}, {st_factor}, {st_point:opt:Mid/Close/Pivot}) on the last completed {flt_tf:opt:Daily/Weekly/Monthly} bar is up (band defined)", R_SLC_BUY, R_SLC_UC, R_SLC_SELL, R_SLC_LC],
        Kind::SlcBear => &["F|Supertrend({st_period}, {st_factor}, {st_point:opt:Mid/Close/Pivot}) on the last completed {flt_tf:opt:Daily/Weekly/Monthly} bar is down (band defined)", R_SLC_BUY, R_SLC_UC, R_SLC_SELL, R_SLC_LC],
        Kind::Candles => &[
            "F|?flt_type=1|Supertrend({st_period}, {st_factor}, {st_point:opt:Mid/Close/Pivot}) on {flt_tf:opt:Daily/Weekly/Monthly} bars is up",
            "F|?flt_type=3|No trend filter",
            "E|Buy on the close of {tf:opt:5-min/15-min} candle {entry_candle} counted back from the day's last (0 = last candle)",
            "X|?pt_pct>0|Profit target: High ≥ Buy × (1 + {pt_pct}%); fill there, or at Open × (1 − {buffer_pct}%) if Open gaps above",
            "X|?flt_type=1|Trend exit: Supertrend turns down, sell on the close",
            "X|Sell on the close of candle {exit_candle} after the buy day (1 = the next day's first candle)",
        ],
    }
}

pub fn catalogue() -> Vec<StrategyDef> {
    VARIANTS
        .iter()
        .enumerate()
        .map(|(i, v)| StrategyDef {
            id: v.id,
            name: v.label,
            group: group(v.kind),
            session: 0,
            dir: "Long",
            status: if v.kind == Kind::Stock { Status::NotTestable(STOCK_ONLY) } else if v.kind.universe_only() { Status::NotTestable(UNIVERSE_ONLY) } else { Status::Live },
            needs_volume: false,
            needs_vix: false,
            needs_series: &COVERAGE_SERIES[..1],
            book: BOOK,
            rules: rules(v.kind),
            notes: variant_notes(i),
            claim: None,
        })
        .collect()
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let v = variant(id)?;
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if v.kind == Kind::Stock {
        return Some(Vec::new());
    }
    let mut out = vec![p("src", "Data source (1 IEOD 2007-2023 as stored, 2 futures feed 2019-2026, 3 IEOD as the team's 2021 copy, to reproduce the reference sheets)", 1.0, 1.0, 3.0, 1.0)];
    if v.kind.universe_only() {
        out = vec![p("src", "Data source (1 IEOD as stored, 3 IEOD as the team's 2021 copy, to reproduce the reference sheets)", 1.0, 1.0, 3.0, 2.0)];
        out.extend(universe_params());
    }
    if v.kind.e34() {
        out.push(p("tf", "Candles (1 5-minute, 2 15-minute)", 1.0, 1.0, 2.0, 1.0));
        match v.kind {
            Kind::SlcBull | Kind::SlcBear => out.extend([
                p("flt_tf", "Filter timeframe (1 Daily, 2 Weekly, 3 Monthly)", 3.0, 1.0, 3.0, 1.0),
                p("st_point", "Supertrend price (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
                p("st_period", "Supertrend period", 5.0, 1.0, 20.0, 1.0),
                p("st_factor", "Supertrend factor", 1.0, 0.5, 20.0, 0.5),
            ]),
            Kind::Candles => out.extend([
                p("entry_candle", "Entry candle (0 = last candle of the day, k = k candles before it)", 0.0, 0.0, 6.0, 1.0),
                p("exit_candle", "Exit candle (1 = first candle of the next day)", 1.0, 1.0, 5.0, 1.0),
                p("pt_pct", "Profit target % (0 = off)", 5.0, 0.0, 7.0, 1.0),
                p("buffer_pct", "Selling buffer % (target gapped at the open)", 0.25, 0.0, 10.0, 0.05),
                p("flt_type", "Trend filter (1 Supertrend, 3 off)", 1.0, 1.0, 3.0, 2.0),
                p("flt_tf", "Filter timeframe (1 Daily, 2 Weekly, 3 Monthly)", 1.0, 1.0, 3.0, 1.0),
                p("st_point", "Supertrend price (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
                p("st_period", "Supertrend period", 5.0, 5.0, 25.0, 1.0),
                p("st_factor", "Supertrend factor", 4.0, 1.0, 100.0, 1.0),
            ]),
            _ => {}
        }
        out.extend(sizing_params());
        return Some(out);
    }
    if v.kind == Kind::OldFixed {
        // ver3_bar-count.afl: ENTRY CANDLE FROM CLOSE, EXIT CANDLE FROM OPEN
        out.extend([
            p("entry_n", "Entry candle from close (1 = the day's last candle)", 3.0, 1.0, 20.0, 1.0),
            p("exit_n", "Exit candle from open next day (1 = the first candle)", 2.0, 1.0, 20.0, 1.0),
            p("penny", "Penny stock price (buy only when the entry close is above it; 0 = off)", 0.0, 0.0, 100.0, 1.0),
        ]);
        out.extend(sizing_params());
        return Some(out);
    }
    if v.kind != Kind::AthOnly {
        out.push(p("days", "Lookback days", 2.0, 2.0, 10.0, 1.0));
    }
    out.extend([
        p("entry_time", "Entry time (HHMM, 5-minute grid)", 1450.0, 1400.0, 1520.0, 5.0),
        p("exit_time", "Exit time next day (HHMM, 5-minute grid)", 925.0, 915.0, 955.0, 5.0),
        p("pb_pct", "Profit booking % (0 = off)", 0.0, 0.0, 3.0, 0.5),
        p("hardstop", "Hard stop % (0 = off)", 0.0, 0.0, 50.0, 1.0),
        p("penny", "Penny stock price (buy only when the entry close is above it; 0 = off)", 0.0, 0.0, 100.0, 1.0),
    ]);
    if v.kind != Kind::Athc {
        out.extend([
            p("flt_type", "Trend filter (1 Supertrend, 2 SMA, 3 off)", 3.0, 1.0, 3.0, 1.0),
            p("flt_tf", "Filter timeframe (1 Daily, 2 Weekly, 3 Monthly)", 1.0, 1.0, 3.0, 1.0),
            p("st_point", "Supertrend price (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
            p("st_period", "Supertrend period", 20.0, 1.0, 20.0, 1.0),
            p("st_factor", "Supertrend factor", 1.0, 1.0, 4.0, 1.0),
            p("sma_period", "SMA period", 200.0, 1.0, 2000.0, 1.0),
        ]);
    }
    match v.kind {
        Kind::IbsRoc => out.extend([
            p("ibs_dir", "IBS test (1 IBS <= level, final AFL; 2 IBS >= level, earlier AFL)", 1.0, 1.0, 2.0, 1.0),
            p("ibs_thr", "IBS threshold level", 0.5, 0.0, 1.0, 0.01),
            p("roc_days", "ROC days (3 = base 2 completed days back)", 2.0, 2.0, 5.0, 1.0),
            p("roc_thr", "ROC threshold %", 1.0, 0.0, 2.0, 0.5),
        ]),
        Kind::HighN => out.push(p("hdays", "Highest close days", 3.0, 3.0, 15.0, 2.0)),
        Kind::Green => out.extend([
            p("candle", "Candle colour (1 GREEN, 2 ANY)", 1.0, 1.0, 2.0, 1.0),
            p("green_days", "Green candle days (today + previous days - 1)", 2.0, 2.0, 5.0, 1.0),
        ]),
        Kind::Volume => out.extend([
            p("vol_days", "Volume check days (average of N completed days)", 5.0, 1.0, 100.0, 1.0),
            p("vol_mult", "Volume multiple", 1.0, 1.0, 20.0, 1.0),
        ]),
        Kind::Athc => out.push(p("athc_pct", "ATHC exit %", 10.0, 5.0, 25.0, 1.0)),
        Kind::RankPrice | Kind::RankIbs => out.push(p("rank_dir", "Ranking (1 high to low, 2 low to high)", 1.0, 1.0, 2.0, 1.0)),
        Kind::RankLookback => out.push(p("lb_version", "Lookback ranking (2 = ver 2 lower first, 11 = ver 11 higher first)", 2.0, 2.0, 11.0, 9.0)),
        Kind::RankRoc => out.push(p("roc_days", "ROC days (3 = a 2-day ROC)", 3.0, 2.0, 5.0, 1.0)),
        Kind::RankIbsVolume => out.extend([
            p("vol_days", "Volume check days (average of N completed days)", 5.0, 1.0, 100.0, 1.0),
            p("vol_mult", "Volume multiple", 1.0, 1.0, 100.0, 0.1),
        ]),
        _ => {}
    }
    if !v.kind.universe_only() {
        out.extend(sizing_params());
    }
    Some(out)
}

/// Portfolio parameters of a universe run (also read from the query for the Up-close sub-variants run on the universe).
pub fn universe_params() -> Vec<ParamDef> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    vec![
        p("max_positions", "Maximum open positions", 10.0, 1.0, 100.0, 1.0),
        p("pct", "Position size % of equity (0 = 100 / max positions)", 0.0, 0.0, 100.0, 0.5),
        p("pad", "Pad and align to NIFTY futures (0 off, 1 on: AmiBroker 'pad and align')", 0.0, 0.0, 1.0, 1.0),
        p("sizing", "Position sizing (1 % of equity, compounding: the team's sheets; 2 fixed amount per position, no compounding)", 1.0, 1.0, 2.0, 1.0),
        p("alloc", "Amount per position in Rs for sizing 2 (AFL EQUITY ALLOCATION; 0 = equity / max positions)", 0.0, 0.0, 100_000_000.0, 1000.0),
        p("equity", "Initial capital in Rs (AFL Equity)", 1_000_000.0, 1000.0, 1_000_000_000.0, 1000.0),
    ]
}

/// Position sizing of a single-instrument BTST run (AFL "FIXED EQUITY / % OF Equity", EQUITY ALLOCATION, Equity):
/// 1 = the whole equity compounds trade by trade (the existing report); 2 = floor(alloc / price) units of the futures
/// price per trade on `equity` initial capital, additive P&L (AmiBroker RoundLotSize 1, position shrinking on cash).
pub fn sizing_params() -> Vec<ParamDef> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    vec![
        p("sizing", "Position sizing (1 % of equity, compounding: the team's sheets; 2 fixed amount per trade, no compounding)", 1.0, 1.0, 2.0, 1.0),
        // index runs hold one position: the team's index sheets put the whole capital (~Rs 10 lakh) into each trade
        p("alloc", "Amount per trade in Rs for sizing 2 (AFL EQUITY ALLOCATION; default = the whole initial capital, as in the team's index sheets)", 1_000_000.0, 1000.0, 100_000_000.0, 1000.0),
        p("equity", "Initial capital in Rs (AFL Equity)", 1_000_000.0, 1000.0, 1_000_000_000.0, 1000.0),
    ]
}

/// The futures daily close of each row for a data source (NaN on rows without a day): marks for fixed-amount equity.
pub fn day_closes(x: &Ctx, src: i64, to: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; x.r.len()];
    if !(1..=3).contains(&src) || x.r.is_empty() {
        return out;
    }
    let b = Bars::load(x, src);
    let (rows, days, _) = daily_bars(x, &b, src, to.min(x.r.len() - 1));
    for (k, &i) in rows.iter().enumerate() {
        out[i] = days[k].c;
    }
    out
}

/// The 5-minute series of one run: for each bar time, the O, H, L and C columns and the volume column (None = the
/// column is not loaded).
struct Bars<'a> {
    times: Vec<u32>,
    cols: Vec<[Option<&'a [f64]>; 4]>,
    vol: Vec<Option<&'a [f64]>>,
    /// src 3: the team-copy minute rule (1 NIFTY, 2 BANKNIFTY) and each row's year; 0 = every bar as stored.
    team: u8,
    years: Vec<i32>,
}

/// src 3 ("IEOD as the team's 2021 copy"): the 5-minute buckets the team's 2021/22 copy of IEOD_FO did not have, as
/// their own trade lists prove (scripts/verify_btst_e34.py team_keep and --evidence): no bucket before 09:15 (the 2010
/// pre-open); NIFTY: no 15:30 bucket in 2011-2015 (sessions end at 15:30, so the bucket is exactly that minute);
/// BANKNIFTY: no buckets 15:30-16:15 in any year (Muhurat evening sessions kept). Rule 1 = NIFTY, 2 = BANKNIFTY.
pub fn team_keep(rule: u8, year: i32, hhmm: u32) -> bool {
    if hhmm < 915 {
        return false;
    }
    match rule {
        2 => !(1530..=1615).contains(&hhmm),
        _ => !((2011..=2015).contains(&year) && hhmm == 1530),
    }
}

/// The src-3 rule of a ticker (BANKNIFTY rule for NIFTYBANK, NIFTY rule otherwise).
pub fn team_rule(ticker: &str) -> u8 {
    if ticker == "NIFTYBANK" { 2 } else { 1 }
}

impl<'a> Bars<'a> {
    fn load(x: &'a Ctx, src: i64) -> Self {
        let fam = family(src);
        let times = bar_times(x, src);
        let cols = times.iter().map(|t| ["O", "H", "L", "C"].map(|k| x.series(&format!("{fam}.{k}{t:04}")))).collect();
        let vol = times.iter().map(|t| if src == 2 { None } else { x.series(&format!("{fam}.V{t:04}")) }).collect();
        let team = if src == 3 { x.r.first().map_or(1, |r| team_rule(&r.ticker)) } else { 0 };
        let years = if team > 0 { x.r.iter().map(|r| r.date.get(0..4).and_then(|y| y.parse().ok()).unwrap_or(0)).collect() } else { Vec::new() };
        Bars { times, cols, vol, team, years }
    }

    fn slot(&self, hhmm: i64) -> Option<usize> {
        self.times.iter().position(|&t| i64::from(t) == hhmm)
    }

    /// [open, high, low, close] of the bar in `slot` on row `i`; None when any of the four is missing.
    fn bar(&self, slot: usize, i: usize) -> Option<[f64; 4]> {
        if self.team > 0 && !team_keep(self.team, *self.years.get(i)?, self.times[slot]) {
            return None;
        }
        let mut out = [0.0; 4];
        for (k, col) in self.cols[slot].iter().enumerate() {
            let v = *(*col)?.get(i)?;
            if !v.is_finite() {
                return None;
            }
            out[k] = v;
        }
        Some(out)
    }

    /// Volume of the bar (0 when the cell is empty: AmiBroker volume 0).
    fn volume(&self, slot: usize, i: usize) -> f64 {
        self.vol[slot].and_then(|v| v.get(i)).copied().filter(|v| v.is_finite()).unwrap_or(0.0)
    }

    fn has_volume(&self) -> bool {
        self.vol.iter().any(Option::is_some)
    }

    /// Slots of the present bars of row `i`, in time order.
    fn present(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        (0..self.times.len()).filter(move |&s| self.bar(s, i).is_some())
    }
}

/// One daily, weekly or monthly bar.
#[derive(Clone, Copy)]
struct Ohlc {
    o: f64,
    h: f64,
    l: f64,
    c: f64,
}

/// The days of rows 0..=to: the row index of every day that exists, its bar and its volume (NaN without volume).
/// src 1 reads the D* columns (a day exists when DC is finite); src 2 builds the bar from the 5-minute bars; src 3
/// builds it from the bars left by the team-copy rule (on src 1 data that equals D*: the 0900..1930 buckets hold every
/// stored minute, checked for every NIFTY and BANKNIFTY day), volume = the bars' sum.
fn daily_bars(x: &Ctx, b: &Bars, src: i64, to: usize) -> (Vec<usize>, Vec<Ohlc>, Vec<f64>) {
    let (mut rows, mut days, mut vols) = (Vec::new(), Vec::new(), Vec::new());
    if src == 1 {
        let col = |k: &str| x.series(&format!("BTST_5M.{k}"));
        let (o, h, l, c, v) = (col("DO"), col("DH"), col("DL"), col("DC"), col("DV"));
        let at = |s: Option<&[f64]>, i: usize| s.and_then(|s| s.get(i)).copied().unwrap_or(f64::NAN);
        let Some(cc) = c else { return (rows, days, vols) };
        for i in 0..=to.min(cc.len().saturating_sub(1)) {
            if !cc[i].is_finite() {
                continue;
            }
            rows.push(i);
            days.push(Ohlc { o: at(o, i), h: at(h, i), l: at(l, i), c: cc[i] });
            vols.push(at(v, i));
        }
        return (rows, days, vols);
    }
    for i in 0..=to {
        let mut day: Option<Ohlc> = None;
        let mut vol = 0.0;
        for s in 0..b.times.len() {
            let Some([o, h, l, c]) = b.bar(s, i) else { continue };
            vol += b.volume(s, i);
            day = Some(match day {
                None => Ohlc { o, h, l, c },
                Some(d) => Ohlc { o: d.o, h: d.h.max(h), l: d.l.min(l), c },
            });
        }
        if let Some(d) = day {
            rows.push(i);
            days.push(d);
            vols.push(if src == 3 && b.has_volume() { vol } else { f64::NAN });
        }
    }
    (rows, days, vols)
}

/// Bars of one higher timeframe over the day list: each period's first and last day position (1 Daily, 2 Weekly, 3 Monthly).
fn periods(x: &Ctx, rows: &[usize], tf: i64) -> Vec<(usize, usize)> {
    let key = |k: usize| -> i64 {
        let date = &x.r[rows[k]].date;
        match tf {
            2 => day_number(date).map_or(k as i64, |d| (d + 3).div_euclid(7)), // Monday-based weeks
            3 => {
                let y: i64 = date.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(0);
                let m: i64 = date.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(0);
                y * 12 + m
            }
            _ => k as i64,
        }
    };
    let mut out: Vec<(usize, usize)> = Vec::new();
    for k in 0..rows.len() {
        match out.last_mut() {
            Some(last) if key(last.0) == key(k) => last.1 = k,
            _ => out.push((k, k)),
        }
    }
    out
}

fn period_bars(days: &[Ohlc], ps: &[(usize, usize)]) -> Vec<Ohlc> {
    ps.iter()
        .map(|&(a, e)| Ohlc {
            o: days[a].o,
            h: days[a..=e].iter().map(|d| d.h).fold(f64::MIN, f64::max),
            l: days[a..=e].iter().map(|d| d.l).fold(f64::MAX, f64::min),
            c: days[e].c,
        })
        .collect()
}

/// Simple moving average, NaN until `n` bars exist (AFL `MA`).
fn sma(c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|i| if i + 1 >= n { c[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect()
}

/// The AFL's coded Supertrend: `true` where the trend is up. ATR = simple average of the 1-bar true range over `period`
/// (NaN until `period` bars exist; the first bar's range is high - low). The bands are mutated bar by bar (ratchet and
/// re-seed on a flip) and the next bar compares against the mutated bands, exactly as the AFL loop does.
fn supertrend_up(b: &[Ohlc], point: i64, period: usize, factor: f64) -> Vec<bool> {
    let n = b.len();
    let tr: Vec<f64> = (0..n)
        .map(|i| if i == 0 { b[0].h - b[0].l } else { b[i].h.max(b[i - 1].c) - b[i].l.min(b[i - 1].c) })
        .collect();
    let satr: Vec<f64> = (0..n).map(|i| if i + 1 >= period { tr[i + 1 - period..=i].iter().sum::<f64>() / period as f64 } else { f64::NAN }).collect();
    let src: Vec<f64> = b.iter().map(|d| match point { 2 => d.c, 3 => (d.h + d.l + d.c) / 3.0, _ => (d.h + d.l) / 2.0 }).collect();
    let mut up: Vec<f64> = (0..n).map(|i| src[i] + factor * satr[i]).collect();
    let mut dn: Vec<f64> = (0..n).map(|i| src[i] - factor * satr[i]).collect();
    let mut trend = vec![1i8; n];
    for i in 1..n {
        trend[i] = if b[i].c > up[i - 1] {
            1
        } else if b[i].c < dn[i - 1] {
            -1
        } else {
            trend[i - 1]
        };
        let flag = trend[i] < 0 && trend[i - 1] > 0;
        let flagh = trend[i] > 0 && trend[i - 1] < 0;
        if trend[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if trend[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if flag {
            up[i] = src[i] + factor * satr[i];
        }
        if flagh {
            dn[i] = src[i] - factor * satr[i];
        }
    }
    trend.iter().map(|&t| t == 1).collect()
}

/// Trend-filter state for every day position: the filter value of the last COMPLETED bar of the chosen timeframe
/// (AFL `TimeFrameExpand`, expandLast). False while no completed bar exists, except for the OFF filter.
fn filter_flags(x: &Ctx, rows: &[usize], days: &[Ohlc], kind: i64, tf: i64, point: i64, period: usize, factor: f64, sma_n: usize) -> Vec<bool> {
    if kind == 3 {
        return vec![true; days.len()];
    }
    let ps = periods(x, rows, tf);
    let bars = period_bars(days, &ps);
    let ok: Vec<bool> = if kind == 2 {
        let c: Vec<f64> = bars.iter().map(|d| d.c).collect();
        let ma = sma(&c, sma_n);
        (0..c.len()).map(|k| k >= 1 && c[k] > ma[k - 1]).collect()
    } else {
        supertrend_up(&bars, point, period, factor)
    };
    let mut flags = vec![false; days.len()];
    for (p, &(s, e)) in ps.iter().enumerate() {
        for k in s..=e {
            flags[k] = p > 0 && ok[p - 1];
        }
    }
    flags
}

/// The ATHC-swing filter's BUYSIG per day (the AFL's daily loop): Hcl = running highest close, Up = Hcl,
/// Dn = C - C x pct / 100, trend flipped and bands ratcheted as coded; BUYSIG on at a new highest close (C > Hcl[-1]),
/// off when the trend is down. SELLSIG (trend down) is computed by the AFL but never used, so it is not returned.
fn athc_buysig(dc: &[f64], pct: f64) -> Vec<bool> {
    let n = dc.len();
    let mut hcl = vec![f64::NAN; n];
    for i in 0..n {
        hcl[i] = if i == 0 { dc[0] } else { hcl[i - 1].max(dc[i]) };
    }
    let exit: Vec<f64> = dc.iter().map(|c| c * pct / 100.0).collect();
    let mut up = hcl.clone();
    let mut dn: Vec<f64> = (0..n).map(|i| dc[i] - exit[i]).collect();
    let mut trend = vec![1i8; n];
    for i in 1..n {
        trend[i] = if dc[i] > up[i - 1] {
            1
        } else if dc[i] < dn[i - 1] {
            -1
        } else {
            trend[i - 1]
        };
        let flag = trend[i] < 0 && trend[i - 1] > 0;
        let flagh = trend[i] > 0 && trend[i - 1] < 0;
        if trend[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if trend[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if flag {
            up[i] = hcl[i];
        }
        if flagh {
            dn[i] = dc[i] - exit[i];
        }
    }
    let mut buy = vec![false; n];
    for i in 1..n {
        buy[i] = if dc[i] > hcl[i - 1] {
            true
        } else if trend[i] == -1 && buy[i - 1] {
            false
        } else {
            buy[i - 1]
        };
    }
    buy
}

/// AFL buycond1 and buycond2 on the daily closes, evaluated at the last completed day `d`: the last n-1 closes each rose
/// and the last n-2 daily changes each exceeded the one before (n = 2 has no change terms).
fn lookback_ok(c: &[f64], d: usize, n: usize) -> bool {
    if d + 1 < n {
        return false;
    }
    let diff = |k: usize| c[k] - c[k - 1];
    (0..n - 1).all(|j| c[d - j] > c[d - j - 1]) && (0..n.saturating_sub(2)).all(|j| diff(d - j) > diff(d - j - 1))
}

/// The open position: the trade and its profit-booking and hard-stop levels (AFL pb and hs).
struct Live {
    trade: Trade,
    pb: f64,
    hs: f64,
    k: usize,
    entry_t: u32,
    score: f64,
}

/// One trade with what a universe portfolio needs: entry and exit bar times, the PositionScore at the entry, the symbol's
/// day-end closes while held (row, close) and, on later days on which the position is still open at the entry time, the
/// close of the last bar before the entry time (row, close: the previous-bar mark of AmiBroker's equity).
#[derive(Clone, Debug)]
pub struct Fill {
    pub trade: Trade,
    pub entry_t: u32,
    pub exit_t: u32,
    pub score: f64,
    pub day_marks: Vec<(usize, f64)>,
    pub t_marks: Vec<(usize, f64)>,
}

/// Everything an entry decision reads, precomputed once per run.
struct Setup<'a> {
    kind: Kind,
    b: Bars<'a>,
    rows: Vec<usize>,
    dc: Vec<f64>,
    days: Vec<Ohlc>,
    dv: Vec<f64>,
    flt: Vec<bool>,
    athc: Vec<bool>,
    ath: Vec<f64>,
    n: usize,
    entry_time: u32,
    /// AFL Penny Stock Price: a buy needs the entry close above it (`C > pennyflt`); 0 = off.
    penny: f64,
    ibs_dir: i64,
    ibs_thr: f64,
    roc_days: usize,
    roc_thr: f64,
    hdays: usize,
    green: bool,
    green_days: usize,
    vol_days: usize,
    vol_mult: f64,
    rank_dir: i64,
    lb_version: i64,
    /// Alphabetical position of the symbol in the universe (lookback-ranking ties).
    list_rank: f64,
    /// NIFTY futures aligned to this symbol's timeline, per day position: the daily close and the close at the entry bar.
    nifty_dc: Vec<f64>,
    nifty_e: Vec<f64>,
}

impl Setup<'_> {
    /// IBS of the day so far at the entry bar (NaN when high = low).
    fn ibs(&self, i: usize, s: usize, p: f64) -> f64 {
        let (mut ih, mut il) = (f64::MIN, f64::MAX);
        for t in 0..=s {
            if let Some([_, h, l, _]) = self.b.bar(t, i) {
                ih = ih.max(h);
                il = il.min(l);
            }
        }
        if ih > il { (ih - p) / (ih - il) } else { f64::NAN }
    }

    /// ROC % against Ref(TimeFrameExpand(Ref(C, -(roc_days - 2))), -5); NaN without a value.
    fn roc(&self, k: usize, s: usize, p: f64) -> f64 {
        self.back5(k, s)
            .and_then(|(kb, sb)| {
                let last_of_day = self.b.present(self.rows[kb]).last() == Some(sb);
                let seen = if kb == k || !last_of_day { kb.checked_sub(1) } else { Some(kb) };
                seen.and_then(|j| j.checked_sub(self.roc_days - 2)).map(|x| (p - self.dc[x]) / self.dc[x] * 100.0)
            })
            .unwrap_or(f64::NAN)
    }

    /// The universe PositionScore of a signal (higher first); 0 where the AFL has none or it is Null.
    fn score(&self, k: usize, i: usize, s: usize, p: f64) -> f64 {
        let d = k - 1;
        let v = match self.kind {
            Kind::RankPrice => if self.rank_dir == 2 { 1_000_000.0 - p } else { p },
            Kind::RankIbs => {
                let ibs = self.ibs(i, s, p);
                if self.rank_dir == 2 { 100.0 - ibs } else { ibs }
            }
            Kind::RankIbsVolume => self.ibs(i, s, p),
            Kind::RankRoc => 100.0 + self.roc(k, s, p),
            Kind::RankLookback => {
                let cond = (1..=4).filter(|j| lookback_ok(&self.dc, d, self.n + j)).count() as f64;
                if self.lb_version == 11 { cond * 10_000.0 - self.list_rank } else { -cond * 10_000.0 + self.list_rank }
            }
            _ => 0.0,
        };
        if v.is_finite() { v } else { 0.0 }
    }

    /// The NIFTY conditions (aligned NIFTY futures, the stock's day sequence).
    fn nifty_ok(&self, k: usize) -> bool {
        let d = k - 1;
        let (nf, e) = (&self.nifty_dc, self.nifty_e.get(k).copied().unwrap_or(f64::NAN));
        if nf.len() != self.dc.len() || !e.is_finite() || !nf[d].is_finite() {
            return false;
        }
        match self.kind {
            Kind::NiftyUp => e > nf[d],
            Kind::NiftyInc => d >= 2 && nf[d - 1].is_finite() && lookback_ok(nf, d, self.n) && e > nf[d] && e - nf[d] > nf[d] - nf[d - 1],
            _ => true,
        }
    }
    /// The present bar 5 bars before (day position k, slot s), walking back over earlier days: (day position, slot).
    fn back5(&self, k: usize, s: usize) -> Option<(usize, usize)> {
        let mut left = 5usize;
        let (mut kk, mut top) = (k, s);
        loop {
            let i = self.rows[kk];
            for t in (0..top).rev() {
                if self.b.bar(t, i).is_some() {
                    left -= 1;
                    if left == 0 {
                        return Some((kk, t));
                    }
                }
            }
            kk = kk.checked_sub(1)?;
            top = self.b.times.len();
        }
    }

    /// Today's volume so far as the AFL's avgvol loop has it at the entry bar: summed bar by bar, reset after every
    /// bar at or after the entry time. A day without bars on the grid (only evening-session minutes, all after the
    /// entry time) also resets.
    fn volume_so_far(&self, k: usize, s: usize) -> f64 {
        let i = self.rows[k];
        let mut sum: f64 = (0..=s).filter(|&t| self.b.bar(t, i).is_some()).map(|t| self.b.volume(t, i)).sum();
        let mut kk = k;
        while kk > 0 {
            kk -= 1;
            let j = self.rows[kk];
            let present: Vec<usize> = self.b.present(j).collect();
            if present.is_empty() || present.iter().any(|&t| self.b.times[t] >= self.entry_time) {
                break;
            }
            sum += present.iter().map(|&t| self.b.volume(t, j)).sum::<f64>();
        }
        sum
    }

    /// Entry price P when every entry condition holds at the entry bar `s` of day position `k` (row `i`).
    fn entry(&self, k: usize, i: usize, s: usize) -> Option<f64> {
        let p = self.b.bar(s, i)?[3];
        if !(p > self.penny) {
            return None; // AFL buyflt: C > pennyflt
        }
        let d = k.checked_sub(1)?;
        let dc = &self.dc;
        if self.kind == Kind::OldFixed {
            if d < 2 {
                return None;
            }
            let (c1, c2, c3) = (dc[d], dc[d - 1], dc[d - 2]);
            return (p > c1 && c1 > c2 && c2 > c3 && p - c1 >= c1 - c2 && c1 - c2 >= c2 - c3).then_some(p);
        }
        if self.kind == Kind::AthOnly {
            return (self.flt[k] && p > self.ath[d]).then_some(p);
        }
        // up-close core: buycond1, buycond2, price above the last close by more than the last change; buycond2 is Null
        // (no trade) until DIFF > Ref(DIFF, -1) exists, i.e. before the symbol's 4th data day
        if d < 2 || !lookback_ok(dc, d, self.n) || !(p > dc[d] && p - dc[d] > dc[d] - dc[d - 1]) {
            return None;
        }
        if self.kind == Kind::Athc {
            return self.athc[d].then_some(p);
        }
        if !self.flt[k] {
            return None;
        }
        let ok = match self.kind {
            Kind::IbsRoc => {
                let (mut ih, mut il) = (f64::MIN, f64::MAX);
                for t in 0..=s {
                    if let Some([_, h, l, _]) = self.b.bar(t, i) {
                        ih = ih.max(h);
                        il = il.min(l);
                    }
                }
                let ibs = (ih - p) / (ih - il);
                let ibs_ok = ih > il && if self.ibs_dir == 2 { ibs >= self.ibs_thr } else { ibs <= self.ibs_thr };
                // ROC base: TimeFrameExpand(Ref(C, -(ROC days - 2))) read 5 bars before the entry bar
                let roc_ok = self.back5(k, s).is_some_and(|(kb, sb)| {
                    let last_of_day = self.b.present(self.rows[kb]).last() == Some(sb);
                    let seen = if kb == k || !last_of_day { kb.checked_sub(1) } else { Some(kb) };
                    seen.and_then(|j| j.checked_sub(self.roc_days - 2)).is_some_and(|x| {
                        let base = dc[x];
                        (p - base) / base * 100.0 >= self.roc_thr
                    })
                });
                ibs_ok && roc_ok
            }
            Kind::Ath => p > self.ath[d],
            Kind::HighN => d + 1 >= self.hdays && p > dc[d + 1 - self.hdays..=d].iter().copied().fold(f64::MIN, f64::max),
            Kind::Green => {
                !self.green || {
                    let first_open = self.b.present(i).next().and_then(|t| self.b.bar(t, i)).map_or(f64::NAN, |b| b[0]);
                    let m = self.green_days - 1;
                    p > first_open && d + 1 >= m && self.days[d + 1 - m..=d].iter().all(|x| x.c > x.o)
                }
            }
            Kind::NiftyUp | Kind::NiftyInc => self.nifty_ok(k),
            Kind::Volume | Kind::RankIbsVolume => {
                d + 1 >= self.vol_days && {
                    let w = &self.dv[d + 1 - self.vol_days..=d];
                    let ma = w.iter().sum::<f64>() / self.vol_days as f64;
                    ma.is_finite() && self.volume_so_far(k, s) > self.vol_mult * ma
                }
            }
            _ => true,
        };
        ok.then_some(p)
    }
}

/// The AFL's Supertrend loop with its band: (trend +1/-1 per bar, band = Dn on an up bar, Up on a down bar; NaN while the
/// ATR is still warming up). Same loop as `supertrend_up`.
fn supertrend_full(b: &[Ohlc], point: i64, period: usize, factor: f64) -> (Vec<i8>, Vec<f64>) {
    let n = b.len();
    let period = period.max(1);
    let tr: Vec<f64> = (0..n)
        .map(|i| if i == 0 { b[0].h - b[0].l } else { b[i].h.max(b[i - 1].c) - b[i].l.min(b[i - 1].c) })
        .collect();
    let satr: Vec<f64> = (0..n).map(|i| if i + 1 >= period { tr[i + 1 - period..=i].iter().sum::<f64>() / period as f64 } else { f64::NAN }).collect();
    let src: Vec<f64> = b.iter().map(|d| match point { 2 => d.c, 3 => (d.h + d.l + d.c) / 3.0, _ => (d.h + d.l) / 2.0 }).collect();
    let mut up: Vec<f64> = (0..n).map(|i| src[i] + factor * satr[i]).collect();
    let mut dn: Vec<f64> = (0..n).map(|i| src[i] - factor * satr[i]).collect();
    let mut trend = vec![1i8; n];
    for i in 1..n {
        trend[i] = if b[i].c > up[i - 1] {
            1
        } else if b[i].c < dn[i - 1] {
            -1
        } else {
            trend[i - 1]
        };
        let flag = trend[i] < 0 && trend[i - 1] > 0;
        let flagh = trend[i] > 0 && trend[i - 1] < 0;
        if trend[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if trend[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if flag {
            up[i] = src[i] + factor * satr[i];
        }
        if flagh {
            dn[i] = src[i] - factor * satr[i];
        }
    }
    let band = (0..n).map(|i| if trend[i] == 1 { dn[i] } else { up[i] }).collect();
    (trend, band)
}

/// One candle of the E3/E4 runs: day position k, row i, start time, OHLC.
#[derive(Clone, Copy, Debug)]
struct Candle {
    k: usize,
    i: usize,
    t: u32,
    o: f64,
    h: f64,
    l: f64,
    c: f64,
}

/// The candles of the days in `rows` (day positions), in time order: the present 5-minute bars (tf 1) or 15-minute
/// candles aggregated from them (tf 2: clock buckets :00/:15/:30/:45, open first, high max, low min, close last).
fn candles(b: &Bars, rows: &[usize], tf: i64) -> Vec<Candle> {
    let mut out: Vec<Candle> = Vec::new();
    for (k, &i) in rows.iter().enumerate() {
        for s in 0..b.times.len() {
            let Some([o, h, l, c]) = b.bar(s, i) else { continue };
            let t = b.times[s];
            let t = if tf == 2 { t - (t % 100) % 15 } else { t };
            match out.last_mut() {
                Some(last) if tf == 2 && last.k == k && last.t == t => {
                    last.h = last.h.max(h);
                    last.l = last.l.min(l);
                    last.c = c;
                }
                _ => out.push(Candle { k, i, t, o, h, l, c }),
            }
        }
    }
    out
}

/// Higher-timeframe trend seen at each candle (AFL TimeFrameExpand, expandLast): the latest period whose last candle is at
/// or before the candle. Before the first period is complete AmiBroker shows the FIRST period's own value (the team's lists
/// buy on the first day of data with the filter on: NIFTY 2007-01-02 daily, BANKNIFTY 2007-05-29..06-01 weekly).
/// Returns Some((trend, band defined and > 0)) per candle.
fn trend_seen(x: &Ctx, rows: &[usize], days: &[Ohlc], cs: &[Candle], tf: i64, point: i64, period: usize, factor: f64) -> Vec<Option<(i8, bool)>> {
    let ps = periods(x, rows, tf);
    let bars = period_bars(days, &ps);
    let (trend, band) = supertrend_full(&bars, point, period, factor);
    let mut of_day = vec![(0usize, false); days.len()]; // (period, is the period's last day)
    for (p, &(a, e)) in ps.iter().enumerate() {
        for k in a..=e {
            of_day[k] = (p, k == e);
        }
    }
    (0..cs.len())
        .map(|j| {
            let k = cs[j].k;
            let last_candle_of_day = j + 1 == cs.len() || cs[j + 1].k != k;
            let (p, last_day) = of_day[k];
            let q = if last_candle_of_day && last_day { p } else { p.saturating_sub(1) };
            Some((trend[q], band[q].is_finite() && band[q] > 0.0))
        })
        .collect()
}

/// AmiBroker's backtester fills a price outside the bar's low..high at the bar's low or high (e.g. a gapped open less
/// the selling buffer below the bar's low; team BANKNIFTY candle-count list 2009-05-18 filled at 6250 = the bar's low).
fn clamp(px: f64, l: f64, h: f64) -> f64 {
    px.max(l).min(h)
}

/// AFL Prec(x, 2) equality of two prices (identical to rounding on the 0.05-tick index futures data).
fn eq2(a: f64, b: f64) -> bool {
    (a * 100.0).round() == (b * 100.0).round()
}

/// E3 (second-last candle, with optional bull/bear Supertrend filter) and E4 (candle-count) runs.
/// E3/E4 trades with the start times of their entry and exit candles (exit 0 = still open).
fn run_e34(kind: Kind, x: &Ctx, b: &Bars, src: i64, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<(Trade, u32, u32)> {
    let tf = g("tf").round() as i64;
    if tf != 1 && tf != 2 {
        return Vec::new();
    }
    let (rows, days, _) = daily_bars(x, b, src, to);
    let cs = candles(b, &rows, tf);
    let n = cs.len();
    let flt_on = match kind {
        Kind::SlcBull | Kind::SlcBear => true,
        Kind::Candles => g("flt_type").round() as i64 != 3,
        _ => false,
    };
    let seen = if flt_on {
        let u = |k: &str| g(k).round().max(1.0) as usize;
        trend_seen(x, &rows, &days, &cs, g("flt_tf").round() as i64, g("st_point").round() as i64, u("st_period"), g("st_factor"))
    } else {
        vec![Some((1, true)); n]
    };
    let mk = |c: &Candle| Trade {
        dir: 1.0,
        legs: vec![Leg { i: c.i, px: c.c, w: 1.0 }],
        exit_i: c.i,
        exit_px: c.c,
        reason: String::new(),
        entry_kind: "close",
        signal_i: c.i,
        open: false,
        exits: Vec::new(),
    };
    let close_at = |mut t: Trade, c: &Candle, px: f64, why: String| {
        t.exit_i = c.i;
        t.exit_px = px;
        t.signal_i = c.i;
        t.reason = why;
        t
    };
    let mut trades = Vec::new();
    let mut cur: Option<Trade> = None;
    let mut cur_t = 0u32;
    let mut last_c = f64::NAN;
    if kind == Kind::Candles {
        let entry = g("entry_candle").round().max(0.0) as usize;
        let exit_n = g("exit_candle").round().max(1.0) as usize;
        let (pt_pct, buf) = (g("pt_pct"), g("buffer_pct"));
        let (mut pt, mut count, mut buy_k) = (f64::INFINITY, 0usize, 0usize);
        for j in 0..n {
            let c = &cs[j];
            if c.i < from {
                continue;
            }
            if cur.is_some() && c.k != buy_k {
                count += 1;
            }
            // Entry_Candle: entry k = the candle k before the day's last one
            let is_entry = j + entry < n && cs[j + entry].k == c.k && (j + entry + 1 >= n || cs[j + entry + 1].k != c.k);
            let tr = seen[j].map(|s| s.0);
            if cur.is_none() {
                if is_entry && tr == Some(1) {
                    cur = Some(mk(c));
                    cur_t = c.t;
                    pt = if pt_pct > 0.0 { (1.0 + pt_pct * 0.01) * c.c } else { f64::INFINITY };
                    buy_k = c.k;
                    count = 0;
                    last_c = c.c;
                }
                continue;
            }
            last_c = c.c;
            let hhmm = c.t;
            let done = if c.h >= pt {
                Some((clamp(if c.o > pt { c.o - c.o * buf / 100.0 } else { pt }, c.l, c.h), format!("Profit target {pt_pct}% at {hhmm:04}")))
            } else if flt_on && tr == Some(-1) {
                Some((c.c, format!("Trend exit at {hhmm:04}")))
            } else if c.k != buy_k && count >= exit_n {
                Some((c.c, format!("Candle {exit_n} exit at {hhmm:04}")))
            } else {
                None
            };
            if let Some((px, why)) = done {
                trades.push((close_at(cur.take().unwrap(), c, px, why), cur_t, c.t));
                pt = f64::INFINITY;
                count = 0;
            }
        }
    } else {
        let want: i8 = if kind == Kind::SlcBear { -1 } else { 1 };
        for j in 0..n {
            let c = &cs[j];
            if c.i < from {
                continue;
            }
            let first = j == 0 || cs[j - 1].k != c.k;
            let second_last = j + 1 < n && cs[j + 1].k == c.k && (j + 2 >= n || cs[j + 2].k != c.k);
            let up_circuit = eq2(c.o, c.c) && eq2(c.o, c.h);
            let lo_circuit = eq2(c.o, c.c) && eq2(c.o, c.l);
            let filter_ok = !flt_on || seen[j].is_some_and(|(t, band)| t == want && band);
            let mut bought_now = false;
            if cur.is_none() && second_last && !up_circuit && filter_ok {
                cur = Some(mk(c));
                cur_t = c.t;
                bought_now = true;
            }
            if cur.is_some() {
                last_c = c.c;
            }
            if cur.is_some() && first && !lo_circuit {
                let t = cur.take().unwrap();
                if !bought_now {
                    trades.push((close_at(t, c, c.c, format!("First candle exit at {:04}", c.t)), cur_t, c.t));
                }
            }
        }
    }
    if let Some(mut t) = cur {
        t.exit_i = to;
        t.exit_px = if last_c.is_finite() { last_c } else { t.legs[0].px };
        t.reason = "open".into();
        t.signal_i = to;
        t.open = true;
        trades.push((t, cur_t, 0));
    }
    trades
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    run_inner(id, x, from, to, p, false).map(|f| f.into_iter().map(|f| f.trade).collect())
}

/// Every single-symbol trade of a universe sub-variant on one stock (the whole history), with portfolio details.
/// `p` may carry `list_rank` (the symbol's alphabetical position) and the Ctx the aligned NIFTY series `NIFTYREF.DC` and
/// `NIFTYREF.E` (per row: NIFTY's daily close on this symbol's timeline, NIFTY's close at this symbol's entry bar).
pub fn universe_fills(id: &str, x: &Ctx, p: &Params) -> Option<Vec<Fill>> {
    if !universe_runnable(id) || x.r.is_empty() {
        return None;
    }
    run_inner(id, x, 0, x.r.len() - 1, p, true)
}

fn run_inner(id: &str, x: &Ctx, from: usize, to: usize, p: &Params, universe: bool) -> Option<Vec<Fill>> {
    let v = variant(id).filter(|v| v.kind != Kind::Stock && (universe || !v.kind.universe_only()))?;
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let src = src_of(p);
    if !(1..=3).contains(&src) || to >= x.r.len() {
        return Some(Vec::new());
    }
    let b = Bars::load(x, src);
    if v.kind.e34() {
        let plain = |(trade, entry_t, exit_t)| Fill { trade, entry_t, exit_t, score: 0.0, day_marks: Vec::new(), t_marks: Vec::new() };
        return Some(run_e34(v.kind, x, &b, src, from, to, &g).into_iter().map(plain).collect());
    }
    if matches!(v.kind, Kind::Volume | Kind::RankIbsVolume) && !b.has_volume() {
        return Some(Vec::new());
    }
    let old = v.kind == Kind::OldFixed;
    let on_grid = |t: i64, lo: i64, hi: i64| (lo..=hi).contains(&t) && t % 5 == 0 && t % 100 < 60;
    let (entry_time, exit_time) = if old { (1515, 0) } else { (g("entry_time").round() as i64, g("exit_time").round() as i64) };
    if !old && !(on_grid(entry_time, 1400, 1520) && on_grid(exit_time, 915, 955)) {
        return Some(Vec::new());
    }
    // the old fixed AFL (ver3) has no fixed entry time: its entry candle is counted from the day's last candle
    let Some(entry_s) = b.slot(entry_time).or(old.then_some(usize::MAX)) else { return Some(Vec::new()) };
    let (pb_pct, hardstop) = if old { (0.0, 0.0) } else { (g("pb_pct"), g("hardstop")) };
    let (rows, days, dv) = daily_bars(x, &b, src, to);
    let dc: Vec<f64> = days.iter().map(|d| d.c).collect();
    let flt = if matches!(v.kind, Kind::OldFixed | Kind::Athc) {
        vec![true; days.len()]
    } else {
        filter_flags(x, &rows, &days, g("flt_type").round() as i64, g("flt_tf").round() as i64, g("st_point").round() as i64, u("st_period"), g("st_factor"), u("sma_period"))
    };
    let athc = if v.kind == Kind::Athc { athc_buysig(&dc, g("athc_pct")) } else { Vec::new() };
    let mut ath = Vec::with_capacity(dc.len());
    for (k, &c) in dc.iter().enumerate() {
        ath.push(if k == 0 { c } else { f64::max(ath[k - 1], c) });
    }
    let setup = Setup {
        kind: v.kind,
        rows,
        dc,
        days,
        dv,
        flt,
        athc,
        ath,
        n: u("days").max(2),
        entry_time: entry_time as u32,
        penny: g("penny"),
        ibs_dir: g("ibs_dir").round() as i64,
        ibs_thr: g("ibs_thr"),
        roc_days: u("roc_days").max(2),
        roc_thr: g("roc_thr"),
        hdays: u("hdays"),
        green: g("candle").round() as i64 != 2,
        green_days: u("green_days").max(2),
        vol_days: u("vol_days"),
        vol_mult: g("vol_mult"),
        rank_dir: g("rank_dir").round() as i64,
        lb_version: g("lb_version").round() as i64,
        list_rank: p.get("list_rank").copied().unwrap_or(0.0),
        nifty_dc: Vec::new(),
        nifty_e: Vec::new(),
        b,
    };
    let mut setup = setup;
    if matches!(v.kind, Kind::NiftyUp | Kind::NiftyInc) {
        let at = |name: &str| -> Vec<f64> {
            let col = x.series(name);
            setup.rows.iter().map(|&i| col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN)).collect()
        };
        setup.nifty_dc = at("NIFTYREF.DC");
        setup.nifty_e = at("NIFTYREF.E");
    }
    let setup = setup;
    let b = &setup.b;
    let gap = |o: f64| o - o * 0.25 / 100.0;

    // marks of a position from day position ke to kx (exit day; `held_at_t` = still open at the entry time on day kx)
    let last_close_of = |row: usize| b.present(row).last().and_then(|s| b.bar(s, row)).map(|v| v[3]);
    let marks = |ke: usize, kx: usize, include_exit_day: bool, held_at_t: bool| -> (Vec<(usize, f64)>, Vec<(usize, f64)>) {
        let (mut day, mut tm) = (Vec::new(), Vec::new());
        let mut last = f64::NAN;
        for kk in ke..=kx {
            let row = setup.rows[kk];
            if kk > ke && (kk < kx || held_at_t) {
                let before = (0..b.times.len()).filter(|&s| i64::from(b.times[s]) < entry_time).filter_map(|s| b.bar(s, row)).last().map(|v| v[3]);
                if let Some(c) = before.or(if last.is_finite() { Some(last) } else { None }) {
                    tm.push((row, c));
                }
            }
            if kk < kx || include_exit_day {
                if let Some(c) = last_close_of(row) {
                    day.push((row, c));
                    last = c;
                }
            } else if let Some(c) = last_close_of(row) {
                last = c;
            }
        }
        (day, tm)
    };
    let mut trades: Vec<Fill> = Vec::new();
    let mut cur: Option<Live> = None;
    let mut last_close = f64::NAN; // the last bar close seen while a position was open (marks an open trade)
    if old {
        // ver3_bar-count.afl: each day's candles are counted as the data has them, except a lone 15:30 candle (a 15:30
        // candle that is the day's last). Buy on the entry_n-th counted candle back from the day's last when its close
        // P > C1 > C2 > C3 and P - C1 >= C1 - C2 >= C2 - C3, C1..C3 = the closes of the last three days' last counted
        // candles; sell on the exit_n-th counted candle of a later day than the buy (the AFL's position loop: sell
        // before buy on the same candle).
        let (entry_n, exit_n) = (u("entry_n"), u("exit_n"));
        let lone = b.slot(1530);
        let (mut c1, mut c2, mut c3): (Option<f64>, Option<f64>, Option<f64>) = (None, None, None);
        for (k, &i) in setup.rows.iter().enumerate() {
            let mut bars: Vec<usize> = b.present(i).collect();
            if bars.is_empty() {
                continue;
            }
            if lone.is_some() && bars.last().copied() == lone {
                bars.pop();
            }
            let n = bars.len();
            for (j, &s) in bars.iter().enumerate() {
                let Some([_, h, l, c]) = b.bar(s, i) else { continue };
                if let Some(live) = cur.as_ref() {
                    last_close = c;
                    if j + 1 == exit_n && k > live.k {
                        let t = b.times[s];
                        let live = cur.take().unwrap();
                        let mut tr = live.trade;
                        tr.exit_i = i;
                        tr.exit_px = clamp(c, l, h);
                        tr.signal_i = i;
                        tr.reason = format!("Exit candle {exit_n} at {t:04}");
                        let (day_marks, t_marks) = if universe { marks(live.k, k, false, i64::from(t) >= entry_time) } else { (Vec::new(), Vec::new()) };
                        trades.push(Fill { trade: tr, entry_t: live.entry_t, exit_t: t, score: live.score, day_marks, t_marks });
                    }
                }
                if cur.is_none() && i >= from && n - j == entry_n {
                    let ok = matches!((c1, c2, c3), (Some(a), Some(b2), Some(b3)) if c > a && a > b2 && b2 > b3 && c - a >= a - b2 && a - b2 >= b2 - b3);
                    if ok {
                        let score = if universe { setup.score(k, i, s, c) } else { 0.0 };
                        let trade = Trade {
                            dir: 1.0,
                            legs: vec![Leg { i, px: c, w: 1.0 }],
                            exit_i: i,
                            exit_px: c,
                            reason: String::new(),
                            entry_kind: "close",
                            signal_i: i,
                            open: false,
                            exits: Vec::new(),
                        };
                        cur = Some(Live { trade, pb: f64::INFINITY, hs: 0.0, k, entry_t: b.times[s], score });
                        last_close = c;
                    }
                }
            }
            (c3, c2, c1) = (c2, c1, bars.last().and_then(|&s| b.bar(s, i)).map(|v| v[3]));
        }
    }
    let rows_main: &[usize] = if old { &[] } else { &setup.rows };
    for (k, &i) in rows_main.iter().enumerate() {
        if i < from {
            continue;
        }
        for s in 0..b.times.len() {
            let Some([o, h, l, c]) = b.bar(s, i) else { continue };
            if cur.is_none() && s == entry_s {
                if let Some(px) = setup.entry(k, i, s) {
                    let score = if universe { setup.score(k, i, s, px) } else { 0.0 };
                    let pb = if pb_pct > 0.0 { px + px * pb_pct / 100.0 } else { f64::INFINITY };
                    let hs = if hardstop > 0.0 { px - px * hardstop / 100.0 } else { 0.0 };
                    let trade = Trade {
                        dir: 1.0,
                        legs: vec![Leg { i, px, w: 1.0 }],
                        exit_i: i,
                        exit_px: px,
                        reason: String::new(),
                        entry_kind: "close",
                        signal_i: i,
                        open: false,
                        exits: Vec::new(),
                    };
                    cur = Some(Live { trade, pb, hs, k, entry_t: b.times[s], score });
                }
            }
            let Some(live) = cur.as_ref() else { continue };
            last_close = c;
            let t = i64::from(b.times[s]);
            let time_exit = t >= exit_time && t < entry_time;
            if !(h > live.pb || l < live.hs || time_exit) {
                continue;
            }
            let (px, reason) = if h > live.pb {
                (if o > live.pb { gap(o) } else { live.pb }, format!("Profit booking {pb_pct}% at {t:04}"))
            } else if o < live.hs {
                (gap(o), format!("Hard stop {hardstop}% at {t:04}"))
            } else if l < live.hs {
                (live.hs, format!("Hard stop {hardstop}% at {t:04}"))
            } else {
                (c, format!("Time exit {t:04}"))
            };
            let live = cur.take().unwrap();
            let mut tr = live.trade;
            tr.exit_i = i;
            tr.exit_px = clamp(px, l, h);
            tr.signal_i = i;
            tr.reason = reason;
            let (day_marks, t_marks) = if universe { marks(live.k, k, false, t >= entry_time) } else { (Vec::new(), Vec::new()) };
            trades.push(Fill { trade: tr, entry_t: live.entry_t, exit_t: b.times[s], score: live.score, day_marks, t_marks });
        }
    }
    if let Some(live) = cur {
        let mut t = live.trade;
        t.exit_i = to;
        t.exit_px = if last_close.is_finite() { last_close } else { t.legs[0].px };
        t.reason = "open".into();
        t.signal_i = to;
        t.open = true;
        let (day_marks, t_marks) = if universe { marks(live.k, setup.rows.len() - 1, true, true) } else { (Vec::new(), Vec::new()) };
        trades.push(Fill { trade: t, entry_t: live.entry_t, exit_t: 0, score: live.score, day_marks, t_marks });
    }
    Some(trades)
}

/// The start times (HHMM) of each trade's entry and exit bar or candle, in `run` order (exit 0 = still open).
pub fn trade_times(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<(u32, u32)> {
    run_inner(id, x, from, to, p, false).map_or_else(Vec::new, |f| f.into_iter().map(|f| (f.entry_t, f.exit_t)).collect())
}

/// The IEOD_FO database the BTST series come from (the team's AmiBroker database).
pub const DATABASE: &str = "DATABASE/Intraday_FO/IEOD_FO STOCKS-EODIEOD_UPDATED TILL 07-02-2023";
/// Where the src 2 feed comes from: a rolling Nifty futures file, NOT the IEOD database.
pub const FEED_SOURCE: &str = "Manan Pujara/#Data/stores/NIFTY_full/underlying/rolling_futures.parquet (rolling Nifty futures feed, not the IEOD database)";

/// The data a run's instrument comes from, for the response's `instrument.database`.
pub fn source_of(symbol: &str) -> &'static str {
    if symbol == "NIFTY_FUT_FEED" { FEED_SOURCE } else { DATABASE }
}

/// The instrument actually traded by a single-instrument BTST run: (AmiBroker symbol, description). src 1 and 3 read the
/// IEOD_FO near-month continuous futures (n/NIFTY, b/BANKNIFTY = the team's NIFTY_F1 / BANKNIFTY_F1, price-exact on
/// their 456 and 473 trade lists); src 2 the 2019-2026 rolling futures feed.
pub fn instrument(ticker: &str, src: i64) -> (&'static str, &'static str) {
    let bank = ticker == "NIFTYBANK";
    match (src, bank) {
        (2, _) => ("NIFTY_FUT_FEED", "Nifty futures, 2019-2026 rolling feed (not IEOD)"),
        (3, false) => ("NIFTY_F1", "Nifty near-month futures, continuous (IEOD_FO database, updated till 07-02-2023), as the team's 2021 copy"),
        (3, true) => ("BANKNIFTY_F1", "Bank Nifty near-month futures, continuous (IEOD_FO database, updated till 07-02-2023), as the team's 2021 copy"),
        (_, false) => ("NIFTY_F1", "Nifty near-month futures, continuous (IEOD_FO database, updated till 07-02-2023)"),
        (_, true) => ("BANKNIFTY_F1", "Bank Nifty near-month futures, continuous (IEOD_FO database, updated till 07-02-2023)"),
    }
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    variant(id).map(|v| ("ytbtst", "BTST", v.label))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    use std::collections::BTreeMap;

    /// One synthetic day: its bars as (HHMM, open, high, low, close); an empty day has no bars at all.
    type Day = Vec<(u32, f64, f64, f64, f64)>;

    /// A day with only the last bar (1525) (open = high = low = close = c).
    fn flat(c: f64) -> Day {
        vec![(1525, c, c, c, c)]
    }
    /// Signal day for last-bar close p: day IH = p + 1.5, IL = p - 3.5, so IBS at the 1515 bar is 1.5 / 5 = 0.3.
    /// The 1525 bar closes at p + 0.1 (the day's daily close).
    fn sig(p: f64) -> Day {
        vec![
            (915, p - 1.0, p + 1.5, p - 3.5, p - 0.5),
            (1515, p - 0.5, p + 0.5, p - 1.0, p),
            (1525, p, p + 0.2, p - 0.2, p + 0.1),
        ]
    }
    /// The next morning: only the 0920 bar.
    fn morning(o: f64, h: f64, l: f64, c: f64) -> Day {
        vec![(920, o, h, l, c)]
    }

    fn civil(z: i64) -> String {
        let z = z + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{:04}-{:02}-{:02}", yoe + era * 400 + i64::from(m <= 2), m, d)
    }
    fn rows(n: usize) -> Vec<Row> {
        let mut day = day_number("2024-01-01").unwrap(); // a Monday
        let mut out = Vec::new();
        while out.len() < n {
            if (day + 4).rem_euclid(7) % 6 != 0 {
                out.push(Row { ticker: "NIFTY 50".into(), date: civil(day), open: 100.0, high: 101.0, low: 99.0, close: 100.0, volume: None });
            }
            day += 1;
        }
        out
    }
    fn ps(kv: &[(&str, f64)]) -> Params {
        kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }
    /// Test defaults of the original suite: 2 lookback days, entry 15:15, exit 09:20, profit booking 1%, filter off, IBS
    /// and ROC open (they run on the IBS and ROC sub-variant, the core plus the two thresholds).
    fn base(kv: &[(&str, f64)]) -> Params {
        let mut p = ps(&[("days", 2.0), ("flt_type", 3.0), ("roc_thr", 0.0), ("ibs_thr", 1.0), ("entry_time", 1515.0), ("exit_time", 920.0), ("pb_pct", 1.0)]);
        p.extend(kv.iter().map(|(k, v)| (k.to_string(), *v)));
        p
    }
    /// The extra series of the days for both families: IEOD bar columns with D* derived from the bars (the day's own
    /// first open, highest high, lowest low and last close, as the generator compresses a day without other minutes),
    /// and the feed's bar columns (no D*).
    fn extra(days: &[Day]) -> BTreeMap<String, Vec<f64>> {
        let mut m: BTreeMap<String, Vec<f64>> = BTreeMap::new();
        let n = days.len();
        for (i, day) in days.iter().enumerate() {
            for &(t, o, h, l, c) in day {
                for fam in ["BTST_5M", "BTST_F5M"] {
                    for (k, v) in [("O", o), ("H", h), ("L", l), ("C", c)] {
                        m.entry(format!("{fam}.{k}{t:04}")).or_insert_with(|| vec![f64::NAN; n])[i] = v;
                    }
                }
            }
            let ok: Vec<_> = day.iter().filter(|b| [b.1, b.2, b.3, b.4].iter().all(|v| v.is_finite())).collect();
            if let (Some(f), Some(z)) = (ok.first(), ok.last()) {
                let hi = ok.iter().map(|b| b.2).fold(f64::MIN, f64::max);
                let lo = ok.iter().map(|b| b.3).fold(f64::MAX, f64::min);
                for (k, v) in [("DO", f.1), ("DH", hi), ("DL", lo), ("DC", z.4)] {
                    m.entry(format!("BTST_5M.{k}")).or_insert_with(|| vec![f64::NAN; n])[i] = v;
                }
            }
        }
        m
    }
    fn ctx_of<'a>(r: &'a [Row], days: &[Day]) -> Ctx<'a> {
        let mut x = Ctx::new(r, r, &[]);
        x.extra = extra(days);
        x
    }
    fn trades(days: &[Day], to: usize, p: &Params) -> Vec<Trade> {
        trades_of(ID, days, to, p)
    }
    fn ibs_roc(days: &[Day], to: usize, p: &Params) -> Vec<Trade> {
        trades_of("ytbtst_ibs_roc", days, to, p)
    }
    /// Four neutral bars 0920 to 0935 (all 105) after the first bar, so that the ROC base (5 bars before a 1515 entry)
    /// lies on the same day.
    fn padded(mut day: Day) -> Day {
        for (j, t) in [920, 925, 930, 935].into_iter().enumerate() {
            day.insert(1 + j, (t, 105.0, 105.0, 105.0, 105.0));
        }
        day
    }
    fn trades_of(id: &str, days: &[Day], to: usize, p: &Params) -> Vec<Trade> {
        let r = rows(days.len());
        let x = ctx_of(&r, days);
        run(id, &x, 0, to, p).unwrap()
    }
    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }
    fn key(t: &Trade) -> (usize, f64, usize, f64, String) {
        (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.reason.clone())
    }

    /// History closes 100, 102, 104, then the signal day (bar P = 108.5 beats 104 + diff 2 = 106) and the next morning.
    fn standard(sig_day: Day, next: Day) -> Vec<Day> {
        vec![flat(100.0), flat(102.0), flat(104.0), sig_day, next]
    }

    #[test]
    fn bar_grid_follows_the_loaded_columns_of_the_family() {
        let r = rows(2);
        let mut d = vec![vec![(900, 1.0, 1.0, 1.0, 1.0), (1530, 1.0, 1.0, 1.0, 1.0), (1610, 1.0, 1.0, 1.0, 1.0)], vec![(915, 1.0, 1.0, 1.0, 1.0)]];
        d[1].push((1525, 1.0, 1.0, 1.0, 1.0));
        let x = ctx_of(&r, &d);
        assert_eq!(bar_times(&x, 1), vec![900, 915, 1525, 1530, 1610]);
        assert_eq!(bar_times(&x, 2), vec![915, 1525]); // the feed's 1530 print and anything outside 0915..1525 are never read
        assert_eq!((family(1), family(2)), ("BTST_5M", "BTST_F5M"));
        assert_eq!((src_of(&Params::new()), src_of(&ps(&[("src", 2.0)])), src_of(&ps(&[("src", f64::NAN)]))), (1, 2, 0));
        assert_eq!(required_series(&Params::new()), ["BTST_5M.DC"]);
        assert_eq!(required_series(&ps(&[("src", 2.0)])), ["BTST_F5M.C1525"]);
    }

    /// n = 2: C[d] 104 > 102, P 108.5 > 104 and 108.5 - 104 = 4.5 > DIFF 2. Time exit at the 0920 close.
    #[test]
    fn lookback_two_enters_and_time_exits_next_morning() {
        let d = standard(sig(108.5), morning(108.0, 109.0, 107.0, 108.2));
        let t = trades(&d, 4, &base(&[]));
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px, t[0].open), (3, 108.5, 4, 108.2, false));
        assert_eq!((t[0].reason.as_str(), t[0].dir, t[0].entry_kind, t[0].signal_i), ("Time exit 0920", 1.0, "close", 4));
    }

    /// n = 3 needs two rises: 102 -> 101 -> 104 fails it while n = 2 passes; 100 -> 102 -> 105 -> 109 passes it.
    #[test]
    fn lookback_three_needs_two_rises() {
        let mk = |h: [f64; 3], p: f64| vec![flat(h[0]), flat(h[1]), flat(h[2]), sig(p), morning(p, p + 1.0, p - 1.0, p)];
        let d = mk([102.0, 101.0, 104.0], 108.5);
        assert_eq!(trades(&d, 4, &base(&[])).len(), 1);
        assert!(trades(&d, 4, &base(&[("days", 3.0)])).is_empty());
        let d = mk([100.0, 102.0, 105.0], 108.5);
        assert_eq!(trades(&d, 4, &base(&[("days", 3.0)])).len(), 1);
    }

    /// The change terms: n = 3 needs DIFF[d] > DIFF[d-1], strictly.
    #[test]
    fn incremental_difference_condition() {
        let mk = |h: [f64; 3]| vec![flat(h[0]), flat(h[1]), flat(h[2]), sig(112.0), morning(112.0, 113.0, 111.0, 112.0)];
        let n3 = base(&[("days", 3.0)]);
        assert_eq!(trades(&mk([100.0, 102.0, 105.0]), 4, &n3).len(), 1); // diffs 2, 3
        assert!(trades(&mk([100.0, 103.0, 105.0]), 4, &n3).is_empty()); // diffs 3, 2
        assert!(trades(&mk([100.0, 102.0, 104.0]), 4, &n3).is_empty()); // equal is not larger
        assert_eq!(trades(&mk([100.0, 103.0, 105.0]), 4, &base(&[])).len(), 1); // n = 2 has no change term
    }

    /// P must exceed the last close, and by more than the last daily change (104 + 2 = 106).
    #[test]
    fn price_must_beat_last_close_by_more_than_last_change() {
        for (p, n) in [(104.0, 0), (105.5, 0), (106.0, 0), (106.5, 1)] {
            let d = standard(sig(p), morning(p, p + 1.0, p - 1.0, p));
            assert_eq!(trades(&d, 4, &base(&[])).len(), n, "P = {p}");
            assert_eq!(trades_of(ID, &d, 4, &base(&[])).len(), n, "core P = {p}");
        }
    }

    /// IBS = (108 - P) / (108 - 103) with P = 106.5: 0.3. ibs_dir 1: 0.3 <= 0.3 passes, 0.29 fails; ibs_dir 2 the other way.
    #[test]
    fn ibs_threshold_both_directions() {
        let day = padded(vec![(915, 104.0, 108.0, 103.0, 105.0), (1515, 106.0, 107.0, 105.5, 106.5), (1525, 106.5, 107.0, 106.0, 106.6)]);
        let d = standard(day, morning(106.0, 107.0, 105.0, 106.2));
        assert_eq!(ibs_roc(&d, 4, &base(&[("ibs_thr", 0.3)])).len(), 1);
        assert!(ibs_roc(&d, 4, &base(&[("ibs_thr", 0.29)])).is_empty());
        assert_eq!(ibs_roc(&d, 4, &base(&[("ibs_thr", 0.5)])).len(), 1);
        assert_eq!(ibs_roc(&d, 4, &base(&[("ibs_dir", 2.0), ("ibs_thr", 0.3)])).len(), 1); // 0.3 >= 0.3
        assert_eq!(ibs_roc(&d, 4, &base(&[("ibs_dir", 2.0), ("ibs_thr", 0.29)])).len(), 1);
        assert!(ibs_roc(&d, 4, &base(&[("ibs_dir", 2.0), ("ibs_thr", 0.31)])).is_empty());
        // IH = IL (all bars flat at 108) gives no trade in either direction; bars after the entry bar do not count
        let flat_day: Day = [915, 920, 925, 930, 935, 1515].iter().map(|&t| (t, 108.0, 108.0, 108.0, 108.0)).collect();
        assert!(ibs_roc(&standard(flat_day.clone(), morning(108.0, 109.0, 107.0, 108.0)), 4, &base(&[])).is_empty());
        assert!(ibs_roc(&standard(flat_day.clone(), morning(108.0, 109.0, 107.0, 108.0)), 4, &base(&[("ibs_dir", 2.0), ("ibs_thr", 0.0)])).is_empty());
        assert_eq!(trades(&standard(flat_day, morning(108.0, 109.0, 107.0, 108.0)), 4, &base(&[])).len(), 1); // the core has no IBS
        let late_high = padded(vec![(915, 104.0, 108.0, 103.0, 105.0), (1515, 106.0, 107.0, 105.5, 106.5), (1520, 106.5, 150.0, 106.0, 106.6)]);
        assert_eq!(ibs_roc(&standard(late_high, morning(106.0, 107.0, 105.0, 106.2)), 4, &base(&[("ibs_thr", 0.3)])).len(), 1);
    }

    /// Entry bar 1515 close 106.5. ROC days 2: base C[d] 104 -> 2.4038%; ROC days 3: base 102 -> 4.4118%. The signal day
    /// has 5 bars before the entry bar, so the base is read on the same day.
    #[test]
    fn roc_base_day_two_versus_three() {
        let day = vec![(915, 104.0, 108.0, 103.0, 105.0), (920, 105.0, 105.0, 105.0, 105.0), (925, 105.0, 105.0, 105.0, 105.0), (930, 105.0, 105.0, 105.0, 105.0),
            (935, 105.0, 105.0, 105.0, 105.0), (1515, 106.0, 107.0, 105.5, 106.5), (1525, 106.5, 107.0, 106.0, 106.6)];
        let d = standard(day, morning(106.0, 107.0, 105.0, 106.2));
        assert!(ibs_roc(&d, 4, &base(&[("roc_thr", 3.0)])).is_empty()); // roc_days 2: 2.40 < 3
        assert_eq!(ibs_roc(&d, 4, &base(&[("roc_thr", 3.0), ("roc_days", 3.0)])).len(), 1); // 4.41 >= 3
        assert_eq!(ibs_roc(&d, 4, &base(&[("roc_thr", 2.4)])).len(), 1);
        assert!(ibs_roc(&d, 4, &base(&[("roc_thr", 2.5)])).is_empty());
        assert!(ibs_roc(&d, 4, &base(&[("roc_thr", 4.5), ("roc_days", 3.0)])).is_empty());
        assert!(ibs_roc(&d, 4, &base(&[("roc_thr", 3.0), ("roc_days", 1.0)])).is_empty()); // ROC days below 2 are clamped to 2
    }

    /// Ref(dailyclose1, -5) literally: with only 2 bars before the entry bar the 5th bar back is on an earlier day. History
    /// days are single 1525 bars (each its day's last bar, where expandLast shows that day's own close).
    /// Closes 100, 102, 104 (rows 0-2), signal row 3 with bars 0915, 0920, 1515: the bars are row 0, row 1, row 2, 0915,
    /// 0920, 1515, so 5 bars back from 1515 is row 0's 1525 bar, the last bar of row 0 -> base 100 for ROC days 2
    /// (ROC 6.5%) instead of 104 (2.40%); ROC days 3 would need a day before row 0: no value, no trade.
    #[test]
    fn roc_base_five_bars_back_may_cross_into_earlier_days() {
        let day = vec![(915, 104.0, 108.0, 103.0, 105.0), (920, 105.0, 105.0, 105.0, 105.0), (1515, 106.0, 107.0, 105.5, 106.5)];
        let d = standard(day, morning(106.0, 107.0, 105.0, 106.2));
        assert_eq!(ibs_roc(&d, 4, &base(&[("roc_thr", 6.0)])).len(), 1);
        assert!(ibs_roc(&d, 4, &base(&[("roc_thr", 6.6)])).is_empty());
        assert!(ibs_roc(&d, 4, &base(&[("roc_thr", 0.0), ("roc_days", 3.0)])).is_empty());
        // with fewer than 5 bars in the whole history there is no ROC value: no trade (the core still trades)
        // (rows 0-2 have one bar each and the entry bar is the day's first: only 3 bars precede it)
        let short = vec![flat(100.0), flat(102.0), flat(104.0), vec![(1515, 106.0, 107.0, 105.5, 106.5)], morning(106.0, 107.0, 105.0, 106.2)];
        assert!(ibs_roc(&short, 4, &base(&[("roc_thr", -100.0)])).is_empty());
        assert_eq!(trades(&short, 4, &base(&[])).len(), 1);
    }

    /// pb = 106.5 + 1.065 = 107.565.
    #[test]
    fn profit_booking_fills_at_target_on_the_entry_day() {
        let mut day = sig(106.5);
        day[1] = (1515, 106.0, 106.8, 105.5, 106.5);
        day.insert(2, (1520, 106.5, 108.0, 106.0, 107.0)); // high 108 > 107.565, open below it
        let t = trades(&standard(day, morning(106.0, 107.0, 105.0, 106.2)), 4, &base(&[]));
        assert_eq!(t.len(), 1);
        assert!(close(t[0].exit_px, 107.565) && t[0].exit_i == 3 && t[0].legs[0].i == 3 && !t[0].open);
        assert_eq!(t[0].reason, "Profit booking 1% at 1520");
        let mut day = sig(106.5);
        day[2] = (1525, 106.5, 107.565, 106.0, 106.6); // exactly at the target does not book (strict >)
        let d = standard(day, morning(106.0, 107.0, 105.0, 106.2));
        assert_eq!(trades(&d, 4, &base(&[])).iter().map(|t| t.exit_i).collect::<Vec<_>>(), vec![4]);
        let mut day = sig(106.5);
        day.insert(2, (1520, 106.5, 120.0, 106.0, 107.0));
        let t = trades(&standard(day, morning(106.0, 107.0, 105.0, 106.2)), 4, &base(&[("pb_pct", 0.0)]));
        assert_eq!((t[0].exit_i, t[0].exit_px), (4, 106.2));
    }

    /// The entry bar's own high is checked (AFL AllowSameBarExit): bar high 108 > 107.565 books at 107.565.
    #[test]
    fn profit_booking_on_the_entry_bar_itself() {
        let mut day = sig(106.5);
        day[1] = (1515, 106.0, 108.0, 105.5, 106.5);
        let t = trades(&standard(day, morning(106.0, 107.0, 105.0, 106.2)), 4, &base(&[]));
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (3, 3));
        assert!(close(t[0].exit_px, 107.565));
        assert_eq!(t[0].reason, "Profit booking 1% at 1515");
    }

    /// Next morning opens at 109 above the target: fill 109 - 0.25% = 108.7275.
    #[test]
    fn profit_booking_at_a_gapped_open_pays_open_less_a_quarter_percent() {
        let t = trades(&standard(sig(106.5), morning(109.0, 110.0, 108.5, 109.5)), 4, &base(&[]));
        assert!(close(t[0].exit_px, 109.0 - 109.0 * 0.25 / 100.0) && t[0].exit_i == 4);
        assert_eq!(t[0].reason, "Profit booking 1% at 0920");
    }

    /// Hard stop 1%: hs = 106.5 - 1.065 = 105.435.
    #[test]
    fn hard_stop_at_level_and_at_gapped_open() {
        let hs = base(&[("hardstop", 1.0)]);
        let t = trades(&standard(sig(106.5), morning(106.0, 106.5, 105.0, 105.5)), 4, &hs);
        assert!(close(t[0].exit_px, 105.435) && t[0].exit_i == 4);
        assert_eq!(t[0].reason, "Hard stop 1% at 0920");
        let t = trades(&standard(sig(106.5), morning(105.0, 105.5, 104.0, 104.5)), 4, &hs);
        assert!(close(t[0].exit_px, 105.0 - 105.0 * 0.25 / 100.0));
        let t = trades(&standard(sig(106.5), morning(106.0, 106.5, 105.435, 106.0)), 4, &hs);
        assert_eq!(t[0].reason, "Time exit 0920");
        let t = trades(&standard(sig(106.5), morning(105.0, 105.5, 104.0, 104.5)), 4, &base(&[]));
        assert_eq!(t[0].exit_px, 104.5);
        let t = trades(&standard(sig(106.5), morning(106.0, 108.0, 105.0, 106.0)), 4, &hs);
        assert!(close(t[0].exit_px, 107.565)); // profit booking first when one bar reaches both levels
    }

    /// The exit is the first bar from the exit time on the next trading day, at that bar's close; a missing 0920 bar falls
    /// through to the next present one.
    #[test]
    fn time_exit_uses_the_first_present_bar_from_the_exit_time() {
        let next = vec![(915, 106.0, 106.5, 105.5, 106.0), (925, 106.5, 107.0, 106.0, 106.9), (930, 106.0, 106.5, 105.5, 106.1)];
        let t = trades(&standard(sig(106.5), next.clone()), 4, &base(&[]));
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (4, 106.9, "Time exit 0925"));
        let t = trades(&standard(sig(106.5), next), 4, &base(&[("exit_time", 930.0)]));
        assert_eq!(t[0].exit_px, 106.1);
    }

    /// Before 2010 the session opened at 09:55: with exit 09:25 the first bar from the exit time is the 09:55 bar.
    #[test]
    fn exit_on_a_0955_session_open() {
        let next = vec![(955, 108.0, 108.5, 107.5, 108.3), (1000, 108.3, 108.9, 108.0, 108.8)];
        let d = standard(sig(108.5), next);
        for id in [ID, "ytbtst_ath"] {
            let t = trades_of(id, &d, 4, &base(&[("exit_time", 925.0), ("pb_pct", 0.0)]));
            assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (4, 108.3, "Time exit 0955"), "{id}");
        }
        // an exit time after the 09:55 open still takes the first bar from it
        let t = trades_of(ID, &standard(sig(108.5), vec![(955, 108.0, 108.5, 107.5, 108.3), (1000, 108.3, 108.9, 108.0, 108.8)]), 4, &base(&[("exit_time", 955.0), ("pb_pct", 0.0)]));
        assert_eq!(t[0].exit_px, 108.3);
    }

    #[test]
    fn weekend_gap_between_signal_and_exit_day() {
        let d: Vec<Day> = vec![flat(100.0), flat(101.0), flat(102.0), flat(104.0), sig(108.5), morning(108.0, 109.0, 107.0, 108.2)];
        let r = rows(6);
        assert_eq!((r[4].date.as_str(), r[5].date.as_str()), ("2024-01-05", "2024-01-08"));
        let t = trades(&d, 5, &base(&[]));
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (4, 5));
    }

    /// Open at the end of the window: marked at the last bar close of the last day; no lookahead into later bars.
    #[test]
    fn open_trade_at_window_end_and_no_lookahead() {
        let d = standard(sig(108.5), morning(108.0, 109.0, 107.0, 108.2));
        let t = trades(&d, 3, &base(&[]));
        assert_eq!(t.len(), 1);
        assert!(t[0].open && t[0].reason == "open" && t[0].exit_i == 3 && t[0].signal_i == 3);
        assert!(close(t[0].exit_px, 108.6));
        let mut late = sig(108.5);
        late[2] = (1525, 108.5, 109.0, 1.0, 1.0);
        let e = trades(&standard(late, morning(108.0, 109.0, 107.0, 108.2)), 4, &base(&[]));
        assert_eq!((e[0].legs[0].i, e[0].legs[0].px), (3, 108.5));
    }

    /// Missing bars are NaN: no entry bar, no trade; a bar missing a field is skipped; a day without data is not a day.
    #[test]
    fn missing_bars_are_skipped_and_never_filled() {
        let mut day = sig(108.5);
        day.remove(1); // no 1515 bar
        assert!(trades(&standard(day, morning(108.0, 109.0, 107.0, 108.2)), 4, &base(&[])).is_empty());
        let next = vec![(920, 108.0, 109.0, 107.0, f64::NAN), (925, 108.0, 108.4, 107.5, 108.1)];
        let t = trades(&standard(sig(108.5), next), 4, &base(&[]));
        assert_eq!((t[0].exit_px, t[0].reason.as_str()), (108.1, "Time exit 0925"));
        let day_with_gap = vec![(915, 104.0, 108.0, 103.0, 105.0), (1515, 106.0, 107.0, 105.5, 106.5)];
        assert_eq!(trades(&standard(day_with_gap, morning(106.0, 107.0, 105.0, 106.2)), 4, &base(&[])).len(), 1);
    }

    /// A calendar day without any data (no D* and no bars) is skipped as "the previous day", on both sources.
    #[test]
    fn missing_days_are_skipped_on_both_sources() {
        let d = vec![flat(100.0), flat(102.0), Vec::new(), flat(104.0), sig(108.5), morning(108.0, 109.0, 107.0, 108.2)];
        for src in [1.0, 2.0] {
            let t = trades_of(ID, &d, 5, &base(&[("src", src)]));
            assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(4, 5)], "src {src}");
            assert!(trades_of(ID, &d, 2, &base(&[("src", src)])).is_empty());
        }
    }

    /// src 1 reads the daily bar from D* (all minutes), src 2 builds it from the bars. The D* close of row 2 is 101
    /// (minutes after the last bar on the grid moved it), so on src 1 the history 100, 102, 101 has no rise and no trade,
    /// while src 2 sees 104 and trades. And src 2 never reads the BTST_5M columns, src 1 never the BTST_F5M ones.
    #[test]
    fn src_switches_the_series_family_and_the_daily_bars() {
        let d = standard(sig(108.5), morning(108.0, 109.0, 107.0, 108.2));
        let r = rows(5);
        let mut x = ctx_of(&r, &d);
        assert_eq!(run(ID, &x, 0, 4, &base(&[])).unwrap().len(), 1);
        x.extra.get_mut("BTST_5M.DC").unwrap()[2] = 101.0;
        assert!(run(ID, &x, 0, 4, &base(&[])).unwrap().is_empty());
        assert_eq!(run(ID, &x, 0, 4, &base(&[("src", 2.0)])).unwrap().len(), 1);
        // remove the feed: src 2 has nothing, src 1 is unchanged
        x.extra.retain(|k, _| !k.starts_with("BTST_F5M."));
        assert!(run(ID, &x, 0, 4, &base(&[("src", 2.0)])).unwrap().is_empty());
        x.extra.get_mut("BTST_5M.DC").unwrap()[2] = 104.0;
        assert_eq!(run(ID, &x, 0, 4, &base(&[])).unwrap().len(), 1);
        // an invalid source gives no trades (readiness reports it)
        assert!(run(ID, &x, 0, 4, &base(&[("src", 4.0)])).unwrap().is_empty());
        assert_eq!(unsupported(ID, &ps(&[("src", 4.0)])).unwrap().0, "not_testable");
        // src 3 (team copy) reads the IEOD family too; in 2024 nothing is dropped, so it trades like src 1
        assert_eq!(run(ID, &x, 0, 4, &base(&[("src", 3.0)])).unwrap().len(), 1);
        assert!(unsupported(ID, &ps(&[("src", 2.0)])).is_none() && unsupported(ID, &Params::new()).is_none());
    }

    /// A day with only D* and no bar on the grid (a defensive case; Muhurat sessions now have grid bars 18:00..19:30):
    /// it is a day for the daily rules on src 1 but has no intraday bar, so it neither enters nor exits; the trade exits on the next day with bars.
    #[test]
    fn evening_session_days_count_as_days_but_have_no_bars() {
        // rows: 100, 102, evening day closing 104 (D* only), signal day, next morning
        let d = vec![flat(100.0), flat(102.0), Vec::new(), sig(108.5), Vec::new(), morning(108.0, 109.0, 107.0, 108.2)];
        let r = rows(6);
        let mut x = ctx_of(&r, &d);
        for (k, v) in [("DO", 103.0), ("DH", 104.5), ("DL", 102.5), ("DC", 104.0)] {
            x.extra.get_mut(&format!("BTST_5M.{k}")).unwrap()[2] = v;
        }
        for (k, v) in [("DO", 108.0), ("DH", 108.0), ("DL", 108.0), ("DC", 108.0)] {
            x.extra.get_mut(&format!("BTST_5M.{k}")).unwrap()[4] = v; // an evening day between signal and exit
        }
        let t = run(ID, &x, 0, 5, &base(&[])).unwrap();
        assert_eq!(t.iter().map(key).collect::<Vec<_>>(), vec![(3, 108.5, 5, 108.2, "Time exit 0920".to_string())]);
        // on the feed the evening day does not exist: history 100, 102 only, so the signal falls on the feed's 3rd data
        // day, where buycond2 is still Null (no trade)
        let t = run(ID, &x, 0, 5, &base(&[("src", 2.0)])).unwrap();
        assert!(t.is_empty());
        // with the evening close at 101 the IEOD history has no rise: no trade on src 1
        x.extra.get_mut("BTST_5M.DC").unwrap()[2] = 101.0;
        assert!(run(ID, &x, 0, 5, &base(&[])).unwrap().is_empty());
    }

    /// A Muhurat evening session (bars 18:15, 18:20, ...) is an ordinary day of bars: the old fixed AFL exits at its 2nd
    /// candle (team list NIFTY-TL 2013-11-01 -> 2013-11-03 18:20), profit booking may fire there, and the time exit does not
    /// (18:15 is not before the entry time).
    #[test]
    fn muhurat_evening_bars_are_ordinary_bars() {
        let ev = vec![(1815, 107.0, 107.5, 106.5, 107.2), (1820, 107.2, 107.9, 107.0, 107.6), (1825, 107.6, 108.0, 107.5, 107.9)];
        let next = vec![(915, 108.0, 108.5, 107.5, 108.1), (920, 108.1, 108.6, 107.9, 108.4)];
        // old fixed (ver3): entry candle 2 from the close = the signal day's 15:15, exit candle 2 of the evening = 18:20
        let s = padded(sig(106.0));
        let d = vec![flat(100.0), flat(102.0), flat(104.0), s.clone(), ev.clone(), next.clone()];
        let t = trades_of("ytbtst_old_fixed", &d, 5, &ps(&[("entry_n", 2.0)]));
        assert_eq!(t.iter().map(key).collect::<Vec<_>>(), vec![(3, 106.0, 4, 107.6, "Exit candle 2 at 1820".to_string())]);
        // core: no time exit on the evening day (18:15 >= entry 15:15), exit next morning at 09:20 ...
        let d = vec![flat(100.0), flat(102.0), flat(104.0), sig(108.5), ev.clone(), next.clone()];
        let t = trades(&d, 5, &base(&[("pb_pct", 0.0)]));
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (5, 108.4, "Time exit 0920"));
        // ... but profit booking fires on an evening bar: pb 0.5% = 109.0425, the 18:20 high 109.5 books it
        let mut ev2 = ev;
        ev2[1] = (1820, 107.2, 109.5, 107.0, 109.0);
        let d = vec![flat(100.0), flat(102.0), flat(104.0), sig(108.5), ev2, next];
        let t = trades(&d, 5, &base(&[("pb_pct", 0.5)]));
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (4, "Profit booking 0.5% at 1820"));
        assert!(close(t[0].exit_px, 108.5 * 1.005));
    }

    /// One position at a time: a second qualifying 1515 bar while the first trade is still open is ignored.
    #[test]
    fn one_position_at_a_time() {
        let hold = base(&[("pb_pct", 0.0)]);
        let d4 = vec![(1515, 113.0, 114.0, 112.5, 113.5), (1525, 113.5, 113.6, 113.4, 113.5)];
        let d = vec![flat(100.0), flat(102.0), flat(104.0), sig(108.5), d4.clone(), morning(113.0, 114.0, 112.0, 113.2)];
        let t = trades(&d, 5, &hold);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (3, 5, 113.2));
        let mut d4b = d4;
        d4b.insert(0, (920, 109.0, 109.5, 108.5, 109.2));
        let d = vec![flat(100.0), flat(102.0), flat(104.0), sig(108.5), d4b, morning(113.0, 114.0, 112.0, 113.2)];
        let t = trades(&d, 5, &hold);
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(3, 4), (4, 5)]);
    }

    /// Entry and exit times must be on the 5-minute grid inside the ranges; anything else gives no trades.
    #[test]
    fn exit_and_entry_time_grid_validity() {
        let d = standard(sig(108.5), morning(108.0, 109.0, 107.0, 108.2));
        assert_eq!(trades(&d, 4, &base(&[])).len(), 1);
        for (k, v) in [("entry_time", 1460.0), ("entry_time", 1517.0), ("entry_time", 1330.0), ("entry_time", 1525.0), ("entry_time", 1599.0),
            ("exit_time", 960.0), ("exit_time", 922.0), ("exit_time", 1000.0), ("exit_time", 905.0)] {
            assert!(trades(&d, 4, &base(&[(k, v)])).is_empty(), "{k} = {v}");
        }
        let day = vec![(915, 104.0, 108.0, 103.0, 105.0), (1500, 106.0, 107.0, 105.5, 106.5), (1525, 106.5, 107.0, 106.0, 106.6)];
        let t = trades(&standard(day, morning(106.0, 107.0, 105.0, 106.2)), 4, &base(&[("entry_time", 1500.0)]));
        assert_eq!(t[0].legs[0].px, 106.5);
    }

    fn fcloses(h: &[f64]) -> Vec<Day> {
        h.iter().map(|&c| flat(c)).collect()
    }

    /// Daily SMA(2): the filter compares the last completed close with the SMA of the bar before it.
    #[test]
    fn sma_filter_gates_the_entry() {
        let sma2 = base(&[("flt_type", 2.0), ("sma_period", 2.0)]);
        let mut d = fcloses(&[100.0, 102.0, 104.0, 106.0]);
        d.extend([sig(112.0), morning(112.0, 113.0, 111.0, 112.0)]);
        assert_eq!(trades(&d, 5, &sma2).len(), 1);
        let mut d = fcloses(&[120.0, 110.0, 100.0, 101.0]);
        d.extend([sig(104.0), morning(104.0, 105.0, 103.0, 104.0)]);
        assert_eq!(trades(&d, 5, &base(&[])).len(), 1);
        assert!(trades(&d, 5, &sma2).is_empty());
        let d = vec![flat(100.0), flat(102.0), flat(104.0), sig(108.5), morning(108.0, 109.0, 107.0, 108.2)];
        assert!(trades(&d, 4, &base(&[("flt_type", 2.0), ("sma_period", 5.0)])).is_empty());
    }

    /// Supertrend on closes, period 2, factor 1 (bars are single prices, so TR = |change|).
    #[test]
    fn supertrend_filter_gates_the_entry() {
        let st = base(&[("flt_type", 1.0), ("st_period", 2.0), ("st_point", 2.0)]);
        let mut up = fcloses(&[100.0, 102.0, 104.0, 106.0]);
        up.extend([sig(112.0), morning(112.0, 113.0, 111.0, 112.0)]);
        assert_eq!(trades(&up, 5, &st).len(), 1);
        let mut down = fcloses(&[106.0, 104.0, 102.0, 100.0, 101.0]);
        down.extend([sig(104.0), morning(104.0, 105.0, 103.0, 104.0)]);
        assert_eq!(trades(&down, 6, &base(&[])).len(), 1);
        assert!(trades(&down, 6, &st).is_empty());
        let bars: Vec<Ohlc> = [106.0, 104.0, 102.0, 100.0, 101.0].iter().map(|&c| Ohlc { o: c, h: c, l: c, c }).collect();
        assert_eq!(supertrend_up(&bars, 2, 2, 1.0), vec![true, true, false, false, false]);
        let bars: Vec<Ohlc> = [100.0, 102.0, 104.0, 106.0].iter().map(|&c| Ohlc { o: c, h: c, l: c, c }).collect();
        assert_eq!(supertrend_up(&bars, 2, 2, 1.0), vec![true; 4]);
        let bars: Vec<Ohlc> = [106.0, 104.0, 102.0, 100.0, 101.0, 110.0].iter().map(|&c| Ohlc { o: c, h: c, l: c, c }).collect();
        assert!(supertrend_up(&bars, 2, 2, 1.0)[5]);
    }

    /// Weekly SMA(1): only the last COMPLETED week counts. Rows 0-4 = week 1, 5-9 = week 2, 10-11 = Mon/Tue of week 3.
    #[test]
    fn weekly_filter_uses_the_last_completed_week() {
        let weeks = |w1: f64, w2: f64, mon: f64, tue: f64| {
            let mut d = fcloses(&[w1, w1, w1, w1, w1, w2, w2, w2, w2, w2, mon, tue]);
            d.extend([sig(tue + 6.0), morning(tue + 6.0, tue + 7.0, tue + 5.0, tue + 6.0)]);
            d
        };
        let wk = |tf: f64| base(&[("flt_type", 2.0), ("sma_period", 1.0), ("flt_tf", tf)]);
        let up = weeks(100.0, 110.0, 111.0, 112.0);
        assert_eq!(trades(&up, 13, &wk(2.0)).len(), 1);
        let down = weeks(100.0, 90.0, 91.0, 92.0);
        assert_eq!(trades(&down, 13, &wk(1.0)).len(), 1);
        assert!(trades(&down, 13, &wk(2.0)).is_empty());
        assert!(trades(&up, 13, &wk(3.0)).is_empty());
        assert_eq!(trades(&up, 13, &base(&[("flt_type", 3.0), ("flt_tf", 3.0)])).len(), 1);
    }

    #[test]
    fn periods_group_weeks_and_months() {
        let r = rows(12);
        let x = Ctx::new(&r, &r, &[]);
        let idx: Vec<usize> = (0..12).collect();
        assert_eq!(periods(&x, &idx, 2), vec![(0, 4), (5, 9), (10, 11)]);
        assert_eq!(periods(&x, &idx, 3), vec![(0, 11)]);
        assert_eq!(periods(&x, &idx, 1).len(), 12);
    }

    /// Lookback 4: three rises and DIFF[d] > DIFF[d-1] > DIFF[d-2].
    #[test]
    fn lookback_four_needs_three_rises_and_increasing_changes() {
        let mk = |h: [f64; 4]| {
            let mut d = fcloses(&h);
            d.extend([sig(112.0), morning(112.0, 113.0, 111.0, 112.0)]);
            d
        };
        let n4 = base(&[("days", 4.0)]);
        assert_eq!(trades(&mk([100.0, 101.0, 103.0, 106.0]), 5, &n4).len(), 1);
        assert!(trades(&mk([100.0, 101.0, 103.0, 105.0]), 5, &n4).is_empty());
        assert!(trades(&mk([100.0, 103.0, 104.0, 106.0]), 5, &n4).is_empty());
        assert_eq!(trades(&mk([100.0, 103.0, 104.0, 106.0]), 5, &base(&[("days", 3.0)])).len(), 1);
        assert!(trades(&mk([100.0, 98.0, 103.0, 108.0]), 5, &n4).is_empty());
    }

    /// The feed's stray 1530 print is never read on src 2; on src 1 (IEOD) 1530 is a real bar (after-hours minutes) and
    /// books profit, while the daily close comes from D*.
    #[test]
    fn the_1530_bar_is_ignored_on_the_feed_only() {
        let mut d = standard(sig(108.5), morning(108.0, 109.0, 107.0, 108.2));
        d[2].push((1530, 90.0, 90.0, 90.0, 90.0)); // on the feed this would break C[d] > C[d-1] if read
        d[3].push((1530, 108.6, 200.0, 108.0, 150.0)); // would book profit at the target
        let r = rows(5);
        let mut x = ctx_of(&r, &d);
        let t = run(ID, &x, 0, 4, &base(&[("src", 2.0)])).unwrap();
        assert_eq!((t.len(), t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (1, 3, 4, 108.2));
        x.extra.get_mut("BTST_5M.DC").unwrap()[2] = 104.0; // IEOD daily close of row 2 from all its minutes
        let t = run(ID, &x, 0, 4, &base(&[])).unwrap();
        assert_eq!((t.len(), t[0].legs[0].i, t[0].exit_i, t[0].reason.as_str()), (1, 3, 3, "Profit booking 1% at 1530"));
        assert!(close(t[0].exit_px, 108.5 * 1.01));
    }

    // ------------------------------------------------------------------ sub-variant rules

    /// Core = the IBS and ROC sub-variant with both thresholds open; its defaults are 2 / 14:50 / 09:25 / pb 0 / src 1.
    #[test]
    fn core_defaults_and_equivalence() {
        let day = padded(vec![(915, 104.0, 110.0, 103.0, 105.0), (1450, 106.0, 109.0, 105.5, 108.5), (1525, 108.5, 109.0, 108.0, 108.6)]);
        let d = standard(day, vec![(925, 108.0, 109.0, 107.0, 108.2)]);
        let a = trades_of(ID, &d, 4, &Params::new());
        assert_eq!(a.iter().map(key).collect::<Vec<_>>(), vec![(3, 108.5, 4, 108.2, "Time exit 0925".to_string())]);
        let open = ps(&[("ibs_thr", 1.0), ("roc_thr", -100.0)]);
        assert_eq!(trades_of("ytbtst_ibs_roc", &d, 4, &open).iter().map(key).collect::<Vec<_>>(), a.iter().map(key).collect::<Vec<_>>());
        // the IBS and ROC defaults (IBS <= 0.5, ROC >= 1%): IBS = (110 - 108.5) / 7 = 0.214 passes, ROC = 4.33% passes
        assert_eq!(trades_of("ytbtst_ibs_roc", &d, 4, &Params::new()).len(), 1);
        // IBS >= 0.5 (earlier AFL) blocks it
        assert!(trades_of("ytbtst_ibs_roc", &d, 4, &ps(&[("ibs_dir", 2.0)])).is_empty());
    }

    /// ATH: P must beat the highest close of ALL completed days. History 110, 100, 102, 104: the up-close rules pass
    /// (P 108.5), but 108.5 < 110 blocks; P 111 passes. The ATH-only variant needs no up-close rules at all.
    #[test]
    fn all_time_high_close_with_and_without_up_close() {
        let mk = |p: f64| vec![flat(110.0), flat(100.0), flat(102.0), flat(104.0), sig(p), morning(p, p + 0.5, p - 0.5, p)];
        let core = base(&[("pb_pct", 0.0)]);
        assert_eq!(trades_of(ID, &mk(108.5), 5, &core).len(), 1);
        assert!(trades_of("ytbtst_ath", &mk(108.5), 5, &core).is_empty());
        assert!(trades_of("ytbtst_ath", &mk(110.0), 5, &core).is_empty()); // strict >
        assert_eq!(trades_of("ytbtst_ath", &mk(111.0), 5, &core).len(), 1);
        // ATH only: falling history 110, 109, 108, 107 fails every up-close rule; P 110.5 > ATH 110 still buys
        let fall = |p: f64| vec![flat(110.0), flat(109.0), flat(108.0), flat(107.0), sig(p), morning(p, p + 0.5, p - 0.5, p)];
        assert!(trades_of(ID, &fall(110.5), 5, &core).is_empty());
        assert_eq!(trades_of("ytbtst_ath_only", &fall(110.5), 5, &core).len(), 1);
        assert!(trades_of("ytbtst_ath_only", &fall(110.0), 5, &core).is_empty());
        // the ATH comes from completed days only: today's later bars never count
        let mut today = sig(110.5);
        today[2] = (1525, 110.5, 300.0, 110.0, 300.0);
        let d = vec![flat(110.0), flat(109.0), flat(108.0), flat(107.0), today, morning(110.0, 110.5, 109.5, 110.0)];
        assert_eq!(trades_of("ytbtst_ath_only", &d, 5, &core).len(), 1);
        assert!(params("ytbtst_ath_only").unwrap().iter().all(|p| p.key != "days"));
    }

    /// Highest close of N days: closes 105, 100, 102, 104 then P 108.5. N 3 = max(100, 102, 104) = 104 passes;
    /// N 5 needs five completed days: with only four there is no value, no trade. History 109, 105, 100, 102, 104 and
    /// N 5: max 109 > 108.5 blocks.
    #[test]
    fn highest_close_of_n_days() {
        let d = vec![flat(105.0), flat(100.0), flat(102.0), flat(104.0), sig(108.5), morning(108.0, 109.0, 107.0, 108.2)];
        let hn = |n: f64| base(&[("pb_pct", 0.0), ("hdays", n)]);
        assert_eq!(trades_of("ytbtst_high_n", &d, 5, &hn(3.0)).len(), 1);
        assert!(trades_of("ytbtst_high_n", &d, 5, &hn(5.0)).is_empty());
        let d = vec![flat(109.0), flat(105.0), flat(100.0), flat(102.0), flat(104.0), sig(108.5), morning(108.0, 109.0, 107.0, 108.2)];
        assert!(trades_of("ytbtst_high_n", &d, 6, &hn(5.0)).is_empty());
        assert_eq!(trades_of("ytbtst_high_n", &d, 6, &hn(3.0)).len(), 1);
        let d = vec![flat(100.0), flat(102.0), flat(104.0), sig(108.5), morning(108.0, 109.0, 107.0, 108.2)];
        assert_eq!(trades_of("ytbtst_high_n", &d, 4, &hn(3.0)).len(), 1);
        let def = params("ytbtst_high_n").unwrap().into_iter().find(|p| p.key == "hdays").unwrap();
        assert_eq!((def.def, def.min, def.max, def.step), (3.0, 3.0, 15.0, 2.0));
    }

    /// Green candle: P above the day's first open, and the last green_days - 1 daily candles closed above their open.
    #[test]
    fn green_candle_today_and_previous_days() {
        let candle = |o: f64, c: f64| vec![(1525, o, o.max(c), o.min(c), c)];
        // history candles: 99 -> 100 (green), 101 -> 102 (green), 103 -> 104 (green); signal day first open 107.5 < P 108.5
        let mk = |prev_open: f64, first_open: f64| {
            let mut s = sig(108.5);
            s[0].1 = first_open;
            vec![candle(99.0, 100.0), candle(101.0, 102.0), candle(prev_open, 104.0), s, morning(108.0, 109.0, 107.0, 108.2)]
        };
        let g = |kv: &[(&str, f64)]| { let mut p = base(&[("pb_pct", 0.0)]); p.extend(kv.iter().map(|(k, v)| (k.to_string(), *v))); p };
        assert_eq!(trades_of("ytbtst_green", &mk(103.0, 107.5), 4, &g(&[])).len(), 1);
        assert!(trades_of("ytbtst_green", &mk(104.5, 107.5), 4, &g(&[])).is_empty()); // yesterday red (104.5 -> 104)
        assert!(trades_of("ytbtst_green", &mk(104.0, 107.5), 4, &g(&[])).is_empty()); // doji is not green (strict)
        assert!(trades_of("ytbtst_green", &mk(103.0, 108.5), 4, &g(&[])).is_empty()); // today's candle not green
        assert_eq!(trades_of("ytbtst_green", &mk(104.5, 108.5), 4, &g(&[("candle", 2.0)])).len(), 1); // ANY
        // green_days 4 = the last three daily candles; the first one 99 -> 100 green, make it red
        let mut d = mk(103.0, 107.5);
        d[0] = candle(100.5, 100.0);
        assert_eq!(trades_of("ytbtst_green", &d, 4, &g(&[("green_days", 3.0)])).len(), 1);
        assert!(trades_of("ytbtst_green", &d, 4, &g(&[("green_days", 4.0)])).is_empty());
        // green_days 5 needs four completed days: only three exist
        assert!(trades_of("ytbtst_green", &mk(103.0, 107.5), 4, &g(&[("green_days", 5.0)])).is_empty());
    }

    /// Volume: today's volume up to and including the entry bar > multiplier x MA(daily volume, N) of completed days.
    #[test]
    fn volume_above_the_average() {
        // five history days with DV 100, 200, 300, 400, 500 (closes rising), MA(5) = 300, MA(2) = 450
        let mut d = fcloses(&[96.0, 98.0, 100.0, 102.0, 104.0]);
        d.extend([vec![(915, 107.5, 110.0, 107.0, 108.0), (1450, 108.0, 109.0, 107.8, 108.5), (1455, 108.5, 109.0, 108.0, 108.6)], vec![(925, 108.0, 109.0, 107.0, 108.2)]]);
        let r = rows(7);
        let mut x = ctx_of(&r, &d);
        let dv: Vec<f64> = vec![100.0, 200.0, 300.0, 400.0, 500.0, 9999.0, 1.0];
        x.extra.insert("BTST_5M.DV".into(), dv);
        let set_v = |x: &mut Ctx, t: &str, row: usize, v: f64| x.extra.entry(format!("BTST_5M.V{t}")).or_insert_with(|| vec![f64::NAN; 7])[row] = v;
        set_v(&mut x, "0915", 5, 200.0);
        set_v(&mut x, "1450", 5, 100.0); // today so far = 300, not > 300
        set_v(&mut x, "1455", 5, 5000.0); // after the entry bar: never counted
        let vp = |kv: &[(&str, f64)]| { let mut p = ps(&[("vol_days", 5.0), ("vol_mult", 1.0)]); p.extend(kv.iter().map(|(k, v)| (k.to_string(), *v))); p };
        assert!(run("ytbtst_volume", &x, 0, 6, &vp(&[])).unwrap().is_empty());
        set_v(&mut x, "1450", 5, 101.0); // 301 > 300
        assert_eq!(run("ytbtst_volume", &x, 0, 6, &vp(&[])).unwrap().iter().map(key).collect::<Vec<_>>(), vec![(5, 108.5, 6, 108.2, "Time exit 0925".to_string())]);
        assert!(run("ytbtst_volume", &x, 0, 6, &vp(&[("vol_mult", 2.0)])).unwrap().is_empty()); // 301 < 600
        assert!(run("ytbtst_volume", &x, 0, 6, &vp(&[("vol_days", 2.0)])).unwrap().is_empty()); // 301 < 450
        assert!(run("ytbtst_volume", &x, 0, 6, &vp(&[("vol_days", 6.0)])).unwrap().is_empty()); // six completed days do not exist
        // the feed has no volume: no trades, and readiness says so
        assert!(run("ytbtst_volume", &x, 0, 6, &vp(&[("src", 2.0)])).unwrap().is_empty());
        assert_eq!(unsupported("ytbtst_volume", &ps(&[("src", 2.0)])).unwrap().0, "needs_data");
        assert!(unsupported("ytbtst_volume", &Params::new()).is_none());
        // the avgvol loop resets after every bar at or after the entry time: if the previous day had no such bar, its
        // volume carries into today's sum (the AFL reads it that way)
        x.extra.get_mut("BTST_5M.V1450").unwrap()[5] = 99.0; // today 299
        assert!(run("ytbtst_volume", &x, 0, 6, &vp(&[])).unwrap().is_empty());
        for k in ["O", "H", "L", "C"] {
            // row 4's only bar moves from 1525 to 1000 (before the entry time): its volume 7 carries over
            let v = x.extra.get_mut(&format!("BTST_5M.{k}1525")).unwrap()[4];
            x.extra.get_mut(&format!("BTST_5M.{k}1525")).unwrap()[4] = f64::NAN;
            x.extra.entry(format!("BTST_5M.{k}1000")).or_insert_with(|| vec![f64::NAN; 7])[4] = v;
        }
        set_v(&mut x, "1000", 4, 7.0);
        assert_eq!(run("ytbtst_volume", &x, 0, 6, &vp(&[])).unwrap().len(), 1); // 299 + 7 > 300
    }

    /// ATHC-swing BUYSIG on daily closes, exit 10%: on at a new highest close, off once the close falls below the
    /// trailing Dn (highest-close-era C x 0.9, ratcheted up), on again at the next new high.
    #[test]
    fn athc_swing_state() {
        let c = [100.0, 101.0, 105.0, 100.0, 94.0, 96.0, 104.0, 106.0];
        // trend: Up = Hcl (100, 101, 105, ...), Dn = 0.9 C ratcheted: 90, 90.9, 94.5, 94.5 (C 100 keeps the max), 94 < 94.5 -> down
        assert_eq!(athc_buysig(&c, 10.0), vec![false, true, true, true, false, false, false, true]);
        // a wider swing (20%) never turns it off
        assert_eq!(athc_buysig(&c, 20.0), vec![false, true, true, true, true, true, true, true]);
        // as a filter: history ending on a down-trend day blocks the up-close entry, a new-high history allows it
        let days = |h: &[f64], p: f64| { let mut d = fcloses(h); d.extend([sig(p), morning(p, p + 0.5, p - 0.5, p)]); d };
        let q = base(&[("pb_pct", 0.0)]);
        let up = days(&[100.0, 102.0, 104.0], 108.5);
        assert_eq!(trades_of("ytbtst_athc", &up, 4, &q).len(), 1);
        // 120 high, crash to 100, then 101, 103: up-close passes (P 106) but BUYSIG is off
        let dn = days(&[120.0, 100.0, 101.0, 103.0], 106.0);
        assert_eq!(trades_of(ID, &dn, 5, &q).len(), 1);
        assert!(trades_of("ytbtst_athc", &dn, 5, &q).is_empty());
        assert!(params("ytbtst_athc").unwrap().iter().all(|p| !p.key.starts_with("flt_") && !p.key.starts_with("st_")));
    }

    #[test]
    fn penny_stock_filter() {
        // AFL buyflt = C > pennyflt: the entry close must be strictly above the penny price; 0 (default) = off
        let days = |h: &[f64], p: f64| { let mut d = fcloses(h); d.extend([sig(p), morning(p, p + 0.5, p - 0.5, p)]); d };
        let up = days(&[100.0, 102.0, 104.0], 108.5);
        let with = |x: f64| { let mut q = base(&[("pb_pct", 0.0)]); q.insert("penny".into(), x); q };
        assert_eq!(trades_of(ID, &up, 4, &with(0.0)).len(), 1);
        assert_eq!(trades_of(ID, &up, 4, &with(108.0)).len(), 1);
        assert!(trades_of(ID, &up, 4, &with(109.0)).is_empty());
        assert!(trades_of(ID, &up, 4, &with(108.5)).is_empty(), "strict >: a close equal to the penny price is not bought");
        assert_eq!(trades_of(ID, &up, 4, &base(&[("pb_pct", 0.0)])).len(), 1); // no penny key = the default 0
        for id in ["ytbtst_core", "ytbtst_ath", "ytbtst_athc", "ytbtst_old_fixed", "ytbtst_rank_price"] {
            let d = params(id).unwrap().into_iter().find(|p| p.key == "penny").expect(id);
            assert!(d.def == 0.0 && d.min == 0.0 && d.max == 100.0 && d.step == 1.0, "{id}: AFL Param(\"Penny Stock Price\",0,0,100,1)");
        }
        assert!(params("ytbtst_slc").unwrap().iter().all(|p| p.key != "penny"));
    }

    /// Old fixed AFL, ver3_bar-count: buy on the entry_n-th candle back from the day's last candle when P > C1 > C2 > C3
    /// with P-C1 >= C1-C2 >= C2-C3 (>=, so equal changes pass; P = the entry candle's close, no lookahead), sell on the
    /// exit_n-th candle of a later day counted from its first candle; a lone 15:30 candle is not counted (the same cases
    /// as scripts/verify_ver3_bar_count.py).
    #[test]
    fn old_fixed_ver3_candle_count_entry_and_exit() {
        let at = |times: &[u32], closes: &[f64]| -> Day { times.iter().zip(closes).map(|(&t, &c)| (t, c, c, c, c)).collect() };
        const T: [u32; 7] = [915, 920, 925, 1510, 1515, 1520, 1525];
        let sigd = at(&T, &[104.0, 104.0, 104.0, 104.0, 106.5, 106.5, 106.5]);
        let next = at(&T, &[106.0; 7]); // below 106.5: no new signal on the exit day
        let hist = || vec![flat(100.0), flat(101.0), flat(103.0)];
        let with = |extra: Vec<Day>| { let mut d = hist(); d.extend(extra); d };
        let of = |d: &[Day], p: &Params| trades_of("ytbtst_old_fixed", d, d.len() - 1, p).iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.reason.clone())).collect::<Vec<_>>();
        let none = Params::new();
        // normal day (last candle 15:25): entry 3 = 15:15, exit 2 after a 09:15 open = 09:20
        assert_eq!(of(&with(vec![sigd.clone(), next.clone()]), &none), vec![(3, 106.5, 4, 106.0, "Exit candle 2 at 0920".to_string())]);
        // 09:00 open: 09:05; 09:55 open: 10:00
        let mut t9: Vec<u32> = vec![900, 905];
        t9.extend(T);
        assert_eq!(of(&with(vec![sigd.clone(), at(&t9, &[106.0; 9])]), &none)[0].4, "Exit candle 2 at 0905");
        assert_eq!(of(&with(vec![sigd.clone(), at(&[955, 1000, 1005, 1525], &[106.0; 4])]), &none)[0].4, "Exit candle 2 at 1000");
        // a lone 15:30 candle is not counted (entry stays 15:15) ...
        let mut t30 = T.to_vec();
        t30.push(1530);
        let lone = at(&t30, &[104.0, 104.0, 104.0, 104.0, 106.5, 106.5, 106.5, 999.0]);
        assert_eq!(of(&with(vec![lone, next.clone()]), &none)[0].1, 106.5);
        // ... and is not the day's close: a lone 15:30 print of 50 on the 3rd history day leaves C1 = 103
        let d = vec![flat(100.0), flat(101.0), at(&[1525, 1530], &[103.0, 50.0]), sigd.clone(), next.clone()];
        assert_eq!(of(&d, &none).len(), 1);
        // after-hours to 16:14: the 15:30 candle is trading and counted, entry 3 = 16:00
        let mut tah = T.to_vec();
        tah.extend([1530, 1535, 1600, 1605, 1610]);
        let mut cah = vec![104.0; 9];
        cah.extend([106.5; 3]);
        let t = trades_of("ytbtst_old_fixed", &with(vec![at(&tah, &cah), next.clone()]), 4, &none);
        assert_eq!((t[0].legs[0].px, t[0].exit_i), (106.5, 4));
        // no lookahead: the 15:15 close 104.5 fails (change 1.5 < 2) although the day's close 110 would pass
        assert!(of(&with(vec![at(&T, &[104.0, 104.0, 104.0, 104.0, 104.5, 110.0, 110.0]), next.clone()]), &none).is_empty());
        // entry 1 = the last candle 15:25, exit 1 = the first candle 09:15
        let d = with(vec![at(&T, &[104.0, 104.0, 104.0, 104.0, 104.0, 104.0, 106.5]), next.clone()]);
        assert_eq!(of(&d, &ps(&[("entry_n", 1.0), ("exit_n", 1.0)])), vec![(3, 106.5, 4, 106.0, "Exit candle 1 at 0915".to_string())]);
        // equal changes pass (>=); a smaller last change fails; three completed days are needed
        let eq = vec![flat(100.0), flat(102.0), flat(104.0), at(&T, &[104.0, 104.0, 104.0, 104.0, 106.0, 106.0, 106.0]), next.clone()];
        assert_eq!(of(&eq, &none).len(), 1);
        let lt = vec![flat(100.0), flat(102.0), flat(104.0), at(&T, &[104.0, 104.0, 104.0, 104.0, 105.9, 106.0, 106.0]), next.clone()];
        assert!(of(&lt, &none).is_empty());
        assert!(of(&eq[1..], &none).is_empty());
        // a next day with a single candle: exit 2 falls on the day after
        let d = with(vec![sigd.clone(), at(&[915], &[1.0]), next.clone()]);
        assert_eq!(of(&d, &none)[0].2, 5);
        assert_eq!(params("ytbtst_old_fixed").unwrap().iter().map(|p| p.key).collect::<Vec<_>>(), ["src", "entry_n", "exit_n", "penny", "sizing", "alloc", "equity"]);
        // on the feed the same rules hold
        assert_eq!(trades_of("ytbtst_old_fixed", &with(vec![sigd.clone(), next.clone()]), 4, &ps(&[("src", 2.0)])).len(), 1);
    }

    // ------------------------------------------------------------------ E3 second-last candle, E4 candle-count

    /// A three-candle day at one price x (high x + 1, low x - 1): never a circuit candle.
    fn day3(x: f64) -> Day {
        vec![(915, x, x + 1.0, x - 1.0, x), (1520, x, x + 1.0, x - 1.0, x), (1525, x, x + 1.0, x - 1.0, x)]
    }
    fn day2(x: f64) -> Day {
        vec![(915, x, x + 1.0, x - 1.0, x), (1525, x, x + 1.0, x - 1.0, x)]
    }
    fn keys(t: &[Trade]) -> Vec<(usize, f64, usize, f64, bool)> {
        t.iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.open)).collect()
    }

    /// Buy at the second-last candle's close, sell at the next day's first candle's close; the last trade stays open.
    #[test]
    fn slc_buys_the_second_last_candle_and_sells_the_next_first_candle() {
        let d = |x: f64| vec![(915, x, x + 1.0, x - 1.0, x + 0.5), (1520, x + 1.0, x + 2.0, x, x + 1.5), (1525, x + 1.5, x + 2.0, x + 1.0, x + 1.8)];
        let t = trades_of("ytbtst_slc", &[d(100.0), d(110.0), d(120.0)], 2, &Params::new());
        assert_eq!(keys(&t), vec![(0, 101.5, 1, 110.5, false), (1, 111.5, 2, 120.5, false), (2, 121.5, 2, 121.8, true)]);
        assert_eq!(t[0].reason, "First candle exit at 0915");
        // a one-candle day has no second-last candle
        let t = trades_of("ytbtst_slc", &[vec![(1525, 100.0, 101.0, 99.0, 100.5)], d(110.0)], 1, &Params::new());
        assert_eq!(keys(&t), vec![(1, 111.5, 1, 111.8, true)]);
    }

    /// Up-circuit (O = C = H at 2 decimals) blocks the buy; low-circuit (O = C = L) on the next first candle holds to the
    /// following day's first candle, with no new buy while holding.
    #[test]
    fn slc_up_and_low_circuits() {
        let mut d0 = day3(100.0);
        d0[1] = (1520, 101.0, 101.004, 100.0, 101.0); // O = C = H at 2 decimals
        let t = trades_of("ytbtst_slc", &[d0, day3(110.0), day3(120.0)], 2, &Params::new());
        assert_eq!(keys(&t)[0], (1, 110.0, 2, 120.0, false));
        let mut d1 = day3(110.0);
        d1[0] = (915, 109.0, 112.0, 109.0, 109.0); // O = C = L: low circuit, no sale
        let t = trades_of("ytbtst_slc", &[day3(100.0), d1, day3(120.0), day3(130.0)], 3, &Params::new());
        assert_eq!(keys(&t), vec![(0, 100.0, 2, 120.0, false), (2, 120.0, 3, 130.0, false), (3, 130.0, 3, 130.0, true)]);
    }

    /// A 2-candle day: its first candle is also its second-last. Holding: it sells there and cannot buy (the buy check
    /// runs first while still holding). Flat: the AFL buys and the same-bar sell cancels it (Buy[i] = 0): no trade.
    #[test]
    fn slc_two_candle_days() {
        let t = trades_of("ytbtst_slc", &[day3(100.0), day2(110.0), day3(120.0)], 2, &Params::new());
        assert_eq!(keys(&t), vec![(0, 100.0, 1, 110.0, false), (2, 120.0, 2, 120.0, true)]);
        let t = trades_of("ytbtst_slc", &[vec![(1525, 100.0, 101.0, 99.0, 100.0)], day2(110.0), day3(120.0)], 2, &Params::new());
        assert_eq!(keys(&t), vec![(2, 120.0, 2, 120.0, true)]);
    }

    /// Daily Supertrend, period 1, factor 1, close: daily bars (H = x + 1, L = x - 1) closes 100, 110, 80:
    /// TR 2, 11, 31; Up 102, 121, -; Dn 98, 99, -; day 2: 80 < Dn 99 -> down, Up re-seeded 80 + 31 = 111.
    /// trend +1, +1, -1; band 98, 99, 111.
    #[test]
    fn trend_seen_follows_expand_last() {
        let d = vec![day2(100.0), day2(110.0), day2(80.0)];
        let r = rows(3);
        let x = ctx_of(&r, &d);
        let b = Bars::load(&x, 1);
        let (rws, days, _) = daily_bars(&x, &b, 1, 2);
        let cs = candles(&b, &rws, 1);
        assert_eq!(cs.len(), 6);
        let s = trend_seen(&x, &rws, &days, &cs, 1, 2, 1, 1.0);
        // inside a day the previous day; on the day's last candle that day itself
        // before the first day is complete AmiBroker shows the first day's own value
        assert_eq!(s, vec![Some((1, true)), Some((1, true)), Some((1, true)), Some((1, true)), Some((1, true)), Some((-1, true))]);
        let (tr, band) = supertrend_full(&days, 2, 1, 1.0);
        assert_eq!((tr, band), (vec![1, 1, -1], vec![98.0, 99.0, 111.0]));
        // weekly: all three days are one week (Mon-Wed): only the week's last candle sees it
        let s = trend_seen(&x, &rws, &days, &cs, 2, 2, 1, 1.0);
        assert_eq!(s, vec![Some((1, true)); 6]);
        // warm-up: period 2 has no band on the first bar
        let (_, band) = supertrend_full(&days, 2, 2, 1.0);
        assert!(band[0].is_nan() && band[1].is_finite());
    }

    /// Bull/bear filters on the same trend (daily, period 1, factor 1, close; 4th day 70 keeps it down).
    #[test]
    fn slc_bull_and_bear_filters() {
        let d = vec![day3(100.0), day3(110.0), day3(80.0), day3(70.0)];
        let f = ps(&[("flt_tf", 1.0), ("st_period", 1.0), ("st_factor", 1.0), ("st_point", 2.0)]);
        // day 0's second-last candle sees the first day's own value (up, band 98 defined)
        assert_eq!(keys(&trades_of("ytbtst_slc_bull", &d, 3, &f)), vec![(0, 100.0, 1, 110.0, false), (1, 110.0, 2, 80.0, false), (2, 80.0, 3, 70.0, false)]);
        assert_eq!(keys(&trades_of("ytbtst_slc_bear", &d, 3, &f)), vec![(3, 70.0, 3, 70.0, true)]);
        assert_eq!(trades_of("ytbtst_slc", &d, 3, &f).len(), 4);
        // band undefined during the warm-up (period 5 on 4 days): no trade at all
        assert!(trades_of("ytbtst_slc_bull", &d, 3, &ps(&[("flt_tf", 1.0), ("st_period", 5.0)])).is_empty());
        // monthly default: one month only, its own value has no band yet (period 5)
        assert!(trades_of("ytbtst_slc_bull", &d, 3, &Params::new()).is_empty());
    }

    /// 15-minute candles: clock buckets; a 09:55 session start gives a first candle stamped 09:45.
    #[test]
    fn fifteen_minute_aggregation() {
        let d = vec![vec![(955, 10.0, 12.0, 9.0, 11.0), (1000, 11.0, 13.0, 10.5, 12.0), (1005, 12.0, 15.0, 11.0, 14.0), (1010, 14.0, 14.5, 8.0, 9.0), (1015, 9.0, 9.5, 8.5, 9.2)]];
        let r = rows(1);
        let x = ctx_of(&r, &d);
        let b = Bars::load(&x, 1);
        let (rws, _, _) = daily_bars(&x, &b, 1, 0);
        let c = candles(&b, &rws, 2);
        let v: Vec<(u32, f64, f64, f64, f64)> = c.iter().map(|c| (c.t, c.o, c.h, c.l, c.c)).collect();
        assert_eq!(v, vec![(945, 10.0, 12.0, 9.0, 11.0), (1000, 11.0, 15.0, 8.0, 9.0), (1015, 9.0, 9.5, 8.5, 9.2)]);
        assert_eq!(candles(&b, &rws, 1).len(), 5);
    }

    fn e4(kv: &[(&str, f64)]) -> Params {
        let mut p = ps(&[("flt_type", 3.0), ("pt_pct", 0.0)]);
        p.extend(kv.iter().map(|(k, v)| (k.to_string(), *v)));
        p
    }

    /// The counter counts CANDLES of days after the buy day: exit 1 = the next day's first candle, exit 3 with a 2-candle
    /// next day = the first candle of the day after. Entry k = k candles before the day's last.
    #[test]
    fn candles_count_logic_and_entry_offset() {
        let d = vec![day3(100.0), day2(110.0), day3(120.0), day3(130.0)];
        let t = trades_of("ytbtst_candles", &d, 3, &e4(&[]));
        assert_eq!(keys(&t), vec![(0, 100.0, 1, 110.0, false), (1, 110.0, 2, 120.0, false), (2, 120.0, 3, 130.0, false), (3, 130.0, 3, 130.0, true)]);
        assert_eq!(t[0].reason, "Candle 1 exit at 0915");
        let t = trades_of("ytbtst_candles", &d, 3, &e4(&[("exit_candle", 3.0)]));
        assert_eq!(keys(&t)[0], (0, 100.0, 2, 120.0, false)); // day 1: counts 1, 2; day 2 first candle: 3
        // entry 2 = the first of 3 candles; it does not exist on a 2-candle day; an exit candle cannot also buy (else-if),
        // so day 3's first candle exits the day-2 trade and does not buy
        let t = trades_of("ytbtst_candles", &d, 3, &e4(&[("entry_candle", 2.0)]));
        assert_eq!(keys(&t), vec![(0, 100.0, 1, 110.0, false), (2, 120.0, 3, 130.0, false)]);
    }

    /// Profit target 5% from 100 = 105: high >= 105 sells at 105 (also exactly at it); an open above it sells at the open
    /// less the buffer; the buy candle itself is never checked, the next candle of the same day is.
    #[test]
    fn candles_profit_target_and_buffer() {
        let pt = e4(&[("pt_pct", 5.0), ("entry_candle", 1.0), ("exit_candle", 5.0)]);
        let mut d0 = day3(100.0);
        d0[1] = (1520, 100.0, 110.0, 99.0, 100.0); // the buy candle's own high is ignored
        let d = vec![d0.clone(), vec![(915, 101.0, 105.0, 100.0, 104.0), (1525, 104.0, 104.5, 103.0, 104.0)]];
        let t = trades_of("ytbtst_candles", &d, 1, &pt);
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (1, 105.0, "Profit target 5% at 0915"));
        let d = vec![d0.clone(), vec![(915, 107.0, 108.0, 106.0, 107.5), (1525, 107.5, 108.0, 107.0, 107.5)]];
        let t = trades_of("ytbtst_candles", &d, 1, &pt);
        assert!(close(t[0].exit_px, 107.0 * (1.0 - 0.25 / 100.0)));
        let t = trades_of("ytbtst_candles", &d, 1, &e4(&[("pt_pct", 5.0), ("entry_candle", 1.0), ("buffer_pct", 1.0)]));
        assert_eq!(t[0].exit_px, 106.0); // 107 x 0.99 = 105.93 lies below the bar's low 106: clamped
        let mut d0b = d0;
        d0b[2] = (1525, 100.0, 106.0, 99.0, 100.5); // same-day candle after the buy reaches the target
        let t = trades_of("ytbtst_candles", &[d0b, day3(100.0)], 1, &pt);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (0, 0, 105.0));
    }

    /// AmiBroker price bound: a fill outside the bar's low..high is moved into it. E4: target 105 gapped at an open of 107
    /// that is also the bar's low: open less 0.25% = 106.7325 is below the low, filled at 107. E1: profit booking 1% gapped
    /// at an open 109 = the bar's low: 109 - 0.25% = 108.7275 is filled at 109; a hard-stop gap at an open 105 = the bar's
    /// high and low (a circuit bar) is filled at 105.
    #[test]
    fn fills_are_clamped_to_the_bar_range() {
        let pt = e4(&[("pt_pct", 5.0), ("entry_candle", 1.0), ("exit_candle", 5.0)]);
        let d = vec![day3(100.0), vec![(915, 107.0, 108.0, 107.0, 107.5), (1525, 107.5, 108.0, 107.0, 107.5)]];
        let t = trades_of("ytbtst_candles", &d, 1, &pt);
        assert_eq!((t[0].exit_i, t[0].exit_px), (1, 107.0));
        let t = trades(&standard(sig(106.5), morning(109.0, 110.0, 109.0, 109.5)), 4, &base(&[]));
        assert_eq!((t[0].exit_px, t[0].reason.as_str()), (109.0, "Profit booking 1% at 0920"));
        let t = trades(&standard(sig(106.5), morning(105.0, 105.0, 105.0, 105.0)), 4, &base(&[("hardstop", 1.0)]));
        assert_eq!((t[0].exit_px, t[0].reason.as_str()), (105.0, "Hard stop 1% at 0920"));
        assert_eq!((clamp(1.0, 2.0, 3.0), clamp(4.0, 2.0, 3.0), clamp(2.5, 2.0, 3.0)), (2.0, 3.0, 2.5));
    }

    /// Trend filter (daily, period 1, factor 1, close; closes 100, 110, 80): entry 0 buys on day 0's last candle (it sees
    /// day 0's own trend), exit 5 is never reached, the trend seen on day 2's last candle is down: trend exit at 80.
    /// Starting the window on day 2 with entry 1: buy at day 2's second-last candle (sees day 1, up) and a trend exit on the
    /// same day's last candle.
    #[test]
    fn candles_trend_filter_and_exit() {
        let d = vec![day2(100.0), day2(110.0), day2(80.0)];
        let f = |kv: &[(&str, f64)]| { let mut p = ps(&[("flt_type", 1.0), ("flt_tf", 1.0), ("st_period", 1.0), ("st_factor", 1.0), ("st_point", 2.0), ("pt_pct", 0.0)]); p.extend(kv.iter().map(|(k, v)| (k.to_string(), *v))); p };
        let t = trades_of("ytbtst_candles", &d, 2, &f(&[("exit_candle", 5.0)]));
        assert_eq!(keys(&t), vec![(0, 100.0, 2, 80.0, false)]);
        assert_eq!(t[0].reason, "Trend exit at 1525");
        let r = rows(3);
        let x = ctx_of(&r, &d);
        let t = run("ytbtst_candles", &x, 2, 2, &f(&[("entry_candle", 1.0)])).unwrap();
        assert_eq!((keys(&t), t[0].reason.as_str()), (vec![(2, 80.0, 2, 80.0, false)], "Trend exit at 1525"));
        // entry 1 from the start: day 0's second-last candle sees the first day's own trend (+1) and buys;
        // day 1's first candle exits (count 1) and cannot buy (else-if); day 2's second-last buys and trend-exits on its last
        let t = trades_of("ytbtst_candles", &d, 2, &f(&[("entry_candle", 1.0), ("exit_candle", 1.0)]));
        assert_eq!(keys(&t), vec![(0, 100.0, 1, 110.0, false), (2, 80.0, 2, 80.0, false)]);
    }

    /// 15-minute candle-count: the last 15-minute candle (15:15 = 15:15..15:25 bars) closes at the 15:25 close, the next
    /// day's first 15-minute candle at its 09:25 close.
    #[test]
    fn candles_on_fifteen_minutes() {
        let d = vec![
            vec![(1500, 99.0, 100.0, 98.0, 99.5), (1515, 99.5, 100.5, 99.0, 100.0), (1525, 100.0, 101.0, 99.5, 100.7)],
            vec![(915, 101.0, 102.0, 100.0, 101.2), (925, 101.2, 101.5, 101.0, 101.3), (930, 101.3, 101.6, 101.0, 101.4)],
        ];
        let t = trades_of("ytbtst_candles", &d, 1, &e4(&[("tf", 2.0)]));
        assert_eq!(keys(&t), vec![(0, 100.7, 1, 101.3, false), (1, 101.4, 1, 101.4, true)]); // day 1's last 15-min candle buys again
        let t = trades_of("ytbtst_candles", &d, 1, &e4(&[]));
        assert_eq!(keys(&t), vec![(0, 100.7, 1, 101.2, false), (1, 101.4, 1, 101.4, true)]);
        assert_eq!(unsupported("ytbtst_candles", &ps(&[("tf", 3.0)])).unwrap().0, "not_testable");
        assert!(trades_of("ytbtst_candles", &d, 1, &e4(&[("tf", 3.0)])).is_empty());
    }

    // ------------------------------------------------------------------ src 3: IEOD as the team's 2021 copy

    fn rows_in(n: usize, year: &str, ticker: &str) -> Vec<Row> {
        rows(n).into_iter().map(|mut r| { r.date.replace_range(0..4, year); r.ticker = ticker.into(); r }).collect()
    }

    #[test]
    fn team_copy_rule() {
        assert!(!team_keep(1, 2010, 900) && !team_keep(1, 2010, 910) && team_keep(1, 2010, 915));
        assert!(!team_keep(1, 2011, 1530) && !team_keep(1, 2015, 1530) && team_keep(1, 2010, 1530) && team_keep(1, 2016, 1530) && team_keep(1, 2012, 1525));
        assert!(team_keep(1, 2008, 1610)); // NIFTY 2007-08 after-hours kept (no clean bucket rule, see notes)
        assert!(!team_keep(2, 2008, 1530) && !team_keep(2, 2008, 1615) && !team_keep(2, 2020, 1530) && team_keep(2, 2020, 1525) && team_keep(2, 2013, 1815));
        assert_eq!((team_rule("NIFTYBANK"), team_rule("NIFTY 50")), (2, 1));
        assert!(unsupported(ID, &ps(&[("src", 3.0)])).is_none() && unsupported(ID, &ps(&[("src", 4.0)])).is_some());
        assert_eq!(required_series(&ps(&[("src", 3.0)])), ["BTST_5M.DC"]);
    }

    /// NIFTY 2012: the 15:30 bucket is dropped before the daily bar is built. Row 2's 15:30 print of 90 makes its src-1
    /// daily close 90 (no rise, no trade); src 3 rebuilds the day from 09:15..15:25 (close 104) and trades. In 2016 the
    /// team copy keeps 15:30 as well.
    #[test]
    fn src3_drops_the_1530_bucket_in_2011_2015_before_daily_bars() {
        let mut d = standard(sig(108.5), morning(108.0, 109.0, 107.0, 108.2));
        d[2].push((1530, 90.0, 90.0, 90.0, 90.0));
        for (year, want3) in [("2012", 1), ("2016", 0)] {
            let r = rows_in(5, year, "NIFTY 50");
            let x = ctx_of(&r, &d);
            assert!(run(ID, &x, 0, 4, &base(&[])).unwrap().is_empty(), "{year} src 1");
            assert_eq!(run(ID, &x, 0, 4, &base(&[("src", 3.0)])).unwrap().len(), want3, "{year} src 3");
        }
    }

    /// BANKNIFTY (any year): buckets 15:30-16:15 are dropped, so a 16:00 high above the target no longer books profit and
    /// the trade exits next morning; a Muhurat bar (18:15) is kept.
    #[test]
    fn src3_banknifty_drops_1530_to_1615() {
        let mut day = sig(106.5);
        day.push((1600, 106.6, 120.0, 106.5, 107.0));
        let d = standard(day, morning(106.0, 107.0, 105.0, 106.2));
        let r = rows_in(5, "2020", "NIFTYBANK");
        let x = ctx_of(&r, &d);
        assert_eq!(run(ID, &x, 0, 4, &base(&[])).unwrap()[0].reason, "Profit booking 1% at 1600");
        assert_eq!(run(ID, &x, 0, 4, &base(&[("src", 3.0)])).unwrap()[0].reason, "Time exit 0920");
        let mut day = sig(106.5);
        day.push((1815, 106.6, 120.0, 106.5, 107.0));
        let x = ctx_of(&r, &standard(day, morning(106.0, 107.0, 105.0, 106.2)));
        assert_eq!(run(ID, &x, 0, 4, &base(&[("src", 3.0)])).unwrap()[0].reason, "Profit booking 1% at 1815");
    }

    /// 2010 pre-open: a 09:05 bar is dropped, so the candle-count exit 1 is the 09:15 candle.
    #[test]
    fn src3_drops_the_pre_open() {
        let d = vec![day3(100.0), vec![(905, 101.0, 102.0, 100.0, 101.5), (915, 101.5, 102.0, 101.0, 101.8), (1525, 102.0, 102.0, 102.0, 102.0)]];
        let r = rows_in(2, "2010", "NIFTY 50");
        let x = ctx_of(&r, &d);
        let p = |src: f64| e4(&[("src", src)]);
        assert_eq!(run("ytbtst_candles", &x, 0, 1, &p(1.0)).unwrap()[0].exit_px, 101.5);
        assert_eq!(run("ytbtst_candles", &x, 0, 1, &p(3.0)).unwrap()[0].exit_px, 101.8);
    }

    /// Entry and exit bar start times of each trade (index runs) and the traded instrument per source.
    #[test]
    fn trade_times_and_instrument() {
        let r = rows(5);
        let x = ctx_of(&r, &standard(sig(108.5), morning(108.0, 109.0, 107.0, 108.2)));
        assert_eq!(trade_times(ID, &x, 0, 4, &base(&[])), vec![(1515, 920)]);
        assert_eq!(trade_times(ID, &x, 0, 3, &base(&[])), vec![(1515, 0)]); // open at the window end
        // 15-minute candle-count: entry on the 15:15 candle (15:15-15:29), exit on the next day's 09:15 candle
        let d = vec![
            vec![(1500, 99.0, 100.0, 98.0, 99.5), (1515, 99.5, 100.5, 99.0, 100.0), (1525, 100.0, 101.0, 99.5, 100.7)],
            vec![(915, 101.0, 102.0, 100.0, 101.2), (925, 101.2, 101.5, 101.0, 101.3), (930, 101.3, 101.6, 101.0, 101.4)],
        ];
        let r = rows(2);
        let x = ctx_of(&r, &d);
        assert_eq!(trade_times("ytbtst_candles", &x, 0, 1, &e4(&[("tf", 2.0)])), vec![(1515, 915), (930, 0)]);
        assert_eq!(instrument("NIFTY 50", 1).0, "NIFTY_F1");
        assert_eq!(instrument("NIFTYBANK", 3).0, "BANKNIFTY_F1");
        assert!(instrument("NIFTYBANK", 3).1.contains("team's 2021 copy"));
        assert_eq!(instrument("NIFTY 50", 2), ("NIFTY_FUT_FEED", "Nifty futures, 2019-2026 rolling feed (not IEOD)"));
        assert_eq!(source_of("NIFTY_FUT_FEED"), FEED_SOURCE);
        assert_eq!(source_of("NIFTY_F1"), DATABASE);
        assert_eq!(source_of("BANKNIFTY_F1"), DATABASE);
    }

    // ------------------------------------------------------------------ catalogue

    const TESTABLE: [&str; 9] = [ID, "ytbtst_ibs_roc", "ytbtst_ath", "ytbtst_ath_only", "ytbtst_high_n", "ytbtst_green", "ytbtst_volume", "ytbtst_athc", "ytbtst_old_fixed"];
    const E34: [&str; 4] = ["ytbtst_slc", "ytbtst_slc_bull", "ytbtst_slc_bear", "ytbtst_candles"];
    const PLACEHOLDERS: [&str; 8] = ["ytbtst_rank_price", "ytbtst_rank_ibs", "ytbtst_rank_lookback", "ytbtst_rank_roc", "ytbtst_rank_ibs_volume", "ytbtst_nifty_upclose", "ytbtst_nifty_incremental", "ytbtst_index_filter"];

    #[test]
    fn catalogue_has_the_up_close_sub_variants_and_the_stock_placeholders() {
        let cat = catalogue();
        let ids: Vec<&str> = cat.iter().map(|d| d.id).collect();
        assert_eq!(ids, TESTABLE.iter().chain(PLACEHOLDERS.iter()).chain(E34.iter()).copied().collect::<Vec<_>>());
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 21);
        for d in &cat {
            let (fam, name, label) = family_of(d.id).unwrap();
            assert_eq!((fam, name, label), ("ytbtst", "BTST", d.name));
            assert!(!label.is_empty() && !label.contains(" · "), "{label}");
            let head = d.id.split('_').next().unwrap();
            assert!(head.starts_with("yt") && head.len() > 2 && head.bytes().all(|b| b.is_ascii_lowercase()) && d.id.contains('_'));
            assert!(is_variant(d.id) && d.book == BOOK && d.claim.is_none());
            let want = match d.id { "ytbtst_slc" | "ytbtst_slc_bull" | "ytbtst_slc_bear" => GROUP_SLC, "ytbtst_candles" => GROUP_CANDLES, _ => GROUP };
            assert_eq!(d.group, want, "{}", d.id);
            assert_eq!(d.group.split(" · ").count(), 3);
            assert_eq!(d.needs_series, ["BTST_5M.DC"]);
            assert!(d.notes.starts_with("YT strategy") && d.notes.contains("Source AFL: BTST/"), "{}", d.id);
        }
        for id in ["ytbtst_lb2_nofilter", "ytbtst_lb2_st_weekly", "ytbtst_lb3_1515", "ytbtst_lb3_1450", "ytbtst_lb3_st_weekly", "ytbtst_lb4_nofilter", "ytbtst_up_close_ibs_roc"] {
            assert!(!is_variant(id) && params(id).is_none(), "{id} was removed");
        }
        assert!(params("nope").is_none() && family_of("nope").is_none() && !is_variant("nope"));
        assert!(run("nope", &Ctx::new(&rows(1), &rows(1), &[]), 0, 0, &Params::new()).is_none());
        assert!(variant_notes(8).contains("lookahead") && variant_notes(7).contains("SELLSIG"));
    }

    /// Every rule line follows `<sec>|[?<key><op><num>|]<text>`; every guard key and `{key…}` placeholder is a parameter
    /// of that sub-variant; `:opt:` lists have as many options as the parameter's range.
    #[test]
    fn rule_templates_use_only_the_sub_variants_parameters() {
        for d in catalogue() {
            let ps: BTreeMap<&str, ParamDef> = params(d.id).unwrap().into_iter().map(|p| (p.key, p)).collect();
            for line in d.rules {
                let (sec, rest) = line.split_once('|').unwrap();
                assert!(["F", "E", "X"].contains(&sec), "{line}");
                let text = if let Some(g) = rest.strip_prefix('?') {
                    let (guard, text) = g.split_once('|').unwrap();
                    let at = guard.find(|c: char| "<>=!".contains(c)).unwrap();
                    let (key, cond) = guard.split_at(at);
                    let num = cond.trim_start_matches("!=").trim_start_matches(['>', '=']);
                    assert!(["!=", ">", "="].iter().any(|op| cond.starts_with(op)) && num.parse::<f64>().is_ok(), "{line}");
                    assert!(ps.contains_key(key), "{} guard {key}", d.id);
                    text
                } else {
                    rest
                };
                for ph in text.split('{').skip(1) {
                    let ph = ph.split('}').next().unwrap();
                    let mut it = ph.splitn(3, ':');
                    let key = it.next().unwrap();
                    let p = ps.get(key).unwrap_or_else(|| panic!("{} placeholder {key}", d.id));
                    match (it.next(), it.next()) {
                        (None, _) | (Some("t"), None) => {}
                        (Some("opt"), Some(list)) => assert_eq!(list.split('/').count() as f64, (p.max - p.min) / p.step + 1.0, "{} {ph}", d.id),
                        other => panic!("{} bad placeholder {ph} {other:?}", d.id),
                    }
                }
            }
        }
    }

    #[test]
    fn placeholders_are_not_testable_and_never_run() {
        let r = rows(5);
        let x = ctx_of(&r, &standard(sig(108.5), morning(108.0, 109.0, 107.0, 108.2)));
        for id in PLACEHOLDERS {
            let d = catalogue().into_iter().find(|d| d.id == id).unwrap();
            assert!(run(id, &x, 0, 4, &Params::new()).is_none() && unsupported(id, &Params::new()).is_none());
            if id == "ytbtst_index_filter" {
                assert!(d.status == Status::NotTestable(STOCK_ONLY) && is_placeholder(id) && params(id).unwrap().is_empty() && !universe_runnable(id));
            } else {
                // universe sub-variants: not on a single instrument, runnable on the universe, with portfolio parameters
                assert!(d.status == Status::NotTestable(UNIVERSE_ONLY) && !is_placeholder(id) && universe_runnable(id), "{id}");
                let keys: Vec<&str> = params(id).unwrap().iter().map(|p| p.key).collect();
                assert!(["max_positions", "pct", "pad", "days", "entry_time", "exit_time"].iter().all(|k| keys.contains(k)), "{id}");
            }
        }
        for id in TESTABLE {
            assert!(!is_placeholder(id) && universe_runnable(id));
            assert!(catalogue().into_iter().find(|d| d.id == id).unwrap().status == Status::Live);
        }
        for id in E34 {
            assert!(!universe_runnable(id));
        }
    }

    #[test]
    fn defaults_are_the_verified_nifty_settings_and_inside_their_ranges() {
        for id in TESTABLE {
            let d: BTreeMap<&str, f64> = params(id).unwrap().iter().map(|p| (p.key, p.def)).collect();
            assert_eq!(d["src"], 1.0, "{id}");
            if id != "ytbtst_old_fixed" {
                for (k, v) in [("entry_time", 1450.0), ("exit_time", 925.0), ("pb_pct", 0.0), ("hardstop", 0.0)] {
                    assert_eq!(d[k], v, "{id} {k}");
                }
                if id != "ytbtst_ath_only" {
                    assert_eq!(d["days"], 2.0, "{id}");
                }
                if id != "ytbtst_athc" {
                    assert_eq!(d["flt_type"], 3.0, "{id}");
                }
            }
            for q in params(id).unwrap() {
                assert!(q.def >= q.min && q.def <= q.max, "{id} {}", q.key);
            }
        }
        let d: BTreeMap<&str, f64> = params("ytbtst_ibs_roc").unwrap().iter().map(|p| (p.key, p.def)).collect();
        assert_eq!((d["ibs_dir"], d["ibs_thr"], d["roc_days"], d["roc_thr"]), (1.0, 0.5, 2.0, 1.0));
        let d: BTreeMap<&str, f64> = params("ytbtst_volume").unwrap().iter().map(|p| (p.key, p.def)).collect();
        assert_eq!((d["vol_days"], d["vol_mult"]), (5.0, 1.0));
        let d: BTreeMap<&str, f64> = params("ytbtst_green").unwrap().iter().map(|p| (p.key, p.def)).collect();
        assert_eq!((d["candle"], d["green_days"]), (1.0, 2.0));
        assert_eq!(params("ytbtst_athc").unwrap().iter().find(|p| p.key == "athc_pct").unwrap().def, 10.0);
    }

    /// Every testable sub-variant runs the same synthetic signal on both sources (except volume on the feed).
    #[test]
    fn every_sub_variant_trades_on_a_matching_setup() {
        let day = padded(vec![(915, 107.0, 110.0, 103.0, 108.0), (1450, 108.0, 109.0, 107.8, 108.5), (1515, 108.5, 108.8, 108.3, 108.6), (1525, 108.6, 109.0, 108.0, 108.7)]);
        let next = vec![(915, 108.0, 108.4, 107.9, 108.1), (920, 108.1, 108.3, 107.9, 108.2), (925, 108.0, 109.0, 107.0, 108.3)];
        let candle = |o: f64, c: f64| vec![(1525, o, c, o, c)];
        let d = vec![candle(95.0, 96.0), candle(97.0, 98.0), candle(99.0, 100.0), candle(101.0, 102.0), candle(103.0, 104.0), day, next];
        let r = rows(7);
        let mut x = ctx_of(&r, &d);
        x.extra.insert("BTST_5M.DV".into(), vec![1.0; 7]);
        let mut v = vec![f64::NAN; 7];
        v[5] = 1.0;
        for t in ["0915", "1450"] {
            x.extra.insert(format!("BTST_5M.V{t}"), v.clone());
        }
        for id in TESTABLE {
            for src in [1.0, 2.0] {
                let t = run(id, &x, 0, 6, &ps(&[("src", src)])).unwrap();
                let want = usize::from(!(id == "ytbtst_volume" && src == 2.0));
                assert_eq!(t.len(), want, "{id} src {src}");
                if want == 1 {
                    assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].open), (5, 6, false), "{id} src {src}");
                }
            }
        }
    }
}
