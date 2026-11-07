//! ATH "ATHC: %Swing exit" sub-variants, batch d1 (22 AFLs): fixed-percent / ATR / 50 %-of-equity scale-in, VIX filters, hard stop-loss options,
//! 50 % profit booking, ADX filters, daily-chart entry with a weekly %swing exit, swing calculation points, turnover new / running buy.
//!
//! One `VariantDef` per sub-variant (see `ath_core::VariantDef`), registered through `ath_registry.rs`. Standard library only. Every rule is coded
//! from the local AFL copy (`ATH/afl_copy/<id>.afl`) in float32 like AmiBroker, AFL mistakes copied as written (decision D11). The helpers of the
//! %swing loop, the calendar compression / `TimeFrameExpand` and the index alignment (`Foreign`) are copies of the verified ones in `ath_loss2.rs`
//! (`apx`) and `ath_pctswing.rs`; the fixed-ATR scale-in logic of `ath_loss2.rs` (checked against the UNITECH sheet) is the template of the
//! fixed-percent and variable-ATR scale-in here. Verification notes: `ATH/v2/<id>/VERIFY.md`.
#![allow(dead_code)]
use crate::ath_core::*;

/// AmiBroker's Null (EMPTY) inside arrays.
const NULLV: f32 = -1.0e10;
fn is_null(x: f32) -> bool {
    x <= -9.0e9
}
/// Comparisons with a Null element are false (verified on the team's sheets).
fn gt(a: f32, b: f32) -> bool {
    !is_null(a) && !is_null(b) && a > b
}
fn ge(a: f32, b: f32) -> bool {
    !is_null(a) && !is_null(b) && a >= b
}
fn lt(a: f32, b: f32) -> bool {
    !is_null(a) && !is_null(b) && a < b
}
fn nadd(a: f32, b: f32) -> f32 {
    if is_null(a) || is_null(b) { NULLV } else { a + b }
}
fn nsub(a: f32, b: f32) -> f32 {
    if is_null(a) || is_null(b) { NULLV } else { a - b }
}
fn nmul(a: f32, b: f32) -> f32 {
    if is_null(a) || is_null(b) { NULLV } else { a * b }
}

// ---------------------------------------------------------------- the bar arrays and the filters every AFL here shares

struct Px {
    n: usize,
    date: Vec<i32>,
    c: Vec<f32>,
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    v: Vec<f32>,
    oi: Vec<f32>,
    upc: Vec<bool>,
    loc: Vec<bool>,
    uc: Vec<bool>,
    lc: Vec<bool>,
    /// Highest_Close = HHV(C, BarCount): highest close up to the bar
    hcl: Vec<f32>,
}

fn px(s: &Series) -> Px {
    let n = s.bars.len();
    let c = s.close();
    let o = s.open();
    let h = s.high();
    let l = s.low();
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
        c,
        o,
        h,
        l,
        hcl,
    }
}

/// DEAD STOCK BACKTEST (0 YES), Penny Stock Price and TURNOVER Filter (1 YES): the three switches every AFL here has.
#[derive(Clone, Copy)]
struct Cmn {
    dead_yes: bool,
    penny: f32,
    turn: bool,
}
fn cmn(g: G) -> Cmn {
    Cmn { dead_yes: g("deadstock").round() == 0.0, penny: g("pennyflt") as f32, turn: g("turnflt").round() == 1.0 }
}
impl Cmn {
    /// DEADSTOCKBUYSIG
    fn dead_buy(&self, i: usize, n: usize) -> bool {
        if self.dead_yes { i != n - 1 } else { i != 0 }
    }
    /// deadstocksellsig
    fn dead_sell(&self, i: usize, n: usize) -> bool {
        self.dead_yes && i == n - 1
    }
}

/// Volsig table compared with `V*C` (the `volsig` of the older AFLs: 30k / 50k / 1.8M / 2.6M / 6M / 13M).
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
/// Volsig table compared with `OpenInt` (30k ... 9M by year, the LATEST-AFL table).
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
/// `Turnover` with `(V*C) > volsig` (table A).
fn turn_a(p: &Px, on: bool) -> Vec<bool> {
    (0..p.n).map(|i| if on { p.v[i] * p.c[i] > volsig_a(p.date[i] / 10000) } else { true }).collect()
}
/// `Turnover` with `OpenInt > Volsig` (table B).
fn turn_b(p: &Px, on: bool) -> Vec<bool> {
    (0..p.n).map(|i| if on { p.oi[i] > volsig_b(p.date[i] / 10000) } else { true }).collect()
}

/// AmiBroker keeps a fill price inside the bar's low-high range.
fn clamp_prices(sg: &mut Signals, s: &Series) {
    for i in 0..s.bars.len() {
        let b = &s.bars[i];
        sg.buy_price[i] = sg.buy_price[i].min(b.h).max(b.l);
        sg.sell_price[i] = sg.sell_price[i].min(b.h).max(b.l);
    }
}

fn with_hard_stop(mut sg: Signals, hard: f32) -> Signals {
    if hard > 0.0 {
        sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
    }
    sg
}

/// The effective position size in percent of equity the runner uses (`pct`, 0 = 100 / MAXIMUM POSITION).
fn pct_eff(g: G) -> f32 {
    let m = g("max_positions").round().max(1.0);
    let p = g("pct");
    (if p <= 0.0 { 100.0 / m } else { p }) as f32
}

// ---------------------------------------------------------------- the %swing / supertrend loop (copy of the verified `apx::swing_loop`)

struct Sw {
    trend: Vec<i8>,
    tup: Vec<f32>,
    tdn: Vec<f32>,
}

/// The loop shared by every AFL here. `up` / `dn` are the initial bands, `upf` / `dnf` the values a band takes when the trend flips.
/// `bf_null`: the supertrend AFLs write `TrendUp[i-1] = Null` on a trend change (the %swing ones copy `TrendDown[i-1]`).
fn swing_gen(close: &[f32], mut up: Vec<f32>, mut dn: Vec<f32>, upf: &[f32], dnf: &[f32], bf_null: bool) -> Sw {
    let n = close.len();
    let mut trend = vec![1i8; n];
    let mut tup = vec![NULLV; n];
    let mut tdn = vec![NULLV; n];
    let mut chg = false;
    for i in 1..n {
        let mut t = 1i8;
        if close[i] > up[i - 1] {
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
            up[i] = upf[i];
        }
        if flagh {
            dn[i] = dnf[i];
        }
        if t == 1 {
            tup[i] = dn[i];
            if chg {
                tup[i - 1] = if bf_null { NULLV } else { tdn[i - 1] };
                chg = false;
            }
        } else if t == -1 {
            tdn[i] = up[i];
            if chg {
                tdn[i - 1] = if bf_null { NULLV } else { tup[i - 1] };
                chg = false;
            }
        }
    }
    Sw { trend, tup, tdn }
}

/// The plain %swing of the stock: EXIT = C*EXITPCT/100, Up = C + EXIT, Dn = C - EXIT.
fn swing_std(c: &[f32], exit_pct: f32) -> Sw {
    let n = c.len();
    let exit: Vec<f32> = c.iter().map(|&x| (x * exit_pct) / 100.0).collect();
    let up: Vec<f32> = (0..n).map(|i| c[i] + exit[i]).collect();
    let dn: Vec<f32> = (0..n).map(|i| c[i] - exit[i]).collect();
    swing_gen(c, up.clone(), dn.clone(), &up, &dn, false)
}

/// ATHCBuysig: 1 on a close above all earlier closes, 0 when the swing trend is down while it was 1, else carried.
fn athc_state(c: &[f32], hcl: &[f32], down: &dyn Fn(usize) -> bool) -> Vec<bool> {
    let n = c.len();
    let mut a = vec![false; n];
    for i in 1..n {
        a[i] = if gt(c[i], hcl[i - 1]) {
            true
        } else if down(i) && a[i - 1] {
            false
        } else {
            a[i - 1]
        };
    }
    a
}

/// `ATR(n)`: simple average of the first n true ranges, then Wilder (the variant checked against the UNITECH scale-in sheet).
fn atr_w(h: &[f32], l: &[f32], c: &[f32], n: usize) -> Vec<f32> {
    let len = h.len();
    let tr: Vec<f32> = (0..len).map(|i| if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect();
    let mut out = vec![NULLV; len];
    if n == 0 || len < n {
        return out;
    }
    out[n - 1] = tr[..n].iter().sum::<f32>() / n as f32;
    for i in n..len {
        out[i] = (out[i - 1] * (n as f32 - 1.0) + tr[i]) / n as f32;
    }
    out
}

/// `ADX(n)`: Wilder's ADX (+DM / -DM / TR smoothed with the simple-average seed, DX smoothed again). The seeds of AmiBroker's own ADX were not
/// checked; the first ~3 n bars of a symbol differ slightly from any other seed.
fn adx_w(h: &[f32], l: &[f32], c: &[f32], n: usize) -> Vec<f32> {
    let len = h.len();
    let mut out = vec![NULLV; len];
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
    let mut dx = vec![NULLV; len];
    for i in n - 1..len {
        if i >= n {
            sp = (sp * (nf - 1.0) + pdm[i]) / nf;
            sm = (sm * (nf - 1.0) + mdm[i]) / nf;
            st = (st * (nf - 1.0) + tr[i]) / nf;
        }
        if st > 0.0 {
            let (pdi, mdi) = (100.0 * sp / st, 100.0 * sm / st);
            dx[i] = if pdi + mdi > 0.0 { 100.0 * (pdi - mdi).abs() / (pdi + mdi) } else { 0.0 };
        } else {
            dx[i] = 0.0;
        }
    }
    let first = 2 * n - 2;
    out[first] = dx[n - 1..=first].iter().sum::<f32>() / nf;
    for i in first + 1..len {
        out[i] = (out[i - 1] * (nf - 1.0) + dx[i]) / nf;
    }
    out
}

// ---------------------------------------------------------------- calendar compression, TimeFrameExpand, Foreign alignment (copies of the verified `apx` helpers)

/// A price set on a bar-date grid (the stock's own bars, or an index aligned to them).
struct Al {
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    c: Vec<f32>,
}

fn al_of(s: &Series) -> Al {
    Al { o: s.open(), h: s.high(), l: s.low(), c: s.close() }
}

/// A foreign symbol's price arrays aligned to the current symbol's bar dates (`Foreign` / `SetForeign` fix-up: a missing bar repeats the
/// previous close; before the foreign symbol starts the value is Null).
fn align_idx(idx: &Series, dates: &[i32]) -> Al {
    let m = idx.bars.len();
    let mut j = 0usize;
    let mut a = Al { o: Vec::with_capacity(dates.len()), h: Vec::with_capacity(dates.len()), l: Vec::with_capacity(dates.len()), c: Vec::with_capacity(dates.len()) };
    for &d in dates {
        while j < m && idx.bars[j].date <= d {
            j += 1;
        }
        if j == 0 {
            a.o.push(NULLV);
            a.h.push(NULLV);
            a.l.push(NULLV);
            a.c.push(NULLV);
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

/// Key of the compressed bar a daily bar belongs to: calendar month (tf 0), calendar week, Monday based (tf 1) or the day itself (tf 2).
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

/// `TimeFrameSet(monthly / weekly / daily)`.
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

/// `TimeFrameExpand(x, tf)` (default mode): a compressed bar's value is shown on its last daily bar; the other daily bars of the period carry
/// the previous compressed bar's value.
fn expand(x: &[f32], k: &Comp, n: usize) -> Vec<f32> {
    let mut out = vec![NULLV; n];
    for g in 0..k.start.len() {
        for j in k.start[g]..k.end[g] {
            out[j] = if g > 0 { x[g - 1] } else { NULLV };
        }
        out[k.end[g]] = x[g];
    }
    out
}

/// The AFLs' supertrend block run on `al` (the stock itself or an aligned foreign symbol) in time frame `tf` (0 monthly, 1 weekly, 2 daily):
/// `MSTCAL` by `point` (0 mid point, 1 close, 2 pivot), `satr` = average of ATR(1) over `pd` compressed bars, Up = MSTCAL + fup * satr,
/// Dn = MSTCAL - fdn * satr. Returns (MTREND, TrendUp) expanded back to the daily bars.
fn st_run(dates: &[i32], al: &Al, tf: u8, point: u8, pd: usize, fup: f32, fdn: f32) -> (Vec<f32>, Vec<f32>) {
    let n = dates.len();
    let k = compress(dates, al, tf);
    let m = k.start.len();
    let cal: Vec<f32> = (0..m)
        .map(|g| {
            if is_null(k.h[g]) || is_null(k.l[g]) || is_null(k.c[g]) {
                NULLV
            } else {
                match point {
                    0 => (k.h[g] + k.l[g]) / 2.0,
                    2 => (k.h[g] + k.l[g] + k.c[g]) / 3.0,
                    _ => k.c[g],
                }
            }
        })
        .collect();
    let tr: Vec<f32> = (0..m)
        .map(|g| {
            if g == 0 {
                nsub(k.h[g], k.l[g])
            } else if is_null(k.h[g]) || is_null(k.l[g]) || is_null(k.c[g - 1]) {
                NULLV
            } else {
                k.h[g].max(k.c[g - 1]) - k.l[g].min(k.c[g - 1])
            }
        })
        .collect();
    let satr: Vec<f32> = (0..m)
        .map(|g| {
            if pd == 0 || g + 1 < pd {
                return NULLV;
            }
            let mut s = tr[g];
            for i in 1..pd {
                s = nadd(s, tr[g - i]);
            }
            if is_null(s) { NULLV } else { s / pd as f32 }
        })
        .collect();
    let up: Vec<f32> = (0..m).map(|g| nadd(cal[g], nmul(fup, satr[g]))).collect();
    let dn: Vec<f32> = (0..m).map(|g| nsub(cal[g], nmul(fdn, satr[g]))).collect();
    let sw = swing_gen(&k.c, up.clone(), dn.clone(), &up, &dn, true);
    let tf32: Vec<f32> = sw.trend.iter().map(|&t| t as f32).collect();
    (expand(&tf32, &k, n), expand(&sw.tup, &k, n))
}

/// ParamList index -> time-frame code (0 monthly, 1 weekly, 2 daily); `order` lists the AFL's items.
fn tf_code(order: &[u8], idx: f64) -> u8 {
    order[(idx.round().max(0.0) as usize).min(order.len() - 1)]
}
const TF_DWM: [u8; 3] = [2, 1, 0]; // DAILY|WEEKLY|MONTHLY
const TF_WDM: [u8; 3] = [1, 2, 0]; // WEEKLY|DAILY|MONTHLY
const TF_MWD: [u8; 3] = [0, 1, 2]; // MONTHLY|WEEKLY|DAILY

// ---------------------------------------------------------------- scale-in events -> Signals (as in ath_loss2::scalein_signals, checked on the UNITECH sheet)

struct Ev {
    /// 0 nothing, 1 new entry (`Buy = 1`), 2 scale-in (`Buy = sigScaleIn`)
    buy: Vec<u8>,
    /// the AFL's `po[i]`: size multiple of the order (0 for a first entry = one tranche)
    po: Vec<f32>,
    bp: Vec<f32>,
    sell: Vec<bool>,
    sp: Vec<f32>,
}

/// One buy per position on the bar of its first real tranche, one sell; later tranches go to the portfolio as scale-in orders of `po` units.
fn ev_to_signals(s: &Series, e: &Ev, entry_size: f32, unit: f32) -> Signals {
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
    let mut bp = e.bp.clone();
    let mut sp = e.sp.clone();
    for i in 0..n {
        bp[i] = bp[i].min(s.bars[i].h).max(s.bars[i].l);
        sp[i] = sp[i].min(s.bars[i].h).max(s.bars[i].l);
    }
    let mut sg = Signals::new(buy, sell, bp.clone(), sp);
    let mut open = false;
    let mut entry_mult = vec![1.0f32; n];
    for i in 0..n {
        if e.sell[i] && open {
            open = false;
        }
        if e.buy[i] > 0 && !open {
            open = true;
            // the first REAL order after a blocked first bar (virtual tranche) is `Buy = sigScaleIn` sized `po` tranches (AmiBroker opens the position with all of them:
            // HDFCBANK 2000-03-06 in the team's sheet = 5 tranches)
            if e.buy[i] == 2 && e.po[i] > 1.0 && e.po[i] <= 5.0 {
                entry_mult[i] = e.po[i] * unit;
            }
        } else if e.buy[i] == 2 && open {
            sg.scale.push(ScaleIn { bar: i, price: bp[i], mult: (if e.po[i] > 0.0 { e.po[i] } else { 1.0 }) * unit });
        }
    }
    sg.entry_size = entry_size;
    if entry_mult.iter().any(|&m| m != 1.0) {
        sg.entry_mult = Some(entry_mult);
    }
    sg
}

// ================================================================ 1. fixed-percent scale-in (3/6/9/12 %, or Scalein1..4)

#[derive(Clone, Copy)]
struct PctScale {
    cm: Cmn,
    exit_pct: f32,
    buf: f32,
    /// first level Prcnt after the first tranche (3, or Scalein1)
    p0: f32,
    /// the level offsets after the first: Prcnt + a, + b, + c (3,3,3 or Scalein2..4)
    a: f32,
    b: f32,
    c: f32,
}

fn pct_scale_events(s: &Series, q: &PctScale) -> Ev {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, q.exit_pct);
    let athc = athc_state(&p.c, &p.hcl, &|i| sw.trend[i] == -1);
    let turn = turn_a(&p, q.cm.turn);
    let (c, o, h) = (&p.c, &p.o, &p.h);
    let mut e = Ev { buy: vec![0; n], po: vec![0.0; n], bp: c.clone(), sell: vec![false; n], sp: c.clone() };
    let mut pos = vec![0f32; n];
    let (mut skip, mut abc, mut prcnt, mut bap) = (0f32, 0f32, 0f32, 0f32);
    let maxp = 5.0f32;
    let buf = q.buf;
    for i in 1..n {
        let pp = pos[i - 1];
        let master = c[i] > q.cm.penny && q.cm.dead_buy(i, n);
        let first_ok = turn[i] && !(p.upc[i] || p.uc[i]);
        let skip_ok = turn[i] && !p.upc[i];
        let other = !turn[i] || p.upc[i] || p.uc[i];
        // level k above the entry: (1 + (Prcnt + offset)/100) * BuyAtPrice
        let lv = |k: usize, prcnt: f32| -> f32 {
            let off = match k {
                0 => prcnt,
                1 => prcnt + q.a,
                2 => (prcnt + q.a) + q.b,
                _ => ((prcnt + q.a) + q.b) + q.c,
            };
            (1.0f32 + off / 100.0) * bap
        };
        let ob = (1.0f32 + buf / 100.0) * o[i];
        let (l0, l1, l2, l3) = (lv(0, prcnt), lv(1, prcnt), lv(2, prcnt), lv(3, prcnt));
        // number of tranches, if one of the scale-in branches 2..5 applies
        let m: usize = if ((pp > 0.0 && pp < maxp && h[i] > l0 && h[i] <= l1) || (pp == 4.0 && h[i] > l0)) && master {
            1
        } else if ((pp > 0.0 && pp < maxp && h[i] > l0 && h[i] > l1 && h[i] <= l2) || (pp == 3.0 && pp < maxp && h[i] > l0 && h[i] > l1)) && master {
            2
        } else if ((pp > 0.0 && pp < maxp && h[i] > l0 && h[i] > l1 && h[i] > l2 && h[i] <= l3) || (pp == 2.0 && pp < maxp && h[i] > l0 && h[i] > l1 && h[i] > l2)) && master {
            3
        } else if pp > 0.0 && pp < maxp && h[i] > l3 && master {
            4
        } else {
            0
        };
        if pp == 0.0 && (athc[i] || c[i] > p.hcl[i - 1]) && master {
            if other {
                prcnt = q.p0;
                bap = c[i];
                pos[i] = 1.0;
                skip += 1.0;
            } else if first_ok {
                e.buy[i] = 1;
                prcnt = q.p0;
                bap = c[i];
                pos[i] = 1.0;
                abc += 1.0;
            }
        } else if m > 0 {
            let levels = [l0, l1, l2, l3];
            if pp > 0.0 && !p.upc[i] {
                e.po[i] = skip + m as f32;
                e.buy[i] = 2;
                // j = how many of the crossed levels the open is already above (those fill at Open + buffer)
                let mut j = 0usize;
                while j < m && o[i] > levels[j] {
                    j += 1;
                }
                let mf = m as f32;
                e.bp[i] = if j == m {
                    ob
                } else {
                    // (ob * k) + level_j + ... + level_{m-1}, added left to right like the AFL's expressions; with skipped tranches the divisor is
                    // skip + m and the open-priced share count is skip + j (skip when j == 0); the AFL's 4-level branch with the open below
                    // every level adds a single `ob`, not `ob * skip` (copied as written)
                    let k = if skip > 0.0 { if m == 4 && j == 0 { 1.0 } else if j == 0 { skip } else { skip + j as f32 } } else { j as f32 };
                    let mut acc = ob * k;
                    for lvl in &levels[j..m] {
                        acc += *lvl;
                    }
                    acc / (if skip > 0.0 { skip + mf } else { mf })
                };
                pos[i] = pp + mf;
                if m < 4 {
                    prcnt += 3.0 * mf;
                    abc += 1.0;
                    skip = 0.0;
                } else {
                    prcnt = 0.0;
                    skip = 0.0;
                    abc += 1.0;
                }
                // (the AFL sets Prcnt = Skip = 0 after the 4-level branch and increments abc; the other branches reset skip only)
            } else {
                pos[i] = pp + m as f32;
                if m < 4 {
                    prcnt += 3.0 * m as f32;
                }
                skip += m as f32;
            }
        } else if skip > 0.0 && sw.trend[i] == 1 && skip_ok && master {
            // skipped scale-in
            e.po[i] = skip;
            e.buy[i] = 2;
            e.bp[i] = ob;
            pos[i] = pp;
            abc += 1.0;
            skip = 0.0;
        } else if pp > 0.0 && abc == 0.0 && skip > 0.0 && sw.trend[i] == -1 {
            // virtual sell
            pos[i] = 0.0;
            skip = 0.0;
            abc = 0.0;
        } else if (pp > 0.0 && abc > 0.0 && (sw.trend[i] == -1 || !athc[i]) && !(p.loc[i] || p.lc[i])) || q.cm.dead_sell(i, n) {
            e.sell[i] = true;
            e.sp[i] = if (p.loc[i - 1] || p.lc[i - 1]) && sw.trend[i - 1] == -1 { o[i] - (o[i] * buf) / 100.0 } else { c[i] };
            pos[i] = 0.0;
            prcnt = 0.0;
            skip = 0.0;
            abc = 0.0;
        } else {
            pos[i] = pp;
        }
    }
    e
}

fn pct_scale_signals(s: &Series, q: &PctScale) -> Signals {
    let e = pct_scale_events(s, q);
    ev_to_signals(s, &e, 0.2, 1.0)
}

fn pct_scale_params(g: G, scalein: bool) -> PctScale {
    if scalein {
        PctScale { cm: cmn(g), exit_pct: g("exitpct") as f32, buf: g("buffer_prcnt") as f32, p0: g("scalein1") as f32, a: g("scalein2") as f32, b: g("scalein3") as f32, c: g("scalein4") as f32 }
    } else {
        PctScale { cm: cmn(g), exit_pct: g("exitpct") as f32, buf: g("buffer_prcnt") as f32, p0: 3.0, a: 3.0, b: 3.0, c: 3.0 }
    }
}
fn sig_pct_fixed(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let q = pct_scale_params(g, false);
    series.iter().map(|s| pct_scale_signals(s, &q)).collect()
}
fn sig_pct_param(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let q = pct_scale_params(g, true);
    series.iter().map(|s| pct_scale_signals(s, &q)).collect()
}

// ================================================================ 2. variable ATR scale-in

fn atr_var_signals(s: &Series, cm: Cmn, exit_pct: f32, atr_prd: usize, mult: f32, buf: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, exit_pct);
    let athc = athc_state(&p.c, &p.hcl, &|i| sw.trend[i] == -1);
    let turn = turn_a(&p, cm.turn);
    let myatr = atr_w(&p.h, &p.l, &p.c, atr_prd);
    let (c, o, h) = (&p.c, &p.o, &p.h);
    let mut e = Ev { buy: vec![0; n], po: vec![0.0; n], bp: c.clone(), sell: vec![false; n], sp: c.clone() };
    let mut up_atr = vec![NULLV; n];
    let (mut position, mut skip, mut abc) = (0f32, 0f32, 0f32);
    let maxp = 5.0f32;
    for i in 1..n {
        let master = c[i] > cm.penny && cm.dead_buy(i, n);
        let first_ok = turn[i] && !(p.upc[i] || p.uc[i]);
        let skip_ok = turn[i] && !p.upc[i];
        let other = !turn[i] || p.upc[i] || p.uc[i];
        let ob = o[i] + (o[i] * buf) / 100.0;
        let step = nmul(myatr[i], mult);
        if position == 0.0 && (athc[i] || c[i] > p.hcl[i - 1]) && master && gt(myatr[i], 0.0) {
            if other {
                up_atr[i] = nadd(c[i], step);
                position = 1.0;
                skip += 1.0;
            } else if first_ok {
                e.buy[i] = 1;
                up_atr[i] = nadd(c[i], step);
                position = 1.0;
                abc += 1.0;
            }
        } else if position > 0.0 && position < maxp && gt(h[i], up_atr[i - 1]) && master {
            if position > 0.0 && !p.upc[i] {
                let u = up_atr[i - 1];
                e.po[i] = skip + 1.0;
                e.buy[i] = 2;
                let price = if o[i] > u {
                    ob
                } else if skip > 0.0 {
                    ((ob * skip) + u) / (skip + 1.0)
                } else {
                    u
                };
                e.bp[i] = price;
                up_atr[i] = nadd(price, step);
                position += 1.0;
                skip = 0.0;
                abc += 1.0;
            } else {
                up_atr[i] = nadd(up_atr[i - 1], step);
                position += 1.0;
                skip += 1.0;
            }
        } else if skip > 0.0 && sw.trend[i] == 1 && skip_ok && master {
            e.po[i] = skip;
            e.buy[i] = 2;
            e.bp[i] = ob;
            up_atr[i] = up_atr[i - 1];
            skip = 0.0;
            abc += 1.0;
        } else if position > 0.0 && abc == 0.0 && skip > 0.0 && sw.trend[i] == -1 {
            position = 0.0;
            skip = 0.0;
            abc = 0.0;
        } else if (position > 0.0 && abc > 0.0 && (sw.trend[i] == -1 || !athc[i]) && !(p.loc[i] || p.lc[i])) || cm.dead_sell(i, n) {
            e.sell[i] = true;
            e.sp[i] = if (p.loc[i - 1] || p.lc[i - 1]) && sw.trend[i - 1] == -1 { o[i] - (o[i] * buf) / 100.0 } else { c[i] };
            position = 0.0;
            up_atr[i] = NULLV;
            skip = 0.0;
            abc = 0.0;
        } else {
            up_atr[i] = up_atr[i - 1];
        }
    }
    ev_to_signals(s, &e, 0.2, 1.0)
}
fn sig_atr_var(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    series.iter().map(|s| atr_var_signals(s, cm, g("exitpct") as f32, g("atr_prd").max(0.0) as usize, g("myatr1") as f32, g("buffer_prcnt") as f32)).collect()
}

// ================================================================ 3. VIX filters

/// India VIX close aligned to the stock's bar dates (`Foreign("VIX","Close")`); Null where VIX has no data yet, empty when the series is missing.
fn vix_close(env: &Env, dates: &[i32]) -> Vec<f32> {
    match env.get("VIX") {
        Some(v) => align_idx(v, dates).c,
        None => vec![NULLV; dates.len()],
    }
}

fn ma_null(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![NULLV; a.len()];
    if n == 0 {
        return out;
    }
    for i in 0..a.len() {
        if i + 1 < n {
            continue;
        }
        let w = &a[i + 1 - n..=i];
        if w.iter().any(|&x| is_null(x)) {
            continue;
        }
        out[i] = (w.iter().map(|&x| x as f64).sum::<f64>() / n as f64) as f32;
    }
    out
}

/// `Running_Trade` of the "average India VIX" AFL: set on a new all-time-high close, cleared when the %swing trend turns down.
fn running_trade(c: &[f32], hcl: &[f32], trend: &[i8]) -> Vec<bool> {
    let n = c.len();
    let mut rt = vec![false; n];
    for i in 1..n {
        rt[i] = if c[i] > hcl[i - 1] {
            true
        } else if rt[i - 1] && trend[i] == -1 {
            false
        } else {
            rt[i - 1]
        };
    }
    rt
}

/// AFL `ATHC & %Swing with average India VIX filter`: no ExRem in the AFL (the Position state keeps Buy and Sell alternating).
fn avg_vix_signals(s: &Series, env: &Env, cm: Cmn, exit_pct: f32, prds: usize, vix_value: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, exit_pct);
    let turn = turn_a(&p, cm.turn);
    let rt = running_trade(&p.c, &p.hcl, &sw.trend);
    let avg = ma_null(&vix_close(env, &p.date), prds);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut position = false;
    for i in 1..n {
        if !position && (p.c[i] > p.hcl[i - 1] || rt[i]) && lt(avg[i], vix_value) && !p.upc[i] && cm.dead_buy(i, n) && p.c[i] > cm.penny && turn[i] {
            buy[i] = true;
            position = true;
        }
        if (position && (sw.trend[i] == -1 || !rt[i]) && !p.loc[i]) || cm.dead_sell(i, n) {
            sell[i] = true;
            position = false;
        }
    }
    let mut sg = Signals::new(buy, sell, p.c.clone(), p.c.clone());
    clamp_prices(&mut sg, s);
    sg
}
fn sig_avg_vix(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    series.iter().map(|s| avg_vix_signals(s, env, cm, g("exitpct") as f32, g("avg_vix_prds").max(0.0) as usize, g("vix_value") as f32)).collect()
}

/// AFL `ATHC Buy & %Swing exit with VIX filter`: today's VIX against the limit, with the AFL's `Running_Trade` quirks.
fn vix_filter_signals(s: &Series, env: &Env, cm: Cmn, exit_pct: f32, vix_value: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, exit_pct);
    let turn = turn_a(&p, cm.turn);
    let vix = vix_close(env, &p.date);
    let buysig = athc_state(&p.c, &p.hcl, &|i| sw.trend[i] == -1);
    let mut rt = vec![false; n];
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut position = false;
    for i in 1..n {
        let new_high = p.c[i] > p.hcl[i - 1];
        let blocked = p.upc[i] || !cm.dead_buy(i, n) || lt(p.c[i], cm.penny) || !turn[i];
        rt[i] = if new_high && gt(vix[i], vix_value) {
            true
        } else if new_high && lt(vix[i], vix_value) && blocked {
            true
        } else if rt[i - 1] && sw.trend[i] == -1 {
            false
        } else {
            rt[i - 1]
        };
        if !position && ((new_high && lt(vix[i], vix_value)) || (rt[i] && lt(vix[i], vix_value))) && !p.upc[i] && cm.dead_buy(i, n) && p.c[i] > cm.penny && turn[i] {
            buy[i] = true;
            position = true;
        }
        if (position && (sw.trend[i] == -1 || !buysig[i]) && !p.loc[i]) || cm.dead_sell(i, n) {
            sell[i] = true;
            position = false;
        }
    }
    let mut sg = Signals::new(buy, sell, p.c.clone(), p.c.clone());
    clamp_prices(&mut sg, s);
    sg
}
fn sig_vix_filter(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    series.iter().map(|s| vix_filter_signals(s, env, cm, g("exitpct") as f32, g("vix_value") as f32)).collect()
}

/// 30 identical O/H/L/C bars (the AFL's `deadstock1`; the first 30 bars have a Null reference and are false).
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

/// AFL `ATHC Entry and %Swing exit when VIX ...`: buy while the supertrend of the (aligned) VIX is down (MTREND == -1).
#[derive(Clone, Copy)]
struct WhenVix {
    cm: Cmn,
    exit_pct: f32,
    tf: u8,
    pd: usize,
    fact: f32,
    point: u8,
}
fn when_vix_signals(s: &Series, env: &Env, q: &WhenVix) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, q.exit_pct);
    let turn = turn_a(&p, q.cm.turn);
    let rt = running_trade(&p.c, &p.hcl, &sw.trend);
    let dead1 = dead30(s);
    let mtrend = match env.get("VIX") {
        Some(v) => st_run(&p.date, &align_idx(v, &p.date), q.tf, q.point, q.pd, q.fact, q.fact).0,
        None => vec![NULLV; n],
    };
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut position = false;
    for i in 1..n {
        let other = q.cm.dead_buy(i, n) && p.c[i] > q.cm.penny && turn[i] && !p.upc[i];
        if !position && (p.c[i] > p.hcl[i - 1] || rt[i]) && mtrend[i] == -1.0 && other && !dead1[i] {
            buy[i] = true;
            position = true;
        }
        if (position && (sw.trend[i] == -1 || !rt[i]) && !p.loc[i]) || q.cm.dead_sell(i, n) || dead1[i] {
            sell[i] = true;
            position = false;
        }
    }
    let mut sg = Signals::new(buy, sell, p.c.clone(), p.c.clone());
    clamp_prices(&mut sg, s);
    sg
}
fn sig_when_vix(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
    let q = WhenVix { cm: cmn(g), exit_pct: g("exitpct") as f32, tf: tf_code(&TF_MWD, g("flttf")), pd: g("fltpd").max(0.0) as usize, fact: g("fltfact") as f32, point: g("stflt").round().clamp(0.0, 2.0) as u8 };
    series.iter().map(|s| when_vix_signals(s, env, &q)).collect()
}

// ================================================================ 4. hard stop-loss options (%, ATR, lowest close)

fn hard_sl_signals(s: &Series, cm: Cmn, exit_pct: f32, ver: u8, atr_prd: usize, atr_m: f32, days: usize, hard_pct: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, exit_pct);
    let athc = athc_state(&p.c, &p.hcl, &|i| sw.trend[i] == -1);
    let turn = turn_b(&p, cm.turn);
    let myatr = atr_w(&p.h, &p.l, &p.c, atr_prd);
    // LowestSince(BarCount, C, DAYS_SL): lowest close of the last DAYS_SL bars including the bar (BarCount is true on every bar)
    let sl1: Vec<f32> = (0..n).map(|i| p.c[i + 1 - days.min(i + 1)..=i].iter().copied().fold(f32::MAX, f32::min)).collect();
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut sell_price = p.c.clone();
    let (mut position, mut skipbuy) = (false, false);
    let (mut hard_sl, mut atr_sl, mut sl) = (0f32, 0f32, 0f32);
    for i in 1..n {
        let filt = !p.uc[i] && turn[i] && p.c[i] > cm.penny && cm.dead_buy(i, n);
        // the stop levels a buy on this bar would set (entry price is the Close); the same three-branch skeleton for the three versions
        let lvl = match ver {
            0 => (p.c[i] - (p.c[i] * hard_pct) / 100.0, atr_sl, sl),
            1 => (hard_sl, nsub(p.c[i], nmul(myatr[i], atr_m)), sl),
            _ => (hard_sl, atr_sl, sl1[i]),
        };
        if !position && p.c[i] > p.hcl[i - 1] && !skipbuy {
            if filt {
                buy[i] = true;
                (hard_sl, atr_sl, sl) = lvl;
                position = true;
            } else {
                skipbuy = true;
            }
        } else if skipbuy && athc[i] && filt {
            buy[i] = true;
            (hard_sl, atr_sl, sl) = lvl;
            position = true;
            skipbuy = false;
        } else if position {
            let stop = match ver {
                0 => p.c[i] < hard_sl,
                1 => lt(p.c[i], atr_sl),
                _ => p.c[i] < sl,
            };
            if (!p.lc[i] && (sw.trend[i] == -1 || !athc[i] || stop)) || cm.dead_sell(i, n) {
                sell[i] = true;
                sell_price[i] = p.c[i];
                position = false;
                if ver == 1 {
                    atr_sl = 0.0;
                } else {
                    hard_sl = 0.0;
                }
            }
        }
    }
    let mut sg = Signals::new(buy, sell, p.c.clone(), sell_price);
    clamp_prices(&mut sg, s);
    sg
}
fn sig_hard_sl(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    series.iter().map(|s| hard_sl_signals(s, cm, g("exitpct") as f32, g("ver").round().clamp(0.0, 2.0) as u8, g("atr_prd").max(0.0) as usize, g("atr_sl_m") as f32, g("days_sl").max(1.0) as usize, g("hardslpct") as f32)).collect()
}
fn sig_hard_sl2(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    series.iter().map(|s| hard_sl_signals(s, cm, g("exitpct") as f32, g("ver").round().clamp(0.0, 1.0) as u8, g("atr_prd").max(0.0) as usize, g("atr_sl_m") as f32, 1, g("hardslpct") as f32)).collect()
}

// ================================================================ 5. entry with scale-in on every new all-time-high close

fn scale_every_high_signals(s: &Series, cm: Cmn, exit_pct: f32, big_price: f32, buf: f32, ver: u8, pct: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, exit_pct);
    let turn = turn_b(&p, cm.turn);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut bp, mut sp) = (p.c.clone(), p.c.clone());
    let mut scale: Vec<ScaleIn> = Vec::new();
    let (mut position, mut skipsell) = (0f32, false);
    let max_position = 100.0f32;
    for i in 1..n {
        let filt = turn[i] && !p.uc[i] && p.c[i] > cm.penny && cm.dead_buy(i, n);
        if position > 0.0 && sw.trend[i] == -1 && !skipsell {
            if !p.lc[i] || cm.dead_sell(i, n) {
                sell[i] = true;
                sp[i] = p.c[i];
                position = 0.0;
                skipsell = false;
            } else {
                skipsell = true;
            }
        } else if position > 0.0 && ((!p.loc[i] && skipsell) || cm.dead_sell(i, n)) {
            sell[i] = true;
            sp[i] = p.o[i] - (p.o[i] * buf) / 100.0;
            position = 0.0;
            skipsell = false;
        } else if position == 0.0 && p.c[i] > p.hcl[i - 1] && p.c[i] < big_price && filt {
            buy[i] = true;
            bp[i] = p.c[i];
            position = 1.0;
        } else if position > 0.0 && position < max_position && p.c[i] > p.hcl[i - 1] && p.c[i] < big_price && filt {
            let price = p.c[i].min(p.h[i]).max(p.l[i]);
            scale.push(ScaleIn { bar: i, price, mult: 0.0 });
            position += 1.0;
        }
    }
    // SetPositionSize: "1st" = 1 % of equity for every tranche; "2nd" = 5 % for the first, 1 % for the scale-ins (the AFL ignores POS. SIZE)
    let first = if ver == 1 { 1.0 } else { 5.0 };
    for sc in scale.iter_mut() {
        sc.mult = 1.0 / first;
    }
    let mut sg = Signals::new(buy, sell, bp, sp);
    clamp_prices(&mut sg, s);
    sg.scale = scale;
    sg.entry_size = first / pct;
    sg
}
fn sig_scale_every_high(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    let pct = pct_eff(g);
    series.iter().map(|s| scale_every_high_signals(s, cm, g("exitpct") as f32, g("bigpricefilter") as f32, g("buffer_prcnt") as f32, g("ver").round().clamp(0.0, 1.0) as u8, pct)).collect()
}

// ================================================================ 6. 50 % profit booking at a profit target

fn profit_booking_signals(s: &Series, cm: Cmn, exit_pct: f32, profit_pct: f32, sell_pct: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    // Up = Highest_Close (instead of C + EXIT); Dn = C - EXIT; the flipped bands are C +/- EXIT
    let exit: Vec<f32> = p.c.iter().map(|&x| (x * exit_pct) / 100.0).collect();
    let up0: Vec<f32> = p.hcl.clone();
    let dn0: Vec<f32> = (0..n).map(|i| p.c[i] - exit[i]).collect();
    let upf: Vec<f32> = (0..n).map(|i| p.c[i] + exit[i]).collect();
    let sw = swing_gen(&p.c, up0, dn0.clone(), &upf, &dn0, false);
    let athc = athc_state(&p.c, &p.hcl, &|i| sw.trend[i] == -1);
    let turn = turn_b(&p, cm.turn);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut sell_price = p.c.clone();
    let mut out: Vec<ScaleOutOrder> = Vec::new();
    let (mut position, mut skipbuy) = (0u8, false);
    let mut pt = NULLV;
    for i in 1..n {
        let filt = !p.uc[i] && turn[i] && p.c[i] > cm.penny && cm.dead_buy(i, n);
        if position == 0 && (p.c[i] > p.hcl[i - 1] || athc[i]) && !skipbuy {
            if filt {
                buy[i] = true;
                pt = (1.0f32 + profit_pct * 0.01) * p.c[i];
                position = 2;
            } else {
                skipbuy = true;
            }
        } else if skipbuy && athc[i] && filt {
            buy[i] = true;
            pt = (1.0f32 + profit_pct * 0.01) * p.c[i];
            position = 2;
            skipbuy = false;
        } else if position > 0 {
            if position == 2 && ge(p.h[i], pt) {
                out.push(ScaleOutOrder { bar: i, price: pt.min(p.h[i]).max(p.l[i]), pct: sell_pct });
                position = 1;
                pt = NULLV;
            } else if (sw.trend[i] == -1 || !athc[i]) && !p.lc[i] || cm.dead_sell(i, n) {
                sell[i] = true;
                sell_price[i] = p.c[i];
                position = 0;
                pt = NULLV;
            }
        }
    }
    let mut sg = Signals::new(buy, sell, p.c.clone(), sell_price);
    clamp_prices(&mut sg, s);
    sg.scale_out = out;
    sg
}
fn sig_profit_booking(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    series.iter().map(|s| profit_booking_signals(s, cm, g("exitpct") as f32, g("profit_pct") as f32, g("sharesselling_pct") as f32)).collect()
}

// ================================================================ 7. 50 % of equity scale-in below the stop line

#[derive(Clone, Copy)]
struct Half {
    cm: Cmn,
    ver: u8,
    exit_pct: f32,
    tf: u8,
    pd: usize,
    fact: f32,
    point: u8,
    prcnt: f32,
}
fn half_scale_signals(s: &Series, q: &Half) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, q.exit_pct);
    let athc = athc_state(&p.c, &p.hcl, &|i| sw.trend[i] == -1);
    let turn = turn_b(&p, q.cm.turn);
    // supertrend (Up = MSTCAL + 1 * satr, Dn = MSTCAL - FACT * satr) in the chosen time frame
    let (mtrend, tup_st) = st_run(&p.date, &al_of(s), q.tf, q.point, q.pd, 1.0, q.fact);
    let st_mode = q.ver == 1;
    let tup: &Vec<f32> = if st_mode { &tup_st } else { &sw.tup };
    let level: Vec<f32> = (0..n).map(|i| if is_null(tup[i]) { NULLV } else { tup[i] + (tup[i] * q.prcnt) / 100.0 }).collect();
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut scale: Vec<ScaleIn> = Vec::new();
    let (mut position, mut skipbuy) = (0f32, false);
    let mut buy_at = 0f32;
    let max_position = 2.0f32;
    for i in 1..n {
        let filt = !p.uc[i] && turn[i] && p.c[i] > q.cm.penny && q.cm.dead_buy(i, n);
        let trend_up = if st_mode { mtrend[i] == 1.0 } else { true };
        let trend_down = if st_mode { mtrend[i] == -1.0 } else { sw.trend[i] == -1 };
        if position == 0.0 && (!st_mode || trend_up) && p.c[i] > p.hcl[i - 1] && !skipbuy {
            if filt {
                buy[i] = true;
                buy_at = p.c[i];
                position = 1.0;
            } else {
                skipbuy = true;
            }
        } else if position == 0.0 && skipbuy && filt {
            buy[i] = true;
            buy_at = p.c[i];
            position = 1.0;
            skipbuy = false;
        } else if position > 0.0 && position < max_position && (!st_mode || trend_up) && lt(p.c[i], level[i]) && p.c[i] < buy_at && filt {
            scale.push(ScaleIn { bar: i, price: p.c[i].min(p.h[i]).max(p.l[i]), mult: 1.0 });
            position += 1.0;
        } else if position > 0.0 {
            if (!p.lc[i] && (trend_down || !athc[i])) || q.cm.dead_sell(i, n) {
                sell[i] = true;
                position = 0.0;
                buy_at = 0.0;
            }
        }
    }
    let mut sg = Signals::new(buy, sell, p.c.clone(), p.c.clone());
    clamp_prices(&mut sg, s);
    sg.scale = scale;
    sg.entry_size = 0.5; // SIZE1 = SIZE / 2: both tranches are half of the position size
    sg
}
fn sig_half_scale(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let q = Half { cm: cmn(g), ver: g("ver").round().clamp(0.0, 1.0) as u8, exit_pct: g("exitpct") as f32, tf: tf_code(&TF_DWM, g("flttf")), pd: g("fltpd").max(0.0) as usize, fact: g("fltfact") as f32, point: g("stflt").round().clamp(0.0, 2.0) as u8, prcnt: g("prcnt") as f32 };
    series.iter().map(|s| half_scale_signals(s, &q)).collect()
}

// ================================================================ 8. ADX filters

/// ATHC buy only while ADX is above the limit; %swing / ATHC-state exit.
fn adx_entry_signals(s: &Series, cm: Cmn, exit_pct: f32, adx_prd: usize, adx_value: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, exit_pct);
    let athc = athc_state(&p.c, &p.hcl, &|i| sw.trend[i] == -1);
    let turn = turn_b(&p, cm.turn);
    let adx = adx_w(&p.h, &p.l, &p.c, adx_prd);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut position = false;
    for i in 1..n {
        let filt = !p.uc[i] && turn[i] && p.c[i] > cm.penny && cm.dead_buy(i, n);
        if !position && p.c[i] > p.hcl[i - 1] && gt(adx[i], adx_value) && filt {
            buy[i] = true;
            position = true;
        } else if position && ((!p.lc[i] && (sw.trend[i] == -1 || !athc[i])) || cm.dead_sell(i, n)) {
            sell[i] = true;
            position = false;
        }
    }
    let mut sg = Signals::new(buy, sell, p.c.clone(), p.c.clone());
    clamp_prices(&mut sg, s);
    sg
}
fn sig_adx_entry(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    series.iter().map(|s| adx_entry_signals(s, cm, g("exitpct") as f32, g("adx_prd").max(0.0) as usize, g("adx_value") as f32)).collect()
}

/// ATHC buy with ADX above the buy value; exit when ADX falls below the sell value (after being above) or the %swing turns down.
fn adx_exit_signals(s: &Series, cm: Cmn, exit_pct: f32, adx_prd: usize, buy_v: f32, sell_v: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, exit_pct);
    let turn = turn_b(&p, cm.turn);
    let adx = adx_w(&p.h, &p.l, &p.c, adx_prd);
    let exit_cond = |i: usize| (lt(adx[i], sell_v) && gt(adx[i - 1], sell_v)) || sw.trend[i] == -1;
    let mut bs = vec![false; n];
    for i in 1..n {
        bs[i] = if p.c[i] > p.hcl[i - 1] && gt(adx[i], buy_v) {
            true
        } else if exit_cond(i) && bs[i - 1] {
            false
        } else {
            bs[i - 1]
        };
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut position = false;
    let mut skipsell = false;
    for i in 1..n {
        let filt = !p.uc[i] && turn[i] && p.c[i] > cm.penny && cm.dead_buy(i, n);
        if !position && (p.c[i] > p.hcl[i - 1] || bs[i]) && gt(adx[i], buy_v) && filt {
            buy[i] = true;
            position = true;
        } else if (position && !skipsell && exit_cond(i)) || (position && cm.dead_sell(i, n)) {
            if !p.lc[i] || cm.dead_sell(i, n) {
                sell[i] = true;
                position = false;
                skipsell = false;
            } else {
                skipsell = true;
            }
        } else if (skipsell && !p.lc[i]) || cm.dead_sell(i, n) {
            sell[i] = true;
            position = false;
            skipsell = false;
        }
    }
    let mut sg = Signals::new(buy, sell, p.c.clone(), p.c.clone());
    clamp_prices(&mut sg, s);
    sg
}
fn sig_adx_exit(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    series.iter().map(|s| adx_exit_signals(s, cm, g("exitpct") as f32, g("adx_prd").max(0.0) as usize, g("adx_buy_value") as f32, g("adx_sell_value") as f32)).collect()
}

// ================================================================ 9. ATR profit target / ATR hard stop, exit at the Close

fn atr_profit_signals(s: &Series, cm: Cmn, exit_pct: f32, ver: u8) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, exit_pct);
    let turn = turn_b(&p, cm.turn);
    // Optimize("ATR Prds", 5, ...), Optimize("ATR Profit Target Multiplier", 10, ...), Optimize("ATR Hard SL Multiplier", 3, ...): defaults
    let (atr_prd, pt_m, sl_m) = (5usize, 10.0f32, 3.0f32);
    let myatr = atr_w(&p.h, &p.l, &p.c, atr_prd);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut position, mut new_ath, mut skipbuy, mut skipsell) = (false, false, false, false);
    let (mut pt, mut fixsl) = (NULLV, NULLV);
    for i in 1..n {
        let filt = !p.uc[i] && turn[i] && p.c[i] > cm.penny && cm.dead_buy(i, n);
        if new_ath && sw.trend[i] == -1 {
            new_ath = false;
        }
        let hit_pt = ge(p.h[i], pt);
        if ver == 0 {
            if !position && p.c[i] > p.hcl[i - 1] && sw.trend[i] == 1 && !skipbuy && !new_ath {
                if filt {
                    buy[i] = true;
                    pt = nadd(p.c[i], nmul(myatr[i], pt_m));
                    position = true;
                } else {
                    skipbuy = true;
                }
            } else if skipbuy && sw.trend[i] == -1 {
                skipbuy = false;
            } else if skipbuy && sw.trend[i] == 1 && filt {
                buy[i] = true;
                pt = nadd(p.c[i], nmul(myatr[i], pt_m));
                position = true;
                skipbuy = false;
            } else if position {
                if ((sw.trend[i] == -1 || hit_pt) && !skipsell) || cm.dead_sell(i, n) {
                    if hit_pt {
                        new_ath = true;
                    } else if sw.trend[i] == -1 {
                        new_ath = false;
                    }
                    if !p.lc[i] || cm.dead_sell(i, n) {
                        sell[i] = true;
                        position = false;
                        pt = NULLV;
                    } else {
                        skipsell = true;
                    }
                } else if skipsell && (!p.lc[i] || cm.dead_sell(i, n)) {
                    sell[i] = true;
                    position = false;
                    skipsell = false;
                    pt = NULLV;
                }
            }
        } else {
            if !position && p.c[i] > p.hcl[i - 1] && !skipbuy && !new_ath {
                if filt {
                    buy[i] = true;
                    pt = nadd(p.c[i], nmul(myatr[i], pt_m));
                    fixsl = nsub(p.c[i], nmul(myatr[i], sl_m));
                    position = true;
                } else {
                    skipbuy = true;
                }
            } else if skipbuy && filt {
                buy[i] = true;
                pt = nadd(p.c[i], nmul(myatr[i], pt_m));
                fixsl = nsub(p.c[i], nmul(myatr[i], sl_m));
                position = true;
                skipbuy = false;
            } else if position && !skipsell {
                let hit_sl = !is_null(fixsl) && p.c[i] <= fixsl;
                if hit_pt || hit_sl || cm.dead_sell(i, n) {
                    if hit_pt {
                        new_ath = true;
                    } else if hit_sl {
                        new_ath = false;
                    }
                    if !p.lc[i] || cm.dead_sell(i, n) {
                        sell[i] = true;
                        position = false;
                        pt = NULLV;
                        fixsl = NULLV;
                    } else {
                        skipsell = true;
                    }
                }
            } else if skipsell && (!p.lc[i] || cm.dead_sell(i, n)) {
                sell[i] = true;
                position = false;
                skipsell = false;
                pt = NULLV;
                fixsl = NULLV;
            }
        }
    }
    let mut sg = Signals::new(buy, sell, p.c.clone(), p.c.clone());
    clamp_prices(&mut sg, s);
    sg
}
fn sig_atr_profit(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    series.iter().map(|s| atr_profit_signals(s, cm, g("exitpct") as f32, g("ver").round().clamp(0.0, 1.0) as u8)).collect()
}

// ================================================================ 10. new / running buy (turnover versions), daily new-high buy, swing calculation point

/// AFL `ATHC Exit with %Swing new buy entry daily`: buy only on a close above all earlier closes (V*C turnover table A), %swing exit.
fn new_buy_daily_signals(s: &Series, cm: Cmn, exit_pct: f32, tstart: usize) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, exit_pct);
    let turn = turn_a(&p, cm.turn);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = i >= 1 && p.c[i] > p.hcl[i - 1] && i >= tstart && !p.upc[i] && cm.dead_buy(i, n) && p.c[i] > cm.penny && turn[i];
        sell[i] = (sw.trend[i] == -1 && !p.loc[i]) || cm.dead_sell(i, n);
    }
    finish_exrem(s, &p, buy, sell)
}
/// Buy = ExRem(Buy, Sell); Sell = ExRem(Sell, Buy); BuyPrice / SellPrice = the Close of the signal bar.
fn finish_exrem(s: &Series, p: &Px, buy: Vec<bool>, sell: Vec<bool>) -> Signals {
    let buy = exrem(&buy, &sell);
    let sell = exrem(&sell, &buy);
    let mut sg = Signals::new(buy, sell, p.c.clone(), p.c.clone());
    clamp_prices(&mut sg, s);
    sg
}
fn sig_new_buy_daily(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    let hard = g("hardstop") as f32;
    series.iter().map(|s| with_hard_stop(new_buy_daily_signals(s, cm, g("exitpct") as f32, g("tstart").max(0.0) as usize), hard)).collect()
}

/// `buysig`: 1 on a close above all earlier closes, 0 when the %swing trend is down while it was 1, else carried (the same as the ATHC state).
fn turnover_new_buy_signals(s: &Series, cm: Cmn, exit_pct: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, exit_pct);
    let turn = turn_a(&p, cm.turn);
    let buysig = athc_state(&p.c, &p.hcl, &|i| sw.trend[i] == -1);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let new_high = i >= 1 && p.c[i] > p.hcl[i - 1];
        let b = if cm.turn { new_high && turn[i] && !p.upc[i] } else { buysig[i] && !p.upc[i] && turn[i] };
        buy[i] = b && cm.dead_buy(i, n) && p.c[i] > cm.penny;
        sell[i] = (sw.trend[i] == -1 && !p.loc[i]) || cm.dead_sell(i, n);
    }
    finish_exrem(s, &p, buy, sell)
}
fn sig_turnover_new_buy(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    let hard = g("hardstop") as f32;
    // EXITPCT = Optimize("EXIT %", 10, 14, 14, 1): the default 10 is what a normal run uses
    series.iter().map(|s| with_hard_stop(turnover_new_buy_signals(s, cm, 10.0), hard)).collect()
}

fn turnover_running_signals(s: &Series, cm: Cmn, exit_pct: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let sw = swing_std(&p.c, exit_pct);
    let turn = turn_a(&p, cm.turn);
    let buysig = athc_state(&p.c, &p.hcl, &|i| sw.trend[i] == -1);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let new_high = i >= 1 && p.c[i] > p.hcl[i - 1];
        buy[i] = (buysig[i] || new_high) && !p.upc[i] && cm.dead_buy(i, n) && p.c[i] > cm.penny && turn[i];
        sell[i] = ((sw.trend[i] == -1 || !buysig[i]) && !p.loc[i]) || cm.dead_sell(i, n);
    }
    finish_exrem(s, &p, buy, sell)
}
fn sig_turnover_running(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    let hard = g("hardstop") as f32;
    series.iter().map(|s| with_hard_stop(turnover_running_signals(s, cm, g("exitpct") as f32), hard)).collect()
}

/// `STCAL` of the SWING CALCULATION POINT list (CLOSE | MIDD POINT(H+L/2) | HIGH | PIVOT(H+L+C/3) | MP CL PIVOT AVG | 3DAYS MA | 5DAYS MA).
/// The second item's text differs from the AFL's test string ("MIDD" vs "MID"), so it keeps the initial STCAL = MP = (H+L)/2, the same value.
fn stcal(p: &Px, point: u8) -> Vec<f32> {
    let n = p.n;
    let ma = |k: usize| -> Vec<f32> {
        (0..n).map(|i| if i + 1 < k { NULLV } else { (p.c[i + 1 - k..=i].iter().map(|&x| x as f64).sum::<f64>() / k as f64) as f32 }).collect()
    };
    match point {
        0 => p.c.clone(),
        1 => (0..n).map(|i| (p.h[i] + p.l[i]) / 2.0).collect(),
        2 => p.h.clone(),
        3 => (0..n).map(|i| (p.h[i] + p.l[i] + p.c[i]) / 3.0).collect(),
        4 => (0..n).map(|i| (((p.h[i] + p.l[i]) / 2.0) + ((p.h[i] + p.l[i] + p.c[i]) / 3.0) + p.c[i]) / 3.0).collect(),
        5 => ma(3),
        _ => ma(5),
    }
}
fn midpoint_signals(s: &Series, cm: Cmn, exit_pct: f32, point: u8) -> Signals {
    let p = px(s);
    let n = p.n;
    let st = stcal(&p, point);
    let exit: Vec<f32> = st.iter().map(|&x| if is_null(x) { NULLV } else { (x * exit_pct) / 100.0 }).collect();
    let dexit: Vec<f32> = st.iter().map(|&x| if is_null(x) { NULLV } else { (x * 1.0) / 100.0 }).collect();
    let up0: Vec<f32> = (0..n).map(|i| nadd(p.c[i], dexit[i])).collect();
    let dn0: Vec<f32> = (0..n).map(|i| nsub(st[i], exit[i])).collect();
    let sw = swing_gen(&p.c, up0.clone(), dn0.clone(), &up0, &dn0, true);
    let turn = turn_a(&p, cm.turn);
    let athc = athc_state(&p.c, &p.hcl, &|i| sw.trend[i] == -1);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let new_high = i >= 1 && p.c[i] > p.hcl[i - 1];
        buy[i] = (athc[i] || new_high) && !p.upc[i] && cm.dead_buy(i, n) && p.c[i] > cm.penny && turn[i];
        sell[i] = ((sw.trend[i] == -1 || !athc[i]) && !p.loc[i]) || cm.dead_sell(i, n);
    }
    finish_exrem(s, &p, buy, sell)
}
fn sig_midpoint(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    let hard = g("hardstop") as f32;
    let point = g("swingcal").round().clamp(0.0, 6.0) as u8;
    series.iter().map(|s| with_hard_stop(midpoint_signals(s, cm, g("exitpct") as f32, point), hard)).collect()
}

// ================================================================ 11. daily-chart entry with a weekly %swing exit

fn daily_chart_signals(s: &Series, cm: Cmn, tf: u8, exit_pct: f32, buf: f32) -> Signals {
    let p = px(s);
    let n = p.n;
    let turn = turn_b(&p, cm.turn);
    let al = al_of(s);
    let k = compress(&p.date, &al, tf);
    let sw = swing_std(&k.c, exit_pct);
    let tr: Vec<f32> = sw.trend.iter().map(|&t| t as f32).collect();
    let trend = expand(&tr, &k, n);
    let myclose = expand(&k.c, &k, n);
    let athc = athc_state(&p.c, &p.hcl, &|i| trend[i] == -1.0);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut sell_price = p.c.clone();
    let (mut position, mut skipsell) = (false, false);
    for i in 1..n {
        let filt = turn[i] && !p.uc[i] && p.c[i] > cm.penny && cm.dead_buy(i, n);
        if !position && (athc[i] || p.c[i] > p.hcl[i - 1]) && filt {
            buy[i] = true;
            position = true;
        } else if position && trend[i] == -1.0 && myclose[i] != myclose[i - 1] && !skipsell {
            if !p.lc[i] || cm.dead_sell(i, n) {
                sell[i] = true;
                position = false;
                skipsell = false;
            } else {
                skipsell = true;
            }
        } else if position && ((!p.loc[i] && skipsell) || cm.dead_sell(i, n)) {
            sell[i] = true;
            sell_price[i] = p.o[i] - (p.o[i] * buf) / 100.0;
            position = false;
            skipsell = false;
        }
    }
    let mut sg = Signals::new(buy, sell, p.c.clone(), sell_price);
    clamp_prices(&mut sg, s);
    sg
}
fn sig_daily_chart(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = cmn(g);
    let tf = tf_code(&TF_WDM, g("flt_tf"));
    series.iter().map(|s| daily_chart_signals(s, cm, tf, g("exitpct") as f32, g("buffer_prcnt") as f32)).collect()
}

/// BuyPrice rule of the weekly-ATHC AFL: Open if Open == High, else Open + buffer if High is above it, else High; (the equality case leaves the default Close).
fn weekly_buy_price(o: f32, h: f32, c: f32, buf: f32) -> f32 {
    let lim = o + (o * buf) / 100.0;
    if o == h {
        o
    } else if h > lim {
        lim
    } else if h < lim {
        h
    } else {
        c
    }
}
fn weekly_sell_price(o: f32, l: f32, c: f32, buf: f32) -> f32 {
    let lim = o - (o * buf) / 100.0;
    if o == l {
        o
    } else if l < lim {
        lim
    } else if l > lim {
        l
    } else {
        c
    }
}

#[derive(Clone, Copy)]
struct Weekly {
    cm: Cmn,
    ver: u8,
    buy_buf: f32,
    sell_buf: f32,
    exit_pct: f32,
    tf: u8,
    pd: usize,
    fact: f32,
    point: u8,
}
fn weekly_athc_signals(s: &Series, q: &Weekly) -> Signals {
    let p = px(s);
    let n = p.n;
    let al = al_of(s);
    // Highest_Close = HHV(C, BarCount) and MyClose on the weekly bars, expanded to the daily bars
    let wk = compress(&p.date, &al, 1);
    let mut whcl = vec![0f32; wk.c.len()];
    for g in 0..wk.c.len() {
        whcl[g] = if g == 0 || wk.c[g] > whcl[g - 1] { wk.c[g] } else { whcl[g - 1] };
    }
    let hcl = expand(&whcl, &wk, n);
    let myclose = expand(&wk.c, &wk, n);
    let sw = swing_std(&p.c, q.exit_pct);
    let (mtrend, _tu) = st_run(&p.date, &al, q.tf, q.point, q.pd, 1.0, q.fact);
    let turn = turn_b(&p, q.cm.turn);
    let athc = athc_state(&myclose, &hcl, &|i| sw.trend[i] == -1);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut bp, mut sp) = (p.c.clone(), p.c.clone());
    let (mut position, mut skipbuy, mut skipsell) = (false, false, false);
    let st_mode = q.ver == 1;
    for i in 1..n {
        let filt = turn[i] && p.c[i] > q.cm.penny && q.cm.dead_buy(i, n);
        let newhigh = gt(myclose[i], hcl[i - 1]);
        let ds = q.cm.dead_sell(i, n);
        if !position && newhigh && (if st_mode { mtrend[i] == 1.0 } else { sw.trend[i] == 1 }) && !skipbuy && filt {
            if !p.uc[i] {
                buy[i] = true;
                position = true;
            } else {
                skipbuy = true;
            }
        } else if !position && athc[i] && (!st_mode || mtrend[i] == 1.0) && skipbuy && filt && (st_mode == false || !p.upc[i]) {
            if st_mode || !p.upc[i] {
                buy[i] = true;
                bp[i] = weekly_buy_price(p.o[i], p.h[i], p.c[i], q.buy_buf);
                position = true;
                skipbuy = false;
            }
        } else if !position && sw.trend[i] == -1 {
            skipbuy = false;
        } else if position && !skipsell && (if st_mode { mtrend[i] == -1.0 || ds } else { sw.trend[i] == -1 || !athc[i] || ds }) {
            if !p.lc[i] || ds {
                sell[i] = true;
                position = false;
            } else {
                skipsell = true;
            }
        } else if (position && !p.loc[i] && skipsell) || (position && ds) {
            sell[i] = true;
            position = false;
            skipsell = false;
            sp[i] = weekly_sell_price(p.o[i], p.l[i], p.c[i], q.sell_buf);
        }
    }
    let mut sg = Signals::new(buy, sell, bp, sp);
    clamp_prices(&mut sg, s);
    sg
}
fn sig_weekly_athc(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let q = Weekly {
        cm: cmn(g),
        ver: g("ver").round().clamp(0.0, 1.0) as u8,
        buy_buf: g("buffer_pct") as f32,
        sell_buf: g("bufferpct") as f32,
        exit_pct: g("exitpct") as f32,
        tf: tf_code(&TF_DWM, g("flttf")),
        pd: g("fltpd").max(0.0) as usize,
        fact: g("fltfact") as f32,
        point: g("stflt").round().clamp(0.0, 2.0) as u8,
    };
    series.iter().map(|s| weekly_athc_signals(s, &q)).collect()
}

// ================================================================ 12. small swing entry / big swing exit (stock selection)

fn stock_selection_signals(s: &Series, q: &StockSel) -> Signals {
    let p = px(s);
    let n = p.n;
    let big = swing_std(&p.c, q.exit_pct);
    // small swing: its bands use EXIT_1 = C * Exit_Pct / 100, but the flipped bands copy the BIG `exit[i]` (an AFL mistake kept as written)
    let exit1: Vec<f32> = p.c.iter().map(|&x| (x * q.small_pct) / 100.0).collect();
    let exit_big: Vec<f32> = p.c.iter().map(|&x| (x * q.exit_pct) / 100.0).collect();
    let up1: Vec<f32> = (0..n).map(|i| p.c[i] + exit1[i]).collect();
    let dn1: Vec<f32> = (0..n).map(|i| p.c[i] - exit1[i]).collect();
    let upf: Vec<f32> = (0..n).map(|i| p.c[i] + exit_big[i]).collect();
    let dnf: Vec<f32> = (0..n).map(|i| p.c[i] - exit_big[i]).collect();
    let small = swing_gen(&p.c, up1, dn1, &upf, &dnf, false);
    let (mtrend, _tu) = st_run(&p.date, &al_of(s), q.tf, q.point, q.pd, 1.0, q.fact);
    let turn = turn_b(&p, q.cm.turn);
    let athc = athc_state(&p.c, &p.hcl, &|i| big.trend[i] == -1);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut position = false;
    let st_mode = q.ver == 1;
    for i in 1..n {
        let filt = !p.uc[i] && turn[i] && p.c[i] > q.cm.penny && q.cm.dead_buy(i, n);
        let up = if st_mode { mtrend[i] == 1.0 } else { small.trend[i] == 1 };
        let down = if st_mode { mtrend[i] == -1.0 } else { small.trend[i] == -1 };
        if !position && athc[i] && up && filt {
            buy[i] = true;
            position = true;
        } else if (position && !p.lc[i] && (down || !athc[i])) || q.cm.dead_sell(i, n) {
            sell[i] = true;
            position = false;
        }
    }
    let mut sg = Signals::new(buy, sell, p.c.clone(), p.c.clone());
    clamp_prices(&mut sg, s);
    sg
}
#[derive(Clone, Copy)]
struct StockSel {
    cm: Cmn,
    ver: u8,
    exit_pct: f32,
    small_pct: f32,
    tf: u8,
    pd: usize,
    fact: f32,
    point: u8,
}
fn sig_stock_sel(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let q = StockSel {
        cm: cmn(g),
        ver: g("ver").round().clamp(0.0, 1.0) as u8,
        exit_pct: g("exitpct") as f32,
        small_pct: g("exit_pct") as f32,
        tf: tf_code(&TF_DWM, g("flttf")),
        pd: g("fltpd").max(0.0) as usize,
        fact: g("fltfact") as f32,
        point: g("stflt").round().clamp(0.0, 2.0) as u8,
    };
    series.iter().map(|s| stock_selection_signals(s, &q)).collect()
}

// ================================================================ schemas

fn base() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
    ]
}
fn with(extra: Vec<P>) -> Vec<P> {
    let mut v = base();
    v.extend(extra);
    v
}
fn ver_p(label: &'static str, max: f64) -> P {
    p("ver", label, 0.0, 0.0, max, 1.0)
}
fn st_params(tf_label: &'static str, tf_def: f64, pd_def: f64, pd_min: f64, pd_max: f64, pd_step: f64, fact_def: f64, fact_min: f64, fact_max: f64) -> Vec<P> {
    vec![
        p("flttf", tf_label, tf_def, 0.0, 2.0, 1.0),
        p("fltpd", "ST FILTER PERIOD", pd_def, pd_min, pd_max, pd_step),
        p("fltfact", "ST FILTER FACTOR", fact_def, fact_min, fact_max, 1.0),
        p("stflt", "ST CALCULATION POINT (0 MID POINT(H+L/2), 1 CLOSE, 2 PIVOT(H+L+C/3))", 0.0, 0.0, 2.0, 1.0),
    ]
}

fn schema_daily_chart() -> Vec<P> {
    with(vec![
        p("flt_tf", "FILTER TIMEFRAME (0 WEEKLY, 1 DAILY, 2 MONTHLY)", 0.0, 0.0, 2.0, 1.0),
        p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0),
        p("buffer_prcnt", "Selling Buffer %", 0.25, 0.0, 10.0, 0.05),
    ])
}
fn schema_pct_fixed() -> Vec<P> {
    with(vec![p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0), p("buffer_prcnt", "Buffer %", 0.25, 0.0, 2.0, 0.001)])
}
fn schema_pct_param() -> Vec<P> {
    let mut v = schema_pct_fixed();
    v.extend([
        p("scalein1", "Scale 1", 1.0, 1.0, 100.0, 1.0),
        p("scalein2", "Scale 2", 1.0, 1.0, 100.0, 1.0),
        p("scalein3", "Scale 3", 1.0, 1.0, 100.0, 1.0),
        p("scalein4", "Scale 4", 1.0, 1.0, 100.0, 1.0),
    ]);
    v
}
fn schema_atr_var() -> Vec<P> {
    with(vec![
        p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0),
        p("atr_prd", "ATR Prds", 14.0, 5.0, 100.0, 1.0),
        p("myatr1", "ATR Multiplier", 1.0, 0.0, 5.0, 0.025),
        p("buffer_prcnt", "Buffer %", 0.25, 0.0, 2.0, 0.001),
    ])
}
fn schema_avg_vix() -> Vec<P> {
    with(vec![p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0), p("avg_vix_prds", "Avg Vix Periods", 10.0, 1.0, 100.0, 1.0), p("vix_value", "Vix Value", 25.0, 5.0, 80.0, 1.0)])
}
fn schema_vix_filter() -> Vec<P> {
    with(vec![p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0), p("vix_value", "Vix Value", 25.0, 5.0, 80.0, 1.0)])
}
fn schema_when_vix() -> Vec<P> {
    let mut v = base();
    v.extend(st_params("FILTER TIMEFRAME (0 MONTHLY, 1 WEEKLY, 2 DAILY)", 0.0, 5.0, 0.0, 20.0, 1.0, 1.0, 0.0, 20.0));
    v.push(p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0));
    v
}
fn schema_stock_sel() -> Vec<P> {
    let mut v = base();
    v.push(ver_p("Exit by %Swing OR ST (0 %Swing Exit, 1 SuperTrend Exit)", 1.0));
    v.push(p("exitpct", "Big Swing EXIT %", 21.0, 1.0, 40.0, 1.0));
    v.push(p("exit_pct", "Small Swing EXIT %", 5.0, 1.0, 40.0, 1.0));
    v.extend(st_params("FILTER TIMEFRAME (0 DAILY, 1 WEEKLY, 2 MONTHLY)", 0.0, 10.0, 0.0, 20.0, 1.0, 4.0, 1.0, 5.0));
    v
}
fn schema_hard_sl() -> Vec<P> {
    with(vec![
        ver_p("Exit by HARD SL% OR ATR HARD SL (0 % Based HARD SL, 1 ATR Based HARD SL, 2 5 DAYS LOWEST CLOSE)", 2.0),
        p("atr_prd", "ATR Prds", 5.0, 1.0, 100.0, 1.0),
        p("atr_sl_m", "ATR Hard SL Multiplier", 3.0, 1.0, 3.0, 0.5),
        p("days_sl", "DAYS SL", 1.0, 1.0, 10.0, 1.0),
        p("hardslpct", "HARD SL%", 2.0, 1.0, 7.0, 1.0),
        p("exitpct", "Swing EXIT %", 21.0, 2.0, 22.0, 1.0),
    ])
}
fn schema_hard_sl2() -> Vec<P> {
    with(vec![
        ver_p("Exit by HARD SL% OR ATR HARD SL (0 % Based HARD SL, 1 ATR Based HARD SL)", 1.0),
        p("atr_prd", "ATR Prds", 5.0, 1.0, 100.0, 1.0),
        p("atr_sl_m", "ATR Hard SL Multiplier", 3.0, 1.0, 6.0, 0.5),
        p("hardslpct", "HARD SL%", 2.0, 2.0, 12.0, 1.0),
        p("exitpct", "Swing EXIT %", 21.0, 1.0, 40.0, 1.0),
    ])
}
fn schema_scale_every_high() -> Vec<P> {
    with(vec![
        p("exitpct", "EXIT %", 5.0, 5.0, 25.0, 1.0),
        p("bigpricefilter", "Big Price Filter", 10000.0, 0.0, 100000.0, 1.0),
        p("buffer_prcnt", "Buffer %", 0.25, 0.0, 2.0, 0.001),
        p("ver", "Equity Allocation Version (0 2nd, 1 1st)", 1.0, 0.0, 1.0, 1.0),
    ])
}
fn schema_profit_booking() -> Vec<P> {
    with(vec![
        p("profit_pct", "Profit Booking %", 10.0, 0.0, 30.0, 0.1),
        p("sharesselling_pct", "Shares Selling %", 50.0, 0.0, 99.0, 1.0),
        p("exitpct", "EXIT %", 10.0, 1.0, 20.0, 1.0),
    ])
}
fn schema_half_scale() -> Vec<P> {
    let mut v = base();
    v.push(ver_p("Avg Price Version (0 %Swing Exit, 1 ST Exit)", 1.0));
    v.push(p("exitpct", "Swing EXIT %", 21.0, 5.0, 25.0, 1.0));
    v.extend(st_params("FILTER TIMEFRAME (0 DAILY, 1 WEEKLY, 2 MONTHLY)", 0.0, 10.0, 5.0, 25.0, 5.0, 4.0, 0.0, 20.0));
    v.push(p("prcnt", "% from SL", 5.0, 1.0, 10.0, 1.0));
    v
}
fn schema_adx_entry() -> Vec<P> {
    with(vec![p("exitpct", "Swing EXIT %", 21.0, 5.0, 25.0, 1.0), p("adx_prd", "ADX Period", 14.0, 1.0, 100.0, 1.0), p("adx_value", "ADX Value", 20.0, 20.0, 80.0, 10.0)])
}
fn schema_adx_exit() -> Vec<P> {
    with(vec![
        p("adx_prd", "ADX Period", 14.0, 1.0, 100.0, 1.0),
        p("adx_buy_value", "ADX Buy Value", 20.0, 1.0, 100.0, 1.0),
        p("adx_sell_value", "ADX Sell Value", 20.0, 1.0, 100.0, 1.0),
        p("exitpct", "Swing EXIT %", 21.0, 1.0, 40.0, 1.0),
    ])
}
fn schema_atr_profit() -> Vec<P> {
    with(vec![ver_p("Exit Versions (0 %Swing, 1 ATR HARD SL)", 1.0), p("exitpct", "Swing EXIT %", 21.0, 1.0, 40.0, 1.0)])
}
fn schema_new_buy_daily() -> Vec<P> {
    vec![
        p("tstart", "TRADING START BAR", 1.0, 1.0, 500.0, 1.0),
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop percent loss)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; V*C > Volsig table)", 0.0, 0.0, 1.0, 1.0),
        p("exitpct", "EXIT %", 10.0, 5.0, 25.0, 1.0),
    ]
}
fn schema_turnover_new_buy() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop percent loss)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO = running buy, 1 YES = new-high bar only; V*C > Volsig table)", 0.0, 0.0, 1.0, 1.0),
    ]
}
fn schema_turnover_running() -> Vec<P> {
    let mut v = schema_turnover_new_buy();
    v.push(p("exitpct", "EXIT %", 10.0, 5.0, 25.0, 1.0));
    v
}
fn schema_midpoint() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop percent loss)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; V*C > Volsig table)", 0.0, 0.0, 1.0, 1.0),
        p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0),
        p("swingcal", "SWING CALCULATION POINT (0 CLOSE, 1 MIDD POINT(H+L/2), 2 HIGH, 3 PIVOT(H+L+C/3), 4 MP CL PIVOT AVG, 5 3DAYS MA, 6 5DAYS MA)", 0.0, 0.0, 6.0, 1.0),
    ]
}
fn schema_weekly_athc() -> Vec<P> {
    let mut v = base();
    v.push(ver_p("Exit by %Swing OR ST (0 %Swing Exit, 1 SuperTrend Exit)", 1.0));
    v.push(p("buffer_pct", "Buying Buffer %", 0.25, 0.0, 10.0, 0.05));
    v.push(p("bufferpct", "Selling Buffer %", 0.25, 0.0, 10.0, 0.05));
    v.push(p("exitpct", "Swing EXIT %", 10.0, 5.0, 25.0, 1.0));
    v.extend(st_params("FILTER TIMEFRAME (0 DAILY, 1 WEEKLY, 2 MONTHLY)", 0.0, 10.0, 0.0, 20.0, 1.0, 4.0, 0.0, 20.0));
    v
}

// ================================================================ the variants

const NSE500: Data = Data::Cash22 { watchlist: "NSE 500 C&P.tls" };

pub fn variants() -> Vec<VariantDef> {
    vec![
        VariantDef { id: "ytath_close_entry_on_daily_chart_exit_below", data: NSE500, schema: schema_daily_chart, signals: sig_daily_chart },
        VariantDef { id: "ytath_and_pctswing_with_fixed_percentage", data: NSE500, schema: schema_pct_param, signals: sig_pct_param },
        VariantDef { id: "ytath_and_pctswing_with_fixed_percentage_2", data: NSE500, schema: schema_pct_fixed, signals: sig_pct_fixed },
        VariantDef { id: "ytath_and_pctswing_with_variable_atr_scale", data: NSE500, schema: schema_atr_var, signals: sig_atr_var },
        VariantDef { id: "ytath_and_pctswing_with_average_india_vix", data: NSE500, schema: schema_avg_vix, signals: sig_avg_vix },
        VariantDef { id: "ytath_buy_and_pctswing_exit_with_vix_filter", data: NSE500, schema: schema_vix_filter, signals: sig_vix_filter },
        VariantDef { id: "ytath_based_stock_selection_and_pctswing_or", data: NSE500, schema: schema_stock_sel, signals: sig_stock_sel },
        VariantDef { id: "ytath_entry_and_pctswing_exit_with_pct_hard", data: NSE500, schema: schema_hard_sl, signals: sig_hard_sl },
        VariantDef { id: "ytath_entry_and_pctswing_exit_with_pct_hard_2", data: NSE500, schema: schema_hard_sl2, signals: sig_hard_sl2 },
        VariantDef { id: "ytath_entry_and_pctswing_exit_with_scale_in", data: NSE500, schema: schema_scale_every_high, signals: sig_scale_every_high },
        VariantDef { id: "ytath_entry_50pct_exit_at_profit_booking", data: NSE500, schema: schema_profit_booking, signals: sig_profit_booking },
        VariantDef { id: "ytath_entry_with_50pct_of_equity_scale_in", data: NSE500, schema: schema_half_scale, signals: sig_half_scale },
        VariantDef { id: "ytath_entry_with_adx_and_pctswing_exit", data: NSE500, schema: schema_adx_entry, signals: sig_adx_entry },
        VariantDef { id: "ytath_entry_with_adx_value_exit_below_adx", data: NSE500, schema: schema_adx_exit, signals: sig_adx_exit },
        VariantDef { id: "ytath_entry_with_atr_profit_booking_exit", data: NSE500, schema: schema_atr_profit, signals: sig_atr_profit },
        VariantDef { id: "ytath_entry_and_pctswing_exit_when_vix", data: NSE500, schema: schema_when_vix, signals: sig_when_vix },
        VariantDef { id: "ytath_exit_with_pctswing_new_buy_entry_daily", data: NSE500, schema: schema_new_buy_daily, signals: sig_new_buy_daily },
        VariantDef { id: "ytath_exit_with_pctswing_turnover_new_buy", data: NSE500, schema: schema_turnover_new_buy, signals: sig_turnover_new_buy },
        VariantDef { id: "ytath_exit_with_pctswing_turnover_running", data: NSE500, schema: schema_turnover_running, signals: sig_turnover_running },
        VariantDef { id: "ytath_entry_pctswing_exit_on_close_midpoint", data: NSE500, schema: schema_midpoint, signals: sig_midpoint },
        VariantDef { id: "ytath_weekly_athc_entry_daily_pctswing_or", data: NSE500, schema: schema_weekly_athc, signals: sig_weekly_athc },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// weekday dates from Monday 2020-01-06 (every 5th bar starts a new week)
    fn date_of(i: usize) -> i32 {
        let day = 6 + (i / 5) * 7 + (i % 5);
        // January 2020 then February 2020 (29 days) then March ... : enough for the short tests
        let (mut m, mut d) = (1, day as i32);
        let len = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        while d > len[m - 1] {
            d -= len[m - 1];
            m += 1;
        }
        20200000 + (m as i32) * 100 + d
    }
    /// bars from (open, high, low, close)
    fn mk(rows: &[(f32, f32, f32, f32)]) -> Series {
        let bars = rows.iter().enumerate().map(|(i, &(o, h, l, c))| Bar { date: date_of(i), c, o, h, l, v: 1.0e6, oi: 1.0e8, aux1: 1.0, aux2: 0.0 }).collect();
        Series { name: "T".into(), bars }
    }
    /// a normal bar around the close
    fn nb(c: f32) -> (f32, f32, f32, f32) {
        (c - 0.3, c + 1.0, c - 1.0, c)
    }
    fn cm0() -> Cmn {
        Cmn { dead_yes: false, penny: 0.0, turn: false }
    }
    fn gfn(pairs: &'static [(&'static str, f64)]) -> impl Fn(&str) -> f64 {
        move |k: &str| pairs.iter().find(|(n, _)| *n == k).map(|x| x.1).unwrap_or(0.0)
    }

    #[test]
    fn fixed_percent_scale_in_buys_a_tranche_at_the_first_level_and_the_exit_sells_at_the_close() {
        // bar 1 is a new all-time-high close (first tranche at 101, level 1 = 101 * 1.03); bar 2 trades through it; bar 3 breaks the 10 % swing line
        let mut rows = vec![nb(100.0), nb(101.0)];
        rows.push((102.0, 105.0, 101.5, 104.0));
        rows.push((90.0, 91.0, 84.0, 85.0));
        let s = mk(&rows);
        let q = PctScale { cm: cm0(), exit_pct: 10.0, buf: 0.25, p0: 3.0, a: 3.0, b: 3.0, c: 3.0 };
        let e = pct_scale_events(&s, &q);
        assert_eq!(e.buy[1], 1);
        assert_eq!(e.buy[2], 2);
        assert_eq!(e.po[2], 1.0);
        let level = (1.0f32 + 3.0f32 / 100.0) * 101.0;
        assert!((e.bp[2] - level).abs() < 1e-4, "{} vs {}", e.bp[2], level);
        assert!(e.sell[3]);
        assert_eq!(e.sp[3], 85.0);
        let sg = pct_scale_signals(&s, &q);
        assert_eq!(sg.scale.len(), 1);
        assert_eq!(sg.entry_size, 0.2);
        assert!(sg.buy[1] && sg.sell[3]);
    }

    #[test]
    fn fixed_percent_gap_up_fills_at_open_plus_buffer_and_two_levels_in_one_bar_buy_two_tranches() {
        // open above level 1 and level 2: both tranches at Open + 0.25 %
        let mut rows = vec![nb(100.0), nb(101.0)];
        rows.push((108.0, 109.0, 107.5, 108.5)); // levels 104.03 and 107.06: open above both
        rows.push((90.0, 91.0, 84.0, 85.0));
        let s = mk(&rows);
        let q = PctScale { cm: cm0(), exit_pct: 10.0, buf: 0.25, p0: 3.0, a: 3.0, b: 3.0, c: 3.0 };
        let e = pct_scale_events(&s, &q);
        assert_eq!(e.buy[2], 2);
        assert_eq!(e.po[2], 2.0);
        let ob = (1.0f32 + 0.25f32 / 100.0) * 108.0;
        assert!((e.bp[2] - ob).abs() < 1e-3);
    }

    #[test]
    fn the_first_real_order_after_a_blocked_first_bar_opens_the_position_with_po_tranches() {
        // AFL: `Buy = sigScaleIn` on a flat symbol after a virtual tranche (Skip = 1), po = skip + k tranches: one entry sized po x the tranche (team sheet: HDFCBANK 2000-03-06 = 5 tranches)
        let s = mk(&[nb(100.0), nb(100.0), nb(100.0), nb(100.0)]);
        let n = s.bars.len();
        let mut e = Ev { buy: vec![0; n], po: vec![0.0; n], bp: vec![100.0; n], sell: vec![false; n], sp: vec![100.0; n] };
        e.buy[1] = 2;
        e.po[1] = 5.0;
        let sg = ev_to_signals(&s, &e, 0.2, 1.0);
        assert!(sg.buy[1] && sg.scale.is_empty());
        assert_eq!(sg.entry_mult.as_ref().map(|m| m[1]), Some(5.0));
        // a normal first entry (Buy = 1) and po outside 2..5 keep one tranche
        e.buy[1] = 1;
        assert!(ev_to_signals(&s, &e, 0.2, 1.0).entry_mult.is_none());
        e.buy[1] = 2;
        e.po[1] = 7.0;
        assert!(ev_to_signals(&s, &e, 0.2, 1.0).entry_mult.is_none());
    }

    #[test]
    fn variable_atr_scale_in_recomputes_the_level_from_the_fill_price() {
        let mut rows = vec![nb(100.0), nb(101.0)];
        rows.push((102.0, 104.5, 101.0, 104.0));
        let s = mk(&rows);
        let sg = atr_var_signals(&s, cm0(), 10.0, 1, 1.0, 0.25);
        // ATR(1) on bar 1 = true range = max(H, prevC) - min(L, prevC) = 102 - 100 = 2 -> level = 101 + 2 = 103; bar 2's high 104.5 is above it
        assert!(sg.buy[1]);
        assert_eq!(sg.scale.len(), 1);
        assert!((sg.scale[0].price - 103.0).abs() < 1e-4, "{}", sg.scale[0].price);
    }

    #[test]
    fn vix_filter_blocks_entries_when_vix_is_high() {
        let rows: Vec<_> = (0..12).map(|i| nb(100.0 + i as f32)).collect();
        let s = mk(&rows);
        let mk_vix = |v: f32| {
            let bars = (0..12).map(|i| Bar { date: date_of(i), c: v, o: v, h: v, l: v, v: 0.0, oi: 0.0, aux1: 0.0, aux2: 0.0 }).collect();
            Series { name: "VIX".into(), bars }
        };
        let mut env = Env::empty();
        env.index.insert("VIX".into(), mk_vix(30.0));
        assert!(avg_vix_signals(&s, &env, cm0(), 10.0, 3, 25.0).buy.iter().all(|&b| !b), "average VIX 30 > 25: no entry");
        assert!(vix_filter_signals(&s, &env, cm0(), 10.0, 25.0).buy.iter().all(|&b| !b));
        env.index.insert("VIX".into(), mk_vix(15.0));
        assert!(avg_vix_signals(&s, &env, cm0(), 10.0, 3, 25.0).buy.iter().any(|&b| b));
        assert!(vix_filter_signals(&s, &env, cm0(), 10.0, 25.0).buy.iter().any(|&b| b));
    }

    #[test]
    fn when_vix_buys_only_while_the_vix_supertrend_is_down() {
        // daily VIX falling steadily: its supertrend (period 2, factor 1) turns down; rising: up
        let rows: Vec<_> = (0..40).map(|i| nb(100.0 + i as f32)).collect();
        let s = mk(&rows);
        let vix = |f: &dyn Fn(usize) -> f32| {
            let bars = (0..40).map(|i| { let v = f(i); Bar { date: date_of(i), c: v, o: v, h: v + 0.5, l: v - 0.5, v: 0.0, oi: 0.0, aux1: 0.0, aux2: 0.0 } }).collect();
            Series { name: "VIX".into(), bars }
        };
        let q = WhenVix { cm: cm0(), exit_pct: 10.0, tf: 2, pd: 2, fact: 1.0, point: 1 };
        let mut env = Env::empty();
        env.index.insert("VIX".into(), vix(&|i| 60.0 - i as f32));
        assert!(when_vix_signals(&s, &env, &q).buy.iter().any(|&b| b), "falling VIX: buys");
        env.index.insert("VIX".into(), vix(&|i| 20.0 + i as f32));
        assert!(when_vix_signals(&s, &env, &q).buy.iter().all(|&b| !b), "rising VIX: no entry");
    }

    #[test]
    fn hard_stop_loss_sells_at_the_close_below_the_stop_and_the_lowest_close_version_uses_the_entry_bar_low_close() {
        // entry at 101 (new high), stop 2 % = 98.98; bar 2 closes at 98 -> sell at the close (the swing line is far away)
        let rows = vec![nb(100.0), nb(101.0), nb(98.0), nb(97.0)];
        let s = mk(&rows);
        let sg = hard_sl_signals(&s, cm0(), 21.0, 0, 5, 3.0, 1, 2.0);
        assert!(sg.buy[1] && sg.sell[2]);
        assert_eq!(sg.sell_price[2], 98.0);
        // without the stop (99 %) the position stays open
        let off = hard_sl_signals(&s, cm0(), 21.0, 0, 5, 3.0, 1, 99.0);
        assert!(off.buy[1] && off.sell.iter().all(|&x| !x));
        // lowest-close version, 1 day: stop = the entry bar's close (101); a close below it sells
        let lo = hard_sl_signals(&s, cm0(), 21.0, 2, 5, 3.0, 1, 2.0);
        assert!(lo.sell[2]);
    }

    #[test]
    fn every_new_high_close_adds_a_scale_in_order_and_sizes_are_one_percent() {
        let rows = vec![nb(100.0), nb(101.0), nb(102.0), nb(103.0), nb(80.0)];
        let s = mk(&rows);
        let sg = scale_every_high_signals(&s, cm0(), 10.0, 10000.0, 0.25, 1, 2.0);
        assert!(sg.buy[1] && sg.sell[4]);
        assert_eq!(sg.scale.iter().map(|x| x.bar).collect::<Vec<_>>(), vec![2, 3]);
        assert_eq!(sg.entry_size, 0.5, "1 % of equity with a 2 % position size");
        assert_eq!(sg.scale[0].mult, 1.0);
        // the "2nd" version: 5 % for the first tranche, 1 % for the scale-ins
        let sg2 = scale_every_high_signals(&s, cm0(), 10.0, 10000.0, 0.25, 0, 2.0);
        assert_eq!(sg2.entry_size, 2.5);
        assert!((sg2.scale[0].mult - 0.2).abs() < 1e-6);
    }

    #[test]
    fn profit_booking_sells_half_of_the_shares_at_the_target_and_the_rest_at_the_exit() {
        let mut rows = vec![nb(100.0), nb(101.0), nb(102.0)];
        rows.push((103.0, 115.0, 102.5, 112.0)); // high 115 >= 101 * 1.10 = 111.1
        rows.push((100.0, 101.0, 70.0, 75.0)); // swing break
        let s = mk(&rows);
        let sg = profit_booking_signals(&s, cm0(), 10.0, 10.0, 50.0);
        assert!(sg.buy[1]);
        assert_eq!(sg.scale_out.len(), 1);
        assert_eq!(sg.scale_out[0].bar, 3);
        assert!((sg.scale_out[0].price - (1.0f32 + 10.0f32 * 0.01) * 101.0).abs() < 1e-3);
        assert!(sg.sell[4]);
        // the portfolio: two rows, the shares add up
        let pp = PortfolioParams { initial_equity: 100_000.0, pct_of_equity: 50.0, fixed_amount: None, max_positions: 2, commission_pct: 0.0, start: 20200101, end: 20201231, amibroker_profit: false, amibroker_full: false, current_equity_sizing: false };
        let t = backtest(&[(&s, &sg)], &[0], &pp);
        assert_eq!(t.len(), 2);
        let (a, b) = (&t[0], &t[1]);
        assert_eq!(a.shares + b.shares, (50_000.0f64 / 101.0).floor());
        assert!((a.shares - (a.shares + b.shares) * 0.5).abs() <= 1.0);
        assert!(a.exit_date < b.exit_date);
    }

    #[test]
    fn fifty_percent_of_equity_scale_in_buys_a_second_half_when_the_close_dips_below_the_entry() {
        // %swing exit 21 %, TrendUp (the stop line) under the price; a dip to a close below the entry and below TrendUp * 1.05 adds the second half
        let mut rows = vec![nb(100.0), nb(101.0), nb(102.0), nb(103.0), nb(104.0)];
        rows.push((86.0, 86.5, 84.5, 85.0)); // close 85 < entry 101 and < TrendUp (104 * 0.79 = 82.2) * 1.05 = 86.3, and above the swing line
        rows.push(nb(60.0));
        let s = mk(&rows);
        let q = Half { cm: cm0(), ver: 0, exit_pct: 21.0, tf: 2, pd: 10, fact: 4.0, point: 0, prcnt: 5.0 };
        let sg = half_scale_signals(&s, &q);
        assert!(sg.buy[1]);
        assert_eq!(sg.entry_size, 0.5);
        assert!(!sg.scale.is_empty(), "the dip adds the second tranche");
        assert!(sg.scale[0].bar == 5);
        assert!(sg.sell[6]);
    }

    #[test]
    fn adx_is_high_in_a_straight_trend_and_the_entry_filter_blocks_a_flat_market() {
        let up: Vec<_> = (0..60).map(|i| nb(100.0 + 2.0 * i as f32)).collect();
        let s = mk(&up);
        let a = adx_w(&s.high(), &s.low(), &s.close(), 5);
        assert!(is_null(a[3]) && a[40] > 50.0, "{}", a[40]);
        assert!(adx_entry_signals(&s, cm0(), 21.0, 5, 20.0).buy.iter().any(|&b| b));
        // sideways: closes alternate, no new high after the first bars and a low ADX
        let flat: Vec<_> = (0..60).map(|i| nb(if i % 2 == 0 { 100.0 } else { 100.4 })).collect();
        let f = mk(&flat);
        assert!(adx_entry_signals(&f, cm0(), 21.0, 5, 80.0).buy.iter().all(|&b| !b));
        // the ADX-exit AFL buys in the trend as well
        assert!(adx_exit_signals(&s, cm0(), 21.0, 5, 20.0, 20.0).buy.iter().any(|&b| b));
    }

    #[test]
    fn atr_profit_target_sells_at_the_close_of_the_bar_that_reaches_the_target() {
        // ATR(5) is about 2 on these bars, target = close + 10 * ATR ~ 121: a bar with a high above it sells at its close (not at the target)
        // (ATR(5) needs five bars: a buy before that has a Null target and never books profit)
        let mut rows = vec![nb(100.0)];
        rows.extend((0..6).map(|_| nb(90.0)));
        rows.extend([nb(95.0), nb(99.0), nb(101.0)]);
        rows.push((102.0, 200.0, 101.0, 110.0));
        let s = mk(&rows);
        let sg = atr_profit_signals(&s, cm0(), 21.0, 0);
        assert!(sg.buy[9], "new high with a valid ATR");
        assert!(sg.sell[10]);
        assert_eq!(sg.sell_price[10], 110.0);
        // the ATR hard-stop version buys on a new high without the trend test and also exits on the target
        let sg1 = atr_profit_signals(&s, cm0(), 21.0, 1);
        assert!(sg1.buy[9] && sg1.sell[10]);
    }

    #[test]
    fn new_buy_daily_buys_only_on_a_new_high_bar_and_the_running_version_keeps_buying_after_a_blocked_bar() {
        // bar 2 is a new high but an upper circuit (open = high = close): blocked. Bar 3 is not a new high.
        let rows = vec![nb(100.0), nb(101.0), (102.0, 102.0, 101.0, 102.0), nb(101.5), nb(60.0)];
        // the first buy happens at bar 1 for both; so use a series whose only new-high bar is the circuit bar
        let rows2 = vec![nb(100.0), nb(99.0), (101.0, 101.0, 100.0, 101.0), nb(100.5), nb(100.8), nb(60.0)];
        let s = mk(&rows2);
        let a = new_buy_daily_signals(&s, cm0(), 10.0, 1);
        assert!(a.buy.iter().all(|&b| !b), "the only new-high bar is an upper circuit");
        let b = turnover_running_signals(&s, cm0(), 10.0);
        assert!(b.buy[3], "the running buy keeps buying on a later bar while the ATH state is on");
        let s1 = mk(&rows);
        let a1 = new_buy_daily_signals(&s1, cm0(), 10.0, 1);
        assert!(a1.buy[1] && a1.sell[4]);
        // turnover new buy with the filter off = running buy on the ATH state
        let t = turnover_new_buy_signals(&s1, cm0(), 10.0);
        assert!(t.buy[1]);
        let r = turnover_running_signals(&s1, cm0(), 10.0);
        assert!(r.buy[1] && r.sell[4]);
        // hard stop is an ApplyStop percent stop
        assert_eq!(with_hard_stop(r, 7.0).stop.unwrap().loss_pct, 7.0);
    }

    #[test]
    fn swing_calculation_points() {
        let s = mk(&[nb(100.0), nb(102.0), nb(104.0), nb(106.0), nb(108.0), nb(110.0)]);
        let p = px(&s);
        assert_eq!(stcal(&p, 0), p.c);
        assert_eq!(stcal(&p, 2), p.h);
        assert!((stcal(&p, 1)[2] - 104.0).abs() < 1e-4, "(H + L) / 2");
        assert!(is_null(stcal(&p, 5)[1]) && (stcal(&p, 5)[2] - 102.0).abs() < 1e-4, "3 days MA");
        assert!(is_null(stcal(&p, 6)[3]) && (stcal(&p, 6)[4] - 104.0).abs() < 1e-4, "5 days MA");
        assert!((stcal(&p, 3)[2] - 104.0).abs() < 1e-4);
        // the whole signal runs for every calculation point
        for pt in 0..7u8 {
            let sg = midpoint_signals(&s, cm0(), 10.0, pt);
            assert_eq!(sg.buy.len(), 6);
        }
    }

    #[test]
    fn daily_chart_entry_exits_only_on_the_last_bar_of_a_falling_week() {
        // weekly closes 100, 110, 120 (rising: the weekly trend stays up) then a 30 % fall in the next week
        let mut rows = Vec::new();
        for w in 0..3 {
            for d in 0..5 {
                rows.push(nb(100.0 + 10.0 * w as f32 + d as f32));
            }
        }
        for d in 0..5 {
            rows.push(nb(80.0 - d as f32)); // week 4 closes at 76: below the weekly line (about 120 * 0.9)
        }
        rows.push(nb(75.0));
        let s = mk(&rows);
        let sg = daily_chart_signals(&s, cm0(), 1, 10.0, 0.25);
        assert!(sg.buy.iter().any(|&b| b));
        let sell_i = sg.sell.iter().position(|&x| x).expect("a sell");
        assert_eq!(sell_i, 19, "the last daily bar of the falling week (index 19), not the first falling day");
    }

    #[test]
    fn weekly_athc_enters_at_the_close_of_a_week_with_a_new_weekly_high() {
        let mut rows = Vec::new();
        for w in 0..4 {
            for d in 0..5 {
                rows.push(nb(100.0 + 10.0 * w as f32 + d as f32 * 0.5));
            }
        }
        let s = mk(&rows);
        let q = Weekly { cm: cm0(), ver: 0, buy_buf: 0.25, sell_buf: 0.25, exit_pct: 10.0, tf: 2, pd: 10, fact: 4.0, point: 0 };
        let sg = weekly_athc_signals(&s, &q);
        let b = sg.buy.iter().position(|&x| x).expect("a buy");
        assert_eq!(b % 5, 4, "entries are on the last bar of a week");
        assert_eq!(weekly_buy_price(100.0, 100.0, 99.0, 0.25), 100.0);
        assert_eq!(weekly_buy_price(100.0, 101.0, 99.0, 0.25), 100.25);
        assert_eq!(weekly_buy_price(100.0, 100.1, 99.0, 0.25), 100.1);
        assert_eq!(weekly_sell_price(100.0, 98.0, 99.0, 0.25), 99.75);
    }

    #[test]
    fn stock_selection_enters_on_the_small_swing_and_leaves_on_the_small_swing_or_the_ath_state() {
        let mut rows: Vec<_> = (0..10).map(|i| nb(100.0 + 2.0 * i as f32)).collect();
        rows.push(nb(100.0)); // 11 % below the high: small 5 % swing breaks, big 21 % does not
        let s = mk(&rows);
        let q = StockSel { cm: cm0(), ver: 0, exit_pct: 21.0, small_pct: 5.0, tf: 2, pd: 10, fact: 4.0, point: 0 };
        let sg = stock_selection_signals(&s, &q);
        assert!(sg.buy.iter().any(|&b| b));
        assert!(sg.sell[10], "the small swing exits first");
    }

    #[test]
    fn supertrend_run_expands_like_timeframeexpand_and_compression_is_weekly() {
        let rows: Vec<_> = (0..15).map(|i| nb(100.0 + i as f32)).collect();
        let s = mk(&rows);
        let k = compress(&(0..15).map(|i| date_of(i)).collect::<Vec<_>>(), &al_of(&s), 1);
        assert_eq!(k.start, vec![0, 5, 10]);
        let e = expand(&[1.0, 2.0, 3.0], &k, 15);
        assert_eq!((e[0], e[3], e[4], e[5], e[9], e[10], e[14]), (NULLV, NULLV, 1.0, 1.0, 2.0, 2.0, 3.0));
        let (mt, _tu) = st_run(&(0..15).map(|i| date_of(i)).collect::<Vec<_>>(), &al_of(&s), 1, 0, 1, 1.0, 1.0);
        assert_eq!(mt.len(), 15);
    }

    #[test]
    fn every_variant_is_registered_with_a_valid_schema() {
        let vs = variants();
        assert_eq!(vs.len(), 21);
        let mut ids: Vec<&str> = vs.iter().map(|v| v.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 21);
        for v in &vs {
            let sc = (v.schema)();
            for q in &sc {
                assert!(q.min <= q.max, "{} {}", v.id, q.key);
                assert!(q.def.clamp(q.min, q.max).is_finite());
            }
            let mut keys: Vec<&str> = sc.iter().map(|q| q.key).collect();
            keys.sort();
            keys.dedup();
            assert_eq!(keys.len(), sc.len(), "duplicate key in {}", v.id);
        }
        // the AFL toggles: dead stock default NO (1), turnover default NO (0)
        let g = gfn(&[("deadstock", 1.0), ("turnflt", 0.0), ("pennyflt", 0.0)]);
        let cm = cmn(&g);
        assert!(!cm.dead_yes && !cm.turn && cm.dead_buy(0, 5) == false && cm.dead_buy(4, 5) && !cm.dead_sell(4, 5));
        let cm = Cmn { dead_yes: true, ..cm };
        assert!(!cm.dead_buy(4, 5) && cm.dead_sell(4, 5));
    }
}
