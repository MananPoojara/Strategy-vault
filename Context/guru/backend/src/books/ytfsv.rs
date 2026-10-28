//! YT Strategies · Fresh ST Buy (volume): the team's AmiBroker AFLs in "Old Topics/Fresh ST Buy Next Day High-Close"
//! that need the stock's volume (Volume > MA(Volume)) and RSI; the volume-free Basic Version is in `ytfst.rs`. Each
//! AFL trades one stock at a time on its own daily bars (the team ran them over NSE cash stock universes, and Nifty
//! spot; MaxOpenPositions 20 and sizing are not modelled). Coded from the AFLs as written, with each AFL's default
//! settings (an `Optimize()` uses its default); there is no book PDF and nothing here is compared with the team's
//! workbooks.
//!
//! AmiBroker behaviour the code follows (see `ytnr7.rs`, `ytfst.rs`, `ytsw.rs`): the entry Supertrend is the team's
//! ST loop on the AFL's timeframe expanded with expandLast; the AFL loop runs from bar 1 over the full history and
//! entries are taken only in the test window; inside the loop a Null element is -1e10 (so `Volume > MA(Volume)`
//! passes while the average is Null, `RSI > 50` fails while RSI is Null, and `C > TrendUp` passes on bars where an ST
//! filter line is Null); `Cross()` needs both arrays on the bar and the bar before; a sell price the AFL does not set
//! is the close; trade prices are kept inside the bar's range; the template guards (high = close blocks an entry,
//! penny 0, turnover off - the team's turnover sat in OpenInt, here close x volume -, dead stock off) as written.
#![allow(dead_code)]
use crate::books::ytd7::lv;
use crate::books::ytnr7::{fills, lo_circuit, pd, template_params, volsig17, Guard, Loop};
use crate::books::ytroc::{cross, rsi_opt};
use crate::books::ytsw::{calc_point, compress, expand_last, sma, st_filter, tf_of, true_range};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_fsv";
pub const CLOSE: &str = "ytfsv_close";
pub const HIGH: &str = "ytfsv_high";
pub const CLOSE_FLT: &str = "ytfsv_close_filters";
pub const RSI_CROSS: &str = "ytfsv_rsi_cross";
pub const RSI_RUNNING: &str = "ytfsv_rsi_running";
pub const RSI_CROSS_D1: &str = "ytfsv_rsi_cross_draft";
pub const RSI_CROSS_V2: &str = "ytfsv_rsi_cross_v2";
pub const RSI_CROSS_V3: &str = "ytfsv_rsi_cross_v3";
pub const RSI_CROSS_V3N: &str = "ytfsv_rsi_cross_v3_new";
pub const IDS: [&str; 9] = [CLOSE, HIGH, CLOSE_FLT, RSI_CROSS, RSI_RUNNING, RSI_CROSS_D1, RSI_CROSS_V2, RSI_CROSS_V3, RSI_CROSS_V3N];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Fresh ST Buy (volume)", session: 0, dir: "Long", status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const F_FRESH: &str = "F|Fresh ST: on the bar the stock's Supertrend (period 5, factor 4, mid point) turns up while flat, that bar's high becomes the setup high (count 0); each further up bar adds 1; a down bar (after at least one count) cancels the setup";
const F_FRESH_H: &str = "F|Fresh ST: on the bar the stock's Supertrend (period 5, factor 4, mid point) turns up while flat, that bar's high becomes the setup high (count 0); each further up bar adds 1; a down bar cancels the setup";
const E_CLOSE: &str = "E|Buy at the close when the close is above the setup high, the count is at most 2, volume is above its average and RSI(14) is above 50";
const X_STOPS: &str = "X|Trailing stop (highest high since entry less 3%, the stop of the previous bar) at its level or the open less the 0.25% buffer on a gap; profit target 3% above the entry at its level (the open less the buffer when the open is already above it); no exit of these on a bar whose low equals its close";
const X_ST: &str = "X|Otherwise sell at the close when the Supertrend is down, unless the low equals the close";
const F_AGAIN: &str = "F|After an exit a new setup needs the ST to turn down and up again";
const F_FLT: &str = "F|Filter (default off): ATHC (close above all earlier closes, off when the 20% close swing turns down), ST (monthly ST 5 / 4 of the stock: buy above its up line), or a 44% / 23% % swing; while on it also gates entries and, where the AFL has it, sells when the close falls below the filter's down line";
const RSI_TRAIL: &str = "X|Also sell at the close when RSI falls more than 5 below its highest value since entry";
const F_GUARD: &str = "F|Template guards: no entry on a bar whose high equals its close, close above the penny price (0), turnover filter off, dead-stock backtest off";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(CLOSE, "Fresh ST Buy, Next-Day Close (volume, no filters)", &[F_FRESH, "E|Buy at the close when the close is above the setup high, the count is at most 2, volume is above its 5-day average and RSI(14) is above 50",
            "X|Profit target 3% above the entry (trailing stop off by default: 'High - n%' 0); on a gap above the target the AFL sells at the open less the 0.25% buffer; the low-equals-close and Prec lower-circuit bars do not exit", X_ST, F_AGAIN, F_GUARD],
            "Fresh ST Buy Next Day Close_Original Without Filters.afl (Close folder), coded as the AFL is written. Runs on one stock at a time; the team ran it over NSE cash stock universes (and Nifty spot); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled. Daily ST by default. Kept quirk: a setup cancelled only after a count, so a down bar right after the flip bar leaves the setup armed."),
        def(HIGH, "Fresh ST Buy, Next-Day High Breakout (volume, no filters)", &[F_FRESH_H,
            "E|Buy stop at the setup high when the high trades above it (at the open + 0.25% when the open is already above), the count at most 2, volume above its 30-day average and RSI(14) above 50",
            X_STOPS, X_ST, F_AGAIN, F_GUARD],
            "Fresh ST Buy Next Day High_Original Without Filters.afl (High folder), coded as the AFL is written. Runs on one stock at a time; the team ran it over NSE cash stock universes (and Nifty spot); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled. The entry is intraday at the stop level; the profit target is measured from that price. The lower-circuit test is Prec-based."),
        def(CLOSE_FLT, "Fresh ST Buy, Next-Day Close (volume, filters, RSI trail)", &[F_FRESH, E_CLOSE, X_STOPS, RSI_TRAIL, X_ST, F_AGAIN, F_FLT, F_GUARD],
            "Fresh ST Buy Next Day Close - Original With Filters.afl (Close folder), coded as the AFL is written; the two Trash copies ('Original With Filters', 'Original_With Filter_(ST overlap issue)') have the same rules and are not coded twice (they default the entry ST to daily, this one to monthly). Runs on one stock at a time; the team ran it over NSE cash stock universes (and Nifty spot); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled. Entry ST timeframe default monthly (the AFL's list starts with MONTHLY); its intraday timeframes are not offered."),
        def(RSI_CROSS, "Fresh ST Buy + RSI/MA Cross (volume, filters)", &[F_FRESH, "F|The ST flip bar must also have RSI(14) crossing above its 10-day average", E_CLOSE, X_STOPS, RSI_TRAIL, X_ST, F_AGAIN, F_FLT, F_GUARD],
            "Fresh ST Buy Next Day Close - With filters & MA RSI Crossover_Final.afl (Close folder), coded as the AFL is written. Runs on one stock at a time; the team ran it over NSE cash stock universes (and Nifty spot); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled. Its 'RSI_exit' hold line never fires (RSI is never below itself less 5), so the RSI exit is the RSI trail."),
        def(RSI_RUNNING, "Fresh ST Buy + RSI above its MA (volume, filters)", &[F_FRESH, "F|The ST flip bar must also have RSI(14) above its 10-day average", E_CLOSE, X_STOPS, RSI_TRAIL, X_ST, F_AGAIN, F_FLT, F_GUARD],
            "Fresh ST Buy Next Day Close - With filters & MA RSI Crossover_Final(Running Crossover).afl (Close folder), coded as the AFL is written. Runs on one stock at a time; the team ran it over NSE cash stock universes (and Nifty spot); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled."),
        def(RSI_CROSS_D1, "Fresh ST Buy + RSI/MA(100) Cross, draft (volume)", &[F_FRESH, "F|The ST flip bar must also have RSI(14) crossing above its 100-day average", E_CLOSE, X_STOPS, "X|Also sell at the close when RSI is below its highest value since entry less the RSI exit buffer (0: any dip)", X_ST, F_AGAIN, F_FLT, F_GUARD],
            "Fresh ST Buy Next Day Close - with filters & MA RSI Crossover.afl (Trash), coded as the AFL is written. Runs on one stock at a time; the team ran it over NSE cash stock universes (and Nifty spot); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled. The AFL fixes the RSI average at 100 days (its 'Set Up RSI Moving Average Days' parameter is unused); its circuit tests are Prec-based; its filters gate entries only (no filter-line exit)."),
        def(RSI_CROSS_V2, "Fresh ST Buy + RSI/MA(100) Cross, v2 (volume)", &[F_FRESH, "F|The ST flip bar must also have RSI(14) crossing above its 100-day average", E_CLOSE, X_STOPS, "X|Also sell at the close when RSI is below its highest value since entry less the RSI exit buffer (0: any dip)", X_ST, F_AGAIN, F_FLT, F_GUARD],
            "Fresh ST Buy Next Day Close - with filters & MA RSI Crossover v2.afl (Trash), coded as the AFL is written. Runs on one stock at a time; the team ran it over NSE cash stock universes (and Nifty spot); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled. RSI average fixed at 100 days as in the AFL; its filters gate entries only (the filter-line array is computed but not used in the exit)."),
        def(RSI_CROSS_V3, "Fresh ST Buy + RSI/MA Cross, v3 (volume)", &[F_FRESH, "F|The ST flip bar must also have RSI(14) crossing above its 10-day average", E_CLOSE, X_STOPS, "X|RSI trail (default 0 means 100, i.e. off)", X_ST, F_AGAIN, F_FLT, F_GUARD],
            "Fresh ST Buy Next Day Close - with filters & MA RSI Crossover v3.afl (Trash), coded as the AFL is written. Runs on one stock at a time; the team ran it over NSE cash stock universes (and Nifty spot); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled. Trail RSI default 0, which the AFL turns into 100 (no RSI exit); its filters gate entries only."),
        def(RSI_CROSS_V3N, "Fresh ST Buy + RSI/MA Cross, v3 new (volume)", &[F_FRESH, "F|The ST flip bar must also have RSI(14) crossing above its 10-day average", E_CLOSE, X_STOPS, "X|Also sell at the close when the lowest RSI since entry is more than 5 below the highest RSI since entry", X_ST, F_AGAIN, F_FLT, F_GUARD],
            "Fresh ST Buy Next Day Close - with filters & MA RSI Crossover v3_NEW.afl (Trash), coded as the AFL is written. Runs on one stock at a time; the team ran it over NSE cash stock universes (and Nifty spot); portfolio limits (MaxOpenPositions 20, position sizing, brokerage) not modelled. The AFL's `if (...);` ends the if, so its RSI_exit line runs on every bar in a trade and keeps the lowest RSI since entry (as written)."),
    ]
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Setup {
    Plain,
    Cross,
    Above,
}
#[derive(Clone, Copy, PartialEq, Debug)]
enum Rx {
    None,
    /// RSI below its peak since entry less the trail (0 -> 100)
    Trail,
    /// RSI below its peak since entry less the buffer (0 stays 0)
    Buffer,
    /// lowest RSI since entry below the peak less the trail
    RunMin,
}
#[derive(Clone, Copy, Debug)]
struct V {
    high_entry: bool,
    cancel_needs_count: bool,
    setup: Setup,
    rx: Rx,
    filters: bool,
    s_exit: bool,
    prec_loc: bool,
    dead_first: bool,
}

fn variant(id: &str) -> Option<V> {
    let base = V { high_entry: false, cancel_needs_count: true, setup: Setup::Plain, rx: Rx::None, filters: false, s_exit: false, prec_loc: true, dead_first: false };
    let flt = V { filters: true, s_exit: true, prec_loc: false, rx: Rx::Trail, ..base };
    Some(match id {
        CLOSE => base,
        HIGH => V { high_entry: true, cancel_needs_count: false, ..base },
        CLOSE_FLT => flt,
        RSI_CROSS => V { setup: Setup::Cross, dead_first: true, ..flt },
        RSI_RUNNING => V { setup: Setup::Above, dead_first: true, ..flt },
        RSI_CROSS_D1 => V { setup: Setup::Cross, rx: Rx::Buffer, s_exit: false, prec_loc: true, ..flt },
        RSI_CROSS_V2 => V { setup: Setup::Cross, rx: Rx::Buffer, s_exit: false, ..flt },
        RSI_CROSS_V3 => V { setup: Setup::Cross, s_exit: false, ..flt },
        RSI_CROSS_V3N => V { setup: Setup::Cross, rx: Rx::RunMin, dead_first: true, ..flt },
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let v = variant(id)?;
    let mut out = vec![
        pd("entry_count", "Entry count (up bars after the flip allowed for entry)", 2.0, 1.0, 50.0, 1.0),
        pd("trail_pct", "Trailing SL % below the highest high (0 off)", if id == CLOSE { 0.0 } else { 3.0 }, 0.0, 25.0, 0.001),
        pd("pt_pct", "Profit target % (0 off)", 3.0, 0.0, 20.0, 0.001),
        pd("buffer_pct", "Buffer % (gap fills)", 0.25, 0.0, 10.0, 0.05),
        pd("st_tf", "ST timeframe (1 daily, 2 weekly, 3 monthly)", if id == CLOSE_FLT { 3.0 } else { 1.0 }, 1.0, 3.0, 1.0),
        pd("st_point", "ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
        pd("st_period", "ST period", 5.0, 1.0, 40.0, 1.0),
        pd("st_factor", "ST factor", 4.0, 0.25, 20.0, 0.25),
        pd("rsi_period", "RSI period", 14.0, 2.0, 200.0, 1.0),
        pd("rsi_level", "RSI long value", 50.0, 0.0, 100.0, 1.0),
        pd("vol_period", "Volume average period", if id == CLOSE { 5.0 } else { 30.0 }, 1.0, 200.0, 1.0),
    ];
    if v.setup != Setup::Plain {
        let fixed = matches!(id, RSI_CROSS_D1 | RSI_CROSS_V2);
        out.push(pd("rsi_ma", if fixed { "RSI average days (the AFL fixes 100)" } else { "Set-up RSI moving average days" }, if fixed { 100.0 } else { 10.0 }, 2.0, 400.0, 1.0));
    }
    match v.rx {
        Rx::Trail | Rx::RunMin => out.push(pd("trail_rsi", "Trail RSI value (0 means 100)", if id == RSI_CROSS_V3 { 0.0 } else { 5.0 }, 0.0, 50.0, 1.0)),
        Rx::Buffer => out.push(pd("rsi_buffer", "Position RSI exit buffer", 0.0, 0.0, 50.0, 1.0)),
        Rx::None => {}
    }
    if v.filters {
        out.push(pd("filter", "Filter version (0 off, 1 ATHC, 2 ST, 3 % swing)", 0.0, 0.0, 3.0, 1.0));
        out.push(pd("athc_swing", "ATHC swing %", 20.0, 1.0, 120.0, 1.0));
        out.push(pd("flt_tf", "ST filter timeframe (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0));
        out.push(pd("flt_period", "ST filter period", 5.0, 1.0, 30.0, 1.0));
        out.push(pd("flt_factor", "ST filter factor", 4.0, 0.5, 10.0, 0.5));
        out.push(pd("flt_point", "ST filter calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0));
        out.push(pd("bear_pct", "Swing % for bear market", 23.0, 1.0, 50.0, 1.0));
        out.push(pd("bull_pct", "Swing % for bull market", 44.0, 1.0, 50.0, 1.0));
    }
    out.extend(template_params());
    Some(out)
}

// ---------------------------------------------------------------- filter lines

/// The team's band loop with its two lines: up when the close beats the previous up band (or `extra_up`), down
/// when it falls below the previous down band, else held (a Null band is not compared); the down band only rises
/// in an up trend and the up band only falls in a down trend; a flip resets the other band; up line = down band on
/// up bars, down line = up band on down bars; with `fill` the bar before a flip also gets the new line's... other
/// line value (`TrendUp[i-1] = TrendDown[i-1]` and the reverse).
fn band_loop(c: &[f64], mut up: Vec<Option<f64>>, mut dn: Vec<Option<f64>>, extra_up: &dyn Fn(usize) -> bool, reset_up: &dyn Fn(usize) -> Option<f64>, reset_dn: &dyn Fn(usize) -> Option<f64>, fill: bool) -> (Vec<i8>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = c.len();
    let mut tr = vec![1i8; n];
    let (mut tu, mut td) = (vec![None; n], vec![None; n]);
    for i in 1..n {
        tr[i] = if up[i - 1].map_or(false, |u| c[i] > u) || extra_up(i) {
            1
        } else if dn[i - 1].map_or(false, |d| c[i] < d) {
            -1
        } else {
            tr[i - 1]
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
            up[i] = reset_up(i);
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = reset_dn(i);
        }
        if tr[i] == 1 {
            tu[i] = dn[i];
            if fill && tr[i - 1] == -1 {
                tu[i - 1] = td[i - 1];
            }
        } else {
            td[i] = up[i];
            if fill && tr[i - 1] == 1 {
                td[i - 1] = tu[i - 1];
            }
        }
    }
    (tr, tu, td)
}

/// ATHCBuysig and the sell line test `s` of the filter versions (filter 0: all on, no sells).
fn fresh_filter(x: &Ctx, g: &dyn Fn(&str) -> f64) -> (Vec<bool>, Vec<bool>) {
    let c = &x.c;
    let n = c.len();
    let mut buy = vec![true; n];
    let mut s = vec![false; n];
    match g("filter").round() as i64 {
        1 => {
            let p = g("athc_swing") / 100.0;
            let base = |k: f64| -> Vec<Option<f64>> { c.iter().map(|v| Some(v + v * k)).collect() };
            let (tr, _tu, td) = band_loop(c, base(p), base(-p), &|_| false, &|i| Some(c[i] + c[i] * p), &|i| Some(c[i] - c[i] * p), true);
            let mut hc = c[0];
            for i in 1..n {
                buy[i] = if c[i] > hc { true } else if tr[i] == -1 && buy[i - 1] { false } else { buy[i - 1] };
                hc = hc.max(c[i]);
                s[i] = c[i] < lv(td[i]);
            }
        }
        2 => {
            let b = compress(x, tf_of(g("flt_tf").round() as i64, true));
            let calc = calc_point(&b, g("flt_point").round() as i64);
            let p = g("flt_period").round().max(1.0) as usize;
            let f = g("flt_factor");
            let trr = true_range(&b.h, &b.l, &b.c);
            let satr: Vec<Option<f64>> = (0..b.c.len())
                .map(|i| if i + 1 < p { None } else { trr[i + 1 - p..=i].iter().try_fold(0.0, |acc, v| v.map(|v| acc + v)).map(|acc| acc / p as f64) })
                .collect();
            let up: Vec<Option<f64>> = (0..b.c.len()).map(|i| satr[i].map(|s| calc[i] + f * s)).collect();
            let dn: Vec<Option<f64>> = (0..b.c.len()).map(|i| satr[i].map(|s| calc[i] - f * s)).collect();
            let (_, tu, td) = band_loop(&b.c, up, dn, &|_| false, &|i| satr[i].map(|s| calc[i] + f * s), &|i| satr[i].map(|s| calc[i] - f * s), false);
            let (tu, td) = (expand_last(&tu, &b), expand_last(&td, &b));
            for i in 1..n {
                buy[i] = c[i] > lv(tu[i]);
            }
            for i in 1..n {
                s[i] = c[i] < lv(td[i]);
            }
        }
        3 => {
            let (br, bl) = (g("bear_pct") / 100.0, g("bull_pct") / 100.0);
            let mut hcl = c.clone();
            for i in 1..n {
                hcl[i] = hcl[i - 1].max(c[i]);
            }
            let up: Vec<Option<f64>> = c.iter().map(|v| Some(v + v * bl)).collect();
            let dn: Vec<Option<f64>> = c.iter().map(|v| Some(v - v * br)).collect();
            let (_, tu, td) = band_loop(c, up, dn, &|i| c[i] > hcl[i - 1], &|i| Some(hcl[i].min(c[i] + c[i] * bl)), &|i| Some(c[i] - c[i] * br), true);
            for i in 1..n {
                buy[i] = c[i] > lv(tu[i - 1]);
                s[i] = c[i] < lv(td[i]);
            }
        }
        _ => {}
    }
    (buy, s)
}

// ---------------------------------------------------------------- the AFL loop

const BIG: f64 = 9_999_999.0;

/// Arrays the loop reads (built in `run`; tests supply their own).
struct In {
    tr: Vec<i8>,
    rsi: Vec<Option<f64>>,
    setup_ok: Vec<bool>,
    vol: Vec<f64>,
    vavg: Vec<Option<f64>>,
    athc: Vec<bool>,
    s: Vec<bool>,
}

struct Cfg {
    v: V,
    entry_count: f64,
    trail: f64,
    pt: f64,
    buf: f64,
    rsi_level: f64,
    /// Trail_RSI_ (or the exit buffer)
    rsi_gap: f64,
}

fn fsv_loop(x: &Ctx, a: &In, k: &Cfg, gd: &Guard) -> Loop {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let mut s = Loop::new(n);
    let v = k.v;
    let (mut pos, mut abc, mut count, mut my_high) = (false, 0u8, 0i64, 0.0f64);
    let (mut hh, mut pt, mut tsl) = (BIG, BIG, 0.0f64);
    let mut peak: Vec<f64> = a.rsi.iter().map(|&r| lv(r)).collect();
    let mut rexit = peak.clone();
    for i in 1..n {
        if !pos && a.tr[i] == 1 && a.tr[i - 1] == -1 && abc == 0 && a.setup_ok[i] {
            my_high = h[i];
            abc = 1;
        } else if !pos && a.tr[i] == 1 && abc == 1 {
            count += 1;
        } else if !pos && (count > 0 || !v.cancel_needs_count) && a.tr[i] == -1 && abc == 1 {
            abc = 0;
            my_high = 0.0;
            count = 0;
        } else if abc == 2 && a.tr[i] == -1 {
            abc = 0;
        }
        let dead = gd.dead_sell(x, i);
        let buy_ok = h[i] != c[i] && c[i] > gd.penny && gd.turnover(x, i) && gd.dead_buy(x, i) && a.athc[i];
        let trig = if v.high_entry { h[i] > my_high } else { c[i] > my_high };
        if !pos && abc == 1 && a.vol[i] > lv(a.vavg[i]) && lv(a.rsi[i]) > k.rsi_level && (count as f64) <= k.entry_count && trig && my_high > 0.0 && buy_ok {
            let px = if !v.high_entry { c[i] } else if o[i] > my_high { o[i] + o[i] * k.buf / 100.0 } else { my_high };
            s.entry[i] = Some(px);
            if k.trail > 0.0 {
                hh = h[i];
                tsl = (1.0 - k.trail * 0.01) * hh;
            }
            if k.pt > 0.0 {
                pt = (1.0 + k.pt * 0.01) * px;
            }
            my_high = 0.0;
            pos = true;
        } else if pos {
            if k.trail > 0.0 {
                hh = h[i].max(hh);
            }
            if v.filters {
                if peak[i] < peak[i - 1] {
                    peak[i] = peak[i - 1];
                }
                if v.rx == Rx::RunMin {
                    rexit[i] = rexit[i].min(rexit[i - 1]);
                }
            }
            let rsi_x = match v.rx {
                Rx::None => false,
                Rx::Trail | Rx::Buffer => lv(a.rsi[i]) < peak[i] - k.rsi_gap,
                Rx::RunMin => rexit[i] < peak[i] - k.rsi_gap,
            };
            let sx = v.s_exit && a.s[i];
            let loc = if v.prec_loc { lo_circuit(x, i) } else { l[i] == c[i] };
            let mut out = None;
            if (l[i] <= tsl || h[i] >= pt || rsi_x || sx || (v.dead_first && dead)) && !loc {
                out = Some(if l[i] <= tsl {
                    (if o[i] < tsl { o[i] - o[i] * k.buf / 100.0 } else { tsl }, "Trailing SL")
                } else if h[i] >= pt {
                    (if o[i] > pt { o[i] - o[i] * k.buf / 100.0 } else { pt }, "Profit target")
                } else if rsi_x {
                    (c[i], "RSI trail")
                } else if sx {
                    (c[i], "Filter line")
                } else {
                    (c[i], "Dead stock (last bar)")
                });
            } else if (a.tr[i] == -1 && l[i] != c[i]) || dead {
                out = Some((c[i], if dead { "Dead stock (last bar)" } else { "Supertrend down" }));
            }
            if out.is_some() {
                s.exit[i] = out;
                pos = false;
                hh = BIG;
                pt = BIG;
                count = 0;
                my_high = 0.0;
                abc = 2;
            }
            if k.trail > 0.0 {
                tsl = (1.0 - k.trail * 0.01) * hh;
            }
        }
    }
    s
}

/// AFL MA of an array with leading Nulls: Null unless the last n values exist.
fn ma_opt(v: &[Option<f64>], n: usize) -> Vec<Option<f64>> {
    let n = n.max(1);
    (0..v.len()).map(|i| if i + 1 < n { None } else { v[i + 1 - n..=i].iter().try_fold(0.0, |s, x| x.map(|x| s + x)).map(|s| s / n as f64) }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let v = variant(id)?;
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n < 2 || from > to || to >= n {
        return Some(Vec::new());
    }
    let tf = match g("st_tf").round() as i64 { 2 => 'W', 3 => 'M', _ => 'D' };
    let tr = st_filter(x, tf, g("st_point").round() as i64, g("st_period").round().max(1.0) as usize, g("st_factor"));
    let rsi = rsi_opt(&x.c, g("rsi_period").round().max(1.0) as usize);
    let setup_ok = match v.setup {
        Setup::Plain => vec![true; n],
        Setup::Cross => cross(&rsi, &ma_opt(&rsi, g("rsi_ma").round() as usize)),
        Setup::Above => {
            let m = ma_opt(&rsi, g("rsi_ma").round() as usize);
            (0..n).map(|i| lv(rsi[i]) > lv(m[i])).collect()
        }
    };
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let vavg = sma(&vol, g("vol_period").round().max(1.0) as usize);
    let (athc, s) = if v.filters { fresh_filter(x, &g) } else { (vec![true; n], vec![false; n]) };
    let rsi_gap = match v.rx {
        Rx::Trail | Rx::RunMin => {
            let t = g("trail_rsi");
            if t > 0.0 {
                t
            } else {
                100.0
            }
        }
        Rx::Buffer => g("rsi_buffer"),
        Rx::None => 0.0,
    };
    let k = Cfg { v, entry_count: g("entry_count"), trail: g("trail_pct"), pt: g("pt_pct"), buf: g("buffer_pct"), rsi_level: g("rsi_level"), rsi_gap };
    let gd = Guard::of(&g, volsig17);
    let a = In { tr, rsi, setup_ok, vol, vavg, athc, s };
    Some(fills(x, &fsv_loop(x, &a, &k, &gd), from, to, if v.high_entry { "stop" } else { "close" }))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Fresh ST Buy Next Day High-Close"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytnr7::tests::{bars, ps, smoke, wave};

    const B: (f64, f64, f64, f64) = (100.0, 101.0, 99.0, 100.0);
    fn sample() -> Vec<(f64, f64, f64, f64)> {
        vec![
            B,
            B,                            // 1: ST down
            B,                            // 2: ST turns up: setup high 101, count 0
            (101.0, 103.0, 100.5, 102.5), // 3: count 1, close 102.5 > 101 -> buy
            (102.5, 104.0, 102.0, 103.5), // 4
            (103.5, 106.0, 103.0, 105.0), // 5: high 106 >= 102.5 x 1.03 = 105.575 -> target
            (105.0, 106.0, 104.0, 105.5), // 6
        ]
    }
    fn input(n: usize, tr: &[i8], rsi: f64) -> In {
        In { tr: tr.to_vec(), rsi: vec![Some(rsi); n], setup_ok: vec![true; n], vol: vec![2.0; n], vavg: vec![Some(1.0); n], athc: vec![true; n], s: vec![false; n] }
    }
    fn cfg(id: &str) -> Cfg {
        let schema = params(id).unwrap();
        let g = |k: &str| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def);
        Cfg { v: variant(id).unwrap(), entry_count: g("entry_count"), trail: g("trail_pct"), pt: g("pt_pct"), buf: g("buffer_pct"), rsi_level: g("rsi_level"), rsi_gap: if variant(id).unwrap().rx == Rx::Buffer { g("rsi_buffer") } else if g("trail_rsi") > 0.0 { g("trail_rsi") } else { 100.0 } }
    }
    fn trades(id: &str, v: &[(f64, f64, f64, f64)], a: &In) -> Vec<(usize, f64, usize, f64, String)> {
        let r = bars(v);
        let x = Ctx::new(&r, &r, &[]);
        let schema = params(id).unwrap();
        let g = |k: &str| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def);
        let gd = Guard::of(&g, volsig17);
        fills(&x, &fsv_loop(&x, a, &cfg(id), &gd), 0, r.len() - 1, "close").into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, (t.exit_px * 1e6).round() / 1e6, t.reason)).collect()
    }
    const TR: [i8; 7] = [1, -1, 1, 1, 1, 1, 1];

    #[test]
    fn close_version_buys_after_the_flip_and_takes_the_target() {
        let t = trades(CLOSE, &sample(), &input(7, &TR, 60.0));
        assert_eq!(t, vec![(3, 102.5, 5, 105.575, "Profit target".to_string())]);
        // RSI not above 50 or volume not above its average: no trade
        assert!(trades(CLOSE, &sample(), &input(7, &TR, 50.0)).is_empty());
        let mut a = input(7, &TR, 60.0);
        a.vol = vec![1.0; 7];
        assert!(trades(CLOSE, &sample(), &a).is_empty());
    }

    #[test]
    fn high_version_buys_at_the_setup_high() {
        let t = trades(HIGH, &sample(), &input(7, &TR, 60.0));
        // bar 3: high 103 > 101, open 101 not above it -> buy at 101; target 104.03 reached on bar 4 (high 104 < 104.03) no,
        // bar 5 high 106 -> sell at the target
        assert_eq!((t[0].0, t[0].1, t[0].2, t[0].3), (3, 101.0, 5, 104.03));
    }

    #[test]
    fn filter_version_trails_and_follows_rsi() {
        // trailing stop: 3% below the highest high of the previous bars
        let mut v = sample();
        v[5] = (103.5, 104.5, 100.5, 101.0); // stop 104 x 0.97 = 100.88: low 100.5 -> sell at 100.88
        let mut a = input(7, &TR, 60.0);
        let t = trades(CLOSE_FLT, &v, &a);
        assert_eq!((t[0].0, t[0].2, t[0].3, t[0].4.as_str()), (3, 5, 100.88, "Trailing SL"));
        // RSI trail: RSI 60 at entry, 70 next, then 64 (more than 5 below the 70 peak) -> sell at the close
        a.rsi = vec![Some(60.0), Some(60.0), Some(60.0), Some(60.0), Some(70.0), Some(64.0), Some(64.0)];
        let w = vec![B, B, B, (101.0, 103.0, 100.5, 102.5), (102.5, 104.0, 102.0, 103.5), (103.5, 104.0, 102.5, 103.0), B];
        let t = trades(CLOSE_FLT, &w, &a);
        assert_eq!((t[0].2, t[0].3, t[0].4.as_str()), (5, 103.0, "RSI trail"));
        // v3_new keeps the lowest RSI since entry; the draft exits on any dip below the peak
        assert_eq!(trades(RSI_CROSS_V3N, &w, &a)[0].2, 4); // lowest RSI since entry 60 < 70 - 5 on bar 4
        a.rsi[5] = Some(69.0);
        assert_eq!(trades(CLOSE_FLT, &w, &a)[0].2, 6);
        assert_eq!(trades(RSI_CROSS_D1, &w, &a)[0].2, 5);
    }

    #[test]
    fn st_down_exit_and_fresh_flip_needed() {
        let v = vec![B, B, B, (101.0, 103.0, 100.5, 102.5), (102.0, 102.5, 101.0, 101.5), B, (100.0, 102.5, 99.5, 101.5), (102.0, 104.0, 101.8, 103.0)];
        // sold at the close of bar 4 (ST down); the ST must be down on a later bar (5) before the bar-6 flip arms a
        // new setup (high 102.5); bar 7 closes 103 -> buy
        let tr = [1, -1, 1, 1, -1, -1, 1, 1];
        let t = trades(CLOSE, &v, &input(8, &tr, 60.0));
        assert_eq!(t.iter().map(|t| (t.0, t.2)).collect::<Vec<_>>(), vec![(3, 4), (7, 7)]);
        // with the ST up again right after the exit bar, no new setup is armed
        let tr = [1, -1, 1, 1, -1, 1, 1, 1];
        let t = trades(CLOSE, &v, &input(8, &tr, 60.0));
        assert_eq!(t.iter().map(|t| (t.0, t.2)).collect::<Vec<_>>(), vec![(3, 4)]);
    }

    #[test]
    fn cross_setup_is_required_on_the_flip_bar() {
        let mut a = input(7, &TR, 60.0);
        a.setup_ok = vec![false; 7];
        assert!(trades(RSI_CROSS, &sample(), &a).is_empty());
        a.setup_ok[2] = true;
        assert_eq!(trades(RSI_CROSS, &sample(), &a)[0].0, 3);
    }

    #[test]
    fn band_loop_lines_fill_the_bar_before_a_flip() {
        let c = [100.0, 101.0, 102.0, 85.0, 86.0];
        let up: Vec<Option<f64>> = c.iter().map(|v| Some(v * 1.1)).collect();
        let dn: Vec<Option<f64>> = c.iter().map(|v| Some(v * 0.9)).collect();
        let (tr, tu, td) = band_loop(&c, up, dn, &|_| false, &|i| Some(c[i] * 1.1), &|i| Some(c[i] * 0.9), true);
        assert_eq!(tr, vec![1, 1, 1, -1, -1]);
        assert_eq!(td[2], tu[2]);
        assert!(td[3].is_some() && tu[3].is_none());
    }

    #[test]
    fn every_strategy_runs() {
        assert!(smoke(&IDS, run, params, family_of) > 0);
        let r = wave(420);
        let x = Ctx::new(&r, &r, &[]);
        for id in [CLOSE_FLT, RSI_CROSS, RSI_RUNNING] {
            for f in [1.0, 2.0, 3.0] {
                for tf in [1.0, 2.0, 3.0] {
                    let t = run(id, &x, 30, r.len() - 1, &ps(&[("filter", f), ("flt_tf", tf), ("st_tf", tf)])).unwrap();
                    assert!(t.iter().all(|t| t.legs[0].i >= 30 && t.exit_i >= t.legs[0].i), "{id}");
                }
            }
        }
        for id in IDS {
            let d = catalogue().into_iter().find(|d| d.id == id).unwrap();
            assert!(d.needs_volume && d.notes.contains("coded as the AFL is written") && d.notes.contains("one stock at a time"), "{id}");
        }
    }
}
