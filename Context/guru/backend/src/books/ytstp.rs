//! YT Strategies · Supertrend: the remaining "Old Topics/Supertrend/Supertrend Long_Based on Probabilty" AFLs (the
//! V3-V5 virtual-short AFLs are in `ytst.rs`). Long only, one daily series (the team ran them on Nifty futures /
//! the index; here the daily spot index, as `ytst.rs`). Coded from the AFLs as written, with their own default Param
//! values; there is no book PDF and no trade sheet was compared.
//!
//! What is coded:
//! - "Long if x% Gain in Short": Futures_Long_X%GainInShort_V1 (no filter), its "With Filter_V1" twin (a different
//!   loop, so its own id), and Futures_Long_X%inShort_V2 (long at the n-th ST buy after a winning virtual short);
//! - "Long if x% Loss in Short": Version 1 "SuperTrend Virtual Short" (ST cross virtual short, monthly ST filter),
//!   VirtualShortInLossandLong_ST_V2 / Futures_Long_X%LossinShort_with_STFilter_V2 (n losing virtual shorts; one id,
//!   filter off = the no-filter AFL) and _V3 / with_STFilter_V3 (long at the n-th ST buy after a losing virtual short);
//! - "Long If X% Loss In Long": Post_STBasedLossesInLongTrades_NLosses_V1 and _Drawdown_V2 (virtual longs until n
//!   losses / an x% drawdown, then n real trades or until a profit target), and the three ST-LongTrade_NLoss AFLs.
//!
//! AmiBroker behaviour followed (as in `ytst.rs`'s GAIN3 / GAIN4 / LOSS4 / LOSS5):
//! - the AFLs' own ST is `ytsn::sniper_st` (band ATR = mean of the last `period` ATR(1); loop from bar `period`; no
//!   band reset on a flip); STBuy = Close > ST, STSell = Close < ST; on a weekly / monthly timeframe the AFLs'
//!   `Ref(-1)`, expandFirst, `Ref(+1)` path: a day sees the previous completed period, the period's last day its own,
//!   the data's last bar Null;
//! - the signals are limited to the test range as the AFLs' dateCheck (`DateNum() < rangetoDate`: the range's last
//!   day excluded) or Status("barinrange") (the whole range) or not at all, as each AFL is written;
//! - trades at the close; HSL / PB / TSL % (0 = off; 'live' fills at the level, or at Open − buffer on a gap); a
//!   comparison with a Null level is never true; the TSL keeps its last value between trades, as the AFLs' arrays do;
//! - the loop's last bar (`deadstocksellsig`) sells, and no buy is taken on it (`deadstockbuysig`), where the AFL
//!   says so; a Buy and a Sell on one bar make a same-bar trade (AmiBroker's AllowSameBarExit default);
//! - a comparison with Null is false (so NOT of it is true) where the AFL compares after the timeframe shift; a Null
//!   STBuy / STSell / IsBull (shifted after the comparison) passes neither side;
//! - the deadstock (default NO), penny (0) and turnover (off) settings are the AFLs' defaults and are not offered.
//!
//! Not coded: jitinV1.afl and jitinV2.afl (the NLosses_V1 / Drawdown_V2 loops without the ST filter, with Optimize()
//! ranges and the same defaults: set the filter off); the intraday ST timeframe options of the ParamLists (the main
//! ST runs on the chart's own daily bars, as in `ytst.rs`, except the ST-LongTrade AFLs' Daily / Weekly / Monthly
//! list); the custom-backtester metrics and _TRACE output (reporting only).
#![allow(dead_code)]
use crate::books::ytsn::sniper_st;
use crate::books::ytsw::{compress, cross_down, cross_up};
use crate::books::ytwh::{bound, collect, Ev};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_st";
pub const GAIN1: &str = "ytstp_gain_short_v1";
pub const GAIN1F: &str = "ytstp_gain_short_v1_filter";
pub const GAIN2: &str = "ytstp_gain_short_v2";
pub const VS1: &str = "ytstp_virtual_short_v1";
pub const LOSS2: &str = "ytstp_loss_short_v2";
pub const LOSS3: &str = "ytstp_loss_short_v3";
pub const POST_N: &str = "ytstp_long_loss_nlosses";
pub const POST_DD: &str = "ytstp_long_loss_drawdown";
pub const NLOSS: &str = "ytstp_st_long_nloss";
pub const SLIDE: &str = "ytstp_st_long_nloss_sliding";
pub const SLIDE_ALT: &str = "ytstp_st_long_nloss_sliding_alt";
pub const IDS: [&str; 11] = [GAIN1, GAIN1F, GAIN2, VS1, LOSS2, LOSS3, POST_N, POST_DD, NLOSS, SLIDE, SLIDE_ALT];

const GROUP: &str = "YT Strategies · Supertrend";
const NULL: f64 = -1e10;

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const PROB_ST: &str = "F|ST1 / ST2: the AFL's own Supertrend (period 5, factors 1 and 1, on (Open+Close)/2; loop from bar `period`, no band reset on a flip); STBuy = Close > ST, STSell = Close < ST";
const PROB_SIG: &str = "F|Long entry signal: STBuy1 inside the test range; long exit signal: STSell1 and STSell2 inside the test range (the range's last day excluded, as the AFL's DateNum() < rangetoDate)";
const PROB_X: &str = "X|Sell at the Close on the exit signal, on the optional HSL / PB / TSL % (default 0 = off; 'live' fills at the level or Open − buffer) or on the data's last bar";
const PROB_XF: &str = "X|Sell at the Close on the exit signal, on the optional HSL / PB / TSL % (default 0 = off; 'live' fills at the level or Open − buffer), on the filter turning against the trade or on the data's last bar";
const FLT_W: &str = "F|Optional filter (default off): the same series' ST on weekly (default) / monthly bars, as the AFL sees it (previous completed period, the period's last day its own); long when bullish (default) or bearish";
const FLT_M: &str = "F|Optional filter (default off): the same series' ST on daily / weekly / monthly (default) bars, as the AFL sees it; long when bullish (default) or bearish";
const VS_OPEN: &str = "F|Virtual short: opened at the Close on a long exit signal; closed at the Close of the next long entry signal";
const NL_ST: &str = "F|ST1 / ST2: the AFL's own Supertrend (period 5, factors 1 and 1, on (High+Low)/2) on daily bars (option weekly / monthly, as the AFL sees them); bull phase = Close > ST";
const NL_SIG: &str = "F|Entry signal: ST1 bull phase; exit signal: neither ST in its bull phase; both only inside the test range where the AFL checks Status(\"barinrange\")";
const NL_LOOP: &str = "F|Each entry signal while flat opens a long, virtual or real; it ends on the next exit signal at the Close (a loss when the Close is below the entry Close)";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(GAIN1, "Long after an x% Gain in a Virtual ST Short (V1)", &[
            PROB_ST, PROB_SIG,
            "F|Virtual short: opened at the Close on a long exit signal while flat (real and virtual); closed on the next long entry signal; its gain % is kept (Null when it did not gain)",
            "E|Buy at the Close on a long entry signal while flat when the last virtual short gained at least x% (default 5); not on the data's last bar",
            PROB_X,
        ], "Futures_Long_X%GainInShort_V1.afl (Supertrend Long_Based on Probabilty / Long if x% Gain in Short), coded as the AFL is written: the last virtual short's gain stays in force, so every entry signal buys until the next virtual short closes."),
        def(GAIN1F, "Long after an x% Gain in a Virtual ST Short (V1 with Filter)", &[
            PROB_ST, PROB_SIG, FLT_W,
            "F|Virtual short: opened at the Close on a long exit signal while no virtual short is open and no qualifying gain is waiting; closed on the next long entry signal",
            "E|Buy at the Close on a long entry signal while flat when the virtual short gained at least x% (default 5) and the filter allows; one long per qualifying virtual short; not on the data's last bar",
            PROB_XF,
        ], "Futures_Long_X%GainInShort_With Filter_V1.afl (Long if x% Gain in Short), coded as the AFL is written. Its loop differs from the no-filter V1 (one long per qualifying virtual short, virtual shorts also while long), so it is its own id. The exit test also runs on the buy bar (an 'if', not 'else if'): a live HSL / PB or the TSL left from the previous trade can sell on the entry bar."),
        def(GAIN2, "Long at the n-th ST Buy after an x% Gain in a Virtual ST Short (V2)", &[
            PROB_ST, PROB_SIG,
            "F|Virtual short: opened at the Close on a long exit signal while no virtual short is open and no setup is waiting; closed on the next long entry signal; a setup waits when it gained at least x% (default 5)",
            "E|While a setup waits: the n-th long entry signal (default 1st) buys at the Close; earlier ones are counted, each ending on the next long exit signal; not on the data's last bar",
            PROB_X,
            "X|Each sale ends the setup",
        ], "Futures_Long_X%inShort_V2.afl (Long if x% Gain in Short), coded as the AFL is written. The virtual short's change is not required to be positive here (x = 0 accepts a flat short), unlike V1 / V3."),
        def(VS1, "Long after a Virtual ST Short Loses x% (Version 1, ST Cross)", &[
            "F|D1 / D2: the AFL's own Supertrend (period 5, factors 1 and 2, on (High+Low)/2) on daily bars",
            "F|Virtual short: opened at the Close when the Close crosses below D1 and no virtual short is open, or when a long is sold",
            "E|Buy at the Close when the Close crosses above D1 while not long with a virtual short open (and the filter allows), if the Close is more than x% (default 3) above the virtual short's entry; the virtual short closes there either way",
            "F|Optional filter (default any): monthly ST (period 5, factor 1, (High+Low)/2) as the AFL sees it, bullish or bearish",
            "X|Once the Close has been above D2 in the trade: sell when the Close is below D2; before that: sell when the Close is below D1 and D2; and on the data's last bar",
        ], "SuperTrend Virtual Short.afl (Long if x% Loss in Short / Version 1), coded as the AFL is written. A virtual short opened while long (a cross below D1) also clears the 'above D2' flag of that trade, as the AFL's shared reset does. Its cumulative sum and ParamStr timeframe are not used by the signals."),
        def(LOSS2, "Long after n Virtual ST Shorts Lose x% (V2)", &[
            PROB_ST, PROB_SIG, FLT_M,
            "F|Virtual short: opened at the Close on a long exit signal while flat (real and virtual); closed on the next long entry signal",
            "F|A loss: the virtual short's change ≤ x% (default −5); n (default 2) losses, consecutive (default; a non-losing short resets the count) or not",
            "E|Buy at the Close on a long entry signal while flat once the count reaches n (and the filter allows; not on the data's last bar); the count restarts",
            PROB_XF,
        ], "Futures_Long_X%LossinShort_with_STFilter_V2.afl (Long if x% Loss in Short), coded as the AFL is written; with the filter off it is VirtualShortInLossandLong_ST_V2.afl (that AFL resets the count at the sale instead of the buy, which gives the same trades). Filter timeframe default monthly."),
        def(LOSS3, "Long at the n-th ST Buy after an x% Loss in a Virtual ST Short (V3)", &[
            PROB_ST, PROB_SIG, FLT_M,
            "F|Virtual short: opened at the Close on a long exit signal while no virtual short is open and no setup is waiting; closed on the next long entry signal; a setup waits when its change ≤ x% (default −5)",
            "E|While a setup waits: the n-th long entry signal (default 1st) that the filter allows buys at the Close; earlier ones are counted, each ending on the next exit signal or filter turn; not on the data's last bar",
            PROB_XF,
            "X|Each sale ends the setup",
        ], "Futures_Long_X%LossinShort_with_STFilter_V3.afl (Long if x% Loss in Short), coded as the AFL is written; with the filter off it is VirtualShortInLossandLong_ST_V3.afl. Filter timeframe default monthly."),
        def(POST_N, "Long after n Losing Virtual ST Longs, Then n Trades or a Profit Target", &[
            PROB_ST, PROB_SIG,
            "F|While the entry condition is off, each long entry signal opens a virtual long, closed on the next exit signal; a loss adds 1 to the count (consecutive mode: a non-losing one resets it)",
            "F|Entry condition: count ≥ n (default 0); re-evaluated while no real trade series runs (and, with a profit target, until it is reached)",
            "E|Buy at the Close on a long entry signal while flat when the condition holds, up to n long trades (default 0 = no limit); the count restarts; not on the data's last bar",
            PROB_X,
            "X|Optional profit target % (default 0 = off), individual or cumulative (default): reached, the entry condition switches off",
            "F|Optional ST filter (default off; daily ST 5, 1 on (High+Low)/2, option weekly / monthly): a buy it blocks still runs in the loop, untraded",
        ], "Post_STBasedLossesInLongTrades_NLosses_V1.afl (Long If X% Loss In Long), coded as the AFL is written: its filter only removes the Buy flag, so a blocked trade still runs in the loop (resets the count, counts toward n trades, books profit) without trading. jitinV1.afl is the same loop without the filter (Optimize() ranges, same defaults): set the filter off."),
        def(POST_DD, "Long after an x% Drawdown in Virtual ST Longs, Then n Trades or a Profit Target", &[
            PROB_ST, PROB_SIG,
            "F|While the entry condition is off, each long entry signal opens a virtual long, closed on the next exit signal; its % change is added to a running total, which restarts when it rises above 0",
            "F|Entry condition: total ≤ x% (default −5); re-evaluated while no real trade series runs (and, with a profit target, until it is reached)",
            "E|Buy at the Close on a long entry signal while flat when the condition holds and the optional ST filter allows, up to n long trades (default 0 = no limit); the total restarts; not on the data's last bar",
            PROB_X,
            "X|Optional profit target % (default 0 = off), individual or cumulative (default): reached, the entry condition switches off",
            "F|Optional ST filter (default off; daily ST 5, 1 on (High+Low)/2, option weekly / monthly); a blocked signal opens a virtual long",
        ], "Post_STBasedLossesInLongTrades_Drawdown_V2.afl (Long If X% Loss In Long), coded as the AFL is written. jitinV2.afl is the same loop without the filter (Optimize() ranges, same defaults): set the filter off."),
        def(NLOSS, "Long for n Trades after n Losing Virtual ST Longs (Range-Limited)", &[
            NL_ST, NL_SIG, NL_LOOP,
            "F|A losing trade adds 1 to the loss count; in consecutive mode a winning one resets it (default non-consecutive); a real sale resets it",
            "E|Buy at the Close on an entry signal when the count ≥ n (default 0) or right after a real trade, up to n long trades in a row (default 0, which takes no trade); then one virtual trade restarts the series",
            "X|Sell at the Close on the exit signal",
        ], "ST-LongTrade_NLoss.afl (Long If X% Loss In Long), coded as the AFL is written. Its default 'take n long trades' of 0 takes no trade (IIf(nLongTrades>0, .., 0)): set n_long_trades ≥ 1. Its profit % and type Params are not used by the loop."),
        def(SLIDE, "Long for n Trades after n Losing ST Longs (Sliding Window)", &[
            NL_ST,
            "F|Entry signal: ST1 bull phase; exit signal: neither ST in its bull phase (all bars, no range check)",
            NL_LOOP,
            "F|A losing trade adds 1 to the loss count; in consecutive mode a winning one resets it; non-consecutive mode (default) resets it at each real buy and sale",
            "F|Whenever a trade ends with the count ≥ n, the trade series restarts (sliding window)",
            "E|Buy at the Close on an entry signal when the count ≥ n (default 0) or right after a real trade, up to n long trades in a row (default 0, which takes no trade)",
            "X|Sell at the Close on the exit signal",
        ], "ST-LongTrade_NLoss_SlidingWindow.afl (Long If X% Loss In Long), coded as the AFL is written; signals on all bars (its barinrange check is commented out). Default 'take n long trades' 0 takes no trade: set n_long_trades ≥ 1."),
        def(SLIDE_ALT, "Long for n Trades after n Losing ST Longs (Sliding Window, Alternate)", &[
            NL_ST, NL_SIG, NL_LOOP,
            "F|A losing trade adds 1 to the loss count; in consecutive mode a winning one resets it; buys and sales do not reset it",
            "E|Buy at the Close on an entry signal when the count ≥ n (default 0) or right after a real trade, up to n long trades in a row (default 0, which takes no trade); then one virtual trade restarts the series",
            "X|Sell at the Close on the exit signal",
        ], "ST-LongTrade_NLoss_SlidingWindow_alternate.afl (Long If X% Loss In Long), coded as the AFL is written (range-limited, no count reset). Default 'take n long trades' 0 takes no trade: set n_long_trades ≥ 1."),
    ]
}

// ---------------------------------------------------------------- params

const AVG: &str = "ST average value (0 close, 1 open, 2 high, 3 low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)";

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let st = || {
        vec![
            p("st_period", "ST period", 5.0, 1.0, 100.0, 1.0),
            p("st_factor1", "ST factor 1", 1.0, 1.0, 10.0, 0.1),
            p("st_factor2", "ST factor 2", 1.0, 1.0, 10.0, 0.1),
            p("st_avg", AVG, 4.0, 0.0, 7.0, 1.0),
        ]
    };
    let exits = || {
        vec![
            p("exit_live", "Exit at (0 close, 1 live)", 0.0, 0.0, 1.0, 1.0),
            p("hsl_pct", "HSL % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("pb_pct", "PB % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("tsl_pct", "TSL % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("buffer", "Exit buffer %", 0.25, 0.01, 1.0, 0.01),
        ]
    };
    let flt = |tf: f64, label: &'static str, min: f64, avg: f64| {
        vec![
            p("flt", "ST filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("flt_tf", label, tf, min, 3.0, 1.0),
            p("flt_period", "ST filter period", 5.0, 1.0, 100.0, 1.0),
            p("flt_factor", "ST filter factor", 1.0, 1.0, 10.0, 0.1),
            p("flt_avg", AVG, avg, 0.0, 7.0, 1.0),
            p("flt_bear", "Long when the filter is (0 bullish, 1 bearish)", 0.0, 0.0, 1.0, 1.0),
        ]
    };
    const TF_WM: &str = "ST filter timeframe (2 weekly, 3 monthly)";
    const TF_DWM: &str = "ST filter timeframe (1 daily, 2 weekly, 3 monthly)";
    let gain_x = || p("x_pct", "X% profit in the virtual short trade", 5.0, 0.0, 100.0, 0.1);
    let loss_x = || p("x_pct", "X% loss in the virtual short trade", -5.0, -100.0, 0.0, 0.1);
    let entry_at = || p("entry_at", "Long entry at signal (n-th)", 1.0, 1.0, 10.0, 1.0);
    let post = || {
        vec![
            p("n_long_trades", "Take n long trades post entry condition (0 = no limit)", 0.0, 0.0, 20.0, 1.0),
            p("profit_pct", "% profit in the trades entered (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("profit_type", "Profit type (0 cumulative, 1 individual)", 0.0, 0.0, 1.0, 1.0),
        ]
    };
    let mut v: Vec<ParamDef> = Vec::new();
    match id {
        GAIN1 => {
            v.push(gain_x());
            v.extend(st());
            v.extend(exits());
        }
        GAIN1F => {
            v.push(gain_x());
            v.extend(st());
            v.extend(exits());
            v.extend(flt(2.0, TF_WM, 2.0, 4.0));
        }
        GAIN2 => {
            v.extend([gain_x(), entry_at()]);
            v.extend(st());
            v.extend(exits());
        }
        VS1 => v.extend([
            p("st_period", "ST period", 5.0, 3.0, 9.0, 2.0),
            p("st_factor1", "ST factor 1", 1.0, 1.0, 6.0, 1.0),
            p("st_factor2", "ST factor 2", 2.0, 1.0, 6.0, 1.0),
            p("st_avg", AVG, 5.0, 0.0, 7.0, 1.0),
            p("min_loss_pct", "Minimum loss % of the virtual short (Close above its entry by more than)", 3.0, -100.0, 100.0, 0.1),
            p("flt_type", "Monthly ST filter (0 any, 1 bullish, 2 bearish)", 0.0, 0.0, 2.0, 1.0),
            p("flt_period", "Monthly ST period", 5.0, 1.0, 9.0, 1.0),
            p("flt_factor", "Monthly ST factor", 1.0, 1.0, 6.0, 1.0),
            p("flt_avg", AVG, 5.0, 0.0, 7.0, 1.0),
        ]),
        LOSS2 => {
            v.extend([
                loss_x(),
                p("n_losses", "Number of losses in virtual short trades", 2.0, 1.0, 10.0, 1.0),
                p("loss_type", "Loss type (0 consecutive, 1 non-consecutive)", 0.0, 0.0, 1.0, 1.0),
            ]);
            v.extend(st());
            v.extend(exits());
            v.extend(flt(3.0, TF_DWM, 1.0, 4.0));
        }
        LOSS3 => {
            v.extend([loss_x(), entry_at()]);
            v.extend(st());
            v.extend(exits());
            v.extend(flt(3.0, TF_DWM, 1.0, 4.0));
        }
        POST_N | POST_DD => {
            if id == POST_N {
                v.extend([
                    p("n_losses", "Number of long trades that end up in loss", 0.0, 0.0, 10.0, 1.0),
                    p("loss_type", "Loss type (0 non-consecutive, 1 consecutive)", 0.0, 0.0, 1.0, 1.0),
                ]);
            } else {
                v.push(p("drawdown_pct", "Drawdown %", -5.0, -100.0, 0.0, 0.1));
            }
            v.extend(st());
            v.extend(post());
            v.extend(exits());
            v.extend(flt(1.0, TF_DWM, 1.0, 5.0).into_iter().filter(|d| d.key != "flt_period"));
        }
        NLOSS | SLIDE | SLIDE_ALT => v.extend([
            p("st_tf", "ST timeframe (1 daily, 2 weekly, 3 monthly)", 1.0, 1.0, 3.0, 1.0),
            p("st_period", "ST period", 5.0, 1.0, 100.0, 1.0),
            p("st_factor1", "ST 1 factor", 1.0, 1.0, 6.0, 0.5),
            p("st_factor2", "ST 2 factor", 1.0, 1.0, 6.0, 0.5),
            p("st_avg", AVG, 5.0, 0.0, 7.0, 1.0),
            p("n_losses", "Number of long trades that end up in loss", 0.0, 0.0, 10.0, 1.0),
            p("loss_type", "Loss type (0 non-consecutive, 1 consecutive)", 0.0, 0.0, 1.0, 1.0),
            p("n_long_trades", "Take n long trades post entry condition (0 takes none, as written)", 0.0, 0.0, 20.0, 1.0),
        ]),
        _ => return None,
    }
    Some(v)
}

// ---------------------------------------------------------------- indicators

/// The AFLs' ST on the chosen timeframe as the daily bars see it (1 the chart's own bars, 2 weekly, 3 monthly):
/// (STBuy, STSell) computed on the timeframe, then shifted (`Ref(-1)`, expandFirst, `Ref(+1)`): the period's last
/// day sees its own period, other days the previous one, the data's last bar Null.
fn st_seen(x: &Ctx, tf: i64, period: usize, f: f64, avg: i64) -> (Vec<Option<bool>>, Vec<Option<bool>>) {
    let n = x.c.len();
    let sig = |o: &[f64], h: &[f64], l: &[f64], c: &[f64]| {
        let (line, _) = sniper_st(o, h, l, c, period, f, avg);
        let buy: Vec<Option<bool>> = (0..c.len()).map(|i| line[i].map(|v| c[i] > v)).collect();
        let sell: Vec<Option<bool>> = (0..c.len()).map(|i| line[i].map(|v| c[i] < v)).collect();
        (buy, sell)
    };
    if tf != 2 && tf != 3 {
        return sig(&x.o, &x.h, &x.l, &x.c);
    }
    let b = compress(x, if tf == 3 { 'M' } else { 'W' });
    let mut o = vec![0.0; b.c.len()];
    for i in (0..n).rev() {
        o[b.group[i]] = x.o[i];
    }
    let (buy, sell) = sig(&o, &b.h, &b.l, &b.c);
    let see = |i: usize| -> Option<usize> { if i + 1 >= n { None } else { b.group[i + 1].checked_sub(1) } };
    ((0..n).map(|i| see(i).and_then(|g| buy[g])).collect(), (0..n).map(|i| see(i).and_then(|g| sell[g])).collect())
}

fn t(v: Option<bool>) -> bool {
    v == Some(true)
}

/// The probability AFLs' signals: entry = STBuy1, exit = STSell1 and STSell2, both inside the dateCheck range.
struct ProbSig {
    entry: Vec<bool>,
    exit: Vec<bool>,
}

fn prob_sig(x: &Ctx, g: &dyn Fn(&str) -> f64, from: usize, to: usize) -> ProbSig {
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let avg = g("st_avg").round() as i64;
    let (b1, s1) = st_seen(x, 1, gu("st_period"), g("st_factor1"), avg);
    let (_, s2) = st_seen(x, 1, gu("st_period"), g("st_factor2"), avg);
    let inr = |i: usize| i >= from && i < to;
    let n = x.c.len();
    ProbSig { entry: (0..n).map(|i| inr(i) && t(b1[i])).collect(), exit: (0..n).map(|i| inr(i) && t(s1[i]) && t(s2[i])).collect() }
}

/// The optional ST filter of the virtual-short AFLs: (filter allows a long, filter turned against it).
fn prob_filter(x: &Ctx, g: &dyn Fn(&str) -> f64) -> (Vec<bool>, Vec<bool>) {
    let n = x.c.len();
    if g("flt").round() as i64 != 1 {
        return (vec![true; n], vec![false; n]);
    }
    let (fb, fs) = st_seen(x, g("flt_tf").round() as i64, g("flt_period").round().max(1.0) as usize, g("flt_factor"), g("flt_avg").round() as i64);
    let bear = g("flt_bear").round() as i64 == 1;
    (0..n).map(|i| if bear { (t(fs[i]), t(fb[i])) } else { (t(fb[i]), t(fs[i])) }).unzip()
}

// ---------------------------------------------------------------- the AFLs' HSL / PB / TSL block

struct Risk {
    live: bool,
    hsl_pct: f64,
    pb_pct: f64,
    tsl_pct: f64,
    buf: f64,
    bp: f64,
    hsl: Option<f64>,
    pbl: Option<f64>,
    tsl: Vec<Option<f64>>,
}

impl Risk {
    fn new(n: usize, g: &dyn Fn(&str) -> f64) -> Self {
        Risk { live: g("exit_live").round() as i64 == 1, hsl_pct: g("hsl_pct"), pb_pct: g("pb_pct"), tsl_pct: g("tsl_pct"), buf: g("buffer"), bp: 0.0, hsl: None, pbl: None, tsl: vec![None; n] }
    }
    /// `TSL[i] = TSL[i-1]` at the top of the loop.
    fn carry(&mut self, i: usize) {
        self.tsl[i] = self.tsl[i - 1];
    }
    fn enter(&mut self, x: &Ctx, i: usize) {
        self.bp = x.c[i];
        self.hsl = if self.hsl_pct > 0.0 { Some(self.bp * (1.0 - self.hsl_pct * 0.01)) } else { None };
        self.pbl = if self.pb_pct > 0.0 { Some(self.bp * (1.0 + self.pb_pct * 0.01)) } else { None };
        self.tsl[i] = if self.tsl_pct > 0.0 { Some(self.bp * (1.0 - self.tsl_pct * 0.01)) } else { None };
    }
    /// The sale price and reason when HSL, PB, TSL (the previous bar's) or one of `other` (in the AFL's order) hits.
    fn exit(&self, x: &Ctx, i: usize, other: &[(bool, &'static str)]) -> Option<(f64, &'static str)> {
        let (lo, hi) = if self.live { (x.l[i], x.h[i]) } else { (x.c[i], x.c[i]) };
        let hsl_hit = self.hsl.map_or(false, |v| lo < v);
        let pb_hit = self.pbl.map_or(false, |v| hi > v);
        let tsl_hit = self.tsl[i - 1].map_or(false, |v| v != 0.0 && lo < v);
        let why = if hsl_hit {
            "HSL"
        } else if pb_hit {
            "PB"
        } else if tsl_hit {
            "TSL"
        } else {
            other.iter().find(|o| o.0)?.1
        };
        let o = x.o[i];
        let mut px = x.c[i];
        if self.live && hsl_hit {
            let v = self.hsl.unwrap_or(px);
            px = if o < v { x.l[i].max(o * (1.0 - self.buf * 0.01)) } else { v };
        }
        if self.live && pb_hit {
            let v = self.pbl.unwrap_or(px);
            px = if o > v { x.l[i].max(o * (1.0 - self.buf * 0.01)) } else { v };
        }
        Some((px, why))
    }
    /// `if(Nz(TSL[i-1],0)!=0) TSL[i] = Max(High[i] * (1 - TSL%), TSL[i-1])` while in a trade.
    fn trail(&mut self, x: &Ctx, i: usize) {
        if let Some(v) = self.tsl[i - 1] {
            if v != 0.0 {
                self.tsl[i] = Some((x.h[i] * (1.0 - self.tsl_pct * 0.01)).max(v));
            }
        }
    }
}

fn sell(evs: &mut [Ev], x: &Ctx, i: usize, px: f64, why: &'static str) {
    if let Some(v) = evs.last_mut() {
        v.exit = Some((i, bound(x, i, px), i, why));
    }
}

fn buy(evs: &mut Vec<Ev>, x: &Ctx, i: usize) {
    evs.push(Ev { dir: 1.0, e: i, epx: x.c[i], exit: None });
}

// ---------------------------------------------------------------- Long if x% Gain in Short, V1 and V1 with filter

fn gain_v1(x: &Ctx, g: &dyn Fn(&str) -> f64, filtered: bool, from: usize, to: usize) -> Vec<Ev> {
    let (c, n) = (&x.c, x.c.len());
    let s = prob_sig(x, g, from, to);
    let (f_ok, f_exit) = if filtered { prob_filter(x, g) } else { (vec![true; n], vec![false; n]) };
    let xp = g("x_pct");
    let mut r = Risk::new(n, g);
    let mut evs = Vec::new();
    let (mut vpos, mut pos, mut entry) = (false, false, 0.0f64);
    // changeInPct: Null unless the last virtual short gained
    let mut chg: Option<f64> = None;
    let qualifies = |v: Option<f64>| v.map_or(false, |v| v != 0.0 && v >= xp);
    for i in 1..n {
        r.carry(i);
        if vpos && s.entry[i] {
            chg = if entry - c[i] > 0.0 { Some((entry - c[i]) / entry * 100.0) } else { None };
            vpos = false;
        }
        let mut bought = false;
        if filtered && chg.map_or(false, |v| v != 0.0 && v < xp) {
            chg = None;
        } else if qualifies(chg) && !pos && f_ok[i] && s.entry[i] && i + 1 != n {
            buy(&mut evs, x, i);
            r.enter(x, i);
            pos = true;
            bought = true;
        }
        // V1: `else if(position>0)`; with filter: `if(position>0)`, so the buy bar is tested too
        if pos && (filtered || !bought) {
            if let Some((px, why)) = r.exit(x, i, &[(s.exit[i], "ST exit"), (f_exit[i], "Filter exit"), (i + 1 == n, "Last bar")]) {
                sell(&mut evs, x, i, px, why);
                pos = false;
                if filtered {
                    chg = None;
                }
            }
            r.trail(x, i);
        }
        let may_open = if filtered { chg.is_none() } else { !pos };
        if !vpos && may_open && s.exit[i] {
            entry = c[i];
            vpos = true;
        }
    }
    evs
}

// ---------------------------------------------------------------- n-th signal after a virtual short (gain V2, loss V3)

fn nth_signal(x: &Ctx, g: &dyn Fn(&str) -> f64, gain: bool, from: usize, to: usize) -> Vec<Ev> {
    let (c, n) = (&x.c, x.c.len());
    let s = prob_sig(x, g, from, to);
    let (f_ok, f_exit) = prob_filter(x, g);
    let xp = g("x_pct");
    let nth = g("entry_at").round().max(1.0) as i64;
    let mut r = Risk::new(n, g);
    let mut evs = Vec::new();
    let (mut vpos, mut wait, mut entry) = (false, false, 0.0f64);
    // position 0 flat, 1 a counted (untraded) signal, 2 long
    let (mut pos, mut counter) = (0u8, 0i64);
    for i in 1..n {
        r.carry(i);
        if vpos && s.entry[i] {
            let chg = (entry - c[i]) / entry * 100.0;
            wait = if gain { chg >= xp } else { chg <= xp };
            vpos = false;
        }
        if pos == 0 && wait && s.entry[i] && f_ok[i] && i + 1 != n {
            if counter >= nth - 1 {
                buy(&mut evs, x, i);
                r.enter(x, i);
                pos = 2;
            } else {
                pos = 1;
            }
        } else if pos == 2 {
            if let Some((px, why)) = r.exit(x, i, &[(s.exit[i], "ST exit"), (f_exit[i], "Filter exit"), (i + 1 == n, "Last bar")]) {
                sell(&mut evs, x, i, px, why);
                pos = 0;
                counter = 0;
                wait = false;
            }
            r.trail(x, i);
        } else if pos == 1 && (s.exit[i] || f_exit[i]) {
            counter += 1;
            pos = 0;
        }
        if !vpos && !wait && s.exit[i] {
            entry = c[i];
            vpos = true;
        }
    }
    evs
}

// ---------------------------------------------------------------- n losing virtual shorts (loss V2)

fn loss_count(x: &Ctx, g: &dyn Fn(&str) -> f64, from: usize, to: usize) -> Vec<Ev> {
    let (c, n) = (&x.c, x.c.len());
    let s = prob_sig(x, g, from, to);
    let (f_ok, f_exit) = prob_filter(x, g);
    let xp = g("x_pct");
    let need = g("n_losses").round() as i64;
    let consecutive = g("loss_type").round() as i64 == 0;
    let mut r = Risk::new(n, g);
    let mut evs = Vec::new();
    let (mut vpos, mut pos, mut entry, mut count) = (false, false, 0.0f64, 0i64);
    for i in 1..n {
        r.carry(i);
        if vpos && s.entry[i] {
            let hit = (entry - c[i]) / entry * 100.0 <= xp;
            if hit {
                count += 1;
            } else if consecutive && count > 0 {
                count = 0;
            }
            vpos = false;
        }
        if count >= need && !pos && s.entry[i] && i + 1 != n && f_ok[i] {
            buy(&mut evs, x, i);
            r.enter(x, i);
            pos = true;
            count = 0;
        } else if pos {
            if let Some((px, why)) = r.exit(x, i, &[(s.exit[i], "ST exit"), (f_exit[i], "Filter exit"), (i + 1 == n, "Last bar")]) {
                sell(&mut evs, x, i, px, why);
                pos = false;
            }
            r.trail(x, i);
        }
        if !vpos && !pos && s.exit[i] {
            entry = c[i];
            vpos = true;
        }
    }
    evs
}

// ---------------------------------------------------------------- Version 1 "SuperTrend Virtual Short"

fn virtual_short_v1(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Vec<Ev> {
    let (c, n) = (&x.c, x.c.len());
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let avg = g("st_avg").round() as i64;
    let (d1, _) = sniper_st(&x.o, &x.h, &x.l, c, gu("st_period"), g("st_factor1"), avg);
    let (d2, _) = sniper_st(&x.o, &x.h, &x.l, c, gu("st_period"), g("st_factor2"), avg);
    let (up, dn) = (cross_up(c, &d1), cross_down(c, &d1));
    let ftype = g("flt_type").round() as i64;
    let bull = if ftype == 1 || ftype == 2 { st_seen(x, 3, gu("flt_period"), g("flt_factor"), g("flt_avg").round() as i64).0 } else { vec![None; n] };
    let f_ok = |i: usize| match ftype {
        1 => bull[i] == Some(true),
        2 => bull[i] == Some(false),
        _ => true,
    };
    let min_loss = g("min_loss_pct");
    let v = |z: Option<f64>| z.unwrap_or(NULL);
    let mut evs = Vec::new();
    let (mut long, mut vshort, mut st_exit, mut above2) = (false, false, false, false);
    let mut se = 0.0f64;
    for i in 1..n {
        if long {
            let (l1, l2) = (v(d1[i]), v(d2[i]));
            if c[i] > l2 && !above2 {
                above2 = true;
            }
            let st = (c[i] < l1 && !above2 && c[i] < l2) || (c[i] < l2 && above2);
            if st || i + 1 == n {
                sell(&mut evs, x, i, c[i], if st { "ST exit" } else { "Last bar" });
                long = false;
                st_exit = true;
            }
        }
        if !long && vshort && up[i] && f_ok(i) {
            vshort = false;
            if (c[i] / se - 1.0) * 100.0 > min_loss {
                buy(&mut evs, x, i);
                long = true;
            }
        }
        if (!vshort && dn[i]) || st_exit {
            vshort = true;
            se = c[i];
            st_exit = false;
            above2 = false;
        }
    }
    evs
}

// ---------------------------------------------------------------- Post_STBasedLossesInLongTrades (NLosses V1, Drawdown V2)

fn post_st(x: &Ctx, g: &dyn Fn(&str) -> f64, dd: bool, from: usize, to: usize) -> Vec<Ev> {
    let (c, n) = (&x.c, x.c.len());
    let s = prob_sig(x, g, from, to);
    let on = g("flt").round() as i64 == 1;
    let fb = if on { st_seen(x, g("flt_tf").round() as i64, 5, g("flt_factor"), g("flt_avg").round() as i64).0 } else { vec![None; n] };
    let bear = g("flt_bear").round() as i64 == 1;
    // STBullPhase = Close_M > ST after the shift: false on Null, so NOT of it is true
    let f_pass = |i: usize| !on || if bear { !t(fb[i]) } else { t(fb[i]) };
    let need = g("n_losses").round() as i64;
    let consecutive = g("loss_type").round() as i64 == 1;
    let dd_pct = g("drawdown_pct");
    let n_long = g("n_long_trades").round() as i64;
    let profit_pct = g("profit_pct");
    let individual = g("profit_type").round() as i64 == 1;
    let mut r = Risk::new(n, g);
    let mut evs = Vec::new();
    let (mut pos, mut real, mut entry_cond, mut cont) = (0u8, false, false, true);
    let (mut count, mut total_loss, mut counter, mut total_profit, mut entry) = (0i64, 0.0f64, 0i64, 0.0f64, 0.0f64);
    for i in 1..n {
        r.carry(i);
        let cond_now = if dd { total_loss <= dd_pct } else { count >= need };
        let series_ok = n_long <= 0 || counter == 0;
        if series_ok && profit_pct == 0.0 {
            entry_cond = cond_now;
        }
        if series_ok && cont && pos != 2 && profit_pct > 0.0 {
            entry_cond = cond_now;
        }
        if pos == 0 && s.entry[i] && i + 1 != n {
            if entry_cond && (n_long <= 0 || counter < n_long) && (!dd || f_pass(i)) {
                // NLosses V1: the filter only removes the Buy flag; the loop runs the trade either way
                real = dd || f_pass(i);
                if real {
                    buy(&mut evs, x, i);
                }
                r.enter(x, i);
                pos = 2;
                if dd {
                    total_loss = 0.0;
                } else {
                    count = 0;
                }
                counter += 1;
            } else {
                entry = c[i];
                pos = 1;
                counter = 0;
            }
        } else if pos == 1 {
            if s.exit[i] {
                let lp = (c[i] - entry) / entry * 100.0;
                if dd {
                    total_loss += lp;
                    if total_loss > 0.0 {
                        total_loss = 0.0;
                    }
                } else if lp < 0.0 {
                    count += 1;
                } else if consecutive && count > 0 {
                    count = 0;
                }
                pos = 0;
            }
        } else if pos == 2 {
            if let Some((px, why)) = r.exit(x, i, &[(s.exit[i], "ST exit"), (i + 1 == n, "Last bar")]) {
                if real {
                    sell(&mut evs, x, i, px, why);
                }
                cont = false;
                if profit_pct > 0.0 {
                    let cur = (px - r.bp) / r.bp * 100.0;
                    if individual {
                        if cur >= profit_pct {
                            entry_cond = false;
                            cont = true;
                            total_profit = 0.0;
                        }
                    } else {
                        total_profit += cur;
                        if total_profit >= profit_pct {
                            entry_cond = false;
                            total_profit = 0.0;
                            cont = true;
                        }
                    }
                }
                pos = 0;
            }
            r.trail(x, i);
        }
    }
    evs
}

// ---------------------------------------------------------------- ST-LongTrade_NLoss (plain, sliding window, alternate)

#[derive(Clone, Copy, PartialEq)]
enum NLoss {
    Plain,
    Sliding,
    Alternate,
}

fn st_nloss(x: &Ctx, g: &dyn Fn(&str) -> f64, kind: NLoss, from: usize, to: usize) -> Vec<Ev> {
    let (c, n) = (&x.c, x.c.len());
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (tf, avg) = (g("st_tf").round() as i64, g("st_avg").round() as i64);
    let (b1, _) = st_seen(x, tf, gu("st_period"), g("st_factor1"), avg);
    let (b2, _) = st_seen(x, tf, gu("st_period"), g("st_factor2"), avg);
    let need = g("n_losses").round() as i64;
    let consecutive = g("loss_type").round() as i64 == 1;
    let n_long = g("n_long_trades").round() as i64;
    let dt = |i: usize| kind == NLoss::Sliding || (i >= from && i <= to);
    let resets = kind == NLoss::Plain || (kind == NLoss::Sliding && !consecutive);
    let mut evs = Vec::new();
    // position 0 flat, 1 virtual, 2 long, 3 flat after a real trade
    let (mut pos, mut count, mut counter, mut entry) = (0u8, 0i64, 0i64, 0.0f64);
    for i in 1..n {
        if t(b1[i]) && (pos == 0 || pos == 3) && dt(i) {
            entry = c[i];
            if (pos == 3 || count >= need) && n_long > 0 && counter < n_long {
                buy(&mut evs, x, i);
                pos = 2;
                if resets {
                    count = 0;
                }
                counter += 1;
            } else {
                pos = 1;
                counter = 0;
            }
        }
        if !t(b1[i]) && !t(b2[i]) && (pos == 1 || pos == 2) && dt(i) {
            let lp = (c[i] - entry) / entry * 100.0;
            if lp < 0.0 {
                count += 1;
            } else if lp > 0.0 && consecutive {
                count = 0;
            }
            if kind == NLoss::Sliding && count >= need {
                counter = 0;
            }
            if pos == 2 {
                sell(&mut evs, x, i, c[i], "ST exit");
                if resets {
                    count = 0;
                }
                pos = 3;
            } else {
                pos = 0;
            }
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
    let evs = match id {
        GAIN1 => gain_v1(x, &g, false, from, to),
        GAIN1F => gain_v1(x, &g, true, from, to),
        GAIN2 => nth_signal(x, &g, true, from, to),
        LOSS3 => nth_signal(x, &g, false, from, to),
        LOSS2 => loss_count(x, &g, from, to),
        VS1 => virtual_short_v1(x, &g),
        POST_N => post_st(x, &g, false, from, to),
        POST_DD => post_st(x, &g, true, from, to),
        NLOSS => st_nloss(x, &g, NLoss::Plain, from, to),
        SLIDE => st_nloss(x, &g, NLoss::Sliding, from, to),
        SLIDE_ALT => st_nloss(x, &g, NLoss::Alternate, from, to),
        _ => return None,
    };
    Some(collect(x, evs, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Supertrend"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    fn b(c: f64) -> (f64, f64, f64, f64) {
        (c - 0.3, c + 1.0, c - 1.2, c)
    }

    /// Up 15 bars, down 10 (the virtual short gains ~16%), up 10 (as `ytst`'s GAIN test).
    fn gain_series() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..15).map(|k| (100.0 + k as f64, 101.5 + k as f64, 99.0 + k as f64, 100.5 + k as f64)).collect();
        v.extend((0..10).map(|k| (113.0 - 2.0 * k as f64, 114.0 - 2.0 * k as f64, 111.0 - 2.0 * k as f64, 112.0 - 2.0 * k as f64)));
        v.extend((0..10).map(|k| (95.0 + 2.0 * k as f64, 97.0 + 2.0 * k as f64, 94.5 + 2.0 * k as f64, 96.5 + 2.0 * k as f64)));
        v
    }

    /// A rise, then `times` × (one sharp drop bar, a gap back up well above it, a further rise): each virtual short
    /// opened on the drop loses ~10% at the next ST buy.
    fn loss_series(times: usize) -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..15).map(|k| b(100.0 + k as f64)).collect();
        let mut lvl = 114.0;
        for _ in 0..times {
            v.push(b(lvl - 9.0));
            lvl += 7.0;
            for k in 0..8 {
                v.push(b(lvl + k as f64));
            }
            lvl += 8.0;
        }
        v
    }

    #[test]
    fn gain_v1_buys_after_winning_virtual_short() {
        let r = bars(&gain_series());
        for id in [GAIN1, GAIN1F] {
            let t = go(run, id, &r, &[]);
            assert_eq!(t.len(), 1, "{id}");
            assert!(t[0].0 > 25, "{id}");
            assert!(go(run, id, &r, &[("x_pct", 50.0)]).is_empty(), "{id}");
        }
    }

    #[test]
    fn gain_v2_buys_at_nth_signal() {
        let r = bars(&gain_series());
        let t = go(run, GAIN2, &r, &[]);
        assert_eq!(t.len(), 1);
        assert!(t[0].0 > 25);
        assert!(go(run, GAIN2, &r, &[("x_pct", 50.0)]).is_empty());
        // the 2nd signal never comes: the series rises to the end after the first
        assert!(go(run, GAIN2, &r, &[("entry_at", 2.0)]).is_empty());
    }

    #[test]
    fn loss_v3_buys_after_losing_virtual_short() {
        let r = bars(&loss_series(1));
        let t = go(run, LOSS3, &r, &[]);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].0, 16); // the gap-up bar after the drop at bar 15
        assert_eq!((t[0].2, t[0].4), (r.len() - 1, false), "sold on the data's last bar (deadstocksellsig)");
        // a gain version sees no winning short here
        assert!(go(run, GAIN2, &r, &[]).is_empty());
    }

    #[test]
    fn loss_v2_needs_two_losing_virtual_shorts() {
        let one = go(run, LOSS2, &bars(&loss_series(1)), &[]);
        assert!(one.is_empty());
        assert_eq!(go(run, LOSS2, &bars(&loss_series(1)), &[("n_losses", 1.0)]).len(), 1);
        let two = go(run, LOSS2, &bars(&loss_series(2)), &[]);
        assert_eq!(two.len(), 1);
        assert_eq!(two[0].0, 25); // the second gap-up bar
    }

    #[test]
    fn virtual_short_v1_buys_when_short_lost() {
        let r = bars(&loss_series(1));
        let t = go(run, VS1, &r, &[]);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].0, 16);
        assert!(go(run, VS1, &r, &[("min_loss_pct", 20.0)]).is_empty());
    }

    /// Rise, a fall below the first entry (a losing long), then a rally.
    fn long_loss_series() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..10).map(|k| b(100.0 + 0.5 * k as f64)).collect();
        v.extend((0..5).map(|k| b(96.0 - 2.0 * k as f64)));
        v.extend((0..10).map(|k| b(92.0 + 2.0 * k as f64)));
        v
    }

    #[test]
    fn post_st_waits_for_losing_virtual_long() {
        let r = bars(&long_loss_series());
        let plain = go(run, POST_N, &r, &[]);
        assert_eq!(plain.len(), 2, "defaults: every ST buy is traded");
        let t = go(run, POST_N, &r, &[("n_losses", 1.0)]);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].0, plain[1].0);
        let d = go(run, POST_DD, &r, &[]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].0, plain[1].0);
        // NLosses V1's filter removes the Buy only: with a bearish filter on a rising daily ST nothing trades
        assert!(go(run, POST_N, &r, &[("flt", 1.0), ("flt_bear", 1.0), ("flt_factor", 6.0)]).iter().all(|t| t.0 > 15));
    }

    #[test]
    fn st_nloss_variants_trade_after_a_loss() {
        let r = bars(&long_loss_series());
        for id in [NLOSS, SLIDE, SLIDE_ALT] {
            assert!(go(run, id, &r, &[]).is_empty(), "{id}: n_long_trades 0 takes no trade");
            let all = go(run, id, &r, &[("n_long_trades", 5.0)]);
            let t = go(run, id, &r, &[("n_long_trades", 1.0), ("n_losses", 1.0)]);
            assert_eq!(t.len(), 1, "{id}");
            assert_eq!(all.len(), 2, "{id}");
            assert_eq!(t[0].0, all[1].0, "{id}");
        }
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        for id in IDS {
            assert!(family_of(id).is_some(), "{id}");
            assert!(params(id).is_some(), "{id}");
        }
    }
}
