//! YT Strategies · Tripple RSI: the team's AmiBroker AFLs in "Old Topics/Tripple RSI" (monthly, weekly and daily RSI
//! of one series; the team ran them on spot Nifty / Bank Nifty and on individual stocks, each symbol on its own daily
//! bars). Coded from the AFLs as written, with the AFLs' own default settings; there is no book PDF.
//!
//! Coded (one id per distinct rule set; the many copies under "Triple RSI - Dharang", "Checking" and "Hrishit" are
//! optimisation copies of the ATHC AFL with the same defaults and only other Param / Optimize ranges):
//! - `yttr_mwd_cross`: "RSI Monthly, Weekly and Daily Crossover based Entry.afl" (RSI trail 20 points, 5% close trail);
//! - `yttr_mwd_cross_athc`: "... with ATHC Stock Filter, RSI Ranking with HSL.afl" (same entry; ATHC filter and hard
//!   SL options, both off by default, and RSI trail / % trail 0, so a position is held until the data end);
//! - `yttr_mwd_cross_swing_exit`: "... with HSL_FINAL.afl" (adds `OR trend1 == -1`: sell when the 20% close swing
//!   is down, which is on by default because the swing is computed whether or not the ATHC filter is on);
//! - `yttr_mwd_trail_entry`: "RSI Monthly, Weekly above 60 and Daily RSI Trail based Entry.afl";
//! - `yttr_dw_cross`: "Daily and Weekly RSI Crossover.afl";
//! - `yttr_athc_fo`: "... with ATHC Stock Filter, RSI Ranking with HSL (FO).afl" (AFL folder) and
//!   `yttr_athc_fo_checking`: the copy of that name in "Checking". Both are the ATHC AFL plus `Aux2`: the buy needs
//!   `Aux2[i] == 1`, the hard SL in the sell test is `( hard_sl AND L <= hsl ) AND Aux2[i+1] == K` (AND binds tighter
//!   than OR in AFL, so the AUX2 term qualifies only the hard SL), and the LC-deferral test adds `OR Aux2[i+1] == K`;
//!   K = 1 in the AFL folder copy, K = 0 in the Checking copy. Both also sell when the ATHC filter (if on) turns off.
//!   AUX2 is the stock's dated F&O membership in the team's C&P cash database (Database Directory: "Use to Run on FO
//!   Stocks but Cash Prices", AUX2), read from the native AmiBroker file (`crate::yt_aux::AUX2_SERIES`).
//!   `Aux2[i+1]` reads the next bar (look-ahead, kept as written). On the last loaded bar there is no next bar:
//!   AmiBroker stops the formula with "Error 10. Array subscript out of range" there (AFL evaluates every operand of
//!   AND / OR, so the term is read whenever the sell test runs on the last bar with a position); the engine instead
//!   treats the missing next bar as neither 1 nor 0 (the AUX2 terms are false) and keeps the bar's other rules.
//!
//! AmiBroker behaviour followed:
//! - `SetBarsRequired(100000, 0)`: the bar loops run over the whole history; positions are reported when they are
//!   opened inside the test range (a position the loop holds from before the range blocks entries until it exits);
//! - `TimeFrameSet(inWeekly / inMonthly)` + `TimeFrameExpand(.., expandLast)` (the default mode): the weekly or
//!   monthly RSI of the compressed closes reaches the daily bars on the period's last bar; earlier days of the
//!   period carry the previous period's value (`ytsw::compress` / `expand_last`, Monday-Sunday weeks);
//! - RSI is Wilder's (`connors::rsi`), Null for the first `period` bars; an entry comparison that reads a Null value
//!   is never true; inside the trail-entry AFL's state machine Null reads as -1e10 (AFL loop value);
//! - the loops test the entry first and the exit in an `else` branch, so no exit and re-entry on one bar; fills at
//!   the close (hard SL at its level or the open less the buffer), kept inside the bar's range;
//! - DEAD STOCK BACKTEST defaults to NO: no forced exit on the last bar, no entry on bar 0; UC (High = Close) blocks
//!   an entry, an exit on an LC bar (Low = Close) waits for the next non-LC bar; penny 0 and turnover off.
//!
//! The RSI ranking (`PositionScore`) of the ATHC AFLs ranks across symbols in a portfolio run; it is off by default
//! and plays no part in a single-symbol run.
//! The "Nifty-BN (Index)_Optim on Spot" folder holds only an analysis .docx (no AFL).
#![allow(dead_code)]
use crate::books::ytsw::{close_swing, compress, expand_last};
use crate::books::ytwh::{bound, collect, loop_val, Ev};
use crate::books::ab::rsi_nan as rsi;
// AFL `TSL = HC_Af_Buy - (HC_Af_Buy * TSL_Pct/100)` is float32 (AFL numbers are float32), compared with float32 Close
use crate::books::ab::{addmul, f32r};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_tr";
pub const CROSS: &str = "yttr_mwd_cross";
pub const ATHC: &str = "yttr_mwd_cross_athc";
pub const SWING_EXIT: &str = "yttr_mwd_cross_swing_exit";
pub const TRAIL_ENTRY: &str = "yttr_mwd_trail_entry";
pub const DW_CROSS: &str = "yttr_dw_cross";
pub const ATHC_FO: &str = "yttr_athc_fo";
pub const ATHC_FO_CHECKING: &str = "yttr_athc_fo_checking";
pub const IDS: [&str; 7] = [CROSS, ATHC, SWING_EXIT, TRAIL_ENTRY, DW_CROSS, ATHC_FO, ATHC_FO_CHECKING];
/// The "(FO)" AFLs: they read the stock's dated AUX2 (F&O membership) and run on actual C&P cash stocks only.
pub const FO_IDS: [&str; 2] = [ATHC_FO, ATHC_FO_CHECKING];

const GROUP: &str = "YT Strategies · Tripple RSI";

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

fn fo_def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { needs_series: &[crate::yt_aux::AUX2_SERIES], ..def(id, name, rules, notes) }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(CROSS, "Monthly, Weekly and Daily RSI Crossover", &[
            "F|Monthly RSI(2) > 60 and weekly RSI(3) > 60 (the higher time frame value of the last completed period, the current one on its last day)",
            "E|Daily RSI(14) crosses above 60 (above today, below yesterday): buy at the Close",
            "X|RSI trail: daily RSI falls below its highest value since entry less 20 points",
            "X|% trail: Close at or below the highest Close since entry (to yesterday) less 5%",
            "X|Daily RSI below the exit value (0: off)",
            "X|Sell at the Close; on a bar whose Low = Close the sale waits for the next bar that is not",
            "F|No entry on a bar whose High = Close",
        ], "RSI Monthly, Weekly and Daily Crossover based Entry.afl (Tripple RSI/AFL), coded as the AFL is written. Defaults as the AFL: RSI periods 2 / 3 / 14, all entry values 60, RSI trail 20, trail SL 5%, daily exit value 0. Dead-stock NO, penny 0 and turnover off are the stock-template defaults."),
        def(ATHC, "Monthly, Weekly and Daily RSI Crossover (ATHC filter, hard SL)", &[
            "F|Monthly RSI(2) > 60 and weekly RSI(3) > 60",
            "F|ATHC filter (off by default): on from a Close above the highest earlier Close, off once the 20% close swing turns down",
            "E|Daily RSI(14) crosses above 60: buy at the Close",
            "X|RSI trail and % trail as the base AFL, both 0 (off) by default; daily RSI exit value 0 (off)",
            "X|Hard SL (off by default): Low at or below the buy less 1%, sold at that level or at the Open less the 0.25% buffer on a gap below it",
            "X|ATHC exit variant (off): the copies in Checking / Triple RSI - Dharang also sell when the ATHC filter turns off",
            "X|With the defaults no exit rule is on: the position is held to the end of the data",
            "F|No entry on a bar whose High = Close; an LC exit (not the hard SL) waits for the next non-LC bar",
        ], "RSI Monthly, Weekly and Daily Crossover based Entry with ATHC Stock Filter, RSI Ranking with HSL.afl (Tripple RSI/AFL), coded as the AFL is written; the copies under Checking, Triple RSI - Dharang (NSENIFTY, BANKNIFTY, BN, Stocks, PC 2, PC 3, hR, New folder, New folder (2), Trash, Trash 2) have the same defaults with other Param / Optimize ranges, and all but the AFL-folder copy add the ATHC exit (athc_exit = 1). The daily RSI period default 14 lies outside its Param range 2-10 and is used as written. RSI scoring (PositionScore across symbols) is off by default and not used."),
        def(SWING_EXIT, "Monthly, Weekly and Daily RSI Crossover (20% Swing Exit)", &[
            "F|Monthly RSI(2) > 60 and weekly RSI(3) > 60",
            "F|ATHC filter as the ATHC AFL (off by default)",
            "E|Daily RSI(14) crosses above 60: buy at the Close",
            "X|Sell when the 20% close swing trend is down (Close below the swing band that trails 20% under the Closes)",
            "X|RSI trail / % trail / RSI exit value 0 (off); hard SL off by default",
            "X|Sell at the Close; on an LC bar the sale waits for the next non-LC bar",
            "F|No entry on a bar whose High = Close",
        ], "RSI Monthly, Weekly and Daily Crossover based Entry with ATHC Stock Filter, RSI Ranking with HSL_FINAL.afl (Tripple RSI/AFL; the Hrishit/With ATH Filter/.../% TRAIL SL copy has the same defaults with Optimize ranges), coded as the AFL is written: its sell rule adds trend1 == -1, and trend1 is computed even with the ATHC filter off. The buy does not check the swing, so a buy in a down swing is sold on the next bar."),
        def(TRAIL_ENTRY, "Monthly, Weekly above 60 and Daily RSI Trail Entry", &[
            "F|Monthly RSI(2) > 60 and weekly RSI(3) > 60",
            "F|Daily RSI(8) state while flat: a fall in the RSI records the low (stage 1); the next rise sets the entry level = that low + the trail entry value (0) (stage 2); a new lower RSI in stage 2 lowers the low and the level",
            "E|Stage 2 and daily RSI above the entry level and below 60: buy at the Close",
            "X|Daily RSI below 40",
            "X|% trail: Close at or below the highest Close since entry (to yesterday) less 5%",
            "X|RSI trail exit value 0 (off)",
            "X|Sell at the Close; on an LC bar the exit is re-checked on the next bars and taken on the first non-LC bar where it still holds",
            "F|No entry on a bar whose High = Close",
        ], "RSI Monthly, Weekly above 60 and Daily RSI Trail based Entry.afl (Tripple RSI/AFL), coded as the AFL is written. Defaults as the AFL: RSI periods 2 / 3 / 8, monthly and weekly 60, daily threshold 60, exit 40, trail entry 0, trail exit 0, trail SL 5%. Its sell branch has no SkipSell check, so an exit blocked by an LC bar is simply re-tested (AFL quirk kept)."),
        def(DW_CROSS, "Daily and Weekly RSI Crossover", &[
            "E|Daily RSI(14) crosses above the weekly RSI(14) (expanded to daily bars): buy at the Close",
            "X|Daily RSI crosses below the weekly RSI: sell at the Close",
            "X|On an LC bar the sale waits for the next non-LC bar",
            "F|No entry on a bar whose High = Close",
        ], "Daily and Weekly RSI Crossover.afl (Tripple RSI/AFL), coded as the AFL is written. Both RSI periods 14 (the AFL defaults)."),
        fo_def(ATHC_FO, "Monthly, Weekly and Daily RSI Crossover (ATHC filter, hard SL, F&O stocks by AUX2)", &[
            "F|AUX2 = 1 on the entry day: the stock is an F&O stock on that date (dated AUX2 of the team's C&P cash database)",
            "F|Monthly RSI(2) > 60 and weekly RSI(3) > 60",
            "F|ATHC filter (off by default): on from a Close above the highest earlier Close, off once the 20% close swing turns down",
            "E|Daily RSI(14) crosses above 60: buy at the Close",
            "X|RSI trail and % trail as the base AFL, both 0 (off) by default; daily RSI exit value 0 (off)",
            "X|ATHC filter turns off (only when the filter is on)",
            "X|Hard SL (off by default): Low at or below the buy less 1%, taken only when AUX2 of the NEXT day is 1 (look-ahead as written); sold at that level or at the Open less the 0.25% buffer on a gap below it",
            "X|With the hard SL on, a sale on a day whose Low reaches the hard SL level is filled at that level (or the gapped Open less the buffer), whichever rule sold (the AFL's SellPrice)",
            "X|With the defaults no exit rule is on: the position is held to the end of the data",
            "F|No entry on a bar whose High = Close; an LC exit waits for the next non-LC bar unless the hard SL is hit, the ATHC filter is off or AUX2 of the next day is 1",
        ], "RSI Monthly, Weekly and Daily Crossover based Entry with ATHC Stock Filter, RSI Ranking with HSL (FO).afl (Tripple RSI/AFL), coded as the AFL is written: the ATHC AFL plus Buy AND Aux2[i]==1, the sell test ... OR ( hard_sl_switch == \"ON\" AND L[i] <= hsl_price ) AND Aux2[i+1]==1 (AFL's AND binds tighter than OR, so the AUX2 term qualifies the hard SL only) and the LC test ... OR Aux2[i+1]==1. AUX2 is the stock's dated F&O membership in the team's C&P cash database (Database Directory: Use to Run on FO Stocks but Cash Prices), read from the native AmiBroker file; the run needs an actual C&P cash stock with that file (AMIBROKER_DATA_DIR) and reports needs_data otherwise. Aux2[i+1] reads the next day (look-ahead in the AFL). On the last loaded bar AmiBroker stops with Error 10 (array subscript out of range) when the sell test runs with a position; the engine treats the missing next bar as neither 1 nor 0. Defaults as the AFL (Optimize returns its default in a backtest): RSI periods 2 / 3 / 14, entry values 60, RSI trail 0, trail SL 0, filter OFF, hard SL OFF 1% with 0.25% buffer, swing 20%; the weekly period default 3 lies outside its Param range 4-20 and is used as written. RSI scoring is off by default and not used."),
        fo_def(ATHC_FO_CHECKING, "Monthly, Weekly and Daily RSI Crossover (ATHC filter, hard SL, F&O stocks by AUX2, Checking copy)", &[
            "F|AUX2 = 1 on the entry day: the stock is an F&O stock on that date (dated AUX2 of the team's C&P cash database)",
            "F|Monthly RSI(2) > 60 and weekly RSI(3) > 60",
            "F|ATHC filter (off by default): on from a Close above the highest earlier Close, off once the 20% close swing turns down",
            "E|Daily RSI(14) crosses above 60: buy at the Close",
            "X|RSI trail and % trail as the base AFL, both 0 (off) by default; daily RSI exit value 0 (off)",
            "X|ATHC filter turns off (only when the filter is on)",
            "X|Hard SL (off by default): Low at or below the buy less 1%, taken only when AUX2 of the NEXT day is 0, i.e. the stock leaves F&O tomorrow (look-ahead as written); sold at that level or at the Open less the 0.25% buffer on a gap below it",
            "X|With the hard SL on, a sale on a day whose Low reaches the hard SL level is filled at that level (or the gapped Open less the buffer), whichever rule sold (the AFL's SellPrice)",
            "X|With the defaults no exit rule is on: the position is held to the end of the data",
            "F|No entry on a bar whose High = Close; an LC exit waits for the next non-LC bar unless the hard SL is hit, the ATHC filter is off or AUX2 of the next day is 0",
        ], "RSI Monthly, Weekly and Daily Crossover based Entry with ATHC Stock Filter, RSI Ranking with HSL (FO).afl (Tripple RSI/Checking), coded as the AFL is written. It differs from the AFL-folder copy only in Aux2[i+1]==0 (not ==1) in the sell test (( hard_sl_switch == \"ON\" AND L[i] <= hsl_price ) AND Aux2[i+1]==0, the AUX2 term qualifying the hard SL only, as AND binds tighter than OR) and in the LC test (OR Aux2[i+1]==0), and in Param instead of Optimize with other ranges (same defaults). Buy AND Aux2[i]==1. AUX2 is the stock's dated F&O membership in the team's C&P cash database (Database Directory: Use to Run on FO Stocks but Cash Prices), read from the native AmiBroker file; the run needs an actual C&P cash stock with that file (AMIBROKER_DATA_DIR) and reports needs_data otherwise. Aux2[i+1] reads the next day (look-ahead in the AFL); on the last loaded bar AmiBroker stops with Error 10 (array subscript out of range) when the sell test runs with a position; the engine treats the missing next bar as neither 1 nor 0. Defaults: RSI periods 2 / 3 / 14, entry values 60, RSI trail 0, trail SL 0, filter OFF, hard SL OFF 1% with 0.25% buffer, swing 20%. RSI scoring is off by default and not used."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mwd = |d_prd: f64| {
        vec![
            p("m_prd", "Monthly RSI period", 2.0, 1.0, 100.0, 1.0),
            p("m_val", "Monthly RSI entry value", 60.0, 0.0, 100.0, 5.0),
            p("w_prd", "Weekly RSI period", 3.0, 1.0, 100.0, 1.0),
            p("w_val", "Weekly RSI entry value", 60.0, 0.0, 100.0, 5.0),
            p("d_prd", "Daily RSI period", d_prd, 1.0, 100.0, 1.0),
        ]
    };
    Some(match id {
        CROSS | ATHC | SWING_EXIT | ATHC_FO | ATHC_FO_CHECKING => {
            let base = id == CROSS;
            let mut v = mwd(14.0);
            v.extend([
                p("d_val", "Daily RSI entry value", 60.0, 0.0, 100.0, 5.0),
                p("d_exit", "Daily RSI exit value (0 off)", 0.0, 0.0, 100.0, 5.0),
                p("rsi_trail", "RSI trail points (0 off)", if base { 20.0 } else { 0.0 }, 0.0, 50.0, 5.0),
                p("tsl_pct", "Trail SL % (0 off)", if base { 5.0 } else { 0.0 }, 0.0, 100.0, 1.0),
            ]);
            if !base {
                v.extend([
                    p("athc", "ATHC filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
                    p("swing_pct", "Swing exit %", 20.0, 0.0, 100.0, 1.0),
                    p("hard_sl", "Hard SL (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
                    p("hsl_pct", "Hard SL %", 1.0, 0.0, 20.0, 0.25),
                    p("hsl_buffer", "Hard SL buffer %", 0.25, 0.0, 20.0, 0.25),
                ]);
            }
            if id == ATHC {
                v.push(p("athc_exit", "Sell when the ATHC filter turns off (0 AFL folder, 1 Checking / Dharang copies)", 0.0, 0.0, 1.0, 1.0));
            }
            v
        }
        TRAIL_ENTRY => {
            let mut v = mwd(8.0);
            v.extend([
                p("d_thr", "Daily RSI entry threshold (below)", 60.0, 0.0, 100.0, 5.0),
                p("d_exit", "Daily RSI exit value", 40.0, 0.0, 100.0, 5.0),
                p("trail_entry", "RSI trail entry value", 0.0, 0.0, 40.0, 5.0),
                p("trail_exit", "RSI trail exit value (0 off)", 0.0, 0.0, 40.0, 5.0),
                p("tsl_pct", "Trail SL % (0 off)", 5.0, 0.0, 20.0, 0.05),
            ]);
            v
        }
        DW_CROSS => vec![p("w_prd", "Weekly RSI period", 14.0, 1.0, 100.0, 1.0), p("d_prd", "Daily RSI period", 14.0, 1.0, 100.0, 1.0)],
        _ => return None,
    })
}

// ---------------------------------------------------------------- series

fn opt(v: f64) -> Option<f64> {
    if v.is_finite() {
        Some(v)
    } else {
        None
    }
}

/// Wilder RSI of the daily closes (None while AmiBroker's RSI is Null).
fn rsi_d(c: &[f64], n: usize) -> Vec<Option<f64>> {
    rsi(c, n.max(1)).into_iter().map(opt).collect()
}

/// RSI of the weekly ('W') or monthly ('M') closes, TimeFrameExpand(.., expandLast) to the daily bars.
pub(crate) fn rsi_tf(x: &Ctx, tf: char, n: usize) -> Vec<Option<f64>> {
    let b = compress(x, tf);
    let r = rsi(&b.c, n.max(1));
    expand_last(&r, &b).into_iter().map(opt).collect()
}

/// The AFLs' buy filter with the stock-template defaults: not bar 0 (dead stock NO), High != Close (UC), Close > 0.
fn buy_ok(x: &Ctx, i: usize) -> bool {
    i != 0 && x.h[i] != x.c[i] && x.c[i] > 0.0
}

fn lc(x: &Ctx, i: usize) -> bool {
    x.l[i] == x.c[i]
}

// ---------------------------------------------------------------- monthly / weekly / daily crossover family

pub(crate) struct Mwd {
    pub d: Vec<Option<f64>>,
    pub w: Vec<Option<f64>>,
    pub m: Vec<Option<f64>>,
}

#[derive(Clone, Copy)]
pub(crate) struct CrossCfg {
    pub d_val: f64,
    pub w_val: f64,
    pub m_val: f64,
    pub d_exit: f64,
    pub rsi_trail: f64,
    pub tsl_pct: f64,
    pub athc: bool,
    pub athc_exit: bool,
    pub swing_exit: bool,
    pub swing_pct: f64,
    pub hard_sl: bool,
    pub hsl_pct: f64,
    pub hsl_buf: f64,
}

/// ATHCBuysig of the ATHC AFLs: 1 from a Close above the highest earlier Close (HHV(C, BarCount) of the previous bar),
/// 0 once the % close swing is down while it was 1, otherwise carried. Returns (filter, swing trend).
pub(crate) fn athc_filter(c: &[f64], swing_pct: f64) -> (Vec<bool>, Vec<i8>) {
    let n = c.len();
    let trend = close_swing(c, swing_pct);
    let mut sig = vec![false; n];
    let mut hc = if n > 0 { c[0] } else { 0.0 };
    for i in 1..n {
        sig[i] = if c[i] > hc { true } else if trend[i] == -1 && sig[i - 1] { false } else { sig[i - 1] };
        hc = hc.max(c[i]);
    }
    (sig, trend)
}

/// The "(FO)" AFLs' AUX2 terms: `aux2` is the stock's dated AUX2 on its bars (NaN where unknown: neither 1 nor 0),
/// `next_eq` the K of `Aux2[i+1] == K` (1 AFL folder, 0 Checking). `gate` is an extra dated entry condition supplied by
/// a portfolio basket (AmiBroker-style mode); None in a single-stock run.
#[derive(Clone, Copy)]
pub(crate) struct Fo<'a> {
    pub aux2: &'a [f64],
    pub next_eq: f64,
    pub gate: Option<&'a [bool]>,
}

pub(crate) fn cross_loop(x: &Ctx, s: &Mwd, k: &CrossCfg) -> Vec<Ev> {
    cross_loop_fo(x, s, k, None)
}

pub(crate) fn cross_loop_fo(x: &Ctx, s: &Mwd, k: &CrossCfg, fo: Option<Fo>) -> Vec<Ev> {
    let (o, l, c) = (&x.o, &x.l, &x.c);
    let n = c.len();
    let (athc, trend) = athc_filter(c, k.swing_pct);
    let mut evs: Vec<Ev> = Vec::new();
    let (mut pos, mut skip) = (false, false);
    let (mut hi_rsi, mut rsi_trail, mut hc, mut tsl, mut hsl_px) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let sell = |evs: &mut Vec<Ev>, i: usize, px: f64, r: &'static str| {
        if let Some(v) = evs.last_mut() {
            v.exit = Some((i, bound(x, i, px), i, r));
        }
    };
    for i in 1..n {
        let entry = match (s.d[i], s.d[i - 1], s.w[i], s.m[i]) {
            (Some(d), Some(dp), Some(w), Some(m)) => d > k.d_val && dp < k.d_val && w > k.w_val && m > k.m_val,
            _ => false,
        };
        let aux_entry = fo.map_or(true, |f| f.aux2.get(i) == Some(&1.0) && f.gate.map_or(true, |g| g.get(i) == Some(&true)));
        if !pos && entry && buy_ok(x, i) && (!k.athc || athc[i]) && aux_entry {
            let d = s.d[i].unwrap_or(0.0);
            hsl_px = c[i] * (1.0 - k.hsl_pct / 100.0);
            hi_rsi = d;
            if k.tsl_pct > 0.0 {
                hc = c[i];
                tsl = addmul(hc, -k.tsl_pct / 100.0);
            }
            if k.rsi_trail > 0.0 {
                rsi_trail = hi_rsi - k.rsi_trail;
            }
            pos = true;
            evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
        } else if pos && !skip {
            let d = loop_val(s.d[i]);
            if k.rsi_trail > 0.0 {
                hi_rsi = d.max(hi_rsi);
                rsi_trail = hi_rsi - k.rsi_trail;
            }
            if k.tsl_pct > 0.0 {
                hc = hc.max(c[i]);
            }
            let hsl_hit = k.hard_sl && l[i] <= hsl_px;
            // with the filter OFF the AFL sets ATHCBuysig = 1, so `ATHCBuysig[i] == 0` never sells
            let athc_off = k.athc_exit && k.athc && !athc[i];
            // FO AFLs: `Aux2[i+1] == K`; no next bar (the last loaded bar) or an unknown value is neither 1 nor 0
            let aux_next = fo.is_some_and(|f| f.aux2.get(i + 1).is_some_and(|&v| v == f.next_eq));
            // `... OR ( hard_sl AND L <= hsl ) AND Aux2[i+1] == K`: AND binds tighter than OR in AFL
            let hsl_sell = hsl_hit && (fo.is_none() || aux_next);
            let cond = f32r(c[i]) <= tsl || d < rsi_trail || d < k.d_exit || athc_off || (k.swing_exit && trend[i] == -1) || hsl_sell;
            if cond {
                if !lc(x, i) || hsl_hit || athc_off || aux_next {
                    let r = if hsl_sell {
                        "Hard SL"
                    } else if f32r(c[i]) <= tsl {
                        "Trail SL"
                    } else if d < rsi_trail {
                        "RSI trail"
                    } else if d < k.d_exit {
                        "Daily RSI exit"
                    } else if athc_off {
                        "ATHC filter off"
                    } else {
                        "Swing down"
                    };
                    // SellPrice: the hard SL price whenever the Low reached it, whichever rule sold (FO AFLs: also when
                    // the hard SL's own AUX2 term is false)
                    if hsl_hit {
                        sell(&mut evs, i, if o[i] < hsl_px { o[i] - o[i] * k.hsl_buf / 100.0 } else { hsl_px }, r);
                    } else {
                        sell(&mut evs, i, c[i], r);
                    }
                    pos = false;
                    rsi_trail = 0.0;
                    hi_rsi = 0.0;
                    hc = 0.0;
                    tsl = 0.0;
                } else {
                    skip = true;
                }
            } else if k.tsl_pct > 0.0 {
                tsl = addmul(hc, -k.tsl_pct / 100.0);
            }
        } else if skip && !lc(x, i) {
            sell(&mut evs, i, c[i], "Blocked exit, next bar");
            pos = false;
            skip = false;
            rsi_trail = 0.0;
            hi_rsi = 0.0;
            hc = 0.0;
            tsl = 0.0;
        }
    }
    evs
}

// ---------------------------------------------------------------- RSI trail entry

#[derive(Clone, Copy)]
pub(crate) struct TrailCfg {
    pub w_val: f64,
    pub m_val: f64,
    pub d_thr: f64,
    pub d_exit: f64,
    pub trail_entry: f64,
    pub trail_exit: f64,
    pub tsl_pct: f64,
}

pub(crate) fn trail_loop(x: &Ctx, s: &Mwd, k: &TrailCfg) -> Vec<Ev> {
    let c = &x.c;
    let n = c.len();
    let mut evs: Vec<Ev> = Vec::new();
    let mut pos = false;
    let (mut count, mut lowest, mut rsi_entry) = (0i32, 0.0f64, 9_999_999.0f64);
    let (mut hi_rsi, mut rsi_trail, mut hc, mut tsl) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for i in 1..n {
        let (d, dp) = (loop_val(s.d[i]), loop_val(s.d[i - 1]));
        if !pos && d < dp && count <= 1 {
            lowest = d;
            count = 1;
        } else if !pos && d > 0.0 && d > dp && count == 1 {
            count = 2;
            rsi_entry = lowest + k.trail_entry;
        } else if !pos && lowest > 0.0 && d < lowest && count == 2 {
            lowest = d;
            rsi_entry = lowest + k.trail_entry;
        }
        if !pos && d > rsi_entry && d < k.d_thr && count == 2 && loop_val(s.w[i]) > k.w_val && loop_val(s.m[i]) > k.m_val && buy_ok(x, i) {
            hi_rsi = d;
            if k.tsl_pct > 0.0 {
                hc = c[i];
                tsl = addmul(hc, -k.tsl_pct / 100.0);
            }
            if k.trail_exit > 0.0 {
                rsi_trail = hi_rsi - k.trail_exit;
            }
            pos = true;
            lowest = 0.0;
            count = 0;
            rsi_entry = 9_999_999.0;
            evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
        } else if pos {
            if k.trail_exit > 0.0 {
                hi_rsi = d.max(hi_rsi);
                rsi_trail = hi_rsi - k.trail_exit;
            }
            if k.tsl_pct > 0.0 {
                hc = hc.max(c[i]);
            }
            if f32r(c[i]) <= tsl || d < rsi_trail || d < k.d_exit {
                // the AFL sets SkipSell on an LC bar, but its sell branch never reads it: the exit is re-tested next bar
                if !lc(x, i) {
                    let r = if f32r(c[i]) <= tsl { "Trail SL" } else if d < rsi_trail { "RSI trail" } else { "Daily RSI exit" };
                    if let Some(v) = evs.last_mut() {
                        v.exit = Some((i, bound(x, i, c[i]), i, r));
                    }
                    pos = false;
                    rsi_trail = 0.0;
                    hi_rsi = 0.0;
                    hc = 0.0;
                    tsl = 0.0;
                }
            } else if k.tsl_pct > 0.0 {
                tsl = addmul(hc, -k.tsl_pct / 100.0);
            }
        }
    }
    evs
}

// ---------------------------------------------------------------- daily / weekly crossover

pub(crate) fn dw_loop(x: &Ctx, d: &[Option<f64>], w: &[Option<f64>]) -> Vec<Ev> {
    let n = x.c.len();
    let mut evs: Vec<Ev> = Vec::new();
    let (mut pos, mut skip) = (false, false);
    let rel = |i: usize| -> Option<(f64, f64, f64, f64)> { Some((d[i]?, w[i]?, d[i - 1]?, w[i - 1]?)) };
    for i in 1..n {
        let r = rel(i);
        if !pos && r.map_or(false, |(a, b, ap, bp)| a > b && ap < bp) && buy_ok(x, i) {
            pos = true;
            evs.push(Ev { dir: 1.0, e: i, epx: x.c[i], exit: None });
        } else if pos && !skip {
            if r.map_or(false, |(a, b, ap, bp)| a < b && ap > bp) {
                if !lc(x, i) {
                    if let Some(v) = evs.last_mut() {
                        v.exit = Some((i, bound(x, i, x.c[i]), i, "Daily RSI below weekly"));
                    }
                    pos = false;
                } else {
                    skip = true;
                }
            }
        } else if skip && !lc(x, i) {
            if let Some(v) = evs.last_mut() {
                v.exit = Some((i, bound(x, i, x.c[i]), i, "Blocked exit, next bar"));
            }
            pos = false;
            skip = false;
        }
    }
    evs
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    run_with_gate(id, x, from, to, p, None)
}

/// `run` with an extra dated entry condition from a portfolio basket (AmiBroker-style mode, FO ids only): the
/// basket's AUX rule joins the AFL's own `Aux2[i] == 1` on the Buy, inside the loop.
pub(crate) fn run_with_gate(id: &str, x: &Ctx, from: usize, to: usize, p: &Params, gate: Option<&[bool]>) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let gb = |k: &str| g(k).round() as i64 == 1;
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let fo = if FO_IDS.contains(&id) {
        // readiness reports needs_data when the stock's dated AUX2 is not loaded; no AUX2 is ever assumed
        let Some(aux2) = x.extra.get(crate::yt_aux::AUX2_SERIES).filter(|a| a.len() == n) else { return Some(Vec::new()) };
        if gate.is_some_and(|g| g.len() != n) {
            return None;
        }
        Some(Fo { aux2, next_eq: if id == ATHC_FO { 1.0 } else { 0.0 }, gate })
    } else {
        None
    };
    let evs = match id {
        DW_CROSS => dw_loop(x, &rsi_d(&x.c, gu("d_prd")), &rsi_tf(x, 'W', gu("w_prd"))),
        _ => {
            let s = Mwd { d: rsi_d(&x.c, gu("d_prd")), w: rsi_tf(x, 'W', gu("w_prd")), m: rsi_tf(x, 'M', gu("m_prd")) };
            if id == TRAIL_ENTRY {
                let k = TrailCfg { w_val: g("w_val"), m_val: g("m_val"), d_thr: g("d_thr"), d_exit: g("d_exit"), trail_entry: g("trail_entry"), trail_exit: g("trail_exit"), tsl_pct: g("tsl_pct") };
                trail_loop(x, &s, &k)
            } else {
                let base = id == CROSS;
                let fo_afl = fo.is_some();
                let k = CrossCfg {
                    d_val: g("d_val"),
                    w_val: g("w_val"),
                    m_val: g("m_val"),
                    d_exit: g("d_exit"),
                    rsi_trail: g("rsi_trail"),
                    tsl_pct: g("tsl_pct"),
                    athc: !base && gb("athc"),
                    athc_exit: fo_afl || (id == ATHC && gb("athc_exit")),
                    swing_exit: id == SWING_EXIT,
                    swing_pct: if base { 20.0 } else { g("swing_pct") },
                    hard_sl: !base && gb("hard_sl"),
                    hsl_pct: if base { 1.0 } else { g("hsl_pct") },
                    hsl_buf: if base { 0.25 } else { g("hsl_buffer") },
                };
                cross_loop_fo(x, &s, &k, fo)
            }
        }
    };
    Some(collect(x, evs, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Tripple RSI"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    /// Quiet bars (Close mid-range, so neither UC nor LC).
    fn quiet(n: usize) -> Vec<(f64, f64, f64, f64)> {
        (0..n).map(|k| { let c = 100.0 + k as f64 * 0.1; (c, c + 1.0, c - 1.0, c) }).collect()
    }
    fn some(v: &[f64]) -> Vec<Option<f64>> {
        v.iter().map(|&a| if a.is_nan() { None } else { Some(a) }).collect()
    }
    fn cfg() -> CrossCfg {
        CrossCfg { d_val: 60.0, w_val: 60.0, m_val: 60.0, d_exit: 0.0, rsi_trail: 20.0, tsl_pct: 5.0, athc: false, athc_exit: false, swing_exit: false, swing_pct: 20.0, hard_sl: false, hsl_pct: 1.0, hsl_buf: 0.25 }
    }
    fn hi(n: usize) -> Vec<Option<f64>> {
        vec![Some(70.0); n]
    }
    fn ev(e: &[Ev]) -> Vec<(usize, Option<usize>, &'static str)> {
        e.iter().map(|v| (v.e, v.exit.map(|t| t.0), v.exit.map_or("", |t| t.3))).collect()
    }

    #[test]
    fn cross_entry_and_rsi_trail_exit() {
        let r = bars(&quiet(8));
        let x = Ctx::new(&r, &r, &[]);
        let nan = f64::NAN;
        // bar 1: previous Null -> no cross; bar 3 crosses 60; peak 85 at bar 4; bar 6 is 64 < 85 - 20
        let d = some(&[nan, 65.0, 55.0, 62.0, 85.0, 70.0, 64.0, 70.0]);
        let s = Mwd { d: d.clone(), w: hi(8), m: hi(8) };
        assert_eq!(ev(&cross_loop(&x, &s, &cfg())), vec![(3, Some(6), "RSI trail")]);
        // weekly not above 60: no entry; a Null weekly value: no entry
        let mut w = hi(8);
        w[3] = Some(60.0);
        assert!(cross_loop(&x, &Mwd { d: d.clone(), w, m: hi(8) }, &cfg()).is_empty());
        let mut m = hi(8);
        m[3] = None;
        assert!(cross_loop(&x, &Mwd { d, w: hi(8), m }, &cfg()).is_empty());
    }

    #[test]
    fn trail_sl_uses_previous_bars_level_and_lc_defers() {
        let mut v = quiet(8);
        v[2] = (100.0, 101.0, 99.0, 100.0); // entry at 100
        v[3] = (110.0, 111.0, 109.0, 110.0); // highest close 110 -> TSL 104.5 from bar 4 on
        v[4] = (105.0, 106.0, 104.5, 104.5); // close 104.5 <= 104.5 but Low = Close (LC): deferred
        v[5] = (104.0, 105.0, 103.0, 104.0); // next non-LC bar: sold at the close
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let d = some(&[50.0, 50.0, 61.0, 70.0, 70.0, 70.0, 70.0, 70.0]);
        let e = cross_loop(&x, &Mwd { d, w: hi(8), m: hi(8) }, &cfg());
        assert_eq!(ev(&e), vec![(2, Some(5), "Blocked exit, next bar")]);
        assert!((e[0].exit.unwrap().1 - 104.0).abs() < 1e-9);
    }

    #[test]
    fn trail_sl_is_float32_in_the_afl_shape() {
        // EVEREADY 2003: highest close 26, TSL 15% = 26 - 26 * 15 / 100 = 22.100000381 in float32 = the Close 22.1
        // (float32), so AmiBroker sells on that bar; 64-bit 22.1 > 22.099999999999998 held one more bar
        let mut v = quiet(8);
        v[2] = (26.0, 26.5, 25.5, 26.0);
        v[3] = (25.0, 25.5, 24.5, 25.0);
        v[4] = (22.5, 23.0, 22.0, 22.1);
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let d = some(&[50.0, 50.0, 61.0, 70.0, 70.0, 70.0, 70.0, 70.0]);
        let k = CrossCfg { rsi_trail: 0.0, tsl_pct: 15.0, ..cfg() };
        assert_eq!(ev(&cross_loop(&x, &Mwd { d, w: hi(8), m: hi(8) }, &k)), vec![(2, Some(4), "Trail SL")]);
    }

    #[test]
    fn athc_defaults_hold_and_hard_sl_fills() {
        let mut v = quiet(8);
        v[4] = (95.0, 96.0, 94.0, 95.5); // gap below 100.1 x 0.99
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let d = some(&[50.0, 50.0, 61.0, 10.0, 10.0, 10.0, 10.0, 10.0]);
        let s = Mwd { d, w: hi(8), m: hi(8) };
        let off = CrossCfg { rsi_trail: 0.0, tsl_pct: 0.0, ..cfg() };
        assert_eq!(ev(&cross_loop(&x, &s, &off)), vec![(2, None, "")]);
        let e = cross_loop(&x, &s, &CrossCfg { hard_sl: true, ..off });
        assert_eq!(ev(&e), vec![(2, Some(4), "Hard SL")]);
        assert!((e[0].exit.unwrap().1 - 95.0 * 0.9975).abs() < 1e-9);
    }

    #[test]
    fn athc_filter_and_swing_exit() {
        // new closing high at bar 2, a 20% fall turns the swing down at bar 4
        let c = [100.0, 99.0, 101.0, 100.0, 79.0, 80.0];
        let (sig, tr) = athc_filter(&c, 20.0);
        assert_eq!(sig, vec![false, false, true, true, false, false]);
        assert_eq!(tr, vec![1, 1, 1, 1, -1, -1]);
        let v: Vec<_> = c.iter().map(|&c| (c, c + 1.0, c - 1.0, c)).collect();
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let d = some(&[50.0, 50.0, 61.0, 70.0, 70.0, 70.0]);
        let s = Mwd { d, w: hi(6), m: hi(6) };
        let k = CrossCfg { rsi_trail: 0.0, tsl_pct: 0.0, swing_exit: true, ..cfg() };
        assert_eq!(ev(&cross_loop(&x, &s, &k)), vec![(2, Some(4), "Swing down")]);
        // the ATHC filter blocks an entry before a new closing high
        let d2 = some(&[50.0, 61.0, 61.0, 61.0, 61.0, 61.0]);
        assert!(cross_loop(&x, &Mwd { d: d2, w: hi(6), m: hi(6) }, &CrossCfg { athc: true, ..k }).is_empty());
    }

    #[test]
    fn trail_entry_state_machine() {
        let r = bars(&quiet(9));
        let x = Ctx::new(&r, &r, &[]);
        // falls to 30 (stage 1), rises to 35 (stage 2, level 30), 28 lowers the level, 33 > 28 and < 60: buy at bar 5;
        // daily RSI 39 < 40 at bar 7 sells
        let d = some(&[50.0, 40.0, 30.0, 35.0, 28.0, 33.0, 45.0, 39.0, 45.0]);
        let k = TrailCfg { w_val: 60.0, m_val: 60.0, d_thr: 60.0, d_exit: 40.0, trail_entry: 0.0, trail_exit: 0.0, tsl_pct: 5.0 };
        let e = trail_loop(&x, &Mwd { d: d.clone(), w: hi(9), m: hi(9) }, &k);
        // bar 3 (35 > level 30 in stage 2 the same bar) already buys; after the exit the machine restarts: 39 at
        // bar 7 is a new low, 45 at bar 8 rises above it and buys
        assert_eq!(ev(&e), vec![(3, Some(4), "Daily RSI exit"), (8, None, "")]);
        // an entry level of +10: 35 is not above 40; 28 lowers it to 38; 45 at bar 6 > 38 buys; 39 < 40 sells
        let e = trail_loop(&x, &Mwd { d, w: hi(9), m: hi(9) }, &TrailCfg { trail_entry: 10.0, ..k });
        assert_eq!(ev(&e), vec![(6, Some(7), "Daily RSI exit")]);
    }

    #[test]
    fn daily_weekly_cross() {
        let r = bars(&quiet(6));
        let x = Ctx::new(&r, &r, &[]);
        let d = some(&[40.0, 40.0, 55.0, 56.0, 45.0, 60.0]);
        let w = some(&[50.0, 50.0, 50.0, 50.0, 50.0, 50.0]);
        assert_eq!(ev(&dw_loop(&x, &d, &w)), vec![(2, Some(4), "Daily RSI below weekly"), (5, None, "")]);
    }

    #[test]
    fn weekly_rsi_reaches_days_on_the_weeks_last_bar() {
        let r = wave(60);
        let x = Ctx::new(&r, &r, &[]);
        let w = rsi_tf(&x, 'W', 3);
        let b = compress(&x, 'W');
        let raw = rsi(&b.c, 3);
        // a Wednesday of week 6 shows week 5's value; its Friday shows its own
        assert_eq!(w[32], opt(raw[5]));
        assert_eq!(w[34], opt(raw[6]));
    }

    fn fo(aux2: &[f64], next_eq: f64) -> Option<Fo<'_>> {
        Some(Fo { aux2, next_eq, gate: None })
    }

    #[test]
    fn fo_entry_needs_aux2_and_hard_sl_reads_next_days_aux2() {
        let mut v = quiet(8);
        v[4] = (99.5, 100.0, 98.0, 99.5); // Low 98 <= 100.2 x 0.99 = 99.198: hard SL hit at bar 4 (open above the level)
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let d = some(&[50.0, 50.0, 61.0, 70.0, 70.0, 70.0, 70.0, 70.0]);
        let s = Mwd { d, w: hi(8), m: hi(8) };
        let k = CrossCfg { rsi_trail: 0.0, tsl_pct: 0.0, hard_sl: true, athc_exit: true, ..cfg() };
        let one = [1.0; 8];
        // AFL folder copy (K = 1): AUX2 of bar 5 is 1, so the hard SL sells at bar 4 at the level
        let e = cross_loop_fo(&x, &s, &k, fo(&one, 1.0));
        assert_eq!(ev(&e), vec![(2, Some(4), "Hard SL")]);
        assert!((e[0].exit.unwrap().1 - 100.2 * 0.99).abs() < 1e-9);
        // Checking copy (K = 0): next-day AUX2 is 1, so the hard SL is not taken and the position is held
        assert_eq!(ev(&cross_loop_fo(&x, &s, &k, fo(&one, 0.0))), vec![(2, None, "")]);
        // ... and taken when the stock leaves F&O the next day
        let mut leaves = one;
        leaves[5] = 0.0;
        assert_eq!(ev(&cross_loop_fo(&x, &s, &k, fo(&leaves, 0.0))), vec![(2, Some(4), "Hard SL")]);
        assert_eq!(ev(&cross_loop_fo(&x, &s, &k, fo(&leaves, 1.0))), vec![(2, None, "")]);
        // entry blocked when AUX2 is 0 (or unknown) on the crossing day; there is no later crossing
        for blocked in [0.0, f64::NAN] {
            let mut a = one;
            a[2] = blocked;
            assert!(cross_loop_fo(&x, &s, &k, fo(&a, 1.0)).is_empty());
        }
        // a basket gate (portfolio mode) joins the Buy the same way
        let mut g = [true; 8];
        g[2] = false;
        assert!(cross_loop_fo(&x, &s, &k, Some(Fo { aux2: &one, next_eq: 1.0, gate: Some(&g) })).is_empty());
    }

    #[test]
    fn fo_next_day_aux2_lets_an_lc_exit_through_and_the_last_bar_has_no_next_day() {
        let mut v = quiet(8);
        v[2] = (100.0, 101.0, 99.0, 100.0); // entry at 100
        v[3] = (110.0, 111.0, 109.0, 110.0); // TSL 104.5 from bar 4 on
        v[4] = (105.0, 106.0, 104.5, 104.5); // close <= TSL on an LC bar
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let d = some(&[50.0, 50.0, 61.0, 70.0, 70.0, 70.0, 70.0, 70.0]);
        let s = Mwd { d, w: hi(8), m: hi(8) };
        let k = CrossCfg { rsi_trail: 0.0, athc_exit: true, ..cfg() };
        let one = [1.0; 8];
        // K = 1 and AUX2 of bar 5 is 1: sold on the LC bar itself
        assert_eq!(ev(&cross_loop_fo(&x, &s, &k, fo(&one, 1.0))), vec![(2, Some(4), "Trail SL")]);
        // K = 0: no bypass, the LC exit waits for the next non-LC bar
        assert_eq!(ev(&cross_loop_fo(&x, &s, &k, fo(&one, 0.0))), vec![(2, Some(5), "Blocked exit, next bar")]);
        // hard SL on the last loaded bar: no next day, so `Aux2[i+1] == K` is false for either K
        let mut w = quiet(6);
        w[5] = (99.0, 100.0, 95.0, 99.0);
        let r = bars(&w);
        let x = Ctx::new(&r, &r, &[]);
        let s = Mwd { d: some(&[50.0, 50.0, 61.0, 70.0, 70.0, 70.0]), w: hi(6), m: hi(6) };
        let k = CrossCfg { rsi_trail: 0.0, tsl_pct: 0.0, hard_sl: true, athc_exit: true, ..cfg() };
        for eq in [1.0, 0.0] {
            assert_eq!(ev(&cross_loop_fo(&x, &s, &k, fo(&[1.0, 1.0, 1.0, 1.0, 1.0, eq], eq))), vec![(2, None, "")]);
        }
    }

    #[test]
    fn fo_ids_run_from_the_ctx_aux2_series() {
        // a rising series with daily swings: monthly / weekly RSI above 60, daily RSI crossing 60 again and again
        let v: Vec<_> = (0..900).map(|k| { let k = k as f64; let c = 100.0 + 0.4 * k + 12.0 * (k / 6.0).sin(); (c, c + 1.0, c - 1.0, c) }).collect();
        let r = bars(&v);
        let mut x = Ctx::new(&r, &r, &[]);
        let n = r.len();
        let p = ps(&[("rsi_trail", 20.0)]);
        // no AUX2 loaded: no trades (readiness reports needs_data before a run)
        for id in FO_IDS {
            assert!(run(id, &x, 0, n - 1, &p).unwrap().is_empty());
        }
        // F&O every day, hard SL and the ATHC filter off: both equal the ATHC AFL
        x.extra.insert(crate::yt_aux::AUX2_SERIES.into(), vec![1.0; n]);
        let athc = run(ATHC, &x, 0, n - 1, &p).unwrap();
        assert!(athc.len() >= 2, "{}", athc.len());
        for id in FO_IDS {
            let t = run(id, &x, 0, n - 1, &p).unwrap();
            assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), athc.iter().map(|t| (t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), "{id}");
        }
        // not F&O on the first entry day: that entry is gone
        let first = athc[0].legs[0].i;
        let mut a = vec![1.0; n];
        a[first] = 0.0;
        x.extra.insert(crate::yt_aux::AUX2_SERIES.into(), a);
        for id in FO_IDS {
            let t = run(id, &x, 0, n - 1, &p).unwrap();
            assert!(!t.is_empty() && t.iter().all(|t| t.legs[0].i != first), "{id}");
        }
        x.extra.insert(crate::yt_aux::AUX2_SERIES.into(), vec![0.0; n]);
        for id in FO_IDS {
            assert!(run(id, &x, 0, n - 1, &p).unwrap().is_empty());
            assert!(catalogue().iter().any(|d| d.id == id && d.needs_series == [crate::yt_aux::AUX2_SERIES]));
        }
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        for id in IDS {
            assert!(params(id).unwrap().iter().all(|d| d.def >= d.min && d.def <= d.max), "{id}");
        }
    }
}
