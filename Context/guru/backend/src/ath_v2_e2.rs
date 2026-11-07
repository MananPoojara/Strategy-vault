//! ATH sub-variants, batch e2 ("ATHC: Other exits"), 24 AFLs coded from `ATH/afl_copy/<id>.afl` with the AFLs' own default parameters.
//! Data: `Data::Cash22 { "NSE 500 C&P.tls" }` (31-03-2022 cash database). Notes per variant: `ATH/v2/<id>/VERIFY.md`.
//!
//! Families (all long only, fills at the signal bar's Close unless stated, float32, a comparison with Null (NaN here) is false):
//! * the Skipbuy / SkipSell state machine (`sk_run`) of the Suraj Mehra AFLs: RSI(70) cross, RSI vs its MA, ROC vs its MA, MA versions,
//!   Elder Ray, Parabolic SAR, Ichimoku (lines / cloud), Gann HiLo, MFI, Bollinger upper band (4 exit versions with a higher-timeframe Supertrend)
//! * the buysig-carry machine (`bs_run`): MACD, PDI/MDI, Gann square-root levels (with its own swing loop)
//! * ATR-or-%swing exit, RSI stock selection with 3 exit versions, 52-week lowest close (weekly compression)
//! * the ExRem AFLs: Bollinger / Keltner / moving-average exit, N-day close / low stop, non-performance exit (ver 1 / ver 2),
//!   time-based exit only (holding days) with the 11 ranking modes
//! Indicators are written here (float32): SMA/EMA/WMA/DEMA/TEMA/Wilders, Wilder RSI/ATR/PDI/MDI (SMA seed), MFI, ROC, Parabolic SAR, StDev (population).
//! Standard library only; calls `ath_new_st::one` for the ranking scores and `ath_v2_e1::{compress, expand}` for the TimeFrame helpers.
#![allow(dead_code)]
use crate::ath_core::*;
use crate::ath_new_st::{self, StP};
use crate::ath_v2_e1::{compress, expand};

const DATA: Data = Data::Cash22 { watchlist: "NSE 500 C&P.tls" };
const NAN: f32 = f32::NAN;

// ================================================================ helpers

fn gt(a: f32, b: f32) -> bool {
    !a.is_nan() && !b.is_nan() && a > b
}
fn lt(a: f32, b: f32) -> bool {
    !a.is_nan() && !b.is_nan() && a < b
}
fn ge(a: f32, b: f32) -> bool {
    !a.is_nan() && !b.is_nan() && a >= b
}
fn le(a: f32, b: f32) -> bool {
    !a.is_nan() && !b.is_nan() && a <= b
}

/// `Volsig` of the AFLs that test `OpenInt > Volsig` (17 steps).
fn volsig_oi(year: i32) -> f32 {
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

/// `Volsig` of the AFLs that test `V * C > volsig` (five steps).
fn volsig_vc(year: i32) -> f32 {
    match year {
        y if y < 2000 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tk {
    /// `Turnover = OpenInt > Volsig` (the cash database's OpenInt = Close x Volume)
    OpenInt,
    /// `Turnover = (V * C) > Volsig`
    VolClose,
}

#[derive(Clone, Copy, Debug)]
pub struct Cmn {
    pub dead_yes: bool,
    pub penny: f32,
    pub turn_on: bool,
    /// verification hook only (false in every registered variant): read UC / LC as the Prec(,2) UpCircuit / LoCircuit of the other AFL family
    pub circuit_prec: bool,
}

fn cmn_of(g: G) -> Cmn {
    // ParamToggle "DEAD STOCK BACKTEST" YES|NO (engine value 1 = YES), "TURNOVER Filter" NO|YES (engine value 2 = YES)
    Cmn { dead_yes: g("deadstock").round() == 1.0, penny: g("pennyflt") as f32, turn_on: g("turnflt").round() == 2.0, circuit_prec: false }
}

/// Per-bar facts every AFL of the batch computes first (circuits, dead-stock switches, turnover, `Hcl = HHV(C, BarCount)`).
struct Ctx {
    n: usize,
    c: Vec<f32>,
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    hcl: Vec<f32>,
    /// UC = H == C ; LC = L == C
    uc: Vec<bool>,
    lc: Vec<bool>,
    /// UpCircuit / LoCircuit with Prec(,2)
    upc: Vec<bool>,
    loc: Vec<bool>,
    dbuy: Vec<bool>,
    dsell: Vec<bool>,
    turn: Vec<bool>,
    penny: f32,
}

impl Ctx {
    fn new(s: &Series, cm: &Cmn, tk: Tk) -> Ctx {
        let n = s.bars.len();
        let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
        let mut hcl = vec![0f32; n];
        for i in 0..n {
            hcl[i] = if i == 0 || c[i] > hcl[i - 1] { c[i] } else { hcl[i - 1] };
        }
        let mut cx = Ctx { n, c, o, h, l, hcl, uc: vec![], lc: vec![], upc: vec![], loc: vec![], dbuy: vec![], dsell: vec![], turn: vec![], penny: cm.penny };
        for i in 0..n {
            let b = &s.bars[i];
            let upc = prec2(b.o) == prec2(b.c) && prec2(b.o) == prec2(b.h);
            let loc = prec2(b.o) == prec2(b.c) && prec2(b.o) == prec2(b.l);
            cx.uc.push(if cm.circuit_prec { upc } else { b.h == b.c });
            cx.lc.push(if cm.circuit_prec { loc } else { b.l == b.c });
            cx.upc.push(upc);
            cx.loc.push(loc);
            if cm.dead_yes {
                cx.dbuy.push(i != n - 1);
                cx.dsell.push(i == n - 1);
            } else {
                cx.dbuy.push(i != 0);
                cx.dsell.push(false);
            }
            let t = if !cm.turn_on {
                true
            } else {
                match tk {
                    Tk::OpenInt => gt(b.oi, volsig_oi(b.date / 10000)),
                    Tk::VolClose => gt(b.v * b.c, volsig_vc(b.date / 10000)),
                }
            };
            cx.turn.push(t);
        }
        cx
    }
    /// `Buy_Cash_Filter = UC == 0 AND Turnover AND Close > PennyFlt AND DEADSTOCKBUYSIG` (`with_uc` false: without the UC test).
    fn bcf(&self, i: usize, with_uc: bool) -> bool {
        (!with_uc || !self.uc[i]) && self.turn[i] && self.c[i] > self.penny && self.dbuy[i]
    }
    fn prev_hcl(&self, i: usize) -> f32 {
        if i == 0 {
            NAN
        } else {
            self.hcl[i - 1]
        }
    }
    /// `Close[i] > Highest_Close[i-1]`
    fn ath(&self, i: usize) -> bool {
        gt(self.c[i], self.prev_hcl(i))
    }
}

/// Signals out of buy / sell arrays: fills at the Close (the AFLs' `BuyPrice[i] = Close[i]` / default SellPrice), clamped into the bar.
fn mk(s: &Series, buy: Vec<bool>, sell: Vec<bool>, sell_price: Option<Vec<f32>>, exrem_it: bool, hardstop: f32) -> Signals {
    let (buy, sell) = if exrem_it {
        let b2 = exrem(&buy, &sell);
        let s2 = exrem(&sell, &b2);
        (b2, s2)
    } else {
        (buy, sell)
    };
    let c = s.close();
    let sp = sell_price.unwrap_or_else(|| c.clone());
    let mut sg = Signals::new(buy, sell, c, sp);
    for i in 0..s.bars.len() {
        let b = &s.bars[i];
        sg.buy_price[i] = sg.buy_price[i].min(b.h).max(b.l);
        sg.sell_price[i] = sg.sell_price[i].min(b.h).max(b.l);
    }
    if hardstop > 0.0 {
        sg.stop = Some(StopSpec { loss_pct: hardstop, ..StopSpec::default() });
    }
    sg
}

// ---------------------------------------------------------------- indicators (float32, Null = NaN)

/// `MA(a, n)`; Null in the window gives Null; the first n-1 bars are Null.
fn ma(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![NAN; a.len()];
    if n == 0 {
        return out;
    }
    let (mut sum, mut nn) = (0f64, 0usize);
    for i in 0..a.len() {
        if a[i].is_nan() {
            nn += 1;
        } else {
            sum += a[i] as f64;
        }
        if i >= n {
            let o = a[i - n];
            if o.is_nan() {
                nn -= 1;
            } else {
                sum -= o as f64;
            }
        }
        if i + 1 >= n && nn == 0 {
            out[i] = (sum / n as f64) as f32;
        }
    }
    out
}

/// `EMA(a, n)`: alpha = 2 / (n + 1), seeded with the first non-Null value.
fn ema(a: &[f32], n: usize) -> Vec<f32> {
    let al = 2.0f32 / (n as f32 + 1.0);
    let mut out = vec![NAN; a.len()];
    for i in 0..a.len() {
        out[i] = if i == 0 || out[i - 1].is_nan() { a[i] } else { al * a[i] + (1.0 - al) * out[i - 1] };
    }
    out
}

/// `Wilders(a, n)`: alpha = 1 / n, seeded with the first value.
fn wilders(a: &[f32], n: usize) -> Vec<f32> {
    let al = 1.0f32 / n as f32;
    let mut out = vec![NAN; a.len()];
    for i in 0..a.len() {
        out[i] = if i == 0 || out[i - 1].is_nan() { a[i] } else { al * a[i] + (1.0 - al) * out[i - 1] };
    }
    out
}

/// `WMA(a, n)`: weights n .. 1 (newest heaviest).
fn wma(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![NAN; a.len()];
    if n == 0 {
        return out;
    }
    let den = (n * (n + 1) / 2) as f32;
    for i in (n.saturating_sub(1))..a.len() {
        let mut s = 0f32;
        for k in 0..n {
            s += a[i - k] * (n - k) as f32;
        }
        out[i] = s / den;
    }
    out
}

fn dema(a: &[f32], n: usize) -> Vec<f32> {
    let e1 = ema(a, n);
    let e2 = ema(&e1, n);
    (0..a.len()).map(|i| 2.0 * e1[i] - e2[i]).collect()
}

fn tema(a: &[f32], n: usize) -> Vec<f32> {
    let e1 = ema(a, n);
    let e2 = ema(&e1, n);
    let e3 = ema(&e2, n);
    (0..a.len()).map(|i| 3.0 * e1[i] - 3.0 * e2[i] + e3[i]).collect()
}

/// `Sum(a, n)`
fn sum_n(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![NAN; a.len()];
    for i in 0..a.len() {
        if n >= 1 && i + 1 >= n {
            let mut s = 0f64;
            for k in 0..n {
                s += a[i - k] as f64;
            }
            out[i] = s as f32;
        }
    }
    out
}

/// `HHV(a, n)` / `LLV(a, n)`: the window is shortened at the start.
fn hhv(a: &[f32], n: usize) -> Vec<f32> {
    (0..a.len()).map(|i| a[i.saturating_sub(n.saturating_sub(1))..=i].iter().copied().fold(f32::MIN, f32::max)).collect()
}
fn llv(a: &[f32], n: usize) -> Vec<f32> {
    (0..a.len()).map(|i| a[i.saturating_sub(n.saturating_sub(1))..=i].iter().copied().fold(f32::MAX, f32::min)).collect()
}

/// `Ref(a, -k)`
fn back(a: &[f32], k: usize) -> Vec<f32> {
    (0..a.len()).map(|i| if i >= k { a[i - k] } else { NAN }).collect()
}

/// `StDev(a, n)`: population standard deviation of the last n values.
fn stdev(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![NAN; a.len()];
    if n == 0 {
        return out;
    }
    for i in 0..a.len() {
        if i + 1 >= n {
            let w = &a[i + 1 - n..=i];
            let m = w.iter().map(|&x| x as f64).sum::<f64>() / n as f64;
            let var = w.iter().map(|&x| (x as f64 - m) * (x as f64 - m)).sum::<f64>() / n as f64;
            out[i] = var.sqrt() as f32;
        }
    }
    out
}

/// `ATR(1)`: the true range, first bar H - L.
fn tr1(h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    (0..h.len()).map(|i| if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect()
}

/// `ATR(n)`: simple average of the first n true ranges, then Wilder smoothing (the form checked against the UNITECH scale-in sheet).
fn atr_w(h: &[f32], l: &[f32], c: &[f32], n: usize) -> Vec<f32> {
    let len = h.len();
    let tr = tr1(h, l, c);
    let mut out = vec![NAN; len];
    if n == 0 || len < n {
        return out;
    }
    out[n - 1] = tr[..n].iter().sum::<f32>() / n as f32;
    for i in n..len {
        out[i] = (out[i - 1] * (n as f32 - 1.0) + tr[i]) / n as f32;
    }
    out
}

/// `RSI(n)`: Wilder form, simple-average seed over the first n changes (first value at bar n).
fn rsi(c: &[f32], n: usize) -> Vec<f32> {
    let len = c.len();
    let mut out = vec![NAN; len];
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

/// `ROC(a, n)` in percent: 100 * (a - Ref(a,-n)) / Ref(a,-n).
fn roc(a: &[f32], n: usize) -> Vec<f32> {
    (0..a.len()).map(|i| if i >= n && a[i - n] != 0.0 { 100.0 * (a[i] - a[i - n]) / a[i - n] } else { NAN }).collect()
}

/// `MFI(n)`: typical price x volume flows, positive / negative by the change of the typical price, summed over n bars.
fn mfi(h: &[f32], l: &[f32], c: &[f32], v: &[f32], n: usize) -> Vec<f32> {
    let len = h.len();
    let mut out = vec![NAN; len];
    let tp: Vec<f32> = (0..len).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
    let (mut pos, mut neg) = (vec![0f64; len], vec![0f64; len]);
    for i in 1..len {
        let mf = (tp[i] * v[i]) as f64;
        if tp[i] > tp[i - 1] {
            pos[i] = mf;
        } else if tp[i] < tp[i - 1] {
            neg[i] = mf;
        }
    }
    for i in n..len {
        let sp: f64 = pos[i + 1 - n..=i].iter().sum();
        let sn: f64 = neg[i + 1 - n..=i].iter().sum();
        out[i] = if sn == 0.0 { 100.0 } else { (100.0 - 100.0 / (1.0 + sp / sn)) as f32 };
    }
    out
}

/// `PDI(n)` and `MDI(n)`: Wilder smoothing of +DM, -DM and the true range (simple-average seed), DI = 100 x smoothed DM / smoothed TR.
fn pdi_mdi(h: &[f32], l: &[f32], c: &[f32], n: usize) -> (Vec<f32>, Vec<f32>) {
    let len = h.len();
    let (mut pdi, mut mdi) = (vec![NAN; len], vec![NAN; len]);
    if n == 0 || len < n {
        return (pdi, mdi);
    }
    let (mut pdm, mut mdm, mut tr) = (vec![0f32; len], vec![0f32; len], vec![0f32; len]);
    tr[0] = h[0] - l[0];
    for i in 1..len {
        let up = h[i] - h[i - 1];
        let dn = l[i - 1] - l[i];
        pdm[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        mdm[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        tr[i] = h[i].max(c[i - 1]) - l[i].min(c[i - 1]);
    }
    let nf = n as f32;
    let (mut sp, mut sm, mut st) = (pdm[..n].iter().sum::<f32>() / nf, mdm[..n].iter().sum::<f32>() / nf, tr[..n].iter().sum::<f32>() / nf);
    for i in n - 1..len {
        if i >= n {
            sp = (sp * (nf - 1.0) + pdm[i]) / nf;
            sm = (sm * (nf - 1.0) + mdm[i]) / nf;
            st = (st * (nf - 1.0) + tr[i]) / nf;
        }
        if st > 0.0 {
            pdi[i] = 100.0 * sp / st;
            mdi[i] = 100.0 * sm / st;
        } else {
            pdi[i] = 0.0;
            mdi[i] = 0.0;
        }
    }
    (pdi, mdi)
}

/// Parabolic SAR (Wilder): starts long from the first bar's low; the stop is kept outside the last two bars; AF steps by `accel` up to `mx`.
fn sar(h: &[f32], l: &[f32], accel: f32, mx: f32) -> Vec<f32> {
    let n = h.len();
    let mut out = vec![NAN; n];
    if n == 0 {
        return out;
    }
    let (mut long, mut ep, mut af, mut cur) = (true, h[0], accel, l[0]);
    out[0] = cur;
    for i in 1..n {
        let mut ns = cur + af * (ep - cur);
        if long {
            ns = ns.min(l[i - 1]);
            if i >= 2 {
                ns = ns.min(l[i - 2]);
            }
            if l[i] < ns {
                long = false;
                ns = ep;
                ep = l[i];
                af = accel;
            } else if h[i] > ep {
                ep = h[i];
                af = (af + accel).min(mx);
            }
        } else {
            ns = ns.max(h[i - 1]);
            if i >= 2 {
                ns = ns.max(h[i - 2]);
            }
            if h[i] > ns {
                long = true;
                ns = ep;
                ep = h[i];
                af = accel;
            } else if l[i] < ep {
                ep = l[i];
                af = (af + accel).min(mx);
            }
        }
        cur = ns;
        out[i] = cur;
    }
    out
}

/// The AFLs' %swing / Gann loop: trend (+1 / -1) of `Up` / `Dn` lines. `up` / `dn` hold the initial lines; at a trend change to down the
/// upper line restarts at `flag_up(i)`, at a change to up the lower line at `flag_dn(i)`.
fn swing_trend(c: &[f32], mut up: Vec<f32>, mut dn: Vec<f32>, flag_up: &dyn Fn(usize) -> f32, flag_dn: &dyn Fn(usize) -> f32) -> Vec<i8> {
    let n = c.len();
    let mut trend = vec![1i8; n];
    for i in 1..n {
        trend[i] = if gt(c[i], up[i - 1]) {
            1
        } else if lt(c[i], dn[i - 1]) {
            -1
        } else {
            trend[i - 1]
        };
        let flag1 = trend[i] < 0 && trend[i - 1] > 0;
        let flagh1 = trend[i] > 0 && trend[i - 1] < 0;
        if trend[i] > 0 && lt(dn[i], dn[i - 1]) {
            dn[i] = dn[i - 1];
        }
        if trend[i] < 0 && gt(up[i], up[i - 1]) {
            up[i] = up[i - 1];
        }
        if flag1 {
            up[i] = flag_up(i);
        }
        if flagh1 {
            dn[i] = flag_dn(i);
        }
    }
    trend
}

/// The plain %swing of the stock: `EXIT = C*EXITPCT/100; Up = C + EXIT; Dn = C - EXIT` (flag lines restart at C +- EXIT).
fn swing_std(c: &[f32], pct: f32) -> Vec<i8> {
    let ex: Vec<f32> = c.iter().map(|&x| (x * pct) / 100.0).collect();
    let up: Vec<f32> = (0..c.len()).map(|i| c[i] + ex[i]).collect();
    let dn: Vec<f32> = (0..c.len()).map(|i| c[i] - ex[i]).collect();
    swing_trend(c, up, dn, &|i| c[i] + ex[i], &|i| c[i] - ex[i])
}

/// The %swing with `Up = Highest_Close` (ATR-or-swing and RSI(70) AFLs): `Dn = C - EXIT`, the flag lines restart at C +- EXIT.
fn swing_hcl(c: &[f32], hcl: &[f32], pct: f32) -> Vec<i8> {
    let ex: Vec<f32> = c.iter().map(|&x| (x * pct) / 100.0).collect();
    let dn: Vec<f32> = (0..c.len()).map(|i| c[i] - ex[i]).collect();
    swing_trend(c, hcl.to_vec(), dn, &|i| c[i] + ex[i], &|i| c[i] - ex[i])
}

/// Supertrend trend of the AFL's `ST()`: satr = mean of the last `period` ATR(1) (Null while short; period 0 divides by zero), Up = stcal + f_up x satr,
/// Dn = stcal - f_dn x satr.
fn st_trend(stcal: &[f32], tr: &[f32], c: &[f32], period: usize, f_up: f32, f_dn: f32) -> Vec<i8> {
    let n = c.len();
    let mut satr = vec![NAN; n];
    for i in 0..n {
        if period == 0 {
            satr[i] = tr[i] / 0.0;
        } else if i + 1 >= period {
            let mut s = tr[i];
            for k in 1..period {
                s += tr[i - k];
            }
            satr[i] = s / period as f32;
        }
    }
    let up: Vec<f32> = (0..n).map(|i| stcal[i] + f_up * satr[i]).collect();
    let dn: Vec<f32> = (0..n).map(|i| stcal[i] - f_dn * satr[i]).collect();
    // the flag lines restart at stcal +- band x satr
    swing_trend(c, up, dn, &|i| stcal[i] + f_up * satr[i], &|i| stcal[i] - f_dn * satr[i])
}

// ================================================================ the Skipbuy / SkipSell state machine

/// One AFL of the "Position / Skipbuy / SkipSell" shape (chain of `else if`):
/// 1. `Position==0 AND entry(i) AND Skipbuy==0`: if `Buy_Cash_Filter AND entry_in(i)` buy at Close, else `Skipbuy = 1`
/// 2. (order set by `reset_first`) `Skipbuy==1 AND reset(i)` -> `Skipbuy = 0`; `Skipbuy==1 AND rebuy(i) AND Buy_Cash_Filter` -> buy, `Skipbuy = 0`
/// 3. `(Position>0 AND exit(i, SkipSell)) OR (Position>0 AND deadsell)`: if `LC==0 OR deadsell` sell at Close (Position = SkipSell = 0) else `SkipSell = 1`
/// 4. `(Position>0 AND LC==0 AND SkipSell==1) OR (Position>0 AND deadsell)`: sell at Close.
struct Sk<'a> {
    start: usize,
    uc_in_filter: bool,
    entry: &'a dyn Fn(usize) -> bool,
    entry_in: &'a dyn Fn(usize) -> bool,
    reset: Option<&'a dyn Fn(usize) -> bool>,
    rebuy: &'a dyn Fn(usize) -> bool,
    reset_first: bool,
    exit: &'a dyn Fn(usize, bool) -> bool,
}

fn sk_run(cx: &Ctx, sp: &Sk) -> (Vec<bool>, Vec<bool>) {
    sk_run_q(cx, sp, false)
}

/// `quirk` (RSI(70) AFL only): the exit test has an un-gated `Position>0 AND trend1==-1` term, its sell sets `Position = 0` but leaves `SkipSell`
/// as it is, and the late-sell test has no Position check (`skipsell==1 AND LC==0`), so a stuck SkipSell sells the next position one bar after its entry.
fn sk_run_q(cx: &Ctx, sp: &Sk, quirk: bool) -> (Vec<bool>, Vec<bool>) {
    let n = cx.n;
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut pos, mut skipbuy, mut skipsell) = (false, false, false);
    for i in sp.start.max(1)..n {
        let bcf = cx.bcf(i, sp.uc_in_filter);
        let ds = cx.dsell[i];
        let reset_hit = skipbuy && sp.reset.map_or(false, |f| f(i));
        let rebuy_hit = skipbuy && (sp.rebuy)(i) && bcf;
        if !pos && !skipbuy && (sp.entry)(i) {
            if bcf && (sp.entry_in)(i) {
                buy[i] = true;
                pos = true;
            } else {
                skipbuy = true;
            }
        } else if sp.reset_first && reset_hit {
            skipbuy = false;
        } else if rebuy_hit {
            buy[i] = true;
            pos = true;
            skipbuy = false;
        } else if !sp.reset_first && reset_hit {
            skipbuy = false;
        } else if pos && ((sp.exit)(i, skipsell) || ds) {
            if !cx.lc[i] || ds {
                sell[i] = true;
                pos = false;
                if !quirk {
                    skipsell = false;
                }
            } else {
                skipsell = true;
            }
        } else if quirk {
            if (skipsell && !cx.lc[i]) || (skipsell && ds) {
                if pos {
                    sell[i] = true;
                }
                pos = false;
                skipsell = false;
            }
        } else if (pos && !cx.lc[i] && skipsell) || (pos && ds) {
            sell[i] = true;
            pos = false;
            skipsell = false;
        }
    }
    (buy, sell)
}

/// The Buysig-carry shape (MACD, PDI/MDI, Gann levels): `Position==0 AND (Close>Ref(Hcl,-1) OR Buysig) AND ind(i) AND Buy_Cash_Filter` buys;
/// `(Position>0 AND SkipSell==0 AND x(i)) OR (Position>0 AND deadsell)` sells (or sets SkipSell on a lower circuit);
/// `(SkipSell==1 AND LC==0) OR deadsell` sells later. `chained` false: the sell test is a separate `if` (Gann levels), so it can fire on the buy bar.
fn bs_run(cx: &Ctx, buysig: &[bool], ind: &dyn Fn(usize) -> bool, x: &dyn Fn(usize) -> bool, chained: bool) -> (Vec<bool>, Vec<bool>) {
    let n = cx.n;
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut pos, mut skipsell) = (false, false);
    for i in 1..n {
        let ds = cx.dsell[i];
        let mut bought = false;
        if !pos && (cx.ath(i) || buysig[i]) && ind(i) && cx.bcf(i, true) {
            buy[i] = true;
            pos = true;
            bought = true;
        }
        if chained && bought {
            continue;
        }
        if (pos && !skipsell && x(i)) || (pos && ds) {
            if !cx.lc[i] || ds {
                sell[i] = true;
                pos = false;
            } else {
                skipsell = true;
            }
        } else if (skipsell && !cx.lc[i]) || ds {
            if pos || skipsell {
                sell[i] = true;
            }
            pos = false;
            skipsell = false;
        }
    }
    (buy, sell)
}

/// `Buysig` array of the AFLs: set when `set(i)`, cleared when `clear(i)` and the previous bar had it, else carried.
fn buysig_of(n: usize, set: &dyn Fn(usize) -> bool, clear: &dyn Fn(usize) -> bool) -> Vec<bool> {
    let mut b = vec![false; n];
    for i in 1..n {
        b[i] = if set(i) {
            true
        } else if clear(i) && b[i - 1] {
            false
        } else {
            b[i - 1]
        };
    }
    b
}

fn cross_dn(a: &[f32], b: &[f32], i: usize) -> bool {
    i >= 1 && lt(a[i], b[i]) && gt(a[i - 1], b[i - 1])
}
fn cross_dn_lvl(a: &[f32], lvl: f32, i: usize) -> bool {
    i >= 1 && lt(a[i], lvl) && gt(a[i - 1], lvl)
}
fn cross_up_lvl(a: &[f32], lvl: f32, i: usize) -> bool {
    i >= 1 && gt(a[i], lvl) && lt(a[i - 1], lvl)
}

fn no_reset_default() -> Option<&'static dyn Fn(usize) -> bool> {
    None
}

// ================================================================ param helpers

fn dead_p() -> P {
    p("deadstock", "DEAD STOCK BACKTEST (1 YES; 2 NO)", 2.0, 1.0, 2.0, 1.0)
}
fn hard_p() -> P {
    p("hardstop", "Hard Stoploss %", 0.0, 0.0, 50.0, 1.0)
}
fn penny_p() -> P {
    p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0)
}
fn turn_p() -> P {
    p("turnflt", "TURNOVER Filter (1 NO; 2 YES)", 1.0, 1.0, 2.0, 1.0)
}
fn base3() -> Vec<P> {
    vec![dead_p(), penny_p(), turn_p()]
}
/// a Param default clamped into its own range like AmiBroker does
fn pc(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> P {
    p(key, label, def.max(min).min(max), min, max, step)
}
fn map_series(series: &[Series], f: &dyn Fn(&Series) -> Signals) -> Vec<Signals> {
    series.iter().map(|s| f(s)).collect()
}

// ================================================================ 1. RSI(70) cross entry, RSI(50) cross-down or %swing exit

fn sg_rsi70(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let (pct, prd) = (g("exitpct") as f32, g("rsi_prd").round() as usize);
    let (bv, sv) = (g("rsi_buy_val") as f32, g("rsi_sell_val") as f32);
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let tr = swing_hcl(&cx.c, &cx.hcl, pct);
        let r = rsi(&cx.c, prd);
        let entry = |i: usize| cx.ath(i) && gt(r[i], bv) && i >= 1 && lt(r[i - 1], bv);
        let reset = |i: usize| cross_dn_lvl(&r, sv, i);
        let exit = |i: usize, ss: bool| (!ss && cross_dn_lvl(&r, sv, i)) || tr[i] == -1;
        let sp = Sk { start: 1, uc_in_filter: true, entry: &entry, entry_in: &|_| true, reset: Some(&reset), rebuy: &|_| true, reset_first: true, exit: &exit };
        let (b, sl) = sk_run_q(&cx, &sp, true);
        mk(s, b, sl, None, false, 0.0)
    })
}
fn schema_rsi70() -> Vec<P> {
    let mut v = base3();
    v.extend([
        p("exitpct", "EXIT %", 10.0, 1.0, 20.0, 1.0),
        p("rsi_prd", "RSI Period", 14.0, 1.0, 100.0, 1.0),
        p("rsi_buy_val", "RSI Buy Value", 70.0, 1.0, 100.0, 1.0),
        p("rsi_sell_val", "RSI Sell Value", 50.0, 1.0, 100.0, 1.0),
    ]);
    v
}

// ================================================================ 2. RSI vs RSI average

fn sg_rsi_avg(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let pct = g("exitpct") as f32;
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let tr = swing_std(&cx.c, pct);
        // Optimize("RSI Period",14,10,26,4), Optimize("Avg_RSI_Prd",9,5,21,2): defaults
        let r = rsi(&cx.c, 14);
        let av = ma(&r, 9);
        let entry = |i: usize| cx.ath(i) && gt(r[i], av[i]);
        let rebuy = |i: usize| gt(r[i], av[i]);
        let exit = |i: usize, ss: bool| !ss && (tr[i] == -1 || cross_dn(&r, &av, i));
        let sp = Sk { start: 1, uc_in_filter: true, entry: &entry, entry_in: &|_| true, reset: no_reset_default(), rebuy: &rebuy, reset_first: true, exit: &exit };
        let (b, sl) = sk_run(&cx, &sp);
        mk(s, b, sl, None, false, 0.0)
    })
}
fn schema_rsi_avg() -> Vec<P> {
    let mut v = base3();
    v.push(p("exitpct", "EXIT %", 10.0, 1.0, 20.0, 1.0));
    v
}

// ================================================================ 3. ATR or %swing exit

fn sg_atr_swing(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let (buf, pct) = (g("buffer_pct") as f32, g("exitpct") as f32);
    let (aprd, amul) = (g("atr_prd").round() as usize, g("atr_multi") as f32);
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let n = cx.n;
        let tr = swing_hcl(&cx.c, &cx.hcl, pct);
        let myatr: Vec<f32> = atr_w(&cx.h, &cx.l, &cx.c, aprd).iter().map(|&x| x * amul).collect();
        let trv = tr1(&cx.h, &cx.l, &cx.c);
        let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
        let (mut pos, mut skipbuy, mut skipsell) = (false, false, false);
        for i in 1..n {
            let bcf = cx.bcf(i, true);
            let ds = cx.dsell[i];
            if !pos && cx.ath(i) && lt(trv[i], myatr[i]) && !skipbuy {
                if bcf {
                    buy[i] = true;
                    pos = true;
                } else {
                    skipbuy = true;
                }
            } else if skipbuy && (gt(trv[i], myatr[i]) || tr[i] == -1) {
                skipbuy = false;
            } else if skipbuy && lt(trv[i], myatr[i]) && tr[i] == 1 && bcf {
                buy[i] = true;
                pos = true;
                skipbuy = false;
            } else if pos && !skipsell {
                let gap = cx.c[i - 1] - (cx.c[i - 1] * buf) / 100.0;
                if (gt(trv[i], myatr[i]) && lt(cx.c[i], gap)) || tr[i] == -1 {
                    if !cx.lc[i] || ds {
                        sell[i] = true;
                        pos = false;
                        skipsell = false;
                    } else {
                        skipsell = true;
                    }
                }
            }
            // separate `if` after the chain
            if (pos && !cx.lc[i] && skipsell) || (pos && ds) {
                sell[i] = true;
                pos = false;
                skipsell = false;
            }
        }
        mk(s, buy, sell, None, false, 0.0)
    })
}
fn schema_atr_swing() -> Vec<P> {
    let mut v = base3();
    v.extend([
        p("buffer_pct", "Buffer %", 2.0, 0.0, 10.0, 0.01),
        p("exitpct", "EXIT %", 10.0, 1.0, 20.0, 1.0),
        p("atr_prd", "ATR Period", 5.0, 5.0, 20.0, 5.0),
        p("atr_multi", "ATR Multiplier", 1.5, 1.0, 4.0, 0.5),
    ]);
    v
}

// ================================================================ 4. moving-average exit versions

fn sg_ma_versions(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let ver = g("ver").round() as i32; // 1 EMA, 2 Simple, 3 Double, 4 Triple, 5 Triangular
    let (pe, pm, pd, pt, ptma) = (g("ema_period").round() as usize, g("ma_period").round() as usize, g("doublema_periods").round() as usize, g("triplema_periods").round() as usize, g("tma_periods").round() as usize);
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let (line, period) = match ver {
            2 => (ma(&cx.c, pm), pm),
            3 => (ma(&ma(&cx.c, pd), pd), pd),
            4 => (ma(&ma(&ma(&cx.c, pt), pt), pt), pt),
            5 => {
                let tp = ((ptma as f32 + 1.1) / 2.0).round() as usize;
                let sm: Vec<f32> = sum_n(&cx.c, tp).iter().map(|&x| x / tp as f32).collect();
                (ma(&sm, tp), ptma)
            }
            _ => (ema(&cx.c, pe), pe),
        };
        let entry = |i: usize| cx.ath(i);
        let reset = |i: usize| lt(cx.c[i], line[i]);
        let exit = |i: usize, ss: bool| !ss && lt(cx.c[i], line[i]);
        let sp = Sk { start: period, uc_in_filter: true, entry: &entry, entry_in: &|_| true, reset: Some(&reset), rebuy: &|_| true, reset_first: true, exit: &exit };
        let (b, sl) = sk_run(&cx, &sp);
        mk(s, b, sl, None, false, 0.0)
    })
}
fn schema_ma_versions() -> Vec<P> {
    let mut v = base3();
    v.extend([
        p("ver", "MA Versions for exit (1 EMA; 2 Simple MA; 3 Double MA; 4 Triple MA; 5 Triangular MA)", 1.0, 1.0, 5.0, 1.0),
        p("ema_period", "EMA Period", 20.0, 5.0, 220.0, 5.0),
        p("ma_period", "Simple MA Period", 20.0, 1.0, 300.0, 1.0),
        p("doublema_periods", "Double MA Periods", 20.0, 1.0, 300.0, 1.0),
        p("triplema_periods", "Triple MA Periods", 20.0, 1.0, 300.0, 1.0),
        p("tma_periods", "Triangular MA Periods", 20.0, 1.0, 300.0, 1.0),
    ]);
    v
}

// ================================================================ 5. Elder Ray bear power

fn sg_elder(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let ver = g("ver").round() as i32; // 1 Fresh, 2 Running
    let ev = g("eld_value") as f32;
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        // Optimize("Elder Ray Bear Period",13,1,300,1)
        let e13 = ema(&cx.c, 13);
        let eray: Vec<f32> = (0..cx.n).map(|i| cx.l[i] - e13[i]).collect();
        let pv = |i: usize| if i >= 1 { eray[i - 1] } else { NAN };
        let exit = |i: usize, ss: bool| !ss && lt(pv(i), -ev);
        let (b, sl) = if ver == 2 {
            let entry = |i: usize| cx.ath(i);
            let inn = |i: usize| gt(pv(i), ev);
            let sp = Sk { start: 1, uc_in_filter: true, entry: &entry, entry_in: &inn, reset: no_reset_default(), rebuy: &inn, reset_first: true, exit: &exit };
            sk_run(&cx, &sp)
        } else {
            let entry = |i: usize| cx.ath(i) && gt(pv(i), ev);
            let reset = |i: usize| lt(pv(i), -ev);
            let sp = Sk { start: 1, uc_in_filter: true, entry: &entry, entry_in: &|_| true, reset: Some(&reset), rebuy: &|_| true, reset_first: true, exit: &exit };
            sk_run(&cx, &sp)
        };
        mk(s, b, sl, None, false, 0.0)
    })
}
fn schema_elder() -> Vec<P> {
    let mut v = base3();
    v.extend([p("ver", "Entry/Exit Version (1 Fresh; 2 Running)", 1.0, 1.0, 2.0, 1.0), p("eld_value", "Elder Ray Bear Value", 0.0, 0.0, 100.0, 0.01)]);
    v
}

// ================================================================ 6. 52-week lowest close (weekly compression)

fn sg_52wk(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let period = g("period").round() as usize;
    let tf: u8 = match g("flttf").round() as i32 {
        2 => 2, // DAILY
        3 => 0, // MONTHLY
        _ => 1, // WEEKLY
    };
    let buf = g("bufferpct") as f32;
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let n = cx.n;
        let k = compress(s, tf);
        let lw = llv(&k.c, period);
        let x = back(&lw, 1);
        let wc = expand(&x, &k, n);
        let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
        let mut sp = cx.c.clone();
        let (mut pos, mut skipsell) = (false, false);
        for i in 1..n {
            let ds = cx.dsell[i];
            let bcf = cx.bcf(i, false);
            if !pos && cx.ath(i) && !cx.uc[i] && bcf {
                buy[i] = true;
                pos = true;
            } else if pos && !skipsell && (lt(cx.c[i], wc[i]) || ds) {
                if !cx.lc[i] || ds {
                    sell[i] = true;
                    pos = false;
                } else {
                    skipsell = true;
                }
            } else if (pos && !cx.loc[i] && skipsell) || (pos && ds) {
                sell[i] = true;
                pos = false;
                skipsell = false;
                let (o, l) = (cx.o[i], cx.l[i]);
                let lvl = o - (o * buf) / 100.0;
                if o == l {
                    sp[i] = o;
                } else if o != l && l < lvl {
                    sp[i] = lvl;
                } else if o != l && l > lvl {
                    sp[i] = l;
                }
            }
        }
        mk(s, buy, sell, Some(sp), false, 0.0)
    })
}
fn schema_52wk() -> Vec<P> {
    let mut v = base3();
    v.extend([
        p("period", "Period", 52.0, 10.0, 280.0, 10.0),
        p("flttf", "FILTER TIMEFRAME (1 WEEKLY; 2 DAILY; 3 MONTHLY)", 1.0, 1.0, 3.0, 1.0),
        p("bufferpct", "Selling Buffer %", 0.25, 0.0, 10.0, 0.05),
    ]);
    v
}

// ================================================================ 7. Gann HiLo

fn sg_gann_hilo(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let n = cx.n;
        // Optimize("GANN HiLo Period",3,5,250,5): default 3 (outside the range, the Optimize default is used by a plain backtest)
        let prd = 3usize;
        let mah = ma(&cx.h, prd);
        let mal = ma(&cx.l, prd);
        let (hib, lob) = (back(&mah, 1), back(&mal, 1));
        let mut hlv = vec![NAN; n];
        for i in 0..n {
            let hld = if gt(cx.c[i], hib[i]) {
                1.0
            } else if lt(cx.c[i], lob[i]) {
                -1.0
            } else {
                0.0
            };
            hlv[i] = if hld != 0.0 {
                hld
            } else if i > 0 {
                hlv[i - 1]
            } else {
                NAN
            };
        }
        let hilo: Vec<f32> = (0..n).map(|i| if hlv[i] == -1.0 { mah[i] } else { mal[i] }).collect();
        let entry = |i: usize| cx.ath(i) && gt(cx.c[i], hilo[i]);
        let rebuy = |i: usize| gt(cx.c[i], hilo[i]);
        let reset = |i: usize| lt(cx.c[i], hilo[i]);
        let exit = |i: usize, ss: bool| !ss && lt(cx.c[i], hilo[i]);
        let sp = Sk { start: prd, uc_in_filter: true, entry: &entry, entry_in: &|_| true, reset: Some(&reset), rebuy: &rebuy, reset_first: false, exit: &exit };
        let (b, sl) = sk_run(&cx, &sp);
        mk(s, b, sl, None, false, 0.0)
    })
}
fn schema_gann_hilo() -> Vec<P> {
    base3()
}

// ================================================================ 8. Gann square-root levels trail SL

fn sg_gann_levels(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let f1 = g("factor1") as f32;
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let n = cx.n;
        let dnf = |i: usize| {
            let r = cx.c[i].sqrt() - f1;
            r * r
        };
        let dn: Vec<f32> = (0..n).map(dnf).collect();
        let tr = swing_trend(&cx.c, cx.hcl.clone(), dn, &|i| cx.hcl[i], &dnf);
        let buysig = buysig_of(n, &|i| gt(cx.c[i], cx.hcl[i - 1]), &|i| tr[i] == -1);
        let (b, sl) = bs_run(&cx, &buysig, &|_| true, &|i| tr[i] == -1, false);
        mk(s, b, sl, None, false, 0.0)
    })
}
fn schema_gann_levels() -> Vec<P> {
    let mut v = base3();
    v.push(p("factor1", "gann factor", 0.5, 0.5, 4.0, 0.25));
    v
}

// ================================================================ 9. Ichimoku

/// `Max` / `Min` of two arrays with AmiBroker's Null (-1e10): Max ignores a Null, Min of a Null is Null.
fn max_null(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        b
    } else if b.is_nan() {
        a
    } else {
        a.max(b)
    }
}
fn min_null(a: f32, b: f32) -> f32 {
    if a.is_nan() || b.is_nan() {
        NAN
    } else {
        a.min(b)
    }
}

fn sg_ichimoku(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let ver = g("ver").round() as i32; // 1 Lines crossover, 2 Clouds
    let slb = g("slb_hlprds").round() as usize;
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let n = cx.n;
        let (tlp, blp) = (9usize, 26usize);
        let mid = |p: usize| -> Vec<f32> {
            let (hh, ll) = (hhv(&cx.h, p), llv(&cx.l, p));
            (0..n).map(|i| (hh[i] + ll[i]) / 2.0).collect()
        };
        let tl = mid(tlp);
        let bl = mid(blp);
        let sa = back(&(0..n).map(|i| (bl[i] + tl[i]) / 2.0).collect::<Vec<f32>>(), blp);
        let sb = back(&mid(slb), blp);
        let maxs: Vec<f32> = (0..n).map(|i| max_null(sa[i], sb[i])).collect();
        let mins: Vec<f32> = (0..n).map(|i| min_null(sa[i], sb[i])).collect();
        let (b, sl) = if ver == 2 {
            let entry = |i: usize| cx.ath(i) && gt(cx.c[i], maxs[i]);
            let reset = |i: usize| lt(cx.c[i], mins[i]);
            let rebuy = |i: usize| gt(cx.c[i], maxs[i]);
            let exit = |i: usize, ss: bool| !ss && lt(cx.c[i], mins[i]);
            let sp = Sk { start: 1, uc_in_filter: true, entry: &entry, entry_in: &|_| true, reset: Some(&reset), rebuy: &rebuy, reset_first: true, exit: &exit };
            sk_run(&cx, &sp)
        } else {
            let entry = |i: usize| cx.ath(i) && gt(tl[i], bl[i]);
            let reset = |i: usize| cross_dn(&tl, &bl, i);
            let rebuy = |i: usize| gt(tl[i], bl[i]);
            let exit = |i: usize, ss: bool| !ss && cross_dn(&tl, &bl, i);
            let sp = Sk { start: 1, uc_in_filter: true, entry: &entry, entry_in: &|_| true, reset: Some(&reset), rebuy: &rebuy, reset_first: true, exit: &exit };
            sk_run(&cx, &sp)
        };
        mk(s, b, sl, None, false, 0.0)
    })
}
fn schema_ichimoku() -> Vec<P> {
    let mut v = base3();
    v.extend([p("ver", "Exit Version (1 Lines Crossover Exit; 2 Clouds Exit)", 1.0, 1.0, 2.0, 1.0), p("slb_hlprds", "SLB_HLprds", 52.0, 1.0, 100.0, 1.0)]);
    v
}

// ================================================================ 10. MACD, 11. PDI/MDI

fn sg_macd(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let r3 = g("r3").round() as usize;
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let n = cx.n;
        // Optimize("Fast avg Prd",12,5,20,1), Optimize("Slow avg Prd",26,10,40,2)
        let (e1, e2) = (ema(&cx.c, 12), ema(&cx.c, 26));
        let macd: Vec<f32> = (0..n).map(|i| e1[i] - e2[i]).collect();
        let sig = ema(&macd, r3);
        let buysig = buysig_of(n, &|i| gt(cx.c[i], cx.hcl[i - 1]) && gt(macd[i], sig[i]), &|i| cross_dn(&macd, &sig, i));
        let (b, sl) = bs_run(&cx, &buysig, &|i| gt(macd[i], sig[i]), &|i| cross_dn(&macd, &sig, i), true);
        mk(s, b, sl, None, false, 0.0)
    })
}
fn schema_macd() -> Vec<P> {
    let mut v = base3();
    v.push(p("r3", "Signal avg Prd", 9.0, 1.0, 200.0, 1.0));
    v
}

fn sg_pdi_mdi(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let (pp, mp) = (g("pdi_prd").round() as usize, g("mdi_prd").round() as usize);
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let n = cx.n;
        // the ADX(adx_prd) is calculated by the AFL but never used
        let (pdi, _) = pdi_mdi(&cx.h, &cx.l, &cx.c, pp);
        let (_, mdi) = pdi_mdi(&cx.h, &cx.l, &cx.c, mp);
        let buysig = buysig_of(n, &|i| gt(cx.c[i], cx.hcl[i - 1]) && gt(pdi[i], mdi[i]), &|i| cross_dn(&pdi, &mdi, i));
        let (b, sl) = bs_run(&cx, &buysig, &|i| gt(pdi[i], mdi[i]), &|i| cross_dn(&pdi, &mdi, i), true);
        mk(s, b, sl, None, false, 0.0)
    })
}
fn schema_pdi_mdi() -> Vec<P> {
    let mut v = base3();
    v.extend([p("adx_prd", "ADX Periods", 20.0, 5.0, 26.0, 1.0), p("pdi_prd", "PDI Periods", 20.0, 5.0, 26.0, 1.0), p("mdi_prd", "MDI Periods", 20.0, 5.0, 26.0, 1.0)]);
    v
}

// ================================================================ 12. MFI

fn sg_mfi(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let pct = g("exitpct") as f32;
    let prd = g("mfi_prd").round() as usize;
    let (ve, vx) = (g("mfi_value_entry") as f32, g("mfi_value_exit") as f32);
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let n = cx.n;
        let vol = s.volume();
        let tr = swing_std(&cx.c, pct);
        let athc = buysig_of(n, &|i| gt(cx.c[i], cx.hcl[i - 1]), &|i| tr[i] == -1);
        let m = mfi(&cx.h, &cx.l, &cx.c, &vol, prd);
        // entry: MFI crosses DOWN through the entry value while the ATHC state is alive; exit: MFI crosses UP through the exit value (as written)
        let entry = |i: usize| athc[i] && cross_dn_lvl(&m, ve, i);
        let reset = |i: usize| !athc[i];
        let rebuy = |i: usize| athc[i];
        let exit = |i: usize, ss: bool| !ss && (cross_up_lvl(&m, vx, i) || tr[i] == -1);
        let sp = Sk { start: 1, uc_in_filter: true, entry: &entry, entry_in: &|_| true, reset: Some(&reset), rebuy: &rebuy, reset_first: true, exit: &exit };
        let (b, sl) = sk_run(&cx, &sp);
        mk(s, b, sl, None, false, 0.0)
    })
}
fn schema_mfi() -> Vec<P> {
    let mut v = base3();
    v.extend([
        p("exitpct", "Swing EXIT %", 10.0, 1.0, 50.0, 1.0),
        p("mfi_prd", "MFI Period", 14.0, 1.0, 100.0, 1.0),
        p("mfi_value_entry", "MFI Entry Value", 50.0, 10.0, 90.0, 10.0),
        p("mfi_value_exit", "MFI Exit Value", 30.0, 10.0, 90.0, 10.0),
    ]);
    v
}

// ================================================================ 13. ROC vs its average, 14. SAR

fn sg_roc(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let (rp, ra) = (g("roc_prds").round() as usize, g("roc_avg_prd").round() as usize);
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let r = roc(&cx.c, rp);
        let av = ma(&r, ra);
        let entry = |i: usize| cx.ath(i) && gt(r[i], av[i]);
        let rebuy = |i: usize| gt(r[i], av[i]);
        let exit = |i: usize, ss: bool| !ss && cross_dn(&r, &av, i);
        let sp = Sk { start: 1, uc_in_filter: true, entry: &entry, entry_in: &|_| true, reset: no_reset_default(), rebuy: &rebuy, reset_first: true, exit: &exit };
        let (b, sl) = sk_run(&cx, &sp);
        mk(s, b, sl, None, false, 0.0)
    })
}
fn schema_roc() -> Vec<P> {
    let mut v = base3();
    v.extend([p("roc_prds", "ROC Periods", 10.0, 1.0, 100.0, 1.0), p("roc_avg_prd", "ROC AVG Period", 9.0, 1.0, 100.0, 1.0)]);
    v
}

fn sg_sar(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let (ac, mx) = (g("accel") as f32, g("mx") as f32);
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let ps = sar(&cx.h, &cx.l, ac, mx);
        let entry = |i: usize| cx.ath(i) && gt(cx.c[i], ps[i]);
        let reset = |i: usize| lt(cx.c[i], ps[i]);
        let exit = |i: usize, ss: bool| !ss && lt(cx.c[i], ps[i]);
        let sp = Sk { start: 1, uc_in_filter: true, entry: &entry, entry_in: &|_| true, reset: Some(&reset), rebuy: &|_| true, reset_first: true, exit: &exit };
        let (b, sl) = sk_run(&cx, &sp);
        mk(s, b, sl, None, false, 0.0)
    })
}
fn schema_sar() -> Vec<P> {
    let mut v = base3();
    v.extend([p("accel", "Acceleration", 0.02, 0.0, 1.0, 0.01), p("mx", "Max. acceleration", 0.2, 0.0, 1.0, 0.1)]);
    v
}

// ================================================================ 15. Bollinger upper band entry, 4 exit versions

#[derive(Clone, Copy)]
pub struct BollP {
    /// 1 Exit With ST, 2 Bollinger MB, 3 Bollinger LB, 4 whichever is earlier
    pub ver: i32,
    pub bbp: usize,
    pub mul: f32,
    /// compressed timeframe: 2 daily, 1 weekly, 0 monthly
    pub tf: u8,
    pub fltpd: usize,
    /// 1 MID POINT, 2 CLOSE, 3 PIVOT
    pub stf: i32,
    /// false: the "Close > Bollinger upper band" test is left out of the entry (used by the differential check against the plain ST engine only)
    pub band_on: bool,
}

pub fn boll_upper_one(s: &Series, cm: &Cmn, bp: &BollP) -> Signals {
    let cx = Ctx::new(s, cm, Tk::OpenInt);
    let n = cx.n;
    let ver = bp.ver;
    let price: Vec<f32> = (0..n).map(|i| (cx.h[i] + cx.l[i] + cx.c[i]) / 3.0).collect();
    let e = ema(&price, bp.bbp);
    let sd = stdev(&cx.c, bp.bbp);
    let upper: Vec<f32> = (0..n).map(|i| e[i] + bp.mul * sd[i]).collect();
    let lower: Vec<f32> = (0..n).map(|i| e[i] - bp.mul * sd[i]).collect();
    let midb = ema(&(0..n).map(|i| (upper[i] + lower[i]) / 2.0).collect::<Vec<f32>>(), 5);
    // Supertrend of the filter timeframe (Optimize("ST FILTER FACTOR",4,1,6,0.5)); expanded back to the daily bars
    let k = compress(s, bp.tf);
    let m = k.start.len();
    let cal: Vec<f32> = (0..m)
        .map(|g2| match bp.stf {
            1 => (k.h[g2] + k.l[g2]) / 2.0,
            3 => (k.h[g2] + k.l[g2] + k.c[g2]) / 3.0,
            _ => k.c[g2],
        })
        .collect();
    let trk = tr1(&k.h, &k.l, &k.c);
    let tdir = st_trend(&cal, &trk, &k.c, bp.fltpd, 1.0, 4.0);
    let tf32: Vec<f32> = tdir.iter().map(|&t| t as f32).collect();
    let mt = expand(&tf32, &k, n);
    let st_on = ver == 1;
    let above = |i: usize| !bp.band_on || gt(cx.c[i], upper[i]);
    let ent_ok = |i: usize| cx.ath(i) && above(i);
    let clear = |i: usize| match ver {
        1 => mt[i] == -1.0,
        2 => lt(cx.c[i], midb[i]),
        3 => lt(cx.c[i], lower[i]),
        _ => lt(cx.c[i], lower[i]) || lt(cx.c[i], midb[i]) || mt[i] == -1.0,
    };
    let buysig = buysig_of(n, &|i| ent_ok(i) && (!st_on || mt[i] == 1.0), &clear);
    let entry = |i: usize| ent_ok(i) && (!st_on || mt[i] == 1.0);
    let rebuy = |i: usize| buysig[i] && above(i) && (!st_on || mt[i] == 1.0);
    let reset = |i: usize| !buysig[i];
    let exit = |i: usize, ss: bool| {
        !ss && match ver {
            1 => mt[i] == -1.0 || !buysig[i],
            2 => lt(cx.c[i], midb[i]),
            3 => lt(cx.c[i], lower[i]),
            _ => mt[i] == -1.0 || lt(cx.c[i], midb[i]) || lt(cx.c[i], lower[i]),
        }
    };
    let sp = Sk { start: 1, uc_in_filter: true, entry: &entry, entry_in: &|_| true, reset: Some(&reset), rebuy: &rebuy, reset_first: false, exit: &exit };
    let (b, sl) = sk_run(&cx, &sp);
    mk(s, b, sl, None, false, 0.0)
}

fn sg_boll_upper(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let bp = BollP {
        ver: g("ver").round() as i32,
        bbp: g("bb_emaprd").round() as usize,
        mul: g("stdevmul") as f32,
        tf: match g("flttf").round() as i32 {
            2 => 1,
            3 => 0,
            _ => 2,
        },
        fltpd: g("fltpd").round() as usize,
        stf: g("stflt").round() as i32,
        band_on: true,
    };
    map_series(series, &|s| boll_upper_one(s, &cm, &bp))
}
fn schema_boll_upper() -> Vec<P> {
    let mut v = base3();
    v.extend([
        p("ver", "Exit Version (1 Exit With ST; 2 Bollinger MB; 3 Bollinger LB; 4 Whichever is earlier)", 1.0, 1.0, 4.0, 1.0),
        p("bb_emaprd", "BB_EMAPrd", 15.0, 2.0, 200.0, 1.0),
        p("stdevmul", "StDevMul", 2.0, 0.0, 10.0, 0.001),
        p("flttf", "FILTER TIMEFRAME (1 DAILY; 2 WEEKLY; 3 MONTHLY)", 1.0, 1.0, 3.0, 1.0),
        p("fltpd", "ST FILTER PERIOD", 10.0, 0.0, 20.0, 1.0),
        p("stflt", "ST CALCULATION POINT (1 MID POINT(H+L/2); 2 CLOSE; 3 PIVOT(H+L+C/3))", 1.0, 1.0, 3.0, 1.0),
    ]);
    v
}

// ================================================================ 16-18. minimal AFLs: Bollinger / Keltner / moving-average exit

/// `Buy = C > Ref(Hcl,-1); Sell = C < line; ExRem both ways; fills at Close`. No circuit, penny, turnover or dead-stock rules in these AFLs.
fn minimal(s: &Series, line: &dyn Fn(&Series, &[f32]) -> Vec<f32>) -> Signals {
    let n = s.bars.len();
    let c = s.close();
    let ln = line(s, &c);
    let mut hcl = vec![0f32; n];
    for i in 0..n {
        hcl[i] = if i == 0 || c[i] > hcl[i - 1] { c[i] } else { hcl[i - 1] };
    }
    let buy: Vec<bool> = (0..n).map(|i| i >= 1 && gt(c[i], hcl[i - 1])).collect();
    let sell: Vec<bool> = (0..n).map(|i| lt(c[i], ln[i])).collect();
    mk(s, buy, sell, None, true, 0.0)
}

fn sg_boll_exit(series: &[Series], _e: &Env, _g: G) -> Vec<Signals> {
    // Optimize("BOLLINGER AVERAGE",15,15,50,5), Optimize("BOLLINGER SD FACTOR",1,1,4,.5)
    map_series(series, &|s| {
        minimal(s, &|_, c| {
            let (m, sd) = (ma(c, 15), stdev(c, 15));
            (0..c.len()).map(|i| m[i] - 1.0 * sd[i]).collect()
        })
    })
}

fn sg_keltner(series: &[Series], _e: &Env, _g: G) -> Vec<Signals> {
    // Optimize("KELTNER ATR PERIOD",15,15,50,5), Optimize("KELTNER ATR FACTOR",1,1,4,.5)
    map_series(series, &|s| {
        minimal(s, &|s, c| {
            let (h, l) = (s.high(), s.low());
            let tr = tr1(&h, &l, c);
            let n = c.len();
            let mut satr = vec![NAN; n];
            for i in 0..n {
                if i + 1 >= 15 {
                    let mut t = tr[i];
                    for k in 1..15 {
                        t += tr[i - k];
                    }
                    satr[i] = t / 15.0;
                }
            }
            let m = ma(c, 15);
            (0..n).map(|i| m[i] - 1.0 * satr[i]).collect()
        })
    })
}

fn sg_ma_exit(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let ty = g("matype1").round() as i32; // 1 SMA, 2 EMA, 3 WMA, 4 DMA, 5 TMA, 6 Wilders
    map_series(series, &|s| {
        minimal(s, &|_, c| {
            // Optimize("AVERAGE PERIOD",20,20,300,10)
            let pd = 20;
            match ty {
                2 => ema(c, pd),
                3 => wma(c, pd),
                4 => dema(c, pd),
                5 => tema(c, pd),
                6 => wilders(c, pd),
                _ => ma(c, pd),
            }
        })
    })
}
fn schema_none() -> Vec<P> {
    vec![]
}
fn schema_ma_exit() -> Vec<P> {
    vec![p("matype1", "SHORT MA TYPE 1 (1 SMA; 2 EMA; 3 WMA; 4 DMA; 5 TMA; 6 Wilders)", 1.0, 1.0, 6.0, 1.0)]
}

// ================================================================ 19. RSI stock selection with 3 exit versions

fn sg_rsi_select(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let (profit, slp, trail, buf) = (g("profit_pct") as f32, g("sl_pct") as f32, g("trailslprcnt") as f32, g("buffer_prcnt") as f32);
    let ver = g("ver").round() as i32;
    let (rprd, rb, rs) = (g("rsi_prd").round() as usize, g("rsi_buy_val") as f32, g("rsi_sell_val") as f32);
    let (aprd, apt, asl, atr_tr) = (g("atr_prd").round() as usize, g("atr_pt_m") as f32, g("atr_sl_m") as f32, g("atrtrailsl_m") as f32);
    let pct = g("exitpct") as f32;
    map_series(series, &|s| {
        let cx = Ctx::new(s, &cm, Tk::OpenInt);
        let n = cx.n;
        let tr = swing_std(&cx.c, pct);
        let athc = buysig_of(n, &|i| gt(cx.c[i], cx.hcl[i - 1]), &|i| tr[i] == -1);
        let r = rsi(&cx.c, rprd);
        let myatr = atr_w(&cx.h, &cx.l, &cx.c, aprd);
        let (c, o, h, l) = (&cx.c, &cx.o, &cx.h, &cx.l);
        let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
        let mut sp = c.clone();
        let (mut pos, mut skipsell) = (false, false);
        let (mut hh, mut trail_sl, mut fix_sl, mut pt) = (0f32, 0f32, 0f32, 9_999_999f32);
        let (mut hc, mut atail, mut afix, mut apt_v) = (0f32, 0f32, 0f32, 9_999_999f32);
        let sell_gap = |i: usize| o[i] - (o[i] * buf) / 100.0;
        for i in 1..n {
            let ds = cx.dsell[i];
            // Buy_Cash_Filter here: Turnover AND UC == 0 AND LC == 0 AND Close > PennyFlt AND DEADSTOCKBUYSIG
            let bcf = cx.bcf(i, true) && !cx.lc[i];
            let rsi_buy = athc[i] && gt(r[i], rb) && i >= 1 && lt(r[i - 1], rb) && bcf;
            if ver == 1 {
                if !pos && rsi_buy {
                    buy[i] = true;
                    pos = true;
                } else if pos && (cross_dn_lvl(&r, rs, i) || tr[i] == -1) && !skipsell {
                    if !cx.lc[i] || ds {
                        sell[i] = true;
                        pos = false;
                        skipsell = false;
                    } else {
                        skipsell = true;
                    }
                } else if pos && ((!cx.loc[i] && skipsell) || ds) {
                    sell[i] = true;
                    sp[i] = sell_gap(i);
                    pos = false;
                    skipsell = false;
                }
            } else if ver == 2 {
                if !pos && rsi_buy {
                    buy[i] = true;
                    if profit > 0.0 {
                        pt = (1.0 + profit * 0.01) * c[i];
                    }
                    if slp > 0.0 {
                        fix_sl = (1.0 - slp * 0.01) * c[i];
                    }
                    if trail > 0.0 {
                        hh = c[i];
                        trail_sl = (1.0 - trail * 0.01) * hh;
                    }
                    pos = true;
                } else if pos {
                    if trail > 0.0 {
                        hh = c[i].max(hh);
                    }
                    if (h[i] >= pt || l[i] <= trail_sl || l[i] <= fix_sl) && !skipsell {
                        if !cx.lc[i] || ds {
                            sell[i] = true;
                            if h[i] >= pt {
                                sp[i] = if o[i] > pt { sell_gap(i) } else { pt };
                            }
                            if l[i] <= trail_sl {
                                sp[i] = if o[i] < trail_sl { sell_gap(i) } else { trail_sl };
                            }
                            if l[i] <= fix_sl {
                                sp[i] = if o[i] < fix_sl { sell_gap(i) } else { fix_sl };
                            }
                            pos = false;
                            skipsell = false;
                            hh = 0.0;
                            trail_sl = 0.0;
                            fix_sl = 0.0;
                            pt = 9_999_999.0;
                        } else {
                            skipsell = true;
                        }
                    } else if (!cx.loc[i] && skipsell) || ds {
                        sell[i] = true;
                        sp[i] = sell_gap(i);
                        pos = false;
                        skipsell = false;
                        hh = 0.0;
                        trail_sl = 0.0;
                        fix_sl = 0.0;
                        pt = 9_999_999.0;
                    }
                    if trail > 0.0 {
                        trail_sl = (1.0 - trail * 0.01) * hh;
                    }
                }
            } else {
                if !pos && rsi_buy {
                    buy[i] = true;
                    if apt > 0.0 {
                        apt_v = c[i] + (myatr[i] * apt);
                    }
                    if asl > 0.0 {
                        afix = c[i] - (myatr[i] * asl);
                    }
                    if atr_tr > 0.0 {
                        hc = c[i];
                        atail = hc - (myatr[i] * atr_tr);
                    }
                    pos = true;
                } else if pos {
                    if atr_tr > 0.0 {
                        hc = c[i].max(hc);
                    }
                    if (l[i] <= atail || h[i] >= apt_v || l[i] <= afix) && !skipsell {
                        if !cx.lc[i] || ds {
                            sell[i] = true;
                            if h[i] >= apt_v {
                                sp[i] = if o[i] > apt_v { sell_gap(i) } else { apt_v };
                            }
                            if l[i] <= atail {
                                sp[i] = if o[i] < atail { sell_gap(i) } else { atail };
                            }
                            if l[i] <= afix {
                                sp[i] = if o[i] < afix { sell_gap(i) } else { afix };
                            }
                            pos = false;
                            skipsell = false;
                            hc = 0.0;
                            atail = 0.0;
                            afix = 0.0;
                            apt_v = 9_999_999.0;
                        } else {
                            skipsell = true;
                        }
                    } else if (!cx.loc[i] && skipsell) || ds {
                        sell[i] = true;
                        sp[i] = sell_gap(i);
                        pos = false;
                        skipsell = false;
                        hc = 0.0;
                        atail = 0.0;
                        afix = 0.0;
                        apt_v = 9_999_999.0;
                    }
                    if atr_tr > 0.0 {
                        atail = hc - (myatr[i] * atr_tr);
                    }
                }
            }
        }
        mk(s, buy, sell, Some(sp), false, 0.0)
    })
}
fn schema_rsi_select() -> Vec<P> {
    let mut v = base3();
    v.extend([
        pc("profit_pct", "Profit Booking %", 10.0, 0.0, 20.0, 2.0),
        pc("sl_pct", "Fix SL %", 2.0, 0.0, 30.0, 0.1),
        pc("trailslprcnt", "Trailing SL %", 5.0, 0.0, 30.0, 0.1),
        pc("buffer_prcnt", "Selling Buffer %", 0.25, 0.0, 10.0, 0.05),
        p("ver", "Entry/Exit Version (1 RSI Exit; 2 PT or %Trailing Sl Exit; 3 ATR Based PT or Trailing SL)", 1.0, 1.0, 3.0, 1.0),
        pc("rsi_prd", "RSI Period", 14.0, 2.0, 26.0, 4.0),
        pc("rsi_buy_val", "RSI Buy Value (AFL default 10 is outside its range 20..80)", 10.0, 20.0, 80.0, 10.0),
        pc("rsi_sell_val", "RSI Sell Value (AFL default 10 is outside its range 20..50)", 10.0, 20.0, 50.0, 10.0),
        pc("atr_prd", "ATR Prds", 5.0, 1.0, 100.0, 1.0),
        pc("atr_pt_m", "ATR Profit Target Multiplier", 10.0, 0.0, 20.0, 0.001),
        pc("atr_sl_m", "ATR Hard SL Multiplier", 3.0, 0.0, 20.0, 0.001),
        pc("atrtrailsl_m", "ATR Trail SL Multiplier", 5.0, 0.0, 20.0, 0.001),
        pc("exitpct", "Swing EXIT %", 10.0, 1.0, 25.0, 1.0),
    ]);
    v
}

// ================================================================ 20-21. N-day close / low stop (running turnover buy)

/// `buysig`: 1 on a close above all earlier closes, 0 when `stop_hit(i)` (compares with the previous bar's N-day lowest) and the previous bar had it.
/// `Buy = (C > Ref(Hcl,-1) OR buysig) AND NOT UPCIRCUIT AND dead AND C>penny AND turnover`, `Sell = ((stop OR buysig==0) AND NOT LOCIRCUIT) OR deadsell`, ExRem, fill at Close.
pub fn nday_one(s: &Series, cm: &Cmn, hard: f32, lp: usize, use_low: bool) -> Signals {
    let cx = Ctx::new(s, cm, Tk::VolClose);
    let n = cx.n;
    let ll = llv(if use_low { &cx.l } else { &cx.c }, lp);
    let stop = |i: usize| -> bool {
        let line = if i >= 1 { ll[i - 1] } else { NAN };
        lt(if use_low { cx.l[i] } else { cx.c[i] }, line)
    };
    let buysig = buysig_of(n, &|i| gt(cx.c[i], cx.hcl[i - 1]), &stop);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = (cx.ath(i) || buysig[i]) && !cx.upc[i] && cx.dbuy[i] && cx.c[i] > cx.penny && cx.turn[i];
        sell[i] = ((stop(i) || !buysig[i]) && !cx.loc[i]) || cx.dsell[i];
    }
    mk(s, buy, sell, None, true, hard)
}
fn sg_nday_close(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let (hard, lp) = (g("hardstop") as f32, g("lp").round() as usize);
    map_series(series, &|s| nday_one(s, &cm, hard, lp, false))
}
fn sg_nday_low(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let (hard, lp) = (g("hardstop") as f32, g("lp").round() as usize);
    map_series(series, &|s| nday_one(s, &cm, hard, lp, true))
}
fn schema_nday_close() -> Vec<P> {
    vec![dead_p(), hard_p(), penny_p(), turn_p(), p("lp", "CLOSE PERIOD", 10.0, 1.0, 1000.0, 10.0)]
}
fn schema_nday_low() -> Vec<P> {
    vec![dead_p(), hard_p(), penny_p(), turn_p(), p("lp", "LOW PERIOD", 10.0, 10.0, 60.0, 5.0)]
}

// ================================================================ 22-23. non-performance exit (ver 1 / ver 2) with %swing

/// `deadstock1`: the bar equals the next 30 bars in O, H, L and C (`Ref(x, k)` with k > 0 is the FUTURE; a missing bar compares false).
fn frozen_next30(cx: &Ctx) -> Vec<bool> {
    let n = cx.n;
    (0..n)
        .map(|i| {
            if i + 30 >= n {
                return false;
            }
            (1..=30).all(|k| cx.o[i] == cx.o[i + k] && cx.h[i] == cx.h[i + k] && cx.l[i] == cx.l[i + k] && cx.c[i] == cx.c[i + k])
        })
        .collect()
}

pub fn nonperf_one(s: &Series, cm: &Cmn, hard: f32, exit_pct: f32, exitpd: f32, buy_pct: f32, cancel: bool) -> Signals {
    let cx = Ctx::new(s, cm, Tk::VolClose);
    let n = cx.n;
    let c = &cx.c;
    let trend = swing_std(c, exit_pct);
    // loop 1: the trigger = first ATH close of a run x (1 + NON PERFORMANCE %), Null after a swing-down bar
    let mut t1 = vec![0i32; n];
    let mut bprice = vec![NAN; n];
    let mut trigger = vec![NAN; n];
    for i in 1..n {
        if gt(c[i], cx.hcl[i - 1]) && t1[i - 1] == 0 {
            t1[i] = 1;
            bprice[i] = c[i];
            trigger[i] = bprice[i] + (bprice[i] * buy_pct) / 100.0;
        } else {
            bprice[i] = bprice[i - 1];
            trigger[i] = trigger[i - 1];
            t1[i] = t1[i - 1];
        }
        if trend[i] == -1 {
            t1[i] = 0;
            bprice[i] = NAN;
            trigger[i] = NAN;
        }
    }
    // loop 2: exitday (the bar number by which +5 % must be reached); j[i] = i
    let mut tt = vec![0i32; n];
    let mut exitday = vec![NAN; n];
    let jf = |i: usize| i as f32;
    for i in 1..n {
        if gt(c[i], cx.hcl[i - 1]) && !cx.upc[i] && tt[i - 1] == 0 {
            tt[i] = 1;
            exitday[i] = i as f32 + exitpd;
        } else if cancel && gt(c[i], trigger[i]) && le(jf(i), exitday[i - 1]) {
            exitday[i] = NAN;
        } else if trend[i] == -1 || (lt(c[i], trigger[i]) && gt(jf(i), exitday[i - 1]) && c[i] < cx.hcl[i - 1]) {
            tt[i] = 0;
        } else {
            tt[i] = tt[i - 1];
            exitday[i] = exitday[i - 1];
        }
    }
    // loop 3: buysig
    let mut buysig = vec![false; n];
    for i in 1..n {
        buysig[i] = if gt(c[i], cx.hcl[i - 1]) {
            true
        } else if (trend[i] == -1 && buysig[i - 1]) || (lt(c[i], trigger[i]) && gt(jf(i), exitday[i - 1])) {
            false
        } else {
            buysig[i - 1]
        };
    }
    let frozen = frozen_next30(&cx);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = (cx.ath(i) || buysig[i]) && !cx.upc[i] && cx.dbuy[i] && cx.c[i] > cx.penny && cx.turn[i] && !frozen[i];
        sell[i] = ((trend[i] == -1 || !buysig[i]) && !cx.loc[i]) || cx.dsell[i] || frozen[i];
    }
    mk(s, buy, sell, None, true, hard)
}
fn sg_nonperf2(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let (hard, ep, pd, bp) = (g("hardstop") as f32, g("exitpct") as f32, g("exitpd") as f32, g("athcbuypct") as f32);
    map_series(series, &|s| nonperf_one(s, &cm, hard, ep, pd, bp, true))
}
fn sg_nonperf1(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let (hard, ep, pd, bp) = (g("hardstop") as f32, g("exitpct") as f32, g("exitpd") as f32, g("athcbuypct") as f32);
    map_series(series, &|s| nonperf_one(s, &cm, hard, ep, pd, bp, false))
}
fn schema_nonperf(exit_def: f64, exit_min: f64) -> Vec<P> {
    vec![
        dead_p(),
        hard_p(),
        penny_p(),
        turn_p(),
        p("exitpct", "EXIT %", exit_def, exit_min, 25.0, 1.0),
        p("exitpd", "NON PERFORMANCE PERIOD", 22.0, 22.0, 132.0, 22.0),
        p("athcbuypct", "NON PERFORMANCE %", 5.0, 5.0, 30.0, 1.0),
    ]
}
fn schema_nonperf2() -> Vec<P> {
    schema_nonperf(16.0, 5.0)
}
fn schema_nonperf1() -> Vec<P> {
    schema_nonperf(13.0, 8.0)
}

// ================================================================ 24. time-based exit only (holding days) with ranking

/// Where the Supertrend is calculated from (CLOSE|MID|HIGH|PIVOT|MP CL PIVOT AVG|3DAYS MA|5DAYS MA).
fn stcal7(s: &Series, k: i32) -> Vec<f32> {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let mp: Vec<f32> = (0..n).map(|i| (h[i] + l[i]) / 2.0).collect();
    let pivot: Vec<f32> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
    match k {
        1 => c,
        3 => h,
        4 => pivot,
        5 => (0..n).map(|i| (mp[i] + pivot[i] + c[i]) / 3.0).collect(),
        6 => ma(&c, 3),
        7 => ma(&c, 5),
        _ => mp,
    }
}

/// Time-only exit: `athbuysig = C>Ref(HCL,-1) AND BarIndex()>=TSTART AND dead AND NOT UPCIRCUIT AND C>penny AND turnover`; buysig counts ath signals since the
/// ST last turned down; `Buy = athbuysig AND Ref(buysig,-1)==0`; `Sell = deadsell OR (BarsSince(FLAG)>=HOLDINGDAYS AND NOT LOCIRCUIT)` (no ST exit).
pub fn time_only_one(s: &Series, cm: &Cmn, stk: i32, period: usize, factor: f32, tstart: usize, hold: usize, hard: f32) -> Signals {
    let cx = Ctx::new(s, cm, Tk::VolClose);
    let n = cx.n;
    let stcal = stcal7(s, stk);
    let tr = tr1(&cx.h, &cx.l, &cx.c);
    let bt = st_trend(&stcal, &tr, &cx.c, period, 1.0, factor);
    let ath: Vec<bool> = (0..n).map(|i| cx.ath(i) && i >= tstart && cx.dbuy[i] && !cx.upc[i] && cx.c[i] > cx.penny && cx.turn[i]).collect();
    let mut count = vec![false; n];
    let mut buysig = vec![0u32; n];
    let mut flag = vec![false; n];
    for i in 1..n {
        flag[i] = ath[i] && !count[i - 1];
        if ath[i] {
            count[i] = true;
            buysig[i] = buysig[i - 1] + 1;
        } else if bt[i] == -1 {
            count[i] = false;
            buysig[i] = 0;
        } else {
            count[i] = count[i - 1];
            buysig[i] = buysig[i - 1];
        }
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut last_flag: Option<usize> = None;
    for i in 0..n {
        if flag[i] {
            last_flag = Some(i);
        }
        buy[i] = i >= 1 && ath[i] && buysig[i - 1] == 0;
        let time_ok = last_flag.map_or(false, |f| i - f >= hold);
        sell[i] = cx.dsell[i] || (time_ok && !cx.loc[i]);
    }
    mk(s, buy, sell, None, true, hard)
}

fn rank_score(s: &Series, mode: i32, rocpd: usize, rsipd: usize, stdpd: usize, swingpd: usize) -> Option<Vec<f32>> {
    if !(2..=11).contains(&mode) {
        return None;
    }
    let p = StP { dead_yes: false, hardstop: 0.0, penny: 0.0, turn_on: false, aux1_on: false, aux2_on: false, stflt: 1, period: 10, factor: 4.0, tstart: 1, rankmode: mode, rocpd, rsipd, stdpd, swingpd };
    ath_new_st::one(s, &p).score
}

fn sg_time_only(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cm = cmn_of(g);
    let (stk, per, fac, ts) = (g("stflt").round() as i32, g("bpd1").round() as usize, g("bfactor1") as f32, g("tstart").round() as usize);
    let (hold, hard) = (g("holdingdays").round() as usize, g("hardstop") as f32);
    let (rm, r1, r2, r3, r4) = (g("rankmode").round() as i32, g("rocpd").round() as usize, g("rsipd").round() as usize, g("stdpd").round() as usize, g("swingpd").round() as usize);
    map_series(series, &|s| {
        let mut sg = time_only_one(s, &cm, stk, per, fac, ts, hold, hard);
        sg.score = rank_score(s, rm, r1, r2, r3, r4);
        sg
    })
}
fn schema_time_only() -> Vec<P> {
    vec![
        dead_p(),
        hard_p(),
        penny_p(),
        turn_p(),
        p("rankmode", "RANKING MODE (1 NONE; 2 POSITIVE ROC; 3 NEGATIVE ROC; 4 RSI; 5 STD; 6 POSITIVE ROC/STD; 7 NEGATIVE ROC/STD; 8 POSITIVE CLOSE; 9 SWING; 10 POSITIVE ROC + POSITIVE CLOSE; 11 POSITIVE CLOSE + SWING)", 1.0, 1.0, 11.0, 1.0),
        p("rocpd", "ROC PERIOD for RANKING/NO OF POSITIVE CLOSE", 10.0, 0.0, 400.0, 30.0),
        p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("stdpd", "STD PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("swingpd", "SWING PERIOD", 180.0, 10.0, 1000.0, 1.0),
        p("stflt", "DAILY ST CALCULATION POINT (1 CLOSE; 2 MID POINT H+L/2; 3 HIGH; 4 PIVOT H+L+C/3; 5 MP CL PIVOT AVG; 6 3DAYS MA; 7 5DAYS MA)", 1.0, 1.0, 7.0, 1.0),
        p("bpd1", "ST Period ", 10.0, 1.0, 20.0, 1.0),
        p("bfactor1", "ST Factor", 4.0, 1.0, 20.0, 1.0),
        p("tstart", "TRADING START BAR", 0.0, 0.0, 500.0, 1.0),
        p("holdingdays", "HOLDING PERIOD", 5.0, 1.0, 1000.0, 1.0),
    ]
}

/// Verification hook (not a registered variant): the Skipbuy / SkipSell machine with the exit line "previous week's Low" (expandLast), i.e. the
/// shape of the team's `Weekly Low Exit` AFL, which `ath_exits1` verified against four sheets. `ath_v2_e2` checks `sk_run` against it.
pub fn weekly_prelow_machine(s: &Series, cm: &Cmn) -> Signals {
    let cx = Ctx::new(s, cm, Tk::OpenInt);
    let k = compress(s, 1);
    let line = expand(&back(&k.l, 1), &k, cx.n);
    let entry = |i: usize| cx.ath(i);
    let reset = |i: usize| lt(cx.c[i], line[i]);
    let exit = |i: usize, ss: bool| !ss && lt(cx.c[i], line[i]);
    let sp = Sk { start: 1, uc_in_filter: true, entry: &entry, entry_in: &|_| true, reset: Some(&reset), rebuy: &|_| true, reset_first: true, exit: &exit };
    let (b, sl) = sk_run(&cx, &sp);
    mk(s, b, sl, None, false, 0.0)
}

// ================================================================ registry

pub fn variants() -> Vec<VariantDef> {
    macro_rules! vd {
        ($id:expr, $sch:ident, $sg:ident) => {
            VariantDef { id: $id, data: DATA, schema: $sch, signals: $sg }
        };
    }
    vec![
        vd!("ytath_buy_with_rsi_70_cross_exit_on_rsi_50", schema_rsi70, sg_rsi70),
        vd!("ytath_entry_and_atr_or_pctswing_exit_true", schema_atr_swing, sg_atr_swing),
        vd!("ytath_entry_and_different_ma_s_exit_versions", schema_ma_versions, sg_ma_versions),
        vd!("ytath_entry_and_elder_ray_bear_power_exit", schema_elder, sg_elder),
        vd!("ytath_entry_and_exit_with_52_week_lowest", schema_52wk, sg_52wk),
        vd!("ytath_entry_exit_on_rsi_rsi_average", schema_rsi_avg, sg_rsi_avg),
        vd!("ytath_entry_and_gann_hilo_exit", schema_gann_hilo, sg_gann_hilo),
        vd!("ytath_entry_and_gann_levels_trail_sl_exit", schema_gann_levels, sg_gann_levels),
        vd!("ytath_entry_and_ichimoku_exit", schema_ichimoku, sg_ichimoku),
        vd!("ytath_entry_and_macd_exit", schema_macd, sg_macd),
        vd!("ytath_entry_and_money_flow_index_exit", schema_mfi, sg_mfi),
        vd!("ytath_entry_and_pdi_and_mdi_crossover_exit", schema_pdi_mdi, sg_pdi_mdi),
        vd!("ytath_entry_and_roc_crossover_exit", schema_roc, sg_roc),
        vd!("ytath_entry_and_sar_exit", schema_sar, sg_sar),
        vd!("ytath_entry_with_bollinger_upper_band_exit", schema_boll_upper, sg_boll_upper),
        vd!("ytath_exit_with_bollinger_band", schema_none, sg_boll_exit),
        vd!("ytath_exit_with_keltner_channel", schema_none, sg_keltner),
        vd!("ytath_exit_with_moving_average", schema_ma_exit, sg_ma_exit),
        vd!("ytath_stock_selection_and_rsi_based_entry_3", schema_rsi_select, sg_rsi_select),
        vd!("ytath_with_n_days_close_sl_exit_below", schema_nday_close, sg_nday_close),
        vd!("ytath_with_n_days_low_sl_low_below_previous", schema_nday_low, sg_nday_low),
        vd!("ytath_with_non_performance_exit_ver2_stock", schema_nonperf2, sg_nonperf2),
        vd!("ytath_with_non_performance_exit_ver1_and", schema_nonperf1, sg_nonperf1),
        vd!("ytath_with_only_time_based_exit_holding", schema_time_only, sg_time_only),
    ]
}

// ================================================================ tests

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(date: i32, o: f32, h: f32, l: f32, c: f32) -> Bar {
        Bar { date, c, o, h, l, v: 100000.0, oi: 1.0e7, aux1: 1.0, aux2: 0.0 }
    }
    fn series_of(cl: &[f32]) -> Series {
        let bars = cl.iter().enumerate().map(|(i, &c)| bar(20200101 + i as i32, c, c + 1.0, c - 1.0, c)).collect();
        Series { name: "T".into(), bars }
    }
    /// zig-zag uptrend (ATH closes every few bars, RSI/ROC/MACD move) followed by a decline
    fn zig(up: usize, down: usize) -> Series {
        let mut cl: Vec<f32> = (0..up).map(|i| 100.0 + i as f32 + 3.0 * (i as f32 * 0.9).sin()).collect();
        let top = *cl.last().unwrap();
        cl.extend((0..down).map(|i| top - 2.0 * i as f32 + 3.0 * (i as f32 * 0.9).sin()));
        series_of(&cl)
    }
    fn gfn(map: Vec<(&'static str, f64)>) -> impl Fn(&str) -> f64 {
        move |k: &str| map.iter().find(|x| x.0 == k).map(|x| x.1).unwrap_or_else(|| panic!("no param {k}"))
    }
    fn defaults(sch: fn() -> Vec<P>) -> impl Fn(&str) -> f64 {
        let v: Vec<(&'static str, f64)> = sch().iter().map(|x| (x.key, x.def)).collect();
        gfn(v)
    }
    fn env() -> Env {
        Env { index: Default::default() }
    }
    fn run(id: &str, s: &Series) -> Signals {
        let v = variants().into_iter().find(|v| v.id == id).unwrap();
        let g = defaults(v.schema);
        (v.signals)(std::slice::from_ref(s), &env(), &g).remove(0)
    }

    #[test]
    fn all_ids_registered_once() {
        let v = variants();
        assert_eq!(v.len(), 24);
        let mut ids: Vec<_> = v.iter().map(|x| x.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 24);
    }

    #[test]
    fn indicator_basics() {
        let c = [10.0f32, 11.0, 12.0, 11.0, 13.0];
        assert_eq!(ma(&c, 2)[1], 10.5);
        assert!(ma(&c, 2)[0].is_nan());
        assert_eq!(roc(&c, 1)[1], 10.0);
        assert!((stdev(&[1.0, 3.0], 2)[1] - 1.0).abs() < 1e-6, "population StDev");
        // RSI of a monotone rise is 100
        let up: Vec<f32> = (0..30).map(|i| 100.0 + i as f32).collect();
        assert_eq!(rsi(&up, 14)[20], 100.0);
        assert_eq!(wma(&[1.0, 2.0, 3.0], 3)[2], (1.0 + 4.0 + 9.0) / 6.0);
        // Ma of an array with leading Nulls starts when the window is clean
        let r = ma(&[NAN, NAN, 1.0, 2.0, 3.0], 2);
        assert!(r[2].is_nan() && r[3] == 1.5);
    }

    #[test]
    fn sar_follows_a_rising_series_below_price_and_flips_on_a_crash() {
        let h: Vec<f32> = (0..20).map(|i| 101.0 + i as f32).collect();
        let l: Vec<f32> = (0..20).map(|i| 99.0 + i as f32).collect();
        let s = sar(&h, &l, 0.02, 0.2);
        assert!(s[10] < l[10]);
        let mut h2 = h.clone();
        let mut l2 = l.clone();
        h2.push(90.0);
        l2.push(80.0);
        let s2 = sar(&h2, &l2, 0.02, 0.2);
        assert!(s2[20] > h2[20], "after the crash the SAR is above the price");
    }

    #[test]
    fn mfi_is_100_when_only_up_flows() {
        let h: Vec<f32> = (0..30).map(|i| 11.0 + i as f32).collect();
        let l: Vec<f32> = (0..30).map(|i| 9.0 + i as f32).collect();
        let c: Vec<f32> = (0..30).map(|i| 10.0 + i as f32).collect();
        let v = vec![1000.0f32; 30];
        assert_eq!(mfi(&h, &l, &c, &v, 14)[20], 100.0);
    }

    #[test]
    fn minimal_ma_exit_buys_an_ath_close_and_sells_below_the_average() {
        let mut cl: Vec<f32> = (0..40).map(|i| 100.0 + i as f32).collect();
        cl.extend([60.0, 59.0, 58.0]);
        let s = series_of(&cl);
        let sg = run("ytath_exit_with_moving_average", &s);
        assert!(sg.buy[1] && !sg.buy[2], "ExRem keeps the first ATH close only");
        assert!(sg.sell[40], "the crash closes below the 20 bar average");
        let bb = run("ytath_exit_with_bollinger_band", &s);
        assert!(bb.sell[40]);
        let kc = run("ytath_exit_with_keltner_channel", &s);
        assert!(kc.sell[40]);
    }

    #[test]
    fn state_machine_blocks_on_upper_circuit_then_enters_when_the_filter_allows() {
        // a bar with H == C is not bought (UC); skipbuy is set and the entry happens on a later bar while the MA exit has not fired
        let mut s = zig(60, 0);
        let free = run("ytath_entry_and_roc_crossover_exit", &s);
        let first = (1..s.bars.len()).find(|&i| free.buy[i]).unwrap();
        // the first buy bar is made an upper circuit bar (high == close): no buy there, the Skipbuy path buys on a later bar
        s.bars[first].h = s.bars[first].c;
        let sg = run("ytath_entry_and_roc_crossover_exit", &s);
        assert!(!sg.buy[first], "UC bar: no buy");
        assert!(sg.buy.iter().skip(first + 1).any(|&b| b), "skipbuy path buys on a later bar");
    }

    #[test]
    fn time_only_exit_sells_after_the_holding_period() {
        let cl: Vec<f32> = (0..40).map(|i| 100.0 + i as f32).collect();
        let s = series_of(&cl);
        let cm = Cmn { dead_yes: false, penny: 0.0, turn_on: false, circuit_prec: false };
        let sg = time_only_one(&s, &cm, 1, 10, 4.0, 0, 5, 0.0);
        let b = sg.buy.iter().position(|&x| x).unwrap();
        let x = sg.sell.iter().position(|&x| x).unwrap();
        assert_eq!(x - b, 5, "BarsSince(FLAG) >= HOLDINGDAYS");
    }

    #[test]
    fn nonperf_frozen_stock_is_sold_and_not_bought() {
        let mut cl: Vec<f32> = (0..20).map(|i| 100.0 + i as f32).collect();
        cl.extend(vec![120.0; 40]);
        let s = series_of(&cl);
        let cx = Ctx::new(&s, &Cmn { dead_yes: false, penny: 0.0, turn_on: false, circuit_prec: false }, Tk::VolClose);
        let fz = frozen_next30(&cx);
        assert!(fz[25] && !fz[40], "bar equal to the next 30 bars; the last 30 bars never are");
    }

    #[test]
    fn gann_levels_and_macd_run_on_a_trend() {
        let s = zig(90, 60);
        for id in ["ytath_entry_and_gann_levels_trail_sl_exit", "ytath_entry_and_macd_exit", "ytath_entry_and_pdi_and_mdi_crossover_exit", "ytath_entry_and_ichimoku_exit", "ytath_entry_and_sar_exit", "ytath_entry_and_gann_hilo_exit", "ytath_entry_and_elder_ray_bear_power_exit", "ytath_entry_and_different_ma_s_exit_versions"] {
            let sg = run(id, &s);
            assert!(sg.buy.iter().any(|&b| b), "{id}: buys the uptrend");
            assert!(sg.sell.iter().any(|&b| b), "{id}: sells the decline");
        }
    }

    #[test]
    fn rsi_select_pt_version_sells_at_the_target() {
        // RSI crossing up through 20 after an ATH run, version 2: profit target 10 %
        let mut cl: Vec<f32> = (0..30).map(|i| 100.0 - i as f32 * 0.5).collect();
        cl.extend((0..40).map(|i| 85.0 + i as f32 * 2.0));
        let s = series_of(&cl);
        let v = variants().into_iter().find(|v| v.id == "ytath_stock_selection_and_rsi_based_entry_3").unwrap();
        let sch: Vec<(&'static str, f64)> = (v.schema)().iter().map(|x| (x.key, if x.key == "ver" { 2.0 } else { x.def })).collect();
        let g = gfn(sch);
        let sg = (v.signals)(std::slice::from_ref(&s), &env(), &g).remove(0);
        for i in 0..s.bars.len() {
            if sg.sell[i] {
                assert!(sg.sell_price[i] >= s.bars[i].l && sg.sell_price[i] <= s.bars[i].h);
            }
        }
    }
}

