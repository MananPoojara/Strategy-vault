//! ATH engines: ATHC entry with a plain exit: 1 or 2 lower closes / lows, previous day(s) low, lower highs, weekly low or close.
//! One `VariantDef` per sub-variant (see `ath_core::VariantDef`); registered in `ath_registry.rs`. Standard library only: depends on
//! `ath_core` only so a verification driver can include it with `#[path]`. Every function below is a statement-by-statement copy of
//! the team's AFL (float32 arithmetic, same order of operations, same quirks); results of the check against the team sheets are in
//! `ATH/verify/<variant id>/RESULT.md`.
#![allow(dead_code)]
#[allow(unused_imports)]
use crate::ath_core::*;

// ---------------------------------------------------------------- shared pieces

struct Px {
    d: Vec<i32>,
    c: Vec<f32>,
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    v: Vec<f32>,
    oi: Vec<f32>,
}

fn px(s: &Series) -> Px {
    Px {
        d: s.bars.iter().map(|b| b.date).collect(),
        c: s.close(),
        o: s.open(),
        h: s.high(),
        l: s.low(),
        v: s.volume(),
        oi: s.openint(),
    }
}

/// `Highest_Close = HHV(C, BarCount)`: highest close up to the bar.
fn hhv_all(c: &[f32]) -> Vec<f32> {
    let mut out = vec![0f32; c.len()];
    for i in 0..c.len() {
        out[i] = if i == 0 || c[i] > out[i - 1] { c[i] } else { out[i - 1] };
    }
    out
}

/// Turnover table of the "Volsig" AFLs (compared with `OpenInt` = Close x Volume of the converted database).
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

/// The other turnover table (the two "2 lower close" AFLs): compared with `V * C`.
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

/// The %swing trend (`trend1`) of the AFLs, float32 like the AFL arrays.
fn swing_trend(c: &[f32], exit_pct: f32) -> Vec<i8> {
    let n = c.len();
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
    trend
}

/// Common AFL options.
#[derive(Clone, Copy)]
struct Common {
    /// DEAD STOCK BACKTEST = YES (toggle 0)
    dead: bool,
    penny: f32,
    turnover_flt: bool,
}

fn common(g: G) -> Common {
    Common { dead: g("deadstock").round() == 0.0, penny: g("pennyflt") as f32, turnover_flt: g("turnflt").round() == 1.0 }
}

/// AmiBroker `Prec(x, 2)` as the team sheets show it: the float32 value taken as a double, times 100, TRUNCATED (not rounded).
/// (`ath_core::prec2` rounds; it differs for prices such as 12.60 / 12.61, which AmiBroker's Prec treats as equal because
/// 12.61f = 12.60999965...)
fn prec_t(x: f32) -> i64 {
    ((x as f64) * 100.0).floor() as i64
}

struct Base {
    x: Px,
    hc: Vec<f32>,
    upc: Vec<bool>,
    loc: Vec<bool>,
    uc: Vec<bool>,
    lc: Vec<bool>,
    dead_buy: Vec<bool>,
    dead_sell: Vec<bool>,
    turnover: Vec<bool>,
}

fn base(s: &Series, cm: &Common, vc_table: bool) -> Base {
    let x = px(s);
    let n = x.c.len();
    let hc = hhv_all(&x.c);
    let mut b = Base { hc, upc: vec![false; n], loc: vec![false; n], uc: vec![false; n], lc: vec![false; n], dead_buy: vec![false; n], dead_sell: vec![false; n], turnover: vec![true; n], x };
    for i in 0..n {
        let (o, c, h, l) = (b.x.o[i], b.x.c[i], b.x.h[i], b.x.l[i]);
        b.upc[i] = prec_t(o) == prec_t(c) && prec_t(o) == prec_t(h);
        b.loc[i] = prec_t(o) == prec_t(c) && prec_t(o) == prec_t(l);
        b.uc[i] = h == c;
        b.lc[i] = l == c;
        if cm.dead {
            b.dead_buy[i] = i != n - 1;
            b.dead_sell[i] = i == n - 1;
        } else {
            b.dead_buy[i] = i != 0;
        }
        if cm.turnover_flt {
            let y = b.x.d[i] / 10000;
            b.turnover[i] = if vc_table { (b.x.v[i] * c) > volsig_vc(y) } else { b.x.oi[i] > volsig(y) };
        }
    }
    b
}

/// AmiBroker keeps the fill price inside the bar's low-high range.
fn finish(b: &Base, buy: Vec<bool>, sell: Vec<bool>, mut bp: Vec<f32>, mut sp: Vec<f32>) -> Signals {
    for i in 0..bp.len() {
        bp[i] = bp[i].min(b.x.h[i]).max(b.x.l[i]);
        sp[i] = sp[i].min(b.x.h[i]).max(b.x.l[i]);
    }
    Signals::new(buy, sell, bp, sp)
}

// ---------------------------------------------------------------- 1. ATHC & %Swing with 1 Lower Close Or Lower Low Exit

pub fn lower_close_or_low(s: &Series, cm: &Common, exit_pct: f32, lower_close: bool) -> Signals {
    let b = base(s, cm, false);
    let n = b.x.c.len();
    let (c, l) = (&b.x.c, &b.x.l);
    let trend = swing_trend(c, exit_pct);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut athc = vec![false; n];
    let (mut position, mut close_at_buy, mut low_at_buy) = (false, 0f32, 0f32);
    for i in 1..n {
        if c[i] > b.hc[i - 1] {
            athc[i] = true;
        } else if trend[i] == -1 && athc[i - 1] {
            athc[i] = false;
        } else {
            athc[i] = athc[i - 1];
        }
        let filter = b.dead_buy[i] && c[i] > cm.penny && b.turnover[i] && !b.uc[i];
        if !position && (athc[i] || c[i] > b.hc[i - 1]) && filter {
            buy[i] = true;
            close_at_buy = c[i];
            low_at_buy = l[i];
            position = true;
        }
        if position && lower_close {
            if (c[i] < close_at_buy && !b.lc[i]) || b.dead_sell[i] {
                sell[i] = true;
                position = false;
                close_at_buy = 0.0;
                low_at_buy = 0.0;
                athc[i] = false;
            }
        } else if position && !lower_close {
            if (l[i] < low_at_buy && !b.lc[i]) || b.dead_sell[i] {
                sell[i] = true;
                position = false;
                close_at_buy = 0.0;
                low_at_buy = 0.0;
                athc[i] = false;
            }
        }
        if position && c[i] > close_at_buy {
            close_at_buy = c[i];
        }
        if position && l[i] > low_at_buy {
            low_at_buy = l[i];
        }
    }
    // the AFL sets `Sell[i] = deadstocksellsig` on the last bar even when flat; the backtester ignores a sell without a position
    finish(&b, buy, sell, c.clone(), c.clone())
}

// ---------------------------------------------------------------- 2. ATHC with 2 continuous Lower Close exit

pub fn two_continuous(s: &Series, cm: &Common) -> Signals {
    let b = base(s, cm, true);
    let n = b.x.c.len();
    let c = &b.x.c;
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut count = vec![0i32; n];
    let mut buysig = vec![false; n];
    let (mut position, mut price_at_buy, mut lower_close, mut sell_cnd, mut temp) = (false, 0f32, 0f32, false, false);
    for i in 1..n {
        if position && c[i] > price_at_buy && count[i - 1] >= 0 {
            price_at_buy = c[i];
            lower_close = 0.0;
            count[i] = 0;
        } else if position && c[i] < price_at_buy && count[i - 1] == 0 {
            count[i] = count[i - 1] + 1;
            lower_close = c[i];
        } else if position && c[i] > lower_close && count[i - 1] == 1 {
            count[i] = count[i - 1];
            lower_close = c[i];
        } else if position && (c[i] < lower_close || c[i] == lower_close) && count[i - 1] == 1 {
            count[i] = 0;
            price_at_buy = 0.0;
            lower_close = 0.0;
            sell_cnd = true;
            temp = true;
        }
        if c[i] > b.hc[i - 1] {
            buysig[i] = true;
        } else if temp && buysig[i - 1] {
            buysig[i] = false;
            temp = false;
        } else {
            buysig[i] = buysig[i - 1];
        }
        if !position && (buysig[i] || c[i] > b.hc[i - 1]) && !b.upc[i] && b.dead_buy[i] && c[i] > cm.penny && b.turnover[i] {
            buy[i] = true;
            price_at_buy = c[i];
            position = true;
        }
        if (position && (sell_cnd || !buysig[i]) && !b.loc[i]) || b.dead_sell[i] {
            sell[i] = true;
            position = false;
            price_at_buy = 0.0;
            lower_close = 0.0;
            sell_cnd = false;
        }
    }
    finish(&b, buy, sell, c.clone(), c.clone())
}

// ---------------------------------------------------------------- 3. ATHC with 2 non-continuous Lower Close exit

pub fn two_non_continuous(s: &Series, cm: &Common) -> Signals {
    let b = base(s, cm, true);
    let n = b.x.c.len();
    let c = &b.x.c;
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut abc = vec![0i32; n];
    let mut temp = vec![0i32; n];
    let mut buysig = vec![false; n];
    let (mut position, mut price_at_buy, mut count_close) = (false, 0f32, 0f32);
    for i in 1..n {
        if position && c[i] > b.hc[i - 1] && abc[i - 1] >= 0 {
            price_at_buy = c[i];
            count_close = 0.0;
            abc[i] = 0;
        } else if position && c[i] < price_at_buy && abc[i - 1] == 0 {
            abc[i] = abc[i - 1] + 1;
            count_close = c[i];
        } else if position && c[i] < price_at_buy && c[i] == count_close && abc[i - 1] >= 0 {
            abc[i] = abc[i - 1] + 1;
            count_close = c[i];
        } else if abc[i - 1] == 1 && c[i] < count_close {
            abc[i] = abc[i - 1] + 1;
            temp[i] = 2;
            price_at_buy = 0.0;
            count_close = 0.0;
        } else {
            abc[i] = abc[i - 1];
        }
        if c[i] > b.hc[i - 1] {
            buysig[i] = true;
        } else if temp[i] == 2 && buysig[i - 1] {
            buysig[i] = false;
            temp[i] = 0;
        } else {
            buysig[i] = buysig[i - 1];
        }
        if !position && (buysig[i] || c[i] > b.hc[i - 1]) && !b.upc[i] && b.dead_buy[i] && c[i] > cm.penny && b.turnover[i] {
            buy[i] = true;
            price_at_buy = c[i];
            position = true;
        }
        if (position && (abc[i] == 2 || !buysig[i]) && !b.loc[i]) || b.dead_sell[i] {
            sell[i] = true;
            abc[i] = 0;
            price_at_buy = 0.0;
            count_close = 0.0;
            position = false;
            buysig[i] = false;
        }
    }
    finish(&b, buy, sell, c.clone(), c.clone())
}

// ---------------------------------------------------------------- 4-6. previous day(s) low / lower high(s) exits

#[derive(Clone, Copy)]
struct StopKind {
    /// first loop index of the AFL (1, 2 or 3)
    start: usize,
    /// use High instead of Low
    high: bool,
    /// two bars (min of the two) instead of one
    two: bool,
    /// the `deadstocksellsig` is part of the exit condition (low AFLs); the high AFLs do not have it there
    dead_in_cond: bool,
    /// the exit is written as `else if(Position>0 AND SkipSell==0){ ...if cond{...} }` (the 2-bar AFLs) instead of
    /// `else if(Position>0 AND SkipSell==0 AND cond)`
    nested: bool,
    /// the first buy condition also accepts the running ATHC state (no AFL does: false)
    ///
    /// (kept to document the difference with the commented-out `/*OR ATHCBuysig[i]*/`)
    _unused: bool,
}

pub fn stop_exit(s: &Series, cm: &Common, exit_pct: f32, buf_buy: f32, buf_sell: f32, k: StopKind) -> Signals {
    let b = base(s, cm, false);
    let n = b.x.c.len();
    let (c, o, h, l) = (&b.x.c, &b.x.o, &b.x.h, &b.x.l);
    let r = if k.high { h } else { l };
    let trend = swing_trend(c, exit_pct);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut bp = c.clone();
    let mut sp = c.clone();
    let mut athc = vec![false; n];
    for i in 1..n {
        if c[i] > b.hc[i - 1] {
            athc[i] = true;
        } else if trend[i] == -1 && athc[i - 1] {
            athc[i] = false;
        } else {
            athc[i] = athc[i - 1];
        }
    }
    let (mut position, mut price_at_buy, mut lw, mut lw2) = (false, 0f32, 0f32, 0f32);
    let (mut skipbuy, mut skipsell) = (false, false);
    for i in k.start..n {
        let filter = b.turnover[i] && c[i] > cm.penny && b.dead_buy[i];
        if position && c[i] > price_at_buy {
            price_at_buy = c[i];
            lw = r[i - 1];
            if k.two {
                lw2 = r[i - 2];
            }
        }
        if !position && c[i] > b.hc[i - 1] && !skipbuy && filter {
            if !b.uc[i] {
                buy[i] = true;
                position = true;
                price_at_buy = c[i];
                lw = r[i - 1];
                if k.two {
                    lw2 = r[i - 2];
                }
            } else {
                skipbuy = true;
            }
        } else if !position && athc[i] && !b.upc[i] && skipbuy && filter {
            buy[i] = true;
            let lim = o[i] + (o[i] * buf_buy) / 100.0;
            if o[i] == h[i] {
                bp[i] = o[i];
            } else if o[i] != h[i] && h[i] > lim {
                bp[i] = lim;
            } else if o[i] != h[i] && h[i] < lim {
                bp[i] = h[i];
            }
            price_at_buy = c[i];
            lw = r[i - 1];
            if k.two {
                lw2 = r[i - 2];
            }
            position = true;
            skipbuy = false;
        } else if !position && trend[i] == -1 {
            skipbuy = false;
        } else if k.nested {
            if position && !skipsell {
                let lowest = if k.two { lw.min(lw2) } else { lw };
                let cond = (k.dead_in_cond && b.dead_sell[i]) || c[i] < lowest || trend[i] == -1;
                if cond {
                    if !b.lc[i] || b.dead_sell[i] {
                        sell[i] = true;
                        position = false;
                        lw = 0.0;
                        lw2 = 0.0;
                        price_at_buy = 0.0;
                    } else {
                        skipsell = true;
                    }
                }
            } else if (position && !b.loc[i] && skipsell) || (position && b.dead_sell[i]) {
                sell[i] = true;
                sell_price_modelled(&mut sp, i, o, l, buf_buy, buf_sell);
                position = false;
                lw = 0.0;
                lw2 = 0.0;
                price_at_buy = 0.0;
                skipsell = false;
            }
        } else if position && !skipsell && ((k.dead_in_cond && b.dead_sell[i]) || c[i] < lw || trend[i] == -1) {
            if !b.lc[i] || b.dead_sell[i] {
                sell[i] = true;
                position = false;
                lw = 0.0;
                price_at_buy = 0.0;
            } else {
                skipsell = true;
            }
        } else if (position && !b.loc[i] && skipsell) || (position && b.dead_sell[i]) {
            sell[i] = true;
            sell_price_modelled(&mut sp, i, o, l, buf_buy, buf_sell);
            position = false;
            lw = 0.0;
            lw2 = 0.0;
            price_at_buy = 0.0;
            skipsell = false;
        }
    }
    finish(&b, buy, sell, bp, sp)
}

/// SellPrice of the delayed (lower circuit) exit, including the AFL's use of `Buffer_Pct` (the BUYING buffer) in the middle branch.
fn sell_price_modelled(sp: &mut [f32], i: usize, o: &[f32], l: &[f32], buf_buy: f32, buf_sell: f32) {
    let lim = o[i] - (o[i] * buf_sell) / 100.0;
    if o[i] == l[i] {
        sp[i] = o[i];
    } else if o[i] != l[i] && l[i] < lim {
        sp[i] = o[i] - (o[i] * buf_buy) / 100.0;
    } else if o[i] != l[i] && l[i] > lim {
        sp[i] = l[i];
    }
}

// ---------------------------------------------------------------- 7. weekly close / weekly low

/// Days since 1970-01-01 of a yyyymmdd date.
fn day_number(d: i32) -> i64 {
    let (y, m, dd) = ((d / 10000) as i64, (d / 100 % 100) as i64, (d % 100) as i64);
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + dd - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// `TimeFrameSet(inWeekly); x = <field>; y = Ref(<field>,-1); TimeFrameRestore(); TimeFrameExpand(..., inWeekly)`:
/// weekly bars are calendar weeks (Monday first) of the symbol's own daily bars. `expand_first` = false is AmiBroker's default
/// `expandLast`: the weekly value appears on the LAST daily bar of its week and is held until the next one.
fn weekly_pair(dates: &[i32], vals: &[f32], hi: bool, vals_hi: &[f32], expand_first: bool) -> (Vec<f32>, Vec<f32>) {
    // weekly bars
    let n = dates.len();
    let mut wk_of = vec![0usize; n];
    let mut wvals: Vec<f32> = Vec::new();
    let mut last_idx: Vec<usize> = Vec::new();
    let mut cur_key = i64::MIN;
    for i in 0..n {
        let key = (day_number(dates[i]) + 3).div_euclid(7);
        if key != cur_key {
            cur_key = key;
            wvals.push(vals[i]);
            last_idx.push(i);
        } else {
            let k = wvals.len() - 1;
            last_idx[k] = i;
            // weekly Low = lowest low, weekly Close = last close
            wvals[k] = if hi { wvals[k].min(vals_hi[i]) } else { vals[i] };
        }
        wk_of[i] = wvals.len() - 1;
    }
    let pre: Vec<f32> = (0..wvals.len()).map(|k| if k == 0 { f32::NAN } else { wvals[k - 1] }).collect();
    let expand = |w: &[f32]| -> Vec<f32> {
        (0..n)
            .map(|i| {
                let k = wk_of[i];
                if expand_first {
                    w[k]
                } else if last_idx[k] == i {
                    w[k]
                } else if k > 0 {
                    w[k - 1]
                } else {
                    f32::NAN
                }
            })
            .collect()
    };
    (expand(&wvals), expand(&pre))
}

pub fn weekly(s: &Series, cm: &Common, low_version: bool, expand_first: bool) -> Signals {
    let mut b = base(s, cm, false);
    let n = b.x.c.len();
    let c = b.x.c.clone();
    // Buy_Cash_Filter = UC == 0 AND Turnover AND Close > PennyFlt AND DEADSTOCKBUYSIG
    let filt: Vec<bool> = (0..n).map(|i| !b.uc[i] && b.turnover[i] && c[i] > cm.penny && b.dead_buy[i]).collect();
    let (wc, pwc) = weekly_pair(&b.x.d, &c, false, &c, expand_first);
    let (wl, pwl) = weekly_pair(&b.x.d, &b.x.l, true, &b.x.l, expand_first);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut position, mut skipbuy, mut skipsell) = (false, false, false);
    let mut wc1 = 0f32;
    for i in 1..n {
        if !low_version {
            if position && wc[i] > wc1 {
                wc1 = pwc[i];
            }
            if !position && c[i] > b.hc[i - 1] && wc[i] > pwc[i] && !skipbuy {
                if filt[i] {
                    buy[i] = true;
                    wc1 = if wc[i].is_nan() { c[i] } else { pwc[i] };
                    position = true;
                } else {
                    skipbuy = true;
                }
            } else if skipbuy && wc[i] < pwc[i] {
                skipbuy = false;
            } else if skipbuy && filt[i] {
                buy[i] = true;
                wc1 = pwc[i];
                position = true;
                skipbuy = false;
            } else if position && wc[i] < wc1 && !skipsell {
                if !b.lc[i] || b.dead_sell[i] {
                    sell[i] = true;
                    position = false;
                    skipsell = false;
                    wc1 = 0.0;
                } else {
                    skipsell = true;
                }
            } else if (position && !b.lc[i] && skipsell) || (position && b.dead_sell[i]) {
                sell[i] = true;
                position = false;
                skipsell = false;
                wc1 = 0.0;
            }
        } else {
            if !position && c[i] > b.hc[i - 1] && !skipbuy {
                if filt[i] {
                    buy[i] = true;
                    position = true;
                } else {
                    skipbuy = true;
                }
            } else if skipbuy && c[i] < pwl[i] {
                skipbuy = false;
            } else if skipbuy && filt[i] {
                buy[i] = true;
                position = true;
                skipbuy = false;
            } else if position && c[i] < pwl[i] && !skipsell {
                if !b.lc[i] || b.dead_sell[i] {
                    sell[i] = true;
                    position = false;
                    skipsell = false;
                } else {
                    skipsell = true;
                }
            } else if (position && !b.lc[i] && skipsell) || (position && b.dead_sell[i]) {
                sell[i] = true;
                position = false;
                skipsell = false;
            }
        }
    }
    let _ = &wl;
    b.x.c = c.clone();
    finish(&b, buy, sell, c.clone(), c)
}

// ---------------------------------------------------------------- the variants

fn common_params() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
    ]
}

fn schema_lower_close() -> Vec<P> {
    let mut v = common_params();
    // the AFL declares Param("EXIT %",10,11,18,1); only the swing line (it does not trigger the exit) uses it
    v.push(p("exitpct", "EXIT %", 10.0, 11.0, 18.0, 1.0));
    v.push(p("typeofexit", "Lower Close / Lower Low (0 Lower Low, 1 Lower Close)", 1.0, 0.0, 1.0, 1.0));
    v
}

fn sig_lower_close(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = common(g);
    let (e, lc) = (g("exitpct") as f32, g("typeofexit").round() == 1.0);
    series.iter().map(|s| lower_close_or_low(s, &cm, e, lc)).collect()
}

fn sig_cont(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = common(g);
    series.iter().map(|s| two_continuous(s, &cm)).collect()
}

fn sig_noncont(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = common(g);
    series.iter().map(|s| two_non_continuous(s, &cm)).collect()
}

fn schema_stop() -> Vec<P> {
    let mut v = common_params();
    v.push(p("buffer_pct", "Buying Buffer %", 0.25, 0.0, 10.0, 0.05));
    v.push(p("bufferpct", "Selling Buffer %", 0.25, 0.0, 10.0, 0.05));
    v.push(p("exitpct", "EXIT %", 10.0, 1.0, 40.0, 1.0));
    v
}

fn schema_high() -> Vec<P> {
    let mut v = schema_stop();
    v.push(p("ver", "Lower High Param (0 = 1 Lower High, 1 = 2 Lower High)", 0.0, 0.0, 1.0, 1.0));
    v
}

const LOW1: StopKind = StopKind { start: 1, high: false, two: false, dead_in_cond: true, nested: false, _unused: false };
const LOW2: StopKind = StopKind { start: 2, high: false, two: true, dead_in_cond: true, nested: true, _unused: false };
const HIGH1: StopKind = StopKind { start: 3, high: true, two: false, dead_in_cond: false, nested: false, _unused: false };
const HIGH2: StopKind = StopKind { start: 3, high: true, two: true, dead_in_cond: false, nested: true, _unused: false };

fn run_stop(series: &[Series], g: G, k: StopKind) -> Vec<Signals> {
    let cm = common(g);
    let (e, bb, bs) = (g("exitpct") as f32, g("buffer_pct") as f32, g("bufferpct") as f32);
    series.iter().map(|s| stop_exit(s, &cm, e, bb, bs, k)).collect()
}

fn sig_low2(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    run_stop(series, g, LOW2)
}

fn sig_low1(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    run_stop(series, g, LOW1)
}

fn sig_high(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    run_stop(series, g, if g("ver").round() == 1.0 { HIGH2 } else { HIGH1 })
}

fn schema_weekly() -> Vec<P> {
    let mut v = common_params();
    v.push(p("ver", "Entry/Exit Version (0 = Weekly Close Exit, 1 = Weekly Low Exit)", 0.0, 0.0, 1.0, 1.0));
    v
}

fn sig_weekly(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let cm = common(g);
    // TimeFrameExpand with AmiBroker's default mode (expandLast): verified against the sheets (expandFirst does not reproduce them)
    let (low, first) = (g("ver").round() == 1.0, false);
    series.iter().map(|s| weekly(s, &cm, low, first)).collect()
}

const NSE500: Data = Data::Cash { watchlist: "NSE 500 C&P.tls" };

pub fn variants() -> Vec<VariantDef> {
    vec![
        VariantDef { id: "ytath_and_pctswing_with_1_lower_close_or", data: NSE500, schema: schema_lower_close, signals: sig_lower_close },
        VariantDef { id: "ytath_with_2_continuous_lower_close_exit", data: NSE500, schema: common_params, signals: sig_cont },
        VariantDef { id: "ytath_with_2_non_continuous_lower_close_exit", data: NSE500, schema: common_params, signals: sig_noncont },
        VariantDef { id: "ytath_entry_and_close_less_than_previous_2", data: NSE500, schema: schema_stop, signals: sig_low2 },
        VariantDef { id: "ytath_entry_and_close_less_than_previous", data: NSE500, schema: schema_stop, signals: sig_low1 },
        VariantDef { id: "ytath_entry_and_exit_on_price_at_buy_below", data: NSE500, schema: schema_high, signals: sig_high },
        VariantDef { id: "ytath_entry_and_exit_versions_with_weekly_1", data: NSE500, schema: schema_weekly, signals: sig_weekly },
    ]
}
