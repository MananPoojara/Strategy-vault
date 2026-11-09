//! ATH sub-variants, batch d2 (16 AFLs): BTST / intraday / live-market-entry ATHC AFLs, the ATHC 5 % to 30 % scanners, the AFLs that only plot or scan
//! (no Buy / Sell at all), the long + short combo and the ATHC-5 % range short.
//!
//! One `VariantDef` per sub-variant, registered through `ath_registry.rs`. Standard library only (plus `ath_core`, `ath_new_scan`). Every rule is coded
//! from the local AFL copy (`ATH/afl_copy/<id>.afl`) in float32 like AmiBroker, AFL mistakes copied as written (decision D11). The data are DAILY bars:
//! an intraday fill is modelled exactly as the AFL's own text defines it from the day's Open / High / Low / Close (fills clamped to [Low, High]).
//!
//! Core features this batch needed (additive, in `ath_core.rs`): `Signals::same_bar_exit` (AllowSameBarExit: Buy and Sell on one bar of a flat symbol = one
//! round trip on that bar), `Signals::short_leg` (Short / Cover with their own prices), `size_pct`, `max_long` / `max_short`, `TradeOut::short`.
//! Parameter convention (catalog `eng`): ParamToggle / ParamList values are the AFL's 0-based value + 1 (deadstock 1 YES / 2 NO, turnflt 1 NO / 2 YES).
//! Verification notes: `ATH/v2/<id>/VERIFY.md`.
#![allow(dead_code)]
use crate::ath_core::*;
use crate::ath_new_scan as scan;

const CASH: Data = Data::Cash22 { watchlist: "NSE 500 C&P.tls" };
const FO: Data = Data::Fo22 { watchlist: "FO STOCKS C&P.tls" };

// ---------------------------------------------------------------- the bar arrays every AFL here shares

struct Px {
    n: usize,
    date: Vec<i32>,
    c: Vec<f32>,
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    v: Vec<f32>,
    oi: Vec<f32>,
    /// HHV(C, BarCount): highest close up to the bar
    hcl: Vec<f32>,
    upc: Vec<bool>,
    loc: Vec<bool>,
    /// H == C, L == C
    uc: Vec<bool>,
    lc: Vec<bool>,
}

fn px(s: &Series) -> Px {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let mut hcl = vec![0f32; n];
    for i in 0..n {
        hcl[i] = if i == 0 || c[i] > hcl[i - 1] { c[i] } else { hcl[i - 1] };
    }
    Px {
        n,
        date: s.bars.iter().map(|b| b.date).collect(),
        upc: (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i])).collect(),
        loc: (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i])).collect(),
        uc: (0..n).map(|i| h[i] == c[i]).collect(),
        lc: (0..n).map(|i| l[i] == c[i]).collect(),
        v: s.volume(),
        oi: s.openint(),
        hcl,
        c,
        o,
        h,
        l,
    }
}

/// Yearly turnover threshold of the older AFLs (compared with Volume x Close).
fn volsig_a(year: i32) -> f32 {
    match year {
        y if y < 2000 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

/// Yearly turnover threshold of the newer AFLs (compared with OpenInt = turnover).
fn volsig_b(year: i32) -> f32 {
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

/// `Turnover`: table A (Volume x Close) or table B (OpenInt), or 1 when the filter is off.
fn turnover(p: &Px, on: bool, table_b: bool) -> Vec<bool> {
    (0..p.n)
        .map(|i| {
            if !on {
                true
            } else if table_b {
                p.oi[i] > volsig_b(p.date[i] / 10000)
            } else {
                p.v[i] * p.c[i] > volsig_a(p.date[i] / 10000)
            }
        })
        .collect()
}

/// (deadstockbuysig, deadstocksellsig): dead stock YES = no buy on the last bar and a sell on it; NO = no buy on the first bar.
fn dead_flags(n: usize, dead_yes: bool) -> (Vec<bool>, Vec<bool>) {
    if dead_yes {
        ((0..n).map(|i| i + 1 != n).collect(), (0..n).map(|i| i + 1 == n).collect())
    } else {
        ((0..n).map(|i| i != 0).collect(), vec![false; n])
    }
}

fn clamp_prices(sg: &mut Signals, s: &Series) {
    for i in 0..s.bars.len() {
        let b = &s.bars[i];
        sg.buy_price[i] = sg.buy_price[i].min(b.h).max(b.l);
        sg.sell_price[i] = sg.sell_price[i].min(b.h).max(b.l);
        if let Some(sl) = sg.short_leg.as_mut() {
            sl.short_price[i] = sl.short_price[i].min(b.h).max(b.l);
            sl.cover_price[i] = sl.cover_price[i].min(b.h).max(b.l);
        }
    }
}

// ---------------------------------------------------------------- indicators (float32, Null = NaN)

fn tr1(h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    (0..h.len()).map(|i| if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect()
}

/// `ATR(n)`: Wilder smoothing of the true range, seed = simple average of the first n true ranges at bar n-1, Null before (as the verified ATR / TurnOver AFL).
fn atr_n(tr: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![f32::NAN; tr.len()];
    if n <= 1 {
        return tr.to_vec();
    }
    if tr.len() < n {
        return out;
    }
    let mut s = 0f32;
    for i in 0..n {
        s += tr[i];
    }
    out[n - 1] = s / n as f32;
    for i in n..tr.len() {
        out[i] = (out[i - 1] * (n as f32 - 1.0) + tr[i]) / n as f32;
    }
    out
}

/// The AFLs' `satr`: (ATR(1) + Ref(ATR(1),-1) + ... ) / period, accumulated in that order; Null while the history is short.
fn satr_n(tr: &[f32], period: usize) -> Vec<f32> {
    (0..tr.len())
        .map(|i| {
            if period == 0 || i + 1 < period {
                return f32::NAN;
            }
            let mut s = tr[i];
            for k in 1..period {
                s += tr[i - k];
            }
            s / period as f32
        })
        .collect()
}

fn ma_n(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![f32::NAN; a.len()];
    let mut sum = 0f64;
    for i in 0..a.len() {
        sum += a[i] as f64;
        if i >= n {
            sum -= a[i - n] as f64;
        }
        if n >= 1 && i + 1 >= n {
            out[i] = (sum / n as f64) as f32;
        }
    }
    out
}

/// `EMA(a, n)`: alpha = 2 / (n + 1), seeded with the first non-Null value.
fn ema_n(a: &[f32], n: usize) -> Vec<f32> {
    let al = 2.0f32 / (n as f32 + 1.0);
    let mut out = vec![f32::NAN; a.len()];
    for i in 0..a.len() {
        out[i] = if a[i].is_nan() {
            f32::NAN
        } else if i == 0 || out[i - 1].is_nan() {
            a[i]
        } else {
            al * a[i] + (1.0 - al) * out[i - 1]
        };
    }
    out
}

/// `StDev(a, n)` (population), Null while the window is short.
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

/// `RSI(n)`, Wilder form (as the other ATH engines; AmiBroker's own seed not verified).
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

/// `ADX(n)`: Wilder's ADX (+DM / -DM / TR smoothed with the simple-average seed, DX smoothed again); seeds not checked against AmiBroker's own.
fn adx_w(h: &[f32], l: &[f32], c: &[f32], n: usize) -> Vec<f32> {
    let len = h.len();
    let mut out = vec![f32::NAN; len];
    if n == 0 || len < 2 * n {
        return out;
    }
    let mut pdm = vec![0f32; len];
    let mut mdm = vec![0f32; len];
    let mut tr = vec![0f32; len];
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
    let mut dx = vec![0f32; len];
    for i in n - 1..len {
        if i >= n {
            sp = (sp * (nf - 1.0) + pdm[i]) / nf;
            sm = (sm * (nf - 1.0) + mdm[i]) / nf;
            st = (st * (nf - 1.0) + tr[i]) / nf;
        }
        if st > 0.0 {
            let (pdi, mdi) = (100.0 * sp / st, 100.0 * sm / st);
            dx[i] = if pdi + mdi > 0.0 { 100.0 * (pdi - mdi).abs() / (pdi + mdi) } else { 0.0 };
        }
    }
    let first = 2 * n - 2;
    out[first] = dx[n - 1..=first].iter().sum::<f32>() / nf;
    for i in first + 1..len {
        out[i] = (out[i - 1] * (nf - 1.0) + dx[i]) / nf;
    }
    out
}

/// `HHV(a, n)` with the window shortened at the start.
fn hhv_n(a: &[f32], n: usize) -> Vec<f32> {
    (0..a.len()).map(|i| a[i.saturating_sub(n.saturating_sub(1))..=i].iter().copied().fold(f32::MIN, f32::max)).collect()
}

/// `ROC(c, n)` in percent, Null for the first n bars.
fn roc_n(c: &[f32], n: usize) -> Vec<f32> {
    (0..c.len()).map(|i| if i >= n && c[i - n] != 0.0 { (c[i] - c[i - n]) / c[i - n] * 100.0 } else { f32::NAN }).collect()
}

/// The AFLs' Supertrend loop (`ST()` function and the in-line copies): Up = stcal + fup x satr, Dn = stcal - fdn x satr, with the AFL's
/// `changeOfTrend` back-fill of the previous bar's TrendUp / TrendDown. Returns (trend +1 / -1, TrendUp, TrendDown); Null = NaN.
fn st_lines(stcal: &[f32], satr: &[f32], c: &[f32], fup: f32, fdn: f32) -> (Vec<i8>, Vec<f32>, Vec<f32>) {
    let n = c.len();
    let mut up: Vec<f32> = (0..n).map(|i| stcal[i] + fup * satr[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| stcal[i] - fdn * satr[i]).collect();
    let mut trend = vec![1i8; n];
    let (mut tup, mut tdn) = (vec![f32::NAN; n], vec![f32::NAN; n]);
    let mut chg = false;
    for i in 1..n {
        let mut t = 1i8;
        if c[i] > up[i - 1] {
            t = 1;
            if trend[i - 1] == -1 {
                chg = true;
            }
        } else if c[i] < dn[i - 1] {
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
        let flag = t < 0 && trend[i - 1] > 0;
        let flagh = t > 0 && trend[i - 1] < 0;
        if t > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if t < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if flag {
            up[i] = stcal[i] + fup * satr[i];
        }
        if flagh {
            dn[i] = stcal[i] - fdn * satr[i];
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
    (trend, tup, tdn)
}

/// `STCAL` of the "CLOSE|MID POINT|PIVOT" lists: item 1 close, 2 (H+L)/2, 3 (H+L+C)/3 (the AFLs' `IIf` chain; anything else = close).
fn stcal3(p: &Px, item: i32) -> Vec<f32> {
    (0..p.n)
        .map(|i| match item {
            2 => (p.h[i] + p.l[i]) / 2.0,
            3 => (p.h[i] + p.l[i] + p.c[i]) / 3.0,
            _ => p.c[i],
        })
        .collect()
}

/// The 7-item list of the live-market Supertrend AFL: 1 CLOSE, 2 MID POINT, 3 HIGH, 4 PIVOT, 5 MP CL PIVOT AVG, 6 3DAYS MA, 7 5DAYS MA.
fn stcal7(p: &Px, item: i32) -> Vec<f32> {
    let mp: Vec<f32> = (0..p.n).map(|i| (p.h[i] + p.l[i]) / 2.0).collect();
    let pv: Vec<f32> = (0..p.n).map(|i| (p.h[i] + p.l[i] + p.c[i]) / 3.0).collect();
    match item {
        1 => p.c.clone(),
        3 => p.h.clone(),
        4 => pv,
        5 => (0..p.n).map(|i| (mp[i] + pv[i] + p.c[i]) / 3.0).collect(),
        6 => ma_n(&p.c, 3),
        7 => ma_n(&p.c, 5),
        _ => mp,
    }
}

fn blank(n: usize) -> Signals {
    Signals::new(vec![false; n], vec![false; n], vec![0.0; n], vec![0.0; n])
}

fn toggle_p(key: &'static str, label: &'static str, def: f64) -> P {
    p(key, label, def, 1.0, 2.0, 1.0)
}
fn dead_p(def: f64) -> P {
    toggle_p("deadstock", "DEAD STOCK BACKTEST (1 YES, 2 NO)", def)
}
fn turn_p() -> P {
    toggle_p("turnflt", "TURNOVER Filter (1 NO, 2 YES)", 1.0)
}
fn penny_p() -> P {
    p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0)
}

// ======================= copied from ath_v2_a1.rs / ath_plus.rs / ath_loss2.rs (verified there): Null helpers, swing loop, Foreign alignment, calendar compression, index filter =======================
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
// ================================================================ 1. ATH close buy, Supertrend 10/4 exit with live-market sell and whipsaw re-entry

/// `buytrend` of the AFL's `exit` loop (i from 2): `buytrigger[i]` = Hcl[i-1] is only written while `exit == 0`, so `Close > buytrigger[i-1]` needs `exit` to
/// have been 0 at the start of the previous bar too (the AFL's one-bar lag, copied); `buytrend` is 1 from the trigger until the Supertrend turns -1.
fn buytrend_loop(c: &[f32], hcl: &[f32], trend: &[i8]) -> Vec<f32> {
    let n = c.len();
    let mut buytrigger = vec![f32::NAN; n];
    let mut buytrend = vec![f32::NAN; n];
    let mut exit = false;
    for i in 2..n {
        if !exit {
            buytrigger[i] = hcl[i - 1];
            buytrend[i] = 0.0;
        }
        if c[i] > buytrigger[i - 1] && !exit {
            exit = true;
            buytrend[i] = 1.0;
        } else if exit {
            buytrend[i] = 1.0;
        }
        if trend[i] == -1 {
            exit = false;
        }
    }
    buytrend
}

pub fn close_buy_signals(s: &Series, dead_yes: bool, buff: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let tr = tr1(&p.h, &p.l, &p.c);
    let natr = satr_n(&tr, 10);
    let (trend, tup, tdn) = st_lines(&p.c, &natr, &p.c, 1.0, 4.0);
    let buytrend = buytrend_loop(&p.c, &p.hcl, &trend);
    // tradelvl = IIf(trend==1, trendup - trendup*BUFF/100, Null), tradedummy = trendup + trendup*BUFF/100
    let tradelvl: Vec<f32> = (0..n).map(|i| if trend[i] == 1 { tup[i] - (tup[i] * buff) / 100.0 } else { f32::NAN }).collect();
    let tradedummy: Vec<f32> = (0..n).map(|i| if trend[i] == 1 { tup[i] + (tup[i] * buff) / 100.0 } else { f32::NAN }).collect();
    let (dbuy, dsell) = dead_flags(n, dead_yes);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut bp, mut sp) = (p.c.clone(), p.c.clone());
    for i in 1..n {
        let ref_h = |k: usize| if i >= k + 1 { p.hcl[i - 1 - k] } else { f32::NAN }; // Ref(Hcl, -(k+1))
        let cross = p.c[i] > ref_h(0) && p.c[i - 1] <= ref_h(1);
        let whip = buytrend[i] == 1.0 && p.l[i] < tradelvl[i - 1];
        buy[i] = (cross || (whip && p.c[i] > tdn[i - 1])) && dbuy[i] && !p.upc[i];
        sell[i] = (trend[i] == -1 && !p.loc[i]) || whip || dsell[i];
        // BuyPrice = ValueWhen(Buy, IIf(buytrend==1 AND L<Ref(TRADELVL,-1) AND C>Ref(tradedummy,-1), Ref(tradedummy,-1), C))
        bp[i] = if whip && p.c[i] > tradedummy[i - 1] { tradedummy[i - 1] } else { p.c[i] };
        // SellPrice = ValueWhen(Sell, IIf(buytrend==1 AND L<Ref(tradelvl,-1), IIf(O<Ref(tradelvl,-1), O-(O*.25/100), Ref(tradelvl,-1)), C))
        sp[i] = if whip { if p.o[i] < tradelvl[i - 1] { p.o[i] - (p.o[i] * 0.25) / 100.0 } else { tradelvl[i - 1] } } else { p.c[i] };
    }
    let mut sg = Signals::new(buy, sell, bp, sp);
    sg.same_bar_exit = true; // SetOption("AllowSameBarExit", True)
    clamp_prices(&mut sg, s);
    sg
}

fn schema_close_buy() -> Vec<P> {
    vec![
        dead_p(2.0),
        p("stflt", "ST CALCULATION POINT (1 CLOSE; 2 MID POINT; 3 PIVOT) - read by the AFL but not used: ST(10,4) is built on Close", 1.0, 1.0, 3.0, 1.0),
        p("buff", "BUY BUFFER %", 1.5, 0.0, 10.0, 0.5),
    ]
}

fn sig_close_buy(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let (dead, buff) = (g("deadstock").round() == 1.0, g("buff") as f32);
    series.iter().map(|s| close_buy_signals(s, dead, buff)).collect()
}

// ================================================================ 2-4. BTST on ATHC (index filter / High-Close ranking / ROC ranking)

#[derive(Clone, Copy, PartialEq)]
pub enum BtstScore {
    None,
    /// PositionScore = 100 - (High - Close) / High x 100
    HighClose,
    /// PositionScore = -ROC(C, 1)
    NegRoc,
}

/// Buy = Close > previous ATHC AND fltbuy AND NOT UPCIRCUIT AND C > penny AND turnover AND deadstockbuysig; Sell = O AND NOT locircuit;
/// Buy = ExRem(Buy, Sell); Sell = ExRem(Sell, Buy); BuyPrice = C; SellPrice = O - O x BUFF / 100. (Turnover table A, Volume x Close.)
pub fn btst_signals(s: &Series, dead_yes: bool, penny: f32, turn_on: bool, buff: f32, fltbuy: Option<&[bool]>, score: BtstScore, circuits: bool, no_rebuy: bool) -> Signals {
    let p = px(s);
    let n = p.n;
    let turn = turnover(&p, turn_on, false);
    let (dbuy, _) = dead_flags(n, dead_yes);
    let mut buyraw = vec![false; n];
    let sellraw: Vec<bool> = (0..n).map(|i| !(circuits && p.loc[i])).collect();
    for i in 1..n {
        buyraw[i] = p.c[i] > p.hcl[i - 1] && fltbuy.map_or(true, |f| f[i]) && !(circuits && p.upc[i]) && p.c[i] > penny && turn[i] && dbuy[i];
    }
    // (verification switch, 0 in production: no new buy on the bar that sells yesterday's position, as the position-state BTST AFL of the nearest sheet)
    if no_rebuy {
        let mut held = false;
        for i in 0..n {
            if held {
                buyraw[i] = false;
                held = false;
            } else if buyraw[i] {
                held = true;
            }
        }
    }
    let buy = exrem(&buyraw, &sellraw);
    let sell = exrem(&sellraw, &buy);
    let sp: Vec<f32> = (0..n).map(|i| if buff == 0.0 { p.o[i] } else { p.o[i] - (p.o[i] * buff) / 100.0 }).collect();
    let mut sg = Signals::new(buy, sell, p.c.clone(), sp);
    clamp_prices(&mut sg, s);
    sg.score = match score {
        BtstScore::None => None,
        BtstScore::HighClose => Some((0..n).map(|i| 100.0 - (p.h[i] - p.c[i]) / p.h[i] * 100.0).collect()),
        BtstScore::NegRoc => Some(roc_n(&p.c, 1).iter().map(|&r| -r).collect()),
    };
    sg
}

fn schema_btst_plain() -> Vec<P> {
    vec![dead_p(1.0), penny_p(), turn_p(), p("buff", "OPEN SELL BUFFER", 0.25, 0.0, 5.0, 0.01)]
}

fn schema_btst_index() -> Vec<P> {
    let mut v = schema_btst_plain();
    v.extend([
        toggle_p("mktflt", "FILTER TYPE (1 NONE, 2 INDEX)", 1.0),
        p("flttf", "FILTER TIMEFRAME (1 MONTHLY, 2 WEEKLY, 3 DAILY)", 1.0, 1.0, 3.0, 1.0),
        p("prflt", "INDEX (1 NSENIFTY, 2 NSENIFTY1, 3 NIFTY, 4 NSE500)", 1.0, 1.0, 4.0, 1.0),
        toggle_p("iflt", "INDEX Filter (1 ST, 2 SMA)", 1.0),
        p("istflt", "ST CALCULATION POINT (1 MID POINT H+L/2, 2 CLOSE, 3 PIVOT H+L+C/3)", 1.0, 1.0, 3.0, 1.0),
        p("ifltpd", "INDEX ST PERIOD", 20.0, 0.0, 20.0, 1.0),
        p("ifltfact", "INDEX ST FACTOR", 1.0, 0.0, 20.0, 1.0),
        p("ifltmapd", "INDEX SMA PERIOD", 200.0, 1.0, 300.0, 1.0),
    ]);
    v
}

/// The ROC-ranking AFL codes the whole index filter block but never puts `FLTBuy` into Buy: the same parameters, all without effect.
fn schema_btst_roc() -> Vec<P> {
    schema_btst_index()
        .into_iter()
        .map(|q| match q.key {
            "mktflt" => P { label: "FILTER TYPE (1 NONE, 2 INDEX): coded in the AFL but FLTBuy is not used in Buy, no effect", ..q },
            "flttf" | "prflt" | "iflt" | "istflt" | "ifltpd" | "ifltfact" | "ifltmapd" => P { label: "index filter parameter: no effect (FLTBuy is not used in Buy)", ..q },
            _ => q,
        })
        .collect()
}

fn btst_common(g: G) -> (bool, f32, bool, f32) {
    (g("deadstock").round() == 1.0, g("pennyflt") as f32, g("turnflt").round() == 2.0, g("buff") as f32)
}

/// FLTBuy of the AFL's market filter block (MKTFLT NONE = TRUE; INDEX = the chosen index's Supertrend / SMA on the chosen time frame).
/// An index that is not in the database (NIFTY) reads as Null: FLTBuy is false.
fn index_fltbuy(s: &Series, env: &Env, g: G) -> Vec<bool> {
    let n = s.bars.len();
    if g("mktflt").round() != 2.0 {
        return vec![true; n];
    }
    let name = INDEX_NAMES[((g("prflt").round() as usize).max(1) - 1).min(3)];
    let f = Flt {
        tf: (g("flttf").round() as u8).max(1) - 1,
        sma: g("iflt").round() == 2.0,
        point: (g("istflt").round() as u8).max(1) - 1,
        pd: g("ifltpd").round().max(0.0) as usize,
        fact: g("ifltfact") as f32,
        mapd: g("ifltmapd").round().max(1.0) as usize,
    };
    match env.get(name) {
        Some(ix) => {
            let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
            st_filter(&align(ix, &dates), &dates, &f).0
        }
        None => vec![false; n],
    }
}

fn sig_btst_index(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    let (dead, penny, turn, buff) = btst_common(g);
    series.iter().map(|s| btst_signals(s, dead, penny, turn, buff, Some(&index_fltbuy(s, env, g)), BtstScore::None, g("dbg_nocirc") == 0.0, g("dbg_norebuy") != 0.0)).collect()
}

fn sig_btst_high(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let (dead, penny, turn, buff) = btst_common(g);
    series.iter().map(|s| btst_signals(s, dead, penny, turn, buff, None, BtstScore::HighClose, g("dbg_nocirc") == 0.0, g("dbg_norebuy") != 0.0)).collect()
}

/// The index filter block of this AFL is coded but `FLTBuy` is not in Buy: no filter, PositionScore = -ROC(C,1).
fn sig_btst_roc(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let (dead, penny, turn, buff) = btst_common(g);
    series.iter().map(|s| btst_signals(s, dead, penny, turn, buff, None, BtstScore::NegRoc, g("dbg_nocirc") == 0.0, g("dbg_norebuy") != 0.0)).collect()
}

// ================================================================ 5. BTST of ATHC with the upper Bollinger band

/// Buy = flat AND Close > previous ATHC AND Close >= (version 1) / > (version 2) the upper band AND UC == 0 AND turnover AND C > penny AND deadstockbuysig, at
/// the close; sell the bar after (not on a lower-circuit bar, here the 4-price version O = C = L = H) at Open - Buffer (version 1: Open if Open == Low, else the
/// level when the Low went below it, else the Low) or at the Close (version 2). Turnover table B (OpenInt).
pub fn bollinger_signals(s: &Series, dead_yes: bool, penny: f32, turn_on: bool, ver2: bool, buffer: f32, bb_prd: usize, mul: f32, no_band: bool, circuits: bool) -> Signals {
    let p = px(s);
    let n = p.n;
    let turn = turnover(&p, turn_on, true);
    let (dbuy, dsell) = dead_flags(n, dead_yes);
    let myprice: Vec<f32> = (0..n).map(|i| (p.h[i] + p.l[i] + p.c[i]) / 3.0).collect();
    let em = ema_n(&myprice, bb_prd);
    let sd = stdev_n(&p.c, bb_prd);
    let upper: Vec<f32> = (0..n).map(|i| em[i] + mul * sd[i]).collect();
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut sp = p.c.clone();
    let mut pos = false;
    for i in 1..n {
        let lo4 = prec2(p.o[i]) == prec2(p.c[i]) && prec2(p.o[i]) == prec2(p.l[i]) && prec2(p.o[i]) == prec2(p.h[i]);
        let band_ok = no_band || if ver2 { p.c[i] > upper[i] } else { p.c[i] >= upper[i] };
        if !pos && p.c[i] > p.hcl[i - 1] && band_ok && !(circuits && p.uc[i]) && turn[i] && p.c[i] > penny && dbuy[i] {
            buy[i] = true;
            pos = true;
        } else if pos && (!(circuits && lo4) || dsell[i]) {
            sell[i] = true;
            sp[i] = if ver2 {
                p.c[i]
            } else {
                let lvl = p.o[i] - (p.o[i] * buffer) / 100.0;
                if p.o[i] == p.l[i] {
                    p.o[i]
                } else if p.l[i] < lvl {
                    lvl
                } else {
                    p.l[i] // Low above the level (an exact tie is read the same way)
                }
            };
            pos = false;
        }
    }
    let mut sg = Signals::new(buy, sell, p.c.clone(), sp);
    clamp_prices(&mut sg, s);
    sg
}

fn schema_bollinger() -> Vec<P> {
    vec![
        dead_p(2.0),
        penny_p(),
        turn_p(),
        toggle_p("ver", "Exit Version (1 Exit on Open, 2 Exit on Close)", 1.0),
        p("buffer", "Selling Buffer %", 0.25, 0.0, 5.0, 0.001),
        p("bb_emaprd", "BB_EMAPrd", 15.0, 2.0, 200.0, 1.0),
        p("stdevmul", "StDevMul", 2.0, 0.0, 10.0, 0.001),
    ]
}

fn sig_bollinger(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    series
        .iter()
        .map(|s| bollinger_signals(s, g("deadstock").round() == 1.0, g("pennyflt") as f32, g("turnflt").round() == 2.0, g("ver").round() == 2.0, g("buffer") as f32, g("bb_emaprd").round() as usize, g("stdevmul") as f32, g("dbg_nobb") != 0.0, g("dbg_nocirc") == 0.0))
        .collect()
}

// ================================================================ 6. ATHC with live market entry, Supertrend exit

#[derive(Clone, Copy)]
pub struct LiveSt {
    pub dead_yes: bool,
    pub penny: f32,
    pub turn_on: bool,
    pub buff: f32,
    pub stflt: i32,
    pub period: usize,
    pub factor: f32,
    pub hardstop: f32,
}

/// `Buy = (Cross(C, Ref(Hcl,-1)) OR (Ref(buytrend,-1)==0 AND H > Ref(buylvl,-1))) AND NOT UPCIRCUIT AND deadstockbuysig AND C > penny AND turnover`, `ExRem(Buy, Sell)`,
/// `Sell = (btrend1==-1 AND NOT LOCIRCUIT) OR (buytrend==0 AND H > Ref(buylvl,-1) AND C < Ref(hcl,-1)) OR deadstocksellsig`. `buylvl` / `selllvl` start as
/// Hcl +- 3 % and are overwritten with +- BUFF % on the bars where `exit == 0` (the AFL's loop); `buytrend` as in the other loop.
pub fn live_st_signals(s: &Series, q: &LiveSt) -> Signals {
    let p = px(s);
    let n = p.n;
    let tr = tr1(&p.h, &p.l, &p.c);
    let satr = satr_n(&tr, q.period);
    let stcal = stcal7(&p, q.stflt);
    let (bt, _, _) = st_lines(&stcal, &satr, &p.c, 1.0, q.factor);
    let turn = turnover(&p, q.turn_on, false);
    let (dbuy, dsell) = dead_flags(n, q.dead_yes);
    let mut buylvl: Vec<f32> = (0..n).map(|i| p.hcl[i] + (p.hcl[i] * 3.0) / 100.0).collect();
    let mut selllvl: Vec<f32> = (0..n).map(|i| p.hcl[i] - (p.hcl[i] * 3.0) / 100.0).collect();
    let mut buytrigger = vec![f32::NAN; n];
    let mut buytrend = vec![f32::NAN; n];
    let mut exit = false;
    for i in 2..n {
        if !exit {
            buytrigger[i] = p.hcl[i - 1];
            buylvl[i] = p.hcl[i] + (p.hcl[i] * q.buff) / 100.0;
            selllvl[i] = p.hcl[i] - (p.hcl[i] * q.buff) / 100.0;
            buytrend[i] = 0.0;
        }
        if p.c[i] > buytrigger[i - 1] && !exit {
            exit = true;
            buytrend[i] = 1.0;
        } else if exit {
            buytrend[i] = 1.0;
        }
        if bt[i] == -1 {
            exit = false;
        }
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut bp, mut sp) = (p.c.clone(), p.c.clone());
    for i in 1..n {
        let cross = i >= 2 && p.c[i] > p.hcl[i - 1] && p.c[i - 1] <= p.hcl[i - 2];
        let live = p.h[i] > buylvl[i - 1]; // H > Ref(buylvl,-1)
        buy[i] = (cross || (buytrend[i - 1] == 0.0 && live)) && !p.upc[i] && dbuy[i] && p.c[i] > q.penny && turn[i];
        sell[i] = (bt[i] == -1 && !p.loc[i]) || (buytrend[i] == 0.0 && live && p.c[i] < p.hcl[i - 1]) || dsell[i];
        // BuyPrice = ValueWhen(Buy, IIf(H>Ref(buylvl,-1), IIf(O>Ref(buylvl,-1), O+(O*.25/100), Ref(buylvl,-1)), C))
        bp[i] = if live { if p.o[i] > buylvl[i - 1] { p.o[i] + (p.o[i] * 0.25) / 100.0 } else { buylvl[i - 1] } } else { p.c[i] };
        // SellPrice = ValueWhen(Sell, IIf(buytrend==0 AND H>Ref(buylvl,-1) AND C<Ref(selllvl,-1), Ref(selllvl,-1), C))
        sp[i] = if buytrend[i] == 0.0 && live && p.c[i] < selllvl[i - 1] { selllvl[i - 1] } else { p.c[i] };
    }
    let buy = exrem(&buy, &sell);
    let mut sg = Signals::new(buy, sell, bp, sp);
    sg.same_bar_exit = true; // AmiBroker default AllowSameBarExit = True: a failed live breakout is bought and sold on its own bar
    clamp_prices(&mut sg, s);
    if q.hardstop > 0.0 {
        sg.stop = Some(StopSpec { loss_pct: q.hardstop, ..StopSpec::default() });
    }
    sg
}

fn schema_live_st() -> Vec<P> {
    vec![
        dead_p(2.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop percent loss, stops start the bar after the entry)", 0.0, 0.0, 50.0, 1.0),
        penny_p(),
        turn_p(),
        p("buff", "BUY BUFFER %", 1.5, 0.0, 10.0, 0.5),
        p("stflt", "DAILY ST CALCULATION POINT (1 CLOSE; 2 MID POINT; 3 HIGH; 4 PIVOT; 5 MP CL PIVOT AVG; 6 3DAYS MA; 7 5DAYS MA)", 1.0, 1.0, 7.0, 1.0),
        p("bpd1", "ST Period", 10.0, 1.0, 20.0, 1.0),
        p("bfactor1", "ST Factor", 4.0, 1.0, 20.0, 1.0),
        p("tstart", "TRADING START BAR (read by the AFL but not used in Buy / Sell)", 1.0, 0.0, 500.0, 1.0),
    ]
}

fn sig_live_st(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let q = LiveSt {
        dead_yes: g("deadstock").round() == 1.0,
        penny: g("pennyflt") as f32,
        turn_on: g("turnflt").round() == 2.0,
        buff: g("buff") as f32,
        stflt: g("stflt").round() as i32,
        period: g("bpd1").round() as usize,
        factor: g("bfactor1") as f32,
        hardstop: g("hardstop") as f32,
    };
    series.iter().map(|s| live_st_signals(s, &q)).collect()
}

// ================================================================ 7. ATHC live market entry and exit on the intraday close (F&O)

/// The AFL's state machine (`abc`, `Count`) around a same-day round trip: buy at the previous ATHC + Buying Buffer when the %swing is up, High > ATHC and Open < ATHC,
/// sell the same bar at Close - Selling Buffer. After a profitable breakout day (virtual Profit > 0) wait for the %swing to turn down; after a loss wait seven up-trend bars.
/// `comm` is the AFL's BROKERAGE (in the virtual Profit). Buy_Cash_Filter / Sell_Cash_Filter are computed but not used by the AFL.
pub fn live_exit_signals(s: &Series, dead_yes: bool, penny: f32, exit_pct: f32, buy_buf: f32, sell_buf: f32, comm: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let trend1 = scan::swing_trend(&p.c, &p.c, exit_pct);
    let (dbuy, _) = dead_flags(n, dead_yes);
    // Buy_FO_Filter = Ref(Close,-1) > PennyFlt AND DEADSTOCKBUYSIG
    let fo: Vec<bool> = (0..n).map(|i| i >= 1 && p.c[i - 1] > penny && dbuy[i]).collect();
    let brk = |i: usize| i >= 1 && p.h[i] > p.hcl[i - 1] && p.o[i] < p.hcl[i - 1];
    // first loop: virtual breakout fills (VB is reset on every bar)
    let (mut my_bp, mut my_sp) = (0f32, 0f32);
    let (mut mybuy, mut mysell) = (vec![0f32; n], vec![0f32; n]);
    for i in 1..n {
        let mut vb = false;
        if trend1[i] == 1 && brk(i) && fo[i] {
            my_bp = p.hcl[i - 1];
            vb = true;
        }
        if vb {
            my_sp = p.c[i] - (p.c[i] * sell_buf) / 100.0;
        }
        mybuy[i] = my_bp;
        mysell[i] = my_sp;
    }
    let profit: Vec<f32> = (0..n)
        .map(|i| if trend1[i] == 1 && brk(i) { (mysell[i] - (mysell[i] * comm) / 100.0) - (mybuy[i] + (mybuy[i] * comm) / 100.0) } else { 0.0 })
        .collect();
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut bp, mut sp) = (p.c.clone(), p.c.clone());
    let (mut position, mut abc, mut count) = (false, 0i32, 0i32);
    for i in 1..n {
        if !position && trend1[i] == 1 && (abc == 0 || abc == 2) && brk(i) && fo[i] {
            buy[i] = true;
            bp[i] = p.hcl[i - 1] + (p.hcl[i - 1] * buy_buf) / 100.0;
            position = true;
            abc = 0;
            count = 0;
        }
        if position {
            sell[i] = true;
            sp[i] = p.c[i] - (p.c[i] * sell_buf) / 100.0;
            position = false;
            abc = 0;
            count = 0;
        }
        if profit[i] > 0.0 {
            if trend1[i] == 1 && brk(i) && (abc == 0 || abc == 2) {
                abc = 1;
            } else if trend1[i] == -1 && abc == 1 {
                abc = 0;
                count = 0;
            }
        }
        // `Profit <= 0 OR Count >= 0` is always true
        if profit[i] <= 0.0 && brk(i) && trend1[i] == 1 && count == 0 && abc == 0 {
            abc = 2;
        }
        if abc >= 2 && trend1[i] == 1 && count <= 6 {
            count += 1;
            abc = 3;
        } else if count == 7 || trend1[i] == -1 {
            abc = 0;
            count = 0;
        }
    }
    let mut sg = Signals::new(buy, sell, bp, sp);
    sg.same_bar_exit = true;
    clamp_prices(&mut sg, s);
    sg
}

fn schema_live_exit() -> Vec<P> {
    vec![
        dead_p(2.0),
        penny_p(),
        turn_p(),
        p("exitpct", "EXIT % (%swing)", 10.0, 1.0, 20.0, 1.0),
        p("buffer_pct", "Buying Buffer Pct", 0.25, 0.0, 10.0, 0.05),
        p("bufferpct", "Selling Buffer Pct", 0.25, 0.0, 10.0, 0.05),
        p("comm", "BROKERAGE (only enters the AFL's virtual Profit)", 0.65, 0.01, 1.0, 0.01),
    ]
}

fn sig_live_exit(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    series
        .iter()
        .map(|s| live_exit_signals(s, g("deadstock").round() == 1.0, g("pennyflt") as f32, g("exitpct") as f32, g("buffer_pct") as f32, g("bufferpct") as f32, g("comm") as f32))
        .collect()
}

// ================================================================ 8. Intraday on ATHC with ranking: buy at Open + buffer, sell the same day at the Close

#[derive(Clone, Copy)]
pub struct Intraday {
    pub rankmode: i32,
    pub rocpd: usize,
    pub rsipd: usize,
    pub swingpd: usize,
    pub prfltpd: usize,
    pub stflt: i32,
    pub tstart: usize,
    pub period: usize,
    pub factor: f32,
    pub buff: f32,
}

/// `buysig` = 1 on a close above the previous highest close; the intended reset on a Supertrend down bar is the comparison `buysig[i]==0` (no assignment), so the element
/// stays Null (state off) until the next new high close. `Buy = Ref(buysig,-1)==1 AND BarIndex >= TSTART (>= PR EMA period in PR mode)`, `Sell = BUY AND C`, `BuyPrice = O + O x BUFF / 100`,
/// `SellPrice = C`; turnover / penny / dead stock / circuit filters are not used by the AFL. `pr` = the index close aligned to the stock's dates (PR ranking only).
pub fn intraday_signals(s: &Series, q: &Intraday, pr: Option<&[f32]>) -> Signals {
    let p = px(s);
    let n = p.n;
    let tr = tr1(&p.h, &p.l, &p.c);
    let satr = satr_n(&tr, q.period);
    let (bt, _, _) = st_lines(&stcal3(&p, q.stflt), &satr, &p.c, 1.0, q.factor);
    let mut state = vec![false; n]; // buysig == 1 (Null = false)
    for i in 1..n {
        state[i] = if p.c[i] > p.hcl[i - 1] {
            true
        } else if bt[i] == -1 {
            false
        } else {
            state[i - 1]
        };
    }
    let start = if q.rankmode == 6 { q.prfltpd } else { q.tstart };
    let buy: Vec<bool> = (0..n).map(|i| i >= 1 && state[i - 1] && i >= start).collect();
    let bp: Vec<f32> = (0..n).map(|i| if q.buff == 0.0 { p.o[i] } else { p.o[i] + (p.o[i] * q.buff) / 100.0 }).collect();
    let mut sg = Signals::new(buy.clone(), buy, bp, p.c.clone());
    sg.same_bar_exit = true; // SetOption("AllowSameBarExit", true)
    clamp_prices(&mut sg, s);
    sg.score = match q.rankmode {
        2 => Some(roc_n(&p.c, q.rocpd).iter().map(|&r| 100.0 + r).collect()),
        3 => Some(roc_n(&p.c, q.rocpd).iter().map(|&r| 100.0 - r).collect()),
        4 => Some(rsi_w(&p.c, q.rsipd)),
        5 => {
            let hh = hhv_n(&p.c, q.swingpd);
            Some((0..n).map(|i| 100.0 - (hh[i] - p.c[i]) / hh[i] * 100.0).collect())
        }
        6 => {
            let ic: Vec<f32> = match pr {
                Some(x) => x.to_vec(),
                None => vec![f32::NAN; n],
            };
            let (se, ie) = (ema_n(&p.c, q.prfltpd), ema_n(&ic, q.prfltpd));
            Some(
                (0..n)
                    .map(|i| {
                        let closepr = p.c[i] / ic[i];
                        let emapr = se[i] / ie[i];
                        (closepr - emapr) / closepr * 100.0
                    })
                    .collect(),
            )
        }
        _ => None,
    };
    sg
}

fn schema_intraday() -> Vec<P> {
    vec![
        dead_p(2.0),
        turn_p(),
        p("rankmode", "RANKING MODE (1 NONE, 2 POSITIVE ROC, 3 NEGATIVE ROC, 4 RSI, 5 SWING, 6 PR)", 1.0, 1.0, 6.0, 1.0),
        p("rocpd", "ROC PERIOD for RANKING", 10.0, 0.0, 400.0, 30.0),
        p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("swingpd", "SWING PERIOD", 1000.0, 10.0, 2000.0, 1.0),
        p("prflt", "PRICE RELATIVE INDEX (1 NSENIFTY, 2 NIFTY, 3 NSE500)", 1.0, 1.0, 3.0, 1.0),
        p("prfltpd", "PRICE RELATIVE EMA PERIOD", 200.0, 1.0, 300.0, 1.0),
        p("stflt", "ST CALCULATION POINT (1 CLOSE, 2 MID POINT, 3 PIVOT)", 1.0, 1.0, 3.0, 1.0),
        p("tstart", "TRADING START BAR", 0.0, 1.0, 500.0, 1.0),
        p("bpd1", "ATHC EXIT ST Period", 10.0, 1.0, 20.0, 1.0),
        p("bfactor1", "ATHC EXIT ST Factor", 4.0, 1.0, 20.0, 1.0),
        p("buff", "OPEN SELL BUFFER (the buy-side buffer: Open + x %)", 0.25, 0.0, 5.0, 0.01),
    ]
}

fn sig_intraday(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    let q = Intraday {
        rankmode: g("rankmode").round() as i32,
        rocpd: g("rocpd").round() as usize,
        rsipd: g("rsipd").round() as usize,
        swingpd: g("swingpd").round() as usize,
        prfltpd: g("prfltpd").round() as usize,
        stflt: g("stflt").round() as i32,
        tstart: g("tstart").round() as usize,
        period: g("bpd1").round() as usize,
        factor: g("bfactor1") as f32,
        buff: g("buff") as f32,
    };
    let name = ["NSENIFTY", "NIFTY", "NSE500"][((g("prflt").round() as usize).max(1) - 1).min(2)];
    series
        .iter()
        .map(|s| {
            let pr: Option<Vec<f32>> = if q.rankmode == 6 {
                env.get(name).map(|ix| {
                    let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
                    align(ix, &dates).c.iter().map(|&x| if is_null(x) { f32::NAN } else { x }).collect()
                })
            } else {
                None
            };
            intraday_signals(s, &q, pr.as_deref())
        })
        .collect()
}

// ================================================================ 9-10. ATHC 5 % to 30 % scanners (the Buy / Sell block of the AFL, as `ath_new_scan.rs`)

/// The scanner AFL's backtest block: `Buy = (athcbuysig OR C > Ref(hcl,-1)) AND NOT UPCIRCUIT AND DEADSTOCKBUYSIG AND C > pennyflt AND turnover`,
/// `Sell = ((trend1==-1 OR athcbuysig==0) AND NOT LOCIRCUIT) OR deadstocksellsig`, both `ExRem`ed, filled at the close. `table_b`: the 1-consecutive-loss file uses
/// `OpenInt > Volsig` (table B) instead of `V*C > volsig` (table A); everything else of its loss counter only feeds the Explore filter.
pub fn scanner_signals(s: &Series, dead_yes: bool, penny: f32, turn_on: bool, table_b: bool, exit_pct: f32, swingcal: u8, hard: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let st = scan::stcal(s, swingcal);
    let trend = scan::swing_trend(&p.c, &st, exit_pct);
    let (athc, _, _) = scan::athc_latch(&p.c, &p.hcl, &trend, 40.0);
    let turn = turnover(&p, turn_on, table_b);
    let (dbuy, dsell) = dead_flags(n, dead_yes);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        // C > Ref(Hcl,-1) is false on bar 0 (Null)
        let fresh = i > 0 && p.c[i] > p.hcl[i - 1];
        buy[i] = (athc[i] || fresh) && !p.upc[i] && dbuy[i] && p.c[i] > penny && turn[i];
        sell[i] = ((trend[i] == -1 || !athc[i]) && !p.loc[i]) || dsell[i];
    }
    let buy2 = exrem(&buy, &sell);
    let sell2 = exrem(&sell, &buy2);
    let mut sg = Signals::new(buy2, sell2, p.c.clone(), p.c.clone());
    clamp_prices(&mut sg, s);
    if hard > 0.0 {
        sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
    }
    sg
}

fn schema_scanner() -> Vec<P> {
    vec![
        dead_p(2.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop percent loss)", 0.0, 0.0, 50.0, 1.0),
        penny_p(),
        turn_p(),
        p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0),
        p("swingcal", "SWING CALCULATION POINT (1 CLOSE; 2 MIDD POINT H+L/2; 3 HIGH; 4 PIVOT H+L+C/3; 5 MP CL PIVOT AVG; 6 3DAYS MA; 7 5DAYS MA)", 1.0, 1.0, 7.0, 1.0),
    ]
}

fn schema_scanner_loss() -> Vec<P> {
    let mut v = schema_scanner();
    v.push(p("losspct", "MIN PROFIT % (Explore filter only: no effect on the backtest)", 3.0, 1.0, 5.0, 1.0));
    v.push(p("noofloss", "CONSICUTIVE LOSS (Explore filter only: no effect on the backtest)", 2.0, 1.0, 2.0, 1.0));
    v
}

fn scanner_all(series: &[Series], g: G, table_b: bool) -> Vec<Signals> {
    let (dead, penny, turn, exit, cal, hard) = (g("deadstock").round() == 1.0, g("pennyflt") as f32, g("turnflt").round() == 2.0, g("exitpct") as f32, (g("swingcal").round() as i64).clamp(1, 7) as u8, g("hardstop") as f32);
    series.iter().map(|s| scanner_signals(s, dead, penny, turn, table_b, exit, cal, hard)).collect()
}

fn sig_scanner(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    scanner_all(series, g, false)
}

fn sig_scanner_loss(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    scanner_all(series, g, true)
}

// ================================================================ 11-14. AFLs without any Buy / Sell: honest "no trades" variants

/// The stock-count scanner and the three PR plots have no `Buy`, `Sell`, `Short` or `Cover` (the scanner only sets `Filter = BuySig` and columns for the
/// Exploration, the PR AFLs only `Plot` the monthly price relative of two indexes). A backtest of them has no signal, hence no trade: the engine runs and returns
/// an empty trade list with a flat equity line; nothing is invented.
fn sig_none(series: &[Series], _env: &Env, _g: G) -> Vec<Signals> {
    series.iter().map(|s| blank(s.bars.len())).collect()
}

fn schema_stock_count() -> Vec<P> {
    vec![dead_p(2.0)]
}

fn schema_pr_plot() -> Vec<P> {
    vec![
        p("prtimeframe", "TIMEFRAME FOR PR CALCULATION (1 MONTHLY, 2 WEEKLY, 3 DAILY): the AFL's TimeFrameSet line is commented out, the plot is always monthly; no backtest signal", 1.0, 1.0, 3.0, 1.0),
        p("period", "EMA PERIOD FOR PR (plot only, no backtest signal)", 10.0, 1.0, 500.0, 1.0),
    ]
}

// ================================================================ 15. Long after losses + MA / RSI / ADX short combo

#[derive(Clone, Copy)]
pub struct Combo {
    pub eqallo: f32,
    pub buysize: f32,
    pub sellsize: f32,
    pub dead_yes: bool,
    pub penny: f32,
    pub turn_on: bool,
    pub exit_pct: f32,
    pub losspct: f32,
    pub noofloss: f32,
    pub comm: f32,
    /// Filter On/Off for the short (false = Off)
    pub filter_on: bool,
    /// Filter ST (false) / %Swing (true)
    pub filter_swing: bool,
    pub pt_pct: f32,
    pub highma: usize,
    pub lowma: usize,
    pub closema: usize,
    pub adx_prd: usize,
    pub adx_value: f32,
    pub rsi_prd: usize,
    pub rsi_val: f32,
    pub flt_tf: u8,
    pub st_point: u8,
    pub flt_pd: usize,
    pub flt_fact: f32,
    pub fact_bear: f32,
    pub fact_bull: f32,
    pub hardsl: f32,
    pub ndays: f32,
    pub slpct: f32,
    pub low_n_pct: f32,
    pub hdays: f32,
}

/// concount of the AFL: the loss streak of the virtual ATHC trades (in on `buysig`, out on trend -1; a `Cross(Ref(TrendUp1,-1), C)` while in books the trade).
fn concount_of(c: &[f32], buysig: &[bool], trend: &[i8], tup: &[f32], losspct: f32, comm: f32) -> Vec<f32> {
    let n = c.len();
    let null = f32::NAN;
    let (mut bcount, mut bprice, mut sprice) = (vec![0i32; n], vec![null; n], vec![null; n]);
    for i in 1..n {
        if buysig[i] && bcount[i - 1] < 1 {
            bprice[i] = c[i];
            sprice[i] = sprice[i - 1];
            bcount[i] = bcount[i - 1] + 1;
        } else if trend[i] == -1 && bcount[i - 1] == 1 {
            sprice[i] = c[i];
            bprice[i] = bprice[i - 1];
            bcount[i] = 0;
        } else {
            bprice[i] = bprice[i - 1];
            sprice[i] = sprice[i - 1];
            bcount[i] = bcount[i - 1];
        }
    }
    let a = |i: usize| -> f32 { if i >= 1 { tup[i - 1] } else { null } };
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

#[allow(unused_assignments)] // LowTrailSL is written on entry / reset as in the AFL but recomputed before each use
pub fn combo_signals(s: &Series, q: &Combo) -> Signals {
    let p = px(s);
    let n = p.n;
    let (dbuy, dsell) = dead_flags(n, q.dead_yes);
    let turn = turnover(&p, q.turn_on, true);
    // ---- long: the %swing here has Up = Hcl (a new high close always lifts the trend) and Dn = C - EXIT
    let exit: Vec<f32> = p.c.iter().map(|&x| x * q.exit_pct / 100.0).collect();
    let dn0: Vec<f32> = (0..n).map(|i| p.c[i] - exit[i]).collect();
    let sw = swing_loop(&p.c, p.hcl.clone(), dn0.clone(), &p.hcl, &dn0, None);
    let mut athc = vec![false; n];
    for i in 1..n {
        athc[i] = if p.c[i] > p.hcl[i - 1] {
            true
        } else if sw.trend[i] == -1 && athc[i - 1] {
            false
        } else {
            athc[i - 1]
        };
    }
    let vbuy: Vec<bool> = (0..n).map(|i| athc[i] && !p.upc[i] && dbuy[i] && p.c[i] > q.penny && turn[i]).collect();
    let conc = concount_of(&p.c, &vbuy, &sw.trend, &sw.tup, q.losspct, q.comm);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let fresh = i > 0 && p.c[i] > p.hcl[i - 1];
        buy[i] = (athc[i] || fresh) && conc[i] >= q.noofloss && !p.upc[i] && !p.uc[i] && dbuy[i] && p.c[i] > q.penny && turn[i];
        sell[i] = ((sw.trend[i] == -1 || !athc[i]) && !p.loc[i]) || dsell[i];
    }
    let buy = exrem(&buy, &sell);
    let sell = exrem(&sell, &buy);
    let mut sg = Signals::new(buy, sell, p.c.clone(), p.c.clone());
    // ---- short
    let highma = ma_n(&p.h, q.highma);
    let lowma = ma_n(&p.l, q.lowma);
    let closema = ma_n(&p.c, q.closema);
    let rsi = rsi_w(&p.c, q.rsi_prd);
    let adx = adx_w(&p.h, &p.l, &p.c, q.adx_prd);
    // ST filter on the stock's own bars in the chosen time frame
    let mtrend_down: Vec<bool> = if q.filter_on && !q.filter_swing {
        let dates = p.date.clone();
        let al = Al { o: p.o.clone(), h: p.h.clone(), l: p.l.clone(), c: p.c.clone() };
        let f = Flt { tf: q.flt_tf, sma: false, point: q.st_point, pd: q.flt_pd, fact: q.flt_fact, mapd: 1 };
        st_filter(&al, &dates, &f).1
    } else {
        vec![false; n]
    };
    // %swing filter: Up = C + C x bull %, Dn = C - C x bear %
    let swing_down: Vec<bool> = if q.filter_on && q.filter_swing {
        let bl: Vec<f32> = p.c.iter().map(|&x| x * q.fact_bull / 100.0).collect();
        let br: Vec<f32> = p.c.iter().map(|&x| x * q.fact_bear / 100.0).collect();
        let up: Vec<f32> = (0..n).map(|i| p.c[i] + bl[i]).collect();
        let dn: Vec<f32> = (0..n).map(|i| p.c[i] - br[i]).collect();
        let (uf, df) = (up.clone(), dn.clone());
        swing_loop(&p.c, up, dn, &uf, &df, None).trend.iter().map(|&t| t == -1).collect()
    } else {
        vec![false; n]
    };
    let (mut short, mut cover) = (vec![false; n], vec![false; n]);
    let (mut shp, mut cvp) = (p.c.clone(), p.c.clone());
    const BIG: f32 = 999999.0;
    let (mut pos, mut abc) = (false, true);
    let (mut pt, mut low_trail, mut ll_after) = (0f32, BIG, BIG);
    let (mut daycount, mut highcount) = (vec![0f32; n], vec![0f32; n]);
    let (mut hardstoploss, mut sllow, mut trailsl, mut highsl) = (vec![BIG; n], vec![BIG; n], vec![BIG; n], vec![BIG; n]);
    let gap = |o: f32| o + (o * 0.25) / 100.0;
    for i in 1..n {
        // the RSI cross flag is only kept by the "Off" and the "On / ST" blocks
        if !(q.filter_on && q.filter_swing) && abc && rsi[i] < q.rsi_val && rsi[i - 1] > q.rsi_val {
            abc = false;
        }
        let gate = if !q.filter_on {
            !abc
        } else if !q.filter_swing {
            mtrend_down[i]
        } else {
            swing_down[i]
        };
        let entry = !pos
            && gate
            && p.c[i] < lowma[i]
            && lowma[i] < closema[i]
            && rsi[i] < q.rsi_val
            && adx[i] > q.adx_value
            && dbuy[i]
            && s.bars[i].aux2 == 1.0; // InWatchListName("FO C&P")
        let reset = |hsl: &mut Vec<f32>, tsl: &mut Vec<f32>, hi: &mut Vec<f32>, dc: &mut Vec<f32>, hc: &mut Vec<f32>, i: usize| {
            hsl[i] = BIG;
            tsl[i] = BIG;
            hi[i] = BIG;
            dc[i] = 0.0;
            hc[i] = 0.0;
        };
        if entry {
            short[i] = true;
            shp[i] = p.c[i];
            ll_after = p.l[i];
            sllow[i] = p.l[i];
            if q.pt_pct > 0.0 {
                pt = (1.0 - q.pt_pct * 0.01) * p.c[i];
            }
            if q.low_n_pct > 0.0 {
                low_trail = (1.0 + q.low_n_pct * 0.01) * ll_after;
            }
            hardstoploss[i] = if q.hardsl > 0.0 { p.c[i] + (p.c[i] * q.hardsl) / 100.0 } else { BIG };
            daycount[i] = daycount[i - 1] + 1.0;
            highsl[i] = BIG;
            pos = true;
        } else if pos && p.h[i] > hardstoploss[i - 1] && daycount[i - 1] <= q.ndays {
            cover[i] = true;
            cvp[i] = if p.o[i] > hardstoploss[i - 1] { gap(p.o[i]) } else { hardstoploss[i - 1] };
            reset(&mut hardstoploss, &mut trailsl, &mut highsl, &mut daycount, &mut highcount, i);
            low_trail = BIG;
            ll_after = BIG;
            pt = 0.0;
            pos = false;
            abc = true;
        } else if pos && p.h[i] > trailsl[i - 1] {
            cover[i] = true;
            cvp[i] = if p.o[i] > trailsl[i - 1] { gap(p.o[i]) } else { trailsl[i - 1] };
            reset(&mut hardstoploss, &mut trailsl, &mut highsl, &mut daycount, &mut highcount, i);
            low_trail = BIG;
            ll_after = BIG;
            pt = 0.0;
            pos = false;
            abc = true;
        } else if pos && p.h[i] > highsl[i - 1] {
            cover[i] = true;
            cvp[i] = if p.o[i] > highsl[i - 1] { gap(p.o[i]) } else { highsl[i - 1] };
            reset(&mut hardstoploss, &mut trailsl, &mut highsl, &mut daycount, &mut highcount, i);
            low_trail = BIG;
            ll_after = BIG;
            pt = 0.0;
            pos = false;
            abc = true;
        } else if pos && p.l[i] <= pt {
            cover[i] = true;
            cvp[i] = if p.o[i] < pt { gap(p.o[i]) } else { pt };
            reset(&mut hardstoploss, &mut trailsl, &mut highsl, &mut daycount, &mut highcount, i);
            low_trail = BIG; // LL_AfterShort is not reset here in the AFL (the next short sets it again)
            pt = 0.0;
            pos = false;
            abc = true;
        } else if pos && (p.c[i] >= highma[i] || p.c[i] > closema[i] || dsell[i]) {
            cover[i] = true;
            cvp[i] = p.c[i];
            reset(&mut hardstoploss, &mut trailsl, &mut highsl, &mut daycount, &mut highcount, i);
            low_trail = BIG;
            ll_after = BIG;
            pt = 0.0;
            pos = false;
            abc = true;
        } else if pos {
            daycount[i] = daycount[i - 1] + 1.0;
            hardstoploss[i] = hardstoploss[i - 1];
            sllow[i] = p.l[i].min(sllow[i - 1]);
            trailsl[i] = if q.slpct > 0.0 {
                if sllow[i] < sllow[i - 1] { sllow[i] + (sllow[i] * q.slpct) / 100.0 } else { trailsl[i - 1] }
            } else {
                BIG
            };
            if q.hdays > 0.0 {
                if p.h[i] > p.h[i - 1] {
                    highcount[i] = highcount[i - 1] + 1.0;
                    highsl[i] = if highcount[i] >= q.hdays { p.h[i] } else { highsl[i - 1] };
                } else if p.h[i] < p.h[i - 1] {
                    highsl[i] = highsl[i - 1];
                    highcount[i] = 0.0;
                }
            }
            if q.low_n_pct > 0.0 {
                ll_after = p.l[i].min(ll_after);
                low_trail = (1.0 + q.low_n_pct * 0.01) * ll_after;
                if p.c[i] > low_trail {
                    cover[i] = true;
                    cvp[i] = p.c[i];
                    reset(&mut hardstoploss, &mut trailsl, &mut highsl, &mut daycount, &mut highcount, i);
                    low_trail = BIG;
                    ll_after = BIG;
                    pt = 0.0;
                    pos = false;
                    abc = true;
                }
            }
        }
    }
    // SetPositionSize(IIf(Buy, BUYALLOCATION, SELLALLOCATION), spsPercentOfEquity)
    let buyalloc = q.eqallo * q.buysize / 100.0;
    let sellalloc = (100.0 - q.eqallo) * q.sellsize / 100.0;
    sg.size_pct = Some(buyalloc);
    let leg = ShortLeg { short, cover, short_price: shp, cover_price: cvp, size_pct: Some(sellalloc) };
    sg.short_leg = Some(leg);
    // SetOption("MaxOpenLong", EQALLO / BUYALLOCATION), ("MaxOpenShort", (100 - EQALLO) / SELLALLOCATION)
    let cap = |x: f32| if x.is_finite() && x >= 1.0 { x as usize } else if x.is_finite() { 0 } else { 100_000 };
    sg.max_long = cap(q.eqallo / buyalloc).max(1);
    sg.max_short = cap((100.0 - q.eqallo) / sellalloc).max(1);
    clamp_prices(&mut sg, s);
    sg
}

fn schema_combo() -> Vec<P> {
    vec![
        p("eqallo", "EQUITY ALLOCATION FOR LONG % (the AFL's default 70 is outside its own range 0-60 and is clamped to 60)", 70.0, 0.0, 60.0, 10.0),
        p("buysize", "POS. SIZE %OF EQUITY for Long", 5.0, 0.0, 100.0, 1.0),
        p("sellsize", "POS. SIZE %OF EQUITY for Short", 10.0, 0.0, 100.0, 1.0),
        dead_p(2.0),
        penny_p(),
        turn_p(),
        p("exitpct", "ATHC %SWING EXIT %", 18.0, 5.0, 22.0, 1.0),
        p("losspct", "MIN PROFIT %", 3.0, 0.0, 5.0, 1.0),
        p("noofloss", "CONSICUTIVE LOSS", 2.0, 0.0, 2.0, 1.0),
        p("comm", "BROKERAGE (enters the AFL's virtual loss counter)", 0.65, 0.0, 1.0, 0.01),
        toggle_p("ver", "Filter On/Off for the short (1 Off, 2 On)", 1.0),
        toggle_p("ver1", "Filter ST / %Swing (1 ST, 2 %Swing)", 1.0),
        p("pt_pct", "PT Percent", 17.0, 0.0, 20.0, 1.0),
        p("highma_prd", "HighMA Prd", 13.0, 1.0, 300.0, 1.0),
        p("lowma_prd", "LowMA Prd (the AFL's default 13 is outside its range 1-10 and is clamped to 10)", 13.0, 1.0, 10.0, 1.0),
        p("closema_prd", "CloseMA Prd", 5.0, 2.0, 10.0, 1.0),
        p("adx_prd", "ADX Period", 10.0, 1.0, 20.0, 1.0),
        p("adx_value", "ADX Value", 10.0, 1.0, 50.0, 1.0),
        p("rsi_prd", "RSI Period", 5.0, 1.0, 100.0, 1.0),
        p("rsi_short_val", "RSI Short Value", 50.0, 5.0, 100.0, 5.0),
        p("flt_tf", "FILTER TIMEFRAME (1 DAILY, 2 WEEKLY, 3 MONTHLY)", 1.0, 1.0, 3.0, 1.0),
        p("st_flt", "ST CALCULATION POINT (1 MID POINT, 2 CLOSE, 3 PIVOT)", 1.0, 1.0, 3.0, 1.0),
        p("flt_pd", "ST PERIOD", 5.0, 5.0, 25.0, 1.0),
        p("flt_fact", "ST FACTOR", 4.0, 1.0, 7.0, 1.0),
        p("fltfact1", "SWING % FOR BEAR MKT", 23.0, 10.0, 60.0, 1.0),
        p("fltfact2", "SWING % FOR BULL MKT", 44.0, 10.0, 60.0, 1.0),
        p("hardsl", "HARD SL %", 1.0, 0.0, 12.0, 1.0),
        p("ndays", "HARD SL DAYS", 1.0, 0.0, 100.0, 1.0),
        p("slpct", "TRAILING SL PCT", 1.0, 0.0, 120.0, 1.0),
        p("low_n_pct", "Low + n%", 3.0, 0.0, 120.0, 1.0),
        p("hdays", "HIHGHER HIGH DAYS", 0.0, 0.0, 100.0, 1.0),
    ]
}

fn sig_combo(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let q = Combo {
        eqallo: g("eqallo") as f32,
        buysize: g("buysize") as f32,
        sellsize: g("sellsize") as f32,
        dead_yes: g("deadstock").round() == 1.0,
        penny: g("pennyflt") as f32,
        turn_on: g("turnflt").round() == 2.0,
        exit_pct: g("exitpct") as f32,
        losspct: g("losspct") as f32,
        noofloss: g("noofloss") as f32,
        comm: g("comm") as f32,
        filter_on: g("ver").round() == 2.0,
        filter_swing: g("ver1").round() == 2.0,
        pt_pct: g("pt_pct") as f32,
        highma: g("highma_prd").round() as usize,
        lowma: g("lowma_prd").round() as usize,
        closema: g("closema_prd").round() as usize,
        adx_prd: g("adx_prd").round() as usize,
        adx_value: g("adx_value") as f32,
        rsi_prd: g("rsi_prd").round() as usize,
        rsi_val: g("rsi_short_val") as f32,
        flt_tf: (g("flt_tf").round() as u8).max(1) - 1, // DAILY|WEEKLY|MONTHLY -> compress codes 2, 1, 0
        st_point: (g("st_flt").round() as u8).max(1) - 1,
        flt_pd: g("flt_pd").round() as usize,
        flt_fact: g("flt_fact") as f32,
        fact_bear: g("fltfact1") as f32,
        fact_bull: g("fltfact2") as f32,
        hardsl: g("hardsl") as f32,
        ndays: g("ndays") as f32,
        slpct: g("slpct") as f32,
        low_n_pct: g("low_n_pct") as f32,
        hdays: g("hdays") as f32,
    };
    let mut q = q;
    q.flt_tf = match q.flt_tf { 0 => 2, 1 => 1, _ => 0 }; // Flt.tf: 0 monthly, 1 weekly, 2 daily
    // st_flt list is MID POINT | CLOSE | PIVOT (Flt.point: 0 mid, 1 close, 2 pivot): value - 1 already
    series.iter().map(|s| combo_signals(s, &q)).collect()
}

// ================================================================ 16. ATHC-5 % range SHORT entry, cover on ATR trail SL / % trail SL / new ATHC

#[derive(Clone, Copy)]
pub struct ShortQ {
    pub dead_yes: bool,
    pub penny: f32,
    pub turn_on: bool,
    pub pct_trail_version: bool,
    pub buypct: f32,
    pub atr_prd: usize,
    pub atr_m: f32,
    pub trail_pct: f32,
    pub exit_pct: f32,
}

/// The AFL's `Short` / `Cover` state machine (`BuySig` 1 -> 2 when the stock's own %swing turns down; short at the close within BUYPCT below the ATHC, cover at the close on
/// the trailing stop or a new ATHC). The AFL has no Buy / Sell: every trade is a short sale.
pub fn short_signals(s: &Series, q: &ShortQ) -> Signals {
    let p = px(s);
    let n = p.n;
    let trend1 = scan::swing_trend(&p.c, &p.c, q.exit_pct);
    let turn = turnover(&p, q.turn_on, true);
    let (dbuy, dsell) = dead_flags(n, q.dead_yes);
    let tr = tr1(&p.h, &p.l, &p.c);
    let myatr = atr_n(&tr, q.atr_prd);
    let buysig2: Vec<f32> = (0..n).map(|i| p.hcl[i] - (p.hcl[i] * q.buypct) / 100.0).collect();
    let (mut short, mut cover) = (vec![false; n], vec![false; n]);
    let mut buysig = 1i32;
    let (mut skipbuy, mut skipsell, mut position) = (false, false, false);
    let (mut lc_af_sell, mut trailsl, mut atrvalue) = (0f32, 0f32, 0f32);
    let mut atr_trailsl = vec![0f32; n];
    for i in 1..n {
        let hprev = p.hcl[i - 1];
        let filt = !p.lc[i] && turn[i] && p.c[i] > q.penny && dbuy[i]; // Short_Cash_Filter
        // BuySig bookkeeping
        if p.c[i] > hprev && buysig == 0 {
            buysig = 1;
        } else if trend1[i] == -1 && buysig == 1 {
            buysig = 2;
        } else if !position && buysig == 2 && p.c[i] > hprev {
            buysig = 1;
        }
        let atr_mode = !q.pct_trail_version;
        let enter = |atr_trailsl: &mut Vec<f32>, lc: &mut f32, tsl: &mut f32| {
            if atr_mode {
                if q.atr_m > 0.0 {
                    *lc = p.c[i];
                    atr_trailsl[i] = *lc + (myatr[i] * q.atr_m);
                }
            } else if q.trail_pct > 0.0 {
                *lc = p.c[i];
                *tsl = (1.0 + q.trail_pct * 0.01) * *lc;
            }
        };
        if !position && p.c[i] > buysig2[i - 1] && buysig == 2 && !skipbuy {
            if filt && p.c[i] < hprev {
                short[i] = true;
                enter(&mut atr_trailsl, &mut lc_af_sell, &mut trailsl);
                position = true;
            } else {
                skipbuy = true;
            }
        } else if skipbuy && p.c[i] > hprev {
            skipbuy = false;
            buysig = 1;
        } else if skipbuy && buysig == 2 && p.c[i] < hprev && filt {
            short[i] = true;
            enter(&mut atr_trailsl, &mut lc_af_sell, &mut trailsl);
            position = true;
            skipbuy = false;
        } else if position && !skipsell {
            if atr_mode {
                if q.atr_m > 0.0 {
                    lc_af_sell = p.c[i].min(lc_af_sell);
                    if p.c[i] == lc_af_sell {
                        atrvalue = myatr[i];
                    }
                }
            } else if q.trail_pct > 0.0 {
                lc_af_sell = p.c[i].min(lc_af_sell);
            }
            let stop_level = if atr_mode { atr_trailsl[i - 1] } else { trailsl };
            if p.c[i] >= stop_level || p.c[i] > hprev {
                if !p.uc[i] || dsell[i] {
                    cover[i] = true;
                    if p.c[i] > hprev {
                        buysig = 1;
                    }
                    position = false;
                    skipsell = false;
                    lc_af_sell = 0.0;
                    atr_trailsl[i] = 0.0;
                    trailsl = 0.0;
                } else {
                    skipsell = true;
                    if p.c[i] > hprev {
                        buysig = 1;
                    }
                }
            }
            if atr_mode {
                if q.atr_m > 0.0 {
                    atr_trailsl[i] = if atrvalue > 0.0 { lc_af_sell + (atrvalue * q.atr_m) } else { atr_trailsl[i - 1] };
                    if atr_trailsl[i] > atr_trailsl[i - 1] {
                        atr_trailsl[i] = atr_trailsl[i - 1];
                    }
                }
            } else if q.trail_pct > 0.0 {
                trailsl = (1.0 + q.trail_pct * 0.01) * lc_af_sell;
            }
        } else if (position && !p.uc[i] && skipsell) || (position && dsell[i]) {
            cover[i] = true;
            position = false;
            skipsell = false;
            lc_af_sell = 0.0;
            atr_trailsl[i] = 0.0;
            trailsl = 0.0;
        }
    }
    let mut sg = blank(n);
    sg.short_leg = Some(ShortLeg { short, cover, short_price: p.c.clone(), cover_price: p.c.clone(), size_pct: None });
    clamp_prices(&mut sg, s);
    sg
}

fn schema_short() -> Vec<P> {
    vec![
        dead_p(2.0),
        penny_p(),
        turn_p(),
        toggle_p("ver", "Exit Versions (1 ATR Trail SL, 2 % Trail SL)", 1.0),
        p("buypct", "Buy %", 5.0, 0.0, 100.0, 1.0),
        p("atr_prd", "ATR Prds", 5.0, 1.0, 100.0, 1.0),
        p("atrtrailsl_m", "ATR Trail SL Multiplier", 5.0, 0.0, 20.0, 1.0),
        p("trailsl_pct", "Trailing SL %", 5.0, 0.0, 30.0, 0.01),
        p("exitpct", "Swing EXIT %", 10.0, 1.0, 30.0, 1.0),
    ]
}

fn sig_short(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let q = ShortQ {
        dead_yes: g("deadstock").round() == 1.0,
        penny: g("pennyflt") as f32,
        turn_on: g("turnflt").round() == 2.0,
        pct_trail_version: g("ver").round() == 2.0,
        buypct: g("buypct") as f32,
        atr_prd: g("atr_prd").round() as usize,
        atr_m: g("atrtrailsl_m") as f32,
        trail_pct: g("trailsl_pct") as f32,
        exit_pct: g("exitpct") as f32,
    };
    series.iter().map(|s| short_signals(s, &q)).collect()
}

// ================================================================ registry

pub fn variants() -> Vec<VariantDef> {
    vec![
        VariantDef { id: "ytath_close_buy_supertrend_10_4_exit_live", data: CASH, schema: schema_close_buy, signals: sig_close_buy },
        VariantDef { id: "ytath_btst_on_athc_modified_with_index", data: CASH, schema: schema_btst_index, signals: sig_btst_index },
        VariantDef { id: "ytath_btst_on_athc_stock_selection_by_high", data: CASH, schema: schema_btst_plain, signals: sig_btst_high },
        VariantDef { id: "ytath_btst_on_athc_stock_selection_by_roc", data: CASH, schema: schema_btst_roc, signals: sig_btst_roc },
        VariantDef { id: "ytath_live_market_entry_and_exit_on", data: FO, schema: schema_live_exit, signals: sig_live_exit },
        VariantDef { id: "ytath_with_live_market_entry_supertrend_10", data: CASH, schema: schema_live_st, signals: sig_live_st },
        VariantDef { id: "ytath_btst_of_athc_with_upper_bollinger", data: CASH, schema: schema_bollinger, signals: sig_bollinger },
        VariantDef { id: "ytath_intraday_on_athc_with_ranking_buy_at", data: CASH, schema: schema_intraday, signals: sig_intraday },
        VariantDef { id: "ytath_5pct_to_30pct_scanner_1_consecutive", data: CASH, schema: schema_scanner_loss, signals: sig_scanner_loss },
        VariantDef { id: "ytath_5pct_to_30pct_scanner", data: CASH, schema: schema_scanner, signals: sig_scanner },
        VariantDef { id: "ytath_stock_count_daily_scanner_of_stocks", data: CASH, schema: schema_stock_count, signals: sig_none },
        VariantDef { id: "ytath_pr_plot_nse_midcap_nse100_price", data: CASH, schema: schema_pr_plot, signals: sig_none },
        VariantDef { id: "ytath_pr_plot_nse_smallcap100_nse100_price", data: CASH, schema: schema_pr_plot, signals: sig_none },
        VariantDef { id: "ytath_pr_plot_nse_smallcap100_nse_midcap", data: CASH, schema: schema_pr_plot, signals: sig_none },
        VariantDef { id: "ytath_long_after_losses_plus_ma_rsi_adx", data: CASH, schema: schema_combo, signals: sig_combo },
        VariantDef { id: "ytath_5pct_range_short_entry_cover_on_atr", data: CASH, schema: schema_short, signals: sig_short },
    ]
}


#[cfg(test)]
mod tests {
    use super::*;

    fn bar(date: i32, o: f32, h: f32, l: f32, c: f32) -> Bar {
        Bar { date, c, o, h, l, v: 1.0e6, oi: 1.0e9, aux1: 1.0, aux2: 1.0 }
    }
    fn series(rows: &[(f32, f32, f32, f32)]) -> Series {
        Series { name: "T".into(), bars: rows.iter().enumerate().map(|(i, &(o, h, l, c))| bar(20200101 + i as i32, o, h, l, c)).collect() }
    }
    /// bars with open = close, high = close + 1, low = close - 1
    fn closes(c: &[f32]) -> Series {
        series(&c.iter().map(|&x| (x, x + 1.0, x - 1.0, x)).collect::<Vec<_>>())
    }
    fn pp(max: usize) -> PortfolioParams {
        PortfolioParams { initial_equity: 100_000.0, pct_of_equity: 100.0 / max as f64, fixed_amount: None, max_positions: max, commission_pct: 0.0, start: 20000101, end: 20300101, amibroker_profit: false, amibroker_full: false, current_equity_sizing: false, size_f32_price: false }
    }
    fn run(s: &Series, sg: &Signals, max: usize) -> Backtest {
        backtest_full(&[(s, sg)], &[0], &pp(max))
    }

    #[test]
    fn schema_defaults_follow_the_catalog_and_every_id_is_registered() {
        let v = variants();
        assert_eq!(v.len(), 16);
        let mut ids: Vec<&str> = v.iter().map(|x| x.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 16);
        let find = |id: &str| v.iter().find(|x| x.id == id).unwrap();
        let get = |id: &str, k: &str| (find(id).schema)().into_iter().find(|q| q.key == k).unwrap_or_else(|| panic!("{k}")).def;
        assert_eq!(get("ytath_close_buy_supertrend_10_4_exit_live", "deadstock"), 2.0);
        assert_eq!(get("ytath_close_buy_supertrend_10_4_exit_live", "buff"), 1.5);
        assert_eq!(get("ytath_btst_on_athc_modified_with_index", "deadstock"), 1.0, "ParamToggle default 0 = YES");
        assert_eq!(get("ytath_btst_of_athc_with_upper_bollinger", "bb_emaprd"), 15.0);
        assert_eq!(get("ytath_5pct_to_30pct_scanner", "exitpct"), 10.0);
        assert_eq!(get("ytath_live_market_entry_and_exit_on", "buffer_pct"), 0.25);
        assert!(matches!(find("ytath_live_market_entry_and_exit_on").data, Data::Fo22 { .. }));
        assert_eq!(get("ytath_5pct_range_short_entry_cover_on_atr", "buypct"), 5.0);
        assert_eq!(get("ytath_long_after_losses_plus_ma_rsi_adx", "rsi_short_val"), 50.0);
    }

    // ---- core features added for this batch
    #[test]
    fn core_same_bar_round_trip_keeps_its_slot_and_books_the_bar_s_trade() {
        let s = closes(&[10.0, 10.0, 10.0]);
        let mut sg = Signals::new(vec![false, true, false], vec![false, true, false], vec![0.0, 10.0, 0.0], vec![0.0, 10.5, 0.0]);
        sg.same_bar_exit = true;
        let bt = run(&s, &sg, 1);
        assert_eq!(bt.trades.len(), 1);
        let t = &bt.trades[0];
        assert_eq!((t.entry_date, t.exit_date, t.entry_price, t.exit_price), (20200102, 20200102, 10.0, 10.5));
        assert!(!t.open && !t.short);
        // without the flag the sell on the entry bar is ignored and the position stays open
        sg.same_bar_exit = false;
        let bt = run(&s, &sg, 1);
        assert!(bt.trades[0].open);
    }

    #[test]
    fn core_sell_then_buy_on_one_bar_is_a_re_entry_not_a_round_trip() {
        let s = closes(&[10.0, 10.0, 10.0, 10.0]);
        let mut sg = Signals::new(vec![false, true, false, true], vec![false, false, false, true], vec![0.0, 10.0, 0.0, 11.0], vec![0.0, 0.0, 0.0, 9.0]);
        sg.same_bar_exit = true;
        let bt = run(&s, &sg, 1);
        let closed: Vec<_> = bt.trades.iter().filter(|t| !t.open).collect();
        assert_eq!(closed.len(), 1);
        assert_eq!((closed[0].entry_date, closed[0].exit_date, closed[0].exit_price), (20200102, 20200104, 9.0));
        let open: Vec<_> = bt.trades.iter().filter(|t| t.open).collect();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].entry_date, 20200104, "bought again on the sell bar and held");
    }

    #[test]
    fn core_short_sale_profit_has_the_right_sign_and_the_equity_follows() {
        let s = closes(&[10.0, 10.0, 8.0, 8.0]);
        let mut sg = blank(4);
        sg.short_leg = Some(ShortLeg { short: vec![false, true, false, false], cover: vec![false, false, true, false], short_price: vec![10.0; 4], cover_price: vec![8.0; 4], size_pct: Some(50.0) });
        let bt = run(&s, &sg, 5);
        assert_eq!(bt.trades.len(), 1);
        let t = &bt.trades[0];
        assert!(t.short);
        assert_eq!(t.shares, 5000.0, "50 % of 100,000 at 10");
        assert!((t.profit - 10_000.0).abs() < 1e-6, "{}", t.profit);
        assert!((bt.equity.last().unwrap() - 110_000.0).abs() < 1e-6);
        // mark to market while open: short at 10, price 8 -> +10,000
        sg.short_leg.as_mut().unwrap().cover = vec![false; 4];
        let bt = run(&s, &sg, 5);
        assert!((bt.equity[2] - 110_000.0).abs() < 1e-6 && bt.trades[0].open && (bt.trades[0].profit - 10_000.0).abs() < 1e-6);
    }

    #[test]
    fn core_separate_long_and_short_caps_replace_the_portfolio_cap() {
        let s = closes(&[10.0; 3]);
        let mut sg = Signals::new(vec![false, true, false], vec![false; 3], vec![10.0; 3], vec![10.0; 3]);
        sg.max_long = 1;
        sg.max_short = 1;
        sg.size_pct = Some(10.0);
        let a = run(&s, &sg, 1);
        assert_eq!(a.accepted, 1);
        // two symbols: one long, one short fit although max_positions is 1; a second long would not
        let mut sh = blank(3);
        sh.short_leg = Some(ShortLeg { short: vec![false, true, false], cover: vec![false; 3], short_price: vec![10.0; 3], cover_price: vec![10.0; 3], size_pct: Some(10.0) });
        sh.max_long = 1;
        sh.max_short = 1;
        let s2 = closes(&[10.0; 3]);
        let bt = backtest_full(&[(&s, &sg), (&s2, &sh)], &[0, 1], &pp(1));
        assert_eq!(bt.accepted, 2);
    }

    // ---- 1. close buy with the Supertrend live-market exit
    #[test]
    fn buytrend_lags_by_one_bar_and_resets_on_a_supertrend_down_bar() {
        // hcl: highest close so far; close 12 breaks 11 at bar 4, but buytrigger[3] = hcl[2] only exists when exit was 0 at bar 3
        let c = [10.0f32, 10.0, 11.0, 11.0, 12.0, 12.0, 9.0, 12.5];
        let mut hcl = c.to_vec();
        for i in 1..c.len() {
            hcl[i] = hcl[i].max(hcl[i - 1]);
        }
        let trend = [1i8, 1, 1, 1, 1, 1, -1, 1];
        let bt = buytrend_loop(&c, &hcl, &trend);
        assert!(bt[0].is_nan() && bt[1].is_nan());
        assert_eq!(bt[2], 0.0);
        assert_eq!(bt[3], 1.0, "close 11 > buytrigger[2] = hcl[1] = 10: the trigger compares with the highest close TWO bars back");
        assert_eq!(bt[4], 1.0);
        assert_eq!(bt[5], 1.0);
        assert_eq!(bt[6], 1.0, "still 1 on the bar the trend flips (exit is reset after the bar)");
        assert_eq!(bt[7], 0.0, "exit reset: back to 0; buytrigger[6] is Null (exit was 1 at bar 6) so 12.5 does not re-trigger");
    }

    #[test]
    fn close_buy_live_sell_exits_at_the_level_and_the_whipsaw_rebuy_cannot_fire() {
        // a steady up-trend (a new high close every bar) then a dip bar whose Low pierces yesterday's level
        let mut rows: Vec<(f32, f32, f32, f32)> = vec![(100.0, 101.0, 99.0, 100.0); 15];
        for i in 0..25 {
            let c = 101.0 + i as f32;
            rows.push((c - 0.5, c + 1.0, c - 1.0, c));
        }
        rows.push((124.0, 126.0, 105.0, 126.0)); // Low far below yesterday's tradelvl, Close back up
        for i in 0..5 {
            let c = 142.0 + i as f32;
            rows.push((c - 0.5, c + 1.0, c - 1.0, c));
        }
        let s = series(&rows);
        let sg = close_buy_signals(&s, false, 1.5);
        let dip = 40;
        assert!(sg.buy[15], "Close crosses above the previous ATH close after a flat spell");
        assert!(sg.sell[dip], "Low < yesterday's tradelvl while buytrend == 1: live-market exit");
        let b = &s.bars[dip];
        assert!(sg.sell_price[dip] >= b.l && sg.sell_price[dip] <= b.h && sg.sell_price[dip] < 124.0);
        // the AFL's whipsaw Buy needs `C > Ref(trenddown,-1)`: TrendDown is Null in an up-trend and a comparison with Null is false, so the re-entry never fires
        assert!(!sg.buy[dip]);
        assert!(sg.same_bar_exit);
        let bt = run(&s, &sg, 1);
        let closed: Vec<_> = bt.trades.iter().filter(|t| t.exit_date == s.bars[dip].date && !t.open).collect();
        assert_eq!(closed.len(), 1, "the position leaves at the level");
    }

    // ---- 2-4. BTST
    #[test]
    fn btst_buys_every_ath_close_and_sells_the_next_open_minus_buffer() {
        let s = series(&[(100.0, 101.0, 99.0, 100.0), (100.0, 102.0, 99.0, 101.0), (102.0, 104.0, 101.0, 103.0), (101.0, 103.0, 100.0, 102.0), (100.0, 100.5, 98.0, 99.0)]);
        let sg = btst_signals(&s, false, 0.0, false, 0.25, None, BtstScore::None, true, false);
        assert_eq!(sg.buy, vec![false, true, true, false, false], "ExRem(Buy, Sell) with a Sell every bar keeps every ATH close");
        assert_eq!(sg.sell, vec![true, false, true, true, false].iter().enumerate().map(|(i, _)| sg.sell[i]).collect::<Vec<_>>());
        assert!(sg.sell[2] && sg.sell[3] && !sg.sell[4], "sell on the bar after each buy (and on the bar of the next buy)");
        assert_eq!(sg.buy_price[1], 101.0);
        assert_eq!(sg.sell_price[2], 102.0 - (102.0f32 * 0.25) / 100.0);
        let bt = run(&s, &sg, 1);
        assert_eq!(bt.trades.len(), 2);
        assert_eq!((bt.trades[0].entry_date, bt.trades[0].exit_date), (20200102, 20200103));
        assert_eq!((bt.trades[1].entry_date, bt.trades[1].exit_date), (20200103, 20200104), "bought again at the close of the day it sold the first position at the open");
    }

    #[test]
    fn btst_upper_circuit_blocks_the_buy_and_lower_circuit_postpones_the_sell() {
        let mut s = series(&[(100.0, 101.0, 99.0, 100.0), (101.0, 101.0, 101.0, 101.0), (101.0, 103.0, 100.0, 102.0), (90.0, 90.0, 90.0, 90.0), (91.0, 92.0, 90.0, 91.5)]);
        let sg = btst_signals(&s, false, 0.0, false, 0.25, None, BtstScore::None, true, false);
        assert!(!sg.buy[1], "O = H = L = C on a new high close: upper circuit");
        assert!(sg.buy[2]);
        assert!(!sg.sell[3], "lower circuit bar: no sell");
        assert!(sg.sell[4], "sold the next bar");
        let off = btst_signals(&s, false, 0.0, false, 0.25, None, BtstScore::None, false, false);
        assert!(off.buy[1] && off.sell[3]);
        s.bars[2].c = 100.5; // no longer a new high close
        assert!(!btst_signals(&s, false, 0.0, false, 0.25, None, BtstScore::None, true, false).buy[2]);
    }

    #[test]
    fn btst_rankings_match_the_afl_formulas() {
        let s = series(&[(100.0, 101.0, 99.0, 100.0), (100.0, 104.0, 99.0, 102.0)]);
        let hc = btst_signals(&s, false, 0.0, false, 0.25, None, BtstScore::HighClose, true, false);
        let diffpct = (104.0f32 - 102.0) / 104.0 * 100.0;
        assert_eq!(hc.score.as_ref().unwrap()[1], 100.0 - diffpct);
        let roc = btst_signals(&s, false, 0.0, false, 0.25, None, BtstScore::NegRoc, true, false);
        assert_eq!(roc.score.as_ref().unwrap()[1], -((102.0f32 - 100.0) / 100.0 * 100.0));
        assert!(roc.score.as_ref().unwrap()[0].is_nan());
        assert!(btst_signals(&s, false, 0.0, false, 0.25, None, BtstScore::None, true, false).score.is_none());
    }

    #[test]
    fn btst_index_filter_gates_the_buy() {
        let s = series(&[(100.0, 101.0, 99.0, 100.0), (100.0, 102.0, 99.0, 101.0), (101.0, 103.0, 100.0, 102.0)]);
        let sg = btst_signals(&s, false, 0.0, false, 0.25, Some(&[true, false, true]), BtstScore::None, true, false);
        assert!(!sg.buy[1] && sg.buy[2]);
    }

    // ---- 5. Bollinger
    #[test]
    fn bollinger_version_1_sells_at_open_or_the_buffer_level_or_the_low() {
        let mut rows = vec![(100.0f32, 101.0f32, 99.0f32, 100.0f32); 20];
        rows.push((100.0, 120.0, 100.0, 118.0)); // new high close well above the band
        rows.push((110.0, 112.0, 100.0, 105.0)); // Low (100) below Open - 0.25 % -> sold at the level
        let s = series(&rows);
        let sg = bollinger_signals(&s, false, 0.0, false, false, 0.25, 15, 2.0, false, true);
        assert!(sg.buy[20] && sg.sell[21]);
        assert_eq!(sg.sell_price[21], 110.0 - (110.0f32 * 0.25) / 100.0);
        // Low above the level: sold at the Low
        let mut rows2 = rows.clone();
        rows2[21] = (110.0, 112.0, 109.9, 111.0);
        let sg2 = bollinger_signals(&series(&rows2), false, 0.0, false, false, 0.25, 15, 2.0, false, true);
        assert_eq!(sg2.sell_price[21], 109.9);
        // Open == Low: sold at the Open
        rows2[21] = (110.0, 112.0, 110.0, 111.0);
        let sg3 = bollinger_signals(&series(&rows2), false, 0.0, false, false, 0.25, 15, 2.0, false, true);
        assert_eq!(sg3.sell_price[21], 110.0);
        // version 2: at the Close
        let sg4 = bollinger_signals(&s, false, 0.0, false, true, 0.25, 15, 2.0, false, true);
        assert_eq!(sg4.sell_price[21], 105.0);
    }

    #[test]
    fn bollinger_needs_a_close_above_the_band_and_the_band_gate_can_be_switched_off() {
        // a new high close that is NOT above the band (very volatile history): no buy; the verification switch removes the band test
        let mut rows = vec![];
        for i in 0..30 {
            let c = if i % 2 == 0 { 80.0 } else { 120.0 };
            rows.push((c, c + 1.0, c - 1.0, c));
        }
        rows.push((121.0, 122.0, 120.0, 121.5));
        let s = series(&rows);
        assert!(!bollinger_signals(&s, false, 0.0, false, false, 0.25, 15, 2.0, false, true).buy[30]);
        assert!(bollinger_signals(&s, false, 0.0, false, false, 0.25, 15, 2.0, true, true).buy[30]);
    }

    // ---- 6. live-market entry with the Supertrend exit
    #[test]
    fn live_entry_fills_at_the_buy_level_and_a_failed_breakout_is_a_same_day_round_trip() {
        // flat history: highest close 100, buylvl = 100 x 1.015 = 101.5 while no breakout state is on
        let mut rows: Vec<(f32, f32, f32, f32)> = vec![(100.0, 100.5, 99.5, 100.0); 15];
        // spike through the level (High 103), Open below it, Close 99.8 below the previous ATH close: failed live breakout
        rows.push((100.5, 103.0, 99.5, 99.8));
        rows.push((99.8, 100.0, 99.0, 99.5));
        let s = series(&rows);
        let q = LiveSt { dead_yes: false, penny: 0.0, turn_on: false, buff: 1.5, stflt: 1, period: 10, factor: 4.0, hardstop: 0.0 };
        let sg = live_st_signals(&s, &q);
        assert!(sg.buy[15] && sg.sell[15], "bought at the live level and sold on the same bar");
        assert_eq!(sg.buy_price[15], 100.0 + (100.0f32 * 1.5) / 100.0, "Open (100.5) is below the level: filled at the level");
        assert_eq!(sg.sell_price[15], 99.8, "Close is not below selllvl (98.5): sold at the Close");
        assert!(sg.same_bar_exit);
        let bt = run(&s, &sg, 1);
        assert_eq!(bt.trades.iter().filter(|t| t.entry_date == t.exit_date).count(), 1);
        // gap above the level: Open + 0.25 %
        let mut rows2 = rows.clone();
        rows2[15] = (102.0, 103.0, 99.5, 99.8);
        let sg2 = live_st_signals(&series(&rows2), &q);
        assert_eq!(sg2.buy_price[15], 102.0 + (102.0f32 * 0.25) / 100.0);
    }

    // ---- 7. live market entry and exit on the intraday close
    #[test]
    fn live_exit_trades_the_breakout_day_and_waits_after_a_profit() {
        // up-trend, then a breakout day (High above, Open below the previous ATH close) that closes high: profit > 0 -> abc = 1 -> no further trade until the swing turns down
        let mut rows: Vec<(f32, f32, f32, f32)> = (0..15).map(|i| (100.0 + i as f32, 101.0 + i as f32, 99.0 + i as f32, 100.0 + i as f32)).collect();
        rows.push((113.0, 118.0, 112.0, 117.0)); // breakout, closes above ATHC 114
        rows.push((113.0, 119.0, 112.0, 118.0)); // breakout again (Open < ATHC 117): blocked while abc == 1
        let s = series(&rows);
        let sg = live_exit_signals(&s, false, 0.0, 10.0, 0.25, 0.25, 0.65);
        assert!(sg.buy[15] && sg.sell[15]);
        assert_eq!(sg.buy_price[15], (114.0f32 + (114.0f32 * 0.25) / 100.0).min(s.bars[15].h));
        assert_eq!(sg.sell_price[15], 117.0 - (117.0f32 * 0.25) / 100.0);
        assert!(!sg.buy[16], "after a profitable breakout the AFL waits for the %swing to turn down");
    }

    #[test]
    fn live_exit_after_a_loss_waits_seven_up_trend_bars() {
        // breakout day that closes below the buy level (a loss), then up-trend bars: the next trade only after 7 counted bars
        let mut rows: Vec<(f32, f32, f32, f32)> = (0..15).map(|i| (100.0 + i as f32, 101.0 + i as f32, 99.0 + i as f32, 100.0 + i as f32)).collect();
        rows.push((113.0, 116.0, 112.0, 113.5)); // breakout (H > 114, O < 114) that closes below: loss, abc = 2
        for _ in 0..12 {
            // quiet days that stay inside the range: no breakout, trend up
            rows.push((113.0, 113.8, 112.5, 113.5));
        }
        rows.push((113.0, 118.0, 112.5, 117.0)); // breakout again after the wait
        let s = series(&rows);
        let sg = live_exit_signals(&s, false, 0.0, 10.0, 0.25, 0.25, 0.65);
        assert!(sg.buy[15], "the first breakout is traded");
        let later: Vec<usize> = (16..s.bars.len()).filter(|&i| sg.buy[i]).collect();
        assert_eq!(later, vec![28], "no trade during the 7-bar wait, the next breakout is traded");
    }

    // ---- 8. intraday on ATHC with ranking
    #[test]
    fn intraday_state_is_the_previous_bars_ath_flag_and_a_supertrend_down_bar_leaves_it_off() {
        let mut c = vec![100.0f32, 101.0, 102.0, 103.0];
        for _ in 0..4 {
            c.push(103.0);
        }
        c.extend([60.0, 61.0, 62.0]); // a crash: the 10/4 Supertrend flips down
        let s = closes(&c);
        let q = Intraday { rankmode: 1, rocpd: 10, rsipd: 14, swingpd: 1000, prfltpd: 200, stflt: 1, tstart: 0, period: 3, factor: 1.0, buff: 0.25 };
        let sg = intraday_signals(&s, &q, None);
        assert!(!sg.buy[1] && sg.buy[2] && sg.buy[3] && sg.buy[4], "buy the day after an ATH close: Open + 0.25 %");
        assert_eq!(sg.buy_price[2], (102.0f32 + (102.0f32 * 0.25) / 100.0).min(s.bars[2].h));
        assert_eq!(sg.sell_price[2], 102.0);
        assert!(sg.same_bar_exit && sg.sell == sg.buy);
        assert!(sg.buy[6] && sg.buy[7], "state stays on while the Supertrend is up");
        assert!(!sg.buy[9] && !sg.buy[10], "after the down bar the state is Null until the next ATH close");
    }

    #[test]
    fn intraday_ranking_keeps_the_best_scores_when_slots_are_short() {
        // two symbols, one slot: the PositionScore picks the buy; both trade the same bar
        let a = closes(&[100.0, 101.0, 102.0, 103.0]);
        let b = closes(&[100.0, 101.0, 102.0, 103.0]);
        let q = Intraday { rankmode: 2, rocpd: 1, rsipd: 14, swingpd: 1000, prfltpd: 200, stflt: 1, tstart: 0, period: 3, factor: 4.0, buff: 0.0 };
        let mut sa = intraday_signals(&a, &q, None);
        let mut sb = intraday_signals(&b, &q, None);
        sa.score = Some(vec![1.0; 4]);
        sb.score = Some(vec![2.0; 4]);
        let bt = backtest_full(&[(&a, &sa), (&b, &sb)], &[0, 1], &pp(1));
        assert!(bt.trades.iter().all(|t| t.sym == 1), "the higher score takes the single slot every day");
        assert!(bt.trades.iter().all(|t| t.entry_date == t.exit_date));
        assert!(!bt.trades.is_empty());
        // the ranking formulas
        let r = intraday_signals(&a, &q, None);
        assert_eq!(r.score.as_ref().unwrap()[2], 100.0 + (102.0f32 - 101.0) / 101.0 * 100.0);
        let q3 = Intraday { rankmode: 3, ..q };
        assert_eq!(intraday_signals(&a, &q3, None).score.as_ref().unwrap()[2], 100.0 - (102.0f32 - 101.0) / 101.0 * 100.0);
        let q5 = Intraday { rankmode: 5, swingpd: 3, ..q };
        assert_eq!(intraday_signals(&a, &q5, None).score.as_ref().unwrap()[3], 100.0);
        assert!(intraday_signals(&a, &Intraday { rankmode: 1, ..q }, None).score.is_none());
        // PR mode: needs the index; without it the score is Null (ranked last), the start bar is the PR EMA period
        let q6 = Intraday { rankmode: 6, prfltpd: 2, ..q };
        let pr = vec![100.0f32, 100.0, 100.0, 100.0];
        let sg6 = intraday_signals(&a, &q6, Some(&pr));
        assert!(sg6.score.as_ref().unwrap()[3].is_finite());
        assert!(!sg6.buy[1] && sg6.buy[2], "BarIndex() >= prFLTPD = 2");
    }

    // ---- 9-10 scanners, 11-14 no signal
    #[test]
    fn scanner_turnover_tables_differ_and_the_default_exit_is_10() {
        let mut rows: Vec<(f32, f32, f32, f32)> = (0..10).map(|i| (100.0 + i as f32, 101.0 + i as f32, 99.0 + i as f32, 100.0 + i as f32)).collect();
        rows.push((100.0, 101.0, 99.0, 100.0));
        let mut s = series(&rows);
        for b in s.bars.iter_mut() {
            b.v = 1.0e5; // V*C about 1e7 ( < 13e6 tier of 2020 ) ; OpenInt 1e9 >> 9e6
        }
        let a = scanner_signals(&s, false, 0.0, true, false, 10.0, 1, 0.0);
        let b = scanner_signals(&s, false, 0.0, true, true, 10.0, 1, 0.0);
        assert!(a.buy.iter().all(|&x| !x), "V*C below the 13M table-A tier");
        assert!(b.buy.iter().any(|&x| x), "OpenInt above the 9M table-B tier");
        let sc = schema_scanner();
        assert_eq!(sc.iter().find(|q| q.key == "exitpct").unwrap().def, 10.0);
    }

    #[test]
    fn stock_count_and_pr_plots_have_no_signal_and_no_trade() {
        let s = closes(&[100.0, 101.0, 102.0, 103.0]);
        for id in ["ytath_stock_count_daily_scanner_of_stocks", "ytath_pr_plot_nse_midcap_nse100_price", "ytath_pr_plot_nse_smallcap100_nse100_price", "ytath_pr_plot_nse_smallcap100_nse_midcap"] {
            let v = variants().into_iter().find(|x| x.id == id).unwrap();
            let g = |_: &str| 0.0;
            let sg = (v.signals)(std::slice::from_ref(&s), &Env::empty(), &g);
            assert!(sg[0].buy.iter().all(|&x| !x) && sg[0].sell.iter().all(|&x| !x) && sg[0].short_leg.is_none());
            assert!(run(&s, &sg[0], 1).trades.is_empty());
        }
    }

    // ---- 15. long + short combo
    #[test]
    fn combo_short_covers_at_the_hard_stop_with_the_gap_rule_and_sizes_follow_the_allocation() {
        // a steady decline gives the short entry (Close < LowMA < CloseMA, RSI < 50, ADX > value), then a gap up through the 1 % hard stop
        let mut rows: Vec<(f32, f32, f32, f32)> = (0..30).map(|i| { let c = 100.0 + 2.0 * i as f32; (c - 0.5, c + 1.0, c - 1.0, c) }).collect();
        rows.extend((0..30).map(|i| { let c = 158.0 - 2.5 * i as f32; (c + 0.5, c + 1.0, c - 1.0, c) }));
        rows.push((90.0, 95.0, 88.0, 94.0));
        let s = series(&rows);
        let q = Combo {
            eqallo: 60.0, buysize: 5.0, sellsize: 10.0, dead_yes: false, penny: 0.0, turn_on: false, exit_pct: 18.0, losspct: 3.0, noofloss: 2.0, comm: 0.65, filter_on: false, filter_swing: false, pt_pct: 17.0, highma: 13,
            lowma: 10, closema: 5, adx_prd: 10, adx_value: 10.0, rsi_prd: 5, rsi_val: 50.0, flt_tf: 2, st_point: 0, flt_pd: 5, flt_fact: 4.0, fact_bear: 23.0, fact_bull: 44.0, hardsl: 1.0, ndays: 1.0, slpct: 1.0, low_n_pct: 3.0, hdays: 0.0,
        };
        let sg = combo_signals(&s, &q);
        let sl = sg.short_leg.as_ref().unwrap();
        let first = (0..s.bars.len()).find(|&i| sl.short[i]);
        assert!(first.is_some(), "short entry in a clean down-trend (abc cleared by the RSI cross)");
        let i = first.unwrap();
        assert_eq!(sl.short_price[i], s.bars[i].c);
        assert_eq!((sg.max_long, sg.max_short), (20, 10));
        assert_eq!(sg.size_pct, Some(60.0 * 5.0 / 100.0));
        assert_eq!(sl.size_pct, Some(40.0 * 10.0 / 100.0));
        // the gap-up bar covers: Open above the hard stop level -> Open + 0.25 %
        let last = s.bars.len() - 1;
        if sl.cover[last] {
            assert_eq!(sl.cover_price[last], (90.0f32 + (90.0f32 * 0.25) / 100.0).min(s.bars[last].h));
        }
        // the short is closed some time (hard stop / trailing stop / MA cover)
        assert!((i + 1..s.bars.len()).any(|j| sl.cover[j]));
        // portfolio: the short trade is a short, never overlaps the long cap logic
        let bt = run(&s, &sg, 20);
        assert!(bt.trades.iter().any(|t| t.short));
    }

    #[test]
    fn combo_long_needs_the_loss_streak_and_trades_only_long_when_the_filter_is_on_and_flat() {
        // noofloss = 0 removes the loss gate: a plain ATHC run (new high closes) is bought
        let c: Vec<f32> = (0..30).map(|i| 100.0 + i as f32).collect();
        let s = closes(&c);
        let mut q = Combo {
            eqallo: 60.0, buysize: 5.0, sellsize: 10.0, dead_yes: false, penny: 0.0, turn_on: false, exit_pct: 18.0, losspct: 3.0, noofloss: 0.0, comm: 0.65, filter_on: false, filter_swing: false, pt_pct: 17.0, highma: 13,
            lowma: 10, closema: 5, adx_prd: 10, adx_value: 10.0, rsi_prd: 5, rsi_val: 50.0, flt_tf: 2, st_point: 0, flt_pd: 5, flt_fact: 4.0, fact_bear: 23.0, fact_bull: 44.0, hardsl: 1.0, ndays: 1.0, slpct: 1.0, low_n_pct: 3.0, hdays: 0.0,
        };
        assert!(combo_signals(&s, &q).buy[1]);
        q.noofloss = 2.0;
        assert!(combo_signals(&s, &q).buy.iter().all(|&x| !x), "with two losses required and none seen: no long");
    }

    // ---- 16. ATHC-5 % range short
    #[test]
    fn short_enters_below_the_athc_after_a_swing_down_and_covers_on_a_new_athc_close() {
        // rise to 150, fall 30 % (the 10 % swing turns down), recover to 145 (inside 5 % of ATHC 150... 142.5 < 145 < 150) -> short at the close
        let mut c: Vec<f32> = (0..20).map(|i| 100.0 + 2.5 * i as f32).collect();
        c.push(150.0);
        c.extend([140.0, 130.0, 120.0, 110.0, 105.0, 120.0, 135.0, 145.0, 146.0, 140.0, 155.0, 156.0]);
        let s = closes(&c);
        let q = ShortQ { dead_yes: false, penny: 0.0, turn_on: false, pct_trail_version: false, buypct: 5.0, atr_prd: 5, atr_m: 5.0, trail_pct: 5.0, exit_pct: 10.0 };
        let sg = short_signals(&s, &q);
        let sl = sg.short_leg.as_ref().unwrap();
        let shorts: Vec<usize> = (0..c.len()).filter(|&i| sl.short[i]).collect();
        assert_eq!(shorts, vec![28], "first close within 5 % below ATHC (145 > 142.5) after the swing-down; Close < ATHC");
        assert_eq!(sl.short_price[28], 145.0);
        let covers: Vec<usize> = (0..c.len()).filter(|&i| sl.cover[i]).collect();
        assert!(covers.contains(&31), "close 155 > previous ATHC covers: {covers:?}");
        assert_eq!(sl.cover_price[31], 155.0);
        assert!(sg.buy.iter().all(|&b| !b) && sg.sell.iter().all(|&b| !b), "short only: no long signal");
        let bt = run(&s, &sg, 1);
        assert!(bt.trades.iter().all(|t| t.short));
        assert!(bt.trades[0].profit < 0.0, "short at 145, covered at 155");
    }

    #[test]
    fn short_percent_trail_version_covers_at_the_trailing_stop() {
        let mut c: Vec<f32> = (0..20).map(|i| 100.0 + 2.5 * i as f32).collect();
        c.push(150.0);
        c.extend([140.0, 130.0, 120.0, 110.0, 105.0, 120.0, 135.0, 145.0, 140.0, 136.0, 143.0]);
        let s = closes(&c);
        let q = ShortQ { dead_yes: false, penny: 0.0, turn_on: false, pct_trail_version: true, buypct: 5.0, atr_prd: 5, atr_m: 5.0, trail_pct: 5.0, exit_pct: 10.0 };
        let sg = short_signals(&s, &q);
        let sl = sg.short_leg.as_ref().unwrap();
        // short at 145 (bar 28); lowest close 136 (bar 30) -> stop 136 x 1.05 = 142.8; bar 31 closes 143 >= 142.8 -> cover
        assert!(sl.short[28]);
        assert!(sl.cover[31]);
        assert_eq!(sl.cover_price[31], 143.0);
    }
}
