//! ATH engines: ATHC after consecutive losing %swing trades (average-turnover rank bucket, index filter) and the ATR scale-in variant.
//! One `VariantDef` per sub-variant (see `ath_core::VariantDef`); registered in `ath_registry.rs`. Standard library only: depends on
//! `ath_core` so a verification driver can include it with `#[path]`.
//!
//! * `ytath_pctswing_exit_after_pctswing_losses`: AFL `ATHC with %swing sl with probability with new turnover_NEW_Average TO.afl`
//! * `ytath_pctswing_after_pctswing_losses`:       AFL `ATHC with %swing sl with probability with new turnover_NEW_iNDEX.afl`
//! * `ytath_and_pctswing_after_consecutive_low_2`: AFL `ATHC & %Swing with 1-Trade loss Fixed ATR Scalein(5).afl`
//! Verification notes: `ATH/verify/<variant id>/RESULT.md`.
#![allow(dead_code)]
use crate::ath_core::*;

/// AmiBroker's Null (EMPTY) value inside arrays.
const NULLV: f32 = -1.0e10;

// ---------------------------------------------------------------- shared pieces of the %swing family

fn volsig(year: i32) -> f32 {
    match year {
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

/// The %swing line of the AFL (identical in the three files): trend1 and TrendUp1 (Null where not set).
struct Swing {
    trend: Vec<i8>,
    trendup: Vec<f32>,
}

fn swing(c: &[f32], exit_pct: f32) -> Swing {
    let n = c.len();
    let exit: Vec<f32> = c.iter().map(|&x| (x * exit_pct) / 100.0).collect();
    let mut up: Vec<f32> = (0..n).map(|i| c[i] + exit[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| c[i] - exit[i]).collect();
    let mut trend = vec![0i8; n];
    let mut trendup = vec![NULLV; n];
    let mut trenddown = vec![NULLV; n];
    if n > 0 {
        trend[0] = 1;
    }
    let mut change = false;
    for i in 1..n {
        trend[i] = 1;
        if c[i] > up[i - 1] {
            trend[i] = 1;
            if trend[i - 1] == -1 {
                change = true;
            }
        } else if c[i] < dn[i - 1] {
            trend[i] = -1;
            if trend[i - 1] == 1 {
                change = true;
            }
        } else if trend[i - 1] == 1 {
            trend[i] = 1;
            change = false;
        } else if trend[i - 1] == -1 {
            trend[i] = -1;
            change = false;
        }
        let flag1 = trend[i] < 0 && trend[i - 1] > 0;
        let flagh1 = trend[i] > 0 && trend[i - 1] < 0;
        if trend[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if trend[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if flag1 {
            up[i] = c[i] + exit[i];
        }
        if flagh1 {
            dn[i] = c[i] - exit[i];
        }
        if trend[i] == 1 {
            trendup[i] = dn[i];
            if change {
                trendup[i - 1] = trenddown[i - 1];
                change = false;
            }
        } else if trend[i] == -1 {
            trenddown[i] = up[i];
            if change {
                trenddown[i - 1] = trendup[i - 1];
                change = false;
            }
        }
    }
    Swing { trend, trendup }
}

/// `Hcl = HHV(C, BarCount)`: highest close up to the bar.
fn running_high(c: &[f32]) -> Vec<f32> {
    let mut h = vec![0f32; c.len()];
    for i in 0..c.len() {
        h[i] = if i == 0 || c[i] > h[i - 1] { c[i] } else { h[i - 1] };
    }
    h
}

/// ATHCbuysig: 1 on a close above all earlier closes, 0 when the swing trend is -1 while it was 1, else carried.
fn athc_state(c: &[f32], hcl: &[f32], trend: &[i8]) -> Vec<bool> {
    let n = c.len();
    let mut a = vec![false; n];
    for i in 1..n {
        a[i] = if c[i] > hcl[i - 1] {
            true
        } else if trend[i] == -1 && a[i - 1] {
            false
        } else {
            a[i - 1]
        };
    }
    a
}

/// 30 identical O/H/L/C bars (the AFL's `deadstock` array: the bar equals each of the previous 30 bars).
fn dead30(s: &Series) -> Vec<bool> {
    let n = s.bars.len();
    let mut d = vec![false; n];
    for i in 30..n {
        let b = &s.bars[i];
        d[i] = (1..=30).all(|k| {
            let q = &s.bars[i - k];
            b.o == q.o && b.h == q.h && b.l == q.l && b.c == q.c
        });
    }
    d
}

/// Loss streak of the virtual %swing trades (in on ATHC buysig, out on trend -1): `concount`.
fn concount(c: &[f32], buysig: &[bool], sw: &Swing, losspct: f32, comm: f32) -> Vec<f32> {
    let n = c.len();
    let (mut bcount, mut bprice, mut sprice) = (vec![0i32; n], vec![NULLV; n], vec![NULLV; n]);
    for i in 1..n {
        if buysig[i] && bcount[i - 1] < 1 {
            bprice[i] = c[i];
            sprice[i] = sprice[i - 1];
            bcount[i] = bcount[i - 1] + 1;
        } else if sw.trend[i] == -1 && bcount[i - 1] == 1 {
            sprice[i] = c[i];
            bprice[i] = bprice[i - 1];
            bcount[i] = 0;
        } else {
            bprice[i] = bprice[i - 1];
            sprice[i] = sprice[i - 1];
            bcount[i] = bcount[i - 1];
        }
    }
    // Cross(Ref(TrendUp1,-1), C): TrendUp1 of the previous bar rises above C now, while it was not above C one bar earlier
    let a = |i: usize| -> f32 { if i >= 1 { sw.trendup[i - 1] } else { NULLV } };
    let mut cc = vec![0f32; n];
    for i in 1..n {
        let cross = i >= 2 && a(i) > c[i] && a(i - 1) <= c[i - 1];
        let ev = cross && bcount[i - 1] == 1;
        let profit = if ev { (sprice[i] - (sprice[i] * comm) / 100.0) - (bprice[i] + (bprice[i] * comm) / 100.0) } else { 0.0 };
        let pct = if ev { (((sprice[i] - (sprice[i] * comm) / 100.0) - (bprice[i] + (bprice[i] * comm) / 100.0)) / bprice[i]) * 100.0 } else { 0.0 };
        cc[i] = if profit != 0.0 {
            if pct <= losspct { cc[i - 1] + 1.0 } else { 0.0 }
        } else {
            cc[i - 1]
        };
    }
    cc
}

/// Wilder's ATR as used by the scale-in AFL (`seed` = Wilder average of the first n true ranges, Null before). Own copy: see RESULT.md.
fn atr_ab(h: &[f32], l: &[f32], c: &[f32], n: usize, mode: u8) -> Vec<f32> {
    let len = h.len();
    let tr: Vec<f32> = (0..len).map(|i| if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect();
    let mut out = vec![NULLV; len];
    if n == 0 {
        return out;
    }
    match mode {
        // simple average of the first n true ranges, then Wilder
        0 => {
            if len >= n {
                out[n - 1] = tr[..n].iter().sum::<f32>() / n as f32;
                for i in n..len {
                    out[i] = (out[i - 1] * (n as f32 - 1.0) + tr[i]) / n as f32;
                }
            }
        }
        // seeded with the first true range, alpha = 1/n from bar 1
        _ => {
            out[0] = tr[0];
            for i in 1..len {
                out[i] = out[i - 1] + (tr[i] - out[i - 1]) / n as f32;
            }
        }
    }
    out
}

fn ma_null(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![NULLV; a.len()];
    if n == 0 {
        return out;
    }
    let mut s = 0f64;
    for i in 0..a.len() {
        s += a[i] as f64;
        if i >= n {
            s -= a[i - n] as f64;
        }
        if i + 1 >= n {
            out[i] = (s / n as f64) as f32;
        }
    }
    out
}

// ---------------------------------------------------------------- index helpers verified in ath_plus.rs (copied: Foreign alignment, calendar compression, TimeFrameExpand, pad and align)
mod apx {
    #![allow(dead_code)]
    use crate::ath_core::*;
    const NULL: f32 = -1.0e10;
pub fn is_null(x: f32) -> bool {
    x <= -9.0e9
}
/// Null-propagating arithmetic helpers (AmiBroker: any Null operand gives Null).
pub fn nadd(a: f32, b: f32) -> f32 {
    if is_null(a) || is_null(b) { NULL } else { a + b }
}
pub fn nsub(a: f32, b: f32) -> f32 {
    if is_null(a) || is_null(b) { NULL } else { a - b }
}
pub fn nmul(a: f32, b: f32) -> f32 {
    if is_null(a) || is_null(b) { NULL } else { a * b }
}

/// Result of the AFL's %swing / supertrend loop: trend (+1 / -1) and the TrendUp / TrendDown arrays (Null elsewhere, with the
/// AFL's `changeOfTrend` back-fill of the previous bar).
pub struct Sw {
    pub trend: Vec<i8>,
    pub tup: Vec<f32>,
    pub tdn: Vec<f32>,
}

/// The loop shared by every AFL here. `up` / `dn` are the initial bands; `up_flag` / `dn_flag` the values the band takes when the trend flips;
/// `extra` is the index swing filter's additional "or Close > index ATH close" condition.
pub fn swing_loop(close: &[f32], mut up: Vec<f32>, mut dn: Vec<f32>, up_flag: &[f32], dn_flag: &[f32], extra: Option<&[f32]>) -> Sw {
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
/// A foreign symbol's price arrays aligned to the current symbol's bar dates (AmiBroker `SetForeign` with the default fix-up:
/// a missing bar repeats the previous close with zero volume; before the foreign symbol starts the value is Null).
pub struct Al {
    pub o: Vec<f32>,
    pub h: Vec<f32>,
    pub l: Vec<f32>,
    pub c: Vec<f32>,
}

pub fn align(idx: &Series, dates: &[i32]) -> Al {
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

pub fn days_from_civil(y: i32, m: i32, d: i32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Key of the compressed bar a daily bar belongs to: calendar month, calendar week (Monday based) or the day itself.
pub fn period_key(d: i32, tf: u8) -> i64 {
    match tf {
        0 => (d / 100) as i64,
        1 => (days_from_civil(d / 10000, d / 100 % 100, d % 100) + 3).div_euclid(7),
        _ => d as i64,
    }
}

pub struct Comp {
    pub start: Vec<usize>,
    pub end: Vec<usize>,
    pub o: Vec<f32>,
    pub h: Vec<f32>,
    pub l: Vec<f32>,
    pub c: Vec<f32>,
}

/// `TimeFrameSet(monthly / weekly / daily)` of an aligned foreign array.
pub fn compress(dates: &[i32], al: &Al, tf: u8) -> Comp {
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
pub fn expand(x: &[f32], k: &Comp, n: usize) -> Vec<f32> {
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
pub struct Flt {
    pub tf: u8,
    pub sma: bool,
    pub point: u8,
    pub pd: usize,
    pub fact: f32,
    pub mapd: usize,
}

/// (FLTBuy, FLTShort) of the AFL's ST / SMA index filter on the stock's bar dates.
pub fn st_filter(al: &Al, dates: &[i32], f: &Flt) -> (Vec<bool>, Vec<bool>) {
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
        let mav: Vec<f32> = super::ma_null(&k.c, f.mapd);
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
pub fn st_filter_padded(ix: &Series, refc: &Series, dates: &[i32], f: &Flt) -> (Vec<bool>, Vec<bool>) {
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
}

// ---------------------------------------------------------------- variants 1 and 2: rank bucket + loss streak

#[derive(Clone, Copy)]
struct LossP {
    exit_pct: f32,
    comm: f32,
    losspct: f32,
    noofloss: f32,
    penny: f32,
    turnflt: bool,
    dead_yes: bool,
    ver: u8, // 1 OFF, 2 % hard SL, 3 ATR hard SL, 4 days lowest close
    hard_sl: f32,
    atr_prd: usize,
    atr_m: f32,
    days_sl: usize,
    turnoverpd: usize,
    bkmkt: u8,
    rankmode: u8,
    rocpd: usize,
    rsipd: usize,
    stdpd: usize,
    swingpd: usize,
    // index filter (variant 2)
    index: bool,
    flttf: u8, // 1 monthly 2 weekly 3 daily
    prflt: u8, // 1 NSENIFTY 2 NSENIFTY1 3 NIFTY 4 NSE500
    bear: bool,
    sma: bool,
    istflt: u8,
    ifltpd: usize,
    ifltfact: f32,
    ifltmapd: usize,
}

fn loss_params(g: G, v2: bool) -> LossP {
    LossP {
        exit_pct: g("exitpct") as f32,
        comm: g("comm") as f32,
        losspct: g("losspct") as f32,
        noofloss: g("noofloss") as f32,
        penny: g("pennyflt") as f32,
        turnflt: g("turnflt").round() == 1.0,
        dead_yes: g("deadstock").round() == 0.0,
        ver: g("ver").round().clamp(0.0, 3.0) as u8 + 1,
        hard_sl: g("hardslpct") as f32,
        atr_prd: g("atr_prd").max(0.0) as usize,
        atr_m: g("atr_sl_m") as f32,
        days_sl: g("days_sl").max(0.0) as usize,
        turnoverpd: g("turnoverpd").max(1.0) as usize,
        bkmkt: g("bkmkt").round().clamp(0.0, 3.0) as u8 + 1,
        rankmode: g("rankmode").round().clamp(0.0, 10.0) as u8 + 1,
        rocpd: g("rocpd").max(1.0) as usize,
        rsipd: g("rsipd").max(1.0) as usize,
        stdpd: g("stdpd").max(1.0) as usize,
        swingpd: g("swingpd").max(1.0) as usize,
        index: v2 && g("mktflt").round() == 1.0,
        flttf: g("flttf").round().clamp(0.0, 2.0) as u8,
        prflt: g("prflt").round().clamp(0.0, 3.0) as u8 + 1,
        bear: g("mktscene").round() == 1.0,
        sma: g("iflt").round() == 1.0,
        istflt: g("istflt").round().clamp(0.0, 2.0) as u8,
        ifltpd: g("ifltpd").max(1.0) as usize,
        ifltfact: g("ifltfact") as f32,
        ifltmapd: g("ifltmapd").max(1.0) as usize,
    }
}

/// PositionScore of the AFL: `turn = MA(C*V, turnoverpd)` unless a RANKING MODE replaces it.
fn position_score(s: &Series, p: &LossP, turn: &[f32]) -> Vec<f32> {
    let n = s.bars.len();
    let c = s.close();
    let nanify = |v: Vec<f32>| -> Vec<f32> { v.into_iter().map(|x| if x <= NULLV { f32::NAN } else { x }).collect() };
    let roc = |k: usize| -> Vec<f32> { (0..n).map(|i| if i >= k && c[i - k] != 0.0 { 100.0 * (c[i] / c[i - k] - 1.0) } else { f32::NAN }).collect() };
    let stdev = |k: usize| -> Vec<f32> {
        (0..n)
            .map(|i| {
                if i + 1 < k {
                    return f32::NAN;
                }
                let w = &c[i + 1 - k..=i];
                let m = w.iter().map(|&x| x as f64).sum::<f64>() / k as f64;
                ((w.iter().map(|&x| (x as f64 - m) * (x as f64 - m)).sum::<f64>() / k as f64).sqrt()) as f32
            })
            .collect()
    };
    // POCL: number of up closes among the last ROCPD bars (what the AFL's nested loop leaves in the array)
    let pocl = || -> Vec<f32> {
        let k = p.rocpd;
        let mut out = vec![0f32; n];
        for i in 1..n {
            let lo = if i > k { i - k + 1 } else { 1 };
            out[i] = (lo..=i).filter(|&j| c[j] > c[j - 1]).count() as f32;
        }
        out
    };
    let sdiffpct = || -> Vec<f32> {
        (0..n)
            .map(|i| {
                let lo = i.saturating_sub(p.swingpd - 1);
                let top = c[lo..=i].iter().copied().fold(f32::MIN, f32::max);
                ((top - c[i]) / top) * 100.0
            })
            .collect()
    };
    let rsi = || -> Vec<f32> {
        let k = p.rsipd;
        let mut out = vec![f32::NAN; n];
        if n > k {
            let (mut au, mut ad) = (0f32, 0f32);
            for i in 1..=k {
                let d = c[i] - c[i - 1];
                if d > 0.0 { au += d } else { ad -= d }
            }
            au /= k as f32;
            ad /= k as f32;
            out[k] = if au + ad > 0.0 { 100.0 * au / (au + ad) } else { 50.0 };
            for i in k + 1..n {
                let d = c[i] - c[i - 1];
                au = (au * (k as f32 - 1.0) + d.max(0.0)) / k as f32;
                ad = (ad * (k as f32 - 1.0) + (-d).max(0.0)) / k as f32;
                out[i] = if au + ad > 0.0 { 100.0 * au / (au + ad) } else { 50.0 };
            }
        }
        out
    };
    let add = |a: Vec<f32>, b: Vec<f32>| -> Vec<f32> { a.iter().zip(b.iter()).map(|(x, y)| x + y).collect() };
    match p.rankmode {
        2 => roc(p.rocpd).into_iter().map(|x| 100.0 + x).collect(),
        3 => roc(p.rocpd).into_iter().map(|x| 100.0 - x).collect(),
        4 => rsi(),
        5 => stdev(p.stdpd),
        6 => {
            let sd = stdev(p.stdpd);
            roc(p.rocpd).iter().zip(sd.iter()).map(|(r, d)| 100.0 + r / d).collect()
        }
        7 => {
            let sd = stdev(p.stdpd);
            roc(p.rocpd).iter().zip(sd.iter()).map(|(r, d)| 100.0 - r / d).collect()
        }
        8 => pocl(),
        9 => sdiffpct().into_iter().map(|x| 100.0 - x).collect(),
        10 => add(roc(p.rocpd).into_iter().map(|x| 100.0 + x).collect(), pocl()),
        11 => add(sdiffpct().into_iter().map(|x| 100.0 - x).collect(), pocl()),
        _ => nanify(turn.to_vec()),
    }
}

/// Rank of every symbol's bar by the turnover average (1 = highest), per date over the symbols that have a bar that date
/// (`StaticVarGenerateRanks("rank", "ValuesToSort", 0, 1234)`: normal mode, ties by order of appearance).
fn turnover_ranks(series: &[Series], values: &[Vec<f32>], fill: bool) -> Vec<Vec<u16>> {
    let mut ranks: Vec<Vec<u16>> = series.iter().map(|s| vec![0u16; s.bars.len()]).collect();
    let mut dates: Vec<i32> = series.iter().flat_map(|s| s.bars.iter().map(|b| b.date)).collect();
    dates.sort_unstable();
    dates.dedup();
    let mut cur: Vec<usize> = vec![0; series.len()]; // next bar index per symbol
    let mut last: Vec<f32> = vec![f32::NAN; series.len()]; // last value (fill mode)
    let mut buf: Vec<(f32, usize, usize)> = Vec::with_capacity(series.len());
    for &d in &dates {
        buf.clear();
        for (k, s) in series.iter().enumerate() {
            let i = cur[k];
            if i < s.bars.len() && s.bars[i].date == d {
                last[k] = values[k][i];
                buf.push((values[k][i], k, i));
                cur[k] += 1;
            } else if fill && !last[k].is_nan() {
                buf.push((last[k], k, usize::MAX));
            }
        }
        // descending by value, ties by order of appearance (sort_by is stable)
        buf.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        for (r, &(_, k, i)) in buf.iter().enumerate() {
            if i != usize::MAX {
                ranks[k][i] = (r + 1).min(u16::MAX as usize) as u16;
            }
        }
    }
    ranks
}

/// The per-bar loop of variants 1 (`v2 == false`) and 2.
fn loss_loop(s: &Series, p: &LossP, v2: bool, rank: &[u16], score: Vec<f32>, idx: Option<&Series>) -> Signals {
    let n = s.bars.len();
    let c: Vec<f32> = s.close();
    let o: Vec<f32> = s.open();
    let h: Vec<f32> = s.high();
    let l: Vec<f32> = s.low();
    let v: Vec<f32> = s.volume();
    let hcl = running_high(&c);
    let sw = swing(&c, p.exit_pct);
    let athc = athc_state(&c, &hcl, &sw.trend);
    let uc: Vec<bool> = (0..n).map(|i| h[i] == c[i]).collect();
    let lc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let upcirc: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i])).collect();
    let locirc: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i])).collect();
    let turnover: Vec<bool> = (0..n).map(|i| if p.turnflt { v[i] * c[i] > volsig(s.bars[i].date / 10000) } else { true }).collect();
    let dead_buy = |i: usize| if p.dead_yes { i != n - 1 } else { i != 0 };
    let dead_sell = |i: usize| p.dead_yes && i == n - 1;
    let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !upcirc[i] && dead_buy(i) && c[i] > p.penny && turnover[i]).collect();
    let cc = concount(&c, &buysig, &sw, p.losspct, p.comm);
    let dead = if v2 { dead30(s) } else { vec![false; n] };
    // index filter (ath_plus.rs semantics, verified there on the team's combo sheets): the AFL's SetForeign/TimeFrameSet block is
    // closed by TimeFrameRestore(), the stock's own arrays stay in use. Weekly/monthly values are shown on the period's last bar.
    let (cm, hm, lm) = (c.clone(), h.clone(), l.clone());
    let (mut flt_buy, mut flt_short) = (vec![false; n], vec![true; n]);
    if p.index {
        match idx {
            Some(ix) => {
                let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
                let al = apx::align(ix, &dates);
                let f = apx::Flt { tf: p.flttf, sma: p.sma, point: p.istflt, pd: p.ifltpd, fact: p.ifltfact, mapd: p.ifltmapd };
                let (fb, fs) = apx::st_filter(&al, &dates, &f);
                flt_buy = fb;
                flt_short = fs;
            }
            None => {
                flt_short = vec![false; n]; // index missing from the database (e.g. NIFTY): no entries
            }
        }
    }
    let myatr = atr_ab(&hm, &lm, &cm, p.atr_prd, 0);
    // SL1 = LLV(C, Ref(Days_SL,-1)) if Days_SL > 1 else Null
    let sl1: Vec<f32> = (0..n)
        .map(|i| {
            if p.ver == 4 && p.days_sl > 1 && i >= 1 {
                let lo = i.saturating_sub(p.days_sl - 1);
                cm[lo..=i].iter().copied().fold(f32::MAX, f32::min)
            } else {
                NULLV
            }
        })
        .collect();
    let band = |r: u16| -> bool {
        let (r1, r2) = match p.bkmkt {
            1 => (0u32, 101u32),
            2 => (100, 251),
            3 => (250, 801),
            _ => (800, 80000),
        };
        (r as u32) > r1 && (r as u32) < r2
    };
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut bp, mut sp) = (c.clone(), c.clone());
    let (mut position, mut skipsell) = (false, false);
    let (mut hsl, mut atr_hsl, mut sl) = (0f32, 0f32, 0f32);
    for i in 1..n {
        let cash_filter = !uc[i] && turnover[i] && cm[i] > p.penny && dead_buy(i);
        let entry_ok = !position
            && cash_filter
            && (!p.index || flt_short[i])
            && cc[i] >= p.noofloss
            && cm[i] > hcl[i - 1]
            && band(rank[i])
            && !dead[i];
        if entry_ok {
            buy[i] = true;
            bp[i] = cm[i];
            if v2 {
                if p.ver == 2 && p.hard_sl > 0.0 {
                    hsl = (1.0 - p.hard_sl * 0.01) * bp[i];
                }
                if p.ver == 3 && myatr[i] > 0.0 {
                    atr_hsl = cm[i] - myatr[i] * p.atr_m;
                }
                if p.ver == 4 {
                    sl = sl1[i];
                }
            } else {
                match p.ver {
                    2 if p.hard_sl > 0.0 => hsl = (1.0 - p.hard_sl * 0.01) * bp[i],
                    3 if myatr[i] > 0.0 => atr_hsl = cm[i] - myatr[i] * p.atr_m,
                    4 if sl1[i - 1] > 0.0 => sl = sl1[i],
                    _ => {}
                }
            }
            position = true;
        } else if position && !skipsell {
            let trig;
            let can_sell;
            if v2 {
                let stops = cm[i] <= hsl || cm[i] <= atr_hsl || cm[i] <= sl;
                if p.index {
                    trig = stops || dead_sell(i) || sw.trend[i] == -1 || flt_buy[i];
                } else {
                    trig = stops || dead_sell(i) || sw.trend[i] == -1 || dead[i];
                }
                can_sell = !lc[i] || dead_sell(i) || dead[i];
            } else {
                let stop = match p.ver {
                    2 => cm[i] <= hsl,
                    3 => cm[i] <= atr_hsl,
                    4 => cm[i] < sl,
                    _ => false,
                };
                trig = stop || dead_sell(i) || sw.trend[i] == -1;
                can_sell = if p.ver == 1 { !lc[i] || dead_sell(i) } else { !locirc[i] || dead_sell(i) };
            }
            if trig {
                if can_sell {
                    sell[i] = true;
                    sp[i] = cm[i];
                    position = false;
                    if !v2 {
                        hsl = 0.0;
                        atr_hsl = 0.0;
                        sl = 0.0;
                    }
                } else {
                    skipsell = true;
                }
            }
        } else if skipsell {
            let ok = if v2 {
                !lc[i] || dead_sell(i) || dead[i]
            } else if p.ver == 1 {
                !locirc[i] || dead_sell(i)
            } else {
                !locirc[i] || dead_sell(i)
            };
            if ok {
                sell[i] = true;
                sp[i] = cm[i];
                position = false;
                skipsell = false;
                if !v2 {
                    hsl = 0.0;
                    atr_hsl = 0.0;
                    sl = 0.0;
                }
            }
        }
    }
    if !v2 {
        buy = exrem(&buy, &sell);
        sell = exrem(&sell, &buy);
    }
    // the fill stays inside the bar's range (AmiBroker)
    for i in 0..n {
        bp[i] = bp[i].min(h[i]).max(l[i]);
        sp[i] = sp[i].min(h[i]).max(l[i]);
    }
    let mut sg = Signals::new(buy, sell, bp, sp);
    sg.score = Some(score);
    sg
}

fn loss_signals(series: &[Series], env: &Env, g: G, v2: bool) -> Vec<Signals> {
    let p = loss_params(g, v2);
    // ranks over the symbols that have a bar on the date (tested against carrying stale values of absent symbols: worse, see RESULT.md)
    let fill = false;
    // turn = MA(C*V, turnoverpd) per symbol; the rank value is 0 for dead stocks (30 identical bars)
    let turns: Vec<Vec<f32>> = series
        .iter()
        .map(|s| {
            let to: Vec<f32> = s.bars.iter().map(|b| b.c * b.v).collect();
            ma_null(&to, p.turnoverpd)
        })
        .collect();
    let values: Vec<Vec<f32>> = series
        .iter()
        .zip(turns.iter())
        .map(|(s, t)| {
            let d = dead30(s);
            (0..t.len()).map(|i| if d[i] { 0.0 } else { t[i] }).collect()
        })
        .collect();
    let ranks = turnover_ranks(series, &values, fill);
    let idx_name = match p.prflt {
        1 => "NSENIFTY",
        2 => "NSENIFTY1",
        3 => "NIFTY",
        _ => "NSE500",
    };
    series
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let score = position_score(s, &p, &turns[k]);
            loss_loop(s, &p, v2, &ranks[k], score, env.get(idx_name))
        })
        .collect()
}

fn loss_common_schema() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
        p("ver", "SL EXIT (0 OFF, 1 % Based HARD SL, 2 ATR Based HARD SL, 3 5 DAYS LOWEST CLOSE)", 0.0, 0.0, 3.0, 1.0),
        p("hardslpct", "HARD SL%", 2.0, 0.0, 12.0, 1.0),
        p("atr_prd", "ATR Prds", 5.0, 0.0, 100.0, 1.0),
        p("atr_sl_m", "ATR Hard SL Multiplier", 3.0, 0.0, 6.0, 1.0),
        p("days_sl", "DAYS SL", 1.0, 1.0, 100.0, 1.0),
    ]
}

fn loss_tail_schema() -> Vec<P> {
    vec![
        p("noofloss", "CONSICUTIVE LOSS", 2.0, 0.0, 2.0, 1.0),
        p("bkmkt", "BACKTEST MARKET AS PER VOLUME (0 LARGECAP rank 1-100, 1 MIDCAP 101-250, 2 SMALLCAP 251-800, 3 MICROCAP 801+)", 0.0, 0.0, 3.0, 1.0),
        p("rankmode", "RANKING MODE (0 NONE, 1 POSITIVE ROC, 2 NEGATIVE ROC, 3 RSI, 4 STD, 5 POSITIVE ROC/STD, 6 NEGATIVE ROC/STD, 7 POSITIVE CLOSE, 8 SWING, 9 POSITIVE ROC + POSITIVE CLOSE, 10 POSITIVE CLOSE + SWING)", 0.0, 0.0, 10.0, 1.0),
        p("rocpd", "ROC PERIOD for RANKING/NO OF POSITIVE CLOSE", 10.0, 30.0, 400.0, 30.0),
        p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("stdpd", "STD PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("swingpd", "SWING PERIOD", 180.0, 10.0, 1000.0, 1.0),
        p("comm", "BROKERAGE % used by the AFL's virtual-trade profit test (the Vault's brokerage)", 0.65, 0.0, 1.0, 0.01),
    ]
}

fn schema_v1() -> Vec<P> {
    let mut v = loss_common_schema();
    v.extend([
        p("exitpct", "EXIT %", 10.0, 5.0, 25.0, 1.0),
        p("losspct", "MIN PROFIT %", 3.0, 0.0, 5.0, 1.0),
        p("turnoverpd", "TURNOVER AVG PERIOD", 60.0, 1.0, 200.0, 10.0),
    ]);
    v.extend(loss_tail_schema());
    v
}

fn schema_v2() -> Vec<P> {
    let mut v = loss_common_schema();
    v.extend([
        p("exitpct", "EXIT % (Optimize 21..24 in the AFL, default 10)", 10.0, 5.0, 50.0, 1.0),
        p("mktflt", "FILTER TYPE (0 NONE, 1 INDEX)", 0.0, 0.0, 1.0, 1.0),
        p("flttf", "FILTER TIMEFRAME (0 MONTHLY, 1 WEEKLY, 2 DAILY)", 0.0, 0.0, 2.0, 1.0),
        p("prflt", "INDEX (0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 0.0, 0.0, 3.0, 1.0),
        p("mktscene", "Market Condition (0 Bull, 1 Bear)", 0.0, 0.0, 1.0, 1.0),
        p("iflt", "INDEX Filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0),
        p("istflt", "ST CALCULATION POINT (0 MID POINT(H+L/2), 1 CLOSE, 2 PIVOT(H+L+C/3))", 0.0, 0.0, 2.0, 1.0),
        p("ifltpd", "INDEX ST PERIOD (Optimize in the AFL, default 20)", 20.0, 5.0, 30.0, 5.0),
        p("ifltfact", "INDEX ST FACTOR (Optimize in the AFL, default 1)", 1.0, 1.0, 2.0, 0.5),
        p("ifltmapd", "INDEX SMA PERIOD", 200.0, 1.0, 300.0, 1.0),
        p("losspct", "MIN PROFIT %", 3.0, 0.0, 45.0, 1.0),
        p("turnoverpd", "TURNOVER AVG PERIOD (Optimize 10..30 in the AFL, default 60)", 60.0, 1.0, 200.0, 5.0),
    ]);
    v.extend(loss_tail_schema());
    v
}

fn signals_v1(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    loss_signals(series, env, g, false)
}
fn signals_v2(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    loss_signals(series, env, g, true)
}

// ---------------------------------------------------------------- variant 3: ATHC & %swing with fixed-ATR scale-in (5 tranches)

#[derive(Clone, Copy, Debug)]
pub struct ScaleP {
    pub exit_pct: f32,
    pub losspct: f32,
    pub noofloss: f32,
    pub comm: f32,
    pub penny: f32,
    pub turnflt: bool,
    pub dead_yes: bool,
    pub atr_prd: usize,
    pub myatr1: f32,
    pub buffer: f32,
    /// 0 = SMA seed + Wilder, 1 = seeded with the first true range, alpha 1/n
    pub atr_mode: u8,
    /// the plain (no loss gate) AFL: no concount gate, V*C turnover with the 1.8M table, MasterFilter missing in the 3-level scale-in
    pub plain: bool,
}

/// What the AFL's main loop does on each bar.
pub struct ScaleOut {
    /// 0 nothing, 1 new entry (`Buy = 1`), 2 scale-in (`Buy = sigScaleIn`)
    pub buy: Vec<u8>,
    /// the AFL's `po[i]`: size multiple of the order (0 for a first entry = one tranche)
    pub po: Vec<f32>,
    pub buy_price: Vec<f32>,
    pub sell: Vec<bool>,
    pub sell_price: Vec<f32>,
    /// `op[i]` branch number (debug)
    pub op: Vec<u8>,
}

pub fn scalein_events(s: &Series, p: &ScaleP) -> ScaleOut {
    let n = s.bars.len();
    let c = s.close();
    let o = s.open();
    let h = s.high();
    let l = s.low();
    let oi = s.openint();
    let hcl = running_high(&c);
    let sw = swing(&c, p.exit_pct);
    let athc = athc_state(&c, &hcl, &sw.trend);
    let uc: Vec<bool> = (0..n).map(|i| h[i] == c[i]).collect();
    let lc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let upc: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i])).collect();
    let loc: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i])).collect();
    let turnover: Vec<bool> = (0..n)
        .map(|i| if !p.turnflt { true } else if p.plain { s.bars[i].v * c[i] > volsig4(s.bars[i].date / 10000) } else { oi[i] > volsig(s.bars[i].date / 10000) })
        .collect();
    let dead_buy = |i: usize| if p.dead_yes { i != n - 1 } else { i != 0 };
    let dead_sell = |i: usize| p.dead_yes && i == n - 1;
    let buysig_v: Vec<bool> = (0..n).map(|i| athc[i] && !upc[i] && dead_buy(i) && c[i] > p.penny && turnover[i]).collect();
    let cc = concount(&c, &buysig_v, &sw, p.losspct, p.comm);
    let myatr = atr_ab(&h, &l, &c, p.atr_prd, p.atr_mode);
    let master: Vec<bool> = (0..n).map(|i| c[i] > p.penny && dead_buy(i)).collect();
    let other_filter: Vec<bool> = (0..n).map(|i| !turnover[i] || upc[i] || uc[i]).collect();
    let skip_buy_filter: Vec<bool> = (0..n).map(|i| turnover[i] && !upc[i]).collect();
    let max_position = 5.0f32;
    let mut out = ScaleOut { buy: vec![0; n], po: vec![0.0; n], buy_price: c.clone(), sell: vec![false; n], sell_price: c.clone(), op: vec![0; n] };
    let mut pos = vec![0f32; n];
    let mut buysig = vec![NULLV; n];
    let (mut atr_value, mut skip, mut abc) = (0f32, 0f32, 0f32);
    let buf = p.buffer;
    for i in 1..n {
        let pp = pos[i - 1];
        let b = buysig[i - 1];
        let a = atr_value;
        let ob = o[i] + (o[i] * buf) / 100.0; // Open + buffer
        let b1 = b + a;
        let b2 = b + 2.0 * a;
        let b3 = b + 3.0 * a;
        if pp == 0.0 && (athc[i] || c[i] > hcl[i - 1]) && (p.plain || cc[i] >= p.noofloss) && master[i] && myatr[i] > 0.0 {
            if other_filter[i] {
                atr_value = myatr[i] * p.myatr1;
                buysig[i] = c[i] + atr_value;
                pos[i] = 1.0;
                skip += 1.0;
                out.op[i] = 1;
            } else {
                out.buy[i] = 1;
                atr_value = myatr[i] * p.myatr1;
                buysig[i] = c[i] + atr_value;
                pos[i] = 1.0;
                abc += 1.0;
                out.op[i] = 2;
            }
        } else if ((pp > 0.0 && pp < max_position && h[i] > b && h[i] < b1) || (pp == 4.0 && h[i] > b)) && master[i] {
            if pp > 0.0 && !upc[i] {
                out.po[i] = skip + 1.0;
                out.buy[i] = 2;
                out.buy_price[i] = if o[i] > b {
                    ob
                } else if skip > 0.0 {
                    ((ob * skip) + b) / (skip + 1.0)
                } else {
                    b
                };
                pos[i] = pp + 1.0;
                buysig[i] = b + atr_value;
                abc += 1.0;
                skip = 0.0;
                out.op[i] = 3;
            } else {
                pos[i] = pp + 1.0;
                buysig[i] = b + atr_value;
                skip += 1.0;
                out.op[i] = 4;
            }
        } else if ((pp > 0.0 && pp < max_position && h[i] > b && h[i] > b1 && h[i] < b + 2.0 * a) || (pp == 3.0 && pp < max_position && h[i] > b && h[i] > b1)) && master[i] {
            if pp > 0.0 && !upc[i] {
                out.po[i] = skip + 2.0;
                out.buy[i] = 2;
                if o[i] > b && o[i] > b1 {
                    out.buy_price[i] = ob;
                } else if skip > 0.0 && o[i] > b && h[i] > b1 {
                    out.buy_price[i] = ((ob * (skip + 1.0)) + b1) / (skip + 2.0);
                } else if skip > 0.0 && h[i] > b && h[i] > b1 {
                    out.buy_price[i] = ((ob * skip) + b + b1) / (skip + 2.0);
                } else if skip == 0.0 && o[i] > b && h[i] > b1 {
                    out.buy_price[i] = (ob + b1) / 2.0;
                } else if skip == 0.0 && h[i] > b && h[i] > b1 {
                    out.buy_price[i] = (b + b1) / 2.0;
                }
                pos[i] = pp + 2.0;
                buysig[i] = b + 2.0 * atr_value;
                abc += 1.0;
                skip = 0.0;
                out.op[i] = 5;
            } else {
                pos[i] = pp + 2.0;
                buysig[i] = b + 2.0 * atr_value;
                skip += 2.0;
                out.op[i] = 6;
            }
        } else if (if p.plain {
            // plain AFL: `(A) OR (B AND MasterFilter)`, precedence of AND over OR
            (pp > 0.0 && pp < max_position && h[i] > b && h[i] > b1 && h[i] > b2 && h[i] < b + 3.0 * a) || (pp == 2.0 && pp < max_position && h[i] > b && h[i] > b1 && h[i] > b2 && master[i])
        } else {
            ((pp > 0.0 && pp < max_position && h[i] > b && h[i] > b1 && h[i] > b2 && h[i] < b + 3.0 * a) || (pp == 2.0 && pp < max_position && h[i] > b && h[i] > b1 && h[i] > b2)) && master[i]
        }) {
            if pp > 0.0 && !upc[i] {
                out.po[i] = skip + 3.0;
                out.buy[i] = 2;
                if o[i] > b && o[i] > b1 && o[i] > b2 {
                    out.buy_price[i] = ob;
                } else if skip > 0.0 && o[i] > b && o[i] > b1 && h[i] > b2 {
                    out.buy_price[i] = ((ob * (skip + 2.0)) + b2) / (skip + 3.0);
                } else if skip > 0.0 && o[i] > b && h[i] > b1 && h[i] > b2 {
                    out.buy_price[i] = ((ob * (skip + 1.0)) + b1 + b2) / (skip + 3.0);
                } else if skip > 0.0 && h[i] > b && h[i] > b1 && h[i] > b2 {
                    out.buy_price[i] = ((ob * skip) + b + b1 + b2) / (skip + 3.0);
                } else if skip == 0.0 && o[i] > b && o[i] > b1 && h[i] > b2 {
                    out.buy_price[i] = ((ob * 2.0) + b2) / 3.0;
                } else if skip == 0.0 && o[i] > b && h[i] > b1 && h[i] > b2 {
                    out.buy_price[i] = (ob + b1 + b2) / 3.0;
                } else if skip == 0.0 && h[i] > b && h[i] > b1 && h[i] > b2 {
                    out.buy_price[i] = (b + b1 + b2) / 3.0;
                }
                pos[i] = pp + 3.0;
                buysig[i] = b + 3.0 * atr_value;
                abc += 1.0;
                skip = 0.0;
                out.op[i] = 7;
            } else {
                pos[i] = pp + 3.0;
                buysig[i] = b + 3.0 * atr_value;
                skip += 3.0;
                out.op[i] = 8;
            }
        } else if pp > 0.0 && pp < max_position && h[i] > b3 && master[i] {
            if pp > 0.0 && !upc[i] {
                out.po[i] = skip + 4.0;
                out.buy[i] = 2;
                if o[i] > b && o[i] > b1 && o[i] > b2 && o[i] > b3 {
                    out.buy_price[i] = ob;
                } else if skip > 0.0 && o[i] > b && o[i] > b1 && o[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = ((ob * (skip + 3.0)) + b3) / (skip + 4.0);
                } else if skip > 0.0 && o[i] > b && o[i] > b1 && h[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = ((ob * (skip + 2.0)) + b2 + b3) / (skip + 4.0);
                } else if skip > 0.0 && o[i] > b && h[i] > b1 && h[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = ((ob * (skip + 1.0)) + b1 + b2 + b3) / (skip + 4.0);
                } else if skip > 0.0 && h[i] > b && h[i] > b1 && h[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = (ob + b + b1 + b2 + b3) / (skip + 4.0);
                } else if skip == 0.0 && o[i] > b && o[i] > b1 && o[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = ((ob * 3.0) + b3) / 4.0;
                } else if skip == 0.0 && o[i] > b && o[i] > b1 && h[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = ((ob * 2.0) + b2 + b3) / 4.0;
                } else if skip == 0.0 && o[i] > b && h[i] > b1 && h[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = (ob + b1 + b2 + b3) / 4.0;
                } else if skip == 0.0 && h[i] > b && h[i] > b1 && h[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = (b + b1 + b2 + b3) / 4.0;
                }
                pos[i] = pp + 4.0;
                buysig[i] = NULLV;
                abc += 1.0;
                skip = 0.0;
                out.op[i] = 9;
            } else {
                pos[i] = pp + 4.0;
                buysig[i] = b;
                skip += 4.0;
                out.op[i] = 10;
            }
        } else if skip > 0.0 && sw.trend[i] == 1 && skip_buy_filter[i] && master[i] {
            out.po[i] = skip;
            out.buy[i] = 2;
            out.buy_price[i] = ob;
            pos[i] = pp;
            buysig[i] = b;
            abc += 1.0;
            skip = 0.0;
            out.op[i] = 11;
        } else if pp > 0.0 && abc == 0.0 && skip > 0.0 && sw.trend[i] == -1 {
            pos[i] = 0.0;
            skip = 0.0;
            abc = 0.0;
        } else if (pp > 0.0 && abc > 0.0 && (sw.trend[i] == -1 || !athc[i]) && !(loc[i] || lc[i])) || dead_sell(i) {
            out.sell[i] = true;
            if (loc[i - 1] || lc[i - 1]) && sw.trend[i - 1] == -1 {
                out.sell_price[i] = o[i] - (o[i] * buf) / 100.0;
                out.op[i] = 12;
            } else {
                out.sell_price[i] = c[i];
                out.op[i] = 13;
            }
            pos[i] = 0.0;
            atr_value = 0.0;
            skip = 0.0;
            abc = 0.0;
            buysig[i] = NULLV;
        } else {
            pos[i] = pp;
            buysig[i] = b;
        }
    }
    out
}

fn scale_params(g: G, plain: bool) -> ScaleP {
    ScaleP {
        exit_pct: g("exitpct") as f32,
        losspct: g("losspct") as f32,
        noofloss: g("noofloss") as f32,
        comm: g("comm") as f32,
        penny: g("pennyflt") as f32,
        turnflt: g("turnflt").round() == 1.0,
        dead_yes: g("deadstock").round() == 0.0,
        atr_prd: g("atr_prd").max(0.0) as usize,
        myatr1: g("myatr1") as f32,
        buffer: g("buffer_prcnt") as f32,
        atr_mode: g("atr_mode").round() as u8,
        plain,
    }
}

/// The signals the portfolio runner can use: one buy per position on the bar of its first real tranche (the first tranche is bought at
/// the close, a scale-in that opens the position at its own price), one sell. The later tranches cannot be represented by `Signals`
/// (see RESULT.md: the AmiBroker scale-in needs support in the portfolio core).
pub fn scalein_signals(s: &Series, p: &ScaleP) -> Signals {
    let e = scalein_events(s, p);
    let n = s.bars.len();
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut open = false;
    for i in 0..n {
        if e.sell[i] && open {
            sell[i] = true;
            open = false;
        }
        if e.buy[i] > 0 && !open {
            buy[i] = true;
            open = true;
        }
    }
    let mut bp = e.buy_price.clone();
    let mut sp = e.sell_price.clone();
    for i in 0..n {
        bp[i] = bp[i].min(s.bars[i].h).max(s.bars[i].l);
        sp[i] = sp[i].min(s.bars[i].h).max(s.bars[i].l);
    }
    let mut sg = Signals::new(buy, sell, bp.clone(), sp);
    // later tranches (Buy = sigScaleIn while the position is open) go to the portfolio as scale-in orders; a tranche is one of 5
    let mut open = false;
    for i in 0..n {
        if e.sell[i] && open {
            open = false;
        }
        if e.buy[i] > 0 && !open {
            open = true;
        } else if e.buy[i] == 2 && open {
            sg.scale.push(ScaleIn { bar: i, price: bp[i], mult: if e.po[i] > 0.0 { e.po[i] } else { 1.0 } });
        }
    }
    sg.entry_size = 0.2;
    sg
}

pub fn schema_v3() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; OpenInt > yearly table)", 0.0, 0.0, 1.0, 1.0),
        p("exitpct", "EXIT %", 10.0, 5.0, 25.0, 1.0),
        p("losspct", "MIN PROFIT %", 3.0, 1.0, 10.0, 0.05),
        p("noofloss", "CONSICUTIVE LOSS", 2.0, 0.0, 2.0, 1.0),
        p("atr_prd", "ATR Prds", 14.0, 5.0, 100.0, 1.0),
        p("myatr1", "ATR Multiplier", 1.0, 0.0, 5.0, 0.025),
        p("buffer_prcnt", "Buffer %", 0.25, 0.0, 2.0, 0.001),
        p("comm", "BROKERAGE % used by the AFL's virtual-trade profit test (the Vault's brokerage)", 0.65, 0.0, 1.0, 0.01),
        p("atr_mode", "ENGINE OPTION (not in the AFL): ATR smoothing, 0 = SMA seed then Wilder, 1 = first true range seed, alpha 1/n", 0.0, 0.0, 1.0, 1.0),
    ]
}

fn signals_v3(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let p = scale_params(g, false);
    series.iter().map(|s| scalein_signals(s, &p)).collect()
}

fn signals_v7(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let p = scale_params(g, true);
    series.iter().map(|s| scalein_signals(s, &p)).collect()
}

fn schema_v7() -> Vec<P> {
    // the plain AFL has no loss gate: MIN PROFIT / CONSICUTIVE LOSS do not exist
    schema_v3().into_iter().filter(|q| q.key != "losspct" && q.key != "noofloss" && q.key != "comm").collect()
}

// ---------------------------------------------------------------- variant 4: running ATH buy, %swing exit after %swing losses, no filter

fn volsig4(year: i32) -> f32 {
    match year {
        y if y < 2000 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

fn run_signals(s: &Series, g: G, tmode: u8, vstate: bool, ucblock: bool) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l, v) = (s.close(), s.open(), s.high(), s.low(), s.volume());
    let exit_pct = g("exitpct") as f32;
    let (losspct, noofloss, comm, penny) = (g("losspct") as f32, g("noofloss") as f32, g("comm") as f32, g("pennyflt") as f32);
    let turnflt = g("turnflt").round() == 1.0;
    let dead_yes = g("deadstock").round() == 0.0;
    let hcl = running_high(&c);
    let sw = swing(&c, exit_pct);
    let athc = athc_state(&c, &hcl, &sw.trend);
    let upc: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i])).collect();
    let loc: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i])).collect();
    let turnover: Vec<bool> = (0..n)
        .map(|i| if !turnflt { true } else if tmode == 1 { s.bars[i].oi > volsig(s.bars[i].date / 10000) } else { v[i] * c[i] > volsig4(s.bars[i].date / 10000) })
        .collect();
    let dead_buy = |i: usize| if dead_yes { i != n - 1 } else { i != 0 };
    let fresh = |i: usize| i >= 1 && c[i] > hcl[i - 1];
    // virtual trades: the 19% file buys on a fresh ATH close only; the older "Running ATH Buy" copy uses the ATHC state (vstate = 1)
    let buysig_v: Vec<bool> = (0..n).map(|i| (if vstate { athc[i] } else { fresh(i) }) && !upc[i] && dead_buy(i) && c[i] > penny && turnover[i]).collect();
    let cc = concount(&c, &buysig_v, &sw, losspct, comm);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = (athc[i] || fresh(i)) && cc[i] >= noofloss && !upc[i] && !(ucblock && h[i] == c[i]) && dead_buy(i) && c[i] > penny && turnover[i];
        sell[i] = ((sw.trend[i] == -1 || !athc[i]) && !loc[i]) || (dead_yes && i == n - 1);
    }
    // bar 0: concount/athc are 0 there, AmiBroker evaluates the vectors on bar 0 too (Ref(Hcl,-1) is Null: C > Null is true)
    if n > 0 {
        buy[0] = (athc[0] || true) && cc[0] >= noofloss && !upc[0] && dead_buy(0) && c[0] > penny && turnover[0];
    }
    let buy = exrem(&buy, &sell);
    let sell = exrem(&sell, &buy);
    let (mut bp, mut sp) = (c.clone(), c.clone());
    for i in 0..n {
        bp[i] = bp[i].min(h[i]).max(l[i]);
        sp[i] = sp[i].min(h[i]).max(l[i]);
    }
    let mut sg = Signals::new(buy, sell, bp, sp);
    let hard = g("hardstop") as f32;
    if hard > 0.0 {
        sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
    }
    sg
}

fn schema_v4() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop, not checked against a sheet)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; V*C > yearly table)", 1.0, 0.0, 1.0, 1.0),
        p("exitpct", "EXIT %", 19.0, 5.0, 25.0, 1.0),
        p("losspct", "MIN PROFIT %", 2.0, 1.0, 3.0, 0.05),
        p("noofloss", "CONSICUTIVE LOSS", 1.0, 0.0, 2.0, 1.0),
        p("rankmode", "RANKING MODE (0 NONE; other modes not coded for this variant)", 0.0, 0.0, 10.0, 1.0),
        p("rocpd", "ROC PERIOD for RANKING/NO OF POSITIVE CLOSE", 10.0, 30.0, 400.0, 30.0),
        p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("stdpd", "STD PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("swingpd", "SWING PERIOD", 180.0, 10.0, 1000.0, 1.0),
        p("comm", "BROKERAGE % used by the AFL's virtual-trade profit test (the Vault's brokerage)", 0.65, 0.0, 1.0, 0.01),
        p("ucblock", "ENGINE OPTION (not in the AFL): 1 = no entry on a bar with Close == High", 0.0, 0.0, 1.0, 1.0),
        p("vstate", "ENGINE OPTION (not in the AFL): 0 = virtual trades buy a fresh ATH close (19% file), 1 = ATHC state (older Running ATH Buy copy)", 0.0, 0.0, 1.0, 1.0),
    ]
}

fn signals_v4(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| run_signals(s, g, 0, g("vstate").round() == 1.0, g("ucblock").round() == 1.0)).collect()
}


// ---------------------------------------------------------------- variant 5: running ATH buy, %swing exit after 1 loss, "Final version" file

fn signals_v5(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    // that AFL: virtual trades on the ATHC state, no entry on Close == High, OpenInt > yearly table
    series.iter().map(|s| run_signals(s, g, 1, true, true)).collect()
}

fn schema_v5() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop, not checked against a sheet)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; OpenInt > yearly table)", 0.0, 0.0, 1.0, 1.0),
        p("exitpct", "EXIT %", 10.0, 5.0, 25.0, 1.0),
        p("losspct", "MIN PROFIT %", 3.0, 0.0, 5.0, 1.0),
        p("noofloss", "CONSICUTIVE LOSS", 2.0, 1.0, 2.0, 1.0),
        p("rankmode", "RANKING MODE (0 NONE; other modes not coded for this variant)", 0.0, 0.0, 10.0, 1.0),
        p("rocpd", "ROC PERIOD for RANKING/NO OF POSITIVE CLOSE", 10.0, 30.0, 400.0, 30.0),
        p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("stdpd", "STD PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("swingpd", "SWING PERIOD", 180.0, 10.0, 1000.0, 1.0),
        p("comm", "BROKERAGE % used by the AFL's virtual-trade profit test (the Vault's brokerage)", 0.65, 0.0, 1.0, 0.01),
    ]
}

// ---------------------------------------------------------------- variant 6: running ATH buy after %swing losses, index supertrend (bull) filter, exit in bear

const INDEX_NAMES: [&str; 4] = ["NSENIFTY", "NSENIFTY1", "NIFTY", "NSE500"];

fn bull_inner(s: &Series, idx: Option<&Series>, refc: Option<&Series>, g: G) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l, v) = (s.close(), s.open(), s.high(), s.low(), s.volume());
    let _ = v;
    let exit_pct = g("exitpct") as f32;
    let (losspct, noofloss, comm, penny) = (g("losspct") as f32, g("noofloss") as f32, g("comm") as f32, g("pennyflt") as f32);
    let turnflt = g("turnflt").round() == 1.0;
    let dead_yes = g("deadstock").round() == 0.0;
    let hcl = running_high(&c);
    let sw = swing(&c, exit_pct);
    let athc = athc_state(&c, &hcl, &sw.trend);
    let uc: Vec<bool> = (0..n).map(|i| h[i] == c[i]).collect();
    let lc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let upc: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i])).collect();
    let loc: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i])).collect();
    let turnover: Vec<bool> = (0..n).map(|i| if turnflt { s.bars[i].oi > volsig(s.bars[i].date / 10000) } else { true }).collect();
    let dead_buy = |i: usize| if dead_yes { i != n - 1 } else { i != 0 };
    let dead1 = dead30(s);
    let buysig_v: Vec<bool> = (0..n).map(|i| athc[i] && !upc[i] && dead_buy(i) && c[i] > penny && turnover[i]).collect();
    let cc = concount(&c, &buysig_v, &sw, losspct, comm);
    // index filter: FLTBuy = bull (index trend up), FLTShort = bear; MKTFLT NONE: FLTBuy true, FLTShort false
    let (fltbuy, fltshort) = if g("mktflt").round() == 1.0 {
        match idx {
            Some(ix) => {
                let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
                let f = apx::Flt { tf: g("flttf").round().clamp(0.0, 2.0) as u8, sma: g("iflt").round() == 1.0, point: g("istflt").round().clamp(0.0, 2.0) as u8, pd: g("ifltpd").round().max(0.0) as usize, fact: g("ifltfact") as f32, mapd: g("ifltmapd").round().max(1.0) as usize };
                match refc {
                    Some(rc) if g("pad") != 0.0 => apx::st_filter_padded(ix, rc, &dates, &f),
                    _ => apx::st_filter(&apx::align(ix, &dates), &dates, &f),
                }
            }
            None => (vec![false; n], vec![false; n]),
        }
    } else {
        (vec![true; n], vec![false; n])
    };
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let fresh = i == 0 || c[i] > hcl[i - 1];
        buy[i] = (athc[i] || fresh) && cc[i] >= noofloss && fltbuy[i] && !dead1[i] && !upc[i] && !uc[i] && dead_buy(i) && c[i] > penny && turnover[i];
        sell[i] = ((!athc[i] || sw.trend[i] == -1) && !loc[i] && !lc[i]) || (dead_yes && i == n - 1) || dead1[i] || (fltshort[i] && !loc[i] && !lc[i]);
    }
    let buy = exrem(&buy, &sell);
    let sell = exrem(&sell, &buy);
    let (mut bp, mut sp) = (c.clone(), c.clone());
    for i in 0..n {
        bp[i] = bp[i].min(h[i]).max(l[i]);
        sp[i] = sp[i].min(h[i]).max(l[i]);
    }
    let mut sg = Signals::new(buy, sell, bp, sp);
    let hard = g("hardstop") as f32;
    if hard > 0.0 {
        sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
    }
    sg
}

/// "Pad and align to the reference symbol" cuts a stock's history at the reference's first bar (see ath_plus.rs / combo RESULT.md).
fn bull_symbol(s: &Series, idx: Option<&Series>, refc: Option<&Series>, g: G) -> Signals {
    if let (true, Some(rc)) = (g("pad") != 0.0, refc) {
        let first = rc.bars.first().map_or(0, |b| b.date);
        let cut = s.bars.partition_point(|b| b.date < first);
        if cut > 0 {
            let n = s.bars.len();
            let sub = Series { name: s.name.clone(), bars: s.bars[cut..].to_vec() };
            let mut out = Signals::new(vec![false; n], vec![false; n], s.close(), s.close());
            if sub.bars.is_empty() {
                return out;
            }
            let inner = bull_inner(&sub, idx, refc, g);
            for i in 0..sub.bars.len() {
                out.buy[cut + i] = inner.buy[i];
                out.sell[cut + i] = inner.sell[i];
                out.buy_price[cut + i] = inner.buy_price[i];
                out.sell_price[cut + i] = inner.sell_price[i];
            }
            out.stop = inner.stop;
            return out;
        }
    }
    bull_inner(s, idx, refc, g)
}

fn signals_v6(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    let idx = env.get(INDEX_NAMES[(g("prflt").round().clamp(0.0, 3.0)) as usize]);
    let r = g("padref").round();
    let refc = if r < 0.0 { idx } else { env.get(INDEX_NAMES[(r.min(3.0)) as usize]) };
    series.iter().map(|s| bull_symbol(s, idx, refc, g)).collect()
}

fn schema_v6() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop, not checked against a sheet)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; OpenInt > yearly table)", 0.0, 0.0, 1.0, 1.0),
        p("exitpct", "EXIT % (Optimize 5..25 in the Final-version file)", 10.0, 5.0, 25.0, 1.0),
        p("losspct", "MIN PROFIT %", 2.0, 0.5, 10.0, 0.05),
        p("noofloss", "CONSICUTIVE LOSS", 2.0, 1.0, 50.0, 1.0),
        p("mktflt", "FILTER TYPE (0 NONE, 1 INDEX)", 0.0, 0.0, 1.0, 1.0),
        p("flttf", "FILTER TIMEFRAME (0 MONTHLY, 1 WEEKLY, 2 DAILY)", 0.0, 0.0, 2.0, 1.0),
        p("prflt", "INDEX (0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 0.0, 0.0, 3.0, 1.0),
        p("iflt", "INDEX Filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0),
        p("istflt", "ST CALCULATION POINT (0 MID POINT(H+L/2), 1 CLOSE, 2 PIVOT(H+L+C/3))", 0.0, 0.0, 2.0, 1.0),
        p("ifltpd", "INDEX ST PERIOD", 20.0, 0.0, 20.0, 1.0),
        p("ifltfact", "INDEX ST FACTOR", 1.0, 0.0, 20.0, 1.0),
        p("ifltmapd", "INDEX SMA PERIOD", 200.0, 1.0, 300.0, 1.0),
        p("comm", "BROKERAGE % used by the AFL's virtual-trade profit test (the Vault's brokerage)", 0.65, 0.0, 1.0, 0.01),
        p("pad", "ENGINE OPTION (Analysis setting, not an AFL parameter): 'Pad and align to the reference symbol' (1 yes)", 0.0, 0.0, 1.0, 1.0),
        p("padref", "ENGINE OPTION: reference symbol of Pad and align (-1 = the filter index, 0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 1.0, -1.0, 3.0, 1.0),
    ]
}

pub fn variants() -> Vec<VariantDef> {
    vec![
        VariantDef { id: "ytath_pctswing_exit_after_pctswing_losses", data: Data::Cash { watchlist: "NSE 500 C&P.tls" }, schema: schema_v1, signals: signals_v1 },
        VariantDef { id: "ytath_pctswing_after_pctswing_losses", data: Data::Cash { watchlist: "NSE 500 C&P.tls" }, schema: schema_v2, signals: signals_v2 },
        VariantDef { id: "ytath_and_pctswing_after_consecutive_low_2", data: Data::Cash { watchlist: "NSE 500 C&P.tls" }, schema: schema_v3, signals: signals_v3 },
        VariantDef { id: "ytath_running_pctswing_exit_after_2_5", data: Data::Cash22 { watchlist: "NSE 500 C&P.tls" }, schema: schema_v4, signals: signals_v4 },
        VariantDef { id: "ytath_running_pctswing_exit_after_2_4", data: Data::Cash { watchlist: "NSE 500 C&P.tls" }, schema: schema_v5, signals: signals_v5 },
        VariantDef { id: "ytath_running_pctswing_after_2_pctswing_4", data: Data::Cash { watchlist: "NSE 500 C&P.tls" }, schema: schema_v6, signals: signals_v6 },
        VariantDef { id: "ytath_and_pctswing_with_fixed_atr_scale_in_5", data: Data::Cash { watchlist: "NSE 500 C&P.tls" }, schema: schema_v7, signals: signals_v7 },
    ]
}
