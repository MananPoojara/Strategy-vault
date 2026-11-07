//! ATH engines: ATHC after consecutive losing trades of the Supertrend 5,2 / 5,4 probability family (rank bucket, index filter).
//! One `VariantDef` per sub-variant (see `ath_core::VariantDef`); registered in `ath_registry.rs`. Standard library only: depends on
//! `ath_core` and `ath_ind` so a verification driver can include it with `#[path]`.
//!
//! The four AFLs (folder `ATH - 1 Trade Loss of ST`, plus the older copy in `ATH/`) share one core, coded once here in float32:
//! * `%swing` line (`trend1`), all-time-high close state (`ATHCbuysig`),
//! * the "virtual trade" of Supertrend(bpd2,bfactor2) in / both Supertrends down out, whose profit is read when the close falls
//!   through the PREVIOUS bar's Supertrend line (the AFL overwrites `BPD2`/`BPD3` with the Supertrend up lines through `VarSet`),
//!   and the loss counter `concount`,
//! * the 30-bar flat-price "dead stock" array (it looks 30 bars FORWARD, `Ref(O, k)` with k > 0),
//! * the turnover-rank band (`StaticVarGenerateRanks` of the 60-bar average of Close x Volume over the watchlist).
#![allow(dead_code)]
use crate::ath_core::*;
use crate::ath_ind::{atr as ind_atr, llv as ind_llv, ma as ind_ma, roc as ind_roc, tr as ind_tr};

/// AmiBroker's Null.
const NULLV: f32 = -1.0e10;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    /// `... index flt for bull mkt exit all in bear mkt.AFL` (expression buy/sell, no rank band)
    A1,
    /// `... new turnover_new (Used for Optim).afl`
    A2,
    /// `... new turnover_new index (MAIN AFL)(WITH FILTER).afl`
    A3,
    /// `ATH/ATHC trading ... new turnover_new/... .afl` (older, SL blocks)
    A4,
}

#[derive(Clone, Copy, Debug)]
struct Cfg {
    exit_pct: f32,
    losspct: f32,
    noofloss: f32,
    stcal: u8,
    bpd2: usize,
    bf2: f32,
    bpd3: usize,
    bf3: f32,
    comm: f32,
    dead_yes: bool,
    penny: f32,
    turn: bool,
    old_turn: bool,
}

fn volsig(year: i32, old: bool) -> f32 {
    if old {
        // a1: the shorter table
        return match year {
            y if y < 2000 => 30_000.0,
            2000..=2004 => 50_000.0,
            2005..=2009 => 1_800_000.0,
            2010..=2014 => 2_600_000.0,
            2015..=2019 => 6_000_000.0,
            _ => 13_000_000.0,
        };
    }
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
            // AmiBroker's ATR(1) is Null on the first bar of the array (TR needs Ref(C,-1)), so a window that contains bar 0 is Null:
            // the first line exists from bar `period` (not `period - 1`). It matters for the index series aligned to a young symbol.
            if i + 1 == period {
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

/// Everything per symbol that the four AFLs share.
struct Base {
    c: Vec<f32>,
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    v: Vec<f32>,
    hcl: Vec<f32>,
    trend1: Vec<i8>,
    athc: Vec<bool>,
    concount: Vec<f32>,
    uc: Vec<bool>,
    lc: Vec<bool>,
    upcircuit: Vec<bool>,
    locircuit: Vec<bool>,
    dead30: Vec<bool>,
    turnover: Vec<bool>,
    dbuy: Vec<bool>,
    dsell: Vec<bool>,
}

fn base(s: &Series, cf: &Cfg) -> Base {
    let n = s.bars.len();
    let c = s.close();
    let o = s.open();
    let h = s.high();
    let l = s.low();
    let v = s.volume();
    let mut hcl = vec![0f32; n];
    for i in 0..n {
        hcl[i] = if i == 0 || c[i] > hcl[i - 1] { c[i] } else { hcl[i - 1] };
    }
    // %swing line
    let exit: Vec<f32> = c.iter().map(|&x| (x * cf.exit_pct) / 100.0).collect();
    let mut up: Vec<f32> = (0..n).map(|i| c[i] + exit[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| c[i] - exit[i]).collect();
    let mut trend1 = vec![1i8; n];
    for i in 1..n {
        let t = if c[i] > up[i - 1] {
            1
        } else if c[i] < dn[i - 1] {
            -1
        } else {
            trend1[i - 1]
        };
        trend1[i] = t;
        let flag1 = t < 0 && trend1[i - 1] > 0;
        let flagh1 = t > 0 && trend1[i - 1] < 0;
        if t > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if t < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if flag1 {
            up[i] = c[i] + exit[i];
        }
        if flagh1 {
            dn[i] = c[i] - exit[i];
        }
    }
    // ATHCbuysig
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
    // ST(bpd2,bfactor2) / ST(bpd3,bfactor3) on the chosen price; BPD2 / BPD3 become the up lines (VarSet)
    let sc = stcal_of(cf.stcal, &h, &l, &c);
    let (up2, _d2, bt2) = supertrend(&sc, &h, &l, &c, cf.bpd2, cf.bf2);
    let (up3, _d3, bt3) = supertrend(&sc, &h, &l, &c, cf.bpd3, cf.bf3);
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
    let cr2 = cross_line_below_close(&up2, &c);
    let cr3 = cross_line_below_close(&up3, &c);
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
                let sp = sprice[i];
                let bp = bprice[i];
                profit = (sp - (sp * cf.comm) / 100.0) - (bp + (bp * cf.comm) / 100.0);
                pct = ((sp - (sp * cf.comm) / 100.0) - (bp + (bp * cf.comm) / 100.0)) / bp * 100.0;
            }
        }
        concount[i] = if profit != 0.0 {
            if pct <= cf.losspct {
                concount[i - 1] + 1.0
            } else {
                0.0
            }
        } else {
            concount[i - 1]
        };
    }
    // circuits
    let uc: Vec<bool> = (0..n).map(|i| h[i] == c[i]).collect();
    let lc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let upcircuit: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i])).collect();
    let locircuit: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i])).collect();
    // dead stock: this bar and the next 30 bars have identical O, H, L, C
    let mut run = vec![0u32; n + 1];
    for i in (0..n.saturating_sub(1)).rev() {
        let same = o[i] == o[i + 1] && h[i] == h[i + 1] && l[i] == l[i + 1] && c[i] == c[i + 1];
        run[i] = if same { run[i + 1] + 1 } else { 0 };
    }
    let dead30: Vec<bool> = (0..n).map(|i| run[i] >= 30).collect();
    let turnover: Vec<bool> = (0..n).map(|i| if cf.turn { v[i] * c[i] > volsig(s.bars[i].date / 10000, cf.old_turn) } else { true }).collect();
    let dbuy: Vec<bool> = (0..n).map(|i| if cf.dead_yes { i != n - 1 } else { i != 0 }).collect();
    let dsell: Vec<bool> = (0..n).map(|i| cf.dead_yes && i == n - 1).collect();
    Base { c, o, h, l, v, hcl, trend1, athc, concount, uc, lc, upcircuit, locircuit, dead30, turnover, dbuy, dsell }
}

// ---------------------------------------------------------------- index filter

/// The index regime expanded back to the index's daily bars (`TimeFrameExpand`).
struct IdxFlags {
    dates: Vec<i32>,
    st: Vec<i8>,
    ic: Vec<f32>,
    mprev: Vec<f32>,
}

fn days_from_civil(date: i32) -> i64 {
    let (y, m, d) = ((date / 10000) as i64, (date / 100 % 100) as i64, (date % 100) as i64);
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// period key of a daily bar: tf 0 = monthly, 1 = weekly (Monday based), 2 = daily
fn period_key(date: i32, tf: u8) -> i64 {
    match tf {
        0 => (date / 100) as i64,
        1 => {
            let d = days_from_civil(date);
            // 1970-01-01 was a Thursday: (d + 3) % 7 = 0 for Monday
            d - (d + 3).rem_euclid(7)
        }
        _ => date as i64,
    }
}

struct IdxCfg {
    tf: u8,
    use_st: bool,
    stcal: u8,
    pd: usize,
    factor: f32,
    mapd: usize,
    /// 0 = AmiBroker's default `expandFirst` (the period's final value is visible from its first bar), 1 = `expandLast`
    expand_last: bool,
}

fn index_flags(idx: &Series, ic: &IdxCfg) -> IdxFlags {
    let n = idx.bars.len();
    // compress
    let mut map = vec![0usize; n];
    let (mut co, mut ch, mut cl, mut cc): (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) = (vec![], vec![], vec![], vec![]);
    let mut last_of: Vec<usize> = vec![];
    let mut prev_key = i64::MIN;
    for j in 0..n {
        let k = period_key(idx.bars[j].date, ic.tf);
        let b = &idx.bars[j];
        if k != prev_key {
            co.push(b.o);
            ch.push(b.h);
            cl.push(b.l);
            cc.push(b.c);
            last_of.push(j);
            prev_key = k;
        } else {
            let m = co.len() - 1;
            if b.h > ch[m] {
                ch[m] = b.h;
            }
            if b.l < cl[m] {
                cl[m] = b.l;
            }
            cc[m] = b.c;
            last_of[m] = j;
        }
        map[j] = co.len() - 1;
    }
    let sc = stcal_of(ic.stcal, &ch, &cl, &cc);
    let (_u, _d, tr) = supertrend(&sc, &ch, &cl, &cc, ic.pd, ic.factor);
    let ma = ind_ma(&cc, ic.mapd);
    let mprev: Vec<f32> = (0..cc.len()).map(|k| if k == 0 { f32::NAN } else { ma[k - 1] }).collect();
    let pick = |j: usize| -> usize {
        if !ic.expand_last {
            map[j]
        } else if last_of[map[j]] == j {
            map[j]
        } else {
            usize::MAX.wrapping_add(map[j]) // map[j] - 1, wraps to MAX for 0
        }
    };
    let mut st = vec![0i8; n];
    let mut icv = vec![f32::NAN; n];
    let mut mp = vec![f32::NAN; n];
    for j in 0..n {
        let k = pick(j);
        if k < cc.len() {
            st[j] = tr[k];
            icv[j] = cc[k];
            mp[j] = mprev[k];
        }
    }
    let _ = ic.use_st;
    IdxFlags { dates: idx.bars.iter().map(|b| b.date).collect(), st, ic: icv, mprev: mp }
}

/// `SetForeign(index)` in AmiBroker aligns the index to the current symbol's bars (fixup): index bars on dates the symbol has no bar
/// are dropped, dates the symbol has but the index lacks are filled with the previous index close, and nothing before the symbol's first bar
/// exists. The monthly Supertrend of the AFL therefore starts at the symbol's own first bar (its first month is a partial bar).
fn align_index(idx: &Series, s: &Series) -> Series {
    let mut bars: Vec<Bar> = Vec::with_capacity(s.bars.len());
    let mut j = 0usize;
    let mut last: Option<Bar> = None;
    for sb in &s.bars {
        while j < idx.bars.len() && idx.bars[j].date <= sb.date {
            last = Some(idx.bars[j]);
            j += 1;
        }
        if let Some(mut b) = last {
            if b.date != sb.date {
                b.o = b.c;
                b.h = b.c;
                b.l = b.c;
                b.date = sb.date;
            }
            bars.push(b);
        }
    }
    Series { name: idx.name.clone(), bars }
}

/// FLTBuy / FLTShort of the AFL on the bars of a symbol. `bull_swap`: a3's "Bull" scene swaps the Supertrend flags.
fn flags_for(f: &IdxFlags, s: &Series, use_st: bool, bull_swap: bool) -> (Vec<bool>, Vec<bool>) {
    let n = s.bars.len();
    let mut fb = vec![false; n];
    let mut fs = vec![false; n];
    for i in 0..n {
        let Some(j) = f.dates.partition_point(|&d| d <= s.bars[i].date).checked_sub(1) else { continue };
        if use_st {
            let up = f.st[j] == 1;
            let dn = f.st[j] == -1;
            if bull_swap {
                fb[i] = dn;
                fs[i] = up;
            } else {
                fb[i] = up;
                fs[i] = dn;
            }
        } else {
            fb[i] = f.ic[j] > f.mprev[j];
            fs[i] = f.ic[j] < f.mprev[j];
        }
    }
    (fb, fs)
}

// ---------------------------------------------------------------- ranking (turnover bucket, position score)

struct RankData {
    rank: Vec<Vec<f32>>,
}

fn turnover_ranks(series: &[Series], pd: usize) -> RankData {
    // value per symbol and bar: MA(C*V, pd), or 0 when the symbol is flat for the next 30 bars; Null (excluded) before pd bars
    let mut ent: Vec<(i32, f32, u32, u32)> = Vec::new();
    for (k, s) in series.iter().enumerate() {
        let n = s.bars.len();
        let to: Vec<f32> = s.bars.iter().map(|b| b.c * b.v).collect();
        let tma = ind_ma(&to, pd);
        let mut run = vec![0u32; n + 1];
        for i in (0..n.saturating_sub(1)).rev() {
            let (a, b) = (&s.bars[i], &s.bars[i + 1]);
            run[i] = if a.o == b.o && a.h == b.h && a.l == b.l && a.c == b.c { run[i + 1] + 1 } else { 0 };
        }
        for i in 0..n {
            if tma[i].is_nan() {
                continue;
            }
            let val = if run[i] >= 30 { 0.0 } else { tma[i] };
            ent.push((s.bars[i].date, val, k as u32, i as u32));
        }
    }
    // per date: highest value = rank 1 (normal mode), ties in the order of the symbols (ordinal)
    ent.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)).then(a.2.cmp(&b.2)));
    let mut rank: Vec<Vec<f32>> = series.iter().map(|s| vec![f32::NAN; s.bars.len()]).collect();
    let mut r = 0f32;
    let mut cur = i32::MIN;
    for e in &ent {
        if e.0 != cur {
            cur = e.0;
            r = 0.0;
        }
        r += 1.0;
        rank[e.2 as usize][e.3 as usize] = r;
    }
    RankData { rank }
}

/// RSI(n) in the Wilder form (unverified against AmiBroker's).
fn rsi(c: &[f32], n: usize) -> Vec<f32> {
    let len = c.len();
    let mut out = vec![f32::NAN; len];
    if n == 0 || len <= n {
        return out;
    }
    let (mut ag, mut al) = (0f32, 0f32);
    for i in 1..=n {
        let d = c[i] - c[i - 1];
        if d > 0.0 { ag += d } else { al -= d }
    }
    ag /= n as f32;
    al /= n as f32;
    out[n] = if al == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + ag / al) };
    for i in n + 1..len {
        let d = c[i] - c[i - 1];
        let (g, l) = if d > 0.0 { (d, 0.0) } else { (0.0, -d) };
        ag = (ag * (n as f32 - 1.0) + g) / n as f32;
        al = (al * (n as f32 - 1.0) + l) / n as f32;
        out[i] = if al == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + ag / al) };
    }
    out
}

fn stdev(c: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![f32::NAN; c.len()];
    for i in 0..c.len() {
        if n >= 2 && i + 1 >= n {
            let w = &c[i + 1 - n..=i];
            let m: f64 = w.iter().map(|&x| x as f64).sum::<f64>() / n as f64;
            let v: f64 = w.iter().map(|&x| (x as f64 - m).powi(2)).sum::<f64>() / n as f64;
            out[i] = v.sqrt() as f32;
        }
    }
    out
}

/// PositionScore of the rank-bucket AFLs: the 60-bar turnover average unless RANKMODE changes it (modes other than NONE unverified).
fn position_score(b: &Base, turnoverpd: usize, mode: u32, rocpd: usize, rsipd: usize, stdpd: usize, swingpd: usize) -> Vec<f32> {
    let n = b.c.len();
    let to: Vec<f32> = (0..n).map(|i| b.c[i] * b.v[i]).collect();
    let turn = ind_ma(&to, turnoverpd);
    if mode == 0 {
        return turn;
    }
    let roc = ind_roc(&b.c, rocpd);
    let sd = stdev(&b.c, stdpd);
    let rocstd: Vec<f32> = (0..n).map(|i| roc[i] / sd[i]).collect();
    let swing: Vec<f32> = {
        let hh = crate::ath_ind::hhv(&b.c, swingpd);
        (0..n).map(|i| 100.0 - (hh[i] - b.c[i]) / hh[i] * 100.0).collect()
    };
    // POCL: the AFL's overlapping count loop, copied literally
    let mut pocl = vec![0f32; n];
    if n > rocpd + 1 {
        let barc = n - (rocpd + 1);
        for m in (1..=barc).rev() {
            for j in 1..rocpd + 1 {
                if m + j >= n {
                    break;
                }
                if b.c[m + j] > b.c[m + j - 1] {
                    pocl[m + j] = pocl[m + j - 1] + 1.0;
                } else {
                    pocl[m + j] = pocl[m + j - 1];
                }
            }
        }
    }
    match mode {
        1 => roc.iter().map(|&x| 100.0 + x).collect(),
        2 => roc.iter().map(|&x| 100.0 - x).collect(),
        3 => rsi(&b.c, rsipd),
        4 => sd,
        5 => rocstd.iter().map(|&x| 100.0 + x).collect(),
        6 => rocstd.iter().map(|&x| 100.0 - x).collect(),
        7 => pocl,
        8 => swing,
        9 => (0..n).map(|i| (100.0 + roc[i]) + pocl[i]).collect(),
        10 => (0..n).map(|i| swing[i] + pocl[i]).collect(),
        _ => turn,
    }
}

// ---------------------------------------------------------------- the four variants

#[derive(Clone, Copy)]
struct LoopCfg {
    kind: Kind,
    ver: u32,
    hardslpct: f32,
    atr_prd: usize,
    atr_sl_m: f32,
    days_sl: usize,
    rank_lo: f32,
    rank_hi: f32,
    index: bool,
    /// engine option: the entry also needs the ATH state `ATHCbuysig` of the PREVIOUS bar (buy the second new high of a run)
    athc_prev: bool,
    /// engine option: every bar that meets the entry test is a Buy signal even while the AFL's own Position flag is set (the backtester ignores it while that symbol is held and takes it when a slot is free)
    repeat: bool,
}

/// The per-bar loop of a2 / a3 / a4: one position state per symbol, entry on a fresh ATH close after the loss streak.
fn loop_signals(s: &Series, b: &Base, cf: &Cfg, lp: &LoopCfg, rank: &[f32], flags: Option<(&[bool], &[bool])>) -> (Vec<bool>, Vec<bool>) {
    let n = s.bars.len();
    let mut buy = vec![false; n];
    let mut sell = vec![false; n];
    let a4 = lp.kind == Kind::A4;
    let sl1: Vec<f32> = if lp.ver == 3 && lp.days_sl > 1 { ind_llv(&b.c, lp.days_sl) } else { vec![NULLV; n] };
    let myatr = ind_atr(&b.h, &b.l, &b.c, lp.atr_prd);
    // a2 / a3 test the toggle `deadstock` where the AFL means the 30-bar array: scalar 1 (NO) -> NOT Deadstock = false / Deadstock = true
    let dead_toggle_no = !cf.dead_yes;
    let (mut position, mut skip) = (false, false);
    let (mut hsl, mut atr_sl, mut slv) = (0f32, 0f32, 0f32);
    let start0 = s.bars.iter().position(|x| x.date >= 20000901).unwrap_or(n).max(1);
    for i in start0..n {
        let cash_filter = !b.uc[i] && b.turnover[i] && b.c[i] > cf.penny && b.dbuy[i];
        let in_band = rank[i] > lp.rank_lo && rank[i] < lp.rank_hi;
        let flt_ok = match flags {
            Some((_fb, fs)) => fs[i],
            None => true,
        };
        let entry_open = cash_filter
            && flt_ok
            && b.concount[i] >= cf.noofloss
            && b.c[i] > b.hcl[i - 1]
            && (!lp.athc_prev || b.athc[i - 1])
            && in_band
            && (a4 || !dead_toggle_no);
        let entry = entry_open && !position;
        if lp.repeat && entry_open && position {
            buy[i] = true;
        }
        if entry {
            buy[i] = true;
            if lp.ver == 1 && lp.hardslpct > 0.0 {
                hsl = (1.0 - lp.hardslpct * 0.01) * b.c[i];
            }
            if lp.ver == 2 && myatr[i] > 0.0 {
                atr_sl = b.c[i] - myatr[i] * lp.atr_sl_m;
            }
            if lp.ver == 3 {
                if a4 {
                    if sl1[i - 1] > 0.0 {
                        slv = sl1[i];
                    }
                } else {
                    slv = sl1[i];
                }
            }
            position = true;
        } else if position && !skip {
            let stop = if a4 {
                match lp.ver {
                    1 => b.c[i] <= hsl,
                    2 => b.c[i] <= atr_sl,
                    3 => b.c[i] < slv,
                    _ => false,
                }
            } else {
                b.c[i] <= hsl || b.c[i] <= atr_sl || b.c[i] <= slv
            };
            let fltb = if lp.index { flags.map(|f| f.0[i]).unwrap_or(false) } else { false };
            let dead_x = !a4 && dead_toggle_no;
            let cond = stop || b.dsell[i] || b.trend1[i] == -1 || fltb || dead_x;
            if cond {
                let lc_ok = if a4 && lp.ver != 0 { !b.locircuit[i] } else { !b.lc[i] };
                if lc_ok || b.dsell[i] || dead_x {
                    sell[i] = true;
                    position = false;
                    if a4 {
                        hsl = 0.0;
                        atr_sl = 0.0;
                        slv = 0.0;
                    }
                } else {
                    skip = true;
                }
            }
        } else if skip {
            let lc_ok = if a4 { !b.locircuit[i] } else { !b.lc[i] };
            if lc_ok || b.dsell[i] || (!a4 && dead_toggle_no) {
                sell[i] = true;
                position = false;
                skip = false;
                if a4 {
                    hsl = 0.0;
                    atr_sl = 0.0;
                    slv = 0.0;
                }
            }
        }
    }
    (buy, sell)
}

fn cfg_of(g: G, kind: Kind) -> Cfg {
    Cfg {
        exit_pct: g("exitpct") as f32,
        losspct: g("losspct") as f32,
        noofloss: g("noofloss") as f32,
        stcal: g("stflt1").round() as u8,
        bpd2: g("bpd2").round() as usize,
        bf2: g("bfactor2") as f32,
        bpd3: g("bpd3").round() as usize,
        bf3: g("bfactor3") as f32,
        comm: g("brokerage") as f32,
        dead_yes: g("deadstock").round() == 0.0,
        penny: g("pennyflt") as f32,
        turn: g("turnflt").round() == 1.0,
        old_turn: kind == Kind::A1,
    }
}

fn idx_cfg(g: G) -> IdxCfg {
    IdxCfg {
        tf: g("flttf").round() as u8,
        use_st: g("iflt").round() == 0.0,
        stcal: g("istflt").round() as u8,
        pd: g("ifltpd").round() as usize,
        factor: g("ifltfact") as f32,
        mapd: g("ifltmapd").round() as usize,
        expand_last: g("idxexpand").round() == 1.0,
    }
}

fn index_name(g: G) -> &'static str {
    match g("prflt").round() as i32 {
        1 => "NSENIFTY1",
        2 => "NIFTY",
        3 => "NSE500",
        _ => "NSENIFTY",
    }
}

fn all_signals(kind: Kind, series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    // engine option `histfrom` (not an AFL parameter): when set, history before that date is ignored for every array, as if the
    // Analysis had loaded only the range's bars. Default 0 = all bars (SetBarsRequired(100000,0) in a2 / a3 / a4).
    let histfrom = g("histfrom").round() as i32;
    if histfrom > 0 {
        let cut: Vec<Series> = series
            .iter()
            .map(|s| Series { name: s.name.clone(), bars: s.bars.iter().filter(|b| b.date >= histfrom).copied().collect() })
            .collect();
        let sg = all_signals_inner(kind, &cut, env, g);
        // map back to the full series (signals before histfrom are false)
        return series
            .iter()
            .zip(cut.iter().zip(sg))
            .map(|(full, (c, s))| {
                let off = full.bars.len() - c.bars.len();
                let n = full.bars.len();
                let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
                let (mut bp, mut sp) = (vec![0f32; n], vec![0f32; n]);
                let mut score = s.score.as_ref().map(|_| vec![f32::NAN; n]);
                for i in 0..c.bars.len() {
                    buy[off + i] = s.buy[i];
                    sell[off + i] = s.sell[i];
                    bp[off + i] = s.buy_price[i];
                    sp[off + i] = s.sell_price[i];
                    if let (Some(d), Some(src)) = (score.as_mut(), s.score.as_ref()) {
                        d[off + i] = src[i];
                    }
                }
                let mut o = Signals::new(buy, sell, bp, sp);
                o.score = score;
                o.stop = s.stop;
                o
            })
            .collect();
    }
    all_signals_inner(kind, series, env, g)
}

fn all_signals_inner(kind: Kind, series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    let cf = cfg_of(g, kind);
    let use_index = g("mktflt").round() == 1.0 && kind != Kind::A4 && kind != Kind::A2;
    let ic = idx_cfg(g);
    let idx_series = if use_index { env.get(index_name(g)) } else { None };
    let bull_swap = kind == Kind::A3 && g("mktscene").round() == 0.0;
    // rank band (a2 / a3 / a4)
    let ranks = if kind != Kind::A1 { Some(turnover_ranks(series, g("turnoverpd").round() as usize)) } else { None };
    let (lo, hi) = match g("bkmkt").round() as i32 {
        1 => (100.0, 251.0),
        2 => (250.0, 801.0),
        3 => (800.0, 80000.0),
        _ => (0.0, 101.0),
    };
    let lp = LoopCfg {
        kind,
        ver: g("ver").round() as u32,
        hardslpct: g("hardslpct") as f32,
        atr_prd: g("atr_prd").round() as usize,
        atr_sl_m: g("atr_sl_m") as f32,
        days_sl: g("days_sl").round() as usize,
        rank_lo: lo,
        rank_hi: hi,
        index: use_index,
        athc_prev: g("athcprev").round() == 1.0,
        repeat: g("repeatbuy").round() == 1.0,
    };
    let hard = g("hardstop") as f32;
    series
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let b = base(s, &cf);
            let n = s.bars.len();
            // SetForeign(index) aligns the index to this symbol's bars, so the monthly index Supertrend is computed per symbol
            let flags = idx_series.map(|x| flags_for(&index_flags(&align_index(x, s), &ic), s, ic.use_st, bull_swap));
            let mut sg;
            if kind == Kind::A1 {
                let (fb, fs) = match &flags {
                    Some((a, c)) => (a.clone(), c.clone()),
                    None => (vec![true; n], vec![false; n]),
                };
                let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
                for i in 0..n {
                    buy[i] = b.athc[i]
                        && i >= 1
                        && b.c[i] > b.hcl[i - 1]
                        && b.concount[i] >= cf.noofloss
                        && fb[i]
                        && !b.upcircuit[i]
                        && b.dbuy[i]
                        && b.c[i] > cf.penny
                        && b.turnover[i]
                        && !b.dead30[i];
                    sell[i] = b.trend1[i] == -1 || b.dsell[i] || b.dead30[i] || fs[i];
                }
                let buy = exrem(&buy, &sell);
                let sell = exrem(&sell, &buy);
                sg = Signals::new(buy, sell, b.c.clone(), b.c.clone());
            } else {
                let rank = &ranks.as_ref().unwrap().rank[k];
                let fl = flags.as_ref().map(|(a, c)| (a.as_slice(), c.as_slice()));
                let (buy, sell) = loop_signals(s, &b, &cf, &lp, rank, fl);
                sg = Signals::new(buy, sell, b.c.clone(), b.c.clone());
                let mode = g("rankmode").round() as u32;
                sg.score = Some(position_score(&b, g("turnoverpd").round() as usize, mode, g("rocpd").round() as usize, g("rsipd").round() as usize, g("stdpd").round() as usize, g("swingpd").round() as usize));
            }
            if hard > 0.0 {
                // ApplyStop(stopTypeLoss, stopModePercent, hardstop) with ActivateStopsImmediately False: not checked against a team sheet
                sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
            }
            sg
        })
        .collect()
}

fn sig_a1(s: &[Series], e: &Env, g: G) -> Vec<Signals> {
    all_signals(Kind::A1, s, e, g)
}
fn sig_a2(s: &[Series], e: &Env, g: G) -> Vec<Signals> {
    all_signals(Kind::A2, s, e, g)
}
fn sig_a3(s: &[Series], e: &Env, g: G) -> Vec<Signals> {
    all_signals(Kind::A3, s, e, g)
}
fn sig_a4(s: &[Series], e: &Env, g: G) -> Vec<Signals> {
    all_signals(Kind::A4, s, e, g)
}

// ---------------------------------------------------------------- schemas (keys of afl-params-ath.ts; toggles / lists are 0-based option indexes)

fn backtest_common(dead: f64) -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", dead, 0.0, 1.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
        p("histfrom", "(engine option, not an AFL parameter) ignore bars before this date (yyyymmdd, 0 = all)", 0.0, 0.0, 99999999.0, 1.0),
        p("athcprev", "(engine option, not an AFL parameter) 1 = also require the previous bar's ATHCbuysig state, i.e. enter on the second new all-time-high close of a run (what the f5173b sheets show)", 0.0, 0.0, 1.0, 1.0),
        p("repeatbuy", "(engine option, not an AFL parameter) 1 = every bar that passes the entry test is a Buy signal, also while the symbol is already in the AFL's position", 0.0, 0.0, 1.0, 1.0),
        p("brokerage", "BROKERAGE % (AFL COMM: used in the loss-streak profit, the portfolio commission is the runner's)", 0.65, 0.01, 1.0, 0.01),
    ]
}

fn st_params() -> Vec<P> {
    vec![
        p("stflt1", "ST CALCULATION POINT FOR PROBABILITY (0 MID POINT(H+L/2), 1 CLOSE, 2 PIVOT(H+L+C/3))", 0.0, 0.0, 2.0, 1.0),
        p("bpd2", "ST ENTRY Period FOR PROBABILITY", 5.0, 1.0, 20.0, 1.0),
        p("bfactor2", "ST ENTRY Factor FOR PROBABILITY", 2.0, 1.0, 6.0, 0.5),
        p("bpd3", "ST EXIT Period FOR PROBABILITY", 5.0, 1.0, 20.0, 1.0),
        p("bfactor3", "ST EXIT Factor FOR PROBABILITY", 4.0, 1.0, 6.0, 0.5),
    ]
}

fn index_params(with_scene: bool) -> Vec<P> {
    let mut v = vec![
        p("mktflt", "FILTER TYPE (0 NONE, 1 INDEX)", 0.0, 0.0, 1.0, 1.0),
        p("flttf", "FILTER TIMEFRAME (0 MONTHLY, 1 WEEKLY, 2 DAILY)", 0.0, 0.0, 2.0, 1.0),
        p("prflt", "INDEX (0 NSENIFTY, 1 NSENIFTY1, 2 NIFTY, 3 NSE500)", 0.0, 0.0, 3.0, 1.0),
    ];
    if with_scene {
        v.push(p("mktscene", "Market Condition (0 Bull, 1 Bear)", 0.0, 0.0, 1.0, 1.0));
    }
    v.extend([
        p("iflt", "INDEX Filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0),
        p("istflt", "ST CALCULATION POINT (0 MID POINT(H+L/2), 1 CLOSE, 2 PIVOT(H+L+C/3))", 0.0, 0.0, 2.0, 1.0),
        p("ifltpd", "INDEX ST PERIOD", 20.0, 0.0, 20.0, 1.0),
        p("ifltfact", "INDEX ST FACTOR", 1.0, 0.0, 20.0, 1.0),
        p("ifltmapd", "INDEX SMA PERIOD", 200.0, 1.0, 300.0, 1.0),
        p("idxexpand", "(engine option, not an AFL parameter) TimeFrameExpand mode: 1 expandLast = what the team's Penny sheet shows (default here), 0 expandFirst = AmiBroker's documented default (period value visible from its first bar, does not match the sheet)", 1.0, 0.0, 1.0, 1.0),
    ]);
    v
}

fn rank_params() -> Vec<P> {
    vec![
        p("rankmode", "RANKING MODE (0 NONE ... 10, order of the AFL list)", 0.0, 0.0, 10.0, 1.0),
        p("rocpd", "ROC PERIOD for RANKING/NO OF POSITIVE CLOSE", 10.0, 1.0, 400.0, 1.0),
        p("rsipd", "RSI PERIOD for RANKING", 14.0, 1.0, 1000.0, 1.0),
        p("stdpd", "STD PERIOD for RANKING", 14.0, 1.0, 1000.0, 1.0),
        p("swingpd", "SWING PERIOD", 180.0, 10.0, 1000.0, 1.0),
        p("turnoverpd", "TURNOVER AVG PERIOD", 60.0, 1.0, 200.0, 1.0),
        p("bkmkt", "BACKTEST MARKET AS PER VOLUME (0 LARGECAP rank 1-100, 1 MIDCAP 101-250, 2 SMALLCAP 251-800, 3 MICROCAP 801+)", 0.0, 0.0, 3.0, 1.0),
    ]
}

fn sl_params() -> Vec<P> {
    vec![
        p("ver", "SL EXIT (0 OFF, 1 % Based HARD SL, 2 ATR Based HARD SL, 3 lowest close of DAYS SL)", 0.0, 0.0, 3.0, 1.0),
        p("hardslpct", "HARD SL%", 2.0, 0.0, 12.0, 1.0),
        p("atr_prd", "ATR Prds", 5.0, 0.0, 100.0, 1.0),
        p("atr_sl_m", "ATR Hard SL Multiplier", 3.0, 0.0, 6.0, 1.0),
        p("days_sl", "DAYS SL", 1.0, 1.0, 100.0, 1.0),
    ]
}

fn schema_a1() -> Vec<P> {
    let mut v = backtest_common(1.0);
    v.push(p("hardstop", "Hard Stoploss % (0 = off; ApplyStop, not checked against a team sheet)", 0.0, 0.0, 50.0, 1.0));
    v.extend(st_params());
    v.extend(index_params(false));
    v.extend([
        p("exitpct", "EXIT %", 10.0, 1.0, 25.0, 1.0),
        p("losspct", "MIN PROFIT %", 2.0, 0.5, 10.0, 0.05),
        p("noofloss", "CONSICUTIVE LOSS", 2.0, 1.0, 50.0, 1.0),
    ]);
    v
}

fn schema_a2() -> Vec<P> {
    // DEFAULT SET TO YES ON REQUEST (the AFL's own default is NO): a2 / a3 read the toggle where the AFL means a 30-bar array, so with NO this port never enters (0 trades)
    let mut v = backtest_common(0.0);
    v.extend([
        // Optimize() in the AFL: its default is what a plain backtest uses
        p("exitpct", "EXIT % (Optimize in the AFL, default 10)", 10.0, 5.0, 22.0, 1.0),
        p("losspct", "MIN PROFIT %", 2.0, 0.0, 10.0, 0.5),
        p("noofloss", "CONSICUTIVE LOSS", 1.0, 0.0, 15.0, 1.0),
    ]);
    v.extend(sl_params());
    v.extend(st_params());
    v.extend(rank_params());
    v
}

fn schema_a3() -> Vec<P> {
    // DEFAULT SET TO YES ON REQUEST (the AFL's own default is NO): a2 / a3 read the toggle where the AFL means a 30-bar array, so with NO this port never enters (0 trades)
    let mut v = backtest_common(0.0);
    v.extend([
        p("exitpct", "EXIT %", 10.0, 1.0, 22.0, 1.0),
        p("losspct", "MIN LOSS %", 2.0, 0.0, 16.0, 0.5),
        p("noofloss", "CONSICUTIVE LOSS", 2.0, 0.0, 5.0, 1.0),
    ]);
    v.extend(sl_params());
    v.extend(st_params());
    v.extend(index_params(true));
    v.extend(rank_params());
    v
}

fn schema_a4() -> Vec<P> {
    let mut v = backtest_common(1.0);
    v.extend(sl_params());
    v.extend(st_params());
    v.extend([
        p("exitpct", "EXIT % (the AFL default 10 is below its range 11..22: AmiBroker clamps it to 11)", 11.0, 11.0, 22.0, 1.0),
        p("losspct", "MIN PROFIT %", 2.0, 0.5, 10.0, 0.5),
        p("noofloss", "CONSICUTIVE LOSS", 2.0, 0.0, 5.0, 1.0),
    ]);
    v.extend(rank_params());
    v
}

pub fn variants() -> Vec<VariantDef> {
    // the Jun-Jul 2022 sheets were made on the 31-03-2022 database; the RELIANCE single-stock sheets (variant 4, and Sheet1 of 2 / 3)
    // need RELIANCE, which the converted 2022 database does not contain, so variant 4 stays on the 2021 copy.
    let cash = Data::Cash { watchlist: "NSE 500 C&P.tls" };
    let cash22 = Data::Cash22 { watchlist: "NSE 500 C&P.tls" };
    vec![
        VariantDef { id: "ytath_pctswing_exit_after_2_st_5_2_5_4_loss_3", data: cash22, schema: schema_a1, signals: sig_a1 },
        VariantDef { id: "ytath_pctswing_exit_after_st_probability", data: cash22, schema: schema_a2, signals: sig_a2 },
        VariantDef { id: "ytath_pctswing_exit_after_st_probability_2", data: cash22, schema: schema_a3, signals: sig_a3 },
        VariantDef { id: "ytath_pctswing_exit_after_st_probability_3", data: cash, schema: schema_a4, signals: sig_a4 },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(rows: &[(f32, f32, f32, f32)]) -> Series {
        let bars = rows
            .iter()
            .enumerate()
            .map(|(i, &(o, h, l, c))| Bar { date: 20200101 + i as i32, c, o, h, l, v: 1000.0, oi: 1.0e7, aux1: 1.0, aux2: 0.0 })
            .collect();
        Series { name: "T".into(), bars }
    }

    #[test]
    fn supertrend_has_no_line_before_the_atr_window_is_full() {
        let h = [11.0f32, 12.0, 13.0, 14.0, 15.0, 16.0];
        let l = [9.0f32, 10.0, 11.0, 12.0, 13.0, 14.0];
        let c = [10.0f32, 11.0, 12.0, 13.0, 14.0, 15.0];
        let sc: Vec<f32> = (0..6).map(|i| (h[i] + l[i]) / 2.0).collect();
        let (up, dn, tr) = supertrend(&sc, &h, &l, &c, 5, 2.0);
        assert!(up[..5].iter().all(|&x| x == NULLV), "satr is Null for the first `period` bars (ATR(1) of bar 0 is Null, so the window of bar period-1 is Null too)");
        assert!(up[5] > 0.0 && dn[5] == NULLV && tr[5] == 1);
    }

    #[test]
    fn a_flat_run_of_31_identical_bars_is_a_dead_stock_and_the_last_30_bars_are_not() {
        let rows: Vec<(f32, f32, f32, f32)> = (0..40).map(|_| (10.0, 10.0, 10.0, 10.0)).collect();
        let s = series(&rows);
        let cf = Cfg { exit_pct: 10.0, losspct: 2.0, noofloss: 0.0, stcal: 1, bpd2: 5, bf2: 2.0, bpd3: 5, bf3: 4.0, comm: 0.65, dead_yes: true, penny: 0.0, turn: false, old_turn: false };
        let b = base(&s, &cf);
        assert!(b.dead30[0] && b.dead30[9]);
        assert!(!b.dead30[10], "bar 10 has only 29 identical bars after it");
    }

    #[test]
    fn the_second_new_high_rule_skips_the_first_new_high_of_a_run() {
        let mut rows = vec![(10.0f32, 10.5f32, 9.5f32, 10.0f32); 3];
        for k in 1..=4 {
            let c = 10.0 + k as f32;
            rows.push((c, c + 0.5, c - 0.5, c));
        }
        let s = series(&rows);
        let cf = Cfg { exit_pct: 10.0, losspct: 2.0, noofloss: 0.0, stcal: 1, bpd2: 5, bf2: 2.0, bpd3: 5, bf3: 4.0, comm: 0.65, dead_yes: false, penny: 0.0, turn: false, old_turn: false };
        let b = base(&s, &cf);
        let mk = |athc_prev| LoopCfg { kind: Kind::A4, ver: 0, hardslpct: 2.0, atr_prd: 5, atr_sl_m: 3.0, days_sl: 1, rank_lo: 0.0, rank_hi: 101.0, index: false, athc_prev, repeat: false };
        let rank = vec![1.0f32; s.bars.len()];
        let (b0, _) = loop_signals(&s, &b, &cf, &mk(false), &rank, None);
        let (b1, _) = loop_signals(&s, &b, &cf, &mk(true), &rank, None);
        assert!(b0[3], "first new high buys");
        assert!(!b1[3] && b1[4], "with athcprev the buy comes one new high later");
    }
}
