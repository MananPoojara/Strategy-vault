//! YT Strategies · Supertrend (Stocks): the team's stock-universe AmiBroker AFLs in "Old Topics/Supertrend" (the
//! three "ST monthly single trade ..." AFLs, "ST trade below filter - Copy", "SMA BUY SELL WITH ALL TIMEFRAME" and
//! "Entry when Closing is above 5,1_Scale-in with 14 days ATR_Exit below 5,1" = "Rajiv Shukla Ver1", which despite its
//! folder name has no Supertrend; see `ytst.rs` for the index / single-series AFLs of that folder). Coded from the AFLs as written, with the AFLs'
//! own default Param values; there is no book PDF and no trade sheet was compared.
//!
//! What runs: each AFL's per-stock Buy / Sell (and, where the AFL has them, Short / Cover) rules on the stock's own
//! daily bars, one stock at a time. The team ran these as portfolio backtests over stock universes (NSE cash /
//! NSE 500 stocks, MAXIMUM POSITION 20, ranked by PositionScore when a RANKING MODE is chosen; default NONE). The
//! ranking, MaxOpenPositions, position sizing (SetPositionSize / AllowPositionShrinking), the 0.65 brokerage and
//! the R-multiple / SQN / risk-of-ruin custom-backtester metrics only choose between stocks or report statistics
//! and are not modelled. "Single trade" in the titles is the AFLs' per-stock latch (one entry per monthly ST buy
//! signal), which is coded; the AFLs have no universe-wide one-trade rule.
//!
//! AmiBroker behaviour followed (as in `ytst.rs` / `ytsw.rs`):
//! - the team's ST loop (`ytst::st_lines`): band ATR = mean of the last `period` ATR(1) values, bands ratchet and
//!   reset on a flip, trend 1 while the bands are Null; the AFLs' `ST()` function writes its up-trend line into
//!   BPD1 and its down-trend line into BFACTOR1 (VarSet);
//! - the market filter is the same stock's ST on calendar-month bars (filter timeframe 2160001), expanded to daily
//!   bars with expandLast (a month's value reaches the daily bars on the month's last bar); its TrendDown line is
//!   the Up band on down-trend months only and TrendUp the Dn band on up-trend months only (the AFL sets the bar
//!   before a flip to Null); the daily filter timeframe (86400) is offered; the weekly one is not (its `mcl` comes
//!   from TimeFrameGetPrice, whose partial-week value is not verified here);
//! - `mcl = ValueWhen(eom, C)` with the AFLs' month-end test on the next bar's day number (1 to 5), so the last bar
//!   of the data is never a month end;
//! - `Cross()` starts once both arrays have a value; after that a Null is -1e10, so a filter line turning Null is
//!   a cross; in the bar loops a Null element is -1e10 and Null x 2 stays Null;
//! - MA() is Null until `n` values exist; `HHV(C, BarCount)` is the highest close so far; upper circuit = Open,
//!   High and Close equal to 2 decimals (Prec), lower circuit = Open, Low and Close equal;
//! - Buy / Sell / Short / Cover are ExRem'd as the AFLs do; trades at the close; exit first, then entry on the same
//!   bar; with both sides on an opposite entry reverses; an ApplyStop loss % (ActivateStopsImmediately off) is
//!   checked from the bar after entry and fills at the stop or at the Open when the Open is already through it;
//!   bar loops run over all loaded bars, trades open only inside the test window;
//! - the template guards are the AFLs' defaults: dead-stock backtest NO (no entry on the first bar), penny price
//!   and turnover as each AFL sets them (turnover = Volume x Close above the AFL's yearly threshold; two of the
//!   AFLs apply it by default because their toggle tests the value 0).
//! - Rajiv Shukla Ver1 (its own bar loop): the 52-week high is HHV(High, 52) on Monday-Sunday weekly bars expanded
//!   with expandLast (a week's value reaches the daily bars on the week's last bar), HHV using the weeks available near
//!   the start; MA(C, 200) is Null (-1e10 in the loop) for the first 199 bars; the 50% scale-out (`Buy = sigScaleOut`,
//!   SetPositionSize(50, spsPercentOfPosition)) is one piece of the Trade's `exits`: AmiBroker fills a long scale-out
//!   at BuyPrice, which the AFL sets to the previous Close + 3% unless the Open is already above it (then it sets
//!   SellPrice, which the scale-out does not read, and the default BuyPrice, the Close, is used); whole-share rounding
//!   of the 50% is not modelled.
//!
//! Not coded: PositionScore ranking, MaxOpenPositions and sizing (portfolio level, see above); the NSE 500
//! `Foreign("NSE500", "C")` RSI of the two ATHC AFLs, which feeds only an unused COMBINERSI (no rule reads it); the
//! weekly filter timeframe; the SMA branch of "ST trade below filter" (its monthlytrend is computed on the filter
//! timeframe and not expanded) and the weekly / monthly SMA timeframes of "SMA BUY SELL" (its DIFF reads the
//! un-expanded MA), so both are offered on the ST filter / daily SMA only. Rajiv Shukla Ver1: its PositionScore =
//! RSI(14) ranking, MAXIMUM POSITION 20, sizing and 0.65 brokerage (portfolio level) and its dead-stock option (default
//! NO, which only stops entries on the first bar).
#![allow(dead_code)]
use crate::books::ytst::{calc_on, st_lines};
use crate::books::ytsw::{close_swing, compress, expand_last, exrem, trade, true_range};
use crate::books::ytwh::{bound, hhv, prec2};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_sts";
pub const SQN: &str = "ytsts_monthly_single_trade";
pub const ATHC: &str = "ytsts_monthly_single_athc";
pub const ATHC_DAILY: &str = "ytsts_monthly_single_athc_daily";
pub const BELOW: &str = "ytsts_trade_below_monthly_st";
pub const SMA: &str = "ytsts_sma_buy_sell";
pub const RAJIV: &str = "ytsts_rajiv_shukla_v1";
pub const IDS: [&str; 6] = [SQN, ATHC, ATHC_DAILY, BELOW, SMA, RAJIV];

const GROUP: &str = "YT Strategies · Supertrend (Stocks)";
const NULL: f64 = f64::NAN; // AFL Null in a loop: every comparison with it is false (SV-Audit)

fn def(id: &'static str, name: &'static str, dir: &'static str, needs_volume: bool, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir, status: Status::Live, needs_volume, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

macro_rules! universe {
    () => {
        " Runs on one actual stock at a time; the team ran it as a portfolio backtest over a stock universe: PositionScore ranking (RANKING MODE, default NONE), MAXIMUM POSITION (20), position sizing and brokerage are not modelled, nor the SQN / R-multiple report."
    };
}

const ST_F: &str = "F|ST = the team's Supertrend: bands = calculation point ± factor × mean of the last `period` ATR(1); the trend flips when the Close crosses the previous bar's band";
const M_F: &str = "F|Monthly filter: the stock's own ST on calendar-month bars (period 5, factor 1), expanded to daily bars on each month's last bar";
const MCL_F: &str = "F|Month close (mcl) = the Close of the last bar before a new month";
const CIRC_F: &str = "F|Upper circuit = Open, High and Close equal to 2 decimals; lower circuit = Open, Low and Close equal";
const ATH_F: &str = "F|ATHC filter: Close within 8% of the highest Close so far";
const TURN_F: &str = "F|Turnover: Volume × Close above the yearly threshold (30 thousand before 2000 up to 1.3 crore from 2020)";
const GUARD_F: &str = "F|No entry on the data's first bar; no entry on an upper-circuit bar";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(SQN, "ST Monthly Single Trade (Monthly ST Buy, Daily ST 5,4 Exit)", "Long", true, &[
            ST_F, M_F, MCL_F,
            "F|Exit ST: daily ST period 5, factor 4 on the mid point (option: 5% Close swing)",
            "F|Buy latch: set when mcl crosses above the previous bar's monthly ST down line while the exit ST is up; cleared when the exit ST or the monthly ST turns down",
            "E|Buy at the Close on the monthly cross or while the latch is set, with the exit ST up",
            "E|Also: Close > 50 (penny price) and MA(Volume, 10) > 1 lakh",
            GUARD_F,
            "X|Sell at the Close when the exit ST or the monthly ST is down, or the latch is cleared; not on a lower-circuit bar",
            "X|Hard stop: 5% below the entry, from the next bar",
            "F|Short side (option; the title says long only): mirrored on the monthly ST turning down",
            CIRC_F,
        ], concat!("ST monthly single trade long only WITH SQN.afl (folder of that name), coded as the AFL is written. The AFL also defines Short / Cover; the default here is long only as its title says (side option 3 trades both).", universe!())),
        def(ATHC, "ST Monthly Single Trade within ATHC − 8%, Exit with Daily STs", "Long", true, &[
            ST_F, M_F, MCL_F,
            "F|Entry ST: daily period 5, factor 2; exit ST: period 5, factor 4; calculation point default Close (monthly too)",
            ATH_F, TURN_F,
            "F|Buy latch: set on the monthly cross (mcl above the previous bar's monthly ST down line) with the turnover and ATHC filters; cleared when both daily STs or the monthly ST turn down",
            "F|Second latch: set on the monthly cross while the entry ST is down; cleared when the entry ST turns up, the exit ST or the monthly ST turns down",
            "E|Buy at the Close on the monthly cross, while the latch is set or the bar after the second latch, with either daily ST up, turnover and ATHC filters passing",
            GUARD_F,
            "X|Sell at the Close when both daily STs are down, the monthly ST is down or the latch is cleared; not on a lower-circuit bar",
            CIRC_F,
        ], concat!("ST monthly single trade long only  with 2 ST athc-30 pct WITH SQN n ranking.afl (folder 'ST monthly single trade within ATH-30%_Exit with ST daily'), coded as the AFL is written; its '% DIFF FROM ATHC' default is 8 (the folder name says 30). Its turnover toggle applies the filter at the default value. The Foreign NSE 500 RSI it computes is not used by any rule.", universe!())),
        def(ATHC_DAILY, "ST Monthly Single Trade, Daily Execution, ATHC − 8% (Exit ST 5,4 / Monthly ST 5,1)", "Long", false, &[
            ST_F, M_F,
            "F|Entry ST: daily period 5, factor 2; exit ST: period 5, factor 4; calculation point default mid point",
            ATH_F,
            "F|ATR %: MA(ATR(1), 14) / Close × 100, recorded on the entry-cross day",
            "F|Buy latch: set when the daily Close crosses above the previous bar's monthly ST down line with the entry ST up and the ATHC filter; cleared when the exit ST turns down or ATR % exceeds twice the entry-day value",
            "E|Buy at the Close on that cross or while the latch is set, with the entry ST up and the ATHC filter",
            GUARD_F,
            "X|Monthly ST in sell: Sell when both daily STs are down (or the latch is cleared and the entry ST is down); not on a lower-circuit bar",
            "X|Monthly ST in buy: Sell when the Close crosses below the previous bar's monthly ST up line (monthly ST 5,1)",
            "F|Turnover filter off by default",
            CIRC_F,
        ], concat!("ST monthly single trade with 2 ST  execution on daily WITH ATHC -30 PCTl.afl (folder 'ST monthly single trade within ATH-30%_If in MONTHLY SELL, exit with 54. If in MONTHLY buy, exit with 51'), coded as the AFL is written; '% DIFF FROM ATHC' default 8. Its second latch (BUYSIG1) needs the entry ST both up and down on the same bar, so it never sets (kept as written). The Foreign NSE 500 RSI it computes is not used by any rule.", universe!())),
        def(BELOW, "ST Trade Below Monthly ST Filter (Long Below, Short Above)", "Long/Short", true, &[
            ST_F, M_F,
            "F|Entry ST: daily period 5, factor 2; exit ST: period 5, factor 4; calculation point default mid point",
            TURN_F,
            "E|Buy at the Close when the monthly ST is in SELL and the entry ST is up (latched until both daily STs turn down or the monthly ST turns up)",
            "X|Sell when both daily STs are down, the monthly ST turns up or the latch is cleared; not on a lower-circuit bar",
            "E|Short at the Close when the monthly ST is in BUY and the entry ST is down (latched until the monthly ST turns down); not on a lower-circuit bar",
            "X|Cover when the entry ST turns up or the monthly ST turns down (not on a lower-circuit bar), or when the latch is cleared (not on an upper-circuit bar)",
            GUARD_F,
            "F|Side: long and short (default; an opposite entry reverses), long only or short only",
            CIRC_F,
        ], concat!("ST trade below filter - Copy.afl (folder 'ST trade below supertrend monthly filter'), coded as the AFL is written with its ST filter; its turnover toggle ('YES' at 0) applies the filter by default. Its short-latch reset reads `sellsig1=!1` (an assignment, false), so the short latch clears only on the monthly ST turning down (kept as written). The SMA filter branch is not offered (its monthly trend is not expanded to daily bars).", universe!())),
        def(SMA, "SMA Buy / Sell (Close vs 200-day SMA ± 1%)", "Long", false, &[
            "F|SMA = MA(Close, 200) on daily bars; compared with the previous bar's value",
            "F|Distance = |Close − SMA| as % of the Close (above) or of the SMA (below)",
            "E|Buy at the Close when the Close is above the SMA by more than 1% (buffer)",
            "X|Sell at the Close when the Close is below the SMA by more than 1%",
            "F|Short side (option; the AFL's header says long only): Short below, Cover above",
            GUARD_F,
            "F|Turnover filter off by default",
            CIRC_F,
        ], concat!("SMA BUY SELL WITH ALL TIMEFRAME.afl (folder of that name; its header: 'FOR NSE 500 ... 200 MA LONG ONLY WITH RANKING ON NSE 500'), coded as the AFL is written on the daily SMA; its weekly / monthly SMA choices are not offered (DIFF reads the un-expanded MA). The team ran it on NSE 500 stocks.", universe!())),
        def(RAJIV, "Rajiv Shukla Ver1 (52-Week High / 200-Day SMA Entry, 50% Scale-out)", "Long", false, &[
            "F|52-week high = HHV(High, 52) on weekly bars, reaching the daily bars on each week's last bar",
            "E|Buy at the Close when flat and the Close is within 25% below the 52-week high (above 75% of it and below it), above MA(Close, 200) and more than 2% above the previous Close",
            "E|Not on an upper-circuit bar; Close above the penny price (0); turnover filter off by default",
            "X|Scale out 50% (once per trade) when the Close is more than 3% above the previous Close, not on a lower-circuit bar: at the previous Close + 3%, or at the Close when the Open is already above that level",
            "X|Sell the rest at the Close when the Close is below the previous bar's Low, not on a lower-circuit bar",
            "F|Ranking: PositionScore = RSI(14) chooses between stocks in the team's portfolio run (not modelled here)",
            "F|Daily bars, long only, positional; no entry on the data's first bar",
            CIRC_F,
        ], concat!("Entry when Closing is above 5,1_Scale-in with 14 days ATR_Exit below 5,1.afl (folder 'Supertrend/Entry when Closing is above 5,1_Scale-in with 14 days ATR_Exit below 5,1'; its header: 'Rajiv Shukla Ver1', daily, long only, positional), coded as the AFL is written; despite the folder name it has no Supertrend or ATR scale-in. Its 50% scale-out is one piece of the trade (Buy = sigScaleOut, SetPositionSize 50% of position); AmiBroker fills it at BuyPrice, which the AFL leaves at the Close when the Open is already above the level (it sets SellPrice there, which the scale-out does not read). Whole-share rounding of the 50% is not modelled. Runs on one actual stock at a time; the team ran it as a portfolio backtest over a stock universe ranked by PositionScore = RSI(14): the ranking, MAXIMUM POSITION (20), position sizing and the 0.65 brokerage are not modelled.")),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let side = |d: f64| p("side", "Side (1 long only, 2 short only, 3 long and short)", d, 1.0, 3.0, 1.0);
    let point3 = || p("point", "ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0);
    let flt = || {
        vec![
            p("flt_tf", "Filter timeframe (1 monthly, 3 daily; weekly not offered)", 1.0, 1.0, 3.0, 2.0),
            p("flt_period", "Monthly ST period", 5.0, 1.0, 20.0, 1.0),
            p("flt_factor", "Monthly ST factor", 1.0, 1.0, 20.0, 1.0),
        ]
    };
    let two_st = || {
        vec![
            p("entry_period", "ST period for entry", 5.0, 1.0, 20.0, 1.0),
            p("entry_factor", "ST factor for entry", 2.0, 1.0, 20.0, 1.0),
            p("exit_period", "ST period for exit", 5.0, 1.0, 20.0, 1.0),
            p("exit_factor", "ST factor for exit", 4.0, 1.0, 20.0, 1.0),
        ]
    };
    let hard = || p("hard_stop", "Hard stop loss % (0 off)", 0.0, 0.0, 50.0, 1.0);
    let penny = || p("penny", "Penny stock price (Close must be above)", 0.0, 0.0, 100.0, 1.0);
    let turn = |d: f64| p("turnover", "Turnover filter (0 off, 1 on)", d, 0.0, 1.0, 1.0);
    let ath = || p("ath_pct", "% diff from ATHC (Close within)", 8.0, 1.0, 50.0, 1.0);
    let mut out = Vec::new();
    match id {
        SQN => {
            out.push(side(1.0));
            out.push(point3());
            out.extend(flt());
            out.extend([
                p("exit_mode", "Exit mode (0 ST, 1 % swing)", 0.0, 0.0, 1.0, 1.0),
                p("exit_period", "ST period for exit", 5.0, 1.0, 20.0, 1.0),
                p("exit_factor", "ST factor for exit", 4.0, 1.0, 6.0, 0.5),
                p("swing_pct", "Swing % for exit", 5.0, 3.0, 18.0, 1.0),
                p("hard_stop", "Hard stop loss %", 5.0, 1.0, 50.0, 1.0),
                p("penny", "Penny stock price (Close must be above)", 50.0, 1.0, 100.0, 1.0),
                p("vol_period", "Average volume period", 10.0, 1.0, 100.0, 1.0),
                p("vol_min", "Average volume above", 100_000.0, 100.0, 1_000_000_000.0, 1.0),
            ]);
        }
        ATHC => {
            out.push(p("point", "Daily ST calculation point (1 mid point, 2 close, 3 pivot, 4 (mid + pivot + close) / 3, 5 MA(C, 3), 6 MA(C, 5), 7 high; monthly: mid point, pivot, else close)", 2.0, 1.0, 7.0, 1.0));
            out.extend(flt());
            out.extend(two_st());
            out.extend([ath(), hard(), penny(), turn(1.0)]);
        }
        ATHC_DAILY => {
            out.push(point3());
            out.extend(flt());
            out.extend(two_st());
            out.extend([p("atr_period", "ATR period for exit", 14.0, 1.0, 50.0, 1.0), ath(), hard(), penny(), turn(0.0)]);
        }
        BELOW => {
            out.push(side(3.0));
            out.push(point3());
            out.extend(flt());
            out.extend(two_st());
            out.extend([hard(), penny(), turn(1.0)]);
        }
        SMA => {
            out.extend([
                side(1.0),
                p("sma_period", "SMA period (daily)", 200.0, 1.0, 1000.0, 1.0),
                p("buffer", "SMA buy buffer %", 1.0, 0.0, 10.0, 1.0),
                hard(),
                penny(),
                turn(0.0),
            ]);
        }
        RAJIV => {
            out.extend([
                p("hi_weeks", "52-week high: weeks", 52.0, 1.0, 104.0, 1.0),
                p("hi_pct", "Close within % below the 52-week high", 25.0, 1.0, 100.0, 1.0),
                p("sma_period", "SMA period (daily)", 200.0, 1.0, 1000.0, 1.0),
                p("buy_pct", "Buy: Close above the previous Close by %", 2.0, 0.0, 20.0, 0.5),
                p("scale_pct", "Scale out: Close above the previous Close by %", 3.0, 0.0, 20.0, 0.5),
                p("scale_frac", "Scale out: % of the position", 50.0, 1.0, 99.0, 1.0),
                penny(),
                turn(0.0),
            ]);
        }
        _ => return None,
    }
    Some(out)
}

// ---------------------------------------------------------------- helpers

fn nv(v: Option<f64>) -> f64 {
    v.unwrap_or(NULL)
}

fn some(v: &[f64]) -> Vec<Option<f64>> {
    v.iter().map(|&x| Some(x)).collect()
}

fn shift1(v: &[Option<f64>]) -> Vec<Option<f64>> {
    (0..v.len()).map(|i| if i == 0 { None } else { v[i - 1] }).collect()
}

/// AFL Cross(a, b): from the first bar where both have a value; after that a Null is -1e10.
fn cross(a: &[Option<f64>], b: &[Option<f64>]) -> Vec<bool> {
    let fa = a.iter().position(|v| v.is_some()).unwrap_or(a.len());
    let fb = b.iter().position(|v| v.is_some()).unwrap_or(b.len());
    let first = fa.max(fb);
    let v = |z: Option<f64>| z.unwrap_or(crate::books::ab::CROSS_NULL);
    (0..a.len()).map(|i| i > first && v(a[i]) > v(b[i]) && v(a[i - 1]) <= v(b[i - 1])).collect()
}

/// AFL MA(v, n): Null until `n` consecutive values exist.
fn ma(v: &[Option<f64>], n: usize) -> Vec<Option<f64>> {
    let n = n.max(1);
    (0..v.len())
        .map(|i| if i + 1 < n { None } else { v[i + 1 - n..=i].iter().try_fold(0.0, |s, x| x.map(|x| s + x)).map(|s| s / n as f64) })
        .collect()
}

/// Daily ST calculation point: 1 mid, 2 close, 3 pivot, 4 (mid + pivot + close) / 3, 5 MA(C, 3), 6 MA(C, 5), 7 high.
fn calc_d(x: &Ctx, kind: i64) -> Vec<f64> {
    if kind == 7 {
        x.h.clone()
    } else {
        calc_on(&x.h, &x.l, &x.c, kind, false)
    }
}

/// The monthly (or daily, tf 3) ST of the stock expanded to daily bars: trend, TrendDown (Up band on down-trend
/// periods) and TrendUp (Dn band on up-trend periods). Calculation point: 1 mid, 3 pivot, else close.
struct Flt {
    tr: Vec<i8>,
    td: Vec<Option<f64>>,
    tu: Vec<Option<f64>>,
}

fn filter(x: &Ctx, tf: i64, kind: i64, period: usize, f: f64) -> Flt {
    let b = compress(x, if tf == 3 { 'D' } else { 'M' });
    let m = b.c.len();
    let cp: Vec<f64> = (0..m)
        .map(|k| match kind {
            1 => (b.h[k] + b.l[k]) / 2.0,
            3 => (b.h[k] + b.l[k] + b.c[k]) / 3.0,
            _ => b.c[k],
        })
        .collect();
    let (tr, lu, ld) = st_lines(&cp, &b.h, &b.l, &b.c, period, f);
    let td: Vec<Option<f64>> = (0..m).map(|k| if tr[k] == -1 { ld[k] } else { None }).collect();
    let tu: Vec<Option<f64>> = (0..m).map(|k| if tr[k] == 1 { lu[k] } else { None }).collect();
    Flt { tr: expand_last(&tr, &b), td: expand_last(&td, &b), tu: expand_last(&tu, &b) }
}

fn day(x: &Ctx, i: usize) -> Option<u32> {
    x.r[i].date.get(8..10)?.parse().ok()
}

/// `mcl`: ValueWhen(eom, C) on the monthly filter, the Close itself on the daily one. eom = the next bar's day
/// number d1 is 1 to 5 and this bar's day number is not 1 to d1 - 1 (the AFL's five-way test).
fn mcl(x: &Ctx, tf: i64) -> Vec<Option<f64>> {
    if tf == 3 {
        return some(&x.c);
    }
    let n = x.c.len();
    let mut last = None;
    (0..n)
        .map(|i| {
            let eom = i + 1 < n
                && match (day(x, i + 1), day(x, i)) {
                    (Some(d1), Some(d0)) => (1..=5).contains(&d1) && !(1..d1).contains(&d0),
                    _ => false,
                };
            if eom {
                last = Some(x.c[i]);
            }
            last
        })
        .collect()
}

/// The AFLs' turnover thresholds by calendar year (Volume x Close must exceed them).
fn volsig(date: &str) -> f64 {
    match date.get(..4).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0) {
        ..=1999 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

fn turnover_ok(x: &Ctx, i: usize) -> bool {
    x.r[i].volume.map_or(false, |v| v * x.c[i] > volsig(&x.r[i].date))
}

fn up_circuit(x: &Ctx, i: usize) -> bool {
    prec2(x.o[i]) == prec2(x.c[i]) && prec2(x.o[i]) == prec2(x.h[i])
}
fn lo_circuit(x: &Ctx, i: usize) -> bool {
    prec2(x.o[i]) == prec2(x.c[i]) && prec2(x.o[i]) == prec2(x.l[i])
}

/// A bar-loop latch: set on `set`, cleared on `clear`, else held (0 on bar 0).
fn latch(n: usize, set: impl Fn(usize) -> bool, clear: impl Fn(usize, &[bool]) -> bool) -> Vec<bool> {
    let mut v = vec![false; n];
    for i in 1..n {
        v[i] = if set(i) {
            true
        } else if clear(i, &v) {
            false
        } else {
            v[i - 1]
        };
    }
    v
}

// ---------------------------------------------------------------- backtester

struct Sig {
    buy: Vec<bool>,
    sell: Vec<bool>,
    short: Vec<bool>,
    cover: Vec<bool>,
}

/// AmiBroker regular backtest at the close of ExRem'd signals (side 1 long only, 2 short only, 3 both, reversing),
/// with an optional ApplyStop loss % (0 off) checked from the bar after entry.
fn backtest(x: &Ctx, s: &Sig, side: i64, hard: f64, from: usize, to: usize) -> Vec<Trade> {
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
            if i > e && hard > 0.0 {
                let lv = epx * (1.0 - d * hard / 100.0);
                if d > 0.0 && x.l[i] <= lv {
                    ex = Some((x.o[i].min(lv), "Hard stop"));
                } else if d < 0.0 && x.h[i] >= lv {
                    ex = Some((x.o[i].max(lv), "Hard stop"));
                }
            }
            if ex.is_none() {
                let sig = if d > 0.0 { sell[i] || (shorts && short[i]) } else { cover[i] || (longs && buy[i]) };
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

// ---------------------------------------------------------------- Rajiv Shukla Ver1

/// "Rajiv Shukla Ver1": the AFL's own bar loop over all loaded bars (from bar 1) with its one 50% scale-out. Positions
/// entered in from..=to; the scale-out is a piece of the Trade's `exits`; one still open after `to` (or a scale-out
/// after it) is marked at the Close of `to`.
fn rajiv(x: &Ctx, g: &dyn Fn(&str) -> f64, from: usize, to: usize) -> Vec<Trade> {
    let (o, c, l) = (&x.o, &x.c, &x.l);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let w = compress(x, 'W');
    let hi52 = expand_last(&hhv(&w.h, gi("hi_weeks").max(1) as usize), &w);
    let sma = ma(&some(c), gi("sma_period").max(1) as usize);
    let (hi_pct, buy_pct, sc_pct, frac) = (g("hi_pct"), g("buy_pct"), g("scale_pct"), g("scale_frac") / 100.0);
    let penny = g("penny");
    let tov = |i: usize| gi("turnover") != 1 || turnover_ok(x, i);
    // (entry, scale-out (bar, price), exit (bar, price))
    let mut pos: Vec<(usize, Option<(usize, f64)>, Option<(usize, f64)>)> = Vec::new();
    let (mut open, mut scale_ok) = (false, false);
    for i in 1..n {
        if !open
            && c[i] > hi52[i] - hi52[i] * hi_pct / 100.0
            && c[i] < hi52[i]
            && c[i] > nv(sma[i])
            && c[i] > c[i - 1] + c[i - 1] * buy_pct / 100.0
            && !up_circuit(x, i)
            && c[i] > penny
            && tov(i)
        {
            pos.push((i, None, None));
            (open, scale_ok) = (true, true);
        } else if open {
            let Some(p) = pos.last_mut() else { break };
            let lvl = c[i - 1] + c[i - 1] * sc_pct / 100.0;
            if c[i] > lvl && scale_ok && !lo_circuit(x, i) {
                p.1 = Some((i, bound(x, i, if o[i] > lvl { c[i] } else { lvl })));
                scale_ok = false;
            }
            if c[i] < l[i - 1] && !lo_circuit(x, i) {
                p.2 = Some((i, c[i]));
                open = false;
            }
        }
    }
    pos.into_iter()
        .filter(|p| p.0 >= from && p.0 <= to)
        .map(|(e, so, ex)| {
            let (xi, xpx, open) = match ex {
                Some((i, px)) if i <= to => (i, px, false),
                _ => (to, c[to], true),
            };
            let so = so.filter(|s| s.0 <= xi);
            let (exit_px, exits, reason) = match so {
                Some((si, spx)) => (
                    frac * spx + (1.0 - frac) * xpx,
                    vec![(si, spx, frac), (xi, xpx, 1.0 - frac)],
                    format!("{:.0}% scaled out on {} at {spx:.2}; rest: {}", frac * 100.0, x.r[si].date, if open { "open" } else { "Sell: Close below the previous Low" }),
                ),
                None => (xpx, Vec::new(), if open { "open".to_string() } else { "Sell: Close below the previous Low".to_string() }),
            };
            Trade { dir: 1.0, legs: vec![Leg { i: e, px: c[e], w: 1.0 }], exit_i: xi, exit_px, reason, entry_kind: "close", signal_i: xi, open, exits }
        })
        .collect()
}

// ---------------------------------------------------------------- the AFLs

fn signals(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64) -> Sig {
    let n = x.c.len();
    let c = &x.c;
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let mut s = Sig { buy: vec![false; n], sell: vec![false; n], short: vec![false; n], cover: vec![false; n] };
    let penny = g("penny");
    let tf = if gi("flt_tf") == 3 { 3 } else { 1 };
    let point = gi("point");
    let tov = |i: usize| gi("turnover") != 1 || turnover_ok(x, i);
    let hcl: Vec<f64> = c.iter().scan(f64::MIN, |m, &v| { *m = m.max(v); Some(*m) }).collect();
    let ath = |i: usize| (hcl[i] - c[i]) / hcl[i] * 100.0 <= g("ath_pct");
    let st = |k: &str, f: &str| st_lines(&calc_d(x, point), &x.h, &x.l, &x.c, gu(k), g(f));
    match id {
        SQN => {
            let ex: Vec<i8> = if gi("exit_mode") == 1 { close_swing(c, g("swing_pct")) } else { st("exit_period", "exit_factor").0 };
            let f = filter(x, tf, point, gu("flt_period"), g("flt_factor"));
            let mt = &f.tr;
            let m = mcl(x, tf);
            let up_x = cross(&m, &shift1(&f.td));
            let dn_x = cross(&f.td, &m);
            let vol: Vec<Option<f64>> = x.r.iter().map(|r| Some(r.volume.unwrap_or(0.0))).collect();
            let avgv = ma(&vol, gu("vol_period"));
            let liq = |i: usize| avgv[i].map_or(false, |a| a > g("vol_min"));
            let bs = latch(n, |i| up_x[i] && ex[i] == 1, |i, _| ex[i] == -1 || mt[i] == -1);
            let ss = latch(n, |i| dn_x[i] && ex[i] == -1, |i, _| ex[i] == 1 || mt[i] == 1);
            for i in 0..n {
                let (upc, loc) = (up_circuit(x, i), lo_circuit(x, i));
                s.buy[i] = (up_x[i] || bs[i]) && i != 0 && !upc && ex[i] == 1 && c[i] > penny && liq(i);
                s.sell[i] = ((ex[i] == -1 || mt[i] == -1) && !loc) || (!bs[i] && !loc);
                s.short[i] = (dn_x[i] || ss[i]) && i != 0 && !loc && ex[i] == -1 && c[i] > penny && liq(i);
                s.cover[i] = ((ex[i] == 1 || mt[i] == 1) && !loc) || (!ss[i] && !upc);
            }
        }
        ATHC => {
            let en = st("entry_period", "entry_factor").0;
            let ex = st("exit_period", "exit_factor").0;
            let f = filter(x, tf, point, gu("flt_period"), g("flt_factor"));
            let mt = &f.tr;
            let up_x = cross(&mcl(x, tf), &shift1(&f.td));
            let dummy = |i: usize| up_x[i] && c[i] >= penny && tov(i) && ath(i);
            let bs1 = latch(n, |i| dummy(i) && en[i] == -1, |i, _| en[i] == 1 || mt[i] == -1 || ex[i] == -1);
            let bs = latch(n, dummy, |i, _| (ex[i] == -1 && en[i] == -1) || mt[i] == -1);
            for i in 0..n {
                let (upc, loc) = (up_circuit(x, i), lo_circuit(x, i));
                let prev1 = i > 0 && bs1[i - 1];
                s.buy[i] = (up_x[i] || bs[i] || prev1) && i != 0 && !upc && (en[i] == 1 || ex[i] == 1) && c[i] > penny && tov(i) && ath(i);
                s.sell[i] = (((ex[i] == -1 && en[i] == -1) || mt[i] == -1) && !loc) || (!bs[i] && !loc);
            }
        }
        ATHC_DAILY => {
            let (en, _, bfactor1) = st("entry_period", "entry_factor");
            let ex = st("exit_period", "exit_factor").0;
            let f = filter(x, tf, point, gu("flt_period"), g("flt_factor"));
            let mt = &f.tr;
            let cc = some(c);
            let up_x = cross(&cc, &shift1(&f.td));
            let dn_x = cross(&shift1(&f.tu), &cc);
            let dummy = |i: usize| up_x[i] && en[i] == 1 && c[i] >= penny && tov(i) && ath(i);
            let bs1 = latch(n, |i| dummy(i) && en[i] == -1, |i, _| en[i] == 1 || mt[i] == -1);
            let atr = ma(&true_range(&x.h, &x.l, c), gu("atr_period"));
            let atrpct: Vec<Option<f64>> = (0..n).map(|i| atr[i].map(|a| a / c[i] * 100.0)).collect();
            let bfx = cross(&cc, &shift1(&bfactor1));
            let mut buydayatr: Vec<Option<f64>> = vec![None; n];
            for i in 0..n {
                let cond = (up_x[i] && en[i] == 1) || (i > 0 && bs1[i - 1] && bfx[i]);
                buydayatr[i] = if cond { atrpct[i] } else if i > 0 { buydayatr[i - 1] } else { None };
            }
            let bs = latch(n, dummy, |i, _| ex[i] == -1 || nv(atrpct[i]) > buydayatr[i].map_or(NULL, |v| 2.0 * v));
            for i in 0..n {
                let (upc, loc) = (up_circuit(x, i), lo_circuit(x, i));
                let prev1 = i > 0 && bs1[i - 1];
                s.buy[i] = (up_x[i] || bs[i] || prev1) && i != 0 && !upc && en[i] == 1 && c[i] > penny && tov(i) && ath(i);
                s.sell[i] = (ex[i] == -1 && en[i] == -1 && !loc && mt[i] == -1) || (!bs[i] && !loc && mt[i] == -1 && en[i] == -1) || dn_x[i];
            }
        }
        BELOW => {
            let en = st("entry_period", "entry_factor").0;
            let ex = st("exit_period", "exit_factor").0;
            let mt = filter(x, tf, point, gu("flt_period"), g("flt_factor")).tr;
            let dummyb = |i: usize| mt[i] == -1 && en[i] == 1 && c[i] >= penny && tov(i);
            let dummys = |i: usize| mt[i] == 1 && c[i] >= penny && tov(i);
            // BUYSIG1 needs the entry ST up and down on the same bar: never set. SELLSIG1 is overwritten with 0 by
            // the `sellsig1=!1` in the short-latch reset, so Ref(SELLSIG1, -1) == 1 is never true either.
            let bs1 = latch(n, |i| dummyb(i) && en[i] == -1, |i, _| en[i] == 1 || mt[i] == -1);
            let bs = latch(n, dummyb, |i, _| (ex[i] == -1 && en[i] == -1) || mt[i] == 1);
            let ss = latch(n, dummys, |i, _| mt[i] == -1);
            for i in 0..n {
                let (upc, loc) = (up_circuit(x, i), lo_circuit(x, i));
                let prev1 = i > 0 && bs1[i - 1];
                s.buy[i] = mt[i] == -1 && (bs[i] || prev1) && i != 0 && !upc && en[i] == 1 && c[i] > penny && tov(i);
                s.sell[i] = (((ex[i] == -1 && en[i] == -1) || mt[i] == 1) && !loc) || (!bs[i] && !loc);
                s.short[i] = mt[i] == 1 && ss[i] && i != 0 && !loc && en[i] == -1 && c[i] > penny && tov(i);
                s.cover[i] = ((en[i] == 1 || mt[i] == -1) && !loc) || (!ss[i] && !upc);
            }
        }
        SMA => {
            let len = gu("sma_period");
            let mp = shift1(&ma(&some(c), len));
            let buff = g("buffer");
            for i in 0..n {
                let Some(m) = mp[i] else { continue };
                let diff = if c[i] > m { (c[i] - m) / c[i] * 100.0 } else { (m - c[i]) / m * 100.0 };
                let (fb, fs) = (c[i] > m && diff > buff, c[i] < m && diff > buff);
                let upc = up_circuit(x, i);
                s.buy[i] = fb && i != 0 && !upc && c[i] > penny && tov(i) && n > len;
                s.sell[i] = fs;
                s.short[i] = fs && i != 0 && !upc && c[i] > penny && tov(i);
                s.cover[i] = fb;
            }
        }
        _ => {}
    }
    s
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    if id == RAJIV {
        return Some(rajiv(x, &g, from, to));
    }
    let s = signals(id, x, &g);
    let side = if schema.iter().any(|d| d.key == "side") { g("side").round() as i64 } else { 1 };
    Some(backtest(x, &s, side, g("hard_stop"), from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Supertrend (Stocks)"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;
    use crate::Row;

    /// Bars with the open away from the close (no circuit bar) and a range of 2.
    fn bar(c: f64) -> (f64, f64, f64, f64) {
        (c + 0.3, c + 1.0, c - 1.0, c)
    }
    fn with_volume(mut r: Vec<Row>) -> Vec<Row> {
        for b in &mut r {
            b.volume = Some(1_000_000.0);
        }
        r
    }
    /// About six months up, six months down, six months up, then a sharp daily fall.
    fn cycle() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..130).map(|k| bar(100.0 + 0.5 * k as f64)).collect();
        v.extend((0..130).map(|k| bar(165.0 - 0.5 * k as f64)));
        v.extend((0..150).map(|k| bar(100.0 + 0.5 * k as f64)));
        v.extend((0..15).map(|k| bar(172.0 - 4.0 * k as f64)));
        v
    }
    /// First bar where the expanded monthly ST turns from sell to buy.
    fn monthly_flip_up(x: &Ctx) -> usize {
        let f = filter(x, 1, 1, 5, 1.0);
        (1..x.c.len()).find(|&i| f.tr[i] == 1 && f.tr[i - 1] == -1).expect("monthly ST turns up")
    }

    #[test]
    fn mcl_month_end_and_cross_with_null() {
        let r = bars(&(0..30).map(|k| bar(100.0 + k as f64)).collect::<Vec<_>>());
        let x = Ctx::new(&r, &r, &[]);
        let m = mcl(&x, 1);
        // 2024-01-31 (bar 22) is the last January weekday; the next bar is 1 February
        assert_eq!(&x.r[22].date[..], "2024-01-31");
        assert_eq!(m[21], None);
        assert_eq!(m[22], Some(122.0));
        assert_eq!(m[29], Some(122.0));
        // a line turning Null after it started is a cross (Null = -1e10)
        assert_eq!(cross(&[Some(5.0), Some(5.0), Some(5.0)], &[Some(6.0), Some(6.0), None]), vec![false, false, true]);
        assert_eq!(ma(&[None, Some(2.0), Some(4.0)], 2), vec![None, None, Some(3.0)]);
    }

    #[test]
    fn monthly_single_trade_buys_on_the_month_end_flip() {
        let r = with_volume(bars(&cycle()));
        let x = Ctx::new(&r, &r, &[]);
        let flip = monthly_flip_up(&x);
        let t = go(run, SQN, &r, &[]);
        let after: Vec<_> = t.iter().filter(|t| t.0 >= 260).collect();
        assert_eq!(after.len(), 1, "{t:?}");
        // bought at the month-end close on which the monthly ST turns up; sold in the final fall
        assert_eq!(after[0].0, flip);
        assert_ne!(&x.r[flip].date[5..7], &x.r[flip + 1].date[5..7]);
        assert!(after[0].2 > 410 && !after[0].4);
        // no volume: the MA(Volume) filter blocks every entry
        assert!(go(run, SQN, &bars(&cycle()), &[]).is_empty());
    }

    #[test]
    fn athc_filter_needs_close_near_the_high() {
        let r = with_volume(bars(&cycle()));
        let x = Ctx::new(&r, &r, &[]);
        let flip = monthly_flip_up(&x);
        // at the flip the Close is far below the earlier 165 high: no trade within 8%
        assert!((165.0 - x.c[flip]) / 165.0 * 100.0 > 8.0);
        assert!(go(run, ATHC, &r, &[]).iter().all(|t| t.0 < 260));
        let t = go(run, ATHC, &r, &[("ath_pct", 50.0)]);
        let after: Vec<_> = t.iter().filter(|t| t.0 >= 260).collect();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].0, flip);
        assert!(after[0].2 > 410);
    }

    #[test]
    fn daily_execution_buys_on_the_daily_cross_of_last_months_line() {
        let r = bars(&cycle());
        let x = Ctx::new(&r, &r, &[]);
        let f = filter(&x, 1, 1, 5, 1.0);
        let prev = shift1(&f.td);
        let t = go(run, ATHC_DAILY, &r, &[("ath_pct", 50.0)]);
        let e = t.iter().find(|t| t.0 >= 260).expect("a trade in the second rise").0;
        // the entry bar's Close crosses above the previous bar's expanded monthly down line, inside the month
        assert!(x.c[e] > nv(prev[e]) && x.c[e - 1] <= nv(prev[e - 1]));
        assert!(prev[e].is_some());
        assert!(e < monthly_flip_up(&x), "daily execution enters before the month-end flip");
    }

    #[test]
    fn below_filter_buys_while_monthly_st_is_in_sell() {
        let r = with_volume(bars(&cycle()));
        let x = Ctx::new(&r, &r, &[]);
        let f = filter(&x, 1, 1, 5, 1.0);
        let flip = monthly_flip_up(&x);
        let t = go(run, BELOW, &r, &[("side", 1.0)]);
        // a long opened in the second rise while the monthly ST is still down, sold when it turns up
        let l = t.iter().find(|t| t.0 >= 260).expect("long below the filter");
        assert_eq!(f.tr[l.0], -1);
        assert_eq!(l.2, flip);
        for t in &t {
            assert_eq!(f.tr[t.0], -1);
        }
        // short side: only while the monthly ST is up
        for t in go(run, BELOW, &r, &[("side", 2.0)]) {
            assert_eq!(f.tr[t.0], 1);
        }
    }

    #[test]
    fn sma_buy_and_sell_around_the_200_day_average() {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..210).map(|_| bar(100.0)).collect();
        v.extend([bar(102.0), bar(98.0), bar(98.0)]);
        let r = bars(&v);
        // Ref(MA, -1) = 100 at bar 210: 102 is 1.96% above -> buy; bar 211: 98 is 2.0% below 100.01 -> sell
        let t = go(run, SMA, &r, &[]);
        assert_eq!(t, vec![(210, 102.0, 211, 98.0, false)]);
        // within the 1% buffer: no trade
        let mut w: Vec<(f64, f64, f64, f64)> = (0..210).map(|_| bar(100.0)).collect();
        w.extend([bar(100.5), bar(99.5)]);
        assert!(go(run, SMA, &bars(&w), &[]).is_empty());
        // short side as an option
        let ts = go(run, SMA, &r, &[("side", 2.0)]);
        assert_eq!((ts[0].0, ts[0].1), (211, 98.0));
    }

    #[test]
    fn hard_stop_fills_at_the_level_or_the_open() {
        let mut s = Sig { buy: vec![false; 4], sell: vec![false; 4], short: vec![false; 4], cover: vec![false; 4] };
        s.buy[1] = true;
        let r = bars(&[bar(100.0), bar(100.0), (99.0, 99.5, 94.0, 96.0), bar(96.0)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = backtest(&x, &s, 1, 5.0, 0, 3);
        assert_eq!((t[0].exit_i, t[0].exit_px), (2, 95.0));
        let r2 = bars(&[bar(100.0), bar(100.0), (93.0, 94.0, 92.0, 93.5), bar(96.0)]);
        let x2 = Ctx::new(&r2, &r2, &[]);
        assert_eq!(backtest(&x2, &s, 1, 5.0, 0, 3)[0].exit_px, 93.0);
    }

    /// A rise to 131.5 by bar 210, a fall to 119.5 by bar 225, then a 2.5% up day (bar 226, the entry).
    fn rajiv_bars() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..=210).map(|k| bar(100.0 + 0.15 * k as f64)).collect();
        v.extend((0..15).map(|k| bar(131.5 - 0.8 * (k + 1) as f64)));
        v.push(bar(119.5 * 1.025));
        v
    }

    #[test]
    fn rajiv_buys_near_the_52_week_high_and_scales_out_half() {
        let mut v = rajiv_bars();
        let e = v.len() - 1;
        let epx = v[e].3;
        v.push(bar(epx * 1.035)); // more than 3% up: scale out 50% (the Open is above the level -> at the Close)
        v.push(bar(epx * 1.035 + 0.2));
        v.push(bar(epx * 1.035 - 1.5)); // below the previous Low: sell the rest
        v.extend((0..5).map(|_| bar(epx * 1.035 - 1.5)));
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(RAJIV, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1, "{t:?}");
        let t = &t[0];
        assert_eq!((t.legs[0].i, t.legs[0].px), (e, x.c[e]));
        assert_eq!(t.exit_i, e + 3);
        assert!(t.exits.len() == 2 && t.exits.iter().zip([e + 1, e + 3]).all(|(p, k)| p.0 == k && p.2 == 0.5 && crate::books::ab::near_ab(p.1, x.c[k])));
        assert!(crate::books::ab::near_ab(t.exit_px, (x.c[e + 1] + x.c[e + 3]) / 2.0));
        assert!(!t.open);
        // the Open below the level: the scale-out fills at the previous Close + 3%
        let mut v2 = v.clone();
        v2[e + 1] = (epx, epx * 1.035 + 1.0, epx - 0.2, epx * 1.035);
        let r2 = bars(&v2);
        let x2 = Ctx::new(&r2, &r2, &[]);
        let t2 = &run(RAJIV, &x2, 0, r2.len() - 1, &Params::new()).unwrap()[0];
        assert!(crate::books::ab::near_ab(t2.exits[0].1, epx * 1.03));
        // no 3% day: one exit at the Close, no pieces
        let mut v3 = v.clone();
        v3[e + 1] = bar(epx + 1.0);
        v3[e + 2] = bar(epx + 1.2);
        for b in &mut v3[e + 3..] {
            *b = bar(epx);
        }
        let r3 = bars(&v3);
        let x3 = Ctx::new(&r3, &r3, &[]);
        let t3 = &run(RAJIV, &x3, 0, r3.len() - 1, &Params::new()).unwrap()[0];
        assert!(t3.exits.is_empty());
        assert_eq!((t3.exit_i, t3.exit_px), (e + 3, x3.c[e + 3]));
        // the entry needs the Close within 25% of the 52-week high: within 5% the 2.5% day (122.5 vs 132.5) is too far
        // below it; the next day (126.8, up 3.5%) enters instead
        let t5 = run(RAJIV, &x, 0, r.len() - 1, &ps(&[("hi_pct", 5.0)])).unwrap();
        assert_eq!(t5.iter().map(|t| t.legs[0].i).collect::<Vec<_>>(), vec![e + 1]);
    }

    #[test]
    fn rajiv_open_trade_keeps_its_scale_out() {
        let mut v = rajiv_bars();
        let e = v.len() - 1;
        let epx = v[e].3;
        v.push(bar(epx * 1.035));
        v.push(bar(epx * 1.035 + 0.5));
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = &run(RAJIV, &x, 0, r.len() - 1, &Params::new()).unwrap()[0];
        assert!(t.open);
        assert!(t.exits.len() == 2 && t.exits.iter().zip([e + 1, e + 2]).all(|(p, k)| p.0 == k && p.2 == 0.5 && crate::books::ab::near_ab(p.1, x.c[k])));
        // a window ending on the entry bar: open, no pieces
        let t = &run(RAJIV, &x, 0, e, &Params::new()).unwrap()[0];
        assert!(t.open && t.exits.is_empty() && t.exit_i == e);
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        let r = with_volume(wave(400));
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            assert!(family_of(id).is_some(), "{id}");
            assert!(catalogue().iter().any(|d| d.id == id));
            assert!(run(id, &x, 0, r.len() - 1, &Params::new()).is_some());
        }
    }
}
