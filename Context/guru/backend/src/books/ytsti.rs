//! YT Strategies · Supertrend (Intraday): the team's AmiBroker AFLs in "Old Topics/Supertrend/ST Fresh Running
//! Buy-Sell Index Intraday" that trade one F&O futures series on intraday bars, long and short, squared off at
//! 15:20. Coded from the AFLs as written, with the AFLs' own default Param values; there is no book PDF and no trade
//! sheet was compared (coded only, 14-11-2026).
//!
//! Coded (one id per AFL; the two files differ only in the entry condition):
//! - "LONG & Short Intraday in Nifty FO by ST(52) with FRESH Entry.afl" (Suraj Mehra, checked by Jimit, 21-02-2022):
//!   buy on the bar where the ST trend turns up (`MTREND[i]==1 AND MTREND[i-1]==-1`), short where it turns down;
//! - "LONG & Short Intraday in Nifty FO by ST(52) with RUNNING Entry.afl" (checked by Saumya, 22-02-2022): buy on any
//!   bar where the ST trend is up and the position is not long (so also on the day's first bar after a square-off),
//!   short on any bar where it is down and the position is not short.
//! Both: the entry is taken only while `TimeNow < SquareOffTime` (1520); an opposite entry closes the open position
//! at the same close (reversal); a long (short) still open on the first bar with `TimeNow >= SquareOffTime` is sold
//! (covered) at that bar's close. All fills are at the bar's close (`BuyPrice = Close` etc.).
//!
//! Data: the 1-minute F&O futures of the team's AmiBroker IEOD database (`crate::intraday`), compressed to the AFL
//! header's "Timeframe: 5 Minute" (the AFL itself has no TimeFrameSet: it runs on the Analysis periodicity, offered as
//! the `tf_min` setting, default 5). Candles are anchored at 09:15 and labelled with their START time, so
//! `TimeNow = Hour()*100+Minute()` of the 15:20 candle is 1520 and the square-off fills at that candle's close (the
//! 15:24 minute); for 5-minute candles this grid equals AmiBroker's own time compression (see `intraday::compress`),
//! for other timeframes it is a convention not checked against an AmiBroker run. The engine's daily `ieodfo` series
//! (the same minutes aggregated to days) carries the trades: entry leg / exit_i are the daily bars of the entry /
//! exit dates, prices are the candle closes, and the entry / exit times are written in each trade's reason.
//!
//! AmiBroker behaviour followed (as in `ytst.rs`):
//! - the ST is the AFL's own loop, which is the team's ST (`ytst::st_lines`): satr = mean of the last `period`
//!   ATR(1) values (ATR(1) Null on the first bar, so the bands are Null until bar `period` and the trend stays 1),
//!   bands ratchet and reset on a flip, the trend flips when the Close crosses the previous bar's band; calculation
//!   point mid point (H+L)/2 (default), close or pivot (H+L+C)/3;
//! - the ST and the position loop run over all loaded candles, continuously across days and sessions (state starts at
//!   the data's first candle, `Mtrend[0]=1`, `Position=0`); a position the data never squares off (a day without a
//!   candle at or after the square-off time) is carried to the next day, as the loop does;
//! - the backtest range: a trade counts when its entry date is inside the selected period; a position still open at
//!   the period's last candle is closed at that candle's close (end of backtest).
//!
//! Not modelled: the AFLs' backtest settings (initial equity 10,00,000, position size, `MAXOPENPOSITIONS` 20,
//! brokerage 0.65 with commission mode 1, `AllowPositionShrinking`) — the engine's own capital / cost settings
//! apply; the commented-out dead-stock / circuit / turnover filters are not used by the AFLs. Contract roll-overs of
//! the continuous futures series are not modelled (an intraday position never crosses an expiry in normal sessions).
//!
//! Also coded (16-11-2026): the four AFLs in "Old Topics AFL/Supertrend/Different ST Entry & exit/Nifty & BN_Diff ST
//! entry- exit on 25 min, 60 min, 75 min timeframe_With ST & %S daily filter" (watchlist "INDEX IEOD FO": Nifty /
//! Bank Nifty futures of the IEOD F&O database), coded as written with their default Params; one id per AFL:
//! - `ytsti_bear_v2_short` — "BEAR VER2/Entry and Exit with Hourly ST with Daily ST as Filter for Bear VERSION (Early
//!   entry late exit)(FO Database).afl" (Suraj Mehra, 29-11-2021): short only; short when flat, daily ST(5,4) down and
//!   hourly ST1 = ST(5,1) down (case 1 if ST2 = ST(5,2) is up, case 2 if down); case 1 covers when ST1 turns up and
//!   becomes case 2 when ST2 turns down first; case 2 covers when ST2 turns up; any case covers when the daily ST
//!   turns up. The loop's if / else-if order is kept (a case-1 trade turning case 2 is not covered on that bar).
//! - `ytsti_bear_diff_st` — "BEAR/...for Bear(FO)Diff ST.afl" (Suraj Mehra, 26-11-2021): Buy = filter bear AND ST1 up,
//!   Sell = (ST1 AND ST2 down) OR filter bull, Short = ST1 AND ST2 down AND filter bear, Cover = filter bull OR ST1 up,
//!   each pair ExRem'd. Defaults: filter "ST" and Trade Type "Long Only" (the first ParamList items) — the default
//!   buys while the daily filter is bear, as the AFL is written. Swing % filter and Short Only / Long & Short are
//!   settings. Buy and Sell (Short and Cover) never hold on the same bar, so ExRem + the backtester equal a
//!   one-position state machine; a Short bar always carries Sell and a Buy bar Cover, so Long & Short reverses at the
//!   same close.
//! - `ytsti_bear_single_st` — "BEAR/...for Bear(FO Database) - Final with Single ST.AFL": the same code as Diff ST
//!   except `BPd2 = BPd1; BFactor2 = BFactor1` (ST2 is ST1 = ST(5,1)) and narrower Param ranges; coded through the same
//!   path with the stated defaults (factor 1 below its Param minimum 2; swing 23 / 44 above the Param maximum 21).
//! - `ytsti_bear_pt_sl` — "BEAR/...for Bear with PT and SL.afl" (checked by Prasha & Nishtha): the AFL's own loop
//!   with targets. Default PT type "SL and PT Raito Based": long target = close + PT_Multi × (close − ST1 up-line),
//!   short target = close − PT_Multi × (ST1 down-line − close); "PT Only Percentage Based" ± PT_Prcnt %; "None" no
//!   target. A target hit (high >= target / low <= target) fills at the target, or at the open ∓ Buffer_Pct % when the
//!   candle opens through it, and blocks the next entry in that direction until re-armed (long: ST1 and ST2 both down;
//!   short: ST2 up). Long Only / Short Only also exit on ST1 & ST2 down / ST1 up; the AFL's Long & Short branch exits
//!   only on target, filter bull or reversal.
//! Shared: the hourly STs are the AFL's ST function (the team's ST loop, `ytst::st_lines`) on the candle mid point;
//! the AFL's TimeFrameSet(inDaily) filter is the daily ieodfo carrier's ST(5,4) on the daily mid point
//! (`ytrsi::st_trend`) or the swing % loop (`ytrsi::swing_filter`, 23 / 44 %) — the daily ieodfo bars are the same
//! minutes aggregated, so they are the bars TimeFrameSet(inDaily) builds. TimeFrameExpand's default expandLast is
//! followed as in `ytrsi_ieod.rs`: the day's filter value applies from the day's last candle, earlier candles of the
//! day use the previous day's value. The AFLs have no TimeFrameSet for the trading bars, so they run on the Analysis
//! periodicity: the folder's 25 / 60 / 75 minutes are the `tf_min` setting, default 60 (the headers' "Hourly");
//! candles are anchored at 09:15 (`intraday::compress`), a convention not checked against AmiBroker's own 60 / 75
//! minute compression. No time square-off in any of the four (positions carry overnight). Fills at the candle's close
//! except target fills; a target fill outside the candle's high-low range is clamped into it, and a ratio target
//! with no ST line (the ST's first bars, AmiBroker Null) is treated as no target (AmiBroker's Null comparison not
//! verified). Template settings at their defaults (dead stock NO, penny 0, turnover NO) only drop bar 0.
//! Not modelled (as above): equity, position size, brokerage, max positions, contract roll-overs.
#![allow(dead_code)]
use crate::books::ytst::{calc_on, st_lines};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::{self, Candles, Minutes};

pub const BOOK: &str = "yt_sti";
pub const FRESH: &str = "ytsti_st52_fresh_ls";
pub const RUNNING: &str = "ytsti_st52_running_ls";
pub const BEAR_V2: &str = "ytsti_bear_v2_short";
pub const BEAR_DIFF: &str = "ytsti_bear_diff_st";
pub const BEAR_SINGLE: &str = "ytsti_bear_single_st";
pub const BEAR_PT: &str = "ytsti_bear_pt_sl";
/// The four Nifty / Bank Nifty "Hourly ST with Daily ST (or swing %) filter" AFLs.
pub const BEAR_IDS: [&str; 4] = [BEAR_V2, BEAR_DIFF, BEAR_SINGLE, BEAR_PT];
pub const IDS: [&str; 6] = [FRESH, RUNNING, BEAR_V2, BEAR_DIFF, BEAR_SINGLE, BEAR_PT];

const GROUP: &str = "YT Strategies · Supertrend (Intraday)";

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    def_dir(id, name, "Long/Short", rules, notes)
}

fn def_dir(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const DATA_F: &str = "F|Data: 1-minute F&O futures (team AmiBroker IEOD database) compressed to 5-minute candles from 09:15, each labelled with its start time";
const ST_F: &str = "F|ST (the AFL's own loop): period 5, factor 2, on the mid point (H+L)/2; bands = point ± factor × mean of the last 5 ATR(1); the trend flips when the Close crosses the previous candle's band";
const TIME_F: &str = "F|Entries only while the candle's time is before the square-off time 15:20";
const X_SQ: &str = "X|Square off: sell (cover) at the close of the first candle at or after 15:20";
const X_REV: &str = "X|An opposite entry closes the position at the same close and reverses it";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(FRESH, "Intraday ST(5,2) Long-Short, Fresh Entry, 15:20 Square-off", &[
            DATA_F, ST_F, TIME_F,
            "E|Buy at the candle's close when the ST turns up (up now, down on the previous candle); short when it turns down",
            X_REV, X_SQ,
        ], "LONG & Short Intraday in Nifty FO by ST(52) with FRESH Entry.afl (folder 'ST Fresh Running Buy-Sell Index Intraday'), coded as the AFL is written on the AFL header's 5-minute timeframe. Runs on one F&O futures symbol with intraday data (ieodfo); trades are intraday and each trade's note gives its entry / exit times. The ST runs continuously across days. The AFL's equity, position-size, brokerage and max-position settings are not modelled. Not compared with the team's trade sheets."),
        def(RUNNING, "Intraday ST(5,2) Long-Short, Running Entry, 15:20 Square-off", &[
            DATA_F, ST_F, TIME_F,
            "E|Buy at the candle's close whenever the ST is up and the position is not long (also the day's first candle); short whenever it is down and the position is not short",
            X_REV, X_SQ,
        ], "LONG & Short Intraday in Nifty FO by ST(52) with RUNNING Entry.afl (same folder; identical to the FRESH Entry AFL except that the entry needs only the current ST trend, not a flip), coded as the AFL is written on the 5-minute timeframe. Runs on one F&O futures symbol with intraday data (ieodfo); each trade's note gives its entry / exit times. The AFL's equity, position-size, brokerage and max-position settings are not modelled. Not compared with the team's trade sheets."),
        def_dir(BEAR_V2, "Hourly ST(5,1)/ST(5,2) Short, Daily ST(5,4) Bear Filter (Early Entry, Late Exit)", "Short", &[
            BEAR_DATA_F, BEAR_ST_FILTER_F, BEAR_EXPAND_F, BEAR_ST_F,
            "E|Short at the candle's close when flat, the daily filter is bear and ST1 is down: case 1 if ST2 is still up, case 2 if ST2 is down too",
            "X|Case 1: cover at the close when ST1 turns up; if ST2 turns down first, the trade becomes case 2",
            "X|Case 2: cover at the close when ST2 turns up",
            "X|Cover at the close when the daily filter turns bull",
            BEAR_NO_SQ,
        ], "BEAR VER2 / Entry and Exit with Hourly ST with Daily ST as Filter for Bear VERSION (Early entry late exit)(FO Database).afl (folder 'Nifty & BN_Diff ST entry- exit on 25 min, 60 min, 75 min timeframe_With ST & %S daily filter', watchlist INDEX IEOD FO), coded as the AFL is written with its default settings on 60-minute candles (the header's Hourly; 25 and 75 minutes are the timeframe setting). Short only, no time square-off. The daily filter is the daily ieodfo bars' ST, applied from each day's last candle (TimeFrameExpand expandLast). Hourly candles are anchored at 09:15, a convention not checked against AmiBroker's hourly compression. The AFL's equity, position-size, brokerage and max-position settings are not modelled. Not compared with the team's trade sheets."),
        def_dir(BEAR_DIFF, "Hourly ST(5,1) Long in Bear Market, Exit below ST(5,1) and ST(5,2), Daily ST(5,4) or Swing % Filter", "Long", &[
            BEAR_DATA_F, BEAR_FILTER_F, BEAR_EXPAND_F, BEAR_ST_F,
            BEAR_LONG_E, BEAR_LONG_X, BEAR_SHORT_EX, BEAR_NO_SQ,
        ], "BEAR / Entry and Exit with Hourly ST with Daily ST OR Swing pct as Filter for Bear(FO)Diff ST.afl (same folder), coded as the AFL is written with its default settings: filter ST, Trade Type Long Only (buy while the daily filter is bear), on 60-minute candles. The other filter (swing %) and trade types (Short Only, Long & Short) are settings. Positional, no time square-off. Hourly candles are anchored at 09:15 (convention, not checked against AmiBroker). The AFL's equity, position-size, brokerage and max-position settings are not modelled. Not compared with the team's trade sheets."),
        def_dir(BEAR_SINGLE, "Hourly Single ST(5,1) Long in Bear Market, Daily ST(5,4) or Swing % Filter", "Long", &[
            BEAR_DATA_F, BEAR_FILTER_F, BEAR_EXPAND_F,
            "F|Hourly ST1 = ST2 = ST(5,1) (the AFL sets the exit ST's period and factor equal to the entry ST's) on the mid point (H+L)/2",
            BEAR_LONG_E, BEAR_LONG_X, BEAR_SHORT_EX, BEAR_NO_SQ,
        ], "BEAR / Entry and Exit with Hourly ST with Daily ST OR Swing pct as Filter for Bear(FO Database) - Final with Single ST.AFL (same folder): the same code as the Diff ST AFL except that the exit ST is the entry ST (BPd2 = BPd1, BFactor2 = BFactor1) and the Param ranges differ. Coded with the stated defaults (ST factor 1 although its Param minimum is 2; swing 23 / 44 % although the Param maximum is 21) on 60-minute candles: filter ST, Long Only. Positional, no time square-off. Not compared with the team's trade sheets."),
        def_dir(BEAR_PT, "Hourly ST(5,1) Long in Bear Market with ST-Ratio Target, Daily ST(5,4) or Swing % Filter", "Long", &[
            BEAR_DATA_F, BEAR_FILTER_F, BEAR_EXPAND_F, BEAR_ST_F,
            "E|Default Long Only: buy at the candle's close when flat, the daily filter is bear and ST1 is up; after a target exit the next long waits until ST1 and ST2 have both been down",
            "X|Target (default ratio based): entry close + 1 × (entry close − ST1 line); sell at the target when the high reaches it, or at the open − 0.5 % when the candle opens at or above it",
            "X|Otherwise sell at the close when ST1 and ST2 are both down or the daily filter turns bull",
            "E|Short Only (setting): short when ST1 and ST2 are down under a bear filter; cover at the close when ST1 turns up or the filter turns bull, or at the target close − 1 × (ST1 line − close). Long & Short (setting) follows the AFL's own branch (exits only on target, filter or reversal)",
            BEAR_NO_SQ,
        ], "BEAR / Entry and Exit with Hourly ST with Daily ST OR Swing pct as Filter for Bear with PT and SL.afl (same folder; checked by Prasha & Nishtha), coded as the AFL is written with its default settings: filter ST, Long Only, PT type SL and PT Ratio Based (multiplier 1, buffer 0.5 %), on 60-minute candles. A target fill outside the candle's high-low range is clamped into it; a target with no ST line (the ST's first bars) is treated as no target. Positional, no time square-off. Not compared with the team's trade sheets."),
    ]
}

const BEAR_DATA_F: &str = "F|Data: 1-minute F&O futures (team AmiBroker IEOD database) compressed to 60-minute candles from 09:15 (the AFL folder's 25 / 60 / 75-minute options are the timeframe setting), each labelled with its start time";
const BEAR_ST_FILTER_F: &str = "F|Daily filter: ST(5,4) on the daily mid point (H+L)/2 of the daily ieodfo bars (the AFL's TimeFrameSet daily); bear when the daily ST is down, bull when up";
const BEAR_FILTER_F: &str = "F|Daily filter (default ST): ST(5,4) on the daily mid point (H+L)/2 of the daily ieodfo bars (the AFL's TimeFrameSet daily); setting: swing % filter (23 % bear / 44 % bull swings of the daily close)";
const BEAR_EXPAND_F: &str = "F|The day's filter value applies from the day's last candle; the day's earlier candles use the previous day's value (TimeFrameExpand, expandLast)";
const BEAR_ST_F: &str = "F|Hourly ST1 = ST(5,1) and ST2 = ST(5,2), the AFL's ST loop on the candle mid point (H+L)/2";
const BEAR_LONG_E: &str = "E|Default Long Only: buy at the candle's close when flat, the daily filter is bear and ST1 is up";
const BEAR_LONG_X: &str = "X|Sell at the close when ST1 and ST2 are both down or the daily filter turns bull";
const BEAR_SHORT_EX: &str = "E|Short Only / Long & Short (setting): short at the close when ST1 and ST2 are down and the daily filter is bear; cover when ST1 turns up or the filter turns bull";
const BEAR_NO_SQ: &str = "X|No time square-off: positions carry overnight, as the AFL is written";

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let mut v = base_params(id)?;
    v.push(intraday::align_param());
    Some(v)
}

fn base_params(id: &str) -> Option<Vec<ParamDef>> {
    if BEAR_IDS.contains(&id) {
        return Some(bear_params(id));
    }
    if !IDS.contains(&id) {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![
        p("tf_min", "Timeframe in minutes (the AFL header: 5 Minute)", 5.0, 1.0, 60.0, 1.0),
        p("st_period", "ST period", 5.0, 1.0, 20.0, 1.0),
        p("st_factor", "ST factor", 2.0, 1.0, 20.0, 1.0),
        p("st_point", "ST calculation point (1 mid point (H+L)/2, 2 close, 3 pivot (H+L+C)/3)", 1.0, 1.0, 3.0, 1.0),
        p("sq_time", "Square-off time (HHMM)", 1520.0, 1200.0, 1530.0, 5.0),
    ])
}

/// Settings of one run, read from the params.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cfg {
    pub fresh: bool,
    pub tf_min: u16,
    pub period: usize,
    pub factor: f64,
    pub point: i64,
    pub sq_time: u16,
}

fn cfg(id: &str, g: &dyn Fn(&str) -> f64) -> Cfg {
    Cfg {
        fresh: id == FRESH,
        tf_min: g("tf_min").round().clamp(1.0, 375.0) as u16,
        period: g("st_period").round().max(1.0) as usize,
        factor: g("st_factor"),
        point: g("st_point").round() as i64,
        sq_time: g("sq_time").round().clamp(0.0, 2359.0) as u16,
    }
}

fn hm(t: u16) -> String {
    format!("{:02}:{:02}", t / 100, t % 100)
}

/// The AFL's position loop on candles `b` (all loaded candles; `di[k]` = daily index of candle k), trades whose
/// entry daily bar is in from..=to.
pub(crate) struct TimedTrade { pub trade:Trade, pub entry_candle:usize, pub exit_candle:usize }

pub(crate) fn trades_on(b: &Candles, di: &[usize], x: &Ctx, from: usize, to: usize, c: Cfg) -> Vec<Trade> {
    timed_trades_on(b,di,x,from,to,c).into_iter().map(|v|v.trade).collect()
}

fn timed_trades_on(b: &Candles, di: &[usize], x: &Ctx, from: usize, to: usize, c: Cfg) -> Vec<TimedTrade> {
    let n = b.len();
    if n == 0 {
        return Vec::new();
    }
    let calc = calc_on(&b.h, &b.l, &b.c, c.point, false);
    let (tr, _, _) = st_lines(&calc, &b.h, &b.l, &b.c, c.period, c.factor);
    // the last candle of the selected period: positions still open there are closed at its close
    let end = match (0..n).rev().find(|&k| di[k] <= to) {
        Some(k) => k,
        None => return Vec::new(),
    };
    let date = |k: usize| x.r[di[k]].date.as_str();
    let mut out: Vec<TimedTrade> = Vec::new();
    // open position: (dir, entry candle, why)
    let mut pos: Option<(f64, usize, &'static str)> = None;
    let close = |out: &mut Vec<TimedTrade>, (dir, e, why): (f64, usize, &'static str), k: usize, how: &str, open: bool| {
        if di[e] < from || di[e] > to {
            return;
        }
        let when = if b.day[k] == b.day[e] { hm(b.hhmm[k]) } else { format!("{} {}", date(k), hm(b.hhmm[k])) };
        let side = if dir > 0.0 { "Long" } else { "Short" };
        let reason = format!("{side} {} ({why}) → {how} {when}; {}-minute candles, fills at the candle's close", hm(b.hhmm[e]), c.tf_min);
        out.push(TimedTrade { trade:Trade { dir, legs: vec![Leg { i: di[e], px: b.c[e], w: 1.0 }], exit_i: di[k], exit_px: b.c[k], reason, entry_kind: "close", signal_i: di[k], open, exits: Vec::new() },entry_candle:e,exit_candle:k });
    };
    for k in 1..=end {
        let t = b.hhmm[k];
        let state = pos.map_or(0.0, |p| p.0);
        let flip_up = !c.fresh || tr[k - 1] == -1;
        let flip_dn = !c.fresh || tr[k - 1] == 1;
        let why_up = if c.fresh { "ST turned up" } else { "ST up" };
        let why_dn = if c.fresh { "ST turned down" } else { "ST down" };
        if state != 1.0 && tr[k] == 1 && flip_up && t < c.sq_time {
            if let Some(p) = pos.take() {
                close(&mut out, p, k, "reversed", false);
            }
            pos = Some((1.0, k, why_up));
        } else if state != -1.0 && tr[k] == -1 && flip_dn && t < c.sq_time {
            if let Some(p) = pos.take() {
                close(&mut out, p, k, "reversed", false);
            }
            pos = Some((-1.0, k, why_dn));
        } else if state != 0.0 && t >= c.sq_time {
            if let Some(p) = pos.take() {
                close(&mut out, p, k, "square-off", false);
            }
        }
    }
    if let Some(p) = pos {
        // still open at the period's last candle: closed there (books::close_at_end words it as end of backtest)
        close(&mut out, p, end, "open", true);
    }
    out
}

/// Candles of `m` on the run's timeframe, restricted to the dates the daily carrier has, with their daily indices.
pub(crate) fn candles_for(m: &Minutes, x: &Ctx, tf_min: u16, align: intraday::Align) -> (Candles, Vec<usize>) {
    let map = intraday::day_index(m, x.r);
    let all = intraday::compress_aligned(m, tf_min, align);
    let keep: Vec<usize> = (0..all.len()).filter(|&k| map[all.day[k] as usize].is_some()).collect();
    let pick = |v: &[f64]| keep.iter().map(|&k| v[k]).collect::<Vec<f64>>();
    let b = Candles {
        day: keep.iter().map(|&k| all.day[k]).collect(),
        hhmm: keep.iter().map(|&k| all.hhmm[k]).collect(),
        o: pick(&all.o),
        h: pick(&all.h),
        l: pick(&all.l),
        c: pick(&all.c),
        v: pick(&all.v),
        oi: keep.iter().map(|&k| all.oi[k]).collect(),
        start: keep.iter().map(|&k| all.start[k]).collect(),
        end: keep.iter().map(|&k| all.end[k]).collect(),
    };
    let di = b.day.iter().map(|&d| map[d as usize].unwrap()).collect();
    (b, di)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    // The daily carrier's rows hold the symbol; admission (instrument_policy / readiness) has already required an
    // ieodfo series with an intraday file, so a missing file here only means no trades.
    let Some(m) = intraday::minutes(&format!("{}{}", intraday::BASKET_PREFIX, x.r[0].ticker)) else {
        return Some(Vec::new());
    };
    let align = intraday::align_of(p);
    if BEAR_IDS.contains(&id) {
        let c = bear_cfg(id, &g);
        let (b, di) = candles_for(&m, x, c.tf_min, align);
        let flt = expand_daily(&bear_filter(x, &c), &b, &di);
        return Some(recorded(&m, &b, c.tf_min, align, bear_timed(&b, &di, x, from, to, &flt, c)));
    }
    let c = cfg(id, &g);
    let (b, di) = candles_for(&m, x, c.tf_min, align);
    Some(recorded(&m, &b, c.tf_min, align, timed_trades_on(&b, &di, x, from, to, c)))
}

/// The trades of `timed`, with their intraday timings recorded for the run report (`intraday::record`).
fn recorded(m: &Minutes, b: &Candles, tf: u16, align: intraday::Align, timed: Vec<TimedTrade>) -> Vec<Trade> {
    let pairs: Vec<(usize, usize)> = timed.iter().map(|t| (t.entry_candle, t.exit_candle)).collect();
    let trades: Vec<Trade> = timed.into_iter().map(|t| t.trade).collect();
    intraday::record(m, b, tf, align, &trades, &pairs);
    trades
}

/// Source-observed pilot events for the held portfolio research path. This adapter
/// carries candle indices directly from the strategy loop, never from trade note text.
pub(crate) fn portfolio_observations(id:&str,x:&Ctx,from:usize,to:usize,p:&Params,first_id:u64)
    ->Result<(Vec<crate::yt_intraday_portfolio::Signal>,Vec<crate::yt_intraday_portfolio::Mark>),String> {
    use crate::yt_intraday_portfolio::{Signal,Mark};
    let schema=params(id).ok_or("unknown intraday Supertrend strategy")?;
    intraday::validate_research_params(&schema,p)?;
    if BEAR_IDS.contains(&id) {return bear_portfolio_observations(id,&schema,x,from,to,p,first_id);}
    if x.r.is_empty() || from>to || to>=x.r.len(){return Err("invalid selected daily coverage".into());}
    let ticker=format!("{}{}",intraday::BASKET_PREFIX,x.r[0].ticker);
    intraday::available(&ticker)?;
    let minutes=intraday::minutes(&ticker).ok_or("intraday bars are unavailable")?;
    intraday::validate_daily_carrier(&minutes,x.r,to)?;
    let g=|k:&str|p.get(k).copied().unwrap_or_else(||schema.iter().find(|d|d.key==k).map_or(0.,|d|d.def));
    let config=cfg(id,&g);let (bars,di)=candles_for(&minutes,x,config.tf_min,crate::intraday::align_of(p));
    let mut marks=Vec::new();
    let timestamp=|date:&str,hhmm:u16,phase:i64|->Result<i64,String>{
        if !crate::baskets::valid_date(date) || hhmm/100>23 || hhmm%100>59 {return Err("invalid source date or minute".into());}
        Ok(crate::day_number(date)*2880+i64::from(hhmm/100*60+hhmm%100)*2+phase)
    };
    for i in 0..minutes.len() {
        let d=minutes.date(i);
        if d<x.r[from].date.as_str() || d>x.r[to].date.as_str(){continue;}
        if marks.len()>=2_000_000 {return Err("selected source exceeds the bounded intraday mark limit".into());}
        marks.push(Mark{time:timestamp(d,minutes.hhmm[i],0)?,symbol:x.r[0].ticker.clone(),price:intraday::price(minutes.o[i])});
        marks.push(Mark{time:timestamp(d,minutes.hhmm[i],1)?,symbol:x.r[0].ticker.clone(),price:intraday::price(minutes.c[i])});
    }
    let timed=timed_trades_on(&bars,&di,x,from,to,config);
    let mut signals=Vec::with_capacity(timed.len());
    for (i,t) in timed.into_iter().enumerate() {
        let e=t.entry_candle;let z=t.exit_candle;
        let when=|k:usize|timestamp(minutes.date(bars.end[k]),minutes.hhmm[bars.end[k]],1);
        let entry_time=when(e)?;
        let exit=if t.trade.open {None}else{Some((when(z)?,t.trade.exit_px))};
        // The AFL leaves PositionScore undefined and defaults to one share. Equal
        // sizes therefore use alphabetical order; fixed notional sizing is an adaptation.
        signals.push(Signal{id:first_id.checked_add(i as u64).ok_or("portfolio signal ID overflow")?,symbol:x.r[0].ticker.clone(),entry_time,entry_price:t.trade.legs[0].px,
            score:0.,direction:if t.trade.dir>0. {1}else{-1},exit});
    }
    Ok((signals,marks))
}

// ---------------------------------------------------------------- Hourly ST with daily ST / swing % bear filter

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BearKind {
    /// BEAR VER2: short only, case 1 / case 2 exits.
    V2,
    /// Diff ST / Single ST: ExRem'd Buy / Sell / Short / Cover arrays.
    Signals,
    /// PT and SL: the AFL's loop with targets and re-arm flags.
    Pt,
}

/// Settings of one run of the bear-filter AFLs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BearCfg {
    pub kind: BearKind,
    pub tf_min: u16,
    pub p1: usize,
    pub f1: f64,
    pub p2: usize,
    pub f2: f64,
    pub point: i64,
    pub swing: bool,
    pub filter_tf: i64,
    pub fpoint: i64,
    pub fperiod: usize,
    pub ffactor: f64,
    pub bear_pct: f64,
    pub bull_pct: f64,
    pub long: bool,
    pub short: bool,
    /// 0 SL and PT ratio based, 1 PT percentage, 2 none (PT AFL only).
    pub pt_type: i64,
    pub pt_multi: f64,
    pub pt_pct: f64,
    pub buffer: f64,
}

fn bear_params(id: &str) -> Vec<ParamDef> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![
        p("tf_min", "Timeframe in minutes (the AFL folder: 25, 60 or 75; header Hourly)", 60.0, 1.0, 375.0, 1.0),
        p("st1_period", "Hourly ST1 (entry) period", 5.0, 1.0, 20.0, 1.0),
        p("st1_factor", "Hourly ST1 (entry) factor", 1.0, 0.5, 20.0, 0.5),
    ];
    if id != BEAR_SINGLE {
        v.push(p("st2_period", "Hourly ST2 (exit) period", 5.0, 1.0, 20.0, 1.0));
        v.push(p("st2_factor", "Hourly ST2 (exit) factor", 2.0, 0.5, 20.0, 0.5));
    }
    v.push(p("st_point", "Hourly ST calculation point (1 mid point, 2 close, 3 pivot, 4 MP/CL/pivot avg, 5 MA(C,3), 6 MA(C,5))", 1.0, 1.0, 6.0, 1.0));
    if id != BEAR_V2 {
        v.push(p("filter", "Bear market filter (0 ST, 1 swing %)", 0.0, 0.0, 1.0, 1.0));
        v.push(p("ver", "Trade type (0 Long Only, 1 Short Only, 2 Long & Short)", 0.0, 0.0, 2.0, 1.0));
    }
    v.extend([
        p("filter_tf", "Filter timeframe (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
        p("filter_period", "Filter ST period", 5.0, 1.0, 20.0, 1.0),
        p("filter_factor", "Filter ST factor", 4.0, 1.0, 20.0, 1.0),
        p("filter_point", "Filter ST calculation point (1 mid point .. 6 MA(C,5), as the hourly list)", 1.0, 1.0, 6.0, 1.0),
    ]);
    if id != BEAR_V2 {
        v.push(p("bear_pct", "Swing % for bear market", 23.0, 5.0, 50.0, 1.0));
        v.push(p("bull_pct", "Swing % for bull market", 44.0, 5.0, 50.0, 1.0));
    }
    if id == BEAR_PT {
        v.extend([
            p("pt_type", "PT type (0 SL and PT ratio based, 1 PT percentage, 2 none)", 0.0, 0.0, 2.0, 1.0),
            p("pt_multi", "PT multiplier (ratio based)", 1.0, 0.0, 15.0, 0.01),
            p("pt_pct", "PT percent (percentage based)", 1.0, 0.0, 10.0, 0.01),
            p("buffer_pct", "Open buffer percent (target gap fill)", 0.5, 0.0, 5.0, 0.01),
        ]);
    }
    v
}

fn bear_cfg(id: &str, g: &dyn Fn(&str) -> f64) -> BearCfg {
    let kind = match id {
        BEAR_V2 => BearKind::V2,
        BEAR_PT => BearKind::Pt,
        _ => BearKind::Signals,
    };
    let per = |k: &str| g(k).round().clamp(1.0, 100.0) as usize;
    let (p1, f1) = (per("st1_period"), g("st1_factor"));
    let (p2, f2) = if id == BEAR_SINGLE { (p1, f1) } else { (per("st2_period"), g("st2_factor")) };
    let ver = if kind == BearKind::V2 { 1 } else { g("ver").round() as i64 };
    BearCfg {
        kind,
        tf_min: g("tf_min").round().clamp(1.0, 375.0) as u16,
        p1,
        f1,
        p2,
        f2,
        point: g("st_point").round().clamp(1.0, 6.0) as i64,
        swing: kind != BearKind::V2 && g("filter") >= 0.5,
        filter_tf: g("filter_tf").round() as i64,
        fpoint: g("filter_point").round().clamp(1.0, 6.0) as i64,
        fperiod: per("filter_period"),
        ffactor: g("filter_factor"),
        bear_pct: g("bear_pct"),
        bull_pct: g("bull_pct"),
        long: ver != 1,
        short: ver != 0,
        pt_type: if kind == BearKind::Pt { g("pt_type").round() as i64 } else { 2 },
        pt_multi: g("pt_multi"),
        pt_pct: g("pt_pct"),
        buffer: g("buffer_pct"),
    }
}

/// The daily (weekly / monthly) filter trend on the daily carrier rows: the AFLs' TimeFrameSet(TF) ST loop or swing %
/// loop on the compressed bars, expanded to the daily rows with expandLast.
fn bear_filter(x: &Ctx, c: &BearCfg) -> Vec<i8> {
    let tf = match c.filter_tf {
        1 => 'W',
        2 => 'M',
        _ => 'D',
    };
    if c.swing {
        let bars = crate::books::ytsw::compress(x, tf);
        let t = crate::books::ytrsi::swing_filter(&bars.c, c.bear_pct, c.bull_pct);
        crate::books::ytsw::expand_last(&t, &bars)
    } else {
        crate::books::ytrsi::st_trend(x, tf, c.fpoint, c.fperiod, c.ffactor)
    }
}

/// TimeFrameExpand(daily, inDaily) with expandLast on the candles: the day's value from the day's last candle, the
/// previous day's value on the day's earlier candles (0 = no value yet), as `ytrsi_ieod.rs`.
pub(crate) fn expand_daily(daily: &[i8], b: &Candles, di: &[usize]) -> Vec<i8> {
    (0..b.len()).map(|k| if b.last_of_day(k) { daily[di[k]] } else { di[k].checked_sub(1).map_or(0, |j| daily[j]) }).collect()
}

struct BearPos {
    dir: f64,
    e: usize,
    why: &'static str,
    target: Option<f64>,
    case2: bool,
}

/// The bear-filter AFLs on candles `b` (all loaded candles, `di[k]` = daily index, `flt[k]` = expanded filter trend),
/// trades whose entry daily bar is in from..=to.
pub(crate) fn bear_timed(b: &Candles, di: &[usize], x: &Ctx, from: usize, to: usize, flt: &[i8], c: BearCfg) -> Vec<TimedTrade> {
    let n = b.len();
    if n == 0 {
        return Vec::new();
    }
    let calc = calc_on(&b.h, &b.l, &b.c, c.point, false);
    let (t1, lu1, ld1) = st_lines(&calc, &b.h, &b.l, &b.c, c.p1, c.f1);
    let (t2, _, _) = st_lines(&calc, &b.h, &b.l, &b.c, c.p2, c.f2);
    let Some(end) = (0..n).rev().find(|&k| di[k] <= to) else { return Vec::new() };
    let date = |k: usize| x.r[di[k]].date.as_str();
    let mut out: Vec<TimedTrade> = Vec::new();
    let close = |out: &mut Vec<TimedTrade>, p: &BearPos, k: usize, px: f64, how: &str, open: bool| {
        if di[p.e] < from || di[p.e] > to {
            return;
        }
        let when = if b.day[k] == b.day[p.e] { hm(b.hhmm[k]) } else { format!("{} {}", date(k), hm(b.hhmm[k])) };
        let side = if p.dir > 0.0 { "Long" } else { "Short" };
        let reason = format!("{side} {} ({}) → {how} {when}; {}-minute candles", hm(b.hhmm[p.e]), p.why, c.tf_min);
        out.push(TimedTrade { trade: Trade { dir: p.dir, legs: vec![Leg { i: di[p.e], px: b.c[p.e], w: 1.0 }], exit_i: di[k], exit_px: px, reason, entry_kind: "close", signal_i: di[k], open, exits: Vec::new() }, entry_candle: p.e, exit_candle: k });
    };
    // target fills: the target, or the open less (long) / plus (short) the buffer when the candle opens through it;
    // kept inside the candle's range
    let long_fill = |k: usize, t: f64| (if b.o[k] >= t { b.o[k] - b.o[k] * c.buffer / 100.0 } else { t }).clamp(b.l[k], b.h[k]);
    let short_fill = |k: usize, t: f64| (if b.o[k] <= t { b.o[k] + b.o[k] * c.buffer / 100.0 } else { t }).clamp(b.l[k], b.h[k]);
    let long_target = |k: usize| match c.pt_type {
        0 => lu1[k].map(|d| b.c[k] + c.pt_multi * (b.c[k] - d)),
        1 => Some(b.c[k] + b.c[k] * c.pt_pct / 100.0),
        _ => None,
    };
    let short_target = |k: usize| match c.pt_type {
        0 => ld1[k].map(|u| b.c[k] - c.pt_multi * (u - b.c[k])),
        1 => Some(b.c[k] - b.c[k] * c.pt_pct / 100.0),
        _ => None,
    };
    let mut pos: Option<BearPos> = None;
    let (mut buysig, mut shortsig) = (2u8, 2u8);
    for k in 1..=end {
        let (fbull, fbear) = (flt[k] == 1, flt[k] == -1);
        let (a, z) = (t1[k], t2[k]);
        match c.kind {
            BearKind::V2 => {
                if pos.is_none() && fbear && a == -1 {
                    let why = if z == 1 { "ST1 down, ST2 up; daily bear" } else { "ST1 and ST2 down; daily bear" };
                    pos = Some(BearPos { dir: -1.0, e: k, why, target: None, case2: z == -1 });
                }
                if let Some(p) = pos.as_mut() {
                    if !p.case2 && z == -1 {
                        p.case2 = true;
                    } else if (!p.case2 && a == 1) || fbull || (p.case2 && z == 1) {
                        let how = if fbull { "daily filter bull" } else if p.case2 { "ST2 up" } else { "ST1 up" };
                        close(&mut out, p, k, b.c[k], how, false);
                        pos = None;
                    }
                }
            }
            BearKind::Signals => {
                if let Some(p) = &pos {
                    let exit = if p.dir > 0.0 { (a == -1 && z == -1) || fbull } else { fbull || a == 1 };
                    if exit {
                        let how = if fbull { "daily filter bull" } else if p.dir > 0.0 { "ST1 and ST2 down" } else { "ST1 up" };
                        close(&mut out, p, k, b.c[k], how, false);
                        pos = None;
                    }
                }
                if pos.is_none() {
                    if c.long && fbear && a == 1 {
                        pos = Some(BearPos { dir: 1.0, e: k, why: "ST1 up; daily bear", target: None, case2: false });
                    } else if c.short && fbear && a == -1 && z == -1 {
                        pos = Some(BearPos { dir: -1.0, e: k, why: "ST1 and ST2 down; daily bear", target: None, case2: false });
                    }
                }
            }
            BearKind::Pt => {
                let state = pos.as_ref().map_or(0.0, |p| p.dir);
                let buy_ok = fbear && a == 1;
                let short_ok = fbear && a == -1 && z == -1;
                let hit_long = |p: &BearPos| p.target.is_some_and(|t| b.h[k] >= t);
                let hit_short = |p: &BearPos| p.target.is_some_and(|t| b.l[k] <= t);
                if c.long && c.short {
                    if buysig == 1 && z == -1 && a == -1 {
                        buysig = 2;
                    }
                    if shortsig == 1 && z == 1 {
                        shortsig = 2;
                    }
                    if state <= 0.0 && buysig == 2 && buy_ok {
                        if let Some(p) = pos.take() {
                            close(&mut out, &p, k, b.c[k], "reversed", false);
                        }
                        if shortsig == 1 {
                            shortsig = 2;
                        }
                        pos = Some(BearPos { dir: 1.0, e: k, why: "ST1 up; daily bear", target: long_target(k), case2: false });
                    } else if state > 0.0 && (pos.as_ref().is_some_and(hit_long) || fbull) {
                        let p = pos.take().unwrap();
                        if hit_long(&p) {
                            close(&mut out, &p, k, long_fill(k, p.target.unwrap()), "target", false);
                            buysig = 1;
                        } else {
                            close(&mut out, &p, k, b.c[k], "daily filter bull", false);
                        }
                    } else if state >= 0.0 && shortsig == 2 && short_ok {
                        if let Some(p) = pos.take() {
                            close(&mut out, &p, k, b.c[k], "reversed", false);
                        }
                        if buysig == 1 {
                            buysig = 2;
                        }
                        pos = Some(BearPos { dir: -1.0, e: k, why: "ST1 and ST2 down; daily bear", target: short_target(k), case2: false });
                    } else if state < 0.0 && (pos.as_ref().is_some_and(hit_short) || fbull) {
                        let p = pos.take().unwrap();
                        if hit_short(&p) {
                            close(&mut out, &p, k, short_fill(k, p.target.unwrap()), "target", false);
                            shortsig = 1;
                        } else {
                            close(&mut out, &p, k, b.c[k], "daily filter bull", false);
                        }
                    }
                } else if c.long {
                    if buysig == 1 && z == -1 && a == -1 {
                        buysig = 2;
                    }
                    if state == 0.0 && buysig == 2 && buy_ok {
                        pos = Some(BearPos { dir: 1.0, e: k, why: "ST1 up; daily bear", target: long_target(k), case2: false });
                    } else if state > 0.0 {
                        let p = pos.as_ref().unwrap();
                        let hit = hit_long(p);
                        if (a == -1 && z == -1) || fbull || hit {
                            let p = pos.take().unwrap();
                            if hit {
                                close(&mut out, &p, k, long_fill(k, p.target.unwrap()), "target", false);
                                buysig = 1;
                            } else {
                                close(&mut out, &p, k, b.c[k], if fbull { "daily filter bull" } else { "ST1 and ST2 down" }, false);
                            }
                        }
                    }
                } else {
                    if shortsig == 1 && z == 1 {
                        shortsig = 2;
                    }
                    if state == 0.0 && shortsig == 2 && short_ok {
                        pos = Some(BearPos { dir: -1.0, e: k, why: "ST1 and ST2 down; daily bear", target: short_target(k), case2: false });
                    } else if state < 0.0 {
                        let p = pos.as_ref().unwrap();
                        let hit = hit_short(p);
                        if a == 1 || fbull || hit {
                            let p = pos.take().unwrap();
                            if hit {
                                close(&mut out, &p, k, short_fill(k, p.target.unwrap()), "target", false);
                                shortsig = 1;
                            } else {
                                close(&mut out, &p, k, b.c[k], if fbull { "daily filter bull" } else { "ST1 up" }, false);
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(p) = pos {
        close(&mut out, &p, end, b.c[end], "open", true);
    }
    out
}

/// Close-filled bear-filter paths join the held portfolio research path; target fills inside a candle do not.
fn bear_portfolio_observations(id:&str,schema:&[ParamDef],x:&Ctx,from:usize,to:usize,p:&Params,first_id:u64)
    ->Result<(Vec<crate::yt_intraday_portfolio::Signal>,Vec<crate::yt_intraday_portfolio::Mark>),String> {
    let g=|k:&str|p.get(k).copied().unwrap_or_else(||schema.iter().find(|d|d.key==k).map_or(0.,|d|d.def));
    let c=bear_cfg(id,&g);
    if c.kind==BearKind::Pt && c.pt_type!=2 {return Err("needs_data: Supertrend target event timing within a candle is not established".into());}
    if x.r.is_empty() || from>to || to>=x.r.len(){return Err("invalid selected daily coverage".into());}
    let ticker=format!("{}{}",intraday::BASKET_PREFIX,x.r[0].ticker);
    intraday::available(&ticker)?;
    let minutes=intraday::minutes(&ticker).ok_or("intraday bars are unavailable")?;
    let (bars,di)=candles_for(&minutes,x,c.tf_min,crate::intraday::align_of(p));
    let flt=expand_daily(&bear_filter(x,&c),&bars,&di);
    let timed=bear_timed(&bars,&di,x,from,to,&flt,c).into_iter().map(|t|(t.trade,t.entry_candle,t.exit_candle)).collect();
    crate::yt_intraday_portfolio::close_observations(x,from,to,&minutes,&bars,timed,first_id)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Supertrend (Intraday)"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    #[test]
    fn local_supertrend_portfolio_events_use_actual_candle_close_when_configured() {
        let Ok(csv)=std::env::var("TEST_INTRADAY_DAILY") else {return};
        if std::env::var_os("INTRADAY_DATA_DIR").is_none(){return;}
        let text=std::fs::read_to_string(csv).unwrap();
        let rows:Vec<Row>=text.lines().skip(1).map(|line| {
            let v:Vec<_>=line.split(',').collect();
            Row{ticker:"BANKNIFTY".into(),date:v[0].into(),open:v[1].parse().unwrap(),high:v[2].parse().unwrap(),low:v[3].parse().unwrap(),close:v[4].parse().unwrap(),volume:v[5].parse().ok()}
        }).collect();
        let from=rows.iter().position(|r|r.date.as_str()>="2019-01-01").unwrap();
        let to=rows.iter().rposition(|r|r.date.as_str()<="2019-01-31").unwrap();
        let x=Ctx::new(&rows,&rows,&[]);
        let (events,marks)=portfolio_observations(FRESH,&x,from,to,&Params::new(),5).unwrap();
        assert!(!marks.is_empty());
        for e in &events {
            assert_eq!(e.score,0.);
            assert!(marks.iter().any(|m|m.time==e.entry_time && (m.price-e.entry_price).abs()<0.00011));
            assert!(e.exit.is_none_or(|(time,price)|time>e.entry_time && marks.iter().any(|m|m.time==time && (m.price-price).abs()<0.00011)));
        }
        let ledger=crate::yt_intraday_portfolio::run_strategy_research(FRESH,&[(&x,from,to)],&Params::new(),1000.,1,0.).unwrap();
        assert!(ledger.accepted.len()<=events.len());
    }

    fn day_rows(dates: &[&str]) -> Vec<Row> {
        dates.iter().map(|d| Row { ticker: "TEST".into(), date: (*d).into(), open: 100.0, high: 101.0, low: 99.0, close: 100.0, volume: None }).collect()
    }

    /// Minutes from (day, hhmm, close) with high / low = close ± 0.5.
    fn mins(dates: &[&str], v: &[(u32, u16, f32)]) -> Minutes {
        let mut m = Minutes { symbol: "TEST".into(), dates: dates.iter().map(|d| d.to_string()).collect(), ..Default::default() };
        for &(d, t, c) in v {
            m.day.push(d);
            m.hhmm.push(t);
            m.o.push(c);
            m.h.push(c + 0.5);
            m.l.push(c - 0.5);
            m.c.push(c);
            m.v.push(1.0);
        }
        m
    }

    fn label(minute: u16) -> u16 {
        minute / 60 * 100 + minute % 60
    }

    /// One day of 1-minute bars (09:15 .. 15:29) whose closes follow `f(minute index)`.
    fn day(d: u32, f: impl Fn(usize) -> f32) -> Vec<(u32, u16, f32)> {
        (0..375).map(|k| (d, label(555 + k as u16), f(k))).collect()
    }

    fn cfg1(fresh: bool) -> Cfg {
        Cfg { fresh, tf_min: 1, period: 2, factor: 1.0, point: 1, sq_time: 1520 }
    }

    fn go(m: &Minutes, r: &[Row], c: Cfg, from: usize, to: usize) -> Vec<Trade> {
        let x = Ctx::new(r, r, &[]);
        let (b, di) = candles_for(m, &x, c.tf_min,crate::intraday::Align::Session);
        trades_on(&b, &di, &x, from, to, c)
    }

    #[test]
    fn fresh_entry_on_flips_reverses_and_squares_off_at_1520() {
        // flat at 100, falls to 90 from minute 30 (09:45), rises to 120 from minute 120 (11:15)
        let f = |k: usize| if k < 30 { 100.0 } else if k < 120 { 90.0 } else { 120.0 };
        let dates = ["2022-03-01"];
        let m = mins(&dates, &day(0, f));
        let r = day_rows(&dates);
        let t = go(&m, &r, cfg1(true), 0, 0);
        assert_eq!(t.len(), 2, "{t:?}");
        // short on the 09:45 flip down at 90, reversed on the 11:15 flip up at 120; the long squared off at 15:20
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_px), (-1.0, 90.0, 120.0));
        assert!(t[0].reason.starts_with("Short 09:45 (ST turned down) → reversed 11:15"), "{}", t[0].reason);
        assert_eq!((t[1].dir, t[1].legs[0].px, t[1].exit_px), (1.0, 120.0, 120.0));
        assert!(t[1].reason.contains("Long 11:15 (ST turned up) → square-off 15:20"), "{}", t[1].reason);
        // same-day trades on the daily carrier: entry and exit on daily bar 0, accepted by trade validation
        assert!(t.iter().all(|t| t.legs[0].i == 0 && t.exit_i == 0 && !t.open));
        crate::trade_validation::check(&t, 0, 0, 0.0).unwrap();
    }

    #[test]
    fn running_entry_takes_the_current_trend_each_morning() {
        // day 1 falls at 09:25 and rises at 09:45; day 2 keeps rising. The fresh AFL trades the two day-1 flips only;
        // the running AFL also buys at 09:16 (the trend starts up) and at day 2's first candle, because the trend is
        // still up after the 15:20 square-off
        let f = |k: usize| if k < 10 { 100.0 } else if k < 30 { 90.0 } else { 110.0 + (k as f32) * 0.01 };
        let dates = ["2022-03-01", "2022-03-02"];
        let mut v = day(0, f);
        v.extend(day(1, |k| 115.0 + (k as f32) * 0.01));
        let m = mins(&dates, &v);
        let r = day_rows(&dates);
        let fresh = go(&m, &r, cfg1(true), 0, 1);
        assert_eq!(fresh.iter().map(|t| t.dir).collect::<Vec<_>>(), vec![-1.0, 1.0], "{fresh:?}");
        let running = go(&m, &r, cfg1(false), 0, 1);
        assert_eq!(running.iter().map(|t| t.dir).collect::<Vec<_>>(), vec![1.0, -1.0, 1.0, 1.0], "{running:?}");
        assert!(running[0].reason.starts_with("Long 09:16 (ST up) → reversed 09:25"), "{}", running[0].reason);
        assert_eq!((running[3].legs[0].i, running[3].exit_i), (1, 1));
        assert!(running[3].reason.starts_with("Long 09:15 (ST up) → square-off 15:20"), "{}", running[3].reason);
        crate::trade_validation::check(&running, 0, 1, 0.0).unwrap();
        // the period filter: only day 2's entry when the run starts on day 2
        assert_eq!(go(&m, &r, cfg1(false), 1, 1).len(), 1);
        assert_eq!(go(&m, &r, cfg1(true), 1, 1).len(), 0);
    }

    #[test]
    fn no_entry_at_or_after_the_square_off_time_and_carry_without_a_1520_bar() {
        // the flip comes at 15:22: no entry (TimeNow >= 1520)
        let f = |k: usize| if k < 367 { 100.0 } else { 90.0 };
        let dates = ["2022-03-01", "2022-03-02"];
        let m = mins(&dates, &day(0, f));
        let r = day_rows(&dates[..1]);
        assert!(go(&m, &r, cfg1(true), 0, 0).is_empty());
        // a short session ending 12:44 has no candle at 15:20: the short is carried to the next day's square-off
        let mut v: Vec<(u32, u16, f32)> = (0..60).map(|k| (0, label(675 + k), if k < 20 { 100.0 } else { 90.0 })).collect();
        v.extend(day(1, |_| 90.0));
        let m = mins(&dates, &v);
        let r = day_rows(&dates);
        let t = go(&m, &r, cfg1(true), 0, 1);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (0, 1));
        assert!(t[0].reason.contains("square-off 2022-03-02 15:20"), "{}", t[0].reason);
        // ending the period on day 1 leaves it open at the day's last candle (closed by books::close_at_end)
        let t = go(&m, &r, cfg1(true), 0, 0);
        assert!(t[0].open && t[0].exit_i == 0);
    }

    #[test]
    fn five_minute_candles_square_off_at_the_1520_candle_close() {
        // falls at 10:00; with 5-minute candles the short is covered at the 15:20 candle's close (the 15:24 minute)
        let f = |k: usize| if k < 45 { 100.0 } else if k < 369 { 90.0 } else { 80.0 };
        let dates = ["2022-03-01"];
        let m = mins(&dates, &day(0, f));
        let r = day_rows(&dates);
        let c = Cfg { tf_min: 5, ..cfg1(true) };
        let t = go(&m, &r, c, 0, 0);
        assert_eq!(t.len(), 1, "{t:?}");
        assert!(t[0].reason.starts_with("Short 10:00 (ST turned down) → square-off 15:20; 5-minute candles"), "{}", t[0].reason);
        assert_eq!(t[0].exit_px, 80.0); // 15:24 close (minute 369 = 15:24)
    }

    fn bcfg(kind: BearKind) -> BearCfg {
        BearCfg { kind, tf_min: 1, p1: 2, f1: 1.0, p2: 2, f2: 2.0, point: 1, swing: false, filter_tf: 0, fpoint: 1, fperiod: 5, ffactor: 4.0, bear_pct: 23.0, bull_pct: 44.0, long: true, short: false, pt_type: 2, pt_multi: 1.0, pt_pct: 1.0, buffer: 0.5 }
    }

    /// One 1-minute day, the daily filter given per candle.
    fn go_bear(f: impl Fn(usize) -> f32, flt: impl Fn(usize) -> i8, c: BearCfg) -> Vec<Trade> {
        let dates = ["2022-03-01"];
        let m = mins(&dates, &day(0, f));
        let r = day_rows(&dates);
        let x = Ctx::new(&r, &r, &[]);
        let (b, di) = candles_for(&m, &x, c.tf_min,crate::intraday::Align::Session);
        let flt: Vec<i8> = (0..b.len()).map(flt).collect();
        bear_timed(&b, &di, &x, 0, 0, &flt, c).into_iter().map(|t| t.trade).collect()
    }

    #[test]
    fn bear_v2_case1_covers_on_st1_and_case2_on_st2_or_filter() {
        // 98.5 at 09:45 turns ST(2,1) down but not ST(2,2): case-1 short, covered when ST1 turns up at 10:15 (101)
        let f = |k: usize| if k < 30 { 100.0 } else if k < 60 { 98.5 } else { 101.0 };
        let t = go_bear(f, |_| -1, bcfg(BearKind::V2));
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_px), (-1.0, 98.5, 101.0));
        assert!(t[0].reason.starts_with("Short 09:45 (ST1 down, ST2 up; daily bear) → ST1 up 10:15"), "{}", t[0].reason);
        // 97 at 10:00 turns ST2 down too (case 2): ST1 turning up at 10:45 (98.6) no longer covers; ST2 up at 11:15 does
        let f = |k: usize| if k < 30 { 100.0 } else if k < 45 { 98.5 } else if k < 90 { 97.0 } else if k < 120 { 98.6 } else { 101.0 };
        let t = go_bear(f, |_| -1, bcfg(BearKind::V2));
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!((t[0].legs[0].px, t[0].exit_px), (98.5, 101.0));
        assert!(t[0].reason.contains("→ ST2 up 11:15"), "{}", t[0].reason);
        // the daily filter turning bull covers at that candle's close; no entry while the filter is bull
        let t = go_bear(f, |k| if k < 100 { -1 } else { 1 }, bcfg(BearKind::V2));
        assert_eq!((t.len(), t[0].exit_px), (1, 98.6));
        assert!(t[0].reason.contains("daily filter bull 10:55"), "{}", t[0].reason);
        assert!(go_bear(f, |_| 1, bcfg(BearKind::V2)).is_empty());
        crate::trade_validation::check(&t, 0, 0, 0.0).unwrap();
    }

    #[test]
    fn bear_diff_and_single_st_long_only_default_and_short_setting() {
        let f = |k: usize| if k < 30 { 100.0 } else if k < 45 { 98.5 } else if k < 90 { 97.0 } else { 98.6 };
        // Diff ST: buy at 09:16 (ST1 up from the start, filter bear), sold when ST1 and ST2 are both down (10:00, 97),
        // bought again when ST1 turns up (10:45); that one is open at the period's end
        let t = go_bear(f, |_| -1, bcfg(BearKind::Signals));
        assert_eq!(t.len(), 2, "{t:?}");
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_px), (1.0, 100.0, 97.0));
        assert!(t[0].reason.starts_with("Long 09:16 (ST1 up; daily bear) → ST1 and ST2 down 10:00"), "{}", t[0].reason);
        assert!(t[1].open && t[1].legs[0].px == 98.6);
        // Single ST: the exit ST is ST1 itself, so the 09:45 dip (ST1 down) already sells
        let single = BearCfg { p2: 2, f2: 1.0, ..bcfg(BearKind::Signals) };
        let t = go_bear(f, |_| -1, single);
        assert_eq!((t[0].legs[0].px, t[0].exit_px), (100.0, 98.5));
        // Short Only setting: short when both STs are down (10:00, 97), covered when ST1 turns up (10:45, 98.6)
        let t = go_bear(f, |_| -1, BearCfg { long: false, short: true, ..bcfg(BearKind::Signals) });
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_px), (-1.0, 97.0, 98.6));
        // Long & Short: the 10:00 sell is also the short, the 10:45 cover also the buy
        let t = go_bear(f, |_| -1, BearCfg { short: true, ..bcfg(BearKind::Signals) });
        assert_eq!(t.iter().map(|t| (t.dir, t.legs[0].px)).collect::<Vec<_>>(), vec![(1.0, 100.0), (-1.0, 97.0), (1.0, 98.6)]);
    }

    #[test]
    fn bear_pt_ratio_target_fill_and_rearm() {
        // ST(1,1): the 09:16 buy at 100 has ST1 line 99, so the ratio target is 101; the 09:25 candle (close 100.8,
        // high 101.3) reaches it and sells at 101. No new buy until ST1 and ST2 have both been down (10:15 drop to 90);
        // the next buy is at 11:15 (110)
        let f = |k: usize| if k < 10 { 100.0 } else if k < 60 { 100.8 } else if k < 120 { 90.0 } else { 110.0 };
        let c = BearCfg { p1: 1, p2: 1, pt_type: 0, ..bcfg(BearKind::Pt) };
        let t = go_bear(f, |_| -1, c);
        assert_eq!(t.len(), 2, "{t:?}");
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_px), (1.0, 100.0, 101.0));
        assert!(t[0].reason.starts_with("Long 09:16 (ST1 up; daily bear) → target 09:25"), "{}", t[0].reason);
        assert!(t[1].open && t[1].legs[0].px == 110.0);
        // the candle opening above the target fills at the open less the 0.5 % buffer (kept in the candle's range)
        let f = |k: usize| if k < 10 { 100.0 } else { 102.0 };
        let t = go_bear(f, |_| -1, c);
        assert_eq!(t[0].exit_px, 101.5);
        // PT type none: the same path holds until the filter turns bull
        let t = go_bear(f, |k| if k < 200 { -1 } else { 1 }, BearCfg { pt_type: 2, ..c });
        assert_eq!((t.len(), t[0].exit_px), (1, 102.0));
        assert!(t[0].reason.contains("daily filter bull"), "{}", t[0].reason);
    }

    #[test]
    fn daily_filter_expands_from_the_days_last_candle() {
        let dates = ["2022-03-01", "2022-03-02"];
        let mut v = day(0, |_| 100.0);
        v.extend(day(1, |_| 100.0));
        let m = mins(&dates, &v);
        let r = day_rows(&dates);
        let x = Ctx::new(&r, &r, &[]);
        let (b, di) = candles_for(&m, &x, 60,crate::intraday::Align::Session);
        let e = expand_daily(&[-1, 1], &b, &di);
        // 60-minute candles from 09:15: 09:15 .. 15:15 (7 per day)
        assert_eq!(e, vec![0, 0, 0, 0, 0, 0, -1, -1, -1, -1, -1, -1, -1, 1]);
    }

    #[test]
    fn bear_defaults_and_registration() {
        let d = |id: &str, k: &str| params(id).unwrap().iter().find(|p| p.key == k).map(|p| p.def);
        for id in BEAR_IDS {
            assert_eq!((d(id, "tf_min"), d(id, "st1_period"), d(id, "st1_factor"), d(id, "filter_factor")), (Some(60.0), Some(5.0), Some(1.0), Some(4.0)));
        }
        assert_eq!((d(BEAR_DIFF, "st2_factor"), d(BEAR_SINGLE, "st2_factor"), d(BEAR_V2, "ver")), (Some(2.0), None, None));
        assert_eq!((d(BEAR_PT, "pt_type"), d(BEAR_PT, "pt_multi"), d(BEAR_PT, "buffer_pct")), (Some(0.0), Some(1.0), Some(0.5)));
        let g = |id: &'static str| move |k: &str| params(id).unwrap().iter().find(|p| p.key == k).map_or(0.0, |p| p.def);
        let c = bear_cfg(BEAR_SINGLE, &g(BEAR_SINGLE));
        assert_eq!((c.p2, c.f2, c.long, c.short, c.swing), (5, 1.0, true, false, false));
        let c = bear_cfg(BEAR_V2, &g(BEAR_V2));
        assert_eq!((c.kind, c.long, c.short, c.p2, c.f2), (BearKind::V2, false, true, 5, 2.0));
        let r = day_rows(&["2022-03-01", "2022-03-02"]);
        let x = Ctx::new(&r, &r, &[]);
        for id in BEAR_IDS {
            assert!(catalogue().iter().any(|d| d.id == id));
            assert!(run(id, &x, 0, 1, &Params::new()).unwrap().is_empty());
            let def = catalogue().into_iter().find(|d| d.id == id).unwrap();
            assert!(crate::instrument_policy::intraday_rule(&def));
            assert_eq!(family_of(id).unwrap().2, "Supertrend (Intraday)");
        }
    }

    /// Opt-in smoke run on the local data: TEST_INTRADAY_DAILY (BANKNIFTY daily ieodfo CSV) and INTRADAY_DATA_DIR.
    #[test]
    fn local_bear_smoke_when_configured() {
        let Ok(csv) = std::env::var("TEST_INTRADAY_DAILY") else { return };
        if std::env::var_os("INTRADAY_DATA_DIR").is_none() {
            return;
        }
        let text = std::fs::read_to_string(csv).unwrap();
        let rows: Vec<Row> = text.lines().skip(1).map(|line| {
            let v: Vec<_> = line.split(',').collect();
            Row { ticker: "BANKNIFTY".into(), date: v[0].into(), open: v[1].parse().unwrap(), high: v[2].parse().unwrap(), low: v[3].parse().unwrap(), close: v[4].parse().unwrap(), volume: v[5].parse().ok() }
        }).collect();
        let from = rows.iter().position(|r| r.date.as_str() >= "2019-01-01").unwrap();
        let to = rows.iter().rposition(|r| r.date.as_str() <= "2019-12-31").unwrap();
        let x = Ctx::new(&rows, &rows, &[]);
        for id in BEAR_IDS {
            let t = run(id, &x, from, to, &Params::new()).unwrap();
            crate::trade_validation::check_structure(&t, from, to).unwrap();
            eprintln!("SMOKE {id} BANKNIFTY 2019: {} trades", t.len());
        }
    }

    #[test]
    fn defaults_and_registration() {
        let ps = params(FRESH).unwrap();
        let d = |k: &str| ps.iter().find(|p| p.key == k).unwrap().def;
        assert_eq!((d("tf_min"), d("st_period"), d("st_factor"), d("st_point"), d("sq_time")), (5.0, 5.0, 2.0, 1.0, 1520.0));
        assert!(params("ytst_st_all").is_none());
        assert_eq!(catalogue().len(), IDS.len());
        assert!(catalogue().iter().all(|d| d.book == BOOK && IDS.contains(&d.id)));
        // without intraday data for the ticker the run has no trades (readiness reports why before any run)
        let r = day_rows(&["2022-03-01", "2022-03-02"]);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run(FRESH, &x, 0, 1, &Params::new()).unwrap().is_empty());
        assert!(run("ytst_st_all", &x, 0, 1, &Params::new()).is_none());
    }
}
