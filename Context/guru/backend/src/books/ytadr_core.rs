//! Pure port of the team's Advance Decline Ratio AFLs (share `Jay/Old Topics/Advance Decline Ratio`). No crate imports, so
//! the verification driver can include this file on its own.
//!
//! AmiBroker conventions kept: float32 prices and arithmetic; arithmetic or a comparison on a Null input gives Null and a
//! Null tests false (team sheet "FInal_Updated All Versions ADR_Summary" C: no entry on 2000-01-05, the 3rd bar, where the
//! 5-bar Supertrend filter is still Null, although the ratio met the threshold); the AFL loop
//! runs over every bar of the data (SetBarsRequired 100000) and the backtester then only takes the signals inside the
//! chosen range (`trades`): on each bar exits first, then entries, so a Sell on the bar where a Buy was taken is not used.
#![allow(dead_code)]

pub const NULL: f32 = -1e10;

/// Bars of the traded symbol plus the Nifty 50 breadth (`Foreign("~Advances")`, `Foreign("~Declines")`); NaN = no data.
pub struct Bars<'a> {
    pub d: &'a [i32],
    pub o: &'a [f32],
    pub h: &'a [f32],
    pub l: &'a [f32],
    pub c: &'a [f32],
    pub adv: &'a [f32],
    pub dec: &'a [f32],
}

/// The AFL a variant is coded from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Afl {
    /// AFLs/Advance Decline LongShort.afl
    LongShort,
    /// AFLs/Advance Decline LongShort_Versions.afl (RSI filter; exit checked before the entry on a bar)
    Versions,
    /// AFLs/Advance Decline LongShort_with5DayExit.afl (Versions + "5DP Entry" exit, direction-aware PB / HSL)
    FiveDayExit,
    /// AFLs/Advance Decline LongShort_Versions_Without RSI.afl (no RSI; entry before the exit; Green Candle / UpClose exits)
    WoRsi,
    /// Exit by N days/HSL- PB Optim/V1+V2/Advance Decline LongShort_Versions_Without RSI.afl (older copy: exit before entry)
    WoRsiNDay,
    /// AFLs/Advance Decline LongShort_Versions_Two_SuperTrend.afl (Without RSI + a second ST filter)
    TwoSt,
    /// AFLs/ADR Long 2-11 and 5DP.afl
    Patch,
    /// AFLs/ADR Long 2-11 and 5DP ROC.afl
    PatchRoc,
    /// 5DP + ADR/FIVE Day Phenomena with ADR.afl
    FiveDp,
}

/// Raw AFL signal arrays (Buy / Sell / Short / Cover with their prices) and the exit rule that fired on each bar.
pub struct Sig {
    pub buy: Vec<bool>,
    pub sell: Vec<bool>,
    pub short: Vec<bool>,
    pub cover: Vec<bool>,
    pub bp: Vec<f32>,
    pub sp: Vec<f32>,
    pub shp: Vec<f32>,
    pub cp: Vec<f32>,
    pub why: Vec<&'static str>,
    /// Set when the AFL as written cannot run with these settings (AmiBroker stops with an error).
    pub error: Option<&'static str>,
}

impl Sig {
    fn new(n: usize) -> Sig {
        Sig {
            buy: vec![false; n],
            sell: vec![false; n],
            short: vec![false; n],
            cover: vec![false; n],
            bp: vec![0.0; n],
            sp: vec![0.0; n],
            shp: vec![0.0; n],
            cp: vec![0.0; n],
            why: vec![""; n],
            error: None,
        }
    }
}

/// One trade of the backtester: direction (+1 / -1), entry bar and price, exit bar and price, exit rule, still open.
#[derive(Clone, Debug, PartialEq)]
pub struct Tr {
    pub dir: f32,
    pub ei: usize,
    pub ep: f32,
    pub xi: usize,
    pub xp: f32,
    pub why: &'static str,
    pub open: bool,
}

// ------------------------------------------------------------------------------------------------- AFL helpers

fn nz(x: f32) -> f32 {
    if x.is_nan() { NULL } else { x }
}
/// A value tested in an `if` / AND: Null is false.
fn truthy(x: f32) -> bool {
    x != 0.0 && x != NULL && !x.is_nan()
}
fn is_null(x: f32) -> bool {
    x == NULL || x.is_nan()
}
/// `a > b` / `a < b` with AmiBroker's Null: a Null operand gives Null, which tests false.
fn gt(a: f32, b: f32) -> bool {
    !is_null(a) && !is_null(b) && a > b
}
fn lt(a: f32, b: f32) -> bool {
    !is_null(a) && !is_null(b) && a < b
}
fn b(x: bool) -> f32 {
    if x { 1.0 } else { 0.0 }
}
/// `Ref(a, k)`: k < 0 looks back, k > 0 looks ahead; outside the data = Null.
fn ref_(a: &[f32], k: i64) -> Vec<f32> {
    let n = a.len() as i64;
    (0..n).map(|i| if i + k >= 0 && i + k < n { a[(i + k) as usize] } else { NULL }).collect()
}
/// `Sum(a, n)`: Null for the first n-1 bars or when a Null is in the window.
fn sum(a: &[f32], n: usize) -> Vec<f32> {
    let n = n.max(1);
    (0..a.len())
        .map(|i| {
            if i + 1 < n {
                return NULL;
            }
            let w = &a[i + 1 - n..=i];
            if w.iter().any(|&x| x == NULL || x.is_nan()) { NULL } else { w.iter().sum() }
        })
        .collect()
}
/// `Month() != Ref(Month(), 1)`: the last bar of a month; Null on the last bar of the data (Ref is Null there).
fn month_end(d: &[i32]) -> Vec<f32> {
    (0..d.len()).map(|i| if i + 1 >= d.len() { NULL } else { b(d[i] / 100 % 100 != d[i + 1] / 100 % 100) }).collect()
}
fn days_from_civil(y: i32, m: i32, d: i32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
/// Period of a daily bar: 0 daily, 1 calendar week (Monday first), 2 calendar month.
fn period_key(d: i32, tf: u8) -> i64 {
    match tf {
        1 => (days_from_civil(d / 10000, d / 100 % 100, d % 100) + 3).div_euclid(7),
        2 => (d / 100) as i64,
        _ => d as i64,
    }
}
/// `TimeFrameSet` of daily OHLC: (first bar, last bar, o, h, l, c) per period.
struct Comp {
    start: Vec<usize>,
    end: Vec<usize>,
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    c: Vec<f32>,
}
fn compress(bars: &Bars, tf: u8) -> Comp {
    let mut k = Comp { start: vec![], end: vec![], o: vec![], h: vec![], l: vec![], c: vec![] };
    let mut last = i64::MIN;
    for i in 0..bars.d.len() {
        let key = period_key(bars.d[i], tf);
        if key != last || k.start.is_empty() {
            k.start.push(i);
            k.end.push(i);
            k.o.push(bars.o[i]);
            k.h.push(bars.h[i]);
            k.l.push(bars.l[i]);
            k.c.push(bars.c[i]);
            last = key;
        } else {
            let g = k.start.len() - 1;
            k.end[g] = i;
            k.h[g] = k.h[g].max(bars.h[i]);
            k.l[g] = k.l[g].min(bars.l[i]);
            k.c[g] = bars.c[i];
        }
    }
    k
}
/// `TimeFrameExpand(x, tf, expandFirst)`: every daily bar of a period shows that period's value.
fn expand_first(x: &[f32], k: &Comp, n: usize) -> Vec<f32> {
    let mut out = vec![NULL; n];
    for g in 0..k.start.len() {
        for j in k.start[g]..=k.end[g] {
            out[j] = x[g];
        }
    }
    out
}
/// `TimeFrameExpand(x, tf)` (expandLast): the period's value from its last daily bar, the previous value before.
fn expand_last(x: &[f32], k: &Comp, n: usize) -> Vec<f32> {
    let mut out = vec![NULL; n];
    for g in 0..k.start.len() {
        for j in k.start[g]..k.end[g] {
            out[j] = if g > 0 { x[g - 1] } else { NULL };
        }
        out[k.end[g]] = x[g];
    }
    out
}
/// `ATR(1)` = true range; Null on the first bar (no previous close). Team sheet "Best Setting Tradelist" E (weekly ST,
/// period 5): the first entry is 2000-02-15, not 2000-02-07, so the weekly Supertrend exists one week later than with H - L.
fn tr(h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    (0..c.len()).map(|i| if i == 0 { f32::NAN } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect()
}
/// `ATR(1)` summed over the last `pd` bars (`satr = Ref(ATR(1),0) + Ref(ATR(1),-1) + ...`), NaN until pd bars exist.
fn sum_tr(t: &[f32], pd: usize) -> Vec<f32> {
    let pd = pd.max(1);
    (0..t.len())
        .map(|i| {
            if i + 1 < pd {
                return f32::NAN;
            }
            let mut s = t[i];
            for k in 1..pd {
                s += t[i - k];
            }
            s
        })
        .collect()
}
/// Wilder smoothing seeded with the simple average of the first n values (NaN before).
fn wilder(v: &[f32], n: usize) -> Vec<f32> {
    let n = n.max(1);
    let mut out = vec![f32::NAN; v.len()];
    let Some(s) = v.iter().position(|x| !x.is_nan()) else { return out };
    if v.len() < s + n {
        return out;
    }
    let mut prev = v[s..s + n].iter().sum::<f32>() / n as f32;
    out[s + n - 1] = prev;
    for i in s + n..v.len() {
        prev = (prev * (n as f32 - 1.0) + v[i]) / n as f32;
        out[i] = prev;
    }
    out
}
/// `RSI(n)` (Wilder); Null before it exists.
pub fn rsi(c: &[f32], n: usize) -> Vec<f32> {
    let mut up = vec![f32::NAN; c.len()];
    let mut dn = vec![f32::NAN; c.len()];
    for i in 1..c.len() {
        let d = c[i] - c[i - 1];
        up[i] = d.max(0.0);
        dn[i] = (-d).max(0.0);
    }
    let (au, ad) = (wilder(&up, n), wilder(&dn, n));
    (0..c.len())
        .map(|i| {
            let (u, d) = (au[i], ad[i]);
            if u.is_nan() || d.is_nan() {
                NULL
            } else if u + d == 0.0 {
                50.0
            } else {
                100.0 * u / (u + d)
            }
        })
        .collect()
}
/// `ADX(n)` (Wilder's +DI / -DI / DX, smoothed with Wilder averages); Null before it exists.
pub fn adx(h: &[f32], l: &[f32], c: &[f32], n: usize) -> Vec<f32> {
    let m = c.len();
    let (mut pdm, mut mdm, mut t) = (vec![f32::NAN; m], vec![f32::NAN; m], vec![f32::NAN; m]);
    for i in 1..m {
        let (up, dn) = (h[i] - h[i - 1], l[i - 1] - l[i]);
        pdm[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        mdm[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        t[i] = h[i].max(c[i - 1]) - l[i].min(c[i - 1]);
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&t, n));
    let dx: Vec<f32> = (0..m)
        .map(|i| {
            if st[i].is_nan() || st[i] == 0.0 {
                return f32::NAN;
            }
            let (p, q) = (100.0 * sp[i] / st[i], 100.0 * sm[i] / st[i]);
            if p + q == 0.0 { 0.0 } else { 100.0 * (p - q).abs() / (p + q) }
        })
        .collect();
    wilder(&dx, n).into_iter().map(nz).collect()
}
/// `ROC(Close, n)` in percent; Null for the first n bars.
pub fn roc(c: &[f32], n: usize) -> Vec<f32> {
    (0..c.len()).map(|i| if i >= n && c[i - n] != 0.0 { (c[i] - c[i - n]) / c[i - n] * 100.0 } else { NULL }).collect()
}

/// AD ratio of every bar: Advances / Declines; when Declines is 0 the AFL uses 50 (`dec0_adv` = false) or the Advances
/// (Advance Decline LongShort.afl). NaN where the breadth file has no row.
pub fn ad_ratio(bars: &Bars, dec0_adv: bool) -> Vec<f32> {
    (0..bars.c.len())
        .map(|i| {
            let (a, d) = (bars.adv.get(i).copied().unwrap_or(f32::NAN), bars.dec.get(i).copied().unwrap_or(f32::NAN));
            if a.is_nan() || d.is_nan() {
                f32::NAN
            } else if d != 0.0 {
                a / d
            } else if dec0_adv {
                a
            } else {
                50.0
            }
        })
        .collect()
}

/// STBullPhase of the "ST FILTER PARAMETERS" section: Supertrend on the daily / weekly / monthly bars (tf 0 / 1 / 2),
/// `pd` bars of ATR(1), factor `fact`, average value `avg` (0 Close, 1 Open, 2 High, 3 Low, 4 (O+C)/2, 5 (H+L)/2,
/// 6 (H+L+C)/3, 7 (O+H+L+C)/4). For weekly / monthly the AFL takes Ref(-1) on the period bars, expands with expandFirst
/// and then Ref(+1) on the daily bars: the last day of a period sees that period's value, the other days the previous one.
pub fn st_bull(bars: &Bars, tf: u8, pd: usize, fact: f32, avg: u8) -> Vec<Option<bool>> {
    let n = bars.c.len();
    let k = if tf == 0 {
        Comp { start: (0..n).collect(), end: (0..n).collect(), o: bars.o.to_vec(), h: bars.h.to_vec(), l: bars.l.to_vec(), c: bars.c.to_vec() }
    } else {
        compress(bars, tf)
    };
    let m = k.c.len();
    let pd = pd.max(1);
    let t = tr(&k.h, &k.l, &k.c);
    let satr = sum_tr(&t, pd);
    let av: Vec<f32> = (0..m)
        .map(|g| match avg {
            1 => k.o[g],
            2 => k.h[g],
            3 => k.l[g],
            4 => (k.o[g] + k.c[g]) / 2.0,
            5 => (k.h[g] + k.l[g]) / 2.0,
            6 => (k.h[g] + k.l[g] + k.c[g]) / 3.0,
            7 => (k.o[g] + k.h[g] + k.l[g] + k.c[g]) / 4.0,
            _ => k.c[g],
        })
        .collect();
    let mut up: Vec<f32> = (0..m).map(|g| nz(av[g] + satr[g] / pd as f32 * fact)).collect();
    let mut dn: Vec<f32> = (0..m).map(|g| nz(av[g] - satr[g] / pd as f32 * fact)).collect();
    let mut is_up = vec![true; m];
    for g in pd..m {
        is_up[g] = if k.c[g] > up[g - 1] {
            true
        } else if k.c[g] < dn[g - 1] {
            false
        } else {
            is_up[g - 1]
        };
        if is_up[g] {
            dn[g] = dn[g].max(dn[g - 1]);
        } else {
            up[g] = up[g].min(up[g - 1]);
        }
    }
    let plot: Vec<f32> = (0..m).map(|g| if is_up[g] { dn[g] } else { up[g] }).collect();
    let (plot, close_m) = if tf == 0 {
        (plot, k.c.clone())
    } else {
        let (p1, c1) = (ref_(&plot, -1), ref_(&k.c, -1));
        (ref_(&expand_first(&p1, &k, n), 1), ref_(&expand_first(&c1, &k, n), 1))
    };
    (0..n).map(|i| if is_null(close_m[i]) || is_null(plot[i]) { None } else { Some(close_m[i] > plot[i]) }).collect()
}

/// Closing-percentage swing trend (the AFLs' `trend1` loop): +1 / -1 per bar, the band `pct_up` / `pct_dn` % of the close.
fn pct_swing(c: &[f32], pct_up: f32, pct_dn: f32) -> (Vec<i8>, Vec<f32>, Vec<f32>) {
    let n = c.len();
    let mut up: Vec<f32> = c.iter().map(|&x| x + x * pct_up / 100.0).collect();
    let mut dn: Vec<f32> = c.iter().map(|&x| x - x * pct_dn / 100.0).collect();
    let mut tr1 = vec![1i8; n];
    let (mut up_f, mut dn_f) = (vec![NULL; n], vec![NULL; n]);
    let mut change = false;
    for i in 1..n {
        tr1[i] = if c[i] > up[i - 1] {
            if tr1[i - 1] == -1 {
                change = true;
            }
            1
        } else if c[i] < dn[i - 1] {
            if tr1[i - 1] == 1 {
                change = true;
            }
            -1
        } else {
            change = false;
            tr1[i - 1]
        };
        let flag = tr1[i] < 0 && tr1[i - 1] > 0;
        let flagh = tr1[i] > 0 && tr1[i - 1] < 0;
        if tr1[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if tr1[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if flag {
            up[i] = c[i] + c[i] * pct_up / 100.0;
        }
        if flagh {
            dn[i] = c[i] - c[i] * pct_dn / 100.0;
        }
        if tr1[i] == 1 {
            up_f[i] = dn[i];
            if change {
                up_f[i - 1] = dn_f[i - 1];
                change = false;
            }
        } else {
            dn_f[i] = up[i];
            if change {
                dn_f[i - 1] = up_f[i - 1];
                change = false;
            }
        }
    }
    (tr1, up_f, dn_f)
}

/// MTREND of the 5DP AFL: Supertrend on the filter timeframe (tf 0 monthly, 1 weekly, 2 daily), calculation point
/// `point` (0 (H+L)/2, 1 Close, 2 (H+L+C)/3), expanded with the default expandLast.
fn mtrend(bars: &Bars, tf: u8, point: u8, pd: usize, fact: f32) -> Vec<f32> {
    let n = bars.c.len();
    let tfk = match tf {
        0 => 2,
        1 => 1,
        _ => 0,
    };
    let k = if tfk == 0 {
        Comp { start: (0..n).collect(), end: (0..n).collect(), o: bars.o.to_vec(), h: bars.h.to_vec(), l: bars.l.to_vec(), c: bars.c.to_vec() }
    } else {
        compress(bars, tfk)
    };
    let m = k.c.len();
    let pd = pd.max(1);
    let cal: Vec<f32> = (0..m)
        .map(|g| match point {
            0 => (k.h[g] + k.l[g]) / 2.0,
            2 => (k.h[g] + k.l[g] + k.c[g]) / 3.0,
            _ => k.c[g],
        })
        .collect();
    let satr: Vec<f32> = sum_tr(&tr(&k.h, &k.l, &k.c), pd).into_iter().map(|s| s / pd as f32).collect();
    let mut up: Vec<f32> = (0..m).map(|g| nz(cal[g] + fact * satr[g])).collect();
    let mut dn: Vec<f32> = (0..m).map(|g| nz(cal[g] - fact * satr[g])).collect();
    let mut t = vec![1f32; m];
    for i in 1..m {
        t[i] = if k.c[i] > up[i - 1] {
            1.0
        } else if k.c[i] < dn[i - 1] {
            -1.0
        } else {
            t[i - 1]
        };
        let flag = t[i] < 0.0 && t[i - 1] > 0.0;
        let flagh = t[i] > 0.0 && t[i - 1] < 0.0;
        if t[i] > 0.0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if t[i] < 0.0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if flag {
            up[i] = nz(cal[i] + fact * satr[i]);
        }
        if flagh {
            dn[i] = nz(cal[i] - fact * satr[i]);
        }
    }
    if tfk == 0 { t } else { expand_last(&t, &k, n) }
}

/// Parameter lookup: the caller returns the value of a key (the variant's default when the user did not set it).
pub type G<'a> = &'a dyn Fn(&str) -> f64;

fn gi(g: G, k: &str) -> i64 {
    g(k).round() as i64
}
fn gf(g: G, k: &str) -> f32 {
    g(k) as f32
}
fn on(g: G, k: &str) -> bool {
    g(k) >= 0.5
}

/// Entry-signal count of the "MAIN ENTRY PARAMETERS" section on `sig`: consecutive (count resets on a miss) or, with
/// Consecutive Candles OFF, at least `need` hits in the last `look` bars (the AFL turns a look-back of 2 into 1).
fn main_entry(sig: &[bool], consec: bool, look: i64, need: i64) -> Vec<bool> {
    let n = sig.len();
    let mut out = vec![false; n];
    if consec {
        let mut cnt = 0i64;
        for i in 1..n {
            cnt = if sig[i] { cnt + 1 } else { 0 };
            if cnt >= need {
                out[i] = true;
            }
        }
    } else {
        let look = if look == 2 { 1 } else { look };
        let start = (look + 1).max(0) as usize;
        for i in start..n {
            let mut cnt = 0i64;
            let mut j = look - 1;
            while j >= 0 {
                if sig[i - j as usize] {
                    cnt += 1;
                }
                if cnt >= need {
                    out[i] = true;
                    cnt = 0;
                }
                j -= 1;
            }
        }
    }
    out
}

/// MainEntrySig2 of the "Previous N Candles" filter: `n` candles of the colour in a row (0 Green, 1 Red, 2 Any).
fn prev_candles(bars: &Bars, color: i64, n_prev: i64) -> Vec<bool> {
    let n = bars.c.len();
    let mut out = vec![false; n];
    let mut cnt = 0i64;
    for i in n_prev.max(0) as usize..n {
        let hit = match color {
            0 => bars.c[i] > bars.o[i],
            1 => bars.c[i] < bars.o[i],
            _ => true,
        };
        cnt = if hit { cnt + 1 } else { 0 };
        if cnt >= n_prev {
            out[i] = true;
        }
    }
    out
}

/// Long exit price of the PB / HSL section (PB first, HSL overrides it), else the close.
fn long_exit_px(bars: &Bars, i: usize, live: bool, pb_hit: bool, hsl_hit: bool, pb: f32, hsl: f32, buf: f32) -> f32 {
    let (o, l) = (bars.o[i], bars.l[i]);
    let mut px = bars.c[i];
    if live && pb_hit {
        px = if o > pb { l.max(o * (1.0 - buf * 0.01)) } else { pb };
    }
    if live && hsl_hit {
        px = if o < hsl { l.max(o * (1.0 - buf * 0.01)) } else { hsl };
    }
    px
}
fn short_exit_px(bars: &Bars, i: usize, live: bool, pb_hit: bool, hsl_hit: bool, pb: f32, hsl: f32, buf: f32) -> f32 {
    let (o, h) = (bars.o[i], bars.h[i]);
    let mut px = bars.c[i];
    if live && pb_hit {
        px = if o < pb { h.min(o * (1.0 + buf * 0.01)) } else { pb };
    }
    if live && hsl_hit {
        px = if o > hsl { h.min(o * (1.0 + buf * 0.01)) } else { hsl };
    }
    px
}

// --------------------------------------------------------------------------------------------- the AFLs

/// Advance Decline LongShort.afl: Long when the ratio (Declines 0 -> Advances) <= Lower Threshold, Short when >= Upper
/// Threshold, at the close; exit at the close after `N Candles` bars (type "N Candle"; "AD Ratio" has no exit coded).
fn long_short(bars: &Bars, g: G) -> Sig {
    let n = bars.c.len();
    let mut s = Sig::new(n);
    let short = on(g, "entry_type");
    let ratio = ad_ratio(bars, true);
    let (up, dn) = (gf(g, "thr_up"), gf(g, "thr_dn"));
    let n_candle = gi(g, "exit_type") == 0;
    let num = gf(g, "exit_n");
    let mut pos = false;
    let mut cnt = 0f32;
    for i in 1..n {
        if pos {
            if n_candle {
                cnt += 1.0;
            }
            if cnt == num {
                if short {
                    s.cover[i] = true;
                    s.cp[i] = bars.c[i];
                } else {
                    s.sell[i] = true;
                    s.sp[i] = bars.c[i];
                }
                s.why[i] = "N candles";
                pos = false;
            }
        }
        let entry = if short { ratio[i] >= up } else { ratio[i] <= dn };
        if !pos && entry {
            if short {
                s.short[i] = true;
                s.shp[i] = bars.c[i];
            } else {
                s.buy[i] = true;
                s.bp[i] = bars.c[i];
            }
            pos = true;
            cnt = 0.0;
        }
    }
    s
}

/// The "Versions" family (Versions, with5DayExit, Without RSI, its N-days copy, Two SuperTrend).
fn versions(bars: &Bars, afl: Afl, g: G) -> Sig {
    let n = bars.c.len();
    let mut s = Sig::new(n);
    let short = on(g, "entry_type");
    if short && matches!(afl, Afl::WoRsi | Afl::WoRsiNDay | Afl::TwoSt) {
        // the short branch reads `Close0`, a variable the AFL never defines: AmiBroker stops with an error
        s.error = Some("Entry Type Short: the AFL's short branch reads Close0, which it never defines (AmiBroker error), so only Long runs");
        return s;
    }
    let ratio = ad_ratio(bars, false);
    let rsi_on = matches!(afl, Afl::Versions | Afl::FiveDayExit) && on(g, "rsi_on");
    let entry_first = matches!(afl, Afl::WoRsi | Afl::TwoSt);
    let dir_stops = !matches!(afl, Afl::Versions);
    // ---- entry signal
    let avg_on = on(g, "avg_on");
    let avg_n = gi(g, "avg_n").max(1) as usize;
    let mut avg = vec![0f32; n];
    for i in avg_n - 1..n {
        let mut acc = 0f32;
        for j in (0..avg_n).rev() {
            acc += ratio[i - j];
        }
        avg[i] = acc / avg_n as f32;
    }
    let (base, thr) = if avg_on { (&avg, gf(g, "avg_thr")) } else { (&ratio, gf(g, "thr")) };
    let sig: Vec<bool> = (0..n).map(|i| if short { base[i] >= thr } else { base[i] <= thr }).collect();
    let main = main_entry(&sig, on(g, "consec"), gi(g, "lookback"), gi(g, "n_entry"));
    let prev_on = on(g, "prev_on");
    let main2 = if prev_on { prev_candles(bars, gi(g, "prev_color"), gi(g, "prev_n")) } else { vec![true; n] };
    let rsi_ok: Vec<bool> = if rsi_on {
        let pd = gi(g, "rsi_pd");
        let v = if pd > 1 { rsi(bars.c, pd as usize) } else { vec![NULL; n] };
        let thr = gf(g, "rsi_thr");
        let above = on(g, "rsi_dir");
        v.iter().map(|&x| if above { gt(x, thr) } else { lt(x, thr) }).collect()
    } else {
        vec![true; n]
    };
    let st = |pfx: &str| -> Option<(Vec<Option<bool>>, bool)> {
        if !on(g, &format!("{pfx}_on")) {
            return None;
        }
        let bull = st_bull(bars, gi(g, &format!("{pfx}_tf")).clamp(0, 2) as u8, gi(g, &format!("{pfx}_pd")).max(1) as usize, gf(g, &format!("{pfx}_fact")), gi(g, &format!("{pfx}_avg")).clamp(0, 7) as u8);
        Some((bull, on(g, &format!("{pfx}_dir"))))
    };
    let st1 = st("st");
    let st2 = if afl == Afl::TwoSt { st("st2") } else { None };
    // ST Entry Filter Bullish: enter when STBullPhase, exit when NOT STBullPhase (Bearish the other way); Null does neither
    let st_entry = |i: usize| [&st1, &st2].iter().all(|x| x.as_ref().map_or(true, |(bull, bear)| bull[i] == Some(!*bear)));
    let st_exit = |i: usize| [&st1, &st2].iter().any(|x| x.as_ref().map_or(false, |(bull, bear)| bull[i] == Some(*bear)));
    // ---- exits
    let kind = gi(g, "exit_type");
    let (k_n, k_ad, k_green, k_up, k_5dp) = match afl {
        Afl::FiveDayExit => (kind == 0, kind == 1, false, false, kind == 2),
        Afl::WoRsi | Afl::TwoSt => (kind == 0, kind == 1, kind == 2, kind == 3, false),
        _ => (kind == 0, kind == 1, false, false, false),
    };
    let mut num = gi(g, "exit_n") - 1;
    let five = ref_(&month_end(bars.d), num); // with5DayExit: Ref(MonthEnd, numOfExitCandles) after the -1
    if k_green || k_up {
        num += 1;
    }
    let exit_ad = gf(g, "exit_ad");
    let ad_exit: Vec<bool> = (0..n).map(|i| k_ad && if short { ratio[i] <= exit_ad } else { ratio[i] >= exit_ad }).collect();
    if k_ad {
        num = 1;
    }
    let live = !on(g, "exit_at");
    let (pb_pct, hsl_pct, buf) = (gf(g, "pb"), gf(g, "hsl"), gf(g, "buffer"));
    // ---- the bar loop
    let mut pos = false;
    let mut cnt = 0i64;
    let (mut pb, mut hsl) = (NULL, NULL);
    let mut entry_i = usize::MAX;
    for i in 1..n {
        let enter = |s: &mut Sig, pos: &mut bool, cnt: &mut i64, pb: &mut f32, hsl: &mut f32| {
            let px = bars.c[i];
            if short {
                s.short[i] = true;
                s.shp[i] = px;
                *hsl = if hsl_pct > 0.0 { px * (1.0 + hsl_pct * 0.01) } else { NULL };
                *pb = if pb_pct > 0.0 { px * (1.0 - pb_pct * 0.01) } else { NULL };
            } else {
                s.buy[i] = true;
                s.bp[i] = px;
                *pb = if pb_pct > 0.0 { px * (1.0 + pb_pct * 0.01) } else { NULL };
                *hsl = if hsl_pct > 0.0 { px * (1.0 - hsl_pct * 0.01) } else { NULL };
            }
            *pos = true;
            *cnt = 0;
        };
        let can_enter = |pos: bool| !pos && main[i] && rsi_ok[i] && st_entry(i) && (!prev_on || main2[i - 1]);
        if entry_first && can_enter(pos) {
            enter(&mut s, &mut pos, &mut cnt, &mut pb, &mut hsl);
            entry_i = i;
        }
        if pos && (!entry_first || entry_i != i) {
            let (c, h, l) = (bars.c[i], bars.h[i], bars.l[i]);
            let (hsl_hit, pb_hit) = if short && dir_stops {
                (hsl_pct > 0.0 && (if live { h } else { c }) > hsl, pb_pct > 0.0 && (if live { l } else { c }) < pb)
            } else {
                (hsl_pct > 0.0 && (if live { l } else { c }) <= hsl, pb_pct > 0.0 && (if live { h } else { c }) >= pb)
            };
            if k_green && bars.o[i] < c {
                cnt += 1;
            }
            if k_up && c > bars.c[i - 1] {
                cnt += 1;
            }
            let why = if k_n && cnt == num {
                Some("N candles")
            } else if ad_exit[i] {
                Some("AD ratio exit")
            } else if k_green && cnt == num {
                Some("Green candles")
            } else if k_up && cnt == num {
                Some("Up-close candles")
            } else if k_5dp && truthy(five[i]) {
                Some("5DP day")
            } else if st_exit(i) {
                Some("ST filter")
            } else if i == n - 1 {
                Some("Last bar")
            } else if hsl_hit {
                Some("HSL")
            } else if pb_hit {
                Some("PB")
            } else {
                None
            };
            if let Some(w) = why {
                if short {
                    s.cover[i] = true;
                    s.cp[i] = short_exit_px(bars, i, live, pb_hit, hsl_hit, pb, hsl, buf);
                } else {
                    s.sell[i] = true;
                    s.sp[i] = long_exit_px(bars, i, live, pb_hit, hsl_hit, pb, hsl, buf);
                }
                s.why[i] = if hsl_hit && live { "HSL" } else if pb_hit && live && !hsl_hit { "PB" } else { w };
                pb = NULL;
                hsl = NULL;
                pos = false;
            }
            if k_n {
                cnt += 1;
            }
        }
        if !entry_first && can_enter(pos) {
            enter(&mut s, &mut pos, &mut cnt, &mut pb, &mut hsl);
        }
    }
    s
}

/// ADR Long 2-11 and 5DP(.afl / ROC.afl): long only, trades only inside the "2-11" patch (from `Entry Day` bars after
/// a month end to `Exit Day` bars later) or the 5DP patch (from the `Entry Day`-th last bar of a month to `Exit Day`
/// bars into the next); optional ADR / RSI / ADX / (ROC) / ST filters. The entry is checked before the exit on a bar.
fn patch(bars: &Bars, afl: Afl, g: G) -> Sig {
    let n = bars.c.len();
    let mut s = Sig::new(n);
    let ratio = ad_ratio(bars, false);
    let cmp = |v: &[f32], above: bool, thr: f32| -> Vec<bool> { v.iter().map(|&x| if above { gt(x, thr) } else { lt(x, thr) }).collect() };
    let rsi_ok = if on(g, "rsi_on") {
        let pd = gi(g, "rsi_pd");
        let v = if pd > 1 { rsi(bars.c, pd as usize) } else { vec![NULL; n] };
        cmp(&v, on(g, "rsi_dir"), gf(g, "rsi_thr"))
    } else {
        vec![true; n]
    };
    let adx_ok = if on(g, "adx_on") { cmp(&adx(bars.h, bars.l, bars.c, gi(g, "adx_pd").max(1) as usize), on(g, "adx_dir"), gf(g, "adx_thr")) } else { vec![true; n] };
    let roc_ok = if afl == Afl::PatchRoc && on(g, "roc_on") { cmp(&roc(bars.c, gi(g, "roc_pd").max(1) as usize), on(g, "roc_dir"), gf(g, "roc_thr")) } else { vec![true; n] };
    let st = if on(g, "st_on") { Some((st_bull(bars, gi(g, "st_tf").clamp(0, 2) as u8, gi(g, "st_pd").max(1) as usize, gf(g, "st_fact"), gi(g, "st_avg").clamp(0, 7) as u8), on(g, "st_dir"))) } else { None };
    let thr = gf(g, "thr");
    let sig: Vec<bool> = ratio.iter().map(|&x| x <= thr).collect();
    let main = main_entry(&sig, on(g, "consec"), gi(g, "lookback"), gi(g, "n_entry"));
    let adr_on = on(g, "adr_on");
    let kind = gi(g, "exit_type");
    let mut num = gi(g, "exit_n") - 1;
    let exit_ad = gf(g, "exit_ad");
    if kind == 1 {
        num = 1;
    }
    let me = month_end(bars.d);
    let (ed, xd) = (gi(g, "entry_day"), gi(g, "exit_day"));
    let e5 = ref_(&me, ed - 1);
    let x5 = ref_(&me, -xd);
    let e2 = ref_(&me, -ed);
    let x2 = ref_(&e2, -xd);
    let five = on(g, "patch");
    let mut ps = vec![false; n];
    let mut flag = false;
    for i in 1..n {
        if truthy(if five { e5[i] } else { e2[i] }) && !flag {
            ps[i] = true;
            flag = true;
        }
        if flag {
            ps[i] = true;
        }
        if truthy(if five { x5[i] } else { x2[i] }) && flag {
            ps[i] = true;
            flag = false;
        }
    }
    if afl == Afl::PatchRoc && (kind == 2 || kind == 3) {
        for i in 1..n {
            if truthy(if five { x5[i] } else { x2[i] }) {
                ps[i] = false;
            }
        }
    }
    let short_type = on(g, "entry_type");
    let live = !on(g, "exit_at");
    let (pb_pct, hsl_pct, buf) = (gf(g, "pb"), gf(g, "hsl"), gf(g, "buffer"));
    let mut pos = false;
    let mut cnt = 0i64;
    let (mut pb, mut hsl) = (NULL, NULL);
    for i in 1..n {
        let st_entry = st.as_ref().map_or(true, |(bull, bear)| bull[i] == Some(!*bear));
        if !pos && (!adr_on || main[i]) && rsi_ok[i] && adx_ok[i] && roc_ok[i] && st_entry && ps[i] {
            let px = bars.c[i];
            s.buy[i] = true;
            s.bp[i] = px;
            pb = if pb_pct > 0.0 { px * (1.0 + pb_pct * 0.01) } else { NULL };
            hsl = if hsl_pct > 0.0 { px * (1.0 - hsl_pct * 0.01) } else { NULL };
            pos = true;
            cnt = 0;
        }
        if pos {
            let (c, h, l) = (bars.c[i], bars.h[i], bars.l[i]);
            let (hsl_hit, pb_hit) = if short_type {
                (hsl_pct > 0.0 && (if live { h } else { c }) > hsl, pb_pct > 0.0 && (if live { l } else { c }) < pb)
            } else {
                (hsl_pct > 0.0 && (if live { l } else { c }) <= hsl, pb_pct > 0.0 && (if live { h } else { c }) >= pb)
            };
            let st_exit = st.as_ref().map_or(false, |(bull, bear)| bull[i] == Some(*bear));
            let why = if kind == 0 && cnt == num {
                Some("N candles")
            } else if kind == 1 && ratio[i] >= exit_ad {
                Some("AD ratio exit")
            } else if kind == 2 && truthy(x5[i]) {
                Some("5DP exit day")
            } else if kind == 3 && truthy(x2[i]) {
                Some("2-11 exit day")
            } else if st_exit {
                Some("ST filter")
            } else if i == n - 1 {
                Some("Last bar")
            } else if hsl_hit {
                Some("HSL")
            } else if pb_hit {
                Some("PB")
            } else {
                None
            };
            if let Some(w) = why {
                s.sell[i] = true;
                s.sp[i] = long_exit_px(bars, i, live, pb_hit, hsl_hit, pb, hsl, buf);
                s.why[i] = if hsl_hit && live { "HSL" } else if pb_hit && live { "PB" } else { w };
                pb = NULL;
                hsl = NULL;
                pos = false;
            }
            if kind == 0 {
                cnt += 1;
            }
        }
    }
    s
}

/// FIVE Day Phenomena with ADR.afl: buy at the close of the `Entry Day`-th last bar of a month when the ADR entry test
/// holds (ratio <= threshold on N consecutive bars, or N times within the look-back), sell `Exit Day` bars after the
/// month end, at the profit target or at the hard stop; optional ATHC / ST / Bull-Bear, RSI, ADX, ROC and candle filters.
fn five_dp(bars: &Bars, g: G) -> Sig {
    let n = bars.c.len();
    let mut s = Sig::new(n);
    let (o, h, l, c) = (bars.o, bars.h, bars.l, bars.c);
    let dead_yes = gi(g, "deadstock") == 0;
    let dead_buy: Vec<bool> = (0..n).map(|i| if dead_yes { i + 1 != n } else { i != 0 }).collect();
    let dead_sell: Vec<bool> = (0..n).map(|i| dead_yes && i + 1 == n).collect();
    // UC = H==C==L==O and LC alike: AmiBroker evaluates them left to right, ((H==C)==L)==O
    let uc: Vec<bool> = (0..n).map(|i| (b(b(h[i] == c[i]) == l[i])) == o[i]).collect();
    let lc = uc.clone();
    let penny = gf(g, "penny");
    let turnover_ok = !on(g, "turn"); // OpenInt (turnover) is 0 on an index: the filter blocks every entry when on
    let buf = gf(g, "buffer");
    let (pt_pct, hsl_pct) = (gf(g, "pt"), gf(g, "hsl"));
    // ---- ADR entry test
    let ratio = ad_ratio(bars, false);
    let thr = gf(g, "thr");
    let check: Vec<f32> = ratio.iter().map(|&x| b(x <= thr)).collect();
    let need = gi(g, "n_entry");
    let win = if gi(g, "entry_mode") == 0 { need } else { gi(g, "lookback") };
    let adr_sum = sum(&check, win.max(1) as usize);
    let adr_entry: Vec<f32> = adr_sum.iter().map(|&x| if is_null(x) { NULL } else { b(x >= need as f32) }).collect();
    // ---- the 5DP days
    let me = month_end(bars.d);
    let buy_day = ref_(&me, gi(g, "entry_day") - 1);
    let sell_day = ref_(&me, -gi(g, "exit_day"));
    let buysig: Vec<bool> = (0..n).map(|i| truthy(buy_day[i]) && truthy(adr_entry[i])).collect();
    // ---- candle colour
    let cd = gi(g, "candle_days").max(1) as usize;
    let candle = gi(g, "candle");
    let candle_ok: Vec<bool> = if candle == 2 {
        vec![true; n]
    } else {
        let col: Vec<f32> = (0..n).map(|i| b(if candle == 0 { c[i] > o[i] } else { o[i] > c[i] })).collect();
        sum(&col, cd).iter().map(|&x| x == cd as f32).collect()
    };
    // ---- filters
    let ft = gi(g, "filter");
    let (tr1, _, _) = pct_swing(c, gf(g, "athc_exit"), gf(g, "athc_exit"));
    let mut hcl = vec![0f32; n];
    for i in 0..n {
        hcl[i] = if i == 0 { c[0] } else { hcl[i - 1].max(c[i]) };
    }
    let mut athc = vec![false; n];
    for i in 1..n {
        athc[i] = if c[i] > hcl[i - 1] { true } else if tr1[i] == -1 && athc[i - 1] { false } else { athc[i - 1] };
    }
    let mt = mtrend(bars, gi(g, "flt_tf").clamp(0, 2) as u8, gi(g, "st_point").clamp(0, 2) as u8, gi(g, "flt_pd").max(1) as usize, gf(g, "flt_fact"));
    let (_, up_tf, _) = pct_swing(c, gf(g, "bull"), gf(g, "bear"));
    let buy_filter: Vec<bool> = (0..n)
        .map(|i| {
            let a = ft != 1 || athc[i];
            let st = ft != 2 || mt[i] == 1.0;
            let sw = ft != 3 || (gt(c[i], up_tf[i]) && gt(up_tf[i], 0.0));
            a && st && sw
        })
        .collect();
    let sell_filter: Vec<bool> = (0..n).map(|i| (ft == 1 && !athc[i]) || (ft == 2 && mt[i] == -1.0 && i > 0 && mt[i - 1] == 1.0)).collect();
    let rsi_ok: Vec<bool> = if on(g, "rsi_on") { rsi(c, gi(g, "rsi_pd").max(1) as usize).iter().map(|&x| gt(x, gf(g, "rsi_val"))).collect() } else { vec![true; n] };
    let adx_ok: Vec<bool> = if on(g, "adx_on") { adx(h, l, c, gi(g, "adx_pd").max(1) as usize).iter().map(|&x| gt(x, gf(g, "adx_val"))).collect() } else { vec![true; n] };
    let roc_ok: Vec<bool> = if on(g, "roc_on") {
        let below = on(g, "roc_ud");
        let thr = gf(g, "roc_thr");
        roc(c, gi(g, "roc_day").max(1) as usize).iter().map(|&x| if below { lt(x, -thr) } else { gt(x, thr) }).collect()
    } else {
        vec![true; n]
    };
    let cash_ok: Vec<bool> = (0..n).map(|i| !uc[i] && buy_filter[i] && turnover_ok && c[i] > penny && dead_buy[i]).collect();
    let sellsig: Vec<bool> = (0..n).map(|i| truthy(sell_day[i]) || sell_filter[i]).collect();
    // ---- the loop (one template per RSI / ADX / ROC combination in the AFL; the combination only adds its tests)
    let mut position = false;
    let mut skip = false;
    let mut hsl = 0f32;
    let mut pt = 9_999_999f32;
    for i in 1..n {
        if !position && buysig[i] && cash_ok[i] && candle_ok[i] && rsi_ok[i] && adx_ok[i] && roc_ok[i] {
            s.buy[i] = true;
            s.bp[i] = c[i];
            if pt_pct > 0.0 {
                pt = (1.0 + pt_pct * 0.01) * c[i];
            }
            if hsl_pct > 0.0 {
                hsl = (1.0 - hsl_pct * 0.01) * c[i];
            }
            position = true;
        } else if position && !skip {
            if l[i] <= hsl || h[i] >= pt || sellsig[i] || dead_sell[i] {
                if !lc[i] || dead_sell[i] {
                    s.sell[i] = true;
                    if l[i] <= hsl {
                        s.sp[i] = if o[i] < hsl { o[i] - o[i] * buf / 100.0 } else { hsl };
                        s.why[i] = "Hard SL";
                    } else if h[i] >= pt {
                        s.sp[i] = if o[i] > pt { o[i] - o[i] * buf / 100.0 } else { pt };
                        s.why[i] = "Profit booking";
                    } else {
                        s.sp[i] = c[i];
                        s.why[i] = if truthy(sell_day[i]) { "5DP exit day" } else if dead_sell[i] { "Last bar" } else { "Filter exit" };
                    }
                    position = false;
                    hsl = 0.0;
                    pt = 9_999_999.0;
                } else {
                    skip = true;
                }
            }
        } else if skip && (!lc[i] || dead_sell[i]) {
            s.sell[i] = true;
            s.sp[i] = c[i];
            s.why[i] = "After lower circuit";
            position = false;
            skip = false;
            hsl = 0.0;
            pt = 9_999_999.0;
        }
    }
    s
}

/// The AFL's signal arrays over every bar.
pub fn signals(bars: &Bars, afl: Afl, g: G) -> Sig {
    match afl {
        Afl::LongShort => long_short(bars, g),
        Afl::Versions | Afl::FiveDayExit | Afl::WoRsi | Afl::WoRsiNDay | Afl::TwoSt => versions(bars, afl, g),
        Afl::Patch | Afl::PatchRoc => patch(bars, afl, g),
        Afl::FiveDp => five_dp(bars, g),
    }
}

/// AmiBroker's backtester on the signal arrays inside [from, to]: on each bar exits first (Sell / Cover), then entries
/// (Buy before Short). A position still open at `to` is marked open at that bar's close.
pub fn trades(bars: &Bars, s: &Sig, from: usize, to: usize) -> Vec<Tr> {
    let mut out = Vec::new();
    let mut cur: Option<Tr> = None;
    for i in from..=to.min(bars.c.len().saturating_sub(1)) {
        if let Some(t) = cur.as_mut() {
            let exit = if t.dir > 0.0 { s.sell[i].then(|| s.sp[i]) } else { s.cover[i].then(|| s.cp[i]) };
            if let Some(px) = exit {
                t.xi = i;
                t.xp = px;
                t.why = if s.why[i].is_empty() { "Exit" } else { s.why[i] };
                out.push(cur.take().unwrap());
            }
        }
        if cur.is_none() {
            if s.buy[i] {
                cur = Some(Tr { dir: 1.0, ei: i, ep: s.bp[i], xi: i, xp: s.bp[i], why: "", open: false });
            } else if s.short[i] {
                cur = Some(Tr { dir: -1.0, ei: i, ep: s.shp[i], xi: i, xp: s.shp[i], why: "", open: false });
            }
        }
    }
    if let Some(mut t) = cur {
        t.xi = to;
        t.xp = bars.c[to];
        t.why = "open";
        t.open = true;
        out.push(t);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Data {
        d: Vec<i32>,
        o: Vec<f32>,
        h: Vec<f32>,
        l: Vec<f32>,
        c: Vec<f32>,
        adv: Vec<f32>,
        dec: Vec<f32>,
    }
    impl Data {
        fn bars(&self) -> Bars<'_> {
            Bars { d: &self.d, o: &self.o, h: &self.h, l: &self.l, c: &self.c, adv: &self.adv, dec: &self.dec }
        }
    }
    /// Weekdays from Monday 2024-01-01 with closes `c` (open = previous close, high/low +-1) and advances `a` (declines 50 - a).
    fn data(c: &[f32], a: &[f32]) -> Data {
        let mut d = Vec::new();
        let mut day = days_from_civil(2024, 1, 1);
        while d.len() < c.len() {
            let wd = (day + 3).rem_euclid(7); // 0 = Monday
            if wd < 5 {
                let z = day + 719468;
                let era = z.div_euclid(146097);
                let doe = z - era * 146097;
                let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
                let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
                let mp = (5 * doy + 2) / 153;
                let dd = doy - (153 * mp + 2) / 5 + 1;
                let m = if mp < 10 { mp + 3 } else { mp - 9 };
                let y = yoe + era * 400 + i64::from(m <= 2);
                d.push((y * 10000 + m * 100 + dd) as i32);
            }
            day += 1;
        }
        let o: Vec<f32> = (0..c.len()).map(|i| if i == 0 { c[0] } else { c[i - 1] }).collect();
        Data {
            d,
            h: (0..c.len()).map(|i| c[i].max(o[i]) + 1.0).collect(),
            l: (0..c.len()).map(|i| c[i].min(o[i]) - 1.0).collect(),
            o,
            c: c.to_vec(),
            dec: a.iter().map(|x| 50.0 - x).collect(),
            adv: a.to_vec(),
        }
    }
    fn params(kv: &'static [(&'static str, f64)]) -> impl Fn(&str) -> f64 {
        move |k: &str| kv.iter().find(|(x, _)| *x == k).map_or(0.0, |(_, v)| *v)
    }

    #[test]
    fn ratio_uses_fifty_or_the_advances_when_declines_are_zero() {
        let x = data(&[10.0, 10.0], &[20.0, 50.0]);
        let r = ad_ratio(&x.bars(), false);
        assert_eq!(r, vec![20.0 / 30.0, 50.0]);
        assert_eq!(ad_ratio(&x.bars(), true)[1], 50.0);
    }

    /// Versions, consecutive OFF / look-back 1 (the AFL's loop starts at bar 2): buy at the close when the ratio <= 0.5,
    /// exit at the close 3 bars later.
    #[test]
    fn versions_n_candle_exit_counts_from_the_bar_after_entry() {
        let x = data(&[100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0], &[10.0, 10.0, 10.0, 30.0, 30.0, 30.0, 30.0]);
        let g = params(&[("thr", 0.5), ("n_entry", 1.0), ("lookback", 1.0), ("exit_n", 3.0)]);
        let s = signals(&x.bars(), Afl::Versions, &g);
        let t = trades(&x.bars(), &s, 0, 6);
        assert_eq!((t[0].ei, t[0].ep, t[0].xi, t[0].xp), (2, 102.0, 5, 105.0));
    }

    /// Without RSI checks the entry first and never exits on the entry bar; an up-close count of 2 exits on the 2nd up close.
    #[test]
    fn without_rsi_up_close_exit() {
        let x = data(&[100.0, 101.0, 100.0, 102.0, 101.0, 103.0, 104.0], &[30.0, 30.0, 10.0, 30.0, 30.0, 30.0, 30.0]);
        let g = params(&[("thr", 0.5), ("n_entry", 1.0), ("lookback", 1.0), ("exit_type", 3.0), ("exit_n", 2.0)]);
        let t = trades(&x.bars(), &signals(&x.bars(), Afl::WoRsi, &g), 0, 6);
        assert_eq!((t[0].ei, t[0].xi), (2, 5));
    }

    /// Short in the Without RSI AFL reads the undefined Close0: no trades and an error.
    #[test]
    fn without_rsi_short_is_an_afl_error() {
        let x = data(&[100.0; 4], &[40.0; 4]);
        let g = params(&[("entry_type", 1.0), ("thr", 0.5)]);
        let s = signals(&x.bars(), Afl::WoRsi, &g);
        assert!(s.error.is_some() && !s.short.iter().any(|&v| v));
    }

    /// HSL 5 % live: the low reaches the stop, the open did not -> fill at the stop.
    #[test]
    fn hard_stop_fills_at_the_level() {
        let mut x = data(&[100.0, 100.0, 100.0, 100.0, 100.0], &[30.0, 30.0, 10.0, 30.0, 30.0]);
        x.l[3] = 90.0;
        let g = params(&[("thr", 0.5), ("n_entry", 1.0), ("lookback", 1.0), ("hsl", 5.0)]);
        let t = trades(&x.bars(), &signals(&x.bars(), Afl::Versions, &g), 0, 4);
        assert_eq!((t[0].xi, t[0].xp), (3, 95.0));
    }

    #[test]
    fn weekly_st_expands_with_the_completed_week_on_its_last_day() {
        let k = Comp { start: vec![0, 3], end: vec![2, 4], o: vec![], h: vec![], l: vec![], c: vec![] };
        let e = ref_(&expand_first(&ref_(&[1.0, 2.0], -1), &k, 5), 1);
        assert_eq!(e, vec![NULL, NULL, 1.0, 1.0, NULL]);
    }

    #[test]
    fn exits_before_entries_on_a_bar() {
        let x = data(&[1.0, 2.0, 3.0, 4.0], &[25.0; 4]);
        let mut s = Sig::new(4);
        s.buy[1] = true;
        s.bp[1] = 2.0;
        s.sell[1] = true;
        s.sp[1] = 2.0;
        s.sell[3] = true;
        s.sp[3] = 4.0;
        let t = trades(&x.bars(), &s, 0, 3);
        assert_eq!((t.len(), t[0].ei, t[0].xi), (1, 1, 3));
    }
}
