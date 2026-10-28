//! YT Strategies · NR7: the team's AmiBroker AFLs in "Old Topics/NR7". Every AFL trades one stock at a time on its
//! own daily bars (the team ran them over F&O and NSE cash stock universes with MaxOpenPositions 20; the portfolio
//! limits and position sizing are not modelled). Coded from the AFLs as written, with each AFL's default settings
//! (an `Optimize()` uses its default); there is no book PDF and nothing here is compared with the team's workbooks.
//!
//! AmiBroker behaviour the code follows (see `ytsw.rs` / `ytd7.rs` for the shared conventions):
//! - the AFLs run their own bar loop from bar 1 over the stock's full history and set Buy / Sell and the prices;
//!   the backtest takes an entry only when flat and only in the test window, ignores an exit while flat, and on one
//!   bar exits first and then enters; a position still open at the window's end is marked at that close;
//! - inside a loop an element that is Null is the number -1e10 (so `C[i] < Sell_cnd` is false while the ATR stop is
//!   Null); whole-array comparisons (the NR7 test) are false before the arrays' first values;
//! - the Supertrend is the team's ST loop (`ytsw::supertrend`: ATR(1) Null on bar 0, bands Null until the period's
//!   ATRs exist, trend held while they are Null); ATR(14) is AmiBroker's Wilder ATR;
//! - `HHV(C, BarCount)` is the highest close so far (all-time-high close); the ATHC regime uses the team's % swing
//!   on the close (`ytsw::close_swing`);
//! - the template guards as written: Prec(x, 2) truncates to 2 decimals; upper circuit = open, close and high equal
//!   at 2 decimals (no entry), lower circuit = open, close and low equal (no exit that bar); penny-stock price 0;
//!   turnover filter off (when on, turnover is close x volume, against the AFL's year table); "dead stock" backtest
//!   off (when on, no entry on the last data bar and a forced sale there);
//! - prices are kept inside the bar's low-high range (AmiBroker's price bound check).
#![allow(dead_code)]
use crate::books::ytd7::lv;
use crate::books::ytroc::atr_wilder;
use crate::books::ytsw::{close_swing, supertrend};
use crate::books::ytwh::{bound, llv, prec2};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_nr7";
pub const NR7_ST: &str = "ytnr7_nr7_st";
pub const ATHC_ST: &str = "ytnr7_athc_st";
pub const ATHC_LOW2: &str = "ytnr7_athc_low2";
pub const ATHC_ST_LOW2: &str = "ytnr7_athc_st_or_low2";
pub const ATHC_LLV: &str = "ytnr7_athc_close_llv";
pub const INSIDE_ST: &str = "ytnr7_inside_st";
pub const INSIDE_LOW2: &str = "ytnr7_inside_low2";
pub const INSIDE_ST_LOW2: &str = "ytnr7_inside_st_or_low2";
pub const MOTHER_ST: &str = "ytnr7_mother_st";
pub const MOTHER_LOW2: &str = "ytnr7_mother_low2";
pub const MOTHER_ST_LOW2: &str = "ytnr7_mother_st_or_low2";
pub const IDS: [&str; 11] = [NR7_ST, ATHC_ST, ATHC_LOW2, ATHC_ST_LOW2, ATHC_LLV, INSIDE_ST, INSIDE_LOW2, INSIDE_ST_LOW2, MOTHER_ST, MOTHER_LOW2, MOTHER_ST_LOW2];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · NR7", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const F_NR7: &str = "F|NR7: the previous bar's range (H-L) is below each of the 6 ranges before it, and none of the 7 ranges is 0";
const F_ATHC: &str = "F|ATHC regime: on when the close is above the highest earlier close; off when the 10% close swing turns down; NR7 setups count only while it is on, and it going off cancels a waiting setup";
const F_INSIDE: &str = "F|The previous bar is also an inside bar (high below and low above the bar before it)";
const F_GUARD: &str = "F|Template guards: no entry on an upper-circuit bar, close above the penny price (0), turnover filter off, dead-stock backtest off; a setup whose breakout bar fails a guard is dropped (wait for a new NR7)";
const E_NR7H: &str = "E|Buy at the close when the close is above the NR7 bar's high (the setup bar, kept until used or cancelled)";
const E_INSH: &str = "E|Buy at the close when the close is above the inside (NR7) bar's high";
const E_MOTH: &str = "E|Buy at the close when the close is above the mother bar's high (the bar before the inside NR7 bar)";
const X_ST: &str = "X|Entered with the daily Supertrend (5, 1, mid point) up: sell at the close when it turns down. Entered with it down: sell at the close below entry close - 1 x ATR(14); if the ST turns up first, switch to the ST exit";
const X_LOW2: &str = "X|Sell at the close on a bar whose low is below the lower of the previous 2 lows (from the bar after entry)";
const X_ST_LOW2: &str = "X|Supertrend / ATR exit as in the ST version, or (whichever is earlier) a low below the lower of the previous 2 lows; at the close";
const X_CIRC: &str = "X|No exit on a lower-circuit bar (open, low and close equal)";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(NR7_ST, "NR7 Breakout, Supertrend / ATR Exit", &[F_NR7, "F|No ATHC regime; a waiting setup is cleared on bars when a position is open", E_NR7H, X_ST, X_CIRC, "F|Template guards: no entry on an upper-circuit bar, close above the penny price (0), turnover filter off, dead-stock backtest off"],
            concat!("Entry with NR7 & exit with ST as SL.afl (NR7/Entry with NR7 & exit with ST), coded as the AFL is written.", " Runs on one stock at a time; the team ran it over F&O and NSE cash stock universes; portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled.", " The AFL's 'Buffer %' parameter is not used by its rules. The ATR stop is fixed at entry. Which of the two exits applies is decided by the ST direction on the entry bar.")),
        def(ATHC_ST, "NR7 + ATHC Breakout, Supertrend / ATR Exit", &[F_NR7, F_ATHC, E_NR7H, X_ST, X_CIRC, F_GUARD],
            concat!("Narrow Range 7 With ATHC and exit with ST.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over F&O and NSE cash stock universes (header: stock selection Close > ATHC); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled.", " The ATHC regime gates entries only; an open trade is not closed when it goes off.")),
        def(ATHC_LOW2, "NR7 + ATHC Breakout, 2-Day Low Exit", &[F_NR7, F_ATHC, E_NR7H, X_LOW2, X_CIRC, F_GUARD],
            concat!("Narrow Range 7 With ATHC and exit with Prev. 2 Days Lowest Low - New.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over F&O and NSE cash stock universes (header: stock selection Close > ATHC); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled.", " The exit fires on the low but fills at the close (SellPrice = Close).")),
        def(ATHC_ST_LOW2, "NR7 + ATHC Breakout, ST or 2-Day Low Exit", &[F_NR7, F_ATHC, E_NR7H, X_ST_LOW2, X_CIRC, F_GUARD],
            concat!("Narrow Range 7 With ATHC and exit with ST OR Prev. 2 Days Lowest Low (Whichever is earlier) - New.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over F&O and NSE cash stock universes (header: stock selection Close > ATHC); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled.", " The 2-day-low exit fills at the close. Kept quirk: when a trade entered in an ST down trend leaves on the 2-day low, the AFL does not clear its ATR-stop flag, which only matters through the ST switch of the next trade (as written).")),
        def(ATHC_LLV, "NR7 + ATHC Breakout, Close Below N-Day Low Exit", &[F_NR7, F_ATHC, E_NR7H, "X|Sell at the close when the close is below the lowest low of the previous N bars (N 2)", X_CIRC, F_GUARD],
            concat!("NR7 With ATHC Filter and exit with Prev. N-Days Lowest Low.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over F&O and NSE cash stock universes (header: stock selection Close > ATHC); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled.", " Unlike the 2-day-low versions this exit tests the close, not the low.")),
        def(INSIDE_ST, "NR7 Inside Bar + ATHC, Supertrend / ATR Exit", &[F_NR7, F_INSIDE, F_ATHC, E_INSH, X_ST, X_CIRC, F_GUARD],
            concat!("Narrow Range 7 With Inside Bar and exit with SuperTrend.afl, coded as the AFL is written (the ' - Copy' file has the same rules and defaults and is not coded twice).", " Runs on one stock at a time; the team ran it over F&O and NSE cash stock universes (header: stock selection Close > ATHC); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled.")),
        def(INSIDE_LOW2, "NR7 Inside Bar + ATHC, 2-Day Low Exit", &[F_NR7, F_INSIDE, F_ATHC, E_INSH, X_LOW2, X_CIRC, F_GUARD],
            concat!("Narrow Range 7 With Inside Bar and exit with Prev. 2 Days Lowest Low.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over F&O and NSE cash stock universes (header: stock selection Close > ATHC); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled.", " The exit fills at the close.")),
        def(INSIDE_ST_LOW2, "NR7 Inside Bar + ATHC, ST or 2-Day Low Exit", &[F_NR7, F_INSIDE, F_ATHC, E_INSH, X_ST_LOW2, X_CIRC, F_GUARD],
            concat!("Narrow Range 7 With Inside Bar and exit with SuperTrend OR Prev. 2 Days Lowest Low (Whichever is earlier).afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over F&O and NSE cash stock universes (header: stock selection Close > ATHC); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled.")),
        def(MOTHER_ST, "NR7 Mother Bar + ATHC, Supertrend / ATR Exit", &[F_NR7, F_INSIDE, F_ATHC, E_MOTH, X_ST, X_CIRC, F_GUARD],
            concat!("Narrow Range 7 With Mother Bar and exit with SuperTrend.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over F&O and NSE cash stock universes (header: stock selection Close > ATHC); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled.")),
        def(MOTHER_LOW2, "NR7 Mother Bar + ATHC, 2-Day Low Exit", &[F_NR7, F_INSIDE, F_ATHC, E_MOTH, X_LOW2, X_CIRC, F_GUARD],
            concat!("Narrow Range 7 With Mother Bar and exit with Prev. 2 Days Lowest Low.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over F&O and NSE cash stock universes (header: stock selection Close > ATHC); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled.", " The exit fills at the close.")),
        def(MOTHER_ST_LOW2, "NR7 Mother Bar + ATHC, ST or 2-Day Low Exit", &[F_NR7, F_INSIDE, F_ATHC, E_MOTH, X_ST_LOW2, X_CIRC, F_GUARD],
            concat!("Narrow Range 7 With Mother Bar and exit with SuperTrend OR Prev. 2 Days Lowest Low (Whichever is earlier).afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over F&O and NSE cash stock universes (header: stock selection Close > ATHC); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled.")),
    ]
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Setup {
    Nr7,
    Inside,
    Mother,
}
#[derive(Clone, Copy, PartialEq, Debug)]
enum Exit {
    St,
    Low2,
    StOrLow2,
    CloseLlv,
}
#[derive(Clone, Copy, Debug)]
struct V {
    setup: Setup,
    athc: bool,
    exit: Exit,
}

fn variant(id: &str) -> Option<V> {
    let v = |setup, athc, exit| Some(V { setup, athc, exit });
    match id {
        NR7_ST => v(Setup::Nr7, false, Exit::St),
        ATHC_ST => v(Setup::Nr7, true, Exit::St),
        ATHC_LOW2 => v(Setup::Nr7, true, Exit::Low2),
        ATHC_ST_LOW2 => v(Setup::Nr7, true, Exit::StOrLow2),
        ATHC_LLV => v(Setup::Nr7, true, Exit::CloseLlv),
        INSIDE_ST => v(Setup::Inside, true, Exit::St),
        INSIDE_LOW2 => v(Setup::Inside, true, Exit::Low2),
        INSIDE_ST_LOW2 => v(Setup::Inside, true, Exit::StOrLow2),
        MOTHER_ST => v(Setup::Mother, true, Exit::St),
        MOTHER_LOW2 => v(Setup::Mother, true, Exit::Low2),
        MOTHER_ST_LOW2 => v(Setup::Mother, true, Exit::StOrLow2),
        _ => None,
    }
}

pub(crate) fn pd(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef {
    ParamDef { key, label, def, min, max, step }
}

/// The template's stock guards (shared by the yt_vts / yt_fsv modules).
pub(crate) fn template_params() -> Vec<ParamDef> {
    vec![
        pd("penny", "Penny stock price (entry needs a close above it)", 0.0, 0.0, 100.0, 1.0),
        pd("turnover", "Turnover filter (0 no, 1 yes; close x volume against the AFL's year table)", 0.0, 0.0, 1.0, 1.0),
        pd("dead_stock", "Dead stock backtest (0 yes: no entry on and forced sale at the last data bar, 1 no)", 1.0, 0.0, 1.0, 1.0),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let v = variant(id)?;
    let mut out = Vec::new();
    if v.athc {
        out.push(pd("swing_pct", "ATHC swing exit %", 10.0, 1.0, 35.0, 1.0));
    }
    if matches!(v.exit, Exit::St | Exit::StOrLow2) {
        out.push(pd("st_period", "ST ATR periods", 5.0, 3.0, 20.0, 1.0));
        out.push(pd("st_factor", "ST factor", 1.0, 1.0, 12.0, 0.5));
        out.push(pd("atr_period", "ATR periods (stop for entries in an ST down trend)", 14.0, 5.0, 100.0, 1.0));
        out.push(pd("atr_mult", "ATR multiplier", 1.0, 0.0, 5.0, 0.025));
    }
    if v.exit == Exit::CloseLlv {
        out.push(pd("ll_period", "Lowest low period", 2.0, 1.0, 20.0, 1.0));
    }
    out.extend(template_params());
    Some(out)
}

// ---------------------------------------------------------------- shared helpers (also used by yt_vts / yt_fsv)

/// One AFL loop's output: the entry price where Buy is set, and the exit price and reason where Sell is set.
pub(crate) struct Loop {
    pub entry: Vec<Option<f64>>,
    pub exit: Vec<Option<(f64, &'static str)>>,
}

impl Loop {
    pub(crate) fn new(n: usize) -> Self {
        Loop { entry: vec![None; n], exit: vec![None; n] }
    }
}

/// AmiBroker backtest of loop signals in from..=to: one position; on a bar the open position exits first, then a
/// flat book takes an entry; trade prices are kept inside the bar's range; an open position is marked at `to`.
pub(crate) fn fills(x: &Ctx, s: &Loop, from: usize, to: usize, kind: &'static str) -> Vec<Trade> {
    let n = x.c.len();
    let mut out = Vec::new();
    if n == 0 || from > to || to >= n {
        return out;
    }
    let mk = |e: usize, epx: f64, i: usize, px: f64, why: &str, open: bool| Trade {
        dir: 1.0,
        legs: vec![Leg { i: e, px: epx, w: 1.0 }],
        exit_i: i,
        exit_px: px,
        reason: why.into(),
        entry_kind: kind,
        signal_i: i,
        open,
        exits: Vec::new(),
    };
    let mut pos: Option<(usize, f64)> = None;
    for i in from..=to {
        if let (Some((e, epx)), Some((px, why))) = (pos, s.exit[i]) {
            out.push(mk(e, epx, i, bound(x, i, px), why, false));
            pos = None;
        }
        if pos.is_none() {
            if let Some(px) = s.entry[i] {
                pos = Some((i, bound(x, i, px)));
            }
        }
    }
    if let Some((e, epx)) = pos {
        out.push(mk(e, epx, to, x.c[to], "open", true));
    }
    out
}

/// The team's ATHC loop: on when the close beats the highest earlier close, off when the swing trend is down (while
/// on), else carried; `init` is the value on bar 0.
pub(crate) fn athc(c: &[f64], trend: &[i8], init: bool) -> Vec<bool> {
    let n = c.len();
    let mut out = vec![init; n];
    let mut hc = if n > 0 { c[0] } else { 0.0 };
    for i in 1..n {
        out[i] = if c[i] > hc { true } else if trend[i] == -1 && out[i - 1] { false } else { out[i - 1] };
        hc = hc.max(c[i]);
    }
    out
}

pub(crate) fn year(x: &Ctx, i: usize) -> i64 {
    x.r[i].date.get(..4).and_then(|s| s.parse().ok()).unwrap_or(2000)
}

/// Turnover threshold of the NR7 AFLs.
pub(crate) fn volsig6(y: i64) -> f64 {
    match y {
        ..=1999 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

/// Turnover threshold of the Van Tharp / Fresh ST AFLs.
pub(crate) fn volsig17(y: i64) -> f64 {
    match y {
        ..=1999 => 30_000.0,
        2000..=2003 => 50_000.0,
        2004 => 200_000.0,
        2005 => 400_000.0,
        2006 => 600_000.0,
        2007..=2009 => 800_000.0,
        2010 => 1_000_000.0,
        2011 => 2_000_000.0,
        2012 => 2_500_000.0,
        2013 => 3_000_000.0,
        2014 => 3_500_000.0,
        2015..=2016 => 4_000_000.0,
        2017 => 5_000_000.0,
        2018 => 6_000_000.0,
        2019 => 7_000_000.0,
        2020 => 8_000_000.0,
        _ => 9_000_000.0,
    }
}

/// The template's guards with their settings.
pub(crate) struct Guard {
    pub penny: f64,
    pub turn_on: bool,
    pub dead: bool,
    pub table: fn(i64) -> f64,
}

impl Guard {
    pub(crate) fn of(g: &dyn Fn(&str) -> f64, table: fn(i64) -> f64) -> Self {
        Guard { penny: g("penny"), turn_on: g("turnover").round() == 1.0, dead: g("dead_stock").round() == 0.0, table }
    }
    /// deadstockbuysig
    pub(crate) fn dead_buy(&self, x: &Ctx, i: usize) -> bool {
        if self.dead {
            i + 1 != x.c.len()
        } else {
            i != 0
        }
    }
    /// deadstocksellsig
    pub(crate) fn dead_sell(&self, x: &Ctx, i: usize) -> bool {
        self.dead && i + 1 == x.c.len()
    }
    /// Turnover (the AFLs' `V*C` or their `OpenInt`, where the team stored turnover) above the year's threshold.
    pub(crate) fn turnover(&self, x: &Ctx, i: usize) -> bool {
        !self.turn_on || x.r[i].volume.map_or(false, |v| v * x.c[i] > (self.table)(year(x, i)))
    }
}

/// Prec-based upper / lower circuit bars.
pub(crate) fn up_circuit(x: &Ctx, i: usize) -> bool {
    prec2(x.o[i]) == prec2(x.c[i]) && prec2(x.o[i]) == prec2(x.h[i])
}
pub(crate) fn lo_circuit(x: &Ctx, i: usize) -> bool {
    prec2(x.o[i]) == prec2(x.c[i]) && prec2(x.o[i]) == prec2(x.l[i])
}

/// `Ref(LLV(v, n), -1)` with LLV over the bars available near the start; Null on bar 0.
pub(crate) fn prev_llv(v: &[f64], n: usize) -> Vec<Option<f64>> {
    let m = llv(v, n);
    (0..v.len()).map(|i| if i == 0 { None } else { Some(m[i - 1]) }).collect()
}

// ---------------------------------------------------------------- the AFL loop

const NULL: f64 = -1e10;

struct Cfg {
    v: V,
    swing: f64,
    st_period: usize,
    st_factor: f64,
    atr_period: usize,
    atr_mult: f64,
    ll_period: usize,
}

fn nr7_loop(x: &Ctx, k: &Cfg, gd: &Guard) -> Loop {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    let mut s = Loop::new(n);
    if n < 2 {
        return s;
    }
    let v = k.v;
    let regime = if v.athc { athc(c, &close_swing(c, k.swing), false) } else { vec![true; n] };
    let st = matches!(v.exit, Exit::St | Exit::StOrLow2);
    let mid: Vec<f64> = (0..n).map(|i| (h[i] + l[i]) / 2.0).collect();
    let trend = if st { supertrend(&mid, h, l, c, k.st_period, k.st_factor, k.st_factor).0 } else { vec![1; n] };
    let atr = if st { atr_wilder(x, k.atr_period) } else { vec![None; n] };
    let llv_prev = prev_llv(l, k.ll_period);
    let range = |j: usize| h[j] - l[j];
    let inside = |j: usize| j >= 1 && h[j] < h[j - 1] && l[j] > l[j - 1];
    let nr7 = |i: usize| {
        if i < 7 {
            return false;
        }
        let r1 = range(i - 1);
        if (1..=7).any(|k| range(i - k) == 0.0) {
            return false;
        }
        (2..=7).all(|k| r1 < range(i - k)) && regime[i] && (v.setup == Setup::Nr7 || inside(i - 1))
    };
    let (mut abc, mut hi) = (vec![0u8; n], vec![0.0f64; n]);
    let mut pos = false;
    let (mut c1, mut c2, mut sell_cnd) = (false, false, NULL);
    let mut prev2 = vec![0.0f64; n];
    for i in 1..n {
        // setup state
        if nr7(i) {
            hi[i] = if v.setup == Setup::Mother { h[i - 2] } else { h[i - 1] };
            abc[i] = 1;
        } else if (v.athc && !regime[i]) || (!v.athc && pos) {
            abc[i] = 0;
            hi[i] = 0.0;
        } else {
            abc[i] = abc[i - 1];
            hi[i] = hi[i - 1];
        }
        if matches!(v.exit, Exit::Low2 | Exit::StOrLow2) {
            prev2[i] = if pos { l[i - 1].min(if i >= 2 { l[i - 2] } else { NULL }) } else { 0.0 };
        }
        let filt = !up_circuit(x, i) && gd.dead_buy(x, i) && c[i] > gd.penny && gd.turnover(x, i);
        let dead = gd.dead_sell(x, i);
        let loc = lo_circuit(x, i);
        // a breakout a guard blocks drops the setup (not in the plain NR7 AFL)
        if v.athc && !pos && regime[i] && abc[i] == 1 && c[i] > hi[i] {
            let other = !gd.dead_buy(x, i) || c[i] < gd.penny || !gd.turnover(x, i) || up_circuit(x, i);
            if other {
                abc[i] = 0;
            }
        }
        let ready = !pos && regime[i] && abc[i] == 1 && c[i] > hi[i] && filt;
        let mut bought = false;
        if st {
            if ready && trend[i] == 1 {
                (pos, c1, bought) = (true, true, true);
            } else if ready && trend[i] == -1 {
                (pos, c2, bought) = (true, true, true);
                sell_cnd = c[i] - lv(atr[i]) * k.atr_mult;
            }
        } else if ready {
            (pos, bought) = (true, true);
        }
        if bought {
            s.entry[i] = Some(c[i]);
            abc[i] = 0;
            hi[i] = 0.0;
        }
        let mut sell = |why: &'static str, abc: &mut [u8], hi: &mut [f64]| {
            s.exit[i] = Some((c[i], if dead { "Dead stock (last bar)" } else { why }));
            abc[i] = 0;
            hi[i] = 0.0;
        };
        match v.exit {
            Exit::CloseLlv => {
                if !bought && ((pos && c[i] < lv(llv_prev[i]) && !loc) || dead) {
                    sell("Close below N-day low", &mut abc, &mut hi);
                    pos = false;
                }
            }
            Exit::Low2 => {
                if (pos && l[i] < prev2[i] && !loc) || dead {
                    sell("Low below 2-day low", &mut abc, &mut hi);
                    pos = false;
                }
            }
            Exit::St | Exit::StOrLow2 => {
                let low2 = v.exit == Exit::StOrLow2;
                if pos && c2 && trend[i] == 1 {
                    c2 = false;
                    c1 = true;
                    sell_cnd = NULL;
                }
                let brk = low2 && l[i] < prev2[i];
                if (pos && ((c1 && trend[i] == -1) || brk) && !loc) || dead {
                    sell(if c1 && trend[i] == -1 { "Supertrend down" } else { "Low below 2-day low" }, &mut abc, &mut hi);
                    pos = false;
                    c1 = false;
                    if low2 {
                        sell_cnd = NULL;
                    }
                }
                let brk = low2 && l[i] < prev2[i];
                if (pos && ((c2 && c[i] < sell_cnd) || brk) && !loc) || dead {
                    sell(if c2 && c[i] < sell_cnd { "ATR stop" } else { "Low below 2-day low" }, &mut abc, &mut hi);
                    pos = false;
                    c2 = false;
                    sell_cnd = NULL;
                }
            }
        }
    }
    s
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let v = variant(id)?;
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let k = Cfg {
        v,
        swing: g("swing_pct"),
        st_period: g("st_period").round().max(1.0) as usize,
        st_factor: g("st_factor"),
        atr_period: g("atr_period").round().max(1.0) as usize,
        atr_mult: g("atr_mult"),
        ll_period: g("ll_period").round().max(1.0) as usize,
    };
    let gd = Guard::of(&g, volsig6);
    Some(fills(x, &nr7_loop(x, &k, &gd), from, to, "close"))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "NR7"))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::books::ytmr::day_number;
    use crate::Row;

    pub(crate) fn civil(z: i64) -> String {
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
    /// Consecutive weekdays from Monday 2024-01-01; bars are (open, high, low, close), volume 1000 + 10 x bar.
    pub(crate) fn bars(ohlc: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        let mut d = day_number("2024-01-01").unwrap();
        ohlc.iter()
            .enumerate()
            .map(|(k, &(open, high, low, close))| {
                let z = d;
                d += if (z + 4).rem_euclid(7) == 5 { 3 } else { 1 };
                Row { ticker: "T".into(), date: civil(z), open, high, low, close, volume: Some(1000.0 + 10.0 * k as f64) }
            })
            .collect()
    }
    pub(crate) fn wave(n: usize) -> Vec<Row> {
        let v: Vec<(f64, f64, f64, f64)> = (0..n)
            .map(|k| {
                let t = k as f64;
                let c = 300.0 + 60.0 * (t / 23.0).sin() + 15.0 * (t / 4.1).cos() + t * 0.4;
                let o = c - 2.0 * (t / 1.7).sin();
                let w = 1.0 + (k % 7) as f64 * 0.6;
                (o, o.max(c) + w, o.min(c) - w * 0.8, c)
            })
            .collect();
        let mut r = bars(&v);
        for (k, b) in r.iter_mut().enumerate() {
            b.volume = Some(1000.0 + 500.0 * ((k as f64) / 3.0).sin().abs() + (k % 5) as f64 * 200.0);
        }
        r
    }
    pub(crate) fn ps(kv: &[(&str, f64)]) -> Params {
        kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }
    /// Every id of a module runs on a few hundred bars with and without the benchmark series; trades stay ordered,
    /// inside the window and never overlap.
    pub(crate) fn smoke(ids: &[&str], run: fn(&str, &Ctx, usize, usize, &Params) -> Option<Vec<Trade>>, params: fn(&str) -> Option<Vec<ParamDef>>, fam: fn(&str) -> Option<(&'static str, &'static str, &'static str)>) -> usize {
        let r = wave(420);
        let mut total = 0;
        for bench in [false, true] {
            let mut x = Ctx::new(&r, &r, &[]);
            if bench {
                for (k, f) in [("BENCH.Open", 0.98), ("BENCH.High", 1.01), ("BENCH.Low", 0.97), ("BENCH.Close", 1.0)] {
                    x.extra.insert(k.to_string(), r.iter().map(|b| b.close * f).collect());
                }
            }
            for &id in ids {
                assert!(params(id).is_some() && fam(id).is_some(), "{id}");
                let t = run(id, &x, 30, r.len() - 1, &Params::new()).unwrap();
                let mut last = 0;
                for tr in &t {
                    assert!(tr.legs[0].i >= 30 && tr.exit_i >= tr.legs[0].i && tr.exit_i <= r.len() - 1, "{id}");
                    assert!(tr.legs[0].i >= last, "{id} overlap");
                    assert!(tr.exit_px.is_finite() && tr.legs[0].px.is_finite(), "{id}");
                    last = tr.exit_i;
                }
                assert!(run(id, &x, 50, 40, &Params::new()).unwrap().is_empty());
                total += t.len();
            }
        }
        total
    }

    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<(usize, f64, usize, f64, String)> {
        let x = Ctx::new(r, r, &[]);
        run(id, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.reason)).collect()
    }

    /// Bars 0..6 wide (range 4), bar 7 a narrow inside bar (range 1); bar 8 breaks out above it.
    fn base() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..7).map(|k| (100.0 + k as f64, 102.0 + k as f64, 98.0 + k as f64, 101.0 + k as f64)).collect();
        v.push((106.5, 107.0, 106.0, 106.6)); // 7: range 1, inside bar 6 (high 108, low 104)
        v.push((106.8, 108.5, 106.5, 108.2)); // 8: close above 107 -> buy (NR7 high = bar 7's high)
        v
    }

    #[test]
    fn nr7_low2_exit_fills_at_close() {
        let mut v = base();
        v.push((108.5, 110.0, 108.0, 109.5)); // 9: prev2 = min(106.5, 106.0) = 106.0
        v.push((109.0, 109.5, 105.5, 106.0)); // 10: low 105.5 < min(108.0, 106.5) = 106.5 -> sell at close 106
        let t = go(ATHC_LOW2, &bars(&v), &[]);
        assert_eq!(t, vec![(8, 108.2, 10, 106.0, "Low below 2-day low".to_string())]);
        // the inside-bar version needs bar 7 to be inside bar 6: it is, same trade
        assert_eq!(go(INSIDE_LOW2, &bars(&v), &[]).len(), 1);
        // the mother-bar version needs a close above bar 6's high (108): 108.2 > 108
        assert_eq!(go(MOTHER_LOW2, &bars(&v), &[])[0].0, 8);
    }

    #[test]
    fn mother_bar_needs_the_close_above_the_earlier_high() {
        let mut v = base();
        v[8].3 = 107.8; // above 107 (NR7 bar) but not above 108 (mother bar)
        v.push((107.5, 110.0, 107.0, 109.0)); // 9: a later close above 108 with the setup still waiting
        let t = go(MOTHER_LOW2, &bars(&v), &[]);
        assert_eq!(t[0].0, 9);
        assert_eq!(go(ATHC_LOW2, &bars(&v), &[])[0].0, 8);
    }

    #[test]
    fn upper_circuit_breakout_drops_the_setup() {
        let mut v = base();
        v[8] = (108.2, 108.2, 106.5, 108.2); // open = close = high: upper circuit
        v.push((108.0, 109.0, 107.5, 108.8)); // 9: close above 107 again, but the setup was dropped
        assert!(go(ATHC_LOW2, &bars(&v), &[]).is_empty());
        // the plain NR7 AFL has no drop: it buys on bar 9
        assert_eq!(go(NR7_ST, &bars(&v), &[])[0].0, 9);
    }

    #[test]
    fn close_llv_exit_tests_the_close() {
        let mut v = base();
        v.push((108.5, 110.0, 108.0, 109.5)); // 9
        v.push((109.0, 109.5, 105.5, 106.8)); // 10: low < 106.5 but close 106.8 >= min(108.0, 106.5)
        v.push((106.0, 106.5, 105.0, 105.2)); // 11: close 105.2 < min(108.0, 105.5) = 105.5 -> sell
        let t = go(ATHC_LLV, &bars(&v), &[]);
        assert_eq!(t, vec![(8, 108.2, 11, 105.2, "Close below N-day low".to_string())]);
    }

    #[test]
    fn st_exit_and_atr_stop() {
        // a long rise keeps the ST up; the NR7 breakout buys in the up trend, then a fall turns the ST down
        let mut v: Vec<(f64, f64, f64, f64)> = (0..20).map(|k| (100.0 + 2.0 * k as f64, 103.0 + 2.0 * k as f64, 99.0 + 2.0 * k as f64, 102.0 + 2.0 * k as f64)).collect();
        v.push((140.0, 140.5, 139.8, 140.2)); // 20: narrow bar
        v.push((140.5, 142.5, 140.0, 142.0)); // 21: close above 140.5 -> buy
        v.push((141.0, 141.5, 125.0, 126.0)); // 22: a drop through the ST line
        let r = bars(&v);
        let t = go(ATHC_ST, &r, &[]);
        assert_eq!((t[0].0, t[0].2, t[0].4.as_str()), (21, 22, "Supertrend down"));
        // entered in an ST down trend: the ATR stop (close - ATR(14)) applies until the ST turns up
        let mut w: Vec<(f64, f64, f64, f64)> = (0..20).map(|k| (200.0 - 2.0 * k as f64, 203.0 - 2.0 * k as f64, 199.0 - 2.0 * k as f64, 201.0 - 2.0 * k as f64)).collect();
        w.push((161.5, 162.0, 161.4, 161.8)); // 20: narrow bar
        w.push((162.0, 163.0, 161.0, 162.5)); // 21: close above 162 -> buy, stop = 162.5 - ATR(14)
        w.push((162.0, 162.5, 157.0, 157.5)); // 22: close below the stop
        let t = go(NR7_ST, &bars(&w), &[]);
        assert_eq!((t[0].0, t[0].2, t[0].4.as_str()), (21, 22, "ATR stop"));
    }

    #[test]
    fn athc_regime_carries_and_turns_off_on_the_swing() {
        let c = [100.0, 101.0, 100.5, 102.0, 90.0, 95.0, 103.0];
        let tr = close_swing(&c, 10.0);
        assert_eq!(athc(&c, &tr, false), vec![false, true, true, true, false, false, true]);
    }

    #[test]
    fn every_strategy_runs() {
        assert!(smoke(&IDS, run, params, family_of) > 0);
        for id in IDS {
            let d = catalogue().into_iter().find(|d| d.id == id).unwrap();
            assert!(d.notes.contains("coded as the AFL is written") && d.notes.contains("one stock at a time"), "{id}");
        }
    }
}
