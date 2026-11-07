//! ATH sub-variants of batch c1 ("Ranking & stock selection", 21 AFLs): ROC / RSI / StDev / ATR% / positive-close ranking, average-turnover rank
//! bands (Large / Mid / Small / Micro), market-cap class by turnover rank with a monthly NSEMIDCAP / NSESMLCAP100 / NSE100 EMA price-relative
//! switch, ROC(30) + ROC(365) subset ranking, the ATR scale-in with the fine turnover table, double-turnover entry, the NSE500 / F&O list
//! membership AFLs and the ROC(30) top-ranking entry.
//!
//! One `VariantDef` per sub-variant, registered through `ath_registry.rs`. Standard library only (plus `ath_core` and, for the scale-in file,
//! `ath_loss2::scalein_signals`). Every rule is coded from the local AFL copy (`ATH/afl_copy/<id>.afl`) in float32 like AmiBroker, AFL mistakes
//! copied as written (decision D11). The cross-sectional ranks (`StaticVarGenerateRanks(..., 0, 1234)`) are built per date over the watchlist
//! exactly like `ath_rank.rs` (rank 1 = highest value, a symbol without a value on that date gets no rank, ties by universe order); the index
//! helpers (Foreign alignment, calendar compression, TimeFrameExpand, supertrend / SMA filter) are the copy of the verified ones in `ath_loss2.rs`.
//! Verification notes: `ATH/v2/<id>/VERIFY.md`.
//!
//! Toggles use the raw 0/1 convention of the existing engines (deadstock 0 = YES, 1 = NO; turnflt 0 = NO, 1 = YES), ParamLists are 1-based like
//! `ath_rank.rs` (bkmkt 1 LARGECAP .. 4 MICROCAP). A Param default outside its own range is clamped into the range.
#![allow(dead_code)]
use crate::ath_core::*;

// ---------------------------------------------------------------- small helpers

/// A comparison with Null (NaN here) is false.
fn gt(a: f32, b: f32) -> bool {
    !a.is_nan() && !b.is_nan() && a > b
}
fn lt(a: f32, b: f32) -> bool {
    gt(b, a)
}
fn le(a: f32, b: f32) -> bool {
    !a.is_nan() && !b.is_nan() && a <= b
}

/// Turnover table of the "fine" AFLs (year by year from 2004), compared with OpenInt (the team's turnover field) or Volume * Close.
fn vol_fine(year: i32) -> f32 {
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

/// Turnover table of the "coarse" AFLs (30k / 50k / 1.8M / 2.6M / 6M / 13M).
fn vol_coarse(year: i32) -> f32 {
    match year {
        y if y < 2000 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

/// `MA(a, n)` the way `ath_rank.rs` verified it against the team sheets: the first bars of an array average the bars that exist.
fn ma_f(a: &[f32], n: usize) -> Vec<f32> {
    let n = n.max(1);
    let mut out = vec![f32::NAN; a.len()];
    let mut sum = 0f64;
    for i in 0..a.len() {
        sum += a[i] as f64;
        if i >= n {
            sum -= a[i - n] as f64;
        }
        out[i] = (sum / (i + 1).min(n) as f64) as f32;
    }
    out
}

/// `MA(a, n)` with Null for the first n-1 bars.
fn ma_n(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![f32::NAN; a.len()];
    if n == 0 {
        return out;
    }
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

/// `MA(a, n)` with Null (-1e10, as the copied index helpers expect it) for the first n-1 bars.
fn ma_null(a: &[f32], n: usize) -> Vec<f32> {
    ma_n(a, n).iter().map(|&v| if v.is_nan() { -1.0e10 } else { v }).collect()
}

/// `ROC(c, n)` in percent, Null for the first n bars.
fn roc(c: &[f32], n: usize) -> Vec<f32> {
    (0..c.len()).map(|i| if i >= n && c[i - n] != 0.0 { (c[i] - c[i - n]) / c[i - n] * 100.0 } else { f32::NAN }).collect()
}

/// `Ref(a, -k)`: the value k bars ago (Null before the start).
fn lag(a: &[f32], k: usize) -> Vec<f32> {
    (0..a.len()).map(|i| if i >= k { a[i - k] } else { f32::NAN }).collect()
}

/// `ATR(n)` of AmiBroker (Wilder; the simple average of the first n true ranges seeds it, Null before) as used by the verified scale-in engine.
fn atr_w(h: &[f32], l: &[f32], c: &[f32], n: usize) -> Vec<f32> {
    let len = h.len();
    let mut out = vec![f32::NAN; len];
    if n == 0 || len < n {
        return out;
    }
    let tr: Vec<f32> = (0..len).map(|i| if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect();
    out[n - 1] = tr[..n].iter().sum::<f32>() / n as f32;
    for i in n..len {
        out[i] = (out[i - 1] * (n as f32 - 1.0) + tr[i]) / n as f32;
    }
    out
}

/// `EMA(a, n)`, alpha 2/(n+1), seeded with the first non-Null value.
fn ema_n(a: &[f32], n: usize) -> Vec<f32> {
    let al = 2.0f32 / (n as f32 + 1.0);
    let mut out = vec![f32::NAN; a.len()];
    for i in 0..a.len() {
        if a[i].is_nan() {
            continue;
        }
        out[i] = if i == 0 || out[i - 1].is_nan() { a[i] } else { al * a[i] + (1.0 - al) * out[i - 1] };
    }
    out
}

fn hhv_n(a: &[f32], n: usize) -> Vec<f32> {
    (0..a.len()).map(|i| a[i.saturating_sub(n.saturating_sub(1))..=i].iter().copied().fold(f32::MIN, f32::max)).collect()
}
fn llv_n(a: &[f32], n: usize) -> Vec<f32> {
    (0..a.len()).map(|i| a[i.saturating_sub(n.saturating_sub(1))..=i].iter().copied().fold(f32::MAX, f32::min)).collect()
}

/// 30 identical O/H/L/C bars AHEAD (`Ref(x, k)`, k = 1..30 looks forward; beyond the last bar it is Null and the comparison false): the AFL's `deadstock1`.
fn dead30(s: &Series) -> Vec<bool> {
    let n = s.bars.len();
    let same = |i: usize| {
        let (a, b) = (&s.bars[i], &s.bars[i + 1]);
        a.o == b.o && a.h == b.h && a.l == b.l && a.c == b.c
    };
    let mut out = vec![false; n];
    let mut run = 0usize;
    for i in (0..n).rev() {
        if i + 1 < n && same(i) {
            run += 1;
        } else {
            run = 0;
        }
        out[i] = run >= 30;
    }
    out
}

/// Per-symbol arrays every AFL of this batch needs: price arrays, `Hcl = HHV(C, BarCount)`, the %swing `trend1`, the circuit tests and the dead-stock flags.
struct Ctx {
    n: usize,
    c: Vec<f32>,
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    v: Vec<f32>,
    oi: Vec<f32>,
    hcl: Vec<f32>,
    trend: Vec<i8>,
    /// UC = H == C, LC = L == C
    uc: Vec<bool>,
    lc: Vec<bool>,
    /// UPcircuit / locircuit: the Prec(x, 2) tests
    upc: Vec<bool>,
    loc: Vec<bool>,
    dead1: Vec<bool>,
    /// DEADSTOCKBUYSIG / deadstocksellsig
    dbuy: Vec<bool>,
    dsell: Vec<bool>,
}

fn ctx(s: &Series, exit_pct: f32, dead_yes: bool) -> Ctx {
    let n = s.bars.len();
    let (c, o, h, l, v, oi) = (s.close(), s.open(), s.high(), s.low(), s.volume(), s.openint());
    let mut hcl = vec![0f32; n];
    for i in 0..n {
        hcl[i] = if i == 0 || c[i] > hcl[i - 1] { c[i] } else { hcl[i - 1] };
    }
    // %swing line: EXIT = C*EXITPCT/100; Up = C + EXIT; Dn = C - EXIT (float32)
    let exit: Vec<f32> = c.iter().map(|&x| (x * exit_pct) / 100.0).collect();
    let mut up: Vec<f32> = (0..n).map(|i| c[i] + exit[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| c[i] - exit[i]).collect();
    let mut trend = vec![0i8; n];
    if n > 0 {
        trend[0] = 1;
    }
    for i in 1..n {
        trend[i] = 1;
        if c[i] > up[i - 1] {
            trend[i] = 1;
        } else if c[i] < dn[i - 1] {
            trend[i] = -1;
        } else {
            trend[i] = trend[i - 1];
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
    }
    let uc = (0..n).map(|i| h[i] == c[i]).collect();
    let lc = (0..n).map(|i| l[i] == c[i]).collect();
    let upc = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i])).collect();
    let loc = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i])).collect();
    let dbuy = (0..n).map(|i| if dead_yes { i != n - 1 } else { i != 0 }).collect();
    let dsell = (0..n).map(|i| dead_yes && i == n - 1).collect();
    Ctx { n, c, o, h, l, v, oi, hcl, trend, uc, lc, upc, loc, dead1: dead30(s), dbuy, dsell }
}

impl Ctx {
    /// `buysig`: 1 on a close above all earlier closes, 0 when trend1 == -1 while it was 1, else carried (the AFL's running ATH state).
    fn buysig(&self) -> Vec<bool> {
        let mut b = vec![false; self.n];
        for i in 1..self.n {
            b[i] = if self.c[i] > self.hcl[i - 1] {
                true
            } else if self.trend[i] == -1 && b[i - 1] {
                false
            } else {
                b[i - 1]
            };
        }
        b
    }
    /// turnover test with the OpenInt field (`turnflt`) against a table, or Volume * Close
    fn turnover_oi(&self, s: &Series, on: bool, table: fn(i32) -> f32) -> Vec<bool> {
        (0..self.n).map(|i| !on || self.oi[i] > table(s.bars[i].date / 10000)).collect()
    }
    fn turnover_vc(&self, s: &Series, on: bool, table: fn(i32) -> f32) -> Vec<bool> {
        (0..self.n).map(|i| !on || self.v[i] * self.c[i] > table(s.bars[i].date / 10000)).collect()
    }
    /// fills at the close, clamped into the bar like AmiBroker keeps a price inside the bar
    fn close_px(&self) -> Vec<f32> {
        (0..self.n).map(|i| self.c[i].min(self.h[i]).max(self.l[i])).collect()
    }
}

/// Signals after `ExRem` (when `rem`), fills at the Close, an optional PositionScore and the hard stop (`ApplyStop` loss percent,
/// ActivateStopsImmediately False).
fn finish(x: &Ctx, buy: Vec<bool>, sell: Vec<bool>, rem: bool, score: Option<Vec<f32>>, hard: f32) -> Signals {
    let (b, s) = if rem {
        let b = exrem(&buy, &sell);
        let s = exrem(&sell, &b);
        (b, s)
    } else {
        (buy, sell)
    };
    let px = x.close_px();
    let mut sg = Signals::new(b, s, px.clone(), px);
    sg.score = score;
    if hard > 0.0 {
        sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
    }
    sg
}

/// `StaticVarGenerateRanks("rank", ..., 0, 1234)` per date over the universe: rank 1 = highest value; a Null value (NaN) or a symbol without a bar
/// that date gets no rank (NaN). Ties keep the universe order (the AFL's seeded shuffle cannot be reproduced).
fn ranks_by_date(series: &[Series], vals: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let mut out: Vec<Vec<f32>> = series.iter().map(|s| vec![f32::NAN; s.bars.len()]).collect();
    let mut dates: Vec<i32> = series.iter().flat_map(|s| s.bars.iter().map(|b| b.date)).collect();
    dates.sort_unstable();
    dates.dedup();
    let mut cur = vec![0usize; series.len()];
    let mut buf: Vec<(f32, usize, usize)> = Vec::with_capacity(series.len());
    for &d in &dates {
        buf.clear();
        for (k, s) in series.iter().enumerate() {
            let i = cur[k];
            if i < s.bars.len() && s.bars[i].date == d {
                let x = vals[k][i];
                if x.is_finite() {
                    buf.push((x, k, i));
                }
                cur[k] += 1;
            }
        }
        buf.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        for (r, &(_, k, i)) in buf.iter().enumerate() {
            out[k][i] = (r + 1) as f32;
        }
    }
    out
}

fn dead_yes(g: G) -> bool {
    g("deadstock").round() == 0.0
}
fn turn_on(g: G) -> bool {
    g("turnflt").round() == 1.0
}

/// Parameters shared by almost every file.
fn base_schema() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
    ]
}
fn with(mut v: Vec<P>, more: Vec<P>) -> Vec<P> {
    v.extend(more);
    v
}
fn hard_p() -> P {
    p("hardstop", "Hard Stoploss % (0 = off; ApplyStop percent loss)", 0.0, 0.0, 50.0, 1.0)
}

// ================================================================ 1. ATHC & %Swing with fixed ATR scale-in (5), fine turnover table

fn schema_scale() -> Vec<P> {
    with(
        base_schema(),
        vec![
            p("exitpct", "EXIT %", 10.0, 5.0, 25.0, 1.0),
            p("atr_prd", "ATR Prds", 14.0, 2.0, 100.0, 1.0),
            p("myatr1", "ATR Multiplier", 1.0, 0.0, 5.0, 0.025),
            p("buffer_prcnt", "Buffer %", 0.25, 0.0, 2.0, 0.001),
        ],
    )
}

/// The AFL is the verified `ATHC & %Swing with Fixed ATR Scalein(5)` (no loss gate, `turnover = (V*C) > volsig`) with the fine year-by-year `volsig` table.
/// The scale-in engine of `ath_loss2` (checked against the UNITECH sheet) carries the coarse table for V*C; the table is swapped by handing it a copy of the
/// series whose Volume is a flag: 1e30 where V*C passes the FINE table, 0 where it does not (V*C > coarse table then gives exactly that result).
fn signals_scale(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let tf = turn_on(g);
    let sp = crate::ath_loss2::ScaleP {
        exit_pct: g("exitpct") as f32,
        losspct: 0.0,
        noofloss: 0.0,
        comm: 0.65,
        penny: g("pennyflt") as f32,
        turnflt: tf,
        dead_yes: dead_yes(g),
        atr_prd: g("atr_prd").max(0.0) as usize,
        myatr1: g("myatr1") as f32,
        buffer: g("buffer_prcnt") as f32,
        atr_mode: 0,
        plain: true,
    };
    series
        .iter()
        .map(|s| {
            if !tf {
                return crate::ath_loss2::scalein_signals(s, &sp);
            }
            let bars = s.bars.iter().map(|b| Bar { v: if b.v * b.c > vol_fine(b.date / 10000) { 1.0e30 } else { 0.0 }, ..*b }).collect();
            crate::ath_loss2::scalein_signals(&Series { name: s.name.clone(), bars }, &sp)
        })
        .collect()
}

// ================================================================ ATHC entry, %swing exit with a ranking (4 files, scalar `Position` loop)

/// `ATHCBuysig`: 1 on a close above all earlier closes, 0 when trend1 == -1 while 1, else carried.
fn athcb(x: &Ctx) -> Vec<bool> {
    x.buysig()
}

#[derive(Clone, Copy, PartialEq)]
enum Rk {
    /// PositionScore = Ref(ROC(C, ROC_Prd), -1)
    Roc,
    /// PositionScore = Ref(Ref(ROC(C, ROC_Prd), -DataHole_Prd), -1)
    RocHole,
    /// entry needs ATR% < value, PositionScore = 100 - ATR%
    AtrPct,
}

/// Entry `Position == 0 AND Close > Highest_Close[i-1] AND [ATR% < value AND] Buy_Cash_Filter`; exit
/// `(LC == 0 AND (trend1 == -1 OR ATHCBuysig == 0)) OR deadstocksellsig` while in a position; fills at the Close.
fn entry_rank_one(s: &Series, g: G, kind: Rk) -> Signals {
    let exit_pct = match kind {
        Rk::Roc => 21.0,
        _ => g("exitpct") as f32,
    };
    let x = ctx(s, exit_pct, dead_yes(g));
    let n = x.n;
    let penny = g("pennyflt") as f32;
    let nogate = g("dbg_nogate") != 0.0; // hidden test key (0 in production): the ATR% entry gate off
    let prec_uc = g("dbg_prec") != 0.0; // hidden test key (0 in production): the buy filter's UC test as the nearest sheet's AFL has it (Prec circuit instead of H == C)
    let turnover = x.turnover_oi(s, turn_on(g), vol_fine);
    let ab = athcb(&x);
    let (score, gate): (Vec<f32>, Vec<bool>) = match kind {
        Rk::Roc => {
            let r = roc(&x.c, 12);
            (lag(&r, 1), vec![true; n])
        }
        Rk::RocHole => {
            let r = lag(&roc(&x.c, g("roc_prd").round() as usize), g("datahole_prd").round() as usize);
            (lag(&r, 1), vec![true; n])
        }
        Rk::AtrPct => {
            let atr = atr_w(&x.h, &x.l, &x.c, 14);
            let pct: Vec<f32> = if g("ver").round() == 1.0 {
                (0..n).map(|i| (atr[i] / x.c[i]) * 100.0).collect()
            } else {
                let avg = ma_n(&x.c, 14);
                (0..n).map(|i| (atr[i] / avg[i]) * 100.0).collect()
            };
            ((0..n).map(|i| 100.0 - pct[i]).collect(), (0..n).map(|i| nogate || lt(pct[i], 8.0)).collect())
        }
    };
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut pos = false;
    for i in 1..n {
        let cash = (if prec_uc { !x.upc[i] } else { !x.uc[i] }) && turnover[i] && x.c[i] > penny && x.dbuy[i];
        if !pos && x.c[i] > x.hcl[i - 1] && gate[i] && cash {
            buy[i] = true;
            pos = true;
        } else if pos && ((!x.lc[i] && (x.trend[i] == -1 || !ab[i])) || x.dsell[i]) {
            sell[i] = true;
            pos = false;
        }
    }
    finish(&x, buy, sell, false, Some(score), 0.0)
}

fn signals_ranking(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| entry_rank_one(s, g, Rk::Roc)).collect()
}
fn signals_ranking_2(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| entry_rank_one(s, g, Rk::RocHole)).collect()
}
fn signals_atrpct(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| entry_rank_one(s, g, Rk::AtrPct)).collect()
}

fn schema_ranking() -> Vec<P> {
    base_schema()
}
fn schema_ranking_2() -> Vec<P> {
    with(base_schema(), vec![p("exitpct", "Swing EXIT %", 21.0, 1.0, 40.0, 1.0), p("datahole_prd", "DataHole Prd", 30.0, 1.0, 365.0, 1.0), p("roc_prd", "ROC", 12.0, 1.0, 300.0, 1.0)])
}
fn schema_atrpct() -> Vec<P> {
    with(base_schema(), vec![p("ver", "Avg Price Version (1 Current Close, 2 Avg Close)", 1.0, 1.0, 2.0, 1.0), p("exitpct", "Swing EXIT %", 21.0, 5.0, 25.0, 1.0)])
}

// ================================================================ ATHC entry when turnover is double the average

fn schema_double() -> Vec<P> {
    with(base_schema(), vec![p("exitpct", "Swing EXIT %", 21.0, 1.0, 40.0, 1.0), p("avgturnover_multi", "Multiplier of TurnOver (the AFL default 2 lies outside its range 4-5: clamped to 4)", 4.0, 4.0, 5.0, 1.0)])
}

fn double_one(s: &Series, g: G) -> Signals {
    let x = ctx(s, g("exitpct") as f32, dead_yes(g));
    let n = x.n;
    let penny = g("pennyflt") as f32;
    let multi = g("avgturnover_multi") as f32;
    let turnover = x.turnover_oi(s, turn_on(g), vol_fine);
    let ab = athcb(&x);
    const PRD: usize = 10; // Optimize("Period for Average of TurnOver", 10, 5, 5, 1)
    let avg: Vec<f32> = ma_f(&x.oi, PRD).iter().map(|&m| m * multi).collect();
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut pos, mut skip, mut ath_price) = (false, false, 0f32);
    for i in PRD..n {
        let cash = (if g("dbg_prec") != 0.0 { !x.upc[i] } else { !x.uc[i] }) && turnover[i] && x.c[i] > penny && x.dbuy[i];
        let big = g("dbg_nogate") != 0.0 || gt(x.oi[i], avg[i - 1]); // dbg_nogate: hidden test key, the turnover-spike gate off
        if !pos && x.c[i] > x.hcl[i - 1] && big {
            if cash {
                buy[i] = true;
                pos = true;
            } else {
                skip = true;
                ath_price = x.c[i];
            }
        }
        if !pos && skip && x.trend[i] == -1 {
            skip = false;
        } else if !pos && x.trend[i] == 1 && skip && x.c[i] > ath_price && ath_price > 0.0 && big && cash {
            buy[i] = true;
            pos = true;
            skip = false;
            ath_price = 0.0;
        } else if pos && ((!x.lc[i] && (x.trend[i] == -1 || !ab[i])) || x.dsell[i]) {
            sell[i] = true;
            pos = false;
            skip = false;
            ath_price = 0.0;
        }
    }
    finish(&x, buy, sell, false, None, 0.0)
}
fn signals_double(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| double_one(s, g)).collect()
}

// ================================================================ ATHC buy with a ROC filter (6 months): the plain file and the "running trade" file

fn schema_roc6() -> Vec<P> {
    with(
        base_schema(),
        vec![
            p("circuitflt", "Circuit Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
            p("roc_tf", "ROC TimeFrame Monthly/Daily (0 Monthly, 1 Daily)", 1.0, 0.0, 1.0, 1.0),
            p("roc_prd", "ROC Periods", 180.0, 1.0, 366.0, 1.0),
            p("roc_val", "ROC Value", 100.0, 0.0, 200.0, 1.0),
            p("roc_ranking", "ROC Ranking (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
            p("exitpct", "EXIT %", 10.0, 1.0, 20.0, 1.0),
        ],
    )
}
fn schema_running() -> Vec<P> {
    schema_roc6().into_iter().filter(|q| q.key != "circuitflt").collect()
}

/// `MyROC`: ROC(C, ROC_Prd) on the daily bars, or on the stock's own monthly bars (TimeFrameSet(inMonthly)) and TimeFrameExpand'ed back.
fn my_roc(s: &Series, x: &Ctx, g: G) -> Vec<f32> {
    let prd = g("roc_prd").round() as usize;
    if g("roc_tf").round() == 1.0 {
        return roc(&x.c, prd);
    }
    let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
    let al = apx::align(s, &dates);
    let k = apx::compress(&dates, &al, 0);
    let rm = roc(&k.c, prd);
    apx::expand(&rm, &k, x.n).iter().map(|&v| if apx::is_null(v) { f32::NAN } else { v }).collect()
}

fn roc6_one(s: &Series, g: G, running: bool) -> Signals {
    let x = ctx(s, g("exitpct") as f32, dead_yes(g));
    let n = x.n;
    let penny = g("pennyflt") as f32;
    let nogate = g("dbg_nogate") != 0.0; // hidden test key (0 in production): the ROC gate off (also where the ROC is Null)
    let val = g("roc_val") as f32;
    let myroc = my_roc(s, &x, g);
    let turnover = x.turnover_oi(s, turn_on(g), vol_fine);
    let circuit = !running && g("circuitflt").round() == 1.0;
    // plain file: UpCircuit / LoCircuit are 1 (CircuitFlt NO) or "not a circuit" (YES) and are PASS filters; running file: they are the circuits
    let up_pass: Vec<bool> = (0..n).map(|i| if running { !x.upc[i] } else if circuit { prec2(x.o[i]) != prec2(x.c[i]) || prec2(x.o[i]) != prec2(x.h[i]) } else { true }).collect();
    let lo_pass: Vec<bool> = (0..n).map(|i| if running { !x.loc[i] } else if circuit { prec2(x.o[i]) != prec2(x.c[i]) || prec2(x.o[i]) != prec2(x.l[i]) } else { true }).collect();
    let also_uc = g("dbg_uc") != 0.0; // hidden test key (0 in production): add the nearest AFL's `UC == 0` (H != C) to the buy filter
    let cash: Vec<bool> = (0..n).map(|i| turnover[i] && up_pass[i] && x.c[i] > penny && x.dbuy[i] && !(also_uc && x.uc[i])).collect();
    let blocker: Vec<bool> = (0..n).map(|i| !turnover[i] || x.upc[i] || lt(x.c[i], penny) || !x.dbuy[i]).collect();
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut st = vec![0u8; n]; // ATHCBuysig 0 / 1 / 2 (running file)
    let mut pos = false;
    for i in 1..n {
        let fresh = x.c[i] > x.hcl[i - 1];
        if running {
            st[i] = if fresh && ((!nogate && lt(myroc[i], val)) || blocker[i]) {
                1
            } else if x.trend[i] == -1 {
                0
            } else if st[i - 1] == 1 && (nogate || gt(myroc[i], val)) && x.trend[i] == 1 {
                2
            } else {
                st[i - 1]
            };
        }
        let pass = nogate || gt(myroc[i], val);
        let enter = if running { (fresh && pass) || st[i] == 2 } else { fresh && pass };
        if !pos && enter && cash[i] {
            buy[i] = true;
            pos = true;
        } else if (pos && x.trend[i] == -1 && lo_pass[i]) || x.dsell[i] {
            sell[i] = true;
            pos = false;
        }
    }
    let score = if g("roc_ranking").round() == 1.0 { Some(myroc) } else { None };
    finish(&x, buy, sell, false, score, 0.0)
}
fn signals_roc6(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| roc6_one(s, g, false)).collect()
}
fn signals_running(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| roc6_one(s, g, true)).collect()
}

// ================================================================ ATHC & %Swing with ROC(30) and ROC(365) subset ranking (cross-section over the watchlist)

fn schema_roc365_2() -> Vec<P> {
    with(
        base_schema(),
        vec![
            p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0),
            p("roc_fast_prds", "ROC Outperforming Periods", 30.0, 5.0, 100.0, 1.0),
            p("roc_slow_prds", "ROC Underperforming Periods", 365.0, 10.0, 400.0, 1.0),
            p("roc_30_rank_prds", "ROC 30 Ranking Stocks", 20.0, 1.0, 50.0, 1.0),
            p("roc_365_rank_prds", "ROC 365 Ranking Stocks", 365.0, 20.0, 500.0, 1.0),
        ],
    )
}
fn schema_roc365() -> Vec<P> {
    with(
        base_schema(),
        vec![
            p("roc_fast_prds", "ROC Outperforming Periods", 30.0, 5.0, 100.0, 1.0),
            p("roc_slow_prds", "ROC Underperforming Periods", 365.0, 10.0, 400.0, 1.0),
            p("datahole_prd", "DataHole Prd", 30.0, 1.0, 365.0, 1.0),
            p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0),
            p("roc_30_rank_prds", "ROC 30 Ranking Stocks", 20.0, 1.0, 50.0, 1.0),
            p("roc_365_rank_prds", "ROC 365 Ranking Stocks", 365.0, 20.0, 500.0, 1.0),
        ],
    )
}

fn roc365(series: &[Series], g: G, hole: bool) -> Vec<Signals> {
    let exit_pct = g("exitpct") as f32;
    let (fast, slow) = (g("roc_fast_prds").round() as usize, g("roc_slow_prds").round() as usize);
    let dh = if hole { g("datahole_prd").round() as usize } else { 0 };
    let (r30, r365) = (g("roc_30_rank_prds") as f32, g("roc_365_rank_prds") as f32);
    let penny = g("pennyflt") as f32;
    let dy = dead_yes(g);
    // The per-symbol context is rebuilt where it is needed instead of being kept for all symbols (memory: 1,313 contexts at once ran the
    // 1 GiB engine container out of memory); `ctx` is a pure function of the series, so the signals are unchanged.
    // the per-symbol values that are ranked: ROC(fast) and -ROC(slow) on the bars that are candidates
    let mut d1: Vec<Vec<f32>> = Vec::with_capacity(series.len());
    let mut d2: Vec<Vec<f32>> = Vec::with_capacity(series.len());
    for s in series {
        let x = &ctx(s, exit_pct, dy);
        let n = x.n;
        let mut rf = roc(&x.c, fast);
        let mut rs: Vec<f32> = roc(&x.c, slow).iter().map(|&v| -v).collect();
        if hole {
            rf = lag(&rf, dh);
            rs = lag(&rs, dh);
        }
        let bs = x.buysig();
        let (mut a, mut b) = (vec![f32::NAN; n], vec![f32::NAN; n]);
        for i in 0..n {
            let chk = !rs[i].is_nan();
            let cand = if hole { bs[i] && !x.dead1[i] && chk } else { i >= 1 && x.c[i] > x.hcl[i - 1] && chk };
            if cand {
                a[i] = rf[i];
                b[i] = rs[i];
            }
        }
        d1.push(a);
        d2.push(b);
    }
    let rank1 = ranks_by_date(series, &d1);
    let rank2 = ranks_by_date(series, &d2);
    let tf = turn_on(g);
    let nogate = g("dbg_nogate") != 0.0; // hidden test key (0 in production): the ranking gate off (any rank passes), to compare the machinery with the running-entry engines
    series
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let x = &ctx(s, exit_pct, dy);
            let n = x.n;
            let bs = x.buysig();
            let turnover = if hole { x.turnover_oi(s, tf, vol_fine) } else { x.turnover_vc(s, tf, vol_coarse) };
            let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
            let mut pos = false;
            let score: Vec<f32> = (0..n).map(|i| 100.0 - (rank1[k][i] + rank2[k][i]) / 2.0).collect();
            for i in 1..n {
                let (r1, r2) = (rank1[k][i], rank2[k][i]);
                let ranks_ok = nogate || (le(r1, r30) && le(r2, r365));
                let athx = bs[i] || x.c[i] > x.hcl[i - 1];
                if hole {
                    // Buy_Cash_Filter = UC == 0 AND Turnover AND Close > PennyFlt AND DEADSTOCKBUYSIG; no Position test (backtestRegularRaw); Sell is an else-if
                    let cash = !x.uc[i] && turnover[i] && x.c[i] > penny && x.dbuy[i];
                    if athx && ranks_ok && cash && !x.dead1[i] {
                        buy[i] = true;
                    } else if ((x.trend[i] == -1 || !bs[i]) && !x.lc[i]) || x.dsell[i] || x.dead1[i] {
                        sell[i] = true;
                    }
                } else {
                    // OtherFilterCnd = DEADSTOCKBUYSIG AND C > pennyflt AND turnover AND NOT UPCIRCUIT; Position loop, the sell is a separate `if`
                    let other = x.dbuy[i] && x.c[i] > penny && turnover[i] && !x.upc[i];
                    if !pos && athx && ranks_ok && other && !x.dead1[i] {
                        buy[i] = true;
                        pos = true;
                    }
                    if (pos && (x.trend[i] == -1 || !bs[i]) && !x.loc[i]) || x.dsell[i] || x.dead1[i] {
                        sell[i] = true;
                        pos = false;
                    }
                }
            }
            finish(x, buy, sell, false, Some(score), 0.0)
        })
        .collect()
}
fn signals_roc365(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    roc365(series, g, true)
}
fn signals_roc365_2(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    roc365(series, g, false)
}

// ================================================================ ATHC %swing exit with average-turnover ranking bands (VER 3 / 2 / 1 files)

fn schema_avg(exit_max: f64, pd: f64) -> Vec<P> {
    with(
        base_schema(),
        vec![
            hard_p(),
            p("exitpct", "EXIT %", 10.0, 5.0, exit_max, 1.0),
            p("turnoverpd", "TURNOVER AVG PERIOD", pd, 5.0, 500.0, 1.0),
            p("bkmkt", "BACKTEST MARKET AS PER VOLUME (1 LARGECAP, 2 MIDCAP, 3 SMALLCAP, 4 MICROCAP)", 1.0, 1.0, 4.0, 1.0),
        ],
    )
}
fn schema_avg1() -> Vec<P> {
    schema_avg(35.0, 125.0)
}
fn schema_avg3() -> Vec<P> {
    schema_avg(25.0, 60.0)
}

/// RANKHELD1 / RANKHELD2 per class: `rank > RANKHELD1 AND rank < RANKHELD2`.
type Bands = [(f32, f32); 4];
const BANDS_1000: Bands = [(0.0, 101.0), (100.0, 251.0), (250.0, 501.0), (500.0, 1001.0)];
const BANDS_800: Bands = [(0.0, 101.0), (100.0, 251.0), (250.0, 501.0), (500.0, 801.0)];
const BANDS_VER1: Bands = [(0.0, 101.0), (100.0, 251.0), (250.0, 801.0), (800.0, 80000.0)];

/// Ranked value `IIf(deadstock1 == 0, MA(OpenInt, turnoverpd), 0)` per date over the watchlist (highest = rank 1); buy while the rank is inside the
/// class band: `(buysig OR C > Ref(Hcl,-1)) AND NOT UPCIRCUIT AND DEADSTOCKBUYSIG AND C > pennyflt AND turnover AND rank > lo AND rank < hi AND NOT deadstock1`;
/// `Sell = (SELLSIG AND NOT locircuit) OR deadstocksellsig OR deadstock1`; ExRem; fills at the Close; PositionScore = MA(OpenInt, turnoverpd).
/// Hidden test keys (not schema keys, 0 in production): `dbg_vc` ranks / filters with Volume * Close instead of OpenInt, `dbg_uc` uses UC / LC = H==C / L==C
/// instead of the Prec(x,2) circuits: with both on the file equals the verified VER2 engine of `ath_rank.rs`.
fn avg_run(series: &[Series], g: G, bands: Bands) -> Vec<Signals> {
    let pd = g("turnoverpd").round().max(1.0) as usize;
    let (lo, hi) = bands[(g("bkmkt").round() as usize).clamp(1, 4) - 1];
    let dy = dead_yes(g);
    let exit_pct = g("exitpct") as f32;
    let vc = g("dbg_vc") != 0.0;
    let ucm = g("dbg_uc") != 0.0;
    let xs: Vec<Ctx> = series.iter().map(|s| ctx(s, exit_pct, dy)).collect();
    let turn: Vec<Vec<f32>> = xs
        .iter()
        .map(|x| {
            let base: Vec<f32> = if vc { (0..x.n).map(|i| x.c[i] * x.v[i]).collect() } else { x.oi.clone() };
            ma_f(&base, pd)
        })
        .collect();
    let vals: Vec<Vec<f32>> = xs.iter().zip(&turn).map(|(x, t)| (0..x.n).map(|i| if x.dead1[i] { 0.0 } else { t[i] }).collect()).collect();
    let rk = ranks_by_date(series, &vals);
    let (penny, hard, tf) = (g("pennyflt") as f32, g("hardstop") as f32, turn_on(g));
    series
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let x = &xs[k];
            let n = x.n;
            let bs = x.buysig();
            let turnover = if vc { x.turnover_vc(s, tf, vol_fine) } else { x.turnover_oi(s, tf, vol_fine) };
            let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
            for i in 0..n {
                let r = rk[k][i];
                let in_band = gt(r, lo) && lt(r, hi);
                let ath = bs[i] || (i > 0 && x.c[i] > x.hcl[i - 1]);
                let (upc, loc) = if ucm { (x.uc[i], x.lc[i]) } else { (x.upc[i], x.loc[i]) };
                buy[i] = ath && !upc && x.dbuy[i] && x.c[i] > penny && turnover[i] && in_band && !x.dead1[i];
                sell[i] = (x.trend[i] == -1 && !loc) || x.dsell[i] || x.dead1[i];
            }
            finish(x, buy, sell, true, Some(turn[k].clone()), hard)
        })
        .collect()
}
fn signals_avg1(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    avg_run(series, g, BANDS_1000)
}
fn signals_avg2(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    avg_run(series, g, BANDS_800)
}
fn signals_avg3(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    avg_run(series, g, BANDS_VER1)
}

// ================================================================ avg turnover VER2 with SL options and an index filter (file _5)

const INDEX_NAMES: [&str; 4] = ["NSENIFTY", "NSENIFTY1", "NIFTY", "NSE500"];

fn schema_avg5() -> Vec<P> {
    with(
        base_schema(),
        vec![
            p("exitpct", "EXIT %", 10.0, 10.0, 22.0, 1.0),
            p("ver", "SL EXIT (1 OFF, 2 % Based HARD SL, 3 ATR Based HARD SL, 4 5 DAYS LOWEST CLOSE)", 1.0, 1.0, 4.0, 1.0),
            p("hardslpct", "HARD SL%", 2.0, 0.0, 12.0, 1.0),
            p("atr_prd", "ATR Prds", 5.0, 0.0, 100.0, 1.0),
            p("atr_sl_m", "ATR Hard SL Multiplier", 3.0, 0.0, 6.0, 1.0),
            p("days_sl", "DAYS SL", 1.0, 1.0, 100.0, 1.0),
            p("mktflt", "FILTER TYPE (1 NONE, 2 INDEX)", 1.0, 1.0, 2.0, 1.0),
            p("flttf", "FILTER TIMEFRAME (1 MONTHLY, 2 WEEKLY, 3 DAILY)", 1.0, 1.0, 3.0, 1.0),
            p("prflt", "INDEX (1 NSENIFTY, 2 NSENIFTY1, 3 NIFTY, 4 NSE500)", 1.0, 1.0, 4.0, 1.0),
            p("mktscene", "Market Condition (1 Bull, 2 Bear)", 1.0, 1.0, 2.0, 1.0),
            p("iflt", "INDEX Filter (1 ST, 2 SMA)", 1.0, 1.0, 2.0, 1.0),
            p("istflt", "ST CALCULATION POINT (1 MID POINT(H+L/2), 2 CLOSE, 3 PIVOT(H+L+C/3))", 1.0, 1.0, 3.0, 1.0),
            p("ifltpd", "INDEX ST PERIOD", 20.0, 0.0, 20.0, 1.0),
            p("ifltfact", "INDEX ST FACTOR", 1.0, 0.0, 20.0, 1.0),
            p("ifltmapd", "INDEX SMA PERIOD", 200.0, 1.0, 300.0, 1.0),
            p("turnoverpd", "TURNOVER AVG PERIOD", 60.0, 1.0, 200.0, 1.0),
            p("bkmkt", "BACKTEST MARKET AS PER VOLUME (1 LARGECAP, 2 MIDCAP, 3 SMALLCAP, 4 MICROCAP)", 1.0, 1.0, 4.0, 1.0),
        ],
    )
}

/// (FLTBuy, FLTShort) of the AFL for the symbol: FILTER TYPE NONE = (false, true); INDEX = the monthly / weekly / daily supertrend or SMA of the chosen
/// index on the stock's bar dates (Bull + ST swaps the two, the SMA branch is the same for Bull and Bear), a missing index (NIFTY) gives (false, false).
fn index_flags(s: &Series, env: &Env, g: G) -> (Vec<bool>, Vec<bool>) {
    let n = s.bars.len();
    if g("mktflt").round() != 2.0 {
        return (vec![false; n], vec![true; n]);
    }
    let Some(ix) = env.get(INDEX_NAMES[(g("prflt").round() as usize).clamp(1, 4) - 1]) else {
        return (vec![false; n], vec![false; n]);
    };
    let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
    let f = apx::Flt {
        tf: (g("flttf").round() as u8).clamp(1, 3) - 1,
        sma: g("iflt").round() == 2.0,
        point: (g("istflt").round() as u8).clamp(1, 3) - 1,
        pd: g("ifltpd").round().max(0.0) as usize,
        fact: g("ifltfact") as f32,
        mapd: g("ifltmapd").round().max(1.0) as usize,
    };
    let (b, sh) = apx::st_filter(&apx::align(ix, &dates), &dates, &f);
    if !f.sma && g("mktscene").round() == 1.0 {
        (sh, b)
    } else {
        (b, sh)
    }
}

fn signals_avg5(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    let pd = g("turnoverpd").round().max(1.0) as usize;
    let (lo, hi) = BANDS_VER1[(g("bkmkt").round() as usize).clamp(1, 4) - 1];
    let dy = dead_yes(g);
    let exit_pct = g("exitpct") as f32;
    let xs: Vec<Ctx> = series.iter().map(|s| ctx(s, exit_pct, dy)).collect();
    let turn: Vec<Vec<f32>> = xs.iter().map(|x| ma_f(&(0..x.n).map(|i| x.c[i] * x.v[i]).collect::<Vec<f32>>(), pd)).collect();
    let vals: Vec<Vec<f32>> = xs.iter().zip(&turn).map(|(x, t)| (0..x.n).map(|i| if x.dead1[i] { 0.0 } else { t[i] }).collect()).collect();
    let rk = ranks_by_date(series, &vals);
    let (penny, tf) = (g("pennyflt") as f32, turn_on(g));
    let ver = g("ver").round() as i32;
    let (hslpct, atr_m, days) = (g("hardslpct") as f32, g("atr_sl_m") as f32, g("days_sl").round() as usize);
    let index = g("mktflt").round() == 2.0;
    let running = g("dbg_running") != 0.0;
    series
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let x = &xs[k];
            let n = x.n;
            let bs = x.buysig();
            let (fltbuy, fltshort) = index_flags(s, env, g);
            let turnover = x.turnover_vc(s, tf, vol_fine);
            let atr = atr_w(&x.h, &x.l, &x.c, g("atr_prd").round() as usize);
            // SL1 = LLV(C, DAYS_SL) when "5 DAYS LOWEST CLOSE" and DAYS_SL > 1, else Null
            let sl1: Vec<f32> = if ver == 4 && days > 1 { llv_n(&x.c, days) } else { vec![f32::NAN; n] };
            let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
            // the AFL's persistent variables (never reset on an exit): start at 0
            let (mut pos, mut skip, mut hsl, mut atr_hsl, mut sl) = (false, false, 0f32, 0f32, 0f32);
            for i in 1..n {
                let cash = !x.uc[i] && turnover[i] && x.c[i] > penny && x.dbuy[i];
                let r = rk[k][i];
                let band = gt(r, lo) && lt(r, hi);
                let flt_ok = !index || fltshort[i];
                let ath = x.c[i] > x.hcl[i - 1] || (running && bs[i]); // `running` = hidden test key dbg_running (0 in production): the VER2 running entry, to compare with the verified engine
                if !pos && cash && flt_ok && ath && band && !x.dead1[i] {
                    buy[i] = true;
                    if ver == 2 && hslpct > 0.0 {
                        hsl = (1.0 - hslpct * 0.01) * x.c[i];
                    }
                    if ver == 3 && gt(atr[i], 0.0) {
                        atr_hsl = x.c[i] - (atr[i] * atr_m);
                    }
                    if ver == 4 {
                        sl = sl1[i];
                    }
                    pos = true;
                } else if pos && !skip {
                    let stop = le(x.c[i], hsl) || le(x.c[i], atr_hsl) || le(x.c[i], sl);
                    if stop || x.dsell[i] || x.trend[i] == -1 || (index && fltbuy[i]) || x.dead1[i] {
                        if !x.lc[i] || x.dsell[i] || x.dead1[i] {
                            sell[i] = true;
                            pos = false;
                        } else {
                            skip = true;
                        }
                    }
                } else if skip && (!x.lc[i] || x.dsell[i] || x.dead1[i]) {
                    sell[i] = true;
                    pos = false;
                    skip = false;
                }
            }
            finish(x, buy, sell, false, Some(turn[k].clone()), 0.0)
        })
        .collect()
}

// ================================================================ ATHC %swing exit with ranking (ROC, RSI, STD, positive closes, swing)

fn schema_rsi() -> Vec<P> {
    with(
        base_schema(),
        vec![
            p("tstart", "TRADING START BAR (the AFL default 0 lies outside its range 1-500: clamped to 1)", 1.0, 1.0, 500.0, 1.0),
            hard_p(),
            p("rankmode", "RANKING MODE (1 NONE; 2 POSITIVE ROC; 3 NEGATIVE ROC; 4 RSI; 5 STD; 6 POSITIVE ROC/STD; 7 NEGATIVE ROC/STD; 8 POSITIVE CLOSE; 9 SWING; 10 POSITIVE ROC + POSITIVE CLOSE; 11 POSITIVE CLOSE + SWING)", 1.0, 1.0, 11.0, 1.0),
            p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
            p("stdpd", "STD PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
            p("swingpd", "SWING PERIOD", 180.0, 10.0, 1000.0, 1.0),
            p("exitpct", "EXIT %", 10.0, 1.0, 50.0, 0.5),
        ],
    )
}

fn rsi_w(c: &[f32], n: usize) -> Vec<f32> {
    let len = c.len();
    let mut out = vec![f32::NAN; len];
    if n == 0 || len <= n {
        return out;
    }
    let (mut ag, mut al) = (0f32, 0f32);
    for i in 1..=n {
        let d = c[i] - c[i - 1];
        if d > 0.0 {
            ag += d
        } else {
            al -= d
        }
    }
    ag /= n as f32;
    al /= n as f32;
    out[n] = if al == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + ag / al) };
    for i in n + 1..len {
        let d = c[i] - c[i - 1];
        let (gn, ls) = if d > 0.0 { (d, 0.0) } else { (0.0, -d) };
        ag = (ag * (n as f32 - 1.0) + gn) / n as f32;
        al = (al * (n as f32 - 1.0) + ls) / n as f32;
        out[i] = if al == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + ag / al) };
    }
    out
}
fn stdev_n(c: &[f32], n: usize) -> Vec<f32> {
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
/// The AFL's overlapping POCL loop (`for m = Barc; m > 0; m--  for J = 1..ROCPD  if C[m+j] > C[m+j-1] POCL[m+j] = POCL[m+j-1] + 1 ...`) copied literally.
fn pocl_of(c: &[f32], rocpd: usize) -> Vec<f32> {
    let n = c.len();
    let mut pocl = vec![0f32; n];
    if n > rocpd + 1 {
        let barc = n - (rocpd + 1);
        for m in (1..=barc).rev() {
            for j in 1..rocpd + 1 {
                if m + j >= n {
                    break;
                }
                pocl[m + j] = if c[m + j] > c[m + j - 1] { pocl[m + j - 1] + 1.0 } else { pocl[m + j - 1] };
            }
        }
    }
    pocl
}

/// PositionScore per RANKING MODE (None for NONE). ROC PERIOD / NO OF POSITIVE CLOSE is `Optimize("...", 10, 10, 300, 10)` = 10.
fn rank_score(c: &[f32], mode: i32, rsipd: usize, stdpd: usize, swingpd: usize) -> Option<Vec<f32>> {
    let n = c.len();
    const ROCPD: usize = 10;
    let sdiffpct = || -> Vec<f32> {
        let hh = hhv_n(c, swingpd);
        (0..n).map(|i| (hh[i] - c[i]) / hh[i] * 100.0).collect()
    };
    let rocstd = || -> Vec<f32> {
        let (r, sd) = (roc(c, ROCPD), stdev_n(c, stdpd));
        (0..n).map(|i| r[i] / sd[i]).collect()
    };
    Some(match mode {
        2 => roc(c, ROCPD).iter().map(|&x| 100.0 + x).collect(),
        3 => roc(c, ROCPD).iter().map(|&x| 1000.0 - x).collect(),
        4 => rsi_w(c, rsipd),
        5 => stdev_n(c, stdpd),
        6 => rocstd().iter().map(|&x| 100.0 + x).collect(),
        7 => rocstd().iter().map(|&x| 100.0 - x).collect(),
        8 => pocl_of(c, ROCPD),
        9 => sdiffpct().iter().map(|&x| 100.0 - x).collect(),
        10 => {
            let (r, po) = (roc(c, ROCPD), pocl_of(c, ROCPD));
            (0..n).map(|i| (100.0 + r[i]) + po[i]).collect()
        }
        11 => {
            let (sd, po) = (sdiffpct(), pocl_of(c, ROCPD));
            (0..n).map(|i| (100.0 - sd[i]) + po[i]).collect()
        }
        _ => return None,
    })
}

fn rsi_one(s: &Series, g: G) -> Signals {
    let x = ctx(s, g("exitpct") as f32, dead_yes(g));
    let n = x.n;
    let penny = g("pennyflt") as f32;
    let tstart = g("tstart").round() as usize;
    // dbg_fine: hidden test key (0 in production): the OpenInt / fine-table turnover filter of the nearest sheet-backed AFL instead of this file's V*C / coarse table
    let turnover = if g("dbg_fine") != 0.0 { x.turnover_oi(s, turn_on(g), vol_fine) } else { x.turnover_vc(s, turn_on(g), vol_coarse) };
    let bs = x.buysig();
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let ath = bs[i] || (i > 0 && x.c[i] > x.hcl[i - 1]);
        buy[i] = ath && i >= tstart && !x.upc[i] && x.dbuy[i] && x.c[i] > penny && turnover[i];
        sell[i] = ((x.trend[i] == -1 || !bs[i]) && !x.loc[i]) || x.dsell[i];
    }
    let score = rank_score(&x.c, g("rankmode").round() as i32, g("rsipd").round() as usize, g("stdpd").round() as usize, g("swingpd").round() as usize);
    finish(&x, buy, sell, true, score, g("hardstop") as f32)
}
fn signals_rsi(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| rsi_one(s, g)).collect()
}

// ================================================================ ATHC %swing exit, universe by NSE500 / F&O list membership (Aux1 / Aux2)

fn schema_list(exit_max: f64) -> Vec<P> {
    with(
        base_schema(),
        vec![
            p("tstart", "TRADING START BAR (not used by this AFL's signals; default clamped to 1)", 1.0, 1.0, 500.0, 1.0),
            p("exitpct", "EXIT %", 10.0, 5.0, exit_max, 1.0),
            p("backtest", "BACKTEST for (1 ALL, 2 NSE500, 3 FO, 4 NSE500-FO)", 1.0, 1.0, 4.0, 1.0),
        ],
    )
}
fn schema_list35() -> Vec<P> {
    schema_list(35.0)
}
fn schema_list25() -> Vec<P> {
    schema_list(25.0)
}

/// mode 0: fresh ATH close only, exit also when the NEXT bar is outside the list (`Ref(BACKTESTCHECK,1) == False`); mode 1: running entry, %swing exit only;
/// mode 2: running entry, exit also when the next bar is not in NSE500 (`Ref(Aux1,1) == 0`, for NSE500, FO and NSE500-FO alike).
fn list_one(s: &Series, g: G, mode: u8) -> Signals {
    let x = ctx(s, g("exitpct") as f32, dead_yes(g));
    let n = x.n;
    let penny = g("pennyflt") as f32;
    let bt = g("backtest").round() as i32;
    let chk: Vec<bool> = s
        .bars
        .iter()
        .map(|b| match bt {
            2 => b.aux1 == 1.0,
            3 => b.aux2 == 1.0,
            4 => b.aux1 == 1.0 && b.aux2 == 0.0,
            _ => true,
        })
        .collect();
    let turnover = if g("dbg_fine") != 0.0 { x.turnover_oi(s, turn_on(g), vol_fine) } else { x.turnover_vc(s, turn_on(g), vol_coarse) }; // dbg_fine: hidden test key, see rsi_one
    let bs = x.buysig();
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let cash = x.dbuy[i] && !x.upc[i] && x.c[i] > penny && turnover[i] && chk[i];
        if mode == 0 {
            buy[i] = i > 0 && x.c[i] > x.hcl[i - 1] && cash;
            let gone = bt != 1 && i + 1 < n && !chk[i + 1];
            sell[i] = (x.trend[i] == -1 && !x.loc[i]) || x.dsell[i] || gone;
        } else {
            buy[i] = bs[i] && cash;
            let gone = mode == 2 && bt != 1 && i + 1 < n && s.bars[i + 1].aux1 == 0.0;
            sell[i] = ((x.trend[i] == -1 || !bs[i]) && !x.loc[i]) || x.dsell[i] || gone;
        }
    }
    finish(&x, buy, sell, true, None, 0.0)
}
fn signals_list_new(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| list_one(s, g, 0)).collect()
}
fn signals_list_run(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| list_one(s, g, 1)).collect()
}
fn signals_list_in(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| list_one(s, g, 2)).collect()
}

// ================================================================ ATHC buy, %swing exit, market-cap class by turnover rank switched by the monthly price-relative EMA

fn schema_cap(pd_min: f64, pd_max: f64, pd_step: f64) -> Vec<P> {
    with(
        base_schema(),
        vec![
            hard_p(),
            p("exitpct", "EXIT %", 10.0, 5.0, 25.0, 1.0),
            p("prtimeframe", "TIMEFRAME FOR PR CALCULATION (1 MONTHLY, 2 WEEKLY, 3 DAILY; the AFL always compresses monthly, the list is not read)", 1.0, 1.0, 3.0, 1.0),
            p("period", "EMA PERIOD FOR PR", 10.0, 1.0, 12.0, 1.0),
            p("turnoverpd", "TURNOVER AVG PERIOD FOR MARKET", 60.0, pd_min, pd_max, pd_step),
        ],
    )
}
fn schema_cap_ms() -> Vec<P> {
    schema_cap(5.0, 500.0, 1.0)
}
fn schema_cap_lm() -> Vec<P> {
    schema_cap(20.0, 200.0, 10.0)
}

/// `INDEX = TimeFrameCompress(Foreign(name, "C"), inMonthly)` as monthly arrays (Null as NaN) on the stock's own dates, with the compression map.
fn monthly_close(ix: Option<&Series>, dates: &[i32]) -> (Vec<f32>, apx::Comp) {
    match ix {
        Some(ix) => {
            let al = apx::align(ix, dates);
            let k = apx::compress(dates, &al, 0);
            let c = k.c.iter().map(|&v| if apx::is_null(v) { f32::NAN } else { v }).collect();
            (c, k)
        }
        None => {
            let al = apx::align(&Series { name: String::new(), bars: vec![] }, dates);
            let k = apx::compress(dates, &al, 0);
            (vec![f32::NAN; k.start.len()], k)
        }
    }
}

/// kind 0: NSESMLCAP100 / NSEMIDCAP, EPR > CLPR -> 101 < rank < 251, else 250 < rank < 801.   kind 1: NSEMIDCAP / NSE100, EPR > CLPR -> rank < 101, else 100 < rank < 251
/// (adds `AND turnover`).   kind 2: NSESMLCAP100 / NSE100, EPR > CLPR -> rank < 101, else 250 < rank < 801.
fn cap_run(series: &[Series], env: &Env, g: G, kind: u8) -> Vec<Signals> {
    let (n1, n2) = match kind {
        0 => ("NSESMLCAP100", "NSEMIDCAP"),
        1 => ("NSEMIDCAP", "NSE100"),
        _ => ("NSESMLCAP100", "NSE100"),
    };
    let pd = g("turnoverpd").round().max(1.0) as usize;
    let period = g("period").round().max(1.0) as usize;
    let dy = dead_yes(g);
    let exit_pct = g("exitpct") as f32;
    let xs: Vec<Ctx> = series.iter().map(|s| ctx(s, exit_pct, dy)).collect();
    let turn: Vec<Vec<f32>> = xs.iter().map(|x| ma_f(&x.oi, pd)).collect();
    // TURNOVER = IIf(deadstock == 0, Ref(MA(OpenInt, pd), -1), 0)
    let vals: Vec<Vec<f32>> = xs.iter().zip(&turn).map(|(x, t)| { let l = lag(t, 1); (0..x.n).map(|i| if x.dead1[i] { 0.0 } else { l[i] }).collect() }).collect();
    let rk = ranks_by_date(series, &vals);
    let (penny, hard, tf) = (g("pennyflt") as f32, g("hardstop") as f32, turn_on(g));
    series
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let x = &xs[k];
            let n = x.n;
            let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
            let (c1, km) = monthly_close(env.get(n1), &dates);
            let (c2, _) = monthly_close(env.get(n2), &dates);
            let m = c1.len();
            let closepr: Vec<f32> = (0..m).map(|j| c1[j] / c2[j]).collect();
            let (e1, e2) = (ema_n(&c1, period), ema_n(&c2, period));
            let emapr: Vec<f32> = (0..m).map(|j| e1[j] / e2[j]).collect();
            let nn = |v: Vec<f32>| -> Vec<f32> { v.iter().map(|&q| if apx::is_null(q) { f32::NAN } else { q }).collect() };
            let clpr = nn(apx::expand(&closepr, &km, n));
            let epr = nn(apx::expand(&emapr, &km, n));
            let bs = x.buysig();
            let turnover = x.turnover_oi(s, tf, vol_fine);
            let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
            for i in 0..n {
                let r = rk[k][i];
                let cond = if gt(epr[i], clpr[i]) {
                    match kind {
                        0 => gt(r, 101.0) && lt(r, 251.0),
                        _ => lt(r, 101.0),
                    }
                } else {
                    match kind {
                        1 => gt(r, 100.0) && lt(r, 251.0),
                        _ => gt(r, 250.0) && lt(r, 801.0),
                    }
                };
                buy[i] = bs[i] && x.dbuy[i] && x.c[i] > penny && !x.dead1[i] && cond && !x.uc[i] && (kind != 1 || turnover[i]);
                sell[i] = (!bs[i] && !x.lc[i]) || x.dsell[i] || x.dead1[i];
            }
            // PositionScore = Ref(turn, -1)
            finish(x, buy, sell, true, Some(lag(&turn[k], 1)), hard)
        })
        .collect()
}
fn signals_cap_ms(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    cap_run(series, env, g, 0)
}
fn signals_cap_lm(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    cap_run(series, env, g, 1)
}
fn signals_cap_ls(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    cap_run(series, env, g, 2)
}

// ================================================================ ROC(30) ranking entry (top-ROC watchlist stocks), hard SL + profit target exit, no ATH

fn schema_roc30() -> Vec<P> {
    with(
        base_schema(),
        vec![
            p("bufferpct", "Buffer %", 0.25, 0.0, 10.0, 0.05),
            p("profit_pct", "Profit Booking %", 10.0, 0.0, 30.0, 0.1),
            p("hardsl_pct", "Hard SL %", 2.0, 0.0, 30.0, 0.01),
            p("roc_prds", "ROC Periods", 30.0, 1.0, 100.0, 1.0),
            p("roc_val", "ROC Value (the AFL default 0 lies outside its range 1-500: clamped to 1)", 1.0, 1.0, 500.0, 1.0),
        ],
    )
}

fn signals_roc30(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let prds = g("roc_prds").round().max(1.0) as usize;
    let val = g("roc_val") as f32;
    let (penny, buf) = (g("pennyflt") as f32, g("bufferpct") as f32);
    let (profit, hardsl) = (g("profit_pct") as f32, g("hardsl_pct") as f32);
    let dy = dead_yes(g);
    let xs: Vec<Ctx> = series.iter().map(|s| ctx(s, 10.0, dy)).collect();
    let rocs: Vec<Vec<f32>> = xs.iter().map(|x| roc(&x.c, prds)).collect();
    let rk = ranks_by_date(series, &rocs);
    series
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let x = &xs[k];
            let n = x.n;
            let turnover = x.turnover_vc(s, turn_on(g), vol_fine);
            let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
            let mut sp = x.close_px();
            let (mut pos, mut hsl, mut pt) = (false, 0f32, 9_999_999f32);
            for i in 1..n {
                let bf = !x.uc[i] && x.c[i] > penny && turnover[i] && x.dbuy[i] && !x.dead1[i];
                if !pos && gt(rocs[k][i], val) && bf {
                    buy[i] = true;
                    if hardsl > 0.0 {
                        hsl = (1.0 - hardsl * 0.01) * x.c[i];
                    }
                    if profit > 0.0 {
                        pt = (1.0 + profit * 0.01) * x.c[i];
                    }
                    pos = true;
                } else if pos && (((x.l[i] <= hsl || x.h[i] >= pt) && !x.lc[i]) || x.dsell[i] || x.dead1[i]) {
                    sell[i] = true;
                    sp[i] = if x.l[i] <= hsl {
                        if x.o[i] <= hsl { x.o[i] - (x.o[i] * buf / 100.0) } else { hsl }
                    } else if x.h[i] >= pt {
                        if x.o[i] >= pt { x.o[i] - (x.o[i] * buf / 100.0) } else { pt }
                    } else {
                        x.c[i]
                    };
                    sp[i] = sp[i].min(x.h[i]).max(x.l[i]);
                    pos = false;
                    hsl = 0.0;
                    pt = 9_999_999.0;
                }
            }
            let px = x.close_px();
            let mut sg = Signals::new(buy, sell, px, sp);
            // PositionScore = 10000 - rank
            sg.score = Some((0..n).map(|i| 10000.0 - rk[k][i]).collect());
            sg
        })
        .collect()
}

// ================================================================ the variants

pub fn variants() -> Vec<VariantDef> {
    let d = Data::Cash22 { watchlist: "NSE 500 C&P.tls" };
    vec![
        VariantDef { id: "ytath_and_pctswing_with_fixed_atr_scale_in", data: d, schema: schema_scale, signals: signals_scale },
        VariantDef { id: "ytath_and_pctswing_with_roc_30_and_roc_365", data: d, schema: schema_roc365, signals: signals_roc365 },
        VariantDef { id: "ytath_and_pctswing_with_roc_30_and_roc_365_2", data: d, schema: schema_roc365_2, signals: signals_roc365_2 },
        VariantDef { id: "ytath_buy_and_pctswing_exit_with_roc_6", data: d, schema: schema_roc6, signals: signals_roc6 },
        VariantDef { id: "ytath_buy_and_pctswing_exit_with_running", data: d, schema: schema_running, signals: signals_running },
        VariantDef { id: "ytath_entry_and_pctswing_exit_with_atr_pct", data: d, schema: schema_atrpct, signals: signals_atrpct },
        VariantDef { id: "ytath_entry_and_pctswing_exit_with_ranking", data: d, schema: schema_ranking, signals: signals_ranking },
        VariantDef { id: "ytath_entry_and_pctswing_exit_with_ranking_2", data: d, schema: schema_ranking_2, signals: signals_ranking_2 },
        VariantDef { id: "ytath_entry_when_turnover_is_double_the", data: d, schema: schema_double, signals: signals_double },
        VariantDef { id: "ytath_pctswing_exit_with_average_turnover", data: d, schema: schema_avg1, signals: signals_avg1 },
        VariantDef { id: "ytath_pctswing_exit_with_average_turnover_2", data: d, schema: schema_avg1, signals: signals_avg2 },
        VariantDef { id: "ytath_pctswing_exit_with_average_turnover_3", data: d, schema: schema_avg3, signals: signals_avg3 },
        VariantDef { id: "ytath_pctswing_exit_with_average_turnover_5", data: d, schema: schema_avg5, signals: signals_avg5 },
        VariantDef { id: "ytath_pctswing_exit_with_ranking_roc_rsi", data: d, schema: schema_rsi, signals: signals_rsi },
        VariantDef { id: "ytath_pctswing_exit_take_new_ath_only_exit", data: d, schema: schema_list35, signals: signals_list_new },
        VariantDef { id: "ytath_pctswing_exit_running_entry_and", data: d, schema: schema_list25, signals: signals_list_run },
        VariantDef { id: "ytath_pctswing_exit_running_entry_exit_in", data: d, schema: schema_list25, signals: signals_list_in },
        VariantDef { id: "ytath_buy_pctswing_exit_mid_small_cap_by", data: d, schema: schema_cap_ms, signals: signals_cap_ms },
        VariantDef { id: "ytath_buy_pctswing_exit_large_mid_cap_by", data: d, schema: schema_cap_lm, signals: signals_cap_lm },
        VariantDef { id: "ytath_buy_pctswing_exit_large_small_cap_by", data: d, schema: schema_cap_ms, signals: signals_cap_ls },
        VariantDef { id: "ytath_roc_30_ranking_entry_top_roc", data: d, schema: schema_roc30, signals: signals_roc30 },
    ]
}

// ---------------------------------------------------------------- index helpers verified in ath_plus.rs / ath_loss2.rs (copied: Foreign alignment, calendar compression, TimeFrameExpand)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(date: i32, o: f32, h: f32, l: f32, c: f32, v: f32, oi: f32) -> Bar {
        Bar { date, c, o, h, l, v, oi, aux1: 1.0, aux2: 0.0 }
    }
    /// a series from closes: O = previous close, H = max + 0.5, L = min - 0.5, big volume / turnover
    fn series(name: &str, closes: &[f32]) -> Series {
        let bars = closes
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                let o = if i == 0 { c } else { closes[i - 1] };
                bar(20100101 + i as i32, o, o.max(c) + 0.5, o.min(c) - 0.5, c, 1.0e7, 1.0e9)
            })
            .collect();
        Series { name: name.into(), bars }
    }
    fn reader(schema: Vec<P>, ov: &[(&str, f64)]) -> impl Fn(&str) -> f64 {
        let ov: Vec<(String, f64)> = ov.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        move |k: &str| {
            if let Some((_, v)) = ov.iter().find(|(kk, _)| kk == k) {
                return *v;
            }
            schema.iter().find(|q| q.key == k).map(|q| q.def.clamp(q.min, q.max)).unwrap_or(0.0)
        }
    }
    /// 40 rising closes, then a 30 % drop (breaks any swing line up to 25 %), then a recovery
    fn rise_fall() -> Vec<f32> {
        let mut c: Vec<f32> = (0..40).map(|i| 100.0 + i as f32).collect();
        c.extend((0..10).map(|i| 100.0 - 3.0 * i as f32));
        c.extend((0..30).map(|i| 80.0 + 2.0 * i as f32));
        c
    }

    #[test]
    fn every_variant_has_a_schema_default_inside_its_range() {
        for v in variants() {
            for q in (v.schema)() {
                assert!(q.def >= q.min && q.def <= q.max, "{}: {} default {} outside {}..{}", v.id, q.key, q.def, q.min, q.max);
            }
        }
        assert_eq!(variants().len(), 21);
    }

    #[test]
    fn ranking_entry_buys_the_fresh_ath_close_and_sells_on_the_swing_break_at_the_close() {
        let s = series("A", &rise_fall());
        let g = reader(schema_ranking(), &[]);
        let sg = entry_rank_one(&s, &g, Rk::Roc);
        assert!(sg.buy[1], "the first close above all earlier closes buys");
        let t = trades_of(&sg);
        assert!(!t.is_empty() && t[0].sell_i != usize::MAX, "the 21 % swing line is broken by the drop");
        assert_eq!(sg.buy_price[t[0].buy_i], s.bars[t[0].buy_i].c);
        assert!(sg.score.as_ref().unwrap()[20].is_finite(), "PositionScore = Ref(ROC(C,12),-1)");
        assert!(sg.score.as_ref().unwrap()[5].is_nan(), "Null while the ROC has no history");
    }

    #[test]
    fn atr_pct_gate_blocks_a_volatile_stock() {
        // closes alternate +-10 % around a rising trend: ATR% far above 8 -> no entry; calm series enters
        let wild: Vec<f32> = (0..60).map(|i| 100.0 + i as f32 * 2.0 + if i % 2 == 0 { 10.0 } else { -10.0 }).collect();
        let calm: Vec<f32> = (0..60).map(|i| 100.0 + i as f32).collect();
        let g = reader(schema_atrpct(), &[]);
        assert!(!entry_rank_one(&series("W", &wild), &g, Rk::AtrPct).buy.iter().any(|&b| b));
        assert!(entry_rank_one(&series("C", &calm), &g, Rk::AtrPct).buy.iter().any(|&b| b));
    }

    #[test]
    fn double_turnover_entry_needs_a_turnover_spike() {
        let closes: Vec<f32> = (0..60).map(|i| 100.0 + i as f32).collect();
        let mut s = series("D", &closes);
        let g = reader(schema_double(), &[("avgturnover_multi", 4.0)]);
        // constant turnover: OI > 4 x average never holds
        assert!(!double_one(&s, &g).buy.iter().any(|&b| b));
        // a spike of 10 x on bar 30 (a fresh ATH close) enters on that bar
        s.bars[30].oi *= 10.0;
        let sg = double_one(&s, &g);
        assert!(sg.buy[30]);
    }

    #[test]
    fn roc_running_state_buys_a_missed_ath_when_the_roc_later_passes() {
        // slow start: the ROC(180) filter cannot pass at the first ATH closes; build a series whose ROC reaches > 100 % later while still trending
        let mut c: Vec<f32> = (0..200).map(|i| 100.0 + 0.1 * i as f32).collect();
        c.extend((0..120).map(|i| 120.0 * (1.0 + 0.012 * i as f32)));
        let s = series("R", &c);
        let g = reader(schema_running(), &[("roc_val", 30.0), ("roc_prd", 60.0)]);
        let sg = roc6_one(&s, &g, true);
        assert!(sg.buy.iter().any(|&b| b), "the running state arms a buy once the ROC passes");
        let first = sg.buy.iter().position(|&b| b).unwrap();
        let myroc = roc(&s.close(), 60);
        assert!(myroc[first] > 30.0 || first > 0, "entry only when ROC passes or via state 2");
        // plain file: entry only on an ATH bar with the ROC above the value
        let g2 = reader(schema_roc6(), &[("roc_val", 30.0), ("roc_prd", 60.0)]);
        let sg2 = roc6_one(&s, &g2, false);
        let c2 = s.close();
        let hc = (0..c2.len()).map(|i| c2[..=i].iter().cloned().fold(f32::MIN, f32::max)).collect::<Vec<_>>();
        for i in 1..c2.len() {
            if sg2.buy[i] {
                assert!(c2[i] > hc[i - 1] && myroc[i] > 30.0);
            }
        }
    }

    #[test]
    fn roc_ranking_picks_the_strongest_symbols_and_the_exit_follows_the_swing() {
        let g = reader(schema_roc365_2(), &[("roc_slow_prds", 20.0), ("roc_30_rank_prds", 1.0), ("roc_365_rank_prds", 1.0), ("roc_fast_prds", 5.0)]);
        let strong: Vec<f32> = (0..80).map(|i| 100.0 + 2.0 * i as f32).collect();
        let weak: Vec<f32> = (0..80).map(|i| 100.0 + 1.0 * i as f32).collect();
        let ss = vec![series("A", &strong), series("B", &weak)];
        let r = roc365(&ss, &g, false);
        // rank1 = highest 5-day ROC (A), rank2 = lowest 20-day ROC = highest of -ROC(20): both ranks must be <= 1 -> impossible for one symbol
        // (A has the best ROC5 but the worst -ROC20) so nothing is bought, whichever symbol
        assert!(!r[0].buy.iter().any(|&b| b) && !r[1].buy.iter().any(|&b| b));
        let g2 = reader(schema_roc365_2(), &[("roc_slow_prds", 20.0), ("roc_30_rank_prds", 1.0), ("roc_365_rank_prds", 2.0), ("roc_fast_prds", 5.0)]);
        let r2 = roc365(&ss, &g2, false);
        assert!(r2[0].buy.iter().any(|&b| b), "A is rank 1 on ROC5 and within rank 2 on -ROC20");
        assert!(!r2[1].buy.iter().any(|&b| b), "B is rank 2 on ROC5");
    }

    #[test]
    fn rank_band_selects_the_class_by_average_turnover() {
        // two symbols: A with the larger turnover (rank 1), B rank 2; LARGECAP band is rank < 101 for both, MID needs rank > 100 for neither
        let a = series("A", &(0..60).map(|i| 100.0 + i as f32).collect::<Vec<_>>());
        let mut b = series("B", &(0..60).map(|i| 50.0 + i as f32).collect::<Vec<_>>());
        for x in b.bars.iter_mut() {
            x.oi = 5.0e8;
        }
        let ss = vec![a, b];
        let g = reader(schema_avg3(), &[("turnoverpd", 5.0)]);
        let large = avg_run(&ss, &g, BANDS_VER1);
        assert!(large[0].buy.iter().any(|&q| q) && large[1].buy.iter().any(|&q| q));
        let g2 = reader(schema_avg3(), &[("turnoverpd", 5.0), ("bkmkt", 2.0)]);
        let mid = avg_run(&ss, &g2, BANDS_VER1);
        assert!(!mid[0].buy.iter().any(|&q| q) && !mid[1].buy.iter().any(|&q| q), "MIDCAP needs 100 < rank < 251");
        // the micro band of file 1 ends at rank 1001, of file 2 at 801, of file 3 is open-ended
        assert_eq!(BANDS_1000[3], (500.0, 1001.0));
        assert_eq!(BANDS_800[3], (500.0, 801.0));
        assert_eq!(BANDS_VER1[3], (800.0, 80000.0));
    }

    #[test]
    fn avg5_percent_hard_sl_exits_on_a_close_below_the_level_and_lower_circuit_defers() {
        // 100 falls to 90, a new ATH close of 101 enters at the close, then 100, 98, 98.5: the first close at or below 101 x 0.98 = 98.98 sells
        let mut c: Vec<f32> = (0..11).map(|i| 100.0 - i as f32).collect();
        c.extend([101.0, 100.0, 98.0, 98.5, 99.5, 100.5]);
        let ss = vec![series("S", &c)];
        let g = reader(schema_avg5(), &[("ver", 2.0), ("hardslpct", 2.0), ("turnoverpd", 5.0), ("exitpct", 22.0)]);
        let sg = &signals_avg5(&ss, &Env::empty(), &g)[0];
        let t = trades_of(sg);
        assert!(!t.is_empty());
        assert_eq!(t[0].buy_i, 11);
        assert_eq!(t[0].sell_i, 13, "exit on the 2 % hard SL (close 98 <= 98.98)");
        // with SL OFF the same series stays in the trade (no swing break)
        let g0 = reader(schema_avg5(), &[("ver", 1.0), ("turnoverpd", 5.0), ("exitpct", 22.0)]);
        let sg0 = &signals_avg5(&ss, &Env::empty(), &g0)[0];
        assert!(trades_of(sg0).iter().all(|e| e.sell_i == usize::MAX));
    }

    #[test]
    fn ranking_modes_give_a_score_and_none_gives_none() {
        let c: Vec<f32> = (0..90).map(|i| 100.0 + (i % 9) as f32 + i as f32 * 0.3).collect();
        for m in 2..=11 {
            let sc = rank_score(&c, m, 14, 14, 20).unwrap();
            assert_eq!(sc.len(), 90);
            assert!(sc[89].is_finite(), "mode {m}");
        }
        assert!(rank_score(&c, 1, 14, 14, 20).is_none());
        let po = pocl_of(&(0..30).map(|i| if i % 2 == 0 { 10.0 } else { 11.0 }).collect::<Vec<f32>>(), 10);
        assert_eq!(po[29], 5.0);
    }

    #[test]
    fn list_membership_limits_entries_and_the_exit_in_file_leaves_when_the_stock_drops_out() {
        let mut s = series("L", &(0..60).map(|i| 100.0 + i as f32).collect::<Vec<_>>());
        for i in 0..60 {
            s.bars[i].aux1 = if i < 40 { 1.0 } else { 0.0 };
            s.bars[i].aux2 = 0.0;
        }
        let all = reader(schema_list25(), &[("backtest", 1.0)]);
        assert!(trades_of(&list_one(&s, &all, 1)).iter().all(|e| e.sell_i == usize::MAX), "ALL: never leaves");
        let n500 = reader(schema_list25(), &[("backtest", 2.0)]);
        let sg = list_one(&s, &n500, 2);
        let t = trades_of(&sg);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].sell_i, 39, "Ref(Aux1,1)==0 on the bar before the first bar outside the list");
        let fo = reader(schema_list25(), &[("backtest", 3.0)]);
        assert!(!list_one(&s, &fo, 1).buy.iter().any(|&b| b), "no F&O flag -> no entry");
        // 'take new ATH only' leaves through Ref(BACKTESTCHECK,1)==False as well
        let t0 = trades_of(&list_one(&s, &reader(schema_list35(), &[("backtest", 2.0)]), 0));
        assert_eq!(t0.len(), 1);
        assert_eq!(t0[0].sell_i, 39);
    }

    #[test]
    fn price_relative_switch_picks_the_band_and_ema_is_null_safe() {
        let e = ema_n(&[f32::NAN, f32::NAN, 10.0, 12.0], 3);
        assert!(e[1].is_nan() && e[2] == 10.0 && (e[3] - 11.0).abs() < 1e-6);
        // monthly compression: the last close of the month, expanded back to the last day of the month
        let dates: Vec<i32> = vec![20100128, 20100129, 20100201, 20100202];
        let ix = Series { name: "I".into(), bars: dates.iter().enumerate().map(|(i, &d)| bar(d, 1.0, 1.0, 1.0, 1.0 + i as f32, 1.0, 1.0)).collect() };
        let (c, k) = monthly_close(Some(&ix), &dates);
        assert_eq!(c, vec![2.0, 4.0]);
        let ex = apx::expand(&c, &k, 4);
        assert_eq!(ex[1], 2.0);
        assert_eq!(ex[2], 2.0, "the first days of the next month still show the finished month");
    }

    #[test]
    fn cap_run_buys_only_inside_the_band_chosen_by_the_price_relative() {
        // no index in the Env: EPR/CLPR are Null, EPR > CLPR is false, so the 'else' band applies (mid_small: 250 < rank < 801)
        let a = series("A", &(0..60).map(|i| 100.0 + i as f32).collect::<Vec<_>>());
        let ss = vec![a];
        let g = reader(schema_cap_ms(), &[("turnoverpd", 5.0)]);
        let r = cap_run(&ss, &Env::empty(), &g, 0);
        assert!(!r[0].buy.iter().any(|&b| b), "rank 1 is not in 250 < rank < 801");
        let r2 = cap_run(&ss, &Env::empty(), &reader(schema_cap_lm(), &[("turnoverpd", 20.0)]), 1);
        assert!(!r2[0].buy.iter().any(|&b| b), "large_mid else band is 100 < rank < 251");
    }

    #[test]
    fn roc30_exit_uses_the_hard_sl_and_profit_target_prices() {
        // enters on bar 1 (ROC(1) > 1 %), the next bar gaps through the 2 % hard SL: sold at the open less the buffer (clamped into the bar)
        let mut s = series("Q", &[100.0, 103.0, 103.0, 103.0]);
        s.bars[2] = bar(20100103, 103.0, 103.5, 99.0, 100.0, 1.0e7, 1.0e9);
        let ss = vec![s];
        let g = reader(schema_roc30(), &[("roc_prds", 1.0), ("hardsl_pct", 2.0), ("profit_pct", 10.0), ("deadstock", 1.0)]);
        let sg = &signals_roc30(&ss, &Env::empty(), &g)[0];
        assert!(sg.buy[1] && sg.sell[2]);
        let hsl = (1.0 - 2.0f32 * 0.01) * 103.0;
        assert!(ss[0].bars[2].o > hsl, "no gap: the stop level itself is the price");
        assert!((sg.sell_price[2] - hsl).abs() < 1e-3);
        assert!(sg.score.is_some());
    }

    #[test]
    fn fine_table_swap_in_the_scale_in_file_keeps_the_plain_engine_without_the_filter() {
        let s = series("F", &rise_fall());
        let ss = vec![s];
        let off = reader(schema_scale(), &[]);
        let a = &signals_scale(&ss, &Env::empty(), &off)[0];
        let plain = crate::ath_loss2::ScaleP { exit_pct: 10.0, losspct: 0.0, noofloss: 0.0, comm: 0.65, penny: 0.0, turnflt: false, dead_yes: false, atr_prd: 14, myatr1: 1.0, buffer: 0.25, atr_mode: 0, plain: true };
        let b = crate::ath_loss2::scalein_signals(&ss[0], &plain);
        assert_eq!(a.buy, b.buy);
        assert_eq!(a.sell, b.sell);
        // with the turnover filter on and a huge turnover everything passes the fine table too; with a tiny volume nothing is bought
        let on = reader(schema_scale(), &[("turnflt", 1.0)]);
        assert_eq!(signals_scale(&ss, &Env::empty(), &on)[0].buy, b.buy);
        let mut thin = ss[0].bars.clone();
        for x in thin.iter_mut() {
            x.v = 1.0;
        }
        let ts = vec![Series { name: "T".into(), bars: thin }];
        let thin_buys = signals_scale(&ts, &Env::empty(), &on)[0].buy.clone();
        assert_ne!(thin_buys, b.buy, "the turnover filter changes the entries (the skipped first tranche is bought later as a scale-in)");
    }
}
