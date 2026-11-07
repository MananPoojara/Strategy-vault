//! ATH engines: ATHC entry with ATR / turnover exit, next-day BTST, profit target and Supertrend 10,4 exit.
//! One `VariantDef` per sub-variant (see `ath_core::VariantDef`); registered in `ath_registry.rs`. Standard library only: depends on
//! `ath_core` only so a verification driver can include it with `#[path]`. Verification notes: `ATH/verify/<variant id>/RESULT.md`.
#![allow(dead_code)]
#[allow(unused_imports)]
use crate::ath_core::*;

/// Comparison with a Null element: in the team's sheets a comparison against Null (e.g. `OpenInt > MA(...)` on the first bars) is
/// false (verified on the ATR / turnover sheet). NaN stands for Null here.
fn gt(a: f32, b: f32) -> bool {
    !a.is_nan() && !b.is_nan() && a > b
}

/// Yearly turnover threshold of the AFLs (`Volsig`), compared with `OpenInt`.
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

/// `ATR(1)`: the true range (first bar H - L).
fn tr1(h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    (0..h.len()).map(|i| if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect()
}

/// `ATR(n)`: Wilder smoothing of the true range. Seed: the simple average of the first n true ranges at bar n-1, Null before.
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

/// `MA(a, n)` float32: average of the last n bars, Null for the first n-1.
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

fn hhv_all(c: &[f32]) -> Vec<f32> {
    let mut h = vec![0f32; c.len()];
    for i in 0..c.len() {
        h[i] = if i == 0 || c[i] > h[i - 1] { c[i] } else { h[i - 1] };
    }
    h
}

fn clamp_prices(sg: &mut Signals, s: &Series) {
    for i in 0..s.bars.len() {
        let b = &s.bars[i];
        sg.buy_price[i] = sg.buy_price[i].min(b.h).max(b.l);
        sg.sell_price[i] = sg.sell_price[i].min(b.h).max(b.l);
    }
}

// =============================================================== 1. ATHC entry and exit by ATR and turnover

#[derive(Clone, Copy)]
pub struct AtrTo {
    pub dead_stock: bool, // DEAD STOCK BACKTEST = YES
    pub penny: f32,
    pub turnover_filter: bool,
    pub avg_prd: usize,
    pub atr_prd: usize,
}

pub fn atr_to_signals(s: &Series, p: &AtrTo) -> Signals {
    let n = s.bars.len();
    let c = s.close();
    let o = s.open();
    let h = s.high();
    let l = s.low();
    let oi = s.openint();
    let hcl = hhv_all(&c);
    let tr = tr1(&h, &l, &c);
    let myatr = atr_n(&tr, p.atr_prd);
    let avg = ma_n(&oi, p.avg_prd);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut position = false;
    let (mut skipbuy, mut skipsell) = (false, false);
    for i in 1..n {
        let uc = h[i] == c[i];
        let lc = l[i] == c[i];
        let turnover = if p.turnover_filter { oi[i] > volsig(s.bars[i].date / 10000) } else { true };
        let dead_buy = if p.dead_stock { i != n - 1 } else { i != 0 };
        let dead_sell = p.dead_stock && i == n - 1;
        let filter = !uc && turnover && c[i] > p.penny && dead_buy;
        if !position && c[i] > hcl[i - 1] && !skipbuy {
            if filter {
                buy[i] = true;
                position = true;
            } else {
                skipbuy = true;
            }
        } else if skipbuy && filter {
            buy[i] = true;
            position = true;
            skipbuy = false;
        } else if position && o[i] > c[i] && gt(tr[i], myatr[i]) && gt(oi[i], avg[i]) && !skipsell {
            if !lc || dead_sell {
                sell[i] = true;
                position = false;
                skipsell = false;
            } else {
                skipsell = true;
            }
        } else if (position && !lc && skipsell) || (position && dead_sell) {
            sell[i] = true;
            position = false;
            skipsell = false;
        }
    }
    let mut sg = Signals::new(buy, sell, c.clone(), c.clone());
    clamp_prices(&mut sg, s);
    sg
}

fn atr_to_schema() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
        p("avg_turnover_prd", "Avg TurnOver Period", 10.0, 5.0, 30.0, 5.0),
        p("atr_prd", "ATR Period", 5.0, 5.0, 10.0, 1.0),
    ]
}

fn atr_to_all(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let ep = AtrTo { dead_stock: g("deadstock").round() == 0.0, penny: g("pennyflt") as f32, turnover_filter: g("turnflt").round() == 1.0, avg_prd: g("avg_turnover_prd") as usize, atr_prd: g("atr_prd") as usize };
    series.iter().map(|s| atr_to_signals(s, &ep)).collect()
}

// =============================================================== 2. ATHC entry, exit on the next day (BTST)

pub fn btst_signals(s: &Series, dead_stock: bool, penny: f32, opposite: bool) -> Signals {
    let n = s.bars.len();
    let c = s.close();
    let o = s.open();
    let hcl = hhv_all(&c);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut sell_price = c.clone();
    let mut position = false;
    let mut price_at_buy = 0f32;
    for i in 1..n {
        let dead_buy = if dead_stock { i != n - 1 } else { i != 0 };
        if !position && c[i] > hcl[i - 1] && c[i] > penny && dead_buy {
            buy[i] = true;
            price_at_buy = c[i];
            position = true;
        } else if position {
            if o[i] < price_at_buy {
                sell[i] = true;
                sell_price[i] = if opposite { c[i] } else { o[i] };
                position = false;
            } else if o[i] > price_at_buy {
                sell[i] = true;
                sell_price[i] = if opposite { o[i] } else { c[i] };
                position = false;
            }
        }
    }
    let mut sg = Signals::new(buy, sell, c.clone(), sell_price);
    clamp_prices(&mut sg, s);
    sg
}

fn btst_schema() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO; this AFL only uses it to block a buy on the last bar)", 1.0, 0.0, 1.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("ver", "Exit Version (0 Simple, 1 Opposite)", 0.0, 0.0, 1.0, 1.0),
        p("exitpct", "Swing EXIT % (computed by the AFL, not used by Buy / Sell)", 21.0, 1.0, 40.0, 1.0),
    ]
}

fn btst_all(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| btst_signals(s, g("deadstock").round() == 0.0, g("pennyflt") as f32, g("ver").round() == 1.0)).collect()
}

// =============================================================== 3. ATHC entry, %swing exit or profit target

/// The AFL's %swing line state (`trend1`), float32 as the AFL's array operations.
fn swing_trend(c: &[f32], exit_pct: f32) -> Vec<i8> {
    let n = c.len();
    let exit: Vec<f32> = c.iter().map(|&x| (x * exit_pct) / 100.0).collect();
    let mut up: Vec<f32> = (0..n).map(|i| c[i] + exit[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| c[i] - exit[i]).collect();
    let mut trend = vec![1i8; n];
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

#[derive(Clone, Copy)]
pub struct PtSwing {
    pub exit_pct: f32,
    pub pt_pct: f32,
    pub dead_stock: bool,
    pub penny: f32,
    pub turnover_filter: bool,
}

pub fn pt_swing_signals(s: &Series, p: &PtSwing) -> Signals {
    let n = s.bars.len();
    let c = s.close();
    let o = s.open();
    let h = s.high();
    let l = s.low();
    let oi = s.openint();
    let hcl = hhv_all(&c);
    let trend = swing_trend(&c, p.exit_pct);
    let mut athc = vec![false; n];
    for i in 1..n {
        athc[i] = if c[i] > hcl[i - 1] {
            true
        } else if trend[i] == -1 && athc[i - 1] {
            false
        } else {
            athc[i - 1]
        };
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut sell_price = c.clone();
    let mut position = false;
    let mut price_at_buy = 0f32;
    let mut buy_skip = false;
    for i in 1..n {
        let upcircuit = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
        let locircuit = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
        let turnover = if p.turnover_filter { oi[i] > volsig(s.bars[i].date / 10000) } else { true };
        let dead_buy = if p.dead_stock { i != n - 1 } else { i != 0 };
        let dead_sell = p.dead_stock && i == n - 1;
        let buy_filter = turnover && !upcircuit && c[i] > p.penny && dead_buy;
        if buy_skip && trend[i] == -1 {
            buy_skip = false;
        }
        if !position && (athc[i] || c[i] > hcl[i - 1]) && buy_filter && !buy_skip {
            buy[i] = true;
            price_at_buy = c[i];
            position = true;
        } else if position {
            let pt = (1.0f32 + p.pt_pct * 0.01) * price_at_buy;
            if ((trend[i] == -1 || !athc[i] || h[i] >= pt) && !locircuit) || dead_sell {
                sell[i] = true;
                sell_price[i] = if h[i] >= pt { pt } else { c[i] };
                position = false;
                price_at_buy = 0.0;
                buy_skip = true;
            }
        }
    }
    let mut sg = Signals::new(buy, sell, c.clone(), sell_price);
    clamp_prices(&mut sg, s);
    sg
}

fn pt_swing_schema() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off; ApplyStop percent loss)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
        p("pt_pct", "PT Percent", 5.0, 5.0, 50.0, 5.0),
        p("exitpct", "EXIT %", 10.0, 3.0, 22.0, 1.0),
    ]
}

fn pt_swing_all(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let ep = PtSwing { exit_pct: g("exitpct") as f32, pt_pct: g("pt_pct") as f32, dead_stock: g("deadstock").round() == 0.0, penny: g("pennyflt") as f32, turnover_filter: g("turnflt").round() == 1.0 };
    let hard = g("hardstop") as f32;
    series
        .iter()
        .map(|s| {
            let mut sg = pt_swing_signals(s, &ep);
            if hard > 0.0 {
                sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
            }
            sg
        })
        .collect()
}

// =============================================================== 4. ATHC with Supertrend 10/4 exit

/// The AFL's ST() function: returns btrend. `Null` = NaN; reads of Null in comparisons are -1e10.
fn supertrend(stcal: &[f32], tr: &[f32], c: &[f32], period: usize, factor: f32) -> Vec<i8> {
    let n = c.len();
    let mut satr = vec![f32::NAN; n];
    for i in 0..n {
        // satr = Ref(ATR(1),0) + Ref(ATR(1),-1) + ... + Ref(ATR(1),-(V1-1)), then / V1; Null if any term is Null
        if period >= 1 && i + 1 >= period {
            let mut s = tr[i];
            for k in 1..period {
                s += tr[i - k];
            }
            satr[i] = s / period as f32;
        }
    }
    let addm = |a: f32, m: f32, b: f32, sub: bool| -> f32 {
        if a.is_nan() || b.is_nan() { f32::NAN } else if sub { a - m * b } else { a + m * b }
    };
    let mut up: Vec<f32> = (0..n).map(|i| addm(stcal[i], 1.0, satr[i], false)).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| addm(stcal[i], factor, satr[i], true)).collect();
    let mut tdir = vec![1i8; n];
    for i in 1..n {
        tdir[i] = 1;
        if gt(c[i], up[i - 1]) {
            tdir[i] = 1;
        } else if gt(dn[i - 1], c[i]) {
            tdir[i] = -1;
        } else {
            tdir[i] = tdir[i - 1];
        }
        let flag1 = tdir[i] < 0 && tdir[i - 1] > 0;
        let flagh1 = tdir[i] > 0 && tdir[i - 1] < 0;
        if tdir[i] > 0 && gt(dn[i - 1], dn[i]) {
            dn[i] = dn[i - 1];
        }
        if tdir[i] < 0 && gt(up[i], up[i - 1]) {
            up[i] = up[i - 1];
        }
        if flag1 {
            up[i] = addm(stcal[i], 1.0, satr[i], false);
        }
        if flagh1 {
            dn[i] = addm(stcal[i], factor, satr[i], true);
        }
    }
    tdir
}

pub fn supertrend_signals(s: &Series, dead_stock: bool, tstart: usize, period: usize, factor: f32, stflt: i32) -> Signals {
    let n = s.bars.len();
    let c = s.close();
    let o = s.open();
    let h = s.high();
    let l = s.low();
    let stcal: Vec<f32> = (0..n)
        .map(|i| match stflt {
            1 => (h[i] + l[i]) / 2.0,
            2 => (h[i] + l[i] + c[i]) / 3.0,
            _ => c[i],
        })
        .collect();
    let tr = tr1(&h, &l, &c);
    let hcl = hhv_all(&c);
    let bt = supertrend(&stcal, &tr, &c, period, factor);
    let mut buysig = vec![false; n];
    for i in 1..n {
        buysig[i] = if c[i] > hcl[i - 1] {
            true
        } else if bt[i] == -1 {
            false
        } else {
            buysig[i - 1]
        };
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let upc = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
        let loc = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
        let (dbuy, dsell) = if dead_stock { (i != n - 1, i == n - 1) } else { (i != 0, i == 0) };
        let prev_h = if i == 0 { f32::NAN } else { hcl[i - 1] };
        buy[i] = ((gt(c[i], prev_h) && !upc) || (buysig[i] && !upc)) && i >= tstart && dbuy;
        sell[i] = (bt[i] == -1 && !loc) || dsell || (!buysig[i] && !loc);
    }
    let b2 = exrem(&buy, &sell);
    let s2 = exrem(&sell, &b2);
    let mut sg = Signals::new(b2, s2, c.clone(), c.clone());
    clamp_prices(&mut sg, s);
    sg
}

fn supertrend_schema() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("stflt", "ST CALCULATION POINT (0 CLOSE, 1 MID POINT (H+L)/2, 2 PIVOT (H+L+C)/3)", 0.0, 0.0, 2.0, 1.0),
        p("tstart", "TRADING START BAR", 1.0, 0.0, 500.0, 1.0),
        p("bpd1", "ST Period", 10.0, 1.0, 20.0, 1.0),
        p("bfactor1", "ST Factor", 4.0, 1.0, 20.0, 1.0),
    ]
}

fn supertrend_all(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| supertrend_signals(s, g("deadstock").round() == 0.0, g("tstart") as usize, g("bpd1") as usize, g("bfactor1") as f32, g("stflt").round() as i32)).collect()
}

// =============================================================== 5. ATHC 8 % range with Supertrend buy (new trade only)

/// The AFL's ST() with both bands at `factor` x mean true range. Returns (trend, trendUp line, trendDown line); Null = NaN.
/// The back-fill of the previous bar at a trend change (`bTrendUp1[i-1] = bTrendDown1[i-1]` ...) is kept.
fn st_lines(stcal: &[f32], tr: &[f32], c: &[f32], period: usize, factor: f32) -> (Vec<i8>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let n = c.len();
    let mut satr = vec![f32::NAN; n];
    for i in 0..n {
        // ATR(1) is Null on the first bar of the array (TR needs Ref(C,-1)): a window containing bar 0 is Null, the first line is at bar `period`
        if period >= 1 && i >= period {
            let mut s = tr[i];
            for k in 1..period {
                s += tr[i - k];
            }
            satr[i] = s / period as f32;
        }
    }
    let mut up: Vec<f32> = (0..n).map(|i| stcal[i] + factor * satr[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| stcal[i] - factor * satr[i]).collect();
    let mut trend = vec![1i8; n];
    let (mut tu, mut td) = (vec![f32::NAN; n], vec![f32::NAN; n]);
    let mut change = false;
    for i in 1..n {
        trend[i] = 1;
        if gt(c[i], up[i - 1]) {
            trend[i] = 1;
            if trend[i - 1] == -1 {
                change = true;
            }
        } else if gt(dn[i - 1], c[i]) {
            trend[i] = -1;
            if trend[i - 1] == 1 {
                change = true;
            }
        } else if trend[i - 1] == 1 {
            trend[i] = 1;
        } else if trend[i - 1] == -1 {
            trend[i] = -1;
            change = false;
        }
        let flag1 = trend[i] < 0 && trend[i - 1] > 0;
        let flagh1 = trend[i] > 0 && trend[i - 1] < 0;
        if trend[i] > 0 && gt(dn[i - 1], dn[i]) {
            dn[i] = dn[i - 1];
        }
        if trend[i] < 0 && gt(up[i], up[i - 1]) {
            up[i] = up[i - 1];
        }
        if flag1 {
            up[i] = stcal[i] + factor * satr[i];
        }
        if flagh1 {
            dn[i] = stcal[i] - factor * satr[i];
        }
        if trend[i] == 1 {
            tu[i] = dn[i];
            if change {
                tu[i - 1] = td[i - 1];
                change = false;
            }
        } else if trend[i] == -1 {
            td[i] = up[i];
            if change {
                td[i - 1] = tu[i - 1];
                change = false;
            }
        }
    }
    (trend, tu, td, up, dn)
}

fn volsig_new(year: i32) -> f32 {
    match year {
        y if y < 2000 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

#[derive(Clone, Copy)]
pub struct StNew {
    pub dead_stock: bool,
    pub penny: f32,
    pub turnover_filter: bool, // TURNOVER Filter = YES (toggle 0): V x C > Volsig
    pub stflt: i32,
    pub bpd1: usize,
    pub bfactor1: f32,
    pub bpd2: usize,
    pub bfactor2: f32,
    pub diff_pct: f32,
    pub rankmode: i32,
    pub rocpd: usize,
    pub rsipd: usize,
    pub stdpd: usize,
    pub swingpd: usize,
    /// engine option, NOT in the AFL file (default false = the file's O==C==L / O==C==H): circuits tested as C==L / C==H (Penny TL sheet)
    pub circuit_c: bool,
}

pub fn st_new_signals(s: &Series, p: &StNew) -> Signals {
    let n = s.bars.len();
    let c = s.close();
    let o = s.open();
    let h = s.high();
    let l = s.low();
    let v = s.volume();
    let mp: Vec<f32> = (0..n).map(|i| (h[i] + l[i]) / 2.0).collect();
    let pivot: Vec<f32> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
    let stcal: Vec<f32> = match p.stflt {
        1 => c.clone(),
        2 => pivot.clone(),
        3 => (0..n).map(|i| (mp[i] + pivot[i] + c[i]) / 3.0).collect(),
        5 => ma_n(&c, 5),
        _ => mp.clone(), // 0 MID POINT; 4 "3DAYS MA" never matches ('3DYAS MA' in the code), so the mid point stays
    };
    let tr = tr1(&h, &l, &c);
    let (_t1, _tu1, td1, _u1, _d1) = st_lines(&stcal, &tr, &c, p.bpd1, p.bfactor1);
    let (t2_trend, _, _, u2, d2) = st_lines(&stcal, &tr, &c, p.bpd2, p.bfactor2);
    let t1_trend = _t1;
    let hcl = hhv_all(&c);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let upc = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
        let mut loc = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
        let mut upc = upc;
        if p.circuit_c {
            // other AFL version (the workbook's "11. Penny TL" sheet): CIRCUIT2 = C==H, CIRCUIT3 = L==C as in the 52-week-high AFLs
            loc = c[i] == l[i];
            upc = c[i] == h[i];
        }
        let (dbuy, dsell) = if p.dead_stock { (i != n - 1, i == n - 1) } else { (i != 0, false) };
        let turnover = if p.turnover_filter { v[i] * c[i] > volsig_new(s.bars[i].date / 10000) } else { true };
        let athdiff = hcl[i] - (hcl[i] * p.diff_pct) / 100.0;
        // Cross(C, Ref(bfactor1,-1)): C[i] > b[i] and C[i-1] <= b[i-1], b[i] = bfactor1[i-1]
        let crossed = if i >= 2 {
            let (b_now, b_prev) = (td1[i - 1], td1[i - 2]);
            // Cross(C, line): C above the line now and not above it on the previous bar; a Null line compares false (the team sheets agree)
            gt(c[i], b_now) && !b_prev.is_nan() && c[i - 1] <= b_prev
        } else {
            false
        };
        buy[i] = crossed && c[i] > athdiff && dbuy && !upc && c[i] > p.penny && turnover && i > p.bpd1;
        sell[i] = ((t1_trend[i] == -1 && t2_trend[i] == -1) || dsell) && !loc;
    }
    let b2 = exrem(&buy, &sell);
    let s2 = exrem(&sell, &b2);
    let mut sg = Signals::new(b2, s2, c.clone(), c.clone());
    clamp_prices(&mut sg, s);
    // PositionScore by RANKING MODE (not exercised by a team sheet; POSITIVE CLOSE not ported)
    sg.score = match p.rankmode {
        1 if p.rocpd > 0 => Some((0..n).map(|i| if i >= p.rocpd && c[i - p.rocpd] != 0.0 { 100.0 + 100.0 * (c[i] / c[i - p.rocpd] - 1.0) } else { f32::NAN }).collect()),
        2 => Some(rsi_wilder(&c, p.rsipd)),
        3 => Some(stdev_n(&c, p.stdpd)),
        4 if p.rocpd > 0 => {
            let sd = stdev_n(&c, p.stdpd);
            Some((0..n).map(|i| if i >= p.rocpd && c[i - p.rocpd] != 0.0 { 100.0 + (100.0 * (c[i] / c[i - p.rocpd] - 1.0)) / sd[i] } else { f32::NAN }).collect())
        }
        6 => {
            let mut sc = vec![0f32; n];
            for i in 0..n {
                let lo = i.saturating_sub(p.swingpd.saturating_sub(1));
                let hh = c[lo..=i].iter().copied().fold(f32::MIN, f32::max);
                sc[i] = 100.0 - (hh - c[i]) / hh * 100.0;
            }
            Some(sc)
        }
        _ => None,
    };
    sg
}

fn stdev_n(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![f32::NAN; a.len()];
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

fn rsi_wilder(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![f32::NAN; a.len()];
    if n == 0 || a.len() <= n {
        return out;
    }
    let (mut ag, mut al) = (0f32, 0f32);
    for i in 1..=n {
        let d = a[i] - a[i - 1];
        if d > 0.0 { ag += d } else { al -= d }
    }
    ag /= n as f32;
    al /= n as f32;
    out[n] = if al == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + ag / al) };
    for i in n + 1..a.len() {
        let d = a[i] - a[i - 1];
        ag = (ag * (n as f32 - 1.0) + d.max(0.0)) / n as f32;
        al = (al * (n as f32 - 1.0) + (-d).max(0.0)) / n as f32;
        out[i] = if al == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + ag / al) };
    }
    out
}

fn st_new_schema() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (the AFL declares it but never applies it)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 YES: V x C above the year table, 1 NO)", 0.0, 0.0, 1.0, 1.0),
        p("stflt", "DAILY ST CALCULATION POINT (0 MID, 1 CLOSE, 2 PIVOT, 3 MP/CL/PIVOT AVG, 4 3DAYS MA (=mid, AFL typo), 5 5DAYS MA)", 0.0, 0.0, 5.0, 1.0),
        p("bpd2", "ST Period for EXIT", 5.0, 5.0, 15.0, 5.0),
        p("bfactor2", "ST Factor for EXIT", 4.0, 1.0, 6.0, 0.5),
        p("rankmode", "RANKING MODE (0 NONE, 1 ROC, 2 RSI, 3 STD, 4 ROC/STD, 5 POSITIVE CLOSE (not ported), 6 SWING)", 0.0, 0.0, 6.0, 1.0),
        p("rocpd", "ROC PERIOD for RANKING/NO OF POSITIVE CLOSE", 10.0, 0.0, 400.0, 30.0),
        p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("stdpd", "STD PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("swingpd", "SWING PERIOD", 180.0, 10.0, 1000.0, 1.0),
        p("bpd1", "ST Period for ENTRY (Optimize default)", 5.0, 5.0, 15.0, 5.0),
        p("bfactor1", "ST Factor for ENTRY (Optimize default)", 2.0, 1.0, 6.0, 0.5),
        p("diffathc", "% DIFF FROM ATHC (Optimize default)", 8.0, 1.0, 12.0, 1.0),
        p("circuitc", "ENGINE OPTION, not in the AFL: 0 = circuits as in the AFL file (O==C==L / O==C==H), 1 = C==L / C==H (version behind the Penny TL sheet)", 0.0, 0.0, 1.0, 1.0),
    ]
}

fn st_new_all(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let ep = StNew {
        dead_stock: g("deadstock").round() == 0.0,
        penny: g("pennyflt") as f32,
        turnover_filter: g("turnflt").round() == 0.0,
        stflt: g("stflt").round() as i32,
        bpd1: g("bpd1") as usize,
        bfactor1: g("bfactor1") as f32,
        bpd2: g("bpd2") as usize,
        bfactor2: g("bfactor2") as f32,
        diff_pct: g("diffathc") as f32,
        rankmode: g("rankmode").round() as i32,
        rocpd: g("rocpd") as usize,
        rsipd: g("rsipd") as usize,
        stdpd: g("stdpd") as usize,
        swingpd: g("swingpd") as usize,
        circuit_c: g("circuitc").round() == 1.0,
    };
    series.iter().map(|s| st_new_signals(s, &ep)).collect()
}

pub fn variants() -> Vec<VariantDef> {
    vec![
        VariantDef { id: "ytath_entry_and_exit_by_atr_and_turnover", data: Data::Fo { watchlist: "FO STOCKS C&P.tls" }, schema: atr_to_schema, signals: atr_to_all },
        VariantDef { id: "ytath_entry_and_exit_on_next_day_btst_with", data: Data::Fo { watchlist: "FO STOCKS C&P.tls" }, schema: btst_schema, signals: btst_all },
        VariantDef { id: "ytath_entry_on_athc_exit_below_pctswing_or", data: Data::Fo { watchlist: "FO STOCKS C&P.tls" }, schema: pt_swing_schema, signals: pt_swing_all },
        VariantDef { id: "ytath_with_supertrend_10_4_exit_equal", data: Data::Cash { watchlist: "NSE 500 C&P.tls" }, schema: supertrend_schema, signals: supertrend_all },
        VariantDef { id: "ytath_8pct_range_with_supertrend_buy_new", data: Data::Cash { watchlist: "NSE 500 C&P.tls" }, schema: st_new_schema, signals: st_new_all },
    ]
}
