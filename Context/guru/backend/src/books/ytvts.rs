//! YT Strategies · Van Tharp (stocks): the team's AmiBroker AFLs in "Old Topics/Van Tharp" other than the Index
//! "Channeling" ones (coded in `ytvt.rs`). Each AFL trades one stock at a time on its own daily bars (the team ran
//! them over NSE cash stock universes with MaxOpenPositions 20; portfolio limits and sizing are not modelled). Coded
//! from the AFLs as written, with each AFL's default settings (an `Optimize()` uses its default); there is no book
//! PDF and nothing here is compared with the team's workbooks.
//!
//! AmiBroker behaviour the code follows (see `ytnr7.rs` / `ytd7.rs` for the shared conventions): the AFL loop runs
//! over the stock's full history, entries are taken only in the test window, one position at a time, exit before
//! entry on a bar; inside a loop a Null element is -1e10 (so a trailing stop built on a still-Null ATR is huge, and
//! `C[i] > MA[i]` is true while the average is Null); HHV / LLV use the bars available near the start; the ST filter
//! is the team's ST loop on the filter timeframe expanded with expandLast; trade prices are kept inside the bar's
//! range; the template guards (high = close blocks an entry, Prec circuits, penny 0, turnover off, dead stock off)
//! are kept as written.
#![allow(dead_code)]
use crate::books::ytd7::lv;
use crate::books::ytnr7::{athc, fills, lo_circuit, pd, prev_llv, template_params, up_circuit, volsig17, Guard, Loop};
use crate::books::ytroc::atr_wilder;
use crate::books::ytsw::{close_swing, sma, st_filter, tf_of};
use crate::books::ytwh::{hhv, llv};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_vts";
pub const FIVE_DOWN: &str = "ytvts_five_days_down";
pub const FIVE_FIVE_ONE: &str = "ytvts_five_five_one_w";
pub const TWO_MA: &str = "ytvts_two_ma_gap";
pub const TWO_MA_FLT: &str = "ytvts_two_ma_gap_filter";
pub const WASHOUT: &str = "ytvts_washout";
pub const IDS: [&str; 5] = [FIVE_DOWN, FIVE_FIVE_ONE, TWO_MA, TWO_MA_FLT, WASHOUT];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Van Tharp (stocks)", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const X_HSL: &str = "X|Hard stop for the first HSL days (2): the low reaching the setup low sells there (at the open less the 0.25% buffer if it gapped below)";
const X_TRAIL: &str = "X|After that: an ATR trailing stop (highest high since entry less ATR multiple x ATR(5) of the previous bar, taken on the bar of each new high; it only rises) and a low below the lowest low of the previous 2 bars; each sells at its level, or the open less the buffer on a gap";
const X_CIRC: &str = "X|A stop on a lower-circuit bar (open, low and close equal at 2 decimals) is deferred: sold at the next non-circuit bar's open less the buffer";
const F_FLT: &str = "F|Filter (default off): ST filter (the stock's own Supertrend, monthly, period 5, factor 1, mid point) up to buy and down to sell at the close; or the ATHC regime (close above all earlier closes, off when the 10% close swing turns down) on to buy and off to sell";
const F_GUARD: &str = "F|Template guards: no entry on a bar whose high equals its close, close above the penny price (0), turnover filter off, dead-stock backtest off";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(FIVE_DOWN, "Five Days Down Swing", &[
            "F|Setup: 5 consecutive lower closes ending on the previous bar",
            "E|Buy at the close on the next bar when the close is above the setup bar's high less 10% of its range; the setup bar's low is the hard stop",
            X_HSL, X_TRAIL, X_CIRC, F_FLT, F_GUARD,
        ], "Five Days Down Swing trading Strategy with and without filter(Long Only).afl, coded as the AFL is written; the plain 'Five Days Down Swing trading Strategy.afl' has the same rules and defaults as its 'Without Filter' setting (its loop starts at the lowest-low-period bar instead of bar 2) and is not coded twice. Runs on one stock at a time; the team ran it over NSE cash stock universes; portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled. The lower-close count keeps running while a trade is open; a new entry needs the count to equal 5 + 1 exactly. The filter-off default is the AFL's."),
        def(FIVE_FIVE_ONE, "Five Five One W Swing", &[
            "F|Setup bar: close above the close 25 bars back, previous close below the close 5 bars back, an up close and a white candle, Williams %R(10) below -50",
            "E|Buy at the close on the bar after the setup when the close is above the setup bar's high less 10% of its range; the setup bar's low is the hard stop",
            X_HSL, "X|After that: ATR trailing stop (1 x ATR(5), from each new high, only rising) and a low below the lowest low of the previous 2 bars", X_CIRC, F_FLT, F_GUARD,
        ], "Five Five One W Swing Trading System with and without filter(Long Only).afl, coded as the AFL is written; the plain 'Five Five One W Swing Trading System.afl' has the same rules and defaults as the 'Without Filter' setting and is not coded twice. Runs on one stock at a time; the team ran it over NSE cash stock universes; portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled."),
        def(TWO_MA, "Two MAs with Gap (no filter)", &[
            "E|Buy at the close when the close is above the 200-day average, the high is below the 10-day average and the close is below the 10-day average less 1% (gap version 'Simple %')",
            "X|Sell at the close when the previous close was above the 10-day average or the close is below the 200-day average, on a bar whose low is not its close",
            F_GUARD,
        ], "Two Ma's with gap value EntryExit_Without filter.afl, coded as the AFL is written. Runs on one stock at a time; the team ran it over NSE cash stock universes; portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled. Gap versions: 1 Simple % (MA x gap %), 2 Simple ATR (MA - ATR(5) of the previous bar), 3 ATR % (MA - multiplier x ATR). While the 200-day average is still Null the AFL's loop treats it as -1e10, so the 'above the 200-day average' test passes on a stock's first 199 bars (as written)."),
        def(TWO_MA_FLT, "Two MAs with Gap (with filter option)", &[
            "E|Buy at the close when the close is above the 200-day average, the high is below the 10-day average and the close is below the 10-day average less 1%",
            "X|Sell at the close when the previous close was above the 10-day average or the close is below the 200-day average (or the filter turns off); on a bar whose low equals its close the sale waits for the first later bar where it does not",
            F_FLT, F_GUARD,
        ], "Two Ma's with gap value EntryExit with and without filter(Long Only).afl, coded as the AFL is written (default 'Without Filter'). Runs on one stock at a time; the team ran it over NSE cash stock universes; portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled. Differs from the no-filter AFL in how a low = close bar defers the exit."),
        def(WASHOUT, "Washout Swing", &[
            "F|Setup bar (the previous bar): higher low, white candle, close above the high before it; Williams %R(260) and %R(100) of the bar before the setup below -80",
            "E|Buy stop at the setup bar's high + 0.05% (at the open + 0.25% buffer when the open is already above it), only when the high is outside a band of 2% around the 200-day average; the bar before the setup gives the hard stop (its low)",
            X_HSL, X_TRAIL, X_CIRC,
            "F|Template guards: no entry on an upper-circuit bar (open, close and high equal at 2 decimals), previous close above the penny price (0), turnover filter off, dead-stock backtest off",
        ], "WASHOUT Swing trading Strategy.afl, coded as the AFL is written. Runs on one stock at a time; the team ran it over NSE cash stock universes; portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled. The entry is intraday at the stop level (a buy stop), not at the close. While the 200-day average is Null the AFL's loop treats the band as -1e10, so the band test passes on a stock's first 199 bars (as written)."),
    ]
}

fn filter_params(out: &mut Vec<ParamDef>) {
    out.push(pd("filter", "Filter (0 without filter, 1 ST filter, 2 ATHC filter)", 0.0, 0.0, 2.0, 1.0));
    out.push(pd("flt_tf", "ST filter timeframe (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0));
    out.push(pd("flt_period", "ST filter period", 5.0, 1.0, 20.0, 1.0));
    out.push(pd("flt_factor", "ST filter factor", 1.0, 0.0, 20.0, 1.0));
    out.push(pd("flt_point", "ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0));
    out.push(pd("swing_pct", "ATHC swing exit %", 10.0, 1.0, 35.0, 1.0));
}

fn stop_params(out: &mut Vec<ParamDef>, mult: f64) {
    out.push(pd("hsl_days", "HSL days", 2.0, 1.0, 100.0, 1.0));
    out.push(pd("buffer_pct", "Buffer %", 0.25, 0.0, 5.0, 0.01));
    out.push(pd("ll_period", "Lowest low period", 2.0, 1.0, 500.0, 1.0));
    out.push(pd("atr_period", "ATR periods", 5.0, 1.0, 100.0, 1.0));
    out.push(pd("atr_mult", "ATR trail SL multiplier (0 off)", mult, 0.0, 20.0, 1.0));
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let mut out = Vec::new();
    match id {
        FIVE_DOWN => {
            out.push(pd("lower_closes", "Entry lower close period", 5.0, 2.0, 10.0, 1.0));
            out.push(pd("entry_pct", "Entry % of the setup range below its high", 10.0, 0.0, 12.0, 1.0));
            stop_params(&mut out, 5.0);
            filter_params(&mut out);
        }
        FIVE_FIVE_ONE => {
            out.push(pd("wr_period", "Williams %R period", 10.0, 1.0, 500.0, 1.0));
            out.push(pd("ob", "OB value (%R below minus this)", 50.0, 1.0, 100.0, 1.0));
            out.push(pd("entry_pct", "Entry % of the setup range below its high", 10.0, 0.0, 10.0, 0.01));
            out.push(pd("dc_period", "'5 weeks ago' close period", 25.0, 1.0, 500.0, 1.0));
            out.push(pd("dc2_period", "'5 days ago' close period", 5.0, 1.0, 500.0, 1.0));
            stop_params(&mut out, 1.0);
            filter_params(&mut out);
        }
        TWO_MA | TWO_MA_FLT => {
            out.push(pd("gap_ver", "Fast MA gap version (1 simple %, 2 simple ATR, 3 ATR %)", 1.0, 1.0, 3.0, 1.0));
            out.push(pd("gap_pct", "Fast MA gap %", 1.0, 0.0, 10.0, 0.001));
            out.push(pd("slow", "Slow MA period", 200.0, 1.0, 400.0, 1.0));
            out.push(pd("fast", "Fast MA period", 10.0, 1.0, 100.0, 1.0));
            out.push(pd("atr_period", "ATR periods", 5.0, 1.0, 100.0, 1.0));
            out.push(pd("atr_mult", "ATR multiplier (gap version 3)", 1.0, 0.0, 20.0, 0.5));
            if id == TWO_MA_FLT {
                filter_params(&mut out);
            }
        }
        WASHOUT => {
            out.push(pd("annual_wr", "Annual Williams %R period", 260.0, 1.0, 500.0, 1.0));
            out.push(pd("wr_period", "Short-term Williams %R period", 100.0, 1.0, 500.0, 1.0));
            out.push(pd("ob", "OB value (%R below minus this)", 80.0, 1.0, 100.0, 1.0));
            out.push(pd("entry_pct", "Entry % above the setup high", 0.05, 0.0, 10.0, 0.01));
            stop_params(&mut out, 5.0);
            out.push(pd("ma_period", "Entry MA period", 200.0, 1.0, 500.0, 1.0));
            out.push(pd("ma_pct", "MA band %", 2.0, 0.0, 50.0, 0.01));
        }
        _ => return None,
    }
    out.extend(template_params());
    Some(out)
}

const BIG: f64 = 9_999_999.0;

/// Filter buy / filter exit per bar: (MTREND up or ATHC on, MTREND down or ATHC off); filter 0 = (true, false).
fn filter(x: &Ctx, g: &dyn Fn(&str) -> f64) -> (Vec<bool>, Vec<bool>) {
    let n = x.c.len();
    match g("filter").round() as i64 {
        1 => {
            let tr = st_filter(x, tf_of(g("flt_tf").round() as i64, true), g("flt_point").round() as i64, g("flt_period").round().max(1.0) as usize, g("flt_factor"));
            (tr.iter().map(|&t| t == 1).collect(), tr.iter().map(|&t| t == -1).collect())
        }
        2 => {
            let a = athc(&x.c, &close_swing(&x.c, g("swing_pct")), false);
            (a.clone(), a.iter().map(|&v| !v).collect())
        }
        _ => (vec![true; n], vec![false; n]),
    }
}

/// The shared position block of the HSL / ATR-trail AFLs.
struct Hold {
    hsl_days: i64,
    buf: f64,
    mult: f64,
    my_atr: Vec<Option<f64>>,
    ll: Vec<Option<f64>>,
    trail: Vec<f64>,
    pos: bool,
    skip: bool,
    count: i64,
    hsl: f64,
    hh: f64,
    atr_value: f64,
}

impl Hold {
    fn new(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Self {
        Hold {
            hsl_days: g("hsl_days").round() as i64,
            buf: g("buffer_pct"),
            mult: g("atr_mult"),
            my_atr: {
                let a = atr_wilder(x, g("atr_period").round().max(1.0) as usize);
                (0..a.len()).map(|i| if i == 0 { None } else { a[i - 1] }).collect()
            },
            ll: prev_llv(&x.l, g("ll_period").round().max(1.0) as usize),
            trail: vec![0.0; x.c.len()],
            pos: false,
            skip: false,
            count: 0,
            hsl: 0.0,
            hh: BIG,
            atr_value: 0.0,
        }
    }
    fn enter(&mut self, x: &Ctx, i: usize) {
        if self.mult > 0.0 {
            self.hh = x.h[i];
            self.trail[i] = self.hh - lv(self.my_atr[i]) * self.mult;
        }
        self.pos = true;
    }
    fn reset(&mut self, i: usize) {
        self.pos = false;
        self.skip = false;
        self.hsl = 0.0;
        self.trail[i] = 0.0;
        self.atr_value = 0.0;
        self.count = 0;
        self.hh = BIG;
    }
    /// `else if (Position > 0 AND SkipSell == 0)` and `else if (SkipSell == 1 ...)`; `flt_exit` is the filter
    /// versions' extra exit (sold at the close unless the low equals the close).
    fn step(&mut self, x: &Ctx, i: usize, dead: bool, flt_exit: bool) -> Option<(f64, &'static str)> {
        let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        let loc = lo_circuit(x, i);
        let buf = self.buf;
        let gap = |lvl: f64| if o < lvl { o - o * buf / 100.0 } else { lvl };
        if self.pos && !self.skip {
            self.count += 1;
            if self.mult > 0.0 {
                self.hh = h.max(self.hh);
                if h == self.hh {
                    self.atr_value = lv(self.my_atr[i]);
                }
            }
            if self.count > self.hsl_days {
                self.hsl = 0.0;
            }
            let prev = self.trail[i - 1];
            let ll = lv(self.ll[i]);
            let mut out = None;
            let mut fire = |ok: bool, px: f64, why: &'static str, me: &mut Self| {
                if ok || dead {
                    out = Some((px, if dead { "Dead stock (last bar)" } else { why }));
                    me.reset(i);
                } else {
                    me.skip = true;
                }
            };
            if (l <= self.hsl && self.count <= self.hsl_days) || dead {
                fire(!loc, gap(self.hsl), "Hard SL", self);
            } else if self.count > self.hsl_days && l <= prev {
                fire(!loc, gap(prev), "ATR trailing SL", self);
            } else if self.count > self.hsl_days && l < ll {
                fire(!loc, gap(ll), "Low below previous lows", self);
            } else if flt_exit {
                fire(l != c, c, "Filter exit", self);
            }
            if self.mult > 0.0 {
                self.trail[i] = if self.atr_value > 0.0 { self.hh - self.atr_value * self.mult } else { prev };
                if self.trail[i] < prev {
                    self.trail[i] = prev;
                }
            }
            return out;
        }
        if self.skip && (!loc || dead) {
            let px = o - o * buf / 100.0;
            self.reset(i);
            return Some((px, "Deferred stop (after lower circuit)"));
        }
        None
    }
}

fn williams_r(x: &Ctx, n: usize) -> Vec<f64> {
    let (hh, ll) = (hhv(&x.h, n), llv(&x.l, n));
    (0..x.c.len()).map(|i| (hh[i] - x.c[i]) / (hh[i] - ll[i]) * -100.0).collect()
}

fn five_down(x: &Ctx, g: &dyn Fn(&str) -> f64, gd: &Guard) -> Loop {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    let mut s = Loop::new(n);
    let (fb, fx) = filter(x, g);
    let mut k = Hold::new(x, g);
    let p1 = g("lower_closes").round() as i64 + 1;
    let pct = g("entry_pct");
    let (mut lower, mut count1, mut ev1) = (-1e10, 0i64, -1e10);
    for i in 2..n {
        if !k.pos && count1 == 0 {
            lower = c[i - 2];
            count1 = 1;
        }
        if c[i - 1] < lower && count1 > 0 {
            lower = c[i - 1];
            count1 += 1;
        } else if c[i - 1] > lower {
            count1 = 1;
            lower = c[i - 1];
        }
        if !k.pos && count1 >= p1 {
            ev1 = h[i - 1] - (h[i - 1] - l[i - 1]) * pct / 100.0;
            k.hsl = l[i - 1];
        }
        let buy_ok = h[i] != c[i] && c[i] > gd.penny && gd.turnover(x, i) && gd.dead_buy(x, i);
        if !k.pos && count1 == p1 && c[i] > ev1 && fb[i] && buy_ok {
            s.entry[i] = Some(c[i]);
            k.enter(x, i);
        } else {
            s.exit[i] = k.step(x, i, gd.dead_sell(x, i), fx[i]);
        }
    }
    s
}

fn five_five_one(x: &Ctx, g: &dyn Fn(&str) -> f64, gd: &Guard) -> Loop {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let mut s = Loop::new(n);
    let (fb, fx) = filter(x, g);
    let mut k = Hold::new(x, g);
    let wr = williams_r(x, g("wr_period").round().max(1.0) as usize);
    let ob = g("ob");
    let (dc, dc2) = (g("dc_period").round().max(1.0) as usize, g("dc2_period").round().max(1.0) as usize);
    let pct = g("entry_pct");
    let setup: Vec<bool> = (0..n).map(|i| i >= dc && i >= dc2 && c[i] > c[i - dc] && c[i - 1] < c[i - dc2] && c[i] > c[i - 1] && c[i] > o[i] && wr[i] < -ob).collect();
    let mut ev1 = -1e10;
    for i in 1..n {
        if setup[i - 1] && !k.pos {
            ev1 = h[i - 1] - (h[i - 1] - l[i - 1]) * pct / 100.0;
        }
        let buy_ok = h[i] != c[i] && c[i] > gd.penny && gd.turnover(x, i) && gd.dead_buy(x, i);
        if !k.pos && setup[i - 1] && c[i] > ev1 && fb[i] && buy_ok {
            s.entry[i] = Some(c[i]);
            k.enter(x, i);
            k.hsl = l[i - 1];
        } else {
            s.exit[i] = k.step(x, i, gd.dead_sell(x, i), fx[i]);
        }
    }
    s
}

fn washout(x: &Ctx, g: &dyn Fn(&str) -> f64, gd: &Guard) -> Loop {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let mut s = Loop::new(n);
    let mut k = Hold::new(x, g);
    let awr = williams_r(x, g("annual_wr").round().max(1.0) as usize);
    let wr = williams_r(x, g("wr_period").round().max(1.0) as usize);
    let ob = g("ob");
    let (pct, buf) = (g("entry_pct"), g("buffer_pct"));
    let ma = sma(c, g("ma_period").round().max(1.0) as usize);
    let mp = g("ma_pct");
    let start = (g("ll_period").round().max(1.0) as usize).max(2);
    for i in start..n {
        let lvl = h[i - 1] + h[i - 1] * pct / 100.0;
        let (ub, lb) = (ma[i].map(|m| m + m * mp / 100.0), ma[i].map(|m| m - m * mp / 100.0));
        let buy_ok = !up_circuit(x, i) && c[i - 1] > gd.penny && gd.turnover(x, i) && gd.dead_buy(x, i);
        if !k.pos && l[i - 1] > l[i - 2] && c[i - 1] > o[i - 1] && c[i - 1] > h[i - 2] && awr[i - 2] < -ob && buy_ok && wr[i - 2] < -ob && h[i] > lvl && (h[i] > lv(ub) || h[i] < lv(lb)) {
            s.entry[i] = Some(if o[i] > lvl { o[i] + o[i] * buf / 100.0 } else { lvl });
            k.enter(x, i);
            k.hsl = l[i - 2];
        } else {
            s.exit[i] = k.step(x, i, gd.dead_sell(x, i), false);
        }
    }
    s
}

fn two_ma(x: &Ctx, g: &dyn Fn(&str) -> f64, gd: &Guard, with_filter: bool) -> Loop {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    let mut s = Loop::new(n);
    let (fb, fx) = if with_filter { filter(x, g) } else { (vec![true; n], vec![false; n]) };
    let slow = sma(c, g("slow").round().max(1.0) as usize);
    let fast = sma(c, g("fast").round().max(1.0) as usize);
    let a = atr_wilder(x, g("atr_period").round().max(1.0) as usize);
    let my_atr: Vec<Option<f64>> = (0..n).map(|i| if i == 0 { None } else { a[i - 1] }).collect();
    let (gp, m) = (g("gap_pct"), g("atr_mult"));
    let gap: Vec<Option<f64>> = (0..n)
        .map(|i| match g("gap_ver").round() as i64 {
            2 => fast[i].zip(my_atr[i]).map(|(f, a)| f - a),
            3 => fast[i].zip(my_atr[i]).map(|(f, a)| f - a * m),
            _ => fast[i].map(|f| f - f * gp / 100.0),
        })
        .collect();
    let (mut pos, mut skip) = (false, false);
    for i in 1..n {
        let dead = gd.dead_sell(x, i);
        let lc = l[i] == c[i];
        let buy_ok = h[i] != c[i] && c[i] > gd.penny && gd.turnover(x, i) && gd.dead_buy(x, i);
        let why = if dead { "Dead stock (last bar)" } else { "MA exit" };
        let rule = c[i - 1] > lv(fast[i - 1]) || c[i] < lv(slow[i]);
        if !pos && c[i] > lv(slow[i]) && h[i] < lv(fast[i]) && c[i] < lv(gap[i]) && fb[i] && buy_ok {
            s.entry[i] = Some(c[i]);
            pos = true;
        } else if !with_filter {
            if pos && ((rule && !lc) || dead) {
                s.exit[i] = Some((c[i], why));
                pos = false;
            }
        } else if pos && !skip && (rule || fx[i] || dead) {
            if !lc || dead {
                s.exit[i] = Some((c[i], if !dead && !rule { "Filter exit" } else { why }));
                pos = false;
            } else {
                skip = true;
            }
        } else if skip && (!lc || dead) {
            s.exit[i] = Some((c[i], "Deferred MA exit (low = close)"));
            pos = false;
            skip = false;
        }
    }
    s
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n < 3 || from > to || to >= n {
        return Some(Vec::new());
    }
    let gd = Guard::of(&g, volsig17);
    let (s, kind) = match id {
        FIVE_DOWN => (five_down(x, &g, &gd), "close"),
        FIVE_FIVE_ONE => (five_five_one(x, &g, &gd), "close"),
        TWO_MA => (two_ma(x, &g, &gd, false), "close"),
        TWO_MA_FLT => (two_ma(x, &g, &gd, true), "close"),
        WASHOUT => (washout(x, &g, &gd), "stop"),
        _ => return None,
    };
    Some(fills(x, &s, from, to, kind))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Van Tharp"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytnr7::tests::{bars, ps, smoke};
    use crate::Row;

    fn r6(v: f64) -> f64 {
        (v * 1e6).round() / 1e6
    }
    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<(usize, f64, usize, f64, String)> {
        let x = Ctx::new(r, r, &[]);
        run(id, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, (t.legs[0].px * 1e6).round() / 1e6, t.exit_i, (t.exit_px * 1e6).round() / 1e6, t.reason)).collect()
    }

    /// Bars 0..1 flat, then 5 lower closes (bars 2..6), bar 7 closes above bar 6's high less 10% of its range.
    fn five_down_bars() -> Vec<(f64, f64, f64, f64)> {
        let mut v = vec![(100.0, 101.0, 99.0, 100.0), (100.0, 101.0, 99.0, 100.5)];
        for k in 0..5 {
            let c = 99.0 - k as f64;
            v.push((c + 0.5, c + 1.0, c - 1.0, c));
        }
        // bar 6: (95.5, 96, 94, 95): entry level 96 - 0.2 = 95.8, hard stop 94
        v.push((95.0, 96.5, 94.5, 96.0)); // 7: close 96 > 95.8 -> buy
        v
    }

    #[test]
    fn five_days_down_buys_and_hard_stop() {
        let mut v = five_down_bars();
        v.push((95.0, 95.5, 93.5, 94.0)); // 8: count 1, low 93.5 <= 94 -> sell at 94 (open 95 above it)
        assert_eq!(go(FIVE_DOWN, &bars(&v), &[]), vec![(7, 96.0, 8, 94.0, "Hard SL".to_string())]);
        // a gap below the stop sells at the open less the buffer
        let mut w = five_down_bars();
        w.push((93.0, 93.5, 92.0, 93.2));
        assert_eq!(go(FIVE_DOWN, &bars(&w), &[])[0].3, r6(93.0 - 93.0 * 0.0025));
    }

    #[test]
    fn five_days_down_lower_circuit_defers_the_stop() {
        let mut v = five_down_bars();
        v.push((93.5, 94.2, 93.5, 93.5)); // 8: low below 94 but open = low = close (lower circuit): skip
        v.push((93.8, 94.5, 93.0, 94.0)); // 9: sold at the open less the buffer
        assert_eq!(go(FIVE_DOWN, &bars(&v), &[]), vec![(7, 96.0, 9, r6(93.8 - 93.8 * 0.0025), "Deferred stop (after lower circuit)".to_string())]);
    }

    #[test]
    fn five_days_down_trailing_low_exit_after_hsl_days() {
        let mut v = five_down_bars();
        v.push((96.0, 97.0, 95.5, 96.8)); // 8
        v.push((96.8, 97.5, 96.0, 97.2)); // 9
        v.push((97.0, 97.8, 96.2, 97.5)); // 10: count 3 > 2: low 96.2 not below min(95.5, 96.0)... 95.5 -> stays
        v.push((97.0, 97.2, 95.0, 95.4)); // 11: low 95 < min(96.0, 96.2) = 96.0 -> sell at 96 (open 97 above)
        let t = go(FIVE_DOWN, &bars(&v), &[("atr_mult", 0.0)]);
        assert_eq!(t, vec![(7, 96.0, 11, 96.0, "Low below previous lows".to_string())]);
    }

    #[test]
    fn two_ma_needs_a_pullback_below_the_fast_average() {
        // rising closes, then a dip whose high is under the 10-day average and close 1% below it
        let mut v: Vec<(f64, f64, f64, f64)> = (0..30).map(|k| (100.0 + k as f64, 101.0 + k as f64, 99.5 + k as f64, 100.5 + k as f64)).collect();
        v.push((120.0, 121.0, 118.0, 119.0)); // 30: 10-day average 124.85: high below it, close below it less 1%; 30-day 115.6
        v.push((119.0, 128.0, 118.5, 127.0)); // 31: previous close 119 below the average: hold
        v.push((127.0, 129.0, 126.0, 128.5)); // 32: previous close 127 above the average -> sell at the close
        let r = bars(&v);
        let t = go(TWO_MA, &r, &[("slow", 30.0)]);
        assert_eq!((t[0].0, t[0].2), (30, 32));
        // the filter AFL with a low = close bar defers the sale
        let mut w = v.clone();
        w[32] = (127.0, 129.0, 126.0, 126.0);
        w.push((126.5, 128.0, 126.0, 127.5));
        let t = go(TWO_MA_FLT, &bars(&w), &[("slow", 30.0)]);
        assert_eq!((t[0].0, t[0].2, t[0].3), (30, 33, 127.5));
        // the no-filter AFL re-tests the rule on bar 33 instead (previous close 126 above the 125.75 average)
        let t = go(TWO_MA, &bars(&w), &[("slow", 30.0)]);
        assert_eq!((t[0].0, t[0].2), (30, 33));
    }

    #[test]
    fn washout_buys_at_the_stop_level() {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..30).map(|k| (200.0 - 3.0 * k as f64, 202.0 - 3.0 * k as f64, 197.0 - 3.0 * k as f64, 199.0 - 3.0 * k as f64)).collect();
        // bar 30: setup bar: higher low, white candle, close above bar 29's high (115)
        v.push((113.0, 117.0, 112.0, 116.0));
        v.push((116.5, 118.0, 115.0, 117.5)); // 31: high 118 > 117 x 1.0005 = 117.0585 -> buy there
        let t = go(WASHOUT, &bars(&v), &[("annual_wr", 20.0), ("wr_period", 10.0), ("ma_period", 20.0)]);
        assert_eq!((t[0].0, t[0].1), (31, 117.0585));
        // the open above the level buys at the open + 0.25%
        v[31] = (117.5, 119.0, 117.2, 118.5);
        let t = go(WASHOUT, &bars(&v), &[("annual_wr", 20.0), ("wr_period", 10.0), ("ma_period", 20.0)]);
        assert_eq!((t[0].0, t[0].1), (31, r6(117.5 * 1.0025)));
    }

    #[test]
    fn every_strategy_runs() {
        assert!(smoke(&IDS, run, params, family_of) > 0);
        let r = crate::books::ytnr7::tests::wave(420);
        let x = Ctx::new(&r, &r, &[]);
        for id in [FIVE_DOWN, FIVE_FIVE_ONE, TWO_MA_FLT] {
            for f in [1.0, 2.0] {
                for tf in [1.0, 2.0, 3.0] {
                    let t = run(id, &x, 30, r.len() - 1, &ps(&[("filter", f), ("flt_tf", tf)])).unwrap();
                    assert!(t.iter().all(|t| t.legs[0].i >= 30 && t.exit_i >= t.legs[0].i), "{id}");
                }
            }
        }
        for id in IDS {
            let d = catalogue().into_iter().find(|d| d.id == id).unwrap();
            assert!(d.notes.contains("coded as the AFL is written") && d.notes.contains("one stock at a time"), "{id}");
        }
    }
}
