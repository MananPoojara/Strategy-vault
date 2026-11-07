//! ATH engines: ATHC +x % buy with %swing exit, Bull-market index filters and the gold combo.
//! One `VariantDef` per sub-variant (see `ath_core::VariantDef`); registered in `ath_registry.rs`. Standard library only: depends on
//! `ath_core` (and, if useful, `ath_pctswing` / `ath_ind`) so a verification driver can include it with `#[path]`.
//!
//! Variants (AFL sources and the checks against the team's sheets: `ATH/verify/<variant id>/RESULT.md`):
//! * `ytath_plus5pct_buy_pctswing_exit_after`   : ATHC +x% buy after consecutive losses, index %swing filter   (AFL "...flt %S for BULL mkt (Pending)")
//! * `ytath_plus5pct_buy_pctswing_exit_after_3` : same, index ST / SMA filter on monthly / weekly / daily bars    (AFL "...flt ST for BULL mkt")
//! * `ytath_plus5pct_buy_pctswing_exit_after_4` : like `_after` but the ATHC latch itself needs Close > trigger  (AFL "...n index flt %S for BULL mkt (Pendin")
//! * `ytath_plus5pct_buy_pctswing_exit_after_6` : like `_after_3` with the latch rule of `_after_4`
//! * `ytath_plus_4pct_buy_pctswing_exit_new`    : ATHC +x% buy, %swing exit, new turnover filter, no buy on Close == High
//! * `ytath_plus_5pct_buy_pctswing_exit_avg` / `ytath_plus_4pct_buy_pctswing_exit_avg` : turnover-rank band (cap size), SL versions
//! * `ytath_combo_bull_mkt_athc_buy_plus_pctswing` : bull market ATH stocks, bear market GOLD / SILVER (GOLD is not in the converted data)
//!
//! List parameters are 0-based indexes (as in `ath_pctswing`): deadstock 0 YES / 1 NO, turnflt 0 NO / 1 YES, mktflt 0 NONE / 1 INDEX,
//! prflt 0 NSENIFTY / 1 NSENIFTY1 / 2 NIFTY / 3 NSE500, flttf 0 MONTHLY / 1 WEEKLY / 2 DAILY, iflt 0 ST / 1 SMA,
//! istflt 0 MID POINT / 1 CLOSE / 2 PIVOT, ver 0 OFF / 1 % HARD SL / 2 ATR HARD SL / 3 DAYS LOWEST CLOSE, bkmkt 0 LARGE / 1 MID / 2 SMALL / 3 MICRO,
//! mktscene 0 Bull / 1 Bear, mcxsymbl 0 GOLD / 1 SILVER.
#![allow(dead_code)]
#[allow(unused_imports)]
use crate::ath_core::*;
use std::cell::RefCell;

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

// ---------------------------------------------------------------- family 1: ATHC +x% buy after consecutive losses (4 AFLs)

#[derive(Clone, Copy)]
struct Loss {
    /// the ATHC latch itself needs Close > trigger (AFLs "..._after_4" / "_after_6"); otherwise Buy needs Close > trigger
    latch_trigger: bool,
    /// the index filter is the supertrend / SMA one (AFLs "_after_3" / "_after_6") with its price-array leak
    st_mode: bool,
}

struct LossP {
    dead_yes: bool,
    penny: f32,
    turn: bool,
    index: bool,
    f1: f32,
    f2: f32,
    exit_pct: f32,
    losspct: f32,
    noofloss: f32,
    athc_pct: f32,
    comm: f32,
}

fn loss_signals(s: &Series, idx: Option<&Series>, p: &LossP, flt: &Flt, kind: Loss) -> Signals {
    let n = s.bars.len();
    let sc = s.close();
    let so = s.open();
    let sh = s.high();
    let sl = s.low();
    let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
    let (deadbuy, deadsell) = dead_flags(n, p.dead_yes);
    let (upc, loc) = circuits(&so, &sh, &sl, &sc);
    let turnover: Vec<bool> = (0..n).map(|i| if p.turn { s.bars[i].v * sc[i] > volsig_loss(dates[i] / 10000) } else { true }).collect();
    // price arrays used after the market filter block
    let (o, h, l, c) = (so.clone(), sh.clone(), sl.clone(), sc.clone());
    let mut fltbuy = vec![true; n];
    if p.index {
        let Some(ix) = idx else { return blank(n) };
        let al = align(ix, &dates);
        if !kind.st_mode {
            // index %swing: bear when Close < Dn (-fltfact1 %), bull when Close > Up (+fltfact2 %) or a new index ATH close
            let hcl_i = hhv_all(&al.c);
            let br: Vec<f32> = al.c.iter().map(|&x| (x * p.f1) / 100.0).collect();
            let bl: Vec<f32> = al.c.iter().map(|&x| (x * p.f2) / 100.0).collect();
            let up: Vec<f32> = (0..n).map(|i| al.c[i] + bl[i]).collect();
            let dn: Vec<f32> = (0..n).map(|i| al.c[i] - br[i]).collect();
            let uf: Vec<f32> = (0..n).map(|i| hcl_i[i].min(al.c[i] + bl[i])).collect();
            let df: Vec<f32> = (0..n).map(|i| al.c[i] - br[i]).collect();
            let sw = swing_loop(&al.c, up, dn, &uf, &df, Some(&hcl_i));
            fltbuy = sw.trend.iter().map(|&t| t == 1).collect();
        } else {
            fltbuy = st_filter(&al, &dates, flt).0;
            // NOTE: the AFL calls SetForeign() and never RestorePriceArrays() in this version; the team's sheets (combo AFL, same
            // construction) show the stock's own Close as fill price, i.e. TimeFrameRestore() brings the stock's arrays back. So no leak.
        }
    }
    let hcl = hhv_all(&c);
    let sw = pct_swing(&c, p.exit_pct);
    let trend1 = &sw.trend;
    let trigger = trigger_latch(&c, &hcl, trend1, p.athc_pct);
    let athc = athc_latch(&c, &hcl, trend1, if kind.latch_trigger { Some(&trigger) } else { None });
    // hypothetical ATHC trades -> consecutive loss counter
    let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !upc[i] && deadbuy[i] && c[i] > p.penny && turnover[i]).collect();
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
            profit = (sp - (sp * p.comm) / 100.0) - (bp + (bp * p.comm) / 100.0);
            pct = (((sp - (sp * p.comm) / 100.0) - (bp + (bp * p.comm) / 100.0)) / bp) * 100.0;
        }
        concount[i] = if profit != 0.0 {
            if pct <= p.losspct { concount[i - 1] + 1 } else { 0 }
        } else {
            concount[i - 1]
        };
    }
    let dead1 = flat30(&o, &h, &l, &c);
    let mut buy = vec![false; n];
    let mut sell = vec![false; n];
    for i in 0..n {
        buy[i] = athc[i]
            && (kind.latch_trigger || c[i] > trigger[i])
            && concount[i] as f32 >= p.noofloss
            && fltbuy[i]
            && !dead1[i]
            && !upc[i]
            && deadbuy[i]
            && c[i] > p.penny
            && turnover[i];
        sell[i] = (trend1[i] == -1 && !loc[i]) || deadsell[i] || dead1[i];
    }
    let buy2 = exrem(&buy, &sell);
    let sell2 = exrem(&sell, &buy2);
    let mut sg = Signals::new(buy2, sell2, c.clone(), c);
    clamp_prices(&mut sg, &sh, &sl);
    sg
}

fn loss_params(g: G) -> LossP {
    LossP {
        dead_yes: g("deadstock").round() == 0.0,
        penny: g("pennyflt") as f32,
        turn: g("turnflt").round() == 1.0,
        index: g("mktflt").round() == 1.0,
        f1: g("fltfact1") as f32,
        f2: g("fltfact2") as f32,
        exit_pct: g("exitpct") as f32,
        losspct: g("losspct") as f32,
        noofloss: g("noofloss") as f32,
        athc_pct: g("athcbuypct") as f32,
        comm: g("comm") as f32,
    }
}

fn loss_all(series: &[Series], env: &Env, g: G, kind: Loss) -> Vec<Signals> {
    let p = loss_params(g);
    let flt = flt_from(g);
    let hard = g("hardstop") as f32;
    let idx = index_of(env, g);
    series
        .iter()
        .map(|s| {
            let mut sg = loss_signals(s, idx, &p, &flt, kind);
            if hard > 0.0 {
                sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
            }
            sg
        })
        .collect()
}

fn common_loss_params(swing_filter: bool, exit_rng: (f64, f64), athc_min: f64, noloss_max: f64, f2_max: f64, f1_max: f64) -> Vec<P> {
    let mut v = vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (ApplyStop percent loss; 0 = off)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; V*C against the AFL's own yearly table)", 0.0, 0.0, 1.0, 1.0),
        p("mktflt", "FILTER TYPE (0 NONE, 1 INDEX)", 0.0, 0.0, 1.0, 1.0),
        p("prflt", "INDEX (0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 0.0, 0.0, 3.0, 1.0),
    ];
    if swing_filter {
        v.push(p("fltfact1", "SWING % FOR BEAR MKT", 23.0, 0.0, f1_max, 1.0));
        v.push(p("fltfact2", "SWING % FOR BULL MKT", 44.0, 0.0, f2_max, 1.0));
    } else {
        v.push(p("flttf", "FILTER TIMEFRAME (0 MONTHLY, 1 WEEKLY, 2 DAILY)", 0.0, 0.0, 2.0, 1.0));
        v.push(p("iflt", "INDEX Filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0));
        v.push(p("istflt", "ST CALCULATION POINT (0 MID POINT, 1 CLOSE, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0));
        v.push(p("ifltpd", "INDEX ST PERIOD", 20.0, 0.0, 20.0, 1.0));
        v.push(p("ifltfact", "INDEX ST FACTOR", 1.0, 0.0, 20.0, 1.0));
        v.push(p("ifltmapd", "INDEX SMA PERIOD", 200.0, 1.0, 300.0, 1.0));
    }
    v.push(p("exitpct", "EXIT %", 10.0, exit_rng.0, exit_rng.1, 1.0));
    v.push(p("losspct", "MIN PROFIT %", 2.0, 0.0, 10.0, 0.05));
    v.push(p("noofloss", "CONSICUTIVE LOSS", 1.0, 0.0, noloss_max, 1.0));
    v.push(p("athcbuypct", "BUY PERCENTAGE FROM ATHC", 5.0, athc_min, 10.0, 1.0));
    // not an AFL row of its own: the AFL's `comm` (BROKERAGE) also enters the loss counter's profit %, so the signals depend on it
    v.push(p("comm", "BROKERAGE % used by the AFL's loss counter (set equal to the run's brokerage)", 0.65, 0.0, 5.0, 0.01));
    v
}

fn schema_after() -> Vec<P> {
    common_loss_params(true, (5.0, 100.0), 0.0, 50.0, 50.0, 50.0)
}
fn schema_after_3() -> Vec<P> {
    common_loss_params(false, (10.0, 17.0), 1.0, 50.0, 0.0, 0.0)
}
fn schema_after_4() -> Vec<P> {
    common_loss_params(true, (5.0, 90.0), 1.0, 100.0, 100.0, 50.0)
}
fn schema_after_6() -> Vec<P> {
    common_loss_params(false, (5.0, 100.0), 1.0, 50.0, 0.0, 0.0)
}
fn sig_after(s: &[Series], e: &Env, g: G) -> Vec<Signals> {
    loss_all(s, e, g, Loss { latch_trigger: false, st_mode: false })
}
fn sig_after_3(s: &[Series], e: &Env, g: G) -> Vec<Signals> {
    loss_all(s, e, g, Loss { latch_trigger: false, st_mode: true })
}
fn sig_after_4(s: &[Series], e: &Env, g: G) -> Vec<Signals> {
    loss_all(s, e, g, Loss { latch_trigger: true, st_mode: false })
}
fn sig_after_6(s: &[Series], e: &Env, g: G) -> Vec<Signals> {
    loss_all(s, e, g, Loss { latch_trigger: true, st_mode: true })
}

// ---------------------------------------------------------------- family 2: ATHC +x% buy, %swing exit, new turnover filter

fn new_signals(s: &Series, dead_yes: bool, penny: f32, turn: bool, exit_pct: f32, athc_pct: f32) -> Signals {
    new_signals_k(s, dead_yes, penny, turn, exit_pct, athc_pct, false)
}

/// `ratio` = the "ATHC PLUS 5 PCT BUY WITH RATIO exit with %swing" AFL: turnover is V*C against the consecutive-loss AFLs' table and Buy has no `NOT (H==C)`.
fn new_signals_k(s: &Series, dead_yes: bool, penny: f32, turn: bool, exit_pct: f32, athc_pct: f32, ratio: bool) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let oi = s.openint();
    let hcl = hhv_all(&c);
    let sw = pct_swing(&c, exit_pct);
    let trend1 = &sw.trend;
    let trigger = trigger_latch(&c, &hcl, trend1, athc_pct);
    let buysig = athc_latch(&c, &hcl, trend1, None);
    let (deadbuy, deadsell) = dead_flags(n, dead_yes);
    let (upc, loc) = circuits(&o, &h, &l, &c);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let uc = h[i] == c[i];
        let turnover = if turn {
            if ratio { s.bars[i].v * c[i] > volsig_loss(s.bars[i].date / 10000) } else { oi[i] > volsig_std(s.bars[i].date / 10000) }
        } else {
            true
        };
        // AmiBroker: a comparison with Null is false (bar 0 has no Ref(hcl,-1) and no trigger)
        let new_high = i >= 1 && c[i] > hcl[i - 1];
        buy[i] = (new_high || buysig[i]) && !is_null(trigger[i]) && c[i] > trigger[i] && deadbuy[i] && !upc[i] && (ratio || !uc) && c[i] > penny && turnover;
        sell[i] = ((trend1[i] == -1 || !buysig[i]) && !loc[i]) || deadsell[i];
    }
    let buy2 = exrem(&buy, &sell);
    let sell2 = exrem(&sell, &buy2);
    let mut sg = Signals::new(buy2, sell2, c.clone(), c);
    clamp_prices(&mut sg, &h, &l);
    sg
}

fn schema_new() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (ApplyStop percent loss; 0 = off)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; OpenInt against the yearly table)", 0.0, 0.0, 1.0, 1.0),
        p("exitpct", "EXIT %", 10.0, 5.0, 25.0, 1.0),
        p("swingpd", "SWING PERIOD (not used by this AFL's signals)", 180.0, 10.0, 1000.0, 1.0),
        p("athcbuypct", "BUY PERCENTAGE FROM ATHC", 4.0, 3.0, 12.0, 1.0),
    ]
}

fn sig_new(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    sig_new_k(series, env, g, false)
}
fn sig_new_k(series: &[Series], _env: &Env, g: G, ratio: bool) -> Vec<Signals> {
    let hard = g("hardstop") as f32;
    series
        .iter()
        .map(|s| {
            let mut sg = new_signals_k(s, g("deadstock").round() == 0.0, g("pennyflt") as f32, g("turnflt").round() == 1.0, g("exitpct") as f32, g("athcbuypct") as f32, ratio);
            if hard > 0.0 {
                sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
            }
            sg
        })
        .collect()
}

// ---------------------------------------------------------------- family 3: turnover rank band + SL versions (two AFLs)

/// `ATR(n)`: Wilder smoothing of the true range, first value = first true range (AmiBroker's `Wilders` seed).
fn atr_wilder(h: &[f32], l: &[f32], c: &[f32], n: usize) -> Vec<f32> {
    let len = c.len();
    let mut out = vec![0f32; len];
    let nn = n as f32;
    for i in 0..len {
        let tr = if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) };
        out[i] = if i == 0 { tr } else { (out[i - 1] * (nn - 1.0) + tr) / nn };
    }
    out
}

/// `StaticVarGenerateRanks("rank", ..., 0, 1234)` over the universe: for every date the symbols are ordered by the value, highest = rank 1,
/// ties by position in the list. Values are forward filled inside a symbol's life; before its first bar / after its last it has none.
struct RankBook {
    dates: Vec<i32>,
    vals: Vec<Vec<f32>>,
    cols: RefCell<Vec<Option<Vec<(f32, u32)>>>>,
}

impl RankBook {
    fn new(series: &[Series], values: &[Vec<f32>]) -> RankBook {
        let mut dates: Vec<i32> = series.iter().flat_map(|s| s.bars.iter().map(|b| b.date)).collect();
        dates.sort_unstable();
        dates.dedup();
        let vals: Vec<Vec<f32>> = series
            .iter()
            .zip(values)
            .map(|(s, v)| {
                let mut out = vec![f32::NAN; dates.len()];
                let mut j = 0usize;
                for (gi, &d) in dates.iter().enumerate() {
                    while j < s.bars.len() && s.bars[j].date <= d {
                        j += 1;
                    }
                    if j > 0 && j < s.bars.len() + 1 && !(j == s.bars.len() && s.bars[j - 1].date < d) {
                        out[gi] = v[j - 1];
                    }
                }
                out
            })
            .collect();
        let m = dates.len();
        RankBook { dates, vals, cols: RefCell::new(vec![None; m]) }
    }
    fn gidx(&self, date: i32) -> usize {
        self.dates.binary_search(&date).unwrap_or(0)
    }
    fn rank(&self, k: usize, date: i32) -> Option<usize> {
        let gi = self.gidx(date);
        let v = self.vals[k][gi];
        if v.is_nan() {
            return None;
        }
        let mut cols = self.cols.borrow_mut();
        if cols[gi].is_none() {
            let mut col: Vec<(f32, u32)> = (0..self.vals.len()).filter(|&m| !self.vals[m][gi].is_nan()).map(|m| (self.vals[m][gi], m as u32)).collect();
            col.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
            cols[gi] = Some(col);
        }
        let col = cols[gi].as_ref().unwrap();
        col.binary_search_by(|(cv, ci)| v.partial_cmp(cv).unwrap().then(ci.cmp(&(k as u32)))).ok().map(|p| p + 1)
    }
}

struct AvgP {
    dead_yes: bool,
    penny: f32,
    turn: bool,
    athc_pct: f32,
    exit_pct: f32,
    ver: u8,
    hard_sl: f32,
    atr_prd: usize,
    atr_m: f32,
    days_sl: usize,
    pd: usize,
    band: (i32, i32),
    /// index filter (AFL 7 only): 0 = off
    index: bool,
    scene_bear: bool,
}

fn avg_all(series: &[Series], env: &Env, g: G, with_index: bool) -> Vec<Signals> {
    let p = AvgP {
        dead_yes: g("deadstock").round() == 0.0,
        penny: g("pennyflt") as f32,
        turn: g("turnflt").round() == 1.0,
        athc_pct: g("athcbuypct") as f32,
        exit_pct: g("exitpct") as f32,
        ver: g("ver").round() as u8,
        hard_sl: g("hardslpct") as f32,
        atr_prd: g("atr_prd").round().max(1.0) as usize,
        atr_m: g("atr_sl_m") as f32,
        days_sl: g("days_sl").round().max(1.0) as usize,
        pd: g("turnoverpd").round().max(1.0) as usize,
        band: match g("bkmkt").round() as i32 {
            0 => (0, 101),
            1 => (100, 251),
            2 => (250, 801),
            _ => (800, 80000),
        },
        index: with_index && g("mktflt").round() == 1.0,
        scene_bear: with_index && g("mktscene").round() == 1.0,
    };
    let flt = if with_index { flt_from(g) } else { Flt { tf: 0, sma: false, point: 0, pd: 20, fact: 1.0, mapd: 200 } };
    let idx = if with_index { index_of(env, g) } else { None };
    // ranking value per symbol: IIf(deadstock == 0, MA(C*V, pd), 0)
    let mut turn_ma: Vec<Vec<f32>> = Vec::with_capacity(series.len());
    let mut rank_vals: Vec<Vec<f32>> = Vec::with_capacity(series.len());
    let mut dead_arr: Vec<Vec<bool>> = Vec::with_capacity(series.len());
    for s in series {
        let (c, o, h, l, v) = (s.close(), s.open(), s.high(), s.low(), s.volume());
        let to: Vec<f32> = (0..c.len()).map(|i| c[i] * v[i]).collect();
        let tma = crate::ath_ind::ma(&to, p.pd);
        let d30 = flat30(&o, &h, &l, &c);
        rank_vals.push((0..c.len()).map(|i| if d30[i] { 0.0 } else { tma[i] }).collect());
        turn_ma.push(tma);
        dead_arr.push(d30);
    }
    let book = RankBook::new(series, &rank_vals);
    let mut out = Vec::with_capacity(series.len());
    for (k, s) in series.iter().enumerate() {
        let mut sg = avg_symbol(s, k, &p, &flt, idx, &book, &dead_arr[k]);
        sg.score = Some(turn_ma[k].iter().map(|&x| if x.is_nan() { f32::NEG_INFINITY } else { x }).collect());
        out.push(sg);
    }
    out
}

fn avg_symbol(s: &Series, k: usize, p: &AvgP, flt: &Flt, idx: Option<&Series>, book: &RankBook, dead1: &[bool]) -> Signals {
    let n = s.bars.len();
    let (sc, so, sh, sl) = (s.close(), s.open(), s.high(), s.low());
    let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
    let (deadbuy, deadsell) = dead_flags(n, p.dead_yes);
    let hcl = hhv_all(&sc);
    let sw = pct_swing(&sc, p.exit_pct);
    let trend1 = &sw.trend;
    let trigger = trigger_latch(&sc, &hcl, trend1, p.athc_pct);
    let buysig = athc_latch(&sc, &hcl, trend1, None);
    let _ = buysig;
    let turnover: Vec<bool> = (0..n).map(|i| if p.turn { s.bars[i].v * sc[i] > volsig_std(dates[i] / 10000) } else { true }).collect();
    // index filter (AFL 7): FLTSHORT; the price arrays stay the index's afterwards (no RestorePriceArrays)
    let (c, h, l) = (sc.clone(), sh.clone(), sl.clone());
    let mut fltbuy = vec![false; n];
    let mut fltshort = vec![true; n];
    if p.index {
        let Some(ix) = idx else { return blank(n) };
        let al = align(ix, &dates);
        let (up, dn) = st_filter(&al, &dates, flt);
        if p.scene_bear {
            fltbuy = up;
            fltshort = dn;
        } else if flt.sma {
            fltbuy = up;
            fltshort = dn;
        } else {
            // Bull scene, ST: FLTBuy = trend == -1, FLTShort = trend == 1
            fltbuy = dn;
            fltshort = up;
        }
    }
    let uc: Vec<bool> = (0..n).map(|i| sh[i] == sc[i]).collect();
    let lc: Vec<bool> = (0..n).map(|i| sl[i] == sc[i]).collect();
    // SL helpers (computed on the current price arrays)
    let sl1: Vec<f32> = if p.ver == 3 && p.days_sl > 1 {
        (0..n).map(|i| c[i.saturating_sub(p.days_sl - 1)..=i].iter().copied().fold(f32::MAX, f32::min)).collect()
    } else {
        vec![NULL; n]
    };
    let myatr = atr_wilder(&h, &l, &c, p.atr_prd);
    let mut buy = vec![false; n];
    let mut sell = vec![false; n];
    let mut buy_price = c.clone();
    let mut sell_price = c.clone();
    let (mut position, mut skipsell) = (false, false);
    let (mut hsl, mut atr_sl, mut sl_lvl) = (0f32, 0f32, 0f32);
    let tdate = |i: usize| dates[i];
    for i in 1..n {
        let cash_filter = !uc[i] && turnover[i] && c[i] > p.penny && deadbuy[i];
        if !position
            && cash_filter
            && (!p.index || fltshort[i])
            && c[i] > trigger[i]
            && c[i] > hcl[i - 1]
            && book.rank(k, tdate(i)).map_or(false, |r| (r as i32) > p.band.0 && (r as i32) < p.band.1)
            && !dead1[i]
        {
            buy[i] = true;
            buy_price[i] = c[i];
            if p.ver == 1 && p.hard_sl > 0.0 {
                hsl = (1.0 - p.hard_sl * 0.01) * buy_price[i];
            }
            if p.ver == 2 && myatr[i] > 0.0 {
                atr_sl = c[i] - myatr[i] * p.atr_m;
            }
            if p.ver == 3 {
                sl_lvl = sl1[i];
            }
            position = true;
        } else if position && !skipsell {
            let stop_hit = c[i] <= hsl || c[i] <= atr_sl || c[i] <= sl_lvl;
            if stop_hit || deadsell[i] || trend1[i] == -1 || (p.index && fltbuy[i]) || dead1[i] {
                if !lc[i] || deadsell[i] || dead1[i] {
                    sell[i] = true;
                    sell_price[i] = c[i];
                    position = false;
                    hsl = 0.0;
                    atr_sl = 0.0;
                    sl_lvl = 0.0;
                } else {
                    skipsell = true;
                }
            }
        } else if skipsell && (!lc[i] || deadsell[i] || dead1[i]) {
            sell[i] = true;
            sell_price[i] = c[i];
            position = false;
            skipsell = false;
            hsl = 0.0;
            atr_sl = 0.0;
            sl_lvl = 0.0;
        }
    }
    let mut sg = Signals::new(buy, sell, buy_price, sell_price);
    clamp_prices(&mut sg, &sh, &sl);
    sg
}

fn avg_schema(buy_def: f64, buy_rng: (f64, f64), exit_def: f64, exit_rng: (f64, f64), pd_def: f64, pd_rng: (f64, f64), index: bool) -> Vec<P> {
    let mut v = vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; V*C against the yearly table)", 0.0, 0.0, 1.0, 1.0),
        p("athcbuypct", "BUY PERCENTAGE FROM ATHC", buy_def, buy_rng.0, buy_rng.1, 1.0),
        p("exitpct", "EXIT %", exit_def, exit_rng.0, exit_rng.1, 1.0),
        p("ver", "SL EXIT (0 OFF, 1 % Based HARD SL, 2 ATR Based HARD SL, 3 DAYS LOWEST CLOSE)", 0.0, 0.0, 3.0, 1.0),
        p("hardslpct", "HARD SL%", 2.0, 0.0, 12.0, 1.0),
        p("atr_prd", "ATR Prds", 5.0, 0.0, 100.0, 1.0),
        p("atr_sl_m", "ATR Hard SL Multiplier", 3.0, 0.0, 6.0, 1.0),
        p("days_sl", "DAYS SL", 1.0, 1.0, 100.0, 1.0),
    ];
    if index {
        v.push(p("mktflt", "FILTER TYPE (0 NONE, 1 INDEX)", 0.0, 0.0, 1.0, 1.0));
        v.push(p("flttf", "FILTER TIMEFRAME (0 MONTHLY, 1 WEEKLY, 2 DAILY)", 0.0, 0.0, 2.0, 1.0));
        v.push(p("prflt", "INDEX (0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 0.0, 0.0, 3.0, 1.0));
        v.push(p("mktscene", "Market Condition (0 Bull, 1 Bear)", 0.0, 0.0, 1.0, 1.0));
        v.push(p("iflt", "INDEX Filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0));
        v.push(p("istflt", "ST CALCULATION POINT (0 MID POINT, 1 CLOSE, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0));
        v.push(p("ifltpd", "INDEX ST PERIOD", 20.0, 0.0, 20.0, 1.0));
        v.push(p("ifltfact", "INDEX ST FACTOR", 1.0, 0.0, 20.0, 1.0));
        v.push(p("ifltmapd", "INDEX SMA PERIOD", 200.0, 1.0, 300.0, 1.0));
    }
    v.push(p("swingpd", "SWING PERIOD (not used by this AFL's signals)", 180.0, 10.0, 1000.0, 1.0));
    v.push(p("turnoverpd", "TURNOVER AVG PERIOD", pd_def, pd_rng.0, pd_rng.1, 1.0));
    v.push(p("bkmkt", "BACKTEST MARKET AS PER VOLUME (0 LARGECAP rank 1-100, 1 MIDCAP 101-250, 2 SMALLCAP 251-800, 3 MICROCAP 801+)", 0.0, 0.0, 3.0, 1.0));
    v
}

fn schema_avg5() -> Vec<P> {
    avg_schema(5.0, (1.0, 8.0), 17.0, (5.0, 22.0), 1.0, (1.0, 500.0), true)
}
fn schema_avg4() -> Vec<P> {
    avg_schema(4.0, (0.0, 12.0), 10.0, (5.0, 25.0), 60.0, (1.0, 200.0), false)
}
fn sig_avg5(s: &[Series], e: &Env, g: G) -> Vec<Signals> {
    avg_all(s, e, g, true)
}
fn sig_avg4(s: &[Series], e: &Env, g: G) -> Vec<Signals> {
    avg_all(s, e, g, false)
}

// ---------------------------------------------------------------- family 4: the combo (bull ATH stocks, bear GOLD / SILVER)

struct ComboP {
    dead_yes: bool,
    penny: f32,
    turn: bool,
    exit1: f32,
    mcx: bool,
    comm: f32,
    comm1: f32,
    pad: bool,
}

/// "Pad and align to the reference symbol" also cuts a stock's history to the reference's first bar: everything the AFL computes (all-time
/// high, swing, latch) starts there. Signals are returned for the stock's full bar list (nothing before the cut).
fn combo_symbol(s: &Series, idx: Option<&Series>, refc: Option<&Series>, p: &ComboP, flt: &Flt) -> Signals {
    if let (true, Some(rc)) = (p.pad, refc) {
        let first = rc.bars.first().map_or(0, |b| b.date);
        let cut = s.bars.partition_point(|b| b.date < first);
        if cut > 0 {
            let sub = Series { name: s.name.clone(), bars: s.bars[cut..].to_vec() };
            let m = sub.bars.len();
            if m == 0 {
                return blank(s.bars.len());
            }
            let inner = combo_inner(&sub, idx, refc, p, flt);
            let n = s.bars.len();
            let mut out = blank(n);
            for i in 0..m {
                out.buy[cut + i] = inner.buy[i];
                out.sell[cut + i] = inner.sell[i];
                out.buy_price[cut + i] = inner.buy_price[i];
                out.sell_price[cut + i] = inner.sell_price[i];
            }
            for i in 0..cut {
                out.buy_price[i] = s.bars[i].c;
                out.sell_price[i] = s.bars[i].c;
            }
            return out;
        }
    }
    combo_inner(s, idx, refc, p, flt)
}

fn combo_inner(s: &Series, idx: Option<&Series>, refc: Option<&Series>, p: &ComboP, flt: &Flt) -> Signals {
    let n = s.bars.len();
    let (sc, so, sh, sl) = (s.close(), s.open(), s.high(), s.low());
    let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
    let Some(ix) = idx else { return blank(n) };
    let (deadbuy, deadsell) = dead_flags(n, p.dead_yes);
    let dead1 = flat30(&so, &sh, &sl, &sc);
    let (upc, _loc) = circuits(&so, &sh, &sl, &sc);
    let uc: Vec<bool> = (0..n).map(|i| sh[i] == sc[i]).collect();
    let lc: Vec<bool> = (0..n).map(|i| sl[i] == sc[i]).collect();
    let turnover: Vec<bool> = (0..n).map(|i| if p.turn { s.bars[i].oi > volsig_std(dates[i] / 10000) } else { true }).collect();
    let hcl = hhv_all(&sc); // before SetForeign: the stock's own all-time-high close
    let al = align(ix, &dates);
    let (fltbuy, fltshort) = if let (true, Some(rc)) = (p.pad, refc) { st_filter_padded(ix, rc, &dates, flt) } else { st_filter(&al, &dates, flt) };
    // (the AFL never calls RestorePriceArrays() after SetForeign, but TimeFrameRestore() brings the stock's arrays back: see RESULT.md)
    let c = &sc;
    let bull_exit: Vec<f32> = c.iter().map(|&x| (x * p.exit1) / 100.0).collect();
    let up: Vec<f32> = hcl.clone();
    let dn: Vec<f32> = (0..n).map(|i| c[i] - bull_exit[i]).collect();
    let uf = hcl.clone();
    let df = dn.clone();
    let sw = swing_loop(c, up, dn, &uf, &df, None);
    let trend1 = &sw.trend;
    let mut athc = vec![false; n];
    for i in 1..n {
        athc[i] = if c[i] > hcl[i - 1] {
            true
        } else if trend1[i] == -1 && athc[i - 1] {
            false
        } else {
            athc[i - 1]
        };
    }
    let mut buy = vec![false; n];
    let mut sell = vec![false; n];
    let (mut position, mut bullbuy, mut bearbuy) = (false, false, false);
    for i in 1..n {
        let buyfilter = turnover[i] && !upc[i] && !uc[i] && c[i] > p.penny && deadbuy[i] && !dead1[i] && !p.mcx;
        if !position {
            if athc[i] && fltbuy[i] && buyfilter {
                buy[i] = true;
                bullbuy = true;
                position = true;
            } else if fltshort[i] && p.mcx {
                buy[i] = true;
                bearbuy = true;
                position = true;
            }
        } else if (bullbuy && (trend1[i] == -1 || fltshort[i]) && !lc[i]) || deadsell[i] || dead1[i] {
            sell[i] = true;
            position = false;
            bullbuy = false;
        } else if fltbuy[i] && bearbuy {
            sell[i] = true;
            position = false;
            bearbuy = false;
        } else if bearbuy && fltbuy[i] {
            bullbuy = true;
            bearbuy = false;
        }
    }
    let cm = p.comm;
    let cm1 = p.comm1;
    let buy_price: Vec<f32> = (0..n).map(|i| if p.mcx { c[i] - (c[i] * (cm - cm1)) / 100.0 } else { c[i] }).collect();
    let sell_price: Vec<f32> = (0..n).map(|i| if p.mcx { (c[i] * (cm - cm1)) / 100.0 + c[i] } else { c[i] }).collect();
    let mut sg = Signals::new(buy, sell, buy_price, sell_price);
    clamp_prices(&mut sg, &sh, &sl);
    sg
}

fn schema_combo() -> Vec<P> {
    vec![
        p("mcxsize", "BEAR MARKET ALLOCATION %OF Equity (not applied: the portfolio core sizes every symbol the same)", 100.0, 0.0, 100.0, 1.0),
        p("comm1", "BROKERAGE for MCX", 0.04, 0.01, 1.0, 0.01),
        p("mcxsymbl", "BEAR MARKET INSTURMENT (0 GOLD, 1 SILVER)", 0.0, 0.0, 1.0, 1.0),
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (ApplyStop percent loss; 0 = off)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 500.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; OpenInt against the yearly table)", 0.0, 0.0, 1.0, 1.0),
        p("exitpct1", "EXIT % IN BULL MKT", 19.0, 5.0, 100.0, 1.0),
        p("flttf", "FILTER TIMEFRAME (0 MONTHLY, 1 WEEKLY, 2 DAILY)", 0.0, 0.0, 2.0, 1.0),
        p("prflt", "INDEX (0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 0.0, 0.0, 3.0, 1.0),
        p("iflt", "INDEX Filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0),
        p("istflt", "ST CALCULATION POINT (0 MID POINT, 1 CLOSE, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0),
        p("ifltpd", "INDEX ST PERIOD", 20.0, 0.0, 20.0, 1.0),
        p("ifltfact", "INDEX ST FACTOR", 1.0, 0.0, 20.0, 1.0),
        p("ifltmapd", "INDEX SMA PERIOD", 200.0, 1.0, 300.0, 1.0),
        p("padref", "Reference symbol of 'Pad and align' (-1 = the filter index, 0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 3.0, -1.0, 3.0, 1.0),
        p("pad", "Analysis setting 'Pad and align to the reference symbol' = the filter index (1 yes, as in the team's combo runs; 0 no)", 1.0, 0.0, 1.0, 1.0),
        p("comm", "BROKERAGE for STOCK (the AFL adds it into the GOLD / SILVER prices; set equal to the run's brokerage)", 0.65, 0.0, 5.0, 0.01),
    ]
}

fn sig_combo(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    let flt = flt_from(g);
    let idx = index_of(env, g);
    let refc = {
        let r = g("padref").round();
        if r < 0.0 { idx } else { env.get(INDEX_NAMES[(r as usize).min(3)]) }
    };
    let want = if g("mcxsymbl").round() == 0.0 { "GOLD" } else { "SILVER" };
    let hard = g("hardstop") as f32;
    series
        .iter()
        .map(|s| {
            let p = ComboP {
                dead_yes: g("deadstock").round() == 0.0,
                penny: g("pennyflt") as f32,
                turn: g("turnflt").round() == 1.0,
                exit1: g("exitpct1") as f32,
                mcx: s.name == want,
                comm: g("comm") as f32,
                comm1: g("comm1") as f32,
                pad: g("pad") != 0.0,
            };
            let mut sg = combo_symbol(s, idx, refc, &p, &flt);
            if hard > 0.0 {
                sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
            }
            sg
        })
        .collect()
}

// ---------------------------------------------------------------- family 5: ATHC +x% buy, index ST filter, exit all in the bear market

/// AFL `ATHC plus % buy WITH INDEX FLT N %S EXIT for bull mkt n exit all in bear mkt`: Buy = (latch OR new high) AND Close > trigger AND FLTBuy ...,
/// Sell = swing down (not lower circuit) OR dead stock OR index bear (FLTShort, not lower circuit). History cut at the pad reference.
struct ExP {
    dead_yes: bool,
    penny: f32,
    turn: bool,
    exit_pct: f32,
    athc_pct: f32,
    pad: bool,
}

fn exitall_inner(s: &Series, ix: &Series, refc: Option<&Series>, p: &ExP, flt: &Flt) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
    let (deadbuy, deadsell) = dead_flags(n, p.dead_yes);
    let dead1 = flat30(&o, &h, &l, &c);
    let (upc, _loc_circuit) = circuits(&o, &h, &l, &c);
    // the sheets need the sell deferred on a Close == Low bar (not only on a lower circuit O==C==L)
    let loc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let turnover: Vec<bool> = (0..n).map(|i| if p.turn { s.bars[i].v * c[i] > volsig_loss(dates[i] / 10000) } else { true }).collect();
    let (fltbuy, fltshort) = match (p.pad, refc) {
        (true, Some(rc)) => st_filter_padded(ix, rc, &dates, flt),
        _ => st_filter(&align(ix, &dates), &dates, flt),
    };
    let hcl = hhv_all(&c);
    let sw = pct_swing(&c, p.exit_pct);
    let trend1 = &sw.trend;
    let trigger = trigger_latch(&c, &hcl, trend1, p.athc_pct);
    let latch = athc_latch(&c, &hcl, trend1, Some(&trigger));
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let new_high = i >= 1 && c[i] > hcl[i - 1];
        buy[i] = (latch[i] || new_high) && !is_null(trigger[i]) && c[i] > trigger[i] && fltbuy[i] && deadbuy[i] && !upc[i] && h[i] != c[i] && !dead1[i] && c[i] > p.penny && turnover[i];
        sell[i] = ((trend1[i] == -1 || !latch[i]) && !loc[i]) || deadsell[i] || dead1[i] || (fltshort[i] && !loc[i]);
    }
    let b2 = exrem(&buy, &sell);
    let s2 = exrem(&sell, &b2);
    Signals::new(b2, s2, c.clone(), c)
}

fn exitall_symbol(s: &Series, ix: Option<&Series>, refc: Option<&Series>, p: &ExP, flt: &Flt) -> Signals {
    let n = s.bars.len();
    let Some(ix) = ix else { return blank(n) };
    if let (true, Some(rc)) = (p.pad, refc) {
        let first = rc.bars.first().map_or(0, |b| b.date);
        let cut = s.bars.partition_point(|b| b.date < first);
        if cut > 0 {
            if cut >= n {
                return blank(n);
            }
            let sub = Series { name: s.name.clone(), bars: s.bars[cut..].to_vec() };
            let inner = exitall_inner(&sub, ix, refc, p, flt);
            let mut out = blank(n);
            for i in 0..sub.bars.len() {
                out.buy[cut + i] = inner.buy[i];
                out.sell[cut + i] = inner.sell[i];
                out.buy_price[cut + i] = inner.buy_price[i];
                out.sell_price[cut + i] = inner.sell_price[i];
            }
            return out;
        }
    }
    exitall_inner(s, ix, refc, p, flt)
}

fn schema_exitall() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (ApplyStop percent loss; 0 = off)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; V*C against the AFL's own table)", 0.0, 0.0, 1.0, 1.0),
        p("exitpct", "EXIT %", 16.0, 5.0, 25.0, 1.0),
        p("flttf", "FILTER TIMEFRAME (0 MONTHLY, 1 WEEKLY, 2 DAILY)", 0.0, 0.0, 2.0, 1.0),
        p("prflt", "INDEX (0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 0.0, 0.0, 3.0, 1.0),
        p("iflt", "INDEX Filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0),
        p("istflt", "ST CALCULATION POINT (0 MID POINT, 1 CLOSE, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0),
        p("ifltpd", "INDEX ST PERIOD", 20.0, 0.0, 20.0, 1.0),
        p("ifltfact", "INDEX ST FACTOR", 1.0, 0.0, 20.0, 1.0),
        p("ifltmapd", "INDEX SMA PERIOD", 200.0, 1.0, 300.0, 1.0),
        p("athcbuypct", "BUY PERCENTAGE FROM ATHC", 5.0, 1.0, 10.0, 1.0),
        p("padref", "Reference symbol of 'Pad and align' (-1 = the filter index, 0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 3.0, -1.0, 3.0, 1.0),
        p("pad", "Analysis setting 'Pad and align to the reference symbol' (1 yes, 0 no)", 1.0, 0.0, 1.0, 1.0),
    ]
}

fn sig_exitall(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    let flt = flt_from(g);
    let idx = index_of(env, g);
    let refc = {
        let r = g("padref").round();
        if r < 0.0 { idx } else { env.get(INDEX_NAMES[(r as usize).min(3)]) }
    };
    let p = ExP { dead_yes: g("deadstock").round() == 0.0, penny: g("pennyflt") as f32, turn: g("turnflt").round() == 1.0, exit_pct: g("exitpct") as f32, athc_pct: g("athcbuypct") as f32, pad: g("pad") != 0.0 };
    let hard = g("hardstop") as f32;
    series
        .iter()
        .map(|s| {
            let mut sg = exitall_symbol(s, idx, refc, &p, &flt);
            if hard > 0.0 {
                sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
            }
            sg
        })
        .collect()
}

pub fn variants() -> Vec<VariantDef> {
    let cash = Data::Cash { watchlist: "NSE 500 C&P.tls" };
    vec![
        VariantDef { id: "ytath_plus5pct_buy_pctswing_exit_after", data: cash, schema: schema_after, signals: sig_after },
        VariantDef { id: "ytath_plus5pct_buy_pctswing_exit_after_3", data: cash, schema: schema_after_3, signals: sig_after_3 },
        VariantDef { id: "ytath_plus5pct_buy_pctswing_exit_after_4", data: cash, schema: schema_after_4, signals: sig_after_4 },
        VariantDef { id: "ytath_plus5pct_buy_pctswing_exit_after_6", data: cash, schema: schema_after_6, signals: sig_after_6 },
        VariantDef { id: "ytath_plus_4pct_buy_pctswing_exit_new", data: cash, schema: schema_new, signals: sig_new },
        VariantDef { id: "ytath_plus_5pct_buy_pctswing_exit_avg", data: cash, schema: schema_avg5, signals: sig_avg5 },
        VariantDef { id: "ytath_plus_4pct_buy_pctswing_exit_avg", data: cash, schema: schema_avg4, signals: sig_avg4 },
        VariantDef { id: "ytath_combo_bull_mkt_athc_buy_plus_pctswing", data: cash, schema: schema_combo, signals: sig_combo },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(date: i32, c: f32) -> Bar {
        Bar { date, c, o: c, h: c + 1.0, l: c - 1.0, v: 1000.0, oi: 1.0e7, aux1: 1.0, aux2: 0.0 }
    }

    fn series(name: &str, start: i32, closes: &[f32]) -> Series {
        Series { name: name.into(), bars: closes.iter().enumerate().map(|(i, &c)| bar(start + i as i32, c)).collect() }
    }

    #[test]
    fn the_swing_loop_flips_down_below_the_band_and_back_up_above_it() {
        let c = [100.0f32, 101.0, 102.0, 80.0, 81.0, 120.0];
        let sw = pct_swing(&c, 10.0);
        assert_eq!(sw.trend, vec![1, 1, 1, -1, -1, 1]);
    }

    #[test]
    fn expand_shows_a_monthly_value_on_the_last_bar_of_the_month() {
        let dates = [20200129, 20200130, 20200131, 20200203, 20200204];
        let al = Al { o: vec![1.0; 5], h: vec![1.0; 5], l: vec![1.0; 5], c: vec![1.0; 5] };
        let k = compress(&dates, &al, 0);
        let e = expand(&[7.0, 9.0], &k, 5);
        assert_eq!(e[2], 7.0);
        assert!(is_null(e[0]) && is_null(e[1]));
        assert_eq!(e[3], 7.0);
        assert_eq!(e[4], 9.0);
    }

    #[test]
    fn a_new_high_close_needs_the_trigger_and_a_swing_exit_sells() {
        let s = series("T", 20200101, &[100.0, 101.0, 102.0, 108.0, 109.0, 90.0, 89.0]);
        let sg = new_signals(&s, false, 0.0, false, 10.0, 5.0);
        assert!(sg.buy.iter().any(|&b| b), "the close above the +5% trigger buys");
        assert!(!sg.buy[1] && !sg.buy[2], "the first new highs are below the trigger");
        assert!(sg.sell[5], "the fall through the swing line sells");
    }

    #[test]
    fn the_schemas_carry_the_afl_defaults() {
        for v in variants() {
            let sch = (v.schema)();
            let get = |k: &str| sch.iter().find(|x| x.key == k).map(|x| x.def);
            assert_eq!(get("deadstock"), Some(1.0), "{}", v.id);
        }
        let new = schema_new();
        assert_eq!(new.iter().find(|x| x.key == "athcbuypct").unwrap().def, 4.0);
    }
}
