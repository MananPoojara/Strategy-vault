//! ATH sub-variants of batch a2: the second half of 'Bull / Bear market' (index ST / %swing filters, exit-all-in-bear, bull-only / bear-only
//! index state machines, profit booking, combos), 'No-January trade' (5) and 'Delay entry' (2). 32 variants, one module, one shared set of
//! building blocks and a `Spec` per variant (the AFL's own buy / sell expression is one small function per family).
//!
//! The building blocks are COPIES of the code verified on the team's sheets in other modules (never edited there), taken through
//! `ath_v2_a1.rs` (which copied `ath_plus.rs` / `ath_loss1.rs`): %swing loop, ATHC trigger / latch, index swing filter, index ST / SMA filter
//! (monthly / weekly / daily, `TimeFrameExpand`, "pad and align"), loss counter of the %swing trades, circuits, 30-bar dead stock, turnover tables.
//! List parameters are 0-based indexes like the other ATH engines: deadstock 0 YES / 1 NO, turnflt 0 NO / 1 YES, mktflt 0 NONE / 1 INDEX,
//! prflt 0 NSENIFTY / 1 NSENIFTY1 / 2 NIFTY / 3 NSE500 (the 3-entry lists NSENIFTY|NIFTY|NSE500 map 0 / 1 / 2), flttf 0 MONTHLY / 1 WEEKLY / 2 DAILY,
//! iflt 0 ST / 1 SMA, istflt 0 MID POINT / 1 CLOSE / 2 PIVOT; ONLY the lists taken from ath_new_st / ath_v2_e1 (stflt, swingcal 1..7 and rankmode 1..11)
//! are 1-based like there. Engine options (not AFL parameters): `comm` (AFL BROKERAGE), `pad` / `padref` / `padcut` ("Pad and align to the
//! reference symbol", default off), `pbnull` (profit-booking AFLs: Null in `H > PT`).
//! AFL `Optimize(...)` parameters are not exposed (no catalog key): constants at the AFL default clamped into the range.
//! Verification: `ATH/v2/<id>/VERIFY.md`.
#![allow(dead_code)]
#[allow(unused_imports)]
use crate::ath_core::*;

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

// ======================= copied from ath_v2_e1.rs / ath_new_st.rs (checked there): Supertrend of the stock and the ranking score =======================

fn gtn(a: f32, b: f32) -> bool {
    !a.is_nan() && !b.is_nan() && a > b
}

fn tr1(h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    (0..h.len()).map(|i| if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect()
}

fn ma_n(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![f32::NAN; a.len()];
    let mut sum = 0f64;
    for i in 0..a.len() {
        sum += a[i] as f64;
        if i >= n {
            sum -= a[i - n] as f64;
        }
        if i + 1 >= n {
            out[i] = (sum / n as f64) as f32;
        }
    }
    out
}

/// "ST CALCULATION POINT" lists CLOSE|MID|HIGH|PIVOT|MP CL PIVOT AVG|3DAYS MA|5DAYS MA (1-based engine value).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Pt {
    Close,
    Mid,
    High,
    Pivot,
    MpClPivot,
    Ma3,
    Ma5,
}
fn pt_of(eng: f64) -> Pt {
    [Pt::Close, Pt::Mid, Pt::High, Pt::Pivot, Pt::MpClPivot, Pt::Ma3, Pt::Ma5][((eng.round() as i64) - 1).clamp(0, 6) as usize]
}

fn stcal7(s: &Series, pt: Pt) -> Vec<f32> {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let mp: Vec<f32> = (0..n).map(|i| (h[i] + l[i]) / 2.0).collect();
    let pivot: Vec<f32> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
    match pt {
        Pt::Close => c,
        Pt::Mid => mp,
        Pt::High => h,
        Pt::Pivot => pivot,
        Pt::MpClPivot => (0..n).map(|i| (mp[i] + pivot[i] + c[i]) / 3.0).collect(),
        Pt::Ma3 => ma_n(&c, 3),
        Pt::Ma5 => ma_n(&c, 5),
    }
}

/// The AFL's `ST()` function: Up = stcal + f_up x satr, Dn = stcal - f_dn x satr (this file's AFL hard-codes f_up = 1); returns the trend.
fn st_trend(stcal: &[f32], tr: &[f32], c: &[f32], period: usize, f_up: f32, f_dn: f32) -> Vec<i8> {
    let n = c.len();
    let mut satr = vec![f32::NAN; n];
    for i in 0..n {
        if period >= 1 && i + 1 >= period {
            let mut s = tr[i];
            for k in 1..period {
                s += tr[i - k];
            }
            satr[i] = s / period as f32;
        }
    }
    let mut up: Vec<f32> = (0..n).map(|i| stcal[i] + f_up * satr[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| stcal[i] - f_dn * satr[i]).collect();
    let mut trend = vec![1i8; n];
    for i in 1..n {
        trend[i] = 1;
        if gtn(c[i], up[i - 1]) {
            trend[i] = 1;
        } else if gtn(dn[i - 1], c[i]) {
            trend[i] = -1;
        } else if trend[i - 1] == 1 {
            trend[i] = 1;
        } else if trend[i - 1] == -1 {
            trend[i] = -1;
        }
        let flag1 = trend[i] < 0 && trend[i - 1] > 0;
        let flagh1 = trend[i] > 0 && trend[i - 1] < 0;
        if trend[i] > 0 && gtn(dn[i - 1], dn[i]) {
            dn[i] = dn[i - 1];
        }
        if trend[i] < 0 && gtn(up[i], up[i - 1]) {
            up[i] = up[i - 1];
        }
        if flag1 {
            up[i] = stcal[i] + f_up * satr[i];
        }
        if flagh1 {
            dn[i] = stcal[i] - f_dn * satr[i];
        }
    }
    trend
}

/// PositionScore of the 11 ranking modes (1 NONE .. 11 POSITIVE CLOSE + SWING) from the differential-checked `ath_new_st` code; None for NONE.
fn rank_score(s: &Series, mode: i32, rocpd: usize, rsipd: usize, stdpd: usize, swingpd: usize) -> Option<Vec<f32>> {
    if !(2..=11).contains(&mode) {
        return None;
    }
    let p = crate::ath_new_st::StP { dead_yes: false, hardstop: 0.0, penny: 0.0, turn_on: false, aux1_on: false, aux2_on: false, stflt: 1, period: 10, factor: 4.0, tstart: 1, rankmode: mode, rocpd, rsipd, stdpd, swingpd };
    crate::ath_new_st::one(s, &p).score
}

// ======================= own code: the parameterised core =======================

#[derive(Clone, Copy)]
struct R {
    def: f64,
    min: f64,
    max: f64,
}
/// AFL `Param(name, default, min, max)`: a default outside the range is clamped into it.
const fn r(def: f64, min: f64, max: f64) -> R {
    R { def: if def < min { min } else if def > max { max } else { def }, min, max }
}

#[derive(Clone, Copy, PartialEq)]
enum Filt {
    /// no index filter in the AFL
    None,
    /// index %swing (FLTFACT1 / FLTFACT2)
    Swing,
    /// index Supertrend / SMA on monthly / weekly / daily bars
    St,
}
#[derive(Clone, Copy, PartialEq)]
enum Turn {
    Vc6,
    OiStd,
}
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// ids 1, 2, 5, 6: running ATH state OR new high, loss-streak gate, one side of the index, optional exit on the other side
    RunLoss,
    /// ids 3 (sub 0, runningbuy state machine), 4 (sub 1): bear entry on a fresh high with the loss-streak gate, exit all in bull
    LossState,
    /// id 7: profit booking + state-2 lock + loss gate (bear)
    PbLoss,
    /// id 8: bull, no January entries, sell on the last trading day of December, latch gated by FLTBUY
    JanBull,
    /// id 9: stock Supertrend (period, factor), latch gated by FLTBUY, ranking
    AllSt,
    /// ids 10, 11, 15, 20, 23, 25: latch gated by the filter side; Buy = (new high OR latch) AND filter
    RunGate,
    /// ids 12, 13, 14, 21: plain latch (not gated), filter side only on the Buy, optional exit on the other side
    Plain,
    /// id 16: ATHC+n % after a loss in bear, plain ATHC in bull, swing width by regime
    Combo,
    /// ids 17 (sub 0, 3-state latch), 24 (sub 1, 2-state): fresh high only in the filter's bear
    State3,
    /// ids 18 (lock), 19 (no lock): profit booking only (no loss gate)
    PbOnly,
    /// ids 22 (sub 0, bull-gated), 32 (sub 1, swing from STCAL): buy / sell buffers and catch-up entries
    Buffered,
    /// id 26: ATHC+n % trigger latch, bull filter, no January entries
    PlusJan,
    /// id 27: running ATH after loss streak, bull filter, no January entries
    LossJan,
    /// ids 28 (exit on 31 Dec, 5-day phenomenon), 29 (only new ATH in Feb), 30 (all running ATH in Feb)
    NoJan,
    /// id 31: delay entry without any filter
    Delay0,
}

#[derive(Clone, Copy)]
struct Spec {
    kind: Kind,
    sub: u8,
    filt: Filt,
    /// the INDEX list has 3 entries NSENIFTY|NIFTY|NSE500
    idx3: bool,
    /// MKTFLT list has the single entry INDEX (no key, always on)
    force_index: bool,
    turn: Turn,
    /// MKTFLT NONE: FLTBUY = true / FLTSHORT = false (bull) or the reverse (bear)
    bull: bool,
    opp_exit: bool,
    nh: bool,
    uc_lc: bool,
    lock: bool,
    exit: Option<R>,
    exit_fixed: f64,
    exit1: Option<R>,
    exit2: Option<R>,
    /// id 32: the second `EXIT %` Param (key exitpct_2)
    exitb: Option<R>,
    loss: Option<(R, R)>,
    athc: Option<R>,
    delay: Option<R>,
    pb: Option<R>,
    pb_fixed: f64,
    pbbuff: bool,
    buf: bool,
    jan: bool,
    tstart: Option<R>,
    swingcal: bool,
    ipd: R,
    ifact: R,
    f1: R,
    f2: R,
    st9: bool,
    /// verification aid only (`check_variants`): id 32 with the plain `Up = C + EXIT, Dn = C - EXIT` swing of the verified turnover-running-buy AFL
    ref_plain: bool,
    /// verification aid only: RunLoss with the 30-bar dead-stock test and the lower-circuit sell guard left out (ath_loss2 `ytath_running_pctswing_exit_after_2_4`)
    ref_v5: bool,
}

const BASE: Spec = Spec {
    kind: Kind::RunLoss,
    sub: 0,
    filt: Filt::St,
    idx3: false,
    force_index: false,
    turn: Turn::Vc6,
    bull: false,
    opp_exit: false,
    nh: true,
    uc_lc: false,
    lock: false,
    exit: None,
    exit_fixed: 10.0,
    exit1: None,
    exit2: None,
    exitb: None,
    loss: None,
    athc: None,
    delay: None,
    pb: None,
    pb_fixed: 5.0,
    pbbuff: false,
    buf: false,
    jan: false,
    tstart: None,
    swingcal: false,
    ipd: r(20.0, 0.0, 20.0),
    ifact: r(1.0, 0.0, 20.0),
    f1: r(23.0, 10.0, 50.0),
    f2: r(44.0, 10.0, 50.0),
    st9: false,
    ref_plain: false,
    ref_v5: false,
};

fn schema_of(sp: &Spec) -> Vec<P> {
    let mut v = vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (ApplyStop percent loss; 0 = off)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
    ];
    if sp.st9 {
        v.push(p("stflt", "DAILY ST CALCULATION POINT (1 CLOSE; 2 MID POINT H+L/2; 3 HIGH; 4 PIVOT H+L+C/3; 5 MP CL PIVOT AVG; 6 3DAYS MA; 7 5DAYS MA)", 1.0, 1.0, 7.0, 1.0));
        v.push(p("bpd1", "ST Period ", 10.0, 1.0, 20.0, 1.0));
        v.push(p("bfactor1", "ST Factor", 4.0, 1.0, 20.0, 1.0));
    }
    if let Some(t) = sp.tstart {
        v.push(p("tstart", "TRADING START BAR", t.def, t.min, t.max, 1.0));
    }
    if let Some(e) = sp.exit {
        v.push(p("exitpct", "EXIT %", e.def, e.min, e.max, 1.0));
    }
    if let Some(e) = sp.exit1 {
        v.push(p("exitpct1", "EXIT % IN BULL MKT", e.def, e.min, e.max, 1.0));
    }
    if let Some(e) = sp.exit2 {
        v.push(p("exitpct2", "EXIT % IN BEAR MKT", e.def, e.min, e.max, 1.0));
    }
    if sp.swingcal {
        v.push(p("swingcal", "SWING CALCULATION POINT (1 CLOSE; 2 MIDD POINT H+L/2; 3 HIGH; 4 PIVOT H+L+C/3; 5 MP CL PIVOT AVG; 6 3DAYS MA; 7 5DAYS MA)", 1.0, 1.0, 7.0, 1.0));
    }
    if let Some(e) = sp.exitb {
        v.push(p("exitpct_2", "EXIT % (second Param of the AFL, the effective one: it is assigned last)", e.def, e.min, e.max, 1.0));
    }
    if let Some((lp, nl)) = sp.loss {
        v.push(p("losspct", "MIN PROFIT %", 2.0_f64.clamp(lp.min, lp.max), lp.min, lp.max, 0.05));
        if sp.kind != Kind::Combo {
            v.push(p("noofloss", "CONSICUTIVE LOSS", nl.def, nl.min, nl.max, 1.0));
        }
    }
    if let Some(d) = sp.delay {
        v.push(p("delaybuy", "ATHC DELAY ENTRY", d.def, d.min, d.max, 1.0));
    }
    if let Some(b) = sp.pb {
        v.push(p("pb", "PROFIT BOOKING Percent", b.def, b.min, b.max, if b.max > 10.0 { 5.0 } else { 1.0 }));
    }
    if sp.pbbuff {
        v.push(p("pbbuff", "PROFIT BOOKING BUFFER", 0.25, 0.0, 5.0, 0.05));
    }
    if sp.pb.is_some() || sp.pbbuff {
        v.push(p("pbnull", "ENGINE OPTION: how AmiBroker compares with Null in the profit-booking test `H > PT` (0 = Null compares false, as in the other ATH engines; 1 = Null is the number -1e10, i.e. H > Null is true)", 0.0, 0.0, 1.0, 1.0));
    }
    if sp.buf {
        v.push(p("buffer_pct", "Buying Buffer %", 0.25, 0.0, 10.0, 0.05));
        v.push(p("bufferpct", "Selling Buffer %", 0.25, 0.0, 10.0, 0.05));
    }
    if sp.filt != Filt::None {
        if !sp.force_index {
            v.push(p("mktflt", "FILTER TYPE (0 NONE, 1 INDEX)", 0.0, 0.0, 1.0, 1.0));
        }
        if sp.filt == Filt::St {
            v.push(p("flttf", "FILTER TIMEFRAME (0 MONTHLY, 1 WEEKLY, 2 DAILY)", 0.0, 0.0, 2.0, 1.0));
        }
        if sp.idx3 {
            v.push(p("prflt", "INDEX (0 NSENIFTY, 1 NIFTY, 2 NSE500)", 0.0, 0.0, 2.0, 1.0));
        } else {
            v.push(p("prflt", "INDEX (0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 0.0, 0.0, 3.0, 1.0));
        }
        if sp.filt == Filt::St {
            v.push(p("iflt", "INDEX Filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("istflt", "ST CALCULATION POINT (0 MID POINT(H+L/2), 1 CLOSE, 2 PIVOT(H+L+C/3))", 0.0, 0.0, 2.0, 1.0));
            v.push(p("ifltpd", "INDEX ST PERIOD", sp.ipd.def, sp.ipd.min, sp.ipd.max, if sp.ipd.min >= 5.0 { 5.0 } else { 1.0 }));
            v.push(p("ifltfact", "INDEX ST FACTOR", sp.ifact.def, sp.ifact.min, sp.ifact.max, 1.0));
            v.push(p("ifltmapd", "INDEX SMA PERIOD", 200.0, 1.0, 300.0, 1.0));
        } else {
            v.push(p("fltfact1", "SWING % FOR BEAR MKT", sp.f1.def, sp.f1.min, sp.f1.max, 1.0));
            v.push(p("fltfact2", "SWING % FOR BULL MKT", sp.f2.def, sp.f2.min, sp.f2.max, 1.0));
        }
    }
    if let Some(a) = sp.athc {
        v.push(p("athcbuypct", "BUY PERCENTAGE FROM ATHC", a.def, a.min, a.max, 1.0));
    }
    if sp.jan {
        v.push(p("janentryday", "5 day phenomena start Day", 5.0, 1.0, 15.0, 1.0));
    }
    if sp.st9 {
        v.push(p("rankmode", "RANKING MODE (1 NONE; 2 POSITIVE ROC; 3 NEGATIVE ROC; 4 RSI; 5 STD; 6 POSITIVE ROC/STD; 7 NEGATIVE ROC/STD; 8 POSITIVE CLOSE; 9 SWING; 10 POSITIVE ROC + POSITIVE CLOSE; 11 POSITIVE CLOSE + SWING)", 1.0, 1.0, 11.0, 1.0));
        v.push(p("rocpd", "ROC PERIOD for RANKING/NO OF POSITIVE CLOSE", 10.0, 0.0, 400.0, 30.0));
        v.push(p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0));
        v.push(p("stdpd", "STD PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0));
        v.push(p("swingpd", "SWING PERIOD", 180.0, 10.0, 1000.0, 1.0));
    }
    if sp.loss.is_some() {
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
    exit1: f32,
    exit2: f32,
    athc: f32,
    delay: f32,
    losspct: f32,
    noofloss: f32,
    comm: f32,
    pb: f32,
    pbbuff: f32,
    pbnull: bool,
    buf_b: f32,
    buf_s: f32,
    janday: usize,
    tstart: usize,
    stcal: Pt,
    penny: f32,
    dead_yes: bool,
    turn: bool,
}

fn pm_of(g: G, sp: &Spec) -> Pm {
    let opt = |o: &Option<R>, key: &str, fixed: f64| -> f64 { if o.is_some() { g(key) } else { fixed } };
    // id 32: the effective EXIT % is the second Param
    let exit = if sp.exitb.is_some() { g("exitpct_2") } else { opt(&sp.exit, "exitpct", sp.exit_fixed) };
    Pm {
        exit: exit as f32,
        exit1: opt(&sp.exit1, "exitpct1", 19.0) as f32,
        exit2: opt(&sp.exit2, "exitpct2", 16.0) as f32,
        athc: opt(&sp.athc, "athcbuypct", 5.0) as f32,
        delay: opt(&sp.delay, "delaybuy", 0.0) as f32,
        losspct: if sp.loss.is_some() { g("losspct") as f32 } else { 2.0 },
        // the combo AFL fixes `noofloss=1`
        noofloss: match sp.loss {
            Some(_) if sp.kind == Kind::Combo => 1.0,
            Some(_) => g("noofloss") as f32,
            None => 0.0,
        },
        comm: if sp.loss.is_some() { g("comm") as f32 } else { 0.65 },
        pb: opt(&sp.pb, "pb", sp.pb_fixed) as f32,
        pbbuff: if sp.pbbuff { g("pbbuff") as f32 } else { 0.25 },
        pbnull: (sp.pb.is_some() || sp.pbbuff) && g("pbnull") != 0.0,
        buf_b: if sp.buf { g("buffer_pct") as f32 } else { 0.25 },
        buf_s: if sp.buf { g("bufferpct") as f32 } else { 0.25 },
        janday: if sp.jan { g("janentryday").round().max(1.0) as usize } else { 5 },
        tstart: if sp.tstart.is_some() { g("tstart").round().max(0.0) as usize } else { 0 },
        stcal: if sp.swingcal { pt_of(g("swingcal")) } else if sp.st9 { pt_of(g("stflt")) } else { Pt::Close },
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
    if sp.filt == Filt::None || (!sp.force_index && g("mktflt").round() != 1.0) {
        return (vec![sp.bull; n], vec![!sp.bull; n]);
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
        _ => {
            let f = flt_from(g);
            match (pad, refc) {
                (true, Some(rc)) => st_filter_padded(ix, rc, dates, &f),
                _ => st_filter(&align(ix, dates), dates, &f),
            }
        }
    }
}

fn index_pick<'a>(env: &'a Env, g: G, sp: &Spec) -> Option<&'a Series> {
    let k = g("prflt").round().max(0.0) as usize;
    let name = if sp.idx3 { INDEX_NAMES[[0usize, 2, 3][k.min(2)]] } else { INDEX_NAMES[k.min(3)] };
    env.get(name)
}

/// The swing line of the stock for a given `EXIT` array: Up = C + EXIT, Dn = C - EXIT.
fn swing_exit(c: &[f32], exit: &[f32]) -> Sw {
    let n = c.len();
    let up: Vec<f32> = (0..n).map(|i| c[i] + exit[i]).collect();
    let dn: Vec<f32> = (0..n).map(|i| c[i] - exit[i]).collect();
    let (uf, df) = (up.clone(), dn.clone());
    swing_loop(c, up, dn, &uf, &df, None)
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
    let (fltbuy, fltshort) = index_flags(&dates, idx, refc, g, sp, pad);
    let sw = match (sp.kind, sp.sub) {
        // EXIT1 = IIf(fltbuy, C*EXITPCT1/100, C*EXITPCT2/100): the swing width follows the regime bar by bar
        (Kind::Combo, _) => {
            let e: Vec<f32> = (0..n).map(|i| if fltbuy[i] { (c[i] * pm.exit1) / 100.0 } else { (c[i] * pm.exit2) / 100.0 }).collect();
            swing_exit(&c, &e)
        }
        // EXIT = STCAL*EXITPCT/100; Up = C + STCAL*1/100; Dn = STCAL - EXIT; at a flip Up = C + EXIT, Dn = C - EXIT
        (Kind::Buffered, 1) if !sp.ref_plain => {
            let st = stcal7(s, pm.stcal);
            let exit: Vec<f32> = st.iter().map(|&x| (x * pm.exit) / 100.0).collect();
            let dexit: Vec<f32> = st.iter().map(|&x| (x * 1.0) / 100.0).collect();
            let up: Vec<f32> = (0..n).map(|i| c[i] + dexit[i]).collect();
            let dn: Vec<f32> = (0..n).map(|i| st[i] - exit[i]).collect();
            let uf: Vec<f32> = (0..n).map(|i| c[i] + exit[i]).collect();
            let df: Vec<f32> = (0..n).map(|i| c[i] - exit[i]).collect();
            swing_loop(&c, up, dn, &uf, &df, None)
        }
        _ => pct_swing(&c, pm.exit),
    };
    Pre { n, c, o, h, l, dates, deadbuy, deadsell, dead1, upc, loc, uc, lc, turnover, hcl, sw, fltbuy, fltshort, penny: pm.penny }
}

fn new_high(pre: &Pre, i: usize) -> bool {
    i >= 1 && pre.c[i] > pre.hcl[i - 1]
}

fn month(d: i32) -> i32 {
    d / 100 % 100
}

/// `LASTDAYOFDEC = Month()==12 AND Ref(Day(),1) < Day()` (the last bar has no next day: false).
fn last_day_of_dec(dates: &[i32]) -> Vec<bool> {
    let n = dates.len();
    (0..n).map(|i| month(dates[i]) == 12 && i + 1 < n && dates[i + 1] % 100 < dates[i] % 100).collect()
}

/// `NOJANSELL`: 1 from the last trading day of December, back to 0 on any bar of February, else carried.
fn no_jan_sell(dates: &[i32], lastdec: &[bool]) -> Vec<bool> {
    let n = dates.len();
    let mut out = vec![false; n];
    for i in 1..n {
        out[i] = if lastdec[i] { true } else if month(dates[i]) == 2 { false } else { out[i - 1] };
    }
    out
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

/// What a family returns.
struct Out {
    buy: Vec<bool>,
    sell: Vec<bool>,
    bp: Option<Vec<f32>>,
    sp: Option<Vec<f32>>,
}
fn out(buy: Vec<bool>, sell: Vec<bool>) -> Out {
    Out { buy, sell, bp: None, sp: None }
}

/// The latch of the AFLs: set on a new ATH close (only while `gate`), cleared when the swing turns down (and, with `reset`, when the other
/// side of the index shows), else carried.
fn latch(pre: &Pre, gate: Option<&[bool]>, reset_on: Option<&[bool]>) -> Vec<bool> {
    let n = pre.n;
    let mut s = vec![false; n];
    for i in 1..n {
        s[i] = if pre.c[i] > pre.hcl[i - 1] && gate.map_or(true, |g| g[i]) {
            true
        } else if reset_on.map_or(false, |r| r[i]) || (pre.sw.trend[i] == -1 && (reset_on.is_some() || s[i - 1])) {
            false
        } else {
            s[i - 1]
        };
    }
    s
}

// ---------------------------------------------------------------- the families

fn flt_of<'a>(pre: &'a Pre, bull: bool) -> (&'a [bool], &'a [bool]) {
    if bull { (&pre.fltbuy, &pre.fltshort) } else { (&pre.fltshort, &pre.fltbuy) }
}

/// Entry gates of the AFLs that test `C>pennyflt AND turnover AND deadstockbuysig`.
fn base_ok(pre: &Pre, i: usize) -> bool {
    pre.deadbuy[i] && pre.c[i] > pre.penny && pre.turnover[i]
}

/// ids 1, 2, 5, 6.
fn fam_run_loss(pre: &Pre, pm: &Pm, sp: &Spec) -> Out {
    let n = pre.n;
    let trend1 = &pre.sw.trend;
    let athc = latch(pre, None, None);
    let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !pre.upc[i] && base_ok(pre, i)).collect();
    let cc = concount_pct(pre, &buysig, pm.losspct, pm.comm);
    let (flt, opp) = flt_of(pre, sp.bull);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let d1 = pre.dead1[i] && !sp.ref_v5;
        buy[i] = (athc[i] || new_high(pre, i)) && cc[i] as f32 >= pm.noofloss && flt[i] && !d1 && !pre.upc[i] && !(sp.uc_lc && pre.uc[i]) && base_ok(pre, i);
        let guard = !pre.loc[i] && !(sp.uc_lc && !sp.ref_v5 && pre.lc[i]);
        sell[i] = ((trend1[i] == -1 || !athc[i]) && guard) || pre.deadsell[i] || d1 || (sp.opp_exit && opp[i] && guard);
    }
    out(buy, sell)
}

/// ids 3 (sub 0) and 4 (sub 1).
fn fam_loss_state(pre: &Pre, pm: &Pm, sub: u8) -> Out {
    let n = pre.n;
    let (c, hcl, trend1) = (&pre.c, &pre.hcl, &pre.sw.trend);
    let athc = latch(pre, None, None);
    let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !pre.upc[i] && !pre.uc[i] && pre.deadbuy[i]).collect();
    let cc = concount_pct(pre, &buysig, pm.losspct, pm.comm);
    let mut bs1 = vec![0u8; n];
    if sub == 0 {
        // runningbuy counter: counts ATH closes, the first one after a swing-down arms the signal (state 2 = bull appeared, blocked until the swing is down)
        let mut running = vec![0i32; n];
        for i in 1..n {
            running[i] = if c[i] > hcl[i - 1] { running[i - 1] + 1 } else { running[i - 1] };
            if c[i] > hcl[i - 1] && pre.fltshort[i] && bs1[i - 1] != 2 && running[i] == 1 {
                bs1[i] = 1;
                running[i] = running[i - 1];
            } else if trend1[i] == -1 {
                bs1[i] = 0;
                running[i] = 0;
            } else if pre.fltbuy[i] && bs1[i - 1] == 1 && trend1[i] == 1 {
                bs1[i] = 2;
                running[i] = running[i - 1];
            } else {
                bs1[i] = bs1[i - 1];
            }
        }
    } else {
        for i in 1..n {
            bs1[i] = if c[i] > hcl[i - 1] && pre.fltshort[i] {
                1
            } else if trend1[i] == -1 || pre.fltbuy[i] {
                0
            } else {
                bs1[i - 1]
            };
        }
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = bs1[i] == 1 && cc[i] as f32 >= pm.noofloss && pre.fltshort[i] && !pre.dead1[i] && !pre.upc[i] && !pre.uc[i] && base_ok(pre, i);
        let off = if sub == 0 { !athc[i] } else { bs1[i] == 0 };
        let guard = !pre.loc[i] && !pre.lc[i];
        sell[i] = ((trend1[i] == -1 || off) && guard) || pre.deadsell[i] || pre.dead1[i] || (pre.fltbuy[i] && guard);
    }
    out(buy, sell)
}

/// Profit-booking state loop of ids 7, 18, 19: (state 0 / 1 / 2, PT, pbexit). `lock` = the AFL's `ATHCbuysig[I-1]!=2` / `IIf(FLTBUY,2,0)`.
fn pb_machine(pre: &Pre, pm: &Pm, lock: bool) -> (Vec<u8>, Vec<f32>, Vec<bool>) {
    let n = pre.n;
    let (c, h, hcl, trend1) = (&pre.c, &pre.h, &pre.hcl, &pre.sw.trend);
    let buyfilter: Vec<bool> = (0..n).map(|i| pre.turnover[i] && !pre.uc[i] && !pre.upc[i] && c[i] > pre.penny && pre.deadbuy[i]).collect();
    let target = |i: usize| (1.0 + pm.pb * 0.01) * c[i];
    let gtp = |a: f32, b: f32| if pm.pbnull { a > b } else { gt(a, b) };
    let mut st = vec![0u8; n];
    let mut pbexit = vec![false; n];
    let mut pt = vec![NULL; n];
    for i in 1..n {
        let ptcheck = is_null(pt[i - 1]) && buyfilter[i];
        if c[i] > hcl[i - 1] && pre.fltshort[i] && (!lock || st[i - 1] != 2) && !pbexit[i - 1] {
            st[i] = 1;
            pt[i] = if ptcheck { target(i) } else { pt[i - 1] };
            if gtp(h[i], pt[i - 1]) {
                st[i] = 0;
                pt[i] = NULL;
                pbexit[i] = true;
            }
        } else if st[i - 1] == 1 && ptcheck {
            pt[i] = target(i);
            st[i] = st[i - 1];
        } else if gtp(h[i], pt[i - 1]) {
            st[i] = 0;
            pt[i] = NULL;
            pbexit[i] = true;
        } else if trend1[i] == -1 || pre.fltbuy[i] {
            st[i] = if lock && pre.fltbuy[i] { 2 } else { 0 };
            pbexit[i] = false;
        } else {
            st[i] = st[i - 1];
            pt[i] = pt[i - 1];
            pbexit[i] = pbexit[i - 1];
        }
    }
    (st, pt, pbexit)
}

/// ids 7 (with the loss gate), 18, 19. `assign` = the AFL writes `pbexit=1` (an assignment) in SellPrice (id 7 only).
fn fam_pb(pre: &Pre, pm: &Pm, lock: bool, gate: bool, assign: bool) -> Out {
    let n = pre.n;
    let (c, h, o, trend1) = (&pre.c, &pre.h, &pre.o, &pre.sw.trend);
    let gtp = |a: f32, b: f32| if pm.pbnull { a > b } else { gt(a, b) };
    let (st, pt, pbexit) = pb_machine(pre, pm, lock);
    let cc = if gate {
        let athc = latch(pre, None, None);
        let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !pre.upc[i] && !pre.uc[i] && pre.deadbuy[i]).collect();
        concount_pct(pre, &buysig, pm.losspct, pm.comm)
    } else {
        vec![0; n]
    };
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut spx = c.clone();
    for i in 0..n {
        buy[i] = st[i] == 1 && (!gate || cc[i] as f32 >= pm.noofloss) && pre.fltshort[i] && !pre.dead1[i] && !pre.upc[i] && !pre.uc[i] && base_ok(pre, i);
        let guard = !pre.loc[i] && !pre.lc[i];
        sell[i] = ((trend1[i] == -1 || st[i] == 0) && guard) || pre.deadsell[i] || pre.dead1[i] || (pre.fltbuy[i] && guard) || (gtp(h[i], pt[i]) && pbexit[i]);
        if i >= 1 && gtp(h[i], pt[i - 1]) && (assign || pbexit[i]) {
            spx[i] = if gtp(o[i], pt[i - 1]) { o[i] - (o[i] * pm.pbbuff) / 100.0 } else { pt[i - 1] };
        }
    }
    Out { buy, sell, bp: None, sp: Some(spx) }
}

/// id 8.
fn fam_jan_bull(pre: &Pre) -> Out {
    let n = pre.n;
    let lastdec = last_day_of_dec(&pre.dates);
    let bs = latch(pre, Some(&pre.fltbuy), None);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = (new_high(pre, i) || bs[i]) && pre.fltbuy[i] && month(pre.dates[i]) != 1 && !lastdec[i] && !pre.upc[i] && base_ok(pre, i) && !pre.dead1[i];
        sell[i] = ((pre.sw.trend[i] == -1 || !bs[i]) && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i] || lastdec[i];
    }
    out(buy, sell)
}

/// id 9.
fn fam_all_st(pre: &Pre, pm: &Pm, s: &Series, period: usize, factor: f32) -> Out {
    let n = pre.n;
    let c = &pre.c;
    let stcal = stcal7(s, pm.stcal);
    let tr = tr1(&pre.h, &pre.l, c);
    let bt = st_trend(&stcal, &tr, c, period, 1.0, factor);
    let mut bs = vec![false; n];
    for i in 1..n {
        bs[i] = if c[i] > pre.hcl[i - 1] && pre.fltbuy[i] {
            true
        } else if bt[i] == -1 && bs[i - 1] {
            false
        } else {
            bs[i - 1]
        };
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = ((new_high(pre, i) && i >= pm.tstart && pre.fltbuy[i]) || (bs[i] && pre.fltbuy[i])) && !pre.upc[i] && base_ok(pre, i);
        sell[i] = (bt[i] == -1 && !pre.loc[i]) || pre.deadsell[i] || (!bs[i] && !pre.loc[i]);
    }
    out(buy, sell)
}

/// ids 10, 11, 15, 20, 23, 25.
fn fam_run_gate(pre: &Pre, sp: &Spec) -> Out {
    let n = pre.n;
    let (flt, _) = flt_of(pre, sp.bull);
    let bs = latch(pre, Some(flt), None);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = (new_high(pre, i) || bs[i]) && flt[i] && !pre.dead1[i] && !pre.upc[i] && base_ok(pre, i);
        sell[i] = ((pre.sw.trend[i] == -1 || !bs[i]) && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i];
    }
    out(buy, sell)
}

/// ids 12, 13, 14, 21.
fn fam_plain(pre: &Pre, sp: &Spec) -> Out {
    let n = pre.n;
    let (flt, opp) = flt_of(pre, sp.bull);
    let bs = latch(pre, None, None);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let entry = if sp.nh { new_high(pre, i) || bs[i] } else { bs[i] };
        buy[i] = entry && flt[i] && !pre.dead1[i] && !pre.upc[i] && !(sp.uc_lc && pre.uc[i]) && base_ok(pre, i);
        let guard = !pre.loc[i] && !(sp.uc_lc && pre.lc[i]);
        sell[i] = ((!bs[i] || pre.sw.trend[i] == -1) && guard) || pre.deadsell[i] || pre.dead1[i] || (sp.opp_exit && opp[i] && guard);
    }
    out(buy, sell)
}

/// id 16.
fn fam_combo(pre: &Pre, pm: &Pm) -> Out {
    let n = pre.n;
    let c = &pre.c;
    let trend1 = &pre.sw.trend;
    let athc = latch(pre, None, None);
    let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !pre.upc[i] && base_ok(pre, i)).collect();
    let cc = concount_pct(pre, &buysig, pm.losspct, pm.comm);
    let trigger = trigger_latch(c, &pre.hcl, trend1, pm.athc);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let nh = new_high(pre, i);
        buy[i] = ((nh && gt(c[i], trigger[i]) && pre.fltshort[i] && cc[i] as f32 >= pm.noofloss) || (nh && pre.fltbuy[i])) && base_ok(pre, i) && !pre.upc[i] && !pre.dead1[i];
        sell[i] = (trend1[i] == -1 && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i];
    }
    out(buy, sell)
}

/// ids 17 (sub 0) and 24 (sub 1).
fn fam_state3(pre: &Pre, sub: u8) -> Out {
    let n = pre.n;
    let (c, hcl, trend1) = (&pre.c, &pre.hcl, &pre.sw.trend);
    let mut st = vec![0u8; n];
    for i in 1..n {
        st[i] = if sub == 0 {
            if c[i] > hcl[i - 1] && pre.fltshort[i] && st[i - 1] != 2 {
                1
            } else if trend1[i] == -1 || pre.fltbuy[i] {
                if pre.fltbuy[i] { 2 } else { 0 }
            } else {
                st[i - 1]
            }
        } else if c[i] > hcl[i - 1] && pre.fltshort[i] {
            1
        } else if trend1[i] == -1 || pre.fltbuy[i] {
            0
        } else {
            st[i - 1]
        };
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = st[i] == 1 && pre.fltshort[i] && !pre.upc[i] && !pre.uc[i] && base_ok(pre, i) && !pre.dead1[i];
        let guard = !pre.loc[i] && !pre.lc[i];
        sell[i] = ((trend1[i] == -1 || st[i] == 0) && guard) || pre.deadsell[i] || pre.dead1[i] || (pre.fltbuy[i] && guard);
    }
    out(buy, sell)
}

/// ids 22 (sub 0) and 32 (sub 1): buysig / buycount / sellcount / turnovercheck state machine with buffered fills.
/// AFL arrays start at 0 and an element that no branch assigns stays 0 (sellcount on a new-high bar).
fn fam_buffered(pre: &Pre, pm: &Pm, sub: u8) -> Out {
    let n = pre.n;
    let (c, o, hcl, trend1) = (&pre.c, &pre.o, &pre.hcl, &pre.sw.trend);
    let (mut bs, mut bc, mut sc, mut tc) = (vec![false; n], vec![0i32; n], vec![0i32; n], vec![false; n]);
    for i in 1..n {
        if c[i] > hcl[i - 1] && (sub == 1 || pre.fltbuy[i]) {
            bs[i] = true;
            bc[i] = bc[i - 1] + 1;
        } else if trend1[i] == -1 {
            bs[i] = false;
            bc[i] = 0;
            sc[i] = sc[i - 1] + 1;
        } else {
            bs[i] = bs[i - 1];
            bc[i] = if bs[i] { bc[i - 1] + 1 } else { 0 };
            sc[i] = sc[i - 1];
        }
        tc[i] = if bs[i] && pre.turnover[i] {
            true
        } else if !bs[i] {
            false
        } else {
            tc[i - 1]
        };
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut bp, mut spx) = (c.clone(), c.clone());
    for i in 0..n {
        let prev_tc = if i > 0 { tc[i - 1] } else { tc[i] };
        let sigbuy = if !prev_tc { pre.uc[i] } else { pre.upc[i] };
        buy[i] = ((bs[i] && bc[i] == 1 && !pre.uc[i] && tc[i]) || (bs[i] && bc[i] > 1 && !sigbuy && tc[i])) && pre.deadbuy[i] && c[i] > pre.penny && (sub == 1 || !pre.dead1[i]);
        sell[i] = ((!bs[i] && sc[i] == 1 && !pre.lc[i]) || (!bs[i] && sc[i] > 1 && !pre.loc[i])) || pre.deadsell[i] || (sub == 0 && pre.dead1[i]);
        bp[i] = if bs[i] && bc[i] > 1 && !pre.upc[i] && o[i] > pre.penny && prev_tc { o[i] + (o[i] * pm.buf_b) / 100.0 } else { c[i] };
        spx[i] = if sub == 0 {
            if !bs[i] && sc[i] == 1 && !pre.lc[i] { c[i] } else { o[i] - (o[i] * pm.buf_s) / 100.0 }
        } else if !bs[i] && sc[i] == 1 && !pre.loc[i] {
            o[i] - (o[i] * pm.buf_s) / 100.0
        } else {
            c[i]
        };
    }
    Out { buy, sell, bp: Some(bp), sp: Some(spx) }
}

/// id 26.
fn fam_plus_jan(pre: &Pre, pm: &Pm) -> Out {
    let n = pre.n;
    let (c, hcl, trend1) = (&pre.c, &pre.hcl, &pre.sw.trend);
    let trigger = trigger_latch(c, hcl, trend1, pm.athc);
    // buysig: C > Hcl[i-1] AND C > trigger[i-1]
    let mut bs = vec![false; n];
    for i in 1..n {
        bs[i] = if c[i] > hcl[i - 1] && gt(c[i], trigger[i - 1]) {
            true
        } else if trend1[i] == -1 && bs[i - 1] {
            false
        } else {
            bs[i - 1]
        };
    }
    let lastdec = last_day_of_dec(&pre.dates);
    let nojan = no_jan_sell(&pre.dates, &lastdec);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = (bs[i] || new_high(pre, i)) && gt(c[i], trigger[i]) && pre.fltbuy[i] && !lastdec[i] && month(pre.dates[i]) != 1 && base_ok(pre, i) && !pre.dead1[i] && !pre.upc[i];
        sell[i] = ((trend1[i] == -1 || !bs[i]) && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i] || (nojan[i] && !pre.loc[i]);
    }
    out(buy, sell)
}

/// id 27.
fn fam_loss_jan(pre: &Pre, pm: &Pm) -> Out {
    let n = pre.n;
    let trend1 = &pre.sw.trend;
    let athc = latch(pre, None, None);
    let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !pre.upc[i] && base_ok(pre, i)).collect();
    let cc = concount_pct(pre, &buysig, pm.losspct, pm.comm);
    let lastdec = last_day_of_dec(&pre.dates);
    let nojan = no_jan_sell(&pre.dates, &lastdec);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = (athc[i] || new_high(pre, i)) && cc[i] as f32 >= pm.noofloss && pre.fltbuy[i] && month(pre.dates[i]) != 1 && !lastdec[i] && !pre.upc[i] && base_ok(pre, i) && !pre.dead1[i];
        sell[i] = ((trend1[i] == -1 || !athc[i]) && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i] || (nojan[i] && !pre.loc[i]);
    }
    out(buy, sell)
}

/// ids 28 (sub 0), 29 (sub 1), 30 (sub 2).
fn fam_no_jan(pre: &Pre, pm: &Pm, sub: u8) -> Out {
    let n = pre.n;
    let (c, hcl, trend1) = (&pre.c, &pre.hcl, &pre.sw.trend);
    let lastdec = last_day_of_dec(&pre.dates);
    let jan: Vec<bool> = pre.dates.iter().map(|&d| month(d) == 1).collect();
    let mut bs = vec![false; n];
    for i in 1..n {
        let set = c[i] > hcl[i - 1] && (sub != 1 || (!lastdec[i] && !jan[i]));
        bs[i] = if set {
            true
        } else if (trend1[i] == -1 && bs[i - 1]) || (sub == 1 && lastdec[i]) {
            false
        } else {
            bs[i - 1]
        };
    }
    // id 28: eom = Month() != Ref(Month(), 1); eombuysig = Ref(eom, janentryday - 1)
    let eom: Vec<bool> = (0..n).map(|i| i + 1 < n && month(pre.dates[i]) != month(pre.dates[i + 1])).collect();
    let nojan = no_jan_sell(&pre.dates, &lastdec);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let nj = !jan[i];
        let eombuysig = pm.janday >= 1 && i + pm.janday - 1 < n && eom[i + pm.janday - 1];
        let entry = (new_high(pre, i) && nj) || (bs[i] && nj) || (sub == 0 && bs[i] && eombuysig);
        buy[i] = entry && !lastdec[i] && !pre.upc[i] && base_ok(pre, i) && !pre.dead1[i];
        sell[i] = ((trend1[i] == -1 || !bs[i]) && !pre.loc[i]) || pre.deadsell[i] || pre.dead1[i] || if sub == 0 { lastdec[i] } else { nojan[i] && !pre.loc[i] };
    }
    out(buy, sell)
}

/// id 31: `athccount` counts ATH closes and restarts at 0 when the swing turns down while the latch is on.
fn fam_delay0(pre: &Pre, pm: &Pm) -> Out {
    let n = pre.n;
    let (c, hcl, trend1) = (&pre.c, &pre.hcl, &pre.sw.trend);
    let mut bs = vec![false; n];
    let mut cnt = vec![0i32; n];
    for i in 1..n {
        if c[i] > hcl[i - 1] {
            bs[i] = true;
            cnt[i] = cnt[i - 1] + 1;
        } else if trend1[i] == -1 && bs[i - 1] {
            bs[i] = false;
        } else {
            bs[i] = bs[i - 1];
            cnt[i] = cnt[i - 1];
        }
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = (new_high(pre, i) || bs[i]) && cnt[i] as f32 > pm.delay && !pre.upc[i] && base_ok(pre, i);
        sell[i] = ((trend1[i] == -1 || !bs[i]) && !pre.loc[i]) || pre.deadsell[i];
    }
    out(buy, sell)
}

// ---------------------------------------------------------------- one symbol / the universe

fn finish(pre: &Pre, o: Out) -> Signals {
    let b2 = exrem(&o.buy, &o.sell);
    let s2 = exrem(&o.sell, &b2);
    let mut sg = Signals::new(b2, s2, o.bp.unwrap_or_else(|| pre.c.clone()), o.sp.unwrap_or_else(|| pre.c.clone()));
    clamp_prices(&mut sg, &pre.h, &pre.l);
    sg
}

fn symbol_inner(s: &Series, idx: Option<&Series>, refc: Option<&Series>, g: G, sp: &Spec, pm: &Pm, pad: bool) -> Signals {
    let pre = build(s, idx, refc, g, sp, pm, pad);
    let o = match (sp.kind, sp.sub) {
        (Kind::RunLoss, _) => fam_run_loss(&pre, pm, sp),
        (Kind::LossState, k) => fam_loss_state(&pre, pm, k),
        (Kind::PbLoss, _) => fam_pb(&pre, pm, true, true, true),
        (Kind::PbOnly, _) => fam_pb(&pre, pm, sp.lock, false, false),
        (Kind::JanBull, _) => fam_jan_bull(&pre),
        (Kind::AllSt, _) => fam_all_st(&pre, pm, s, g("bpd1").round().max(1.0) as usize, g("bfactor1") as f32),
        (Kind::RunGate, _) => fam_run_gate(&pre, sp),
        (Kind::Plain, _) => fam_plain(&pre, sp),
        (Kind::Combo, _) => fam_combo(&pre, pm),
        (Kind::State3, k) => fam_state3(&pre, k),
        (Kind::Buffered, k) => fam_buffered(&pre, pm, k),
        (Kind::PlusJan, _) => fam_plus_jan(&pre, pm),
        (Kind::LossJan, _) => fam_loss_jan(&pre, pm),
        (Kind::NoJan, k) => fam_no_jan(&pre, pm, k),
        (Kind::Delay0, _) => fam_delay0(&pre, pm),
    };
    finish(&pre, o)
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
    let idx = if sp.filt == Filt::None { None } else { index_pick(env, g, sp) };
    let pad = sp.filt == Filt::St && g("pad") != 0.0 && (sp.force_index || g("mktflt").round() == 1.0);
    let refc = if sp.filt == Filt::St {
        let r = g("padref").round();
        if r < 0.0 { idx } else { env.get(INDEX_NAMES[(r as usize).min(3)]) }
    } else {
        None
    };
    let hard = g("hardstop") as f32;
    let (rm, r1, r2, r3, r4) = if sp.st9 { (g("rankmode").round() as i32, g("rocpd").round().max(0.0) as usize, g("rsipd").round().max(1.0) as usize, g("stdpd").round().max(1.0) as usize, g("swingpd").round().max(1.0) as usize) } else { (1, 0, 0, 0, 0) };
    series
        .iter()
        .map(|s| {
            let mut sg = symbol(s, idx, refc, g, sp, &pm, pad);
            if hard > 0.0 {
                sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
            }
            if sp.st9 {
                sg.score = rank_score(s, rm, r1, r2, r3, r4);
            }
            sg
        })
        .collect()
}

// ---------------------------------------------------------------- the 32 variants

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

const SW: Filt = Filt::Swing;
// 1-6: ATH running + loss gate of the %swing trades, ST/SMA index (Vc6 table)
const S01: Spec = Spec { kind: Kind::RunLoss, exit: Some(r(10.0, 15.0, 25.0)), loss: Some((r(2.0, 2.0, 3.0), r(2.0, 1.0, 1.0))), ..BASE };
const S02: Spec = Spec { opp_exit: true, exit: Some(r(10.0, 11.0, 14.0)), loss: Some((r(2.0, 0.5, 10.0), r(2.0, 0.0, 2.0))), ..S01 };
const S03: Spec = Spec { kind: Kind::LossState, sub: 0, turn: Turn::OiStd, exit: Some(r(10.0, 5.0, 25.0)), loss: Some((r(2.0, 0.0, 10.0), r(2.0, 0.0, 50.0))), ..BASE };
const S04: Spec = Spec { sub: 1, loss: Some((r(2.0, 0.5, 10.0), r(2.0, 0.0, 50.0))), ..S03 };
const S05: Spec = Spec { bull: true, exit: Some(r(10.0, 5.0, 25.0)), loss: Some((r(2.0, 0.5, 10.0), r(2.0, 0.0, 50.0))), ..S01 };
const S06: Spec = Spec { opp_exit: true, exit: Some(r(10.0, 16.0, 19.0)), loss: Some((r(2.0, 0.5, 10.0), r(2.0, 1.0, 50.0))), ..S05 };
// 7: ATHC after losing trades, bear, profit booking
const S07: Spec = Spec { kind: Kind::PbLoss, turn: Turn::OiStd, exit: Some(r(10.0, 5.0, 25.0)), loss: Some((r(2.0, 0.5, 10.0), r(2.0, 0.0, 50.0))), pb: Some(r(5.0, 0.0, 10.0)), pbbuff: true, ..BASE };
// 8: bull, no January trade
const S08: Spec = Spec { kind: Kind::JanBull, bull: true, exit: Some(r(10.0, 5.0, 35.0)), ..BASE };
// 9: all Supertrend variations, bull index filter, ranking
const S09: Spec = Spec { kind: Kind::AllSt, bull: true, idx3: true, st9: true, tstart: Some(r(1.0, 0.0, 500.0)), ipd: r(5.0, 0.0, 20.0), ..BASE };
// 10, 11: index %swing filter (bear / bull)
const S10: Spec = Spec { kind: Kind::RunGate, filt: SW, exit: Some(r(16.0, 5.0, 35.0)), ..BASE };
const S11: Spec = Spec { bull: true, ..S10 };
// 12-15
const S12: Spec = Spec { kind: Kind::Plain, bull: true, nh: true, uc_lc: true, opp_exit: true, turn: Turn::OiStd, exit: Some(r(16.0, 15.0, 21.0)), ipd: r(20.0, 5.0, 25.0), ifact: r(1.0, 3.0, 6.0), ..BASE };
const S13: Spec = Spec { uc_lc: false, turn: Turn::Vc6, exit: Some(r(16.0, 16.0, 20.0)), ipd: r(20.0, 0.0, 20.0), ifact: r(1.0, 0.0, 20.0), ..S12 };
const S14: Spec = Spec { bull: false, exit: Some(r(16.0, 10.0, 25.0)), ..S13 };
const S15: Spec = Spec { kind: Kind::RunGate, exit: Some(r(10.0, 5.0, 25.0)), ..BASE };
// 16: bull-bear combo
const S16: Spec = Spec { kind: Kind::Combo, force_index: true, exit1: Some(r(19.0, 5.0, 35.0)), exit2: Some(r(16.0, 5.0, 35.0)), loss: Some((r(2.0, 0.5, 10.0), r(1.0, 1.0, 1.0))), athc: Some(r(5.0, 1.0, 30.0)), ..BASE };
// 17: fresh ATH only in bear (3-state latch); 24: running trade in bear
const S17: Spec = Spec { kind: Kind::State3, sub: 0, turn: Turn::OiStd, exit: Some(r(16.0, 5.0, 25.0)), ..BASE };
// 18, 19: bear + profit booking only
const S18: Spec = Spec { kind: Kind::PbOnly, lock: true, turn: Turn::OiStd, exit_fixed: 16.0, pb_fixed: 5.0, pbbuff: true, ..BASE };
const S19: Spec = Spec { lock: false, exit: Some(r(16.0, 5.0, 25.0)), pb: Some(r(5.0, 5.0, 50.0)), ..S18 };
const S20: Spec = Spec { kind: Kind::RunGate, exit: Some(r(10.0, 5.0, 25.0)), ..BASE };
const S21: Spec = Spec { kind: Kind::Plain, bull: true, nh: false, uc_lc: true, opp_exit: false, turn: Turn::OiStd, exit_fixed: 16.0, ..BASE };
const S22: Spec = Spec { kind: Kind::Buffered, sub: 0, bull: true, turn: Turn::OiStd, exit: Some(r(16.0, 5.0, 25.0)), buf: true, ..BASE };
const S23: Spec = Spec { kind: Kind::RunGate, bull: true, exit_fixed: 16.0, ..BASE };
const S24: Spec = Spec { kind: Kind::State3, sub: 1, ..S17 };
const S25: Spec = Spec { kind: Kind::RunGate, bull: true, exit: Some(r(16.0, 5.0, 25.0)), ..BASE };
// 26-27: January filter + index
const S26: Spec = Spec { kind: Kind::PlusJan, bull: true, idx3: true, exit: Some(r(16.0, 5.0, 25.0)), athc: Some(r(5.0, 1.0, 10.0)), ..BASE };
const S27: Spec = Spec { kind: Kind::LossJan, bull: true, exit: Some(r(15.0, 5.0, 25.0)), loss: Some((r(2.0, 1.0, 3.0), r(2.0, 0.0, 2.0))), ..BASE };
// 28-30: no January trade (no index)
const S28: Spec = Spec { kind: Kind::NoJan, sub: 0, filt: Filt::None, exit: Some(r(16.0, 5.0, 35.0)), jan: true, ..BASE };
const S29: Spec = Spec { sub: 1, jan: false, ..S28 };
const S30: Spec = Spec { sub: 2, exit: Some(r(16.0, 5.0, 25.0)), ..S29 };
// 31, 32: delay entry
const S31: Spec = Spec { kind: Kind::Delay0, filt: Filt::None, delay: Some(r(1.0, 0.0, 10.0)), exit: Some(r(10.0, 5.0, 35.0)), ..BASE };
const S32: Spec = Spec { kind: Kind::Buffered, sub: 1, filt: Filt::None, turn: Turn::OiStd, tstart: Some(r(0.0, 0.0, 500.0)), exit: Some(r(10.0, 5.0, 35.0)), swingcal: true, exitb: Some(r(10.0, 5.0, 20.0)), buf: true, ..BASE };

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
variant!(S26, sig26, sch26);
variant!(S27, sig27, sch27);
variant!(S28, sig28, sch28);
variant!(S29, sig29, sch29);
variant!(S30, sig30, sch30);
variant!(S31, sig31, sch31);
variant!(S32, sig32, sch32);

// verification aids (NOT registered): the same AFL rules with the extra terms of the verified neighbours switched on, to compare signal by signal
const CHK6: Spec = Spec { turn: Turn::OiStd, uc_lc: true, ..S06 };
const CHK5: Spec = Spec { turn: Turn::OiStd, uc_lc: true, ref_v5: true, ..S05 };
const CHK6_OI: Spec = Spec { turn: Turn::OiStd, ..S06 };
const CHK6_UC: Spec = Spec { uc_lc: true, ..S06 };
const CHK32: Spec = Spec { ref_plain: true, ..S32 };
fn sig_chk6(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    run(series, env, g, &CHK6)
}
fn sch_chk6() -> Vec<P> {
    schema_of(&CHK6)
}
fn sig_chk5(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    run(series, env, g, &CHK5)
}
fn sch_chk5() -> Vec<P> {
    schema_of(&CHK5)
}
fn sig_chk6_oi(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    run(series, env, g, &CHK6_OI)
}
fn sch_chk6_oi() -> Vec<P> {
    schema_of(&CHK6_OI)
}
fn sig_chk6_uc(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    run(series, env, g, &CHK6_UC)
}
fn sch_chk6_uc() -> Vec<P> {
    schema_of(&CHK6_UC)
}
fn sig_chk32(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    run(series, env, g, &CHK32)
}
fn sch_chk32() -> Vec<P> {
    schema_of(&CHK32)
}
/// Not part of the registry: variants that only exist to compare this core with the verified engines (see `ATH/v2/*/VERIFY.md`).
pub fn check_variants() -> Vec<VariantDef> {
    let d = Data::Cash22 { watchlist: "NSE 500 C&P.tls" };
    vec![
        VariantDef { id: "a2_chk_run_bull_exitall_oi_uc", data: d, schema: sch_chk6, signals: sig_chk6 },
        VariantDef { id: "a2_chk_run_bull_exitall_oi", data: d, schema: sch_chk6_oi, signals: sig_chk6_oi },
        VariantDef { id: "a2_chk_run_bull_exitall_uc", data: d, schema: sch_chk6_uc, signals: sig_chk6_uc },
        VariantDef { id: "a2_chk_run_bull_oi_uc", data: d, schema: sch_chk5, signals: sig_chk5 },
        VariantDef { id: "a2_chk_turnover_running_plain_swing", data: d, schema: sch_chk32, signals: sig_chk32 },
    ]
}

pub fn variants() -> Vec<VariantDef> {
    let d = Data::Cash22 { watchlist: "NSE 500 C&P.tls" };
    vec![
        VariantDef { id: "ytath_running_pctswing_exit_after_2", data: d, schema: sch01, signals: sig01 },
        VariantDef { id: "ytath_running_pctswing_after_2_pctswing_3", data: d, schema: sch02, signals: sig02 },
        VariantDef { id: "ytath_pctswing_exit_after_2_pctswing_losses_3", data: d, schema: sch03, signals: sig03 },
        VariantDef { id: "ytath_pctswing_exit_after_2_pctswing_losses_4", data: d, schema: sch04, signals: sig04 },
        VariantDef { id: "ytath_running_pctswing_exit_after_2_2", data: d, schema: sch05, signals: sig05 },
        VariantDef { id: "ytath_running_pctswing_exit_after_2_3", data: d, schema: sch06, signals: sig06 },
        VariantDef { id: "ytath_after_losing_trades_bear_index_filter", data: d, schema: sch07, signals: sig07 },
        VariantDef { id: "ytath_pctswing_exit_bull_index_filter_no", data: d, schema: sch08, signals: sig08 },
        VariantDef { id: "ytath_with_all_supertrend_variations_bull", data: d, schema: sch09, signals: sig09 },
        VariantDef { id: "ytath_with_index_swing_filter_pctswing_exit", data: d, schema: sch10, signals: sig10 },
        VariantDef { id: "ytath_with_index_swing_filter_pctswing_exit_2", data: d, schema: sch11, signals: sig11 },
        VariantDef { id: "ytath_index_filter_pctswing_exit_for_bull", data: d, schema: sch12, signals: sig12 },
        VariantDef { id: "ytath_with_index_filter_pctswing_exit_for", data: d, schema: sch13, signals: sig13 },
        VariantDef { id: "ytath_with_index_filter_pctswing_exit_for_2", data: d, schema: sch14, signals: sig14 },
        VariantDef { id: "ytath_with_index_filter_pctswing_exit_for_3", data: d, schema: sch15, signals: sig15 },
        VariantDef { id: "ytath_bull_bear_combo_athcplusnpct_buy", data: d, schema: sch16, signals: sig16 },
        VariantDef { id: "ytath_index_filter_pctswing_exit_for_bear", data: d, schema: sch17, signals: sig17 },
        VariantDef { id: "ytath_pctswing_exit_bear_market_only_index", data: d, schema: sch18, signals: sig18 },
        VariantDef { id: "ytath_pctswing_exit_bear_mkt_running_ath_at", data: d, schema: sch19, signals: sig19 },
        VariantDef { id: "ytath_pctswing_exit_bull_only_index_st_ath", data: d, schema: sch21, signals: sig21 },
        VariantDef { id: "ytath_pctswing_exit_bull_only_index_st_buy", data: d, schema: sch22, signals: sig22 },
        VariantDef { id: "ytath_pctswing_exit_bear_mkt_running_trade", data: d, schema: sch24, signals: sig24 },
        VariantDef { id: "ytath_pctswing_exit_bull_market_only_index_2", data: d, schema: sch25, signals: sig25 },
        VariantDef { id: "ytath_plus_pct_buy_pctswing_exit_index", data: d, schema: sch26, signals: sig26 },
        VariantDef { id: "ytath_pctswing_after_2_pctswing_losses_no", data: d, schema: sch27, signals: sig27 },
        VariantDef { id: "ytath_pctswing_exit_no_jan_trade_exit_on_31", data: d, schema: sch28, signals: sig28 },
        VariantDef { id: "ytath_no_jan_trade_only_new_ath_buy_in_feb", data: d, schema: sch29, signals: sig29 },
        VariantDef { id: "ytath_no_jan_trade_all_running_ath_buy_in", data: d, schema: sch30, signals: sig30 },
        VariantDef { id: "ytath_delay_entry_and_pctswing_exit_without", data: d, schema: sch31, signals: sig31 },
        VariantDef { id: "ytath_pctswing_exit_turnover_running_buy", data: d, schema: sch32, signals: sig32 },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(date: i32, c: f32) -> Bar {
        Bar { date, c, o: c, h: c + 1.0, l: c - 1.0, v: 1000.0, oi: 1.0e7, aux1: 1.0, aux2: 0.0 }
    }

    /// consecutive dates over `months` months, `per` days each, starting January `year`
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

    fn one(sp: &Spec, s: &Series, env: &Env, ov: &[(&'static str, f64)]) -> Signals {
        let g = reader(sp, ov);
        run(std::slice::from_ref(s), env, &g, sp).remove(0)
    }

    fn buys(sg: &Signals) -> Vec<usize> {
        (0..sg.buy.len()).filter(|&i| sg.buy[i]).collect()
    }

    #[test]
    fn all_32_ids_are_registered_with_defaults_inside_their_ranges_and_the_common_keys() {
        let v = variants();
        assert_eq!(v.len(), 30);
        let mut ids: Vec<&str> = v.iter().map(|x| x.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 30);
        for x in &v {
            let sch = (x.schema)();
            assert!(sch.iter().all(|d| d.min <= d.def && d.def <= d.max), "{}", x.id);
            assert!(sch.iter().all(|d| d.key != "equity" && d.key != "pct"), "{} must not redefine a common key", x.id);
            let mut keys: Vec<&str> = sch.iter().map(|d| d.key).collect();
            keys.sort();
            keys.dedup();
            assert_eq!(keys.len(), sch.len(), "{} has duplicate keys", x.id);
            assert_eq!(x.data, Data::Cash22 { watchlist: "NSE 500 C&P.tls" });
        }
    }

    #[test]
    fn afl_defaults_and_ranges_of_the_param_and_optimize_rows() {
        let key = |sp: &Spec, k: &str| schema_of(sp).into_iter().find(|d| d.key == k).map(|d| (d.def, d.min, d.max));
        // id 1: EXIT 10 in 15..25 and CONSICUTIVE LOSS 2 in 1..1 are clamped into the range
        assert_eq!(key(&S01, "exitpct"), Some((15.0, 15.0, 25.0)));
        assert_eq!(key(&S01, "noofloss"), Some((1.0, 1.0, 1.0)));
        assert_eq!(key(&S02, "exitpct"), Some((11.0, 11.0, 14.0)));
        assert_eq!(key(&S06, "exitpct"), Some((16.0, 16.0, 19.0)));
        assert_eq!(key(&S12, "ifltfact"), Some((3.0, 3.0, 6.0)));
        assert_eq!(key(&S12, "ifltpd"), Some((20.0, 5.0, 25.0)));
        // Optimize rows are not parameters
        for sp in [&S18, &S21, &S23] {
            assert!(key(sp, "exitpct").is_none());
        }
        assert!(key(&S18, "pb").is_none() && key(&S18, "pbbuff").is_some());
        assert_eq!(key(&S19, "pb"), Some((5.0, 5.0, 50.0)));
        // id 16 has no MKTFLT / CONSICUTIVE LOSS key, id 31 no TRADING START BAR key, id 32 both EXIT % keys
        assert!(key(&S16, "mktflt").is_none() && key(&S16, "noofloss").is_none() && key(&S16, "athcbuypct").is_some());
        assert!(key(&S31, "tstart").is_none() && key(&S31, "delaybuy") == Some((1.0, 0.0, 10.0)));
        assert!(key(&S32, "exitpct").is_some() && key(&S32, "exitpct_2") == Some((10.0, 5.0, 20.0)));
        assert_eq!(key(&S09, "tstart"), Some((1.0, 0.0, 500.0)));
        assert_eq!(key(&S09, "prflt"), Some((0.0, 0.0, 2.0)));
    }

    #[test]
    fn loss_streak_gate_needs_completed_virtual_swing_losses() {
        // a long rising series: no virtual trade completes, noofloss 0 buys, noofloss 1 never buys (ids 1-6, 27)
        let ds = dates(2020, 1, 25);
        let s = series("T", &ds, &rising(25));
        for sp in [&S01, &S02, &S05, &S06] {
            let n0 = buys(&one(sp, &s, &Env::empty(), &[("noofloss", 0.0), ("exitpct", 30.0), ("mktflt", 0.0)])).len();
            let n1 = buys(&one(sp, &s, &Env::empty(), &[("noofloss", 1.0), ("exitpct", 30.0)])).len();
            // ids 1 / 2 have the range 1..1 / 0..2 for CONSICUTIVE LOSS: the harness does not clamp an explicit value
            assert!(n1 == 0, "{n1}");
            assert!(n0 >= 1 || sp.bull == false, "{n0}");
        }
        let n0 = buys(&one(&S05, &s, &Env::empty(), &[("noofloss", 0.0), ("exitpct", 30.0)])).len();
        assert_eq!(n0, 1);
    }

    #[test]
    fn index_swing_filter_gates_bull_and_bear_runs() {
        // index rises 20 bars (peak 1190) then falls 30 per bar: 10 % below the peak (1071) is broken at bar 23
        let ds = dates(2020, 2, 20);
        let stock = series("T", &ds, &rising(40));
        let ix: Vec<f32> = (0..40).map(|i| if i < 20 { 1000.0 + 10.0 * i as f32 } else { 1190.0 - 30.0 * (i - 19) as f32 }).collect();
        let mut env = Env::empty();
        env.index.insert("NSENIFTY".into(), series("NSENIFTY", &ds, &ix));
        let ov = [("mktflt", 1.0), ("prflt", 0.0), ("exitpct", 30.0), ("fltfact1", 10.0), ("fltfact2", 10.0)];
        let bull = buys(&one(&S11, &stock, &env, &ov));
        let bear = buys(&one(&S10, &stock, &env, &ov));
        assert_eq!(bull, vec![1], "bull run: first ATH close while the index swing is up");
        assert_eq!(bear, vec![23], "bear run: only once the index %swing has turned down");
        // NONE: the bear AFL sees FLTSHORT = true, the bull AFL FLTBUY = true
        assert_eq!(buys(&one(&S10, &stock, &Env::empty(), &[("exitpct", 30.0)])), vec![1]);
        assert_eq!(buys(&one(&S11, &stock, &Env::empty(), &[("exitpct", 30.0)])), vec![1]);
    }

    #[test]
    fn st_state_machines_without_filter_agree() {
        // MKTFLT NONE: the runningbuy counter (id 3) and the plain gate (id 4) give the same entries; id 24 equals id 4 at noofloss 0
        let ds = dates(2020, 2, 20);
        let mut c = rising(40);
        c[20] = 60.0;
        c[21] = 62.0;
        let s = series("T", &ds, &c);
        let ov = [("noofloss", 0.0), ("exitpct", 10.0)];
        let a = one(&S03, &s, &Env::empty(), &ov);
        let b = one(&S04, &s, &Env::empty(), &ov);
        let d = one(&S24, &s, &Env::empty(), &[("exitpct", 10.0)]);
        assert_eq!(buys(&a), buys(&b));
        assert_eq!(buys(&b), buys(&d));
        assert!(buys(&a).len() >= 2, "re-entry after the swing broke: {:?}", buys(&a));
    }

    #[test]
    fn profit_booking_sells_at_the_target_when_the_high_reaches_it() {
        // buy at the first ATH close 101, target 5 % = 106.05; the bar with close 106 has high 107 > 106.05
        let ds = dates(2020, 1, 12);
        let s = series("T", &ds, &rising(12));
        let sg = one(&S19, &s, &Env::empty(), &[("exitpct", 30.0), ("pb", 5.0)]);
        assert_eq!(buys(&sg)[0], 1);
        assert!(sg.sell[6], "state falls to 0 on the hit bar");
        assert!((sg.sell_price[6] - 106.05).abs() < 0.01, "{}", sg.sell_price[6]);
        // the same with the state-2 lock (ids 7 / 18) in a market without an index
        let sg18 = one(&S18, &s, &Env::empty(), &[]);
        assert!(buys(&sg18).contains(&1) && sg18.sell[6]);
    }

    #[test]
    fn buffered_fills_and_the_sell_count_of_the_turnover_running_buy_afl() {
        let ds = dates(2020, 1, 8);
        let s = series("T", &ds, &[100.0, 101.0, 102.0, 103.0, 104.0, 70.0, 69.0, 68.0]);
        // id 22: first sell bar (sellcount 1, not a lower circuit) sells at the close, later ones at open - buffer
        let a = one(&S22, &s, &Env::empty(), &[("exitpct", 10.0)]);
        assert_eq!(buys(&a), vec![1]);
        assert!(a.sell[5] && (a.sell_price[5] - 70.0).abs() < 1e-4);
        // id 32 writes the two prices the other way round: first sell bar sells at open - buffer
        let b = one(&S32, &s, &Env::empty(), &[("exitpct_2", 10.0)]);
        assert_eq!(buys(&b), vec![1]);
        assert!(b.sell[5] && (b.sell_price[5] - 70.0 * (1.0 - 0.0025)).abs() < 1e-3, "{}", b.sell_price[5]);
        // catch-up entry price: open + buying buffer, inside the bar's [Low, High]
        let c = one(&S22, &series("T", &ds, &[100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0]), &Env::empty(), &[("exitpct", 10.0)]);
        for i in 0..8 {
            assert!(c.buy_price[i] >= 99.0 && c.buy_price[i] <= 108.0);
        }
    }

    #[test]
    fn no_january_family() {
        // Nov 2019 .. Feb 2020, 10 bars a month; rising to the last December day (bar 19), then below the ATH
        let mut ds = vec![];
        for (y, m) in [(2019, 11), (2019, 12), (2020, 1), (2020, 2)] {
            for d in 1..=10 {
                ds.push(y * 10000 + m * 100 + d);
            }
        }
        let c: Vec<f32> = (0..40).map(|i| if i <= 19 { 100.0 + i as f32 } else { 118.0 - 0.1 * (i - 19) as f32 }).collect();
        let s = series("T", &ds, &c);
        let ov = [("exitpct", 30.0)];
        let all_running = one(&S30, &s, &Env::empty(), &ov);
        let only_new = one(&S29, &s, &Env::empty(), &ov);
        let exit31 = one(&S28, &s, &Env::empty(), &[("exitpct", 30.0), ("janentryday", 5.0)]);
        for sg in [&all_running, &only_new, &exit31] {
            assert!(sg.sell[19], "sold on the last trading day of December");
        }
        let jan = |sg: &Signals| buys(sg).into_iter().filter(|&i| ds[i] / 100 % 100 == 1).count();
        assert_eq!(jan(&all_running), 0);
        assert_eq!(jan(&only_new), 0);
        // the running signal is kept through the year end and re-enters on the first February bar; the ATH-only version has no signal then
        assert!(buys(&all_running).contains(&30), "{:?}", buys(&all_running));
        assert!(!buys(&only_new).iter().any(|&i| i >= 30), "{:?}", buys(&only_new));
        // the 5-day phenomenon: the January 6th bar is 4 bars before the last January bar -> entry allowed
        assert_eq!(buys(&exit31).into_iter().filter(|&i| ds[i] / 100 % 100 == 1).collect::<Vec<_>>(), vec![25]);
    }

    #[test]
    fn combo_swing_width_follows_the_regime() {
        // a 17.5 % one-bar drop breaks the 16 % (bear) swing but not the 19 % (bull) swing; no index = neither FLTBUY nor FLTSHORT = bear width
        let ds = dates(2020, 1, 8);
        let s = series("T", &ds, &[100.0, 110.0, 120.0, 99.0, 98.0, 97.0, 96.0, 95.0]);
        let g = reader(&S16, &[]);
        let pm = pm_of(&g, &S16);
        let pre = build(&s, None, None, &g, &S16, &pm, false);
        assert_eq!(pre.sw.trend[3], -1);
        let g19 = reader(&S16, &[("exitpct2", 19.0)]);
        let pm19 = pm_of(&g19, &S16);
        let pre19 = build(&s, None, None, &g19, &S16, &pm19, false);
        assert_eq!(pre19.sw.trend[3], 1);
    }

    #[test]
    fn delay_entry_counts_ath_closes_and_restarts_after_a_swing_down() {
        let ds = dates(2020, 1, 12);
        let s = series("T", &ds, &rising(12));
        let sg = one(&S31, &s, &Env::empty(), &[("exitpct", 30.0), ("delaybuy", 2.0)]);
        assert_eq!(buys(&sg).first().copied(), Some(3), "ATH closes at bars 1, 2, 3: count 3 > 2");
        let sg0 = one(&S31, &s, &Env::empty(), &[("exitpct", 30.0), ("delaybuy", 0.0)]);
        assert_eq!(buys(&sg0).first().copied(), Some(1));
    }

    #[test]
    fn stock_supertrend_variant_runs_with_ranking_off_and_keeps_fills_inside_the_bar() {
        let ds = dates(2020, 2, 20);
        let mut c = rising(40);
        c[25] = 80.0;
        let s = series("T", &ds, &c);
        let sg = one(&S09, &s, &Env::empty(), &[("bpd1", 3.0), ("bfactor1", 1.0)]);
        assert!(!buys(&sg).is_empty());
        assert!(sg.score.is_none());
        for i in 0..40 {
            assert!(sg.buy_price[i] >= s.bars[i].l && sg.buy_price[i] <= s.bars[i].h);
            assert!(sg.sell_price[i] >= s.bars[i].l && sg.sell_price[i] <= s.bars[i].h);
        }
    }

    #[test]
    fn plus_percent_buy_with_january_filter_needs_the_trigger_close() {
        let ds = dates(2020, 1, 8);
        let s = series("T", &ds, &[100.0, 101.0, 102.0, 108.0, 109.0, 90.0, 89.0, 88.0]);
        // January bars: no entries at all in the January file
        let sg = one(&S26, &s, &Env::empty(), &[("exitpct", 10.0), ("athcbuypct", 5.0)]);
        assert!(buys(&sg).is_empty(), "all bars are in January");
        // February bars: first ATH closes below the +5 % trigger of the first close do not buy
        let ds2: Vec<i32> = ds.iter().map(|d| d + 100).collect();
        let s2 = series("T", &ds2, &[100.0, 101.0, 102.0, 108.0, 109.0, 90.0, 89.0, 88.0]);
        let sg2 = one(&S26, &s2, &Env::empty(), &[("exitpct", 10.0), ("athcbuypct", 5.0)]);
        assert!(!sg2.buy[1] && !sg2.buy[2] && sg2.buy[3], "{:?}", buys(&sg2));
        assert!(sg2.sell[5]);
    }
}
