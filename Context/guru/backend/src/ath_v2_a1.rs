//! ATH sub-variants of batch a1: Bull / Bear market index filters (index Supertrend / SMA / %swing filters, exit-all-in-bear, +x % buy,
//! delay entry, ST 5,2 / 5,4 and %swing loss gates). 25 variants, one parameterised core (`run_symbol`) with a `Spec` per variant.
//!
//! The building blocks are COPIES of the code verified on the team's sheets in other modules (never edited there):
//! * `ath_plus.rs`  : %swing loop, ATHC trigger / latch, index swing filter, index ST / SMA filter (monthly / weekly / daily, `TimeFrameExpand`,
//!   "pad and align"), loss counter of the %swing trades, circuits, 30-bar dead stock, turnover tables;
//! * `ath_loss1.rs` : Supertrend(5,2) / Supertrend(5,4) virtual-trade loss counter (`VarSet` overwrites BPD2 / BPD3 with the ST up lines).
//! List parameters are 0-based indexes: deadstock 0 YES / 1 NO, turnflt 0 NO / 1 YES, mktflt 0 NONE / 1 INDEX, prflt 0 NSENIFTY / 1 NSENIFTY1 / 2 NIFTY /
//! 3 NSE500, flttf 0 MONTHLY / 1 WEEKLY / 2 DAILY, iflt 0 ST / 1 SMA (swing lists: only one entry), istflt 0 MID POINT / 1 CLOSE / 2 PIVOT, stflt1 likewise.
//! Engine options (not AFL parameters): `comm` (AFL BROKERAGE, enters the loss-counter profit; follows the run's brokerage), `pad` / `padref`
//! (Analysis setting "Pad and align to the reference symbol", default off).
//! AFL `Optimize(...)` parameters are not exposed (no catalog key): they are constants at the AFL default clamped into its range.
//! Verification: `ATH/v2/<id>/VERIFY.md`.
#![allow(dead_code)]
#[allow(unused_imports)]
use crate::ath_core::*;
use crate::ath_ind::tr as ind_tr;

/// AmiBroker's Null inside the Supertrend arrays of the ST 5,2 / 5,4 probability.
const NULLV: f32 = -1.0e10;

// ======================= copied from ath_plus.rs (verified there) =======================
/// AmiBroker's Null (EMPTY_VAL).
const NULL: f32 = -1.0e10;
fn is_null(x: f32) -> bool {
    x <= -9.0e9
}
/// Null-propagating arithmetic helpers (AmiBroker: any Null operand gives Null).
fn nadd(a: f32, b: f32) -> f32 {
    if is_null(a) || is_null(b) { NULL } else { a + b }
}
fn nsub(a: f32, b: f32) -> f32 {
    if is_null(a) || is_null(b) { NULL } else { a - b }
}
fn nmul(a: f32, b: f32) -> f32 {
    if is_null(a) || is_null(b) { NULL } else { a * b }
}

const INDEX_NAMES: [&str; 4] = ["NSENIFTY", "NSENIFTY1", "NIFTY", "NSE500"];

// ---------------------------------------------------------------- small AFL building blocks

fn volsig_loss(y: i32) -> f32 {
    // the table of the "consecutive loss" AFLs (compared with V*C)
    match y {
        y if y < 2000 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

fn volsig_std(y: i32) -> f32 {
    // the yearly table of the "new turnover" AFLs (compared with OpenInt or V*C)
    match y {
        y if y < 2000 => 30_000.0,
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
        2015 | 2016 => 4_000_000.0,
        2017 => 5_000_000.0,
        2018 => 6_000_000.0,
        2019 => 7_000_000.0,
        2020 => 8_000_000.0,
        _ => 9_000_000.0,
    }
}

/// `HHV(C, BarCount)`: highest close up to the bar.
fn hhv_all(c: &[f32]) -> Vec<f32> {
    let mut out = vec![0f32; c.len()];
    for i in 0..c.len() {
        out[i] = if i == 0 || c[i] > out[i - 1] { c[i] } else { out[i - 1] };
    }
    out
}

/// The 30-bars-flat "dead stock" test: O,H,L,C equal to those of each of the next 30 bars (`Ref(x, k)` with k > 0 looks forward).
fn flat30(o: &[f32], h: &[f32], l: &[f32], c: &[f32]) -> Vec<bool> {
    let n = c.len();
    let mut out = vec![false; n];
    for i in 0..n {
        if i + 30 < n {
            out[i] = (1..=30).all(|k| o[i] == o[i + k] && h[i] == h[i + k] && l[i] == l[i + k] && c[i] == c[i + k]);
        }
    }
    out
}

/// (UPcircuit, locircuit) with `Prec(x, 2)` equality.
fn circuits(o: &[f32], h: &[f32], l: &[f32], c: &[f32]) -> (Vec<bool>, Vec<bool>) {
    let n = c.len();
    let (mut up, mut lo) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        up[i] = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
        lo[i] = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
    }
    (up, lo)
}

/// Result of the AFL's %swing / supertrend loop: trend (+1 / -1) and the TrendUp / TrendDown arrays (Null elsewhere, with the
/// AFL's `changeOfTrend` back-fill of the previous bar).
struct Sw {
    trend: Vec<i8>,
    tup: Vec<f32>,
    tdn: Vec<f32>,
}

/// The loop shared by every AFL here. `up` / `dn` are the initial bands; `up_flag` / `dn_flag` the values the band takes when the trend flips;
/// `extra` is the index swing filter's additional "or Close > index ATH close" condition.
fn swing_loop(close: &[f32], mut up: Vec<f32>, mut dn: Vec<f32>, up_flag: &[f32], dn_flag: &[f32], extra: Option<&[f32]>) -> Sw {
    let n = close.len();
    let mut trend = vec![1i8; n];
    let mut tup = vec![NULL; n];
    let mut tdn = vec![NULL; n];
    let mut chg = false;
    for i in 1..n {
        let mut t = 1i8;
        if close[i] > up[i - 1] || extra.map_or(false, |e| close[i] > e[i - 1]) {
            t = 1;
            if trend[i - 1] == -1 {
                chg = true;
            }
        } else if close[i] < dn[i - 1] {
            t = -1;
            if trend[i - 1] == 1 {
                chg = true;
            }
        } else if trend[i - 1] == 1 {
            t = 1;
            chg = false;
        } else if trend[i - 1] == -1 {
            t = -1;
            chg = false;
        }
        trend[i] = t;
        let flag1 = t < 0 && trend[i - 1] > 0;
        let flagh = t > 0 && trend[i - 1] < 0;
        if t > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if t < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if flag1 {
            up[i] = up_flag[i];
        }
        if flagh {
            dn[i] = dn_flag[i];
        }
        if t == 1 {
            tup[i] = dn[i];
            if chg {
                tup[i - 1] = tdn[i - 1];
                chg = false;
            }
        } else if t == -1 {
            tdn[i] = up[i];
            if chg {
                tdn[i - 1] = tup[i - 1];
                chg = false;
            }
        }
    }
    Sw { trend, tup, tdn }
}

/// The plain %swing of the stock: Up = C + EXIT, Dn = C - EXIT, EXIT = C * EXITPCT / 100.
fn pct_swing(c: &[f32], exit_pct: f32) -> Sw {
    let n = c.len();
    let exit: Vec<f32> = c.iter().map(|&x| (x * exit_pct) / 100.0).collect();
    let up: Vec<f32> = (0..n).map(|i| c[i] + exit[i]).collect();
    let dn: Vec<f32> = (0..n).map(|i| c[i] - exit[i]).collect();
    let (uf, df) = (up.clone(), dn.clone());
    swing_loop(c, up, dn, &uf, &df, None)
}

/// ATHC trigger: the close that made a new all-time high (while no trade latch is on) x (1 + pct / 100); Null while the swing is down.
fn trigger_latch(c: &[f32], hcl: &[f32], trend1: &[i8], pct: f32) -> Vec<f32> {
    let n = c.len();
    let mut t = vec![false; n];
    let mut bprice = vec![NULL; n];
    let mut trigger = vec![NULL; n];
    for i in 1..n {
        if c[i] > hcl[i - 1] && !t[i - 1] {
            t[i] = true;
            bprice[i] = c[i];
            trigger[i] = bprice[i] + (bprice[i] * pct) / 100.0;
        } else {
            bprice[i] = bprice[i - 1];
            trigger[i] = trigger[i - 1];
            t[i] = t[i - 1];
        }
        if trend1[i] == -1 {
            t[i] = false;
            bprice[i] = NULL;
            trigger[i] = NULL;
        }
    }
    trigger
}

/// ATHCbuysig / buysig: 1 on a new all-time-high close (optionally also needing Close > trigger), 0 once the swing is down, else unchanged.
fn athc_latch(c: &[f32], hcl: &[f32], trend1: &[i8], trigger: Option<&[f32]>) -> Vec<bool> {
    let n = c.len();
    let mut s = vec![false; n];
    for i in 1..n {
        s[i] = if c[i] > hcl[i - 1] && trigger.map_or(true, |t| c[i] > t[i]) {
            true
        } else if trend1[i] == -1 && s[i - 1] {
            false
        } else {
            s[i - 1]
        };
    }
    s
}

fn clamp_prices(sg: &mut Signals, h: &[f32], l: &[f32]) {
    for i in 0..sg.buy_price.len() {
        sg.buy_price[i] = sg.buy_price[i].min(h[i]).max(l[i]);
        sg.sell_price[i] = sg.sell_price[i].min(h[i]).max(l[i]);
    }
}

fn dead_flags(n: usize, dead_yes: bool) -> (Vec<bool>, Vec<bool>) {
    // (deadstockbuysig, deadstocksellsig)
    if dead_yes {
        ((0..n).map(|i| i != n - 1).collect(), (0..n).map(|i| i == n - 1).collect())
    } else {
        ((0..n).map(|i| i != 0).collect(), vec![false; n])
    }
}

// ---------------------------------------------------------------- the index as a Foreign() array

/// A foreign symbol's price arrays aligned to the current symbol's bar dates (AmiBroker `SetForeign` with the default fix-up:
/// a missing bar repeats the previous close with zero volume; before the foreign symbol starts the value is Null).
struct Al {
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    c: Vec<f32>,
}

fn align(idx: &Series, dates: &[i32]) -> Al {
    let m = idx.bars.len();
    let mut j = 0usize;
    let mut a = Al { o: Vec::with_capacity(dates.len()), h: Vec::with_capacity(dates.len()), l: Vec::with_capacity(dates.len()), c: Vec::with_capacity(dates.len()) };
    for &d in dates {
        while j < m && idx.bars[j].date <= d {
            j += 1;
        }
        if j == 0 {
            a.o.push(NULL);
            a.h.push(NULL);
            a.l.push(NULL);
            a.c.push(NULL);
        } else {
            let b = &idx.bars[j - 1];
            if b.date == d {
                a.o.push(b.o);
                a.h.push(b.h);
                a.l.push(b.l);
                a.c.push(b.c);
            } else {
                a.o.push(b.c);
                a.h.push(b.c);
                a.l.push(b.c);
                a.c.push(b.c);
            }
        }
    }
    a
}

fn days_from_civil(y: i32, m: i32, d: i32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Key of the compressed bar a daily bar belongs to: calendar month, calendar week (Monday based) or the day itself.
fn period_key(d: i32, tf: u8) -> i64 {
    match tf {
        0 => (d / 100) as i64,
        1 => (days_from_civil(d / 10000, d / 100 % 100, d % 100) + 3).div_euclid(7),
        _ => d as i64,
    }
}

struct Comp {
    start: Vec<usize>,
    end: Vec<usize>,
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    c: Vec<f32>,
}

/// `TimeFrameSet(monthly / weekly / daily)` of an aligned foreign array.
fn compress(dates: &[i32], al: &Al, tf: u8) -> Comp {
    let mut k = Comp { start: vec![], end: vec![], o: vec![], h: vec![], l: vec![], c: vec![] };
    let mut last_key = i64::MIN;
    for i in 0..dates.len() {
        let key = period_key(dates[i], tf);
        if key != last_key || k.start.is_empty() {
            k.start.push(i);
            k.end.push(i);
            k.o.push(al.o[i]);
            k.h.push(al.h[i]);
            k.l.push(al.l[i]);
            k.c.push(al.c[i]);
            last_key = key;
        } else {
            let g = k.start.len() - 1;
            k.end[g] = i;
            k.h[g] = k.h[g].max(al.h[i]);
            k.l[g] = k.l[g].min(al.l[i]);
            k.c[g] = al.c[i];
        }
    }
    k
}

/// `TimeFrameExpand(x, tf)` (default mode): a compressed bar's value is shown on its last daily bar; the other daily bars of the
/// period carry the previous compressed bar's value.
fn expand(x: &[f32], k: &Comp, n: usize) -> Vec<f32> {
    let mut out = vec![NULL; n];
    for g in 0..k.start.len() {
        for j in k.start[g]..k.end[g] {
            out[j] = if g > 0 { x[g - 1] } else { NULL };
        }
        out[k.end[g]] = x[g];
    }
    out
}

/// Parameters of the index supertrend / SMA filter.
#[derive(Clone, Copy)]
struct Flt {
    tf: u8,
    sma: bool,
    point: u8,
    pd: usize,
    fact: f32,
    mapd: usize,
}

/// (FLTBuy, FLTShort) of the AFL's ST / SMA index filter on the stock's bar dates.
fn st_filter(al: &Al, dates: &[i32], f: &Flt) -> (Vec<bool>, Vec<bool>) {
    let n = dates.len();
    let k = compress(dates, al, f.tf);
    let m = k.start.len();
    let cal: Vec<f32> = (0..m)
        .map(|g| {
            if is_null(k.h[g]) || is_null(k.l[g]) || is_null(k.c[g]) {
                NULL
            } else {
                match f.point {
                    0 => (k.h[g] + k.l[g]) / 2.0,
                    2 => (k.h[g] + k.l[g] + k.c[g]) / 3.0,
                    _ => k.c[g],
                }
            }
        })
        .collect();
    if f.sma {
        let mav: Vec<f32> = crate::ath_ind::ma(&k.c, f.mapd).into_iter().map(|x| if x.is_nan() { NULL } else { x }).collect();
        let mut prev = vec![NULL; m];
        for g in 1..m {
            prev[g] = mav[g - 1];
        }
        let ic = expand(&k.c, &k, n);
        let px = expand(&prev, &k, n);
        return ((0..n).map(|i| ic[i] > px[i]).collect(), (0..n).map(|i| ic[i] < px[i]).collect());
    }
    // ATR(1) = true range of the compressed bar
    let tr: Vec<f32> = (0..m)
        .map(|g| {
            if g == 0 {
                nsub(k.h[g], k.l[g])
            } else if is_null(k.h[g]) || is_null(k.l[g]) || is_null(k.c[g - 1]) {
                NULL
            } else {
                k.h[g].max(k.c[g - 1]) - k.l[g].min(k.c[g - 1])
            }
        })
        .collect();
    let pd = f.pd;
    let satr: Vec<f32> = (0..m)
        .map(|g| {
            if pd == 0 || g + 1 < pd {
                return NULL;
            }
            let mut s = tr[g];
            for i in 1..pd {
                s = nadd(s, tr[g - i]);
            }
            if is_null(s) { NULL } else { s / pd as f32 }
        })
        .collect();
    let up: Vec<f32> = (0..m).map(|g| nadd(cal[g], nmul(f.fact, satr[g]))).collect();
    let dn: Vec<f32> = (0..m).map(|g| nsub(cal[g], nmul(f.fact, satr[g]))).collect();
    let (uf, df) = (up.clone(), dn.clone());
    let sw = swing_loop(&k.c, up, dn, &uf, &df, None);
    let tf: Vec<f32> = sw.trend.iter().map(|&t| t as f32).collect();
    let ex = expand(&tf, &k, n);
    ((0..n).map(|i| ex[i] == 1.0).collect(), (0..n).map(|i| ex[i] == -1.0).collect())
}

/// (FLTBuy, FLTShort) with the stock padded and aligned to the filter index (Analysis setting "Pad and align data to the reference
/// symbol" with `refc` as reference): the filter runs on the reference symbol's calendar (the index aligned to it) and a stock bar
/// takes the value of the reference bar of its date (the last one before it).
fn st_filter_padded(ix: &Series, refc: &Series, dates: &[i32], f: &Flt) -> (Vec<bool>, Vec<bool>) {
    let rdates: Vec<i32> = refc.bars.iter().map(|b| b.date).collect();
    let al = align(ix, &rdates);
    let (b, s) = st_filter(&al, &rdates, f);
    let mut j = 0usize;
    let (mut ob, mut os) = (Vec::with_capacity(dates.len()), Vec::with_capacity(dates.len()));
    for &d in dates {
        while j < rdates.len() && rdates[j] <= d {
            j += 1;
        }
        if j == 0 {
            ob.push(false);
            os.push(false);
        } else {
            ob.push(b[j - 1]);
            os.push(s[j - 1]);
        }
    }
    (ob, os)
}

fn flt_from(g: G) -> Flt {
    Flt {
        tf: g("flttf").round() as u8,
        sma: g("iflt").round() == 1.0,
        point: g("istflt").round() as u8,
        pd: g("ifltpd").round().max(0.0) as usize,
        fact: g("ifltfact") as f32,
        mapd: g("ifltmapd").round().max(1.0) as usize,
    }
}

fn index_of<'a>(env: &'a Env, g: G) -> Option<&'a Series> {
    env.get(INDEX_NAMES[(g("prflt").round() as usize).min(3)])
}

fn blank(n: usize) -> Signals {
    Signals::new(vec![false; n], vec![false; n], vec![0.0; n], vec![0.0; n])
}

// ======================= copied from ath_loss1.rs (verified there) =======================
/// The AFL's Supertrend function `ST1` / the index block: returns (up line, down line, trend). Lines hold NULLV where the AFL has Null.
/// satr = (sum of ATR(1) over `period` bars) / period, accumulated in the AFL's order; Null while the history is too short.
fn supertrend(stcal: &[f32], h: &[f32], l: &[f32], c: &[f32], period: usize, factor: f32) -> (Vec<f32>, Vec<f32>, Vec<i8>) {
    let n = c.len();
    let t = ind_tr(h, l, c);
    let satr: Vec<f32> = (0..n)
        .map(|i| {
            if period == 0 || i + 1 < period {
                return f32::NAN;
            }
            let mut v = t[i];
            for k in 1..period {
                v += t[i - k];
            }
            v / period as f32
        })
        .collect();
    let mut up: Vec<f32> = (0..n).map(|i| stcal[i] + factor * satr[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| stcal[i] - factor * satr[i]).collect();
    let mut trend = vec![1i8; n];
    let mut upline = vec![NULLV; n];
    let mut dnline = vec![NULLV; n];
    for i in 1..n {
        let t1 = if c[i] > up[i - 1] {
            1
        } else if c[i] < dn[i - 1] {
            -1
        } else {
            trend[i - 1]
        };
        trend[i] = t1;
        let flag1 = t1 < 0 && trend[i - 1] > 0;
        let flagh1 = t1 > 0 && trend[i - 1] < 0;
        if t1 > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if t1 < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if flag1 {
            up[i] = stcal[i] + factor * satr[i];
        }
        if flagh1 {
            dn[i] = stcal[i] - factor * satr[i];
        }
        if t1 == 1 {
            upline[i] = if dn[i].is_nan() { NULLV } else { dn[i] };
        } else {
            dnline[i] = if up[i].is_nan() { NULLV } else { up[i] };
        }
    }
    (upline, dnline, trend)
}

/// `Cross(Ref(line, -1), C)`: the previous bar's line crosses above the close (the close falls through it).
fn cross_line_below_close(line: &[f32], c: &[f32]) -> Vec<bool> {
    let n = c.len();
    let mut out = vec![false; n];
    for i in 1..n {
        let a_i = line[i - 1];
        let a_p = if i >= 2 { line[i - 2] } else { NULLV };
        out[i] = a_i > c[i] && a_p <= c[i - 1];
    }
    out
}

fn stcal_of(mode: u8, h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    (0..c.len())
        .map(|i| match mode {
            0 => (h[i] + l[i]) / 2.0,
            2 => (h[i] + l[i] + c[i]) / 3.0,
            _ => c[i],
        })
        .collect()
}

// ======================= own code: the parameterised core =======================

#[derive(Clone, Copy)]
struct R {
    def: f64,
    min: f64,
    max: f64,
}
const fn r(def: f64, min: f64, max: f64) -> R {
    R { def, min, max }
}

/// Index filter of the AFL: `SWING` (23 % / 44 % index %swing) or `ST` (Supertrend / SMA of the index on monthly / weekly / daily bars).
#[derive(Clone, Copy, PartialEq)]
enum Filt {
    Swing,
    St,
}
/// Turnover filter table: `V*C` against the 6-bucket table of the older AFLs, or `OpenInt` (= C*V in the database) against the 16-bucket table.
#[derive(Clone, Copy, PartialEq)]
enum Turn {
    Vc6,
    OiStd,
}
/// Delay entry (`athccount > delaybuy`): which flag Buy needs, the "running" Buy / Sell form, and the extra exit on the other regime.
#[derive(Clone, Copy)]
struct DelayK {
    flt_bull: bool,
    running: bool,
    /// 0 none, 1 `OR fltbuy`, 2 `OR fltshort`
    extra: u8,
}
#[derive(Clone, Copy, PartialEq)]
enum PlusK {
    /// ids 10: `C>Ref(hcl,-1) AND C>trigger AND fltbuy`, no exit on regime
    Swing10,
    /// 11: bear; latch with trigger; exit when bull
    Bear11,
    /// 12: bear; latch without trigger; no exit on regime; OpenInt turnover
    Bear12,
    /// 13: bull; latch with trigger; UC / LC terms; exit when bear
    Bull13,
    /// 14: bull; latch without trigger; no exit on regime
    Bull14,
    /// 15: bear; runningbuy state machine (buysig 0/1/2); exit when bull
    Bear15,
}
#[derive(Clone, Copy, PartialEq)]
enum StK {
    /// buy only in bear: ATHCbuysig armed only while FLTSHORT (state 2 once bull)
    BuyOnlyIn,
    /// ATHCBUYSIG AND C>Ref(Hcl,-1) AND concount AND fltshort; sell trend / dead only
    EntryBear,
    /// the same with fltbuy
    EntryBull,
    /// bull; no January entries, sell on the last trading day of December
    Jan,
    /// bear entry; also sells when the index turns bull (`OR fltbuy`)
    ExitBear,
    /// bull; Buy = ATHCBUYSIG (no fresh-ATH close needed); UC / LC terms; exit when bear
    FinalBull,
    /// bear entry only (no exit on regime)
    EntryOnlyBear,
    /// bull; (ATHCBUYSIG OR fresh ATH close)
    RunningBull,
    /// verification aid only (`check_variants`): the AFL of `ath_loss1` `_loss_3` (bull, exit when bear, no circuit guard)
    RefBull,
}
#[derive(Clone, Copy, PartialEq)]
enum PlK {
    ProfitBook,
    Bear,
    Bull,
}
#[derive(Clone, Copy)]
enum Fam {
    /// ATHC +x % buy after consecutive %swing losses (ath_plus family 1): latch needs Close > trigger or not
    PlusLoss(bool),
    Delay(DelayK),
    PlusIdx(PlusK),
    St52(StK),
    PctLoss(PlK),
}

#[derive(Clone, Copy)]
struct Spec {
    fam: Fam,
    filt: Filt,
    turn: Turn,
    /// MKTFLT NONE: FLTBUY = true / FLTSHORT = false (bull) or the reverse (bear)
    none_bull: bool,
    /// swing-filter variants that also list FILTER TIMEFRAME / INDEX Filter (one entry each)
    lists: bool,
    /// None = `Optimize` in the AFL: not a parameter, constant `exit_fixed`
    exit: Option<R>,
    exit_fixed: f64,
    athc: Option<R>,
    athc_fixed: f64,
    delay: Option<R>,
    /// (MIN PROFIT % minimum, CONSICUTIVE LOSS range)
    loss: Option<(f64, R)>,
    loss_label: &'static str,
    st_list: bool,
    st_bpd: bool,
    st_bf: bool,
    bf2: R,
    bf3: R,
    f1: R,
    f2: R,
    pb: bool,
}

const BASE: Spec = Spec {
    fam: Fam::PlusLoss(false),
    filt: Filt::Swing,
    turn: Turn::Vc6,
    none_bull: true,
    lists: false,
    exit: None,
    exit_fixed: 10.0,
    athc: None,
    athc_fixed: 5.0,
    delay: None,
    loss: None,
    loss_label: "MIN PROFIT %",
    st_list: false,
    st_bpd: false,
    st_bf: false,
    bf2: r(2.0, 1.0, 20.0),
    bf3: r(4.0, 1.0, 20.0),
    f1: r(23.0, 10.0, 50.0),
    f2: r(44.0, 10.0, 50.0),
    pb: false,
};

fn schema_of(sp: &Spec) -> Vec<P> {
    let mut v = vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (ApplyStop percent loss; 0 = off)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
    ];
    if sp.st_list {
        v.push(p("stflt1", "ST CALCULATION POINT FOR PROBABILITY (0 MID POINT(H+L/2), 1 CLOSE, 2 PIVOT(H+L+C/3))", 0.0, 0.0, 2.0, 1.0));
    }
    if sp.st_bpd {
        v.push(p("bpd2", "ST ENTRY Period FOR PROBABILITY", 5.0, 1.0, 20.0, 1.0));
    }
    if sp.st_bf {
        let st = if sp.bf2.max > 10.0 { 1.0 } else { 0.5 };
        v.push(p("bfactor2", "ST ENTRY Factor FOR PROBABILITY", sp.bf2.def, sp.bf2.min, sp.bf2.max, st));
    }
    if sp.st_bpd {
        v.push(p("bpd3", "ST EXIT Period FOR PROBABILITY", 5.0, 1.0, 20.0, 1.0));
    }
    if sp.st_bf {
        let st = if sp.bf3.max > 10.0 { 1.0 } else { 0.5 };
        v.push(p("bfactor3", "ST EXIT Factor FOR PROBABILITY", sp.bf3.def, sp.bf3.min, sp.bf3.max, st));
    }
    v.push(p("mktflt", "FILTER TYPE (0 NONE, 1 INDEX)", 0.0, 0.0, 1.0, 1.0));
    if sp.filt == Filt::St {
        v.push(p("flttf", "FILTER TIMEFRAME (0 MONTHLY, 1 WEEKLY, 2 DAILY)", 0.0, 0.0, 2.0, 1.0));
    } else if sp.lists {
        v.push(p("flttf", "FILTER TIMEFRAME (0 DAILY)", 0.0, 0.0, 0.0, 1.0));
    }
    v.push(p("prflt", "INDEX (0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 0.0, 0.0, 3.0, 1.0));
    if sp.filt == Filt::St {
        v.push(p("iflt", "INDEX Filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0));
        v.push(p("istflt", "ST CALCULATION POINT (0 MID POINT(H+L/2), 1 CLOSE, 2 PIVOT(H+L+C/3))", 0.0, 0.0, 2.0, 1.0));
        v.push(p("ifltpd", "INDEX ST PERIOD", 20.0, 0.0, 20.0, 1.0));
        v.push(p("ifltfact", "INDEX ST FACTOR", 1.0, 0.0, 20.0, 1.0));
        v.push(p("ifltmapd", "INDEX SMA PERIOD", 200.0, 1.0, 300.0, 1.0));
    } else {
        if sp.lists {
            v.push(p("iflt", "INDEX Filter (0 SWING)", 0.0, 0.0, 0.0, 1.0));
        }
        v.push(p("fltfact1", "SWING % FOR BEAR MKT", sp.f1.def, sp.f1.min, sp.f1.max, 1.0));
        v.push(p("fltfact2", "SWING % FOR BULL MKT", sp.f2.def, sp.f2.min, sp.f2.max, 1.0));
    }
    if let Some(e) = sp.exit {
        v.push(p("exitpct", "EXIT %", e.def, e.min, e.max, 1.0));
    }
    if let Some(d) = sp.delay {
        v.push(p("delaybuy", "ATHC DELAY ENTRY", d.def, d.min, d.max, 1.0));
    }
    if let Some(a) = sp.athc {
        v.push(p("athcbuypct", "BUY PERCENTAGE FROM ATHC", a.def, a.min, a.max, 1.0));
    }
    if let Some((lmin, n)) = sp.loss {
        v.push(p("losspct", sp.loss_label, 2.0, lmin, 10.0, 0.05));
        v.push(p("noofloss", "CONSICUTIVE LOSS", n.def, n.min, n.max, 1.0));
    }
    if sp.pb {
        v.push(p("pb", "PROFIT BOOKING Percent", 5.0, 0.0, 50.0, 5.0));
        v.push(p("pbbuff", "PROFIT BOOKING BUFFER", 0.25, 0.0, 5.0, 0.05));
        v.push(p("pbnull", "ENGINE OPTION: how AmiBroker compares with Null in the profit-booking test `H > PT` (0 = Null compares false, as in the other ATH engines; 1 = Null is the number -1e10, i.e. H > Null is true)", 0.0, 0.0, 1.0, 1.0));
    }
    if matches!(sp.fam, Fam::PlusLoss(_) | Fam::St52(_) | Fam::PctLoss(_)) {
        v.push(p("comm", "BROKERAGE % used by the AFL's loss counter (follows the run's brokerage)", 0.65, 0.01, 1.0, 0.01));
    }
    if sp.filt == Filt::St {
        v.push(p("pad", "ENGINE OPTION (Analysis setting, not an AFL parameter): 'Pad and align to the reference symbol' (1 yes; 0 no = the index is aligned to each stock's own bars)", 0.0, 0.0, 1.0, 1.0));
        v.push(p("padref", "ENGINE OPTION: reference symbol of Pad and align (-1 = the filter index, 0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 1.0, -1.0, 3.0, 1.0));
        v.push(p("padcut", "ENGINE OPTION: with Pad, also cut a stock's history before the reference symbol's first bar (1, as ath_plus / the combo sheets) or keep it (0)", 1.0, 0.0, 1.0, 1.0));
    }
    v
}

/// The AFL parameters of a run.
struct Pm {
    exit: f32,
    athc: f32,
    delay: f32,
    losspct: f32,
    noofloss: f32,
    comm: f32,
    stcal: u8,
    bpd2: usize,
    bf2: f32,
    bpd3: usize,
    bf3: f32,
    pb: f32,
    pbbuff: f32,
    /// engine option of the profit-booking variant: Null compares as the number -1e10 (so `H > Null` is true) instead of false
    pbnull: bool,
    penny: f32,
    dead_yes: bool,
    turn: bool,
}

fn pm_of(g: G, sp: &Spec) -> Pm {
    Pm {
        exit: (if sp.exit.is_some() { g("exitpct") } else { sp.exit_fixed }) as f32,
        athc: (if sp.athc.is_some() { g("athcbuypct") } else { sp.athc_fixed }) as f32,
        delay: if sp.delay.is_some() { g("delaybuy") as f32 } else { 0.0 },
        losspct: if sp.loss.is_some() { g("losspct") as f32 } else { 2.0 },
        noofloss: if sp.loss.is_some() { g("noofloss") as f32 } else { 0.0 },
        comm: g("comm") as f32,
        stcal: if sp.st_list { g("stflt1").round().clamp(0.0, 2.0) as u8 } else { 0 },
        bpd2: if sp.st_bpd { g("bpd2").round().max(1.0) as usize } else { 5 },
        bf2: if sp.st_bf { g("bfactor2") as f32 } else { 2.0 },
        bpd3: if sp.st_bpd { g("bpd3").round().max(1.0) as usize } else { 5 },
        bf3: if sp.st_bf { g("bfactor3") as f32 } else { 4.0 },
        pb: if sp.pb { g("pb") as f32 } else { 0.0 },
        pbbuff: if sp.pb { g("pbbuff") as f32 } else { 0.0 },
        pbnull: sp.pb && g("pbnull") != 0.0,
        penny: g("pennyflt") as f32,
        dead_yes: g("deadstock").round() == 0.0,
        turn: g("turnflt").round() == 1.0,
    }
}

/// Comparison with Null on either side is false (AmiBroker, as in the verified engines).
fn gt(a: f32, b: f32) -> bool {
    !is_null(a) && !is_null(b) && a > b
}

/// Everything the families share, per symbol.
struct Pre {
    n: usize,
    c: Vec<f32>,
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    dates: Vec<i32>,
    deadbuy: Vec<bool>,
    deadsell: Vec<bool>,
    dead1: Vec<bool>,
    upc: Vec<bool>,
    loc: Vec<bool>,
    uc: Vec<bool>,
    lc: Vec<bool>,
    turnover: Vec<bool>,
    hcl: Vec<f32>,
    sw: Sw,
    fltbuy: Vec<bool>,
    fltshort: Vec<bool>,
    penny: f32,
}

/// (FLTBuy, FLTShort) of the AFL on the stock's bar dates.
fn index_flags(dates: &[i32], idx: Option<&Series>, refc: Option<&Series>, g: G, sp: &Spec, pad: bool) -> (Vec<bool>, Vec<bool>) {
    let n = dates.len();
    if g("mktflt").round() != 1.0 {
        return (vec![sp.none_bull; n], vec![!sp.none_bull; n]);
    }
    // the index is missing from the database (e.g. NIFTY): the filter is never true, no entries
    let Some(ix) = idx else { return (vec![false; n], vec![false; n]) };
    match sp.filt {
        Filt::Swing => {
            // index %swing: bear when Close < Dn (-fltfact1 %), bull when Close > Up (+fltfact2 %) or a new index ATH close
            let (f1, f2) = (g("fltfact1") as f32, g("fltfact2") as f32);
            let al = align(ix, dates);
            let hcl_i = hhv_all(&al.c);
            let br: Vec<f32> = al.c.iter().map(|&x| (x * f1) / 100.0).collect();
            let bl: Vec<f32> = al.c.iter().map(|&x| (x * f2) / 100.0).collect();
            let up: Vec<f32> = (0..n).map(|i| al.c[i] + bl[i]).collect();
            let dn: Vec<f32> = (0..n).map(|i| al.c[i] - br[i]).collect();
            let uf: Vec<f32> = (0..n).map(|i| hcl_i[i].min(al.c[i] + bl[i])).collect();
            let df: Vec<f32> = (0..n).map(|i| al.c[i] - br[i]).collect();
            let sw = swing_loop(&al.c, up, dn, &uf, &df, Some(&hcl_i));
            (sw.trend.iter().map(|&t| t == 1).collect(), sw.trend.iter().map(|&t| t == -1).collect())
        }
        Filt::St => {
            let f = flt_from(g);
            match (pad, refc) {
                (true, Some(rc)) => st_filter_padded(ix, rc, dates, &f),
                _ => st_filter(&align(ix, dates), dates, &f),
            }
        }
    }
}

fn build(s: &Series, idx: Option<&Series>, refc: Option<&Series>, g: G, sp: &Spec, pm: &Pm, pad: bool) -> Pre {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
    let (deadbuy, deadsell) = dead_flags(n, pm.dead_yes);
    let dead1 = flat30(&o, &h, &l, &c);
    let (upc, loc) = circuits(&o, &h, &l, &c);
    let uc: Vec<bool> = (0..n).map(|i| h[i] == c[i]).collect();
    let lc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let turnover: Vec<bool> = (0..n)
        .map(|i| {
            if !pm.turn {
                return true;
            }
            let y = dates[i] / 10000;
            match sp.turn {
                Turn::Vc6 => s.bars[i].v * c[i] > volsig_loss(y),
                Turn::OiStd => s.bars[i].oi > volsig_std(y),
            }
        })
        .collect();
    let hcl = hhv_all(&c);
    let sw = pct_swing(&c, pm.exit);
    let (fltbuy, fltshort) = index_flags(&dates, idx, refc, g, sp, pad);
    Pre { n, c, o, h, l, dates, deadbuy, deadsell, dead1, upc, loc, uc, lc, turnover, hcl, sw, fltbuy, fltshort, penny: pm.penny }
}

fn new_high(pre: &Pre, i: usize) -> bool {
    i >= 1 && pre.c[i] > pre.hcl[i - 1]
}

/// Loss streak of the virtual %swing trades (in on `buysig`, out on trend -1): copy of the ath_plus counter.
fn concount_pct(pre: &Pre, buysig: &[bool], losspct: f32, comm: f32) -> Vec<i32> {
    let n = pre.n;
    let (c, sw) = (&pre.c, &pre.sw);
    let trend1 = &sw.trend;
    let mut bcount = vec![0i32; n];
    let mut bprice = vec![NULL; n];
    let mut sprice = vec![NULL; n];
    for i in 1..n {
        if buysig[i] && bcount[i - 1] < 1 {
            bprice[i] = c[i];
            sprice[i] = sprice[i - 1];
            bcount[i] = bcount[i - 1] + 1;
        } else if trend1[i] == -1 && bcount[i - 1] == 1 {
            sprice[i] = c[i];
            bprice[i] = bprice[i - 1];
            bcount[i] = 0;
        } else {
            bprice[i] = bprice[i - 1];
            sprice[i] = sprice[i - 1];
            bcount[i] = bcount[i - 1];
        }
    }
    let mut concount = vec![0i32; n];
    let a = |i: usize| -> f32 { if i >= 1 { sw.tup[i - 1] } else { NULL } }; // Ref(TrendUp1, -1)
    for i in 1..n {
        let cross = a(i) > c[i] && a(i - 1) <= c[i - 1];
        let (mut profit, mut pct) = (0f32, 0f32);
        if cross && bcount[i - 1] == 1 {
            let (sp, bp) = (sprice[i], bprice[i]);
            profit = (sp - (sp * comm) / 100.0) - (bp + (bp * comm) / 100.0);
            pct = (((sp - (sp * comm) / 100.0) - (bp + (bp * comm) / 100.0)) / bp) * 100.0;
        }
        concount[i] = if profit != 0.0 {
            if pct <= losspct { concount[i - 1] + 1 } else { 0 }
        } else {
            concount[i - 1]
        };
    }
    concount
}

/// Loss streak of the virtual Supertrend(bpd2, bf2) in / both Supertrends down out trades (copy of ath_loss1 `base`).
fn concount_st(pre: &Pre, pm: &Pm) -> Vec<f32> {
    let n = pre.n;
    let c = &pre.c;
    let sc = stcal_of(pm.stcal, &pre.h, &pre.l, c);
    let (up2, _d2, bt2) = supertrend(&sc, &pre.h, &pre.l, c, pm.bpd2, pm.bf2);
    let (up3, _d3, bt3) = supertrend(&sc, &pre.h, &pre.l, c, pm.bpd3, pm.bf3);
    let buysig: Vec<bool> = bt2.iter().map(|&t| t == 1).collect();
    let sellsig: Vec<bool> = (0..n).map(|i| bt2[i] == -1 && bt3[i] == -1).collect();
    let mut bcount = vec![0i32; n];
    let mut bprice = vec![NULLV; n];
    let mut sprice = vec![NULLV; n];
    for i in 1..n {
        if buysig[i] && bcount[i - 1] < 1 {
            bprice[i] = c[i];
            sprice[i] = sprice[i - 1];
            bcount[i] = bcount[i - 1] + 1;
        } else if sellsig[i] && bcount[i - 1] == 1 {
            sprice[i] = c[i];
            bprice[i] = bprice[i - 1];
            bcount[i] = 0;
        } else {
            bprice[i] = bprice[i - 1];
            sprice[i] = sprice[i - 1];
            bcount[i] = bcount[i - 1];
        }
    }
    let cr2 = cross_line_below_close(&up2, c);
    let cr3 = cross_line_below_close(&up3, c);
    let mut concount = vec![0f32; n];
    for i in 1..n {
        let cond = (bt3[i] == -1 && cr2[i]) || (bt3[i - 1] == 1 && cr3[i]);
        let (mut profit, mut pct) = (0f32, 0f32);
        if cond {
            if bprice[i] == NULLV || sprice[i] == NULLV {
                // Null in the arithmetic: the profit stays Null (non-zero) and counts as a loss
                profit = NULLV;
                pct = NULLV;
            } else {
                let (sp, bp) = (sprice[i], bprice[i]);
                profit = (sp - (sp * pm.comm) / 100.0) - (bp + (bp * pm.comm) / 100.0);
                pct = ((sp - (sp * pm.comm) / 100.0) - (bp + (bp * pm.comm) / 100.0)) / bp * 100.0;
            }
        }
        concount[i] = if profit != 0.0 {
            if pct <= pm.losspct { concount[i - 1] + 1.0 } else { 0.0 }
        } else {
            concount[i - 1]
        };
    }
    concount
}

// ---------------------------------------------------------------- family: ATHC +x % buy after %swing losses (ids 1, 2)

fn fam_plus_loss(pre: &Pre, pm: &Pm, latch_trigger: bool) -> (Vec<bool>, Vec<bool>) {
    let n = pre.n;
    let c = &pre.c;
    let trend1 = &pre.sw.trend;
    let trigger = trigger_latch(c, &pre.hcl, trend1, pm.athc);
    let athc = athc_latch(c, &pre.hcl, trend1, if latch_trigger { Some(&trigger) } else { None });
    let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !pre.upc[i] && pre.deadbuy[i] && c[i] > pre.penny && pre.turnover[i]).collect();
    let cc = concount_pct(pre, &buysig, pm.losspct, pm.comm);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = athc[i]
            && (latch_trigger || c[i] > trigger[i])
            && cc[i] as f32 >= pm.noofloss
            && pre.fltbuy[i]
            && !pre.dead1[i]
            && !pre.upc[i]
            && pre.deadbuy[i]
            && c[i] > pre.penny
            && pre.turnover[i];
        sell[i] = (trend1[i] == -1 && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i];
    }
    (buy, sell)
}

// ---------------------------------------------------------------- family: delay entry (ids 4-9)

fn fam_delay(pre: &Pre, pm: &Pm, k: DelayK) -> (Vec<bool>, Vec<bool>) {
    let n = pre.n;
    let (c, hcl, trend1) = (&pre.c, &pre.hcl, &pre.sw.trend);
    // buysig / athccount: the count restarts at 0 when the swing turns down while the latch is on
    let mut buysig = vec![false; n];
    let mut cnt = vec![0i32; n];
    for i in 1..n {
        if c[i] > hcl[i - 1] {
            buysig[i] = true;
            cnt[i] = cnt[i - 1] + 1;
        } else if trend1[i] == -1 && buysig[i - 1] {
            buysig[i] = false;
        } else {
            buysig[i] = buysig[i - 1];
            cnt[i] = cnt[i - 1];
        }
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let flt = if k.flt_bull { pre.fltbuy[i] } else { pre.fltshort[i] };
        let common = cnt[i] as f32 > pm.delay && !pre.upc[i] && !pre.dead1[i] && pre.deadbuy[i] && c[i] > pre.penny && pre.turnover[i];
        if k.running {
            buy[i] = (new_high(pre, i) || buysig[i]) && common && pre.fltbuy[i];
            sell[i] = ((trend1[i] == -1 || !buysig[i]) && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i];
        } else {
            buy[i] = flt && buysig[i] && common;
            sell[i] = (trend1[i] == -1 && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i] || (k.extra == 1 && pre.fltbuy[i]) || (k.extra == 2 && pre.fltshort[i]);
        }
    }
    (buy, sell)
}

// ---------------------------------------------------------------- family: ATHC +x % buy with the index filter (ids 10-15)

fn fam_plus_idx(pre: &Pre, pm: &Pm, k: PlusK) -> (Vec<bool>, Vec<bool>) {
    let n = pre.n;
    let (c, hcl, trend1) = (&pre.c, &pre.hcl, &pre.sw.trend);
    let trigger = trigger_latch(c, hcl, trend1, pm.athc);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let base_ok = |i: usize| pre.deadbuy[i] && !pre.dead1[i] && c[i] > pre.penny && pre.turnover[i];
    match k {
        PlusK::Swing10 => {
            for i in 0..n {
                buy[i] = new_high(pre, i) && gt(c[i], trigger[i]) && pre.fltbuy[i] && base_ok(i) && !pre.upc[i];
                sell[i] = (trend1[i] == -1 && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i];
            }
        }
        PlusK::Bear11 | PlusK::Bear12 | PlusK::Bull13 | PlusK::Bull14 => {
            let with_trigger = matches!(k, PlusK::Bear11 | PlusK::Bull13);
            let latch = athc_latch(c, hcl, trend1, if with_trigger { Some(&trigger) } else { None });
            for i in 0..n {
                let bear = matches!(k, PlusK::Bear11 | PlusK::Bear12);
                let flt = if bear { pre.fltshort[i] } else { pre.fltbuy[i] };
                let other = if bear { pre.fltbuy[i] } else { pre.fltshort[i] };
                let uc_terms = k == PlusK::Bull13;
                buy[i] = (latch[i] || new_high(pre, i)) && gt(c[i], trigger[i]) && flt && base_ok(i) && !pre.upc[i] && !(uc_terms && pre.uc[i]);
                let guard = !pre.loc[i] && !(uc_terms && pre.lc[i]);
                sell[i] = ((trend1[i] == -1 || !latch[i]) && guard) || pre.deadsell[i] || pre.dead1[i] || (matches!(k, PlusK::Bear11 | PlusK::Bull13) && other && guard);
            }
        }
        PlusK::Bear15 => {
            // buysig 0 / 1 / 2 and the runningbuy counter, as the AFL's loop
            let mut buysig = vec![0u8; n];
            let mut running = vec![0i32; n];
            for i in 1..n {
                running[i] = if c[i] > hcl[i - 1] && gt(c[i], trigger[i]) { running[i - 1] + 1 } else { running[i - 1] };
                if c[i] > hcl[i - 1] && pre.fltshort[i] && gt(c[i], trigger[i]) && buysig[i - 1] != 2 && running[i] == 1 {
                    buysig[i] = 1;
                    running[i] = running[i - 1];
                } else if trend1[i] == -1 {
                    buysig[i] = 0;
                    running[i] = 0;
                } else if pre.fltbuy[i] && buysig[i - 1] == 1 && trend1[i] == 1 {
                    buysig[i] = 2;
                    running[i] = running[i - 1];
                } else {
                    buysig[i] = buysig[i - 1];
                }
            }
            for i in 0..n {
                buy[i] = buysig[i] == 1 && gt(c[i], trigger[i]) && pre.fltshort[i] && base_ok(i) && !pre.upc[i] && !pre.uc[i];
                let guard = !pre.loc[i] && !pre.lc[i];
                sell[i] = ((trend1[i] == -1 || buysig[i] == 0) && guard) || pre.deadsell[i] || pre.dead1[i] || (pre.fltbuy[i] && guard);
            }
        }
    }
    (buy, sell)
}

// ---------------------------------------------------------------- family: ATHC after ST(5,2)/(5,4) loss trades (ids 3, 16-22)

fn fam_st52(pre: &Pre, pm: &Pm, k: StK) -> (Vec<bool>, Vec<bool>) {
    let n = pre.n;
    let (c, hcl, trend1) = (&pre.c, &pre.hcl, &pre.sw.trend);
    let cc = concount_st(pre, pm);
    let athc = athc_latch(c, hcl, trend1, None);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    if k == StK::BuyOnlyIn {
        // ATHCbuysig: 1 on a fresh ATH close while bear (and not yet "2"), else 0 / 2 once the swing is down or the index is bull
        let mut st = vec![0u8; n];
        for i in 1..n {
            if c[i] > hcl[i - 1] && pre.fltshort[i] && st[i - 1] != 2 {
                st[i] = 1;
            } else if trend1[i] == -1 || pre.fltbuy[i] {
                st[i] = if pre.fltbuy[i] { 2 } else { 0 };
            } else {
                st[i] = st[i - 1];
            }
        }
        for i in 0..n {
            buy[i] = (st[i] == 1 || new_high(pre, i))
                && cc[i] >= pm.noofloss
                && pre.fltshort[i]
                && !pre.upc[i]
                && !pre.uc[i]
                && pre.deadbuy[i]
                && c[i] > pre.penny
                && pre.turnover[i]
                && !pre.dead1[i];
            let guard = !pre.loc[i] && !pre.lc[i];
            sell[i] = ((trend1[i] == -1 || st[i] == 0) && guard) || pre.deadsell[i] || pre.dead1[i] || (pre.fltbuy[i] && guard);
        }
        return (buy, sell);
    }
    for i in 0..n {
        let common = pre.deadbuy[i] && c[i] > pre.penny && pre.turnover[i] && !pre.dead1[i];
        let loss_ok = cc[i] >= pm.noofloss;
        match k {
            StK::EntryBear | StK::ExitBear | StK::EntryOnlyBear | StK::EntryBull => {
                let flt = if k == StK::EntryBull { pre.fltbuy[i] } else { pre.fltshort[i] };
                buy[i] = athc[i] && new_high(pre, i) && loss_ok && flt && !pre.upc[i] && common;
                sell[i] = (trend1[i] == -1 && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i] || (k == StK::ExitBear && pre.fltbuy[i]);
            }
            StK::Jan => {
                let month = pre.dates[i] / 100 % 100;
                let last_day_dec = month == 12 && i + 1 < n && pre.dates[i + 1] % 100 < pre.dates[i] % 100;
                buy[i] = athc[i] && new_high(pre, i) && loss_ok && pre.fltbuy[i] && month != 1 && !last_day_dec && !pre.upc[i] && common;
                sell[i] = trend1[i] == -1 || pre.deadsell[i] || last_day_dec || pre.dead1[i];
            }
            StK::FinalBull => {
                buy[i] = athc[i] && loss_ok && pre.fltbuy[i] && !pre.upc[i] && !pre.uc[i] && common;
                let guard = !pre.loc[i] && !pre.lc[i];
                sell[i] = ((trend1[i] == -1 || !athc[i]) && guard) || pre.deadsell[i] || pre.dead1[i] || (pre.fltshort[i] && guard);
            }
            StK::RunningBull => {
                buy[i] = (athc[i] || new_high(pre, i)) && loss_ok && pre.fltbuy[i] && !pre.upc[i] && common;
                sell[i] = ((trend1[i] == -1 || !athc[i]) && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i];
            }
            StK::RefBull => {
                buy[i] = athc[i] && new_high(pre, i) && loss_ok && pre.fltbuy[i] && !pre.upc[i] && common;
                sell[i] = trend1[i] == -1 || pre.deadsell[i] || pre.dead1[i] || pre.fltshort[i];
            }
            StK::BuyOnlyIn => {}
        }
    }
    (buy, sell)
}

// ---------------------------------------------------------------- family: ATHC after %swing losses with the index filter (ids 23-25)

/// (buy, sell, sell price override) of the %swing-loss family.
fn fam_pct_loss(pre: &Pre, pm: &Pm, k: PlK) -> (Vec<bool>, Vec<bool>, Option<Vec<f32>>) {
    let n = pre.n;
    let (c, hcl, trend1) = (&pre.c, &pre.hcl, &pre.sw.trend);
    let athc = athc_latch(c, hcl, trend1, None);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    if k != PlK::ProfitBook {
        let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !pre.upc[i] && pre.deadbuy[i] && c[i] > pre.penny && pre.turnover[i]).collect();
        let cc = concount_pct(pre, &buysig, pm.losspct, pm.comm);
        for i in 0..n {
            let flt = if k == PlK::Bull { pre.fltbuy[i] } else { pre.fltshort[i] };
            buy[i] = buysig[i] && cc[i] as f32 >= pm.noofloss && flt && !pre.dead1[i];
            sell[i] = (trend1[i] == -1 && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i];
        }
        return (buy, sell, None);
    }
    // profit booking (bear): ATHCbuysig / concount without penny and turnover, then the PT / pbexit / buysig1 state loop
    let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !pre.upc[i] && !pre.uc[i] && pre.deadbuy[i]).collect();
    let cc = concount_pct(pre, &buysig, pm.losspct, pm.comm);
    let buyfilter: Vec<bool> = (0..n).map(|i| pre.turnover[i] && !pre.uc[i] && !pre.upc[i] && c[i] > pre.penny && pre.deadbuy[i]).collect();
    let target = |i: usize| (1.0 + pm.pb * 0.01) * c[i];
    let gtp = |a: f32, b: f32| if pm.pbnull { a > b } else { gt(a, b) };
    let mut buysig1 = vec![false; n];
    let mut pbexit = vec![false; n];
    let mut pt = vec![NULL; n];
    let mut ptcheck = vec![true; n];
    for i in 1..n {
        ptcheck[i] = is_null(pt[i - 1]) && buyfilter[i];
        if c[i] > pre.hcl[i - 1] && pre.fltshort[i] && !pbexit[i - 1] {
            buysig1[i] = true;
            pt[i] = if ptcheck[i] { target(i) } else { pt[i - 1] };
            if gtp(pre.h[i], pt[i - 1]) {
                buysig1[i] = false;
                pt[i] = NULL;
                pbexit[i] = true;
            }
        } else if buysig1[i - 1] && ptcheck[i] {
            pt[i] = target(i);
            buysig1[i] = buysig1[i - 1];
        } else if gtp(pre.h[i], pt[i - 1]) {
            pt[i] = NULL;
            pbexit[i] = true;
        } else if trend1[i] == -1 || pre.fltbuy[i] {
            buysig1[i] = false;
            pbexit[i] = false;
        } else {
            buysig1[i] = buysig1[i - 1];
            pt[i] = pt[i - 1];
            pbexit[i] = pbexit[i - 1];
        }
    }
    let mut spx = c.clone();
    for i in 0..n {
        buy[i] = buysig1[i] && cc[i] as f32 >= pm.noofloss && pre.fltshort[i] && !pre.dead1[i] && !pre.upc[i] && !pre.uc[i] && pre.deadbuy[i] && c[i] > pre.penny && pre.turnover[i];
        let guard = !pre.loc[i] && !pre.lc[i];
        // `AND pbexit=1` is an assignment in AFL (pbexit becomes 1 for every bar): the profit-booking term is H > PT of the bar
        sell[i] = ((trend1[i] == -1 || !buysig1[i]) && guard) || pre.deadsell[i] || pre.dead1[i] || (pre.fltbuy[i] && guard) || gtp(pre.h[i], pt[i]);
        if i >= 1 && gtp(pre.h[i], pt[i - 1]) {
            spx[i] = if gtp(pre.o[i], pt[i - 1]) { pre.o[i] - (pre.o[i] * pm.pbbuff) / 100.0 } else { pt[i - 1] };
        }
    }
    (buy, sell, Some(spx))
}

// ---------------------------------------------------------------- one symbol / the universe

fn finish(pre: &Pre, buy: Vec<bool>, sell: Vec<bool>, spx: Option<Vec<f32>>) -> Signals {
    let b2 = exrem(&buy, &sell);
    let s2 = exrem(&sell, &b2);
    let mut sg = Signals::new(b2, s2, pre.c.clone(), spx.unwrap_or_else(|| pre.c.clone()));
    clamp_prices(&mut sg, &pre.h, &pre.l);
    sg
}

fn symbol_inner(s: &Series, idx: Option<&Series>, refc: Option<&Series>, g: G, sp: &Spec, pm: &Pm, pad: bool) -> Signals {
    let pre = build(s, idx, refc, g, sp, pm, pad);
    let (buy, sell, spx) = match sp.fam {
        Fam::PlusLoss(lt) => {
            let (b, s) = fam_plus_loss(&pre, pm, lt);
            (b, s, None)
        }
        Fam::Delay(k) => {
            let (b, s) = fam_delay(&pre, pm, k);
            (b, s, None)
        }
        Fam::PlusIdx(k) => {
            let (b, s) = fam_plus_idx(&pre, pm, k);
            (b, s, None)
        }
        Fam::St52(k) => {
            let (b, s) = fam_st52(&pre, pm, k);
            (b, s, None)
        }
        Fam::PctLoss(k) => fam_pct_loss(&pre, pm, k),
    };
    finish(&pre, buy, sell, spx)
}

/// "Pad and align to the reference symbol" also cuts a stock's history to the reference's first bar (see ath_plus.rs / combo RESULT.md).
fn symbol(s: &Series, idx: Option<&Series>, refc: Option<&Series>, g: G, sp: &Spec, pm: &Pm, pad: bool) -> Signals {
    if let (true, true, Some(rc)) = (pad, g("padcut") != 0.0, refc) {
        let first = rc.bars.first().map_or(0, |b| b.date);
        let cut = s.bars.partition_point(|b| b.date < first);
        if cut > 0 {
            let n = s.bars.len();
            let mut out = blank(n);
            for i in 0..n {
                out.buy_price[i] = s.bars[i].c;
                out.sell_price[i] = s.bars[i].c;
            }
            if cut >= n {
                return out;
            }
            let sub = Series { name: s.name.clone(), bars: s.bars[cut..].to_vec() };
            let inner = symbol_inner(&sub, idx, refc, g, sp, pm, pad);
            for i in 0..sub.bars.len() {
                out.buy[cut + i] = inner.buy[i];
                out.sell[cut + i] = inner.sell[i];
                out.buy_price[cut + i] = inner.buy_price[i];
                out.sell_price[cut + i] = inner.sell_price[i];
            }
            return out;
        }
    }
    symbol_inner(s, idx, refc, g, sp, pm, pad)
}

fn run(series: &[Series], env: &Env, g: G, sp: &Spec) -> Vec<Signals> {
    let pm = pm_of(g, sp);
    let idx = index_of(env, g);
    let pad = sp.filt == Filt::St && g("pad") != 0.0 && g("mktflt").round() == 1.0;
    let refc = {
        let r = g("padref").round();
        if r < 0.0 { idx } else { env.get(INDEX_NAMES[(r as usize).min(3)]) }
    };
    let hard = g("hardstop") as f32;
    series
        .iter()
        .map(|s| {
            let mut sg = symbol(s, idx, refc, g, sp, &pm, pad);
            if hard > 0.0 {
                sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
            }
            sg
        })
        .collect()
}

// ---------------------------------------------------------------- the 25 variants

macro_rules! variant {
    ($spec:ident, $sig:ident, $sch:ident) => {
        fn $sig(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
            run(series, env, g, &$spec)
        }
        fn $sch() -> Vec<P> {
            schema_of(&$spec)
        }
    };
}

const LOSS1: Option<(f64, R)> = Some((0.5, r(1.0, 1.0, 50.0)));
const LOSS2: Option<(f64, R)> = Some((0.5, r(2.0, 1.0, 50.0)));
const LOSS02: Option<(f64, R)> = Some((0.5, r(2.0, 0.0, 2.0)));

// 1, 2: ATHC +5 % buy, %swing exit after consecutive loss, bull index %swing filter (code identical to ath_plus after / after_4)
const S01: Spec = Spec { fam: Fam::PlusLoss(false), exit: Some(r(10.0, 5.0, 25.0)), athc: Some(r(5.0, 1.0, 10.0)), loss: LOSS1, ..BASE };
const S02: Spec = Spec { fam: Fam::PlusLoss(true), ..S01 };
// 3: buy only in bear, ST(5,2)/(5,4) losses, index ST 20/1 (Optimize: exit 10, factors 2 / 4)
const S03: Spec = Spec { fam: Fam::St52(StK::BuyOnlyIn), filt: Filt::St, turn: Turn::OiStd, none_bull: false, st_list: true, st_bpd: true, loss: Some((0.5, r(2.0, 0.0, 2.0))), exit_fixed: 10.0, ..BASE };
// 4-9: delay entry
const S04: Spec = Spec { fam: Fam::Delay(DelayK { flt_bull: false, running: false, extra: 0 }), none_bull: false, exit: Some(r(10.0, 5.0, 35.0)), delay: Some(r(1.0, 1.0, 10.0)), ..BASE };
const S05: Spec = Spec { fam: Fam::Delay(DelayK { flt_bull: true, running: false, extra: 0 }), none_bull: true, ..S04 };
const S06: Spec = Spec { fam: Fam::Delay(DelayK { flt_bull: false, running: false, extra: 0 }), filt: Filt::St, none_bull: false, ..S04 };
const S07: Spec = Spec { fam: Fam::Delay(DelayK { flt_bull: true, running: true, extra: 0 }), filt: Filt::St, none_bull: true, delay: Some(r(1.0, 0.0, 10.0)), ..S04 };
const S08: Spec = Spec { fam: Fam::Delay(DelayK { flt_bull: false, running: false, extra: 1 }), filt: Filt::St, none_bull: false, ..S04 };
const S09: Spec = Spec { fam: Fam::Delay(DelayK { flt_bull: true, running: false, extra: 2 }), filt: Filt::St, none_bull: true, exit: Some(r(10.0, 5.0, 25.0)), delay: Some(r(1.0, 1.0, 2.0)), ..S04 };
// 10-15: +x % buy with index filter
const S10: Spec = Spec { fam: Fam::PlusIdx(PlusK::Swing10), none_bull: true, exit: Some(r(16.0, 5.0, 35.0)), athc: Some(r(5.0, 1.0, 30.0)), ..BASE };
const S11: Spec = Spec { fam: Fam::PlusIdx(PlusK::Bear11), filt: Filt::St, none_bull: false, exit: Some(r(16.0, 5.0, 25.0)), athc: Some(r(5.0, 1.0, 10.0)), ..BASE };
const S12: Spec = Spec { fam: Fam::PlusIdx(PlusK::Bear12), turn: Turn::OiStd, ..S11 };
const S13: Spec = Spec { fam: Fam::PlusIdx(PlusK::Bull13), turn: Turn::OiStd, none_bull: true, exit: Some(r(20.0, 5.0, 22.0)), athc: Some(r(3.0, 1.0, 10.0)), ..S11 };
const S14: Spec = Spec { fam: Fam::PlusIdx(PlusK::Bull14), none_bull: true, exit: Some(r(15.0, 15.0, 18.0)), athc: Some(r(4.0, 4.0, 5.0)), ..S11 };
const S15: Spec = Spec { fam: Fam::PlusIdx(PlusK::Bear15), turn: Turn::OiStd, exit: None, exit_fixed: 16.0, athc: None, athc_fixed: 5.0, ..S11 };
// 16-22: ST(5,2)/(5,4) loss gate
const S16: Spec = Spec { fam: Fam::St52(StK::EntryBear), none_bull: false, lists: true, st_list: true, st_bpd: true, st_bf: true, exit: Some(r(10.0, 5.0, 35.0)), loss: LOSS2, ..BASE };
const S17: Spec = Spec { fam: Fam::St52(StK::EntryBull), none_bull: true, ..S16 };
const S18: Spec = Spec { fam: Fam::St52(StK::Jan), none_bull: true, ..S16 };
const S19: Spec = Spec { fam: Fam::St52(StK::ExitBear), filt: Filt::St, lists: false, none_bull: false, st_list: true, st_bpd: false, st_bf: false, exit: None, exit_fixed: 10.0, loss: Some((0.5, r(2.0, 2.0, 2.0))), ..S16 };
const S20: Spec = Spec { fam: Fam::St52(StK::FinalBull), filt: Filt::St, lists: false, turn: Turn::OiStd, none_bull: true, st_list: true, st_bpd: true, st_bf: false, exit: None, exit_fixed: 11.0, loss: LOSS02, ..S16 };
const S21: Spec = Spec { fam: Fam::St52(StK::EntryOnlyBear), filt: Filt::St, lists: false, none_bull: false, ..S16 };
const S22: Spec = Spec { fam: Fam::St52(StK::RunningBull), filt: Filt::St, lists: false, none_bull: true, bf2: r(2.0, 1.0, 4.0), bf3: r(4.0, 1.0, 6.0), exit: Some(r(10.0, 5.0, 25.0)), loss: LOSS02, ..S16 };
// 23-25: %swing-loss gate
const S23: Spec = Spec { fam: Fam::PctLoss(PlK::ProfitBook), filt: Filt::St, turn: Turn::OiStd, none_bull: false, exit: Some(r(10.0, 5.0, 25.0)), loss: Some((0.5, r(2.0, 0.0, 50.0))), loss_label: "MIN LOSS %", pb: true, ..BASE };
const S24: Spec = Spec { fam: Fam::PctLoss(PlK::Bear), none_bull: false, lists: true, exit: Some(r(10.0, 5.0, 35.0)), loss: LOSS2, ..BASE };
const S25: Spec = Spec { fam: Fam::PctLoss(PlK::Bull), none_bull: true, ..S24 };

// verification aid (NOT registered): the AFL of ath_loss1 `_loss_3`, to compare the shared machinery signal by signal
const SREF: Spec = Spec { fam: Fam::St52(StK::RefBull), filt: Filt::St, none_bull: true, st_list: true, st_bpd: true, st_bf: true, bf2: r(2.0, 1.0, 6.0), bf3: r(4.0, 1.0, 6.0), exit: Some(r(10.0, 1.0, 25.0)), loss: Some((0.5, r(2.0, 1.0, 50.0))), ..BASE };
fn sig_ref(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    run(series, env, g, &SREF)
}
fn sch_ref() -> Vec<P> {
    schema_of(&SREF)
}
/// Not part of the registry: variants that only exist to compare this core with the verified engines (see `ATH/v2/*/VERIFY.md`).
pub fn check_variants() -> Vec<VariantDef> {
    vec![VariantDef { id: "a1_check_loss3_reference", data: Data::Cash22 { watchlist: "NSE 500 C&P.tls" }, schema: sch_ref, signals: sig_ref }]
}

variant!(S01, sig01, sch01);
variant!(S02, sig02, sch02);
variant!(S03, sig03, sch03);
variant!(S04, sig04, sch04);
variant!(S05, sig05, sch05);
variant!(S06, sig06, sch06);
variant!(S07, sig07, sch07);
variant!(S08, sig08, sch08);
variant!(S09, sig09, sch09);
variant!(S10, sig10, sch10);
variant!(S11, sig11, sch11);
variant!(S12, sig12, sch12);
variant!(S13, sig13, sch13);
variant!(S14, sig14, sch14);
variant!(S15, sig15, sch15);
variant!(S16, sig16, sch16);
variant!(S17, sig17, sch17);
variant!(S18, sig18, sch18);
variant!(S19, sig19, sch19);
variant!(S20, sig20, sch20);
variant!(S21, sig21, sch21);
variant!(S22, sig22, sch22);
variant!(S23, sig23, sch23);
variant!(S24, sig24, sch24);
variant!(S25, sig25, sch25);

pub fn variants() -> Vec<VariantDef> {
    let d = Data::Cash22 { watchlist: "NSE 500 C&P.tls" };
    vec![
        VariantDef { id: "ytath_plus5pct_buy_pctswing_exit_after_2", data: d, schema: sch01, signals: sig01 },
        VariantDef { id: "ytath_plus5pct_buy_pctswing_exit_after_5", data: d, schema: sch02, signals: sig02 },
        VariantDef { id: "ytath_buy_and_pctswing_exit_buy_only_in", data: d, schema: sch03, signals: sig03 },
        VariantDef { id: "ytath_delay_entry_and_pctswing_exit_with", data: d, schema: sch04, signals: sig04 },
        VariantDef { id: "ytath_delay_entry_and_pctswing_exit_with_2", data: d, schema: sch05, signals: sig05 },
        VariantDef { id: "ytath_delay_entry_and_pctswing_exit_with_st", data: d, schema: sch06, signals: sig06 },
        VariantDef { id: "ytath_delay_entry_and_pctswing_exit_with_st_2", data: d, schema: sch07, signals: sig07 },
        VariantDef { id: "ytath_delay_entry_and_pctswing_exit_with_3", data: d, schema: sch08, signals: sig08 },
        VariantDef { id: "ytath_delay_entry_and_pctswing_exit_with_4", data: d, schema: sch09, signals: sig09 },
        VariantDef { id: "ytath_plus_pct_buy_with_index_swing_filter", data: d, schema: sch10, signals: sig10 },
        VariantDef { id: "ytath_plus_pct_buy_index_filter_pctswing", data: d, schema: sch11, signals: sig11 },
        VariantDef { id: "ytath_plus_pct_buy_index_filter_pctswing_2", data: d, schema: sch12, signals: sig12 },
        VariantDef { id: "ytath_plus_pct_buy_index_filter_pctswing_3", data: d, schema: sch13, signals: sig13 },
        VariantDef { id: "ytath_plus_pct_buy_pctswing_exit_st_index", data: d, schema: sch14, signals: sig14 },
        VariantDef { id: "ytath_plus_pct_buy_pctswing_exit_index_2", data: d, schema: sch15, signals: sig15 },
        VariantDef { id: "ytath_entry_pctswing_exit_after_2_losses_in", data: d, schema: sch16, signals: sig16 },
        VariantDef { id: "ytath_entry_pctswing_exit_after_2_losses_in_2", data: d, schema: sch17, signals: sig17 },
        VariantDef { id: "ytath_pctswing_exit_after_2_st_5_2_5_4_loss", data: d, schema: sch18, signals: sig18 },
        VariantDef { id: "ytath_pctswing_exit_after_2_st_5_2_5_4_loss_2", data: d, schema: sch19, signals: sig19 },
        VariantDef { id: "ytath_pctswing_after_2_st_5_2_5_4_loss", data: d, schema: sch20, signals: sig20 },
        VariantDef { id: "ytath_pctswing_exit_after_2_st_5_2_5_4_loss_4", data: d, schema: sch21, signals: sig21 },
        VariantDef { id: "ytath_running_pctswing_after_2_st_5_2_5_4", data: d, schema: sch22, signals: sig22 },
        VariantDef { id: "ytath_pctswing_after_2_pctswing_losses_bear", data: d, schema: sch23, signals: sig23 },
        VariantDef { id: "ytath_pctswing_exit_after_2_pctswing_losses", data: d, schema: sch24, signals: sig24 },
        VariantDef { id: "ytath_pctswing_exit_after_2_pctswing_losses_2", data: d, schema: sch25, signals: sig25 },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(date: i32, c: f32) -> Bar {
        Bar { date, c, o: c, h: c + 1.0, l: c - 1.0, v: 1000.0, oi: 1.0e7, aux1: 1.0, aux2: 0.0 }
    }

    /// consecutive dates over `months` (28 days each, enough for the rules under test), starting January `year`
    fn dates(year: i32, months: i32, per: i32) -> Vec<i32> {
        let mut v = vec![];
        for m in 0..months {
            for d in 1..=per {
                v.push((year + m / 12) * 10000 + (m % 12 + 1) * 100 + d);
            }
        }
        v
    }

    fn series(name: &str, ds: &[i32], closes: &[f32]) -> Series {
        Series { name: name.into(), bars: ds.iter().zip(closes).map(|(&d, &c)| bar(d, c)).collect() }
    }

    fn reader(sp: &Spec, ov: &[(&'static str, f64)]) -> impl Fn(&str) -> f64 {
        let sch = schema_of(sp);
        let ov = ov.to_vec();
        move |k: &str| ov.iter().find(|(a, _)| *a == k).map(|x| x.1).or_else(|| sch.iter().find(|d| d.key == k).map(|d| d.def)).unwrap_or(0.0)
    }

    fn rising(n: usize) -> Vec<f32> {
        (0..n).map(|i| 100.0 + i as f32).collect()
    }

    #[test]
    fn all_25_ids_are_registered_with_defaults_inside_their_ranges_and_the_common_keys() {
        let v = variants();
        assert_eq!(v.len(), 25);
        let mut ids: Vec<&str> = v.iter().map(|x| x.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 25);
        for x in &v {
            let sch = (x.schema)();
            assert!(sch.iter().all(|d| d.min <= d.def && d.def <= d.max), "{}", x.id);
            assert!(sch.iter().all(|d| d.key != "equity"), "{} must not redefine a common key", x.id);
            let get = |k: &str| sch.iter().find(|d| d.key == k).map(|d| d.def);
            assert_eq!(get("deadstock"), Some(1.0));
            assert_eq!(get("mktflt"), Some(0.0));
            assert_eq!(x.data, Data::Cash22 { watchlist: "NSE 500 C&P.tls" });
        }
    }

    #[test]
    fn afl_defaults_and_ranges_of_the_optimize_and_param_rows() {
        let key = |sp: &Spec, k: &str| schema_of(sp).into_iter().find(|d| d.key == k);
        assert_eq!(key(&S13, "athcbuypct").map(|d| (d.def, d.min, d.max)), Some((3.0, 1.0, 10.0)));
        assert_eq!(key(&S13, "exitpct").map(|d| (d.def, d.min, d.max)), Some((20.0, 5.0, 22.0)));
        assert_eq!(key(&S14, "exitpct").map(|d| (d.def, d.min, d.max)), Some((15.0, 15.0, 18.0)));
        assert!(key(&S15, "exitpct").is_none() && key(&S15, "athcbuypct").is_none(), "Optimize rows are not parameters");
        assert!(key(&S19, "bpd2").is_none() && key(&S19, "bfactor2").is_none());
        assert!(key(&S20, "bpd2").is_some() && key(&S20, "bfactor2").is_none() && key(&S20, "exitpct").is_none());
        assert_eq!(key(&S09, "delaybuy").map(|d| (d.def, d.min, d.max)), Some((1.0, 1.0, 2.0)));
        assert_eq!(key(&S07, "delaybuy").map(|d| (d.def, d.min, d.max)), Some((1.0, 0.0, 10.0)));
        assert_eq!(key(&S23, "noofloss").map(|d| (d.def, d.min, d.max)), Some((2.0, 0.0, 50.0)));
        assert_eq!(key(&S22, "bfactor3").map(|d| (d.def, d.min, d.max, d.step)), Some((4.0, 1.0, 6.0, 0.5)));
        // Optimize(10, 11, 22) clamps to 11, Optimize(10, 5, 21) stays 10
        assert_eq!(S20.exit_fixed, 11.0);
        assert_eq!(S03.exit_fixed, 10.0);
    }

    #[test]
    fn the_swing_loop_and_the_null_compare() {
        let c = [100.0f32, 101.0, 102.0, 80.0, 81.0, 120.0];
        assert_eq!(pct_swing(&c, 10.0).trend, vec![1, 1, 1, -1, -1, 1]);
        assert!(!gt(5.0, NULL) && !gt(NULL, 5.0) && gt(5.0, 4.0));
    }

    #[test]
    fn delay_entry_buys_only_after_more_than_delaybuy_ath_closes_and_the_count_restarts_after_a_swing_down() {
        let n = 12;
        let ds = dates(2020, 1, 12);
        let mut c = rising(n);
        c[6] = 60.0; // swing down (exit 10 %) while the latch is on: count restarts
        let s = series("T", &ds, &c);
        let g = reader(&S04, &[("exitpct", 10.0), ("delaybuy", 2.0), ("deadstock", 1.0)]);
        let sg = run(std::slice::from_ref(&s), &Env::empty(), &g, &S04);
        let first = sg[0].buy.iter().position(|&b| b);
        assert_eq!(first, Some(3), "ATH closes at bars 1, 2, 3: count 3 > 2 buys at bar 3");
        let g1 = reader(&S04, &[("exitpct", 10.0), ("delaybuy", 1.0), ("deadstock", 1.0)]);
        assert_eq!(run(std::slice::from_ref(&s), &Env::empty(), &g1, &S04)[0].buy.iter().position(|&b| b), Some(2));
    }

    #[test]
    fn index_filter_decides_bull_or_bear_entry_and_the_exit_all_variants_sell_on_the_regime_change() {
        let n = 40;
        let ds = dates(2020, 2, 20);
        let c = rising(n);
        let stock = series("T", &ds, &c);
        // index falling: bear
        let down: Vec<f32> = (0..n).map(|i| 1000.0 - 5.0 * i as f32).collect();
        let mut env = Env::empty();
        env.index.insert("NSENIFTY".into(), series("NSENIFTY", &ds, &down));
        let ov = [("mktflt", 1.0), ("prflt", 0.0), ("exitpct", 30.0), ("fltfact1", 10.0), ("fltfact2", 10.0)];
        // delay variant 5 (swing, bull): the index never makes a new high and falls -> trend of the index flips to bear -> no buy later
        let bull = run(std::slice::from_ref(&stock), &env, &reader(&S05, &ov), &S05);
        let bear = run(std::slice::from_ref(&stock), &env, &reader(&S04, &ov), &S04);
        let nb = |s: &Signals| s.buy.iter().filter(|&&b| b).count();
        assert!(nb(&bull[0]) <= 1 && nb(&bear[0]) <= 1);
        // NONE: bear variant sees FLTSHORT, bull variant FLTBUY
        let none = [("exitpct", 30.0)];
        assert!(nb(&run(std::slice::from_ref(&stock), &Env::empty(), &reader(&S04, &none), &S04)[0]) == 1);
        assert!(nb(&run(std::slice::from_ref(&stock), &Env::empty(), &reader(&S05, &none), &S05)[0]) == 1);
        // the ST variants with a missing index never trade
        let st = [("mktflt", 1.0), ("prflt", 0.0), ("exitpct", 30.0)];
        assert_eq!(nb(&run(std::slice::from_ref(&stock), &Env::empty(), &reader(&S06, &st), &S06)[0]), 0);
    }

    #[test]
    fn plus_percent_buy_needs_the_trigger_close_and_swing_exit_sells() {
        let ds = dates(2020, 1, 8);
        let s = series("T", &ds, &[100.0, 101.0, 102.0, 108.0, 109.0, 90.0, 89.0, 88.0]);
        let g = reader(&S10, &[("exitpct", 10.0), ("athcbuypct", 5.0)]);
        let sg = &run(std::slice::from_ref(&s), &Env::empty(), &g, &S10)[0];
        assert!(!sg.buy[1] && !sg.buy[2] && sg.buy[3], "first ATH closes are below the +5 % trigger of the first close");
        assert!(sg.sell[5]);
    }

    #[test]
    fn st52_january_variant_never_buys_in_january_and_sells_on_the_last_day_of_december() {
        // Dec 2019 .. Feb 2020, 10 bars per month, steadily rising so every bar is a new high close
        let mut ds = vec![];
        for (y, m) in [(2019, 11), (2019, 12), (2020, 1), (2020, 2)] {
            for d in 1..=10 {
                ds.push(y * 10000 + m * 100 + d);
            }
        }
        let c = rising(ds.len());
        let s = series("T", &ds, &c);
        let g = reader(&S18, &[("exitpct", 30.0), ("noofloss", 0.0)]);
        let sg = &run(std::slice::from_ref(&s), &Env::empty(), &g, &S18)[0];
        let buys: Vec<i32> = (0..ds.len()).filter(|&i| sg.buy[i]).map(|i| ds[i]).collect();
        assert!(buys.iter().all(|&d| d / 100 % 100 != 1), "{buys:?}");
        assert!(!buys.is_empty());
        let sells: Vec<i32> = (0..ds.len()).filter(|&i| sg.sell[i]).map(|i| ds[i]).collect();
        assert!(sells.contains(&20191210), "{sells:?}");
    }

    #[test]
    fn running_buy_state_machine_of_the_bear_plus_percent_variant_arms_once() {
        let ds = dates(2020, 1, 14);
        let c: Vec<f32> = (0..14).map(|i| 100.0 + 2.0 * i as f32).collect();
        let s = series("T", &ds, &c);
        let g = reader(&S15, &[]);
        let sg = &run(std::slice::from_ref(&s), &Env::empty(), &g, &S15)[0];
        assert!(sg.buy.iter().filter(|&&b| b).count() <= 1);
        assert!(sg.buy.iter().any(|&b| b), "bear default (NONE) arms on the first close above the +5 % trigger");
    }

    #[test]
    fn profit_booking_variant_runs_and_keeps_fills_inside_the_bar() {
        let ds = dates(2020, 1, 30);
        let c: Vec<f32> = (0..30).map(|i| 100.0 + (i as f32) * 1.5 - if i > 20 { 40.0 } else { 0.0 }).collect();
        let s = series("T", &ds, &c);
        let g = reader(&S23, &[("noofloss", 0.0), ("exitpct", 10.0)]);
        let sg = &run(std::slice::from_ref(&s), &Env::empty(), &g, &S23)[0];
        for i in 0..30 {
            assert!(sg.sell_price[i] >= s.bars[i].l && sg.sell_price[i] <= s.bars[i].h);
            assert!(sg.buy_price[i] >= s.bars[i].l && sg.buy_price[i] <= s.bars[i].h);
        }
        assert!(sg.buy.iter().any(|&b| b));
    }

    #[test]
    fn the_st_loss_counter_and_the_pct_loss_counter_count_streaks() {
        // a long rising series: no loss trade yet, noofloss 0 buys, noofloss 1 never buys
        let ds = dates(2020, 1, 25);
        let s = series("T", &ds, &rising(25));
        let n0 = run(std::slice::from_ref(&s), &Env::empty(), &reader(&S17, &[("noofloss", 0.0), ("exitpct", 30.0)]), &S17)[0].buy.iter().filter(|&&b| b).count();
        let n1 = run(std::slice::from_ref(&s), &Env::empty(), &reader(&S17, &[("noofloss", 1.0), ("exitpct", 30.0)]), &S17)[0].buy.iter().filter(|&&b| b).count();
        assert!(n0 >= 1 && n1 == 0, "{n0} {n1}");
        let m0 = run(std::slice::from_ref(&s), &Env::empty(), &reader(&S25, &[("noofloss", 1.0), ("exitpct", 30.0)]), &S25)[0].buy.iter().filter(|&&b| b).count();
        assert_eq!(m0, 0);
    }
}
