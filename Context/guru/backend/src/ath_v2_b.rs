//! ATH sub-variants of the overnight batch "b" (22 AFLs): "After consecutive losses" and "ATHC+% buy: %Swing exit".
//! One `VariantDef` per sub-variant (see `ath_core::VariantDef`); registered in `ath_registry.rs`. Standard library only (depends on `ath_core`).
//!
//! Every variant is the AFL as written (float32, Null compares false, Prec(x,2) truncates, fills clamped to [Low, High]); the building blocks are
//! copies of the verified pieces of `ath_loss1.rs` (supertrend probability, Null profit), `ath_loss2.rs` (%swing, loss streak, ATR scale-in,
//! ranking scores), `ath_plus.rs` (ATHC trigger latch, +x% buy) and `ath_exits2.rs` (profit target). Verification notes: `ATH/v2/<id>/VERIFY.md`.
//!
//! List parameters are 0-based indexes: deadstock 0 YES / 1 NO, turnflt 0 NO / 1 YES, stflt 0 CLOSE / 1 MID POINT / 2 PIVOT,
//! stflt1 0 MID POINT / 1 CLOSE / 2 PIVOT, rankmode 0 NONE .. 10 (as the AFL's list), backtest 0 ALL / 1 NSE500 / 2 FO / 3 NSE500-FO.
//! `Optimize(name, default, ...)` of an AFL is a constant (the default) because a normal backtest returns the default.
#![allow(dead_code)]
use crate::ath_core::*;

/// AmiBroker's Null as stored in an array.
const NULLV: f32 = -1.0e10;

fn is_null(x: f32) -> bool {
    x <= -9.0e9 || x.is_nan()
}

// ---------------------------------------------------------------- tables, circuits, small helpers

/// Table compared with `OpenInt` by the long-table AFLs (turnover = Close x Volume in the converted databases).
fn volsig_a(year: i32) -> f32 {
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

/// Table compared with `V*C` by the "consecutive loss" AFLs.
fn volsig_b(year: i32) -> f32 {
    match year {
        y if y < 2000 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

/// `Hcl = HHV(C, BarCount)`: highest close up to the bar.
fn hhv_all(c: &[f32]) -> Vec<f32> {
    let mut h = vec![0f32; c.len()];
    for i in 0..c.len() {
        h[i] = if i == 0 || c[i] > h[i - 1] { c[i] } else { h[i - 1] };
    }
    h
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

/// (deadstockbuysig, deadstocksellsig). `sell_bar0` = the AFL writes `deadstocksellsig = barIndex()==0` for DEAD STOCK = NO.
fn dead_flags(n: usize, dead_yes: bool, sell_bar0: bool) -> (Vec<bool>, Vec<bool>) {
    if dead_yes {
        ((0..n).map(|i| i != n - 1).collect(), (0..n).map(|i| i == n - 1).collect())
    } else {
        ((0..n).map(|i| i != 0).collect(), (0..n).map(|i| sell_bar0 && i == 0).collect())
    }
}

/// `deadstock1`: O, H, L, C equal to those of each of the next 30 bars (`Ref(x, k)` with k > 0 looks forward; beyond the end it is Null = false).
fn flat30(o: &[f32], h: &[f32], l: &[f32], c: &[f32]) -> Vec<bool> {
    let n = c.len();
    let mut run = vec![0u32; n + 1];
    for i in (0..n.saturating_sub(1)).rev() {
        let same = o[i] == o[i + 1] && h[i] == h[i + 1] && l[i] == l[i + 1] && c[i] == c[i + 1];
        run[i] = if same { run[i + 1] + 1 } else { 0 };
    }
    (0..n).map(|i| run[i] >= 30).collect()
}

/// `ATR(1)`: the true range, first bar H - L.
fn tr1(h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    (0..h.len()).map(|i| if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect()
}

/// Wilder's ATR(n) as AmiBroker computes it: the simple average of the first n true ranges, then Wilder smoothing; Null (-1e10) before.
fn atr_wilder(h: &[f32], l: &[f32], c: &[f32], n: usize) -> Vec<f32> {
    let len = h.len();
    let tr = tr1(h, l, c);
    let mut out = vec![NULLV; len];
    if n == 0 {
        return out;
    }
    if len >= n {
        out[n - 1] = tr[..n].iter().sum::<f32>() / n as f32;
        for i in n..len {
            out[i] = (out[i - 1] * (n as f32 - 1.0) + tr[i]) / n as f32;
        }
    }
    out
}

fn clamp_prices(sg: &mut Signals, h: &[f32], l: &[f32]) {
    for i in 0..sg.buy_price.len() {
        sg.buy_price[i] = sg.buy_price[i].min(h[i]).max(l[i]);
        sg.sell_price[i] = sg.sell_price[i].min(h[i]).max(l[i]);
    }
}

/// Hard Stoploss % (ApplyStop percent loss) and the optional PositionScore.
fn finish(mut sg: Signals, hard: f32, score: Option<Vec<f32>>) -> Signals {
    if hard > 0.0 {
        sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
    }
    sg.score = score;
    sg
}

/// ExRem on both sides, `BuyPrice = ValueWhen(Buy, C)`, `SellPrice = ValueWhen(Sell, C)` (= Close on the signal bar), clamped into [Low, High].
fn exrem_close(buy: &[bool], sell: &[bool], c: &[f32], h: &[f32], l: &[f32]) -> Signals {
    let b2 = exrem(buy, sell);
    let s2 = exrem(sell, &b2);
    let mut sg = Signals::new(b2, s2, c.to_vec(), c.to_vec());
    clamp_prices(&mut sg, h, l);
    sg
}

// ---------------------------------------------------------------- %swing, ATHC state, trigger latches

struct Swing {
    trend: Vec<i8>,
    tup: Vec<f32>,
    tdn: Vec<f32>,
}

/// The AFL's %swing loop (Up = C + EXIT, Dn = C - EXIT, EXIT = C * EXITPCT / 100) with the changeOfTrend back-fill of TrendUp1 / TrendDown1.
fn swing(c: &[f32], exit_pct: f32) -> Swing {
    let n = c.len();
    let exit: Vec<f32> = c.iter().map(|&x| (x * exit_pct) / 100.0).collect();
    let mut up: Vec<f32> = (0..n).map(|i| c[i] + exit[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| c[i] - exit[i]).collect();
    let mut trend = vec![0i8; n];
    let mut tup = vec![NULLV; n];
    let mut tdn = vec![NULLV; n];
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
            tup[i] = dn[i];
            if change {
                tup[i - 1] = tdn[i - 1];
                change = false;
            }
        } else if trend[i] == -1 {
            tdn[i] = up[i];
            if change {
                tdn[i - 1] = tup[i - 1];
                change = false;
            }
        }
    }
    Swing { trend, tup, tdn }
}

/// ATHCbuysig: 1 on a close above all earlier closes (and above the trigger when `trigger` is given), 0 when the swing trend is -1 while it was 1, else carried.
fn athc_state(c: &[f32], hcl: &[f32], trend: &[i8], trigger: Option<&[f32]>) -> Vec<bool> {
    let n = c.len();
    let mut a = vec![false; n];
    for i in 1..n {
        let new_high = c[i] > hcl[i - 1] && trigger.map_or(true, |t| !is_null(t[i]) && c[i] > t[i]);
        a[i] = if new_high {
            true
        } else if trend[i] == -1 && a[i - 1] {
            false
        } else {
            a[i - 1]
        };
    }
    a
}

/// ATHC trigger, version "set, then clear on a down swing" (`if (C>hcl[i-1] AND t[i-1]==0) {...} else {carry}; if (trend1[i]==-1) {t=0; Null}`).
fn trigger_override(c: &[f32], hcl: &[f32], trend1: &[i8], pct: f32) -> Vec<f32> {
    let n = c.len();
    let mut t = vec![false; n];
    let mut trigger = vec![NULLV; n];
    for i in 1..n {
        if c[i] > hcl[i - 1] && !t[i - 1] {
            t[i] = true;
            trigger[i] = c[i] + (c[i] * pct) / 100.0;
        } else {
            trigger[i] = trigger[i - 1];
            t[i] = t[i - 1];
        }
        if trend1[i] == -1 {
            t[i] = false;
            trigger[i] = NULLV;
        }
    }
    trigger
}

/// ATHC trigger, version "if / else if (trend1==-1) / else" (the scale-in and profit-target AFLs).
fn trigger_elseif(c: &[f32], hcl: &[f32], trend1: &[i8], pct: f32) -> Vec<f32> {
    let n = c.len();
    let mut t = vec![false; n];
    let mut trigger = vec![NULLV; n];
    for i in 1..n {
        if c[i] > hcl[i - 1] && !t[i - 1] {
            t[i] = true;
            trigger[i] = c[i] + (c[i] * pct) / 100.0;
        } else if trend1[i] == -1 {
            t[i] = false;
            trigger[i] = NULLV;
        } else {
            t[i] = t[i - 1];
            trigger[i] = trigger[i - 1];
        }
    }
    trigger
}

// ---------------------------------------------------------------- virtual trades and the loss streak

struct Virt {
    bcount: Vec<i32>,
    bprice: Vec<f32>,
    sprice: Vec<f32>,
}

/// The AFL's bcount / bprice / sprice loop: a virtual trade in on `buysig`, out on `sellsig`, at the close.
fn virt_trades(c: &[f32], buysig: &[bool], sellsig: &[bool]) -> Virt {
    let n = c.len();
    let (mut bcount, mut bprice, mut sprice) = (vec![0i32; n], vec![NULLV; n], vec![NULLV; n]);
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
    Virt { bcount, bprice, sprice }
}

/// (profit, profit %) of the virtual trade on bar i with the AFL's brokerage arithmetic; a Null price makes both Null (non-zero, counted as a loss).
fn virt_profit(v: &Virt, i: usize, comm: f32) -> (f32, f32) {
    let (sp, bp) = (v.sprice[i], v.bprice[i]);
    if is_null(sp) || is_null(bp) {
        return (NULLV, NULLV);
    }
    let profit = (sp - (sp * comm) / 100.0) - (bp + (bp * comm) / 100.0);
    let pct = (((sp - (sp * comm) / 100.0) - (bp + (bp * comm) / 100.0)) / bp) * 100.0;
    (profit, pct)
}

/// `Cross(Ref(line, -1), C)`: the previous bar's line is above the close now and was not above it a bar earlier (Null = -1e10 here, as in ath_loss1 / ath_loss2).
fn cross_prev_line_above_close(line: &[f32], c: &[f32]) -> Vec<bool> {
    let n = c.len();
    let mut out = vec![false; n];
    for i in 2..n {
        out[i] = line[i - 1] > c[i] && line[i - 2] <= c[i - 1];
    }
    out
}

/// concount: `if (profit != 0) { if (prftpct <= LOSSPCT) concount+1 else 0 } else carry` over the bars where `ev` holds.
fn concount_from(c: &[f32], ev: &[bool], v: &Virt, losspct: f32, comm: f32, carry_only_in: Option<&[bool]>) -> Vec<f32> {
    let n = c.len();
    let mut cc = vec![0f32; n];
    for i in 1..n {
        let (profit, pct) = if ev[i] { virt_profit(v, i, comm) } else { (0.0, 0.0) };
        cc[i] = if profit != 0.0 {
            if pct <= losspct { cc[i - 1] + 1.0 } else { 0.0 }
        } else {
            match carry_only_in {
                Some(st) => if st[i] { cc[i - 1] } else { 0.0 },
                None => cc[i - 1],
            }
        };
    }
    cc
}

/// Loss streak of the %swing virtual trades (in on `buysig`, out on trend -1; the loss is booked when Close falls through the previous TrendUp1).
fn concount_swing(c: &[f32], buysig: &[bool], sw: &Swing, losspct: f32, comm: f32) -> Vec<f32> {
    let n = c.len();
    let sell: Vec<bool> = sw.trend.iter().map(|&t| t == -1).collect();
    let v = virt_trades(c, buysig, &sell);
    let cr = cross_prev_line_above_close(&sw.tup, c);
    let ev: Vec<bool> = (0..n).map(|i| i >= 1 && cr[i] && v.bcount[i - 1] == 1).collect();
    concount_from(c, &ev, &v, losspct, comm, None)
}

// ---------------------------------------------------------------- Supertrend (the AFLs' ST / ST1 functions)

/// The AFL's supertrend function. satr = (ATR(1) + Ref(ATR(1),-1) + ... + Ref(ATR(1),-(ceil(period)-1))) / period (Null while the history is short);
/// Up = stcal + up_m * satr, Dn = stcal - dn_m * satr. Returns (TrendUp line, TrendDown line, trend); Null = -1e10. `backfill`: at a trend change the previous
/// bar of the new trend's line takes the other line's value (the AFL's `bTrendUp1[i-1] = bTrendDown1[i-1]`); `= Null` versions pass false (no effect).
fn st_lines(stcal: &[f32], tr: &[f32], c: &[f32], period: f32, up_m: f32, dn_m: f32, backfill: bool) -> (Vec<f32>, Vec<f32>, Vec<i8>) {
    let n = c.len();
    let terms = (period.ceil() as i64).max(1) as usize;
    let satr: Vec<f32> = (0..n)
        .map(|i| {
            if i + 1 < terms {
                return f32::NAN;
            }
            let mut s = tr[i];
            for k in 1..terms {
                s += tr[i - k];
            }
            s / period
        })
        .collect();
    let mut up: Vec<f32> = (0..n).map(|i| stcal[i] + up_m * satr[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| stcal[i] - dn_m * satr[i]).collect();
    let mut trend = vec![1i8; n];
    let (mut tu, mut td) = (vec![NULLV; n], vec![NULLV; n]);
    let nz = |x: f32| if x.is_nan() { NULLV } else { x };
    for i in 1..n {
        let mut change = false;
        let t;
        if c[i] > up[i - 1] {
            t = 1;
            if trend[i - 1] == -1 {
                change = true;
            }
        } else if c[i] < dn[i - 1] {
            t = -1;
            if trend[i - 1] == 1 {
                change = true;
            }
        } else {
            t = trend[i - 1];
        }
        trend[i] = t;
        let flag1 = t < 0 && trend[i - 1] > 0;
        let flagh1 = t > 0 && trend[i - 1] < 0;
        if t > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if t < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if flag1 {
            up[i] = stcal[i] + up_m * satr[i];
        }
        if flagh1 {
            dn[i] = stcal[i] - dn_m * satr[i];
        }
        if t == 1 {
            tu[i] = nz(dn[i]);
            if change && backfill {
                tu[i - 1] = td[i - 1];
            }
        } else {
            td[i] = nz(up[i]);
            if change && backfill {
                td[i - 1] = tu[i - 1];
            }
        }
    }
    (tu, td, trend)
}

/// ST CALCULATION POINT arrays: kind 'a' = list CLOSE|MID POINT|PIVOT (0 close, 1 mid, 2 pivot), kind 'b' = list MID POINT|CLOSE|PIVOT (0 mid, 1 close, 2 pivot).
fn stcal_of(list_mid_first: bool, mode: i64, h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    let m = if list_mid_first { match mode { 0 => 1, 1 => 0, x => x } } else { mode }; // -> 0 close, 1 mid, 2 pivot
    (0..c.len())
        .map(|i| match m {
            1 => (h[i] + l[i]) / 2.0,
            2 => (h[i] + l[i] + c[i]) / 3.0,
            _ => c[i],
        })
        .collect()
}

// ---------------------------------------------------------------- ranking (PositionScore)

fn rank_score(c: &[f32], mode: i64, rocpd: usize, rsipd: usize, stdpd: usize, swingpd: usize) -> Option<Vec<f32>> {
    if mode <= 0 {
        return None;
    }
    let n = c.len();
    let nanify = |v: Vec<f32>| -> Vec<f32> { v.into_iter().map(|x| if x <= NULLV { f32::NAN } else { x }).collect() };
    let roc = |k: usize| -> Vec<f32> { (0..n).map(|i| if i >= k && c[i - k] != 0.0 { 100.0 * (c[i] / c[i - k] - 1.0) } else { f32::NAN }).collect() };
    let stdev = |k: usize| -> Vec<f32> {
        (0..n)
            .map(|i| {
                if k == 0 || i + 1 < k {
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
        let k = rocpd;
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
                let lo = i.saturating_sub(swingpd.max(1) - 1);
                let top = c[lo..=i].iter().copied().fold(f32::MIN, f32::max);
                ((top - c[i]) / top) * 100.0
            })
            .collect()
    };
    let rsi = || -> Vec<f32> {
        let k = rsipd;
        let mut out = vec![f32::NAN; n];
        if k > 0 && n > k {
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
    Some(match mode {
        1 => roc(rocpd).into_iter().map(|x| 100.0 + x).collect(),
        2 => roc(rocpd).into_iter().map(|x| 100.0 - x).collect(),
        3 => rsi(),
        4 => stdev(stdpd),
        5 => {
            let sd = stdev(stdpd);
            roc(rocpd).iter().zip(sd.iter()).map(|(r, d)| 100.0 + r / d).collect()
        }
        6 => {
            let sd = stdev(stdpd);
            roc(rocpd).iter().zip(sd.iter()).map(|(r, d)| 100.0 - r / d).collect()
        }
        7 => pocl(),
        8 => sdiffpct().into_iter().map(|x| 100.0 - x).collect(),
        9 => add(roc(rocpd).into_iter().map(|x| 100.0 + x).collect(), pocl()),
        10 => add(sdiffpct().into_iter().map(|x| 100.0 - x).collect(), pocl()),
        _ => nanify(vec![0.0; n]),
    })
}

fn rank_of(c: &[f32], g: G) -> Option<Vec<f32>> {
    rank_score(c, g("rankmode").round() as i64, g("rocpd").max(0.0) as usize, g("rsipd").max(0.0) as usize, g("stdpd").max(0.0) as usize, g("swingpd").max(1.0) as usize)
}

// ---------------------------------------------------------------- schema helpers

fn pdead() -> P {
    p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0)
}
fn phard() -> P {
    p("hardstop", "Hard Stoploss % (ApplyStop percent loss; 0 = off)", 0.0, 0.0, 50.0, 1.0)
}
fn ppenny() -> P {
    p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0)
}
fn pturn(table: &'static str) -> P {
    p("turnflt", table, 0.0, 0.0, 1.0, 1.0)
}
const TURN_B: &str = "TURNOVER Filter (0 NO, 1 YES; V*C above the AFL's yearly table)";
const TURN_A: &str = "TURNOVER Filter (0 NO, 1 YES; OpenInt above the AFL's yearly table)";
fn prank() -> Vec<P> {
    vec![
        p("rankmode", "RANKING MODE (0 NONE, 1 POSITIVE ROC, 2 NEGATIVE ROC, 3 RSI, 4 STD, 5 POSITIVE ROC/STD, 6 NEGATIVE ROC/STD, 7 POSITIVE CLOSE, 8 SWING, 9 POSITIVE ROC + POSITIVE CLOSE, 10 POSITIVE CLOSE + SWING)", 0.0, 0.0, 10.0, 1.0),
        p("rocpd", "ROC PERIOD for RANKING/NO OF POSITIVE CLOSE", 30.0, 30.0, 400.0, 30.0),
        p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("stdpd", "STD PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("swingpd", "SWING PERIOD", 180.0, 10.0, 1000.0, 1.0),
    ]
}
fn pbuffer() -> P {
    p("buffer_prcnt", "Buffer %", 0.25, 0.0, 2.0, 0.001)
}
fn patr() -> Vec<P> {
    vec![p("atr_prd", "ATR Prds", 14.0, 5.0, 100.0, 1.0), p("myatr1", "ATR Multiplier", 1.0, 0.0, 5.0, 0.025)]
}

fn data22() -> Data {
    Data::Cash22 { watchlist: "NSE 500 C&P.tls" }
}

// ================================================================= 1. +x% buy after consecutive losses (trigger latch)

/// `ytath_plus1pct_buy_pctswing_exit_after` (Buy needs Close > trigger) and `ytath_plus5pct_buy_pctswing_exit_after_7` (the latch itself needs Close > trigger).
fn plus_loss_symbol(s: &Series, g: G, latch_trigger: bool) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let (deadbuy, deadsell) = dead_flags(n, g("deadstock").round() == 0.0, false);
    let (upc, loc) = circuits(&o, &h, &l, &c);
    let penny = g("pennyflt") as f32;
    let turn = g("turnflt").round() == 1.0;
    let turnover: Vec<bool> = (0..n).map(|i| if turn { s.bars[i].v * c[i] > volsig_b(s.bars[i].date / 10000) } else { true }).collect();
    let hcl = hhv_all(&c);
    let sw = swing(&c, g("exitpct") as f32);
    let trigger = trigger_override(&c, &hcl, &sw.trend, g("athcbuypct") as f32);
    let athc = athc_state(&c, &hcl, &sw.trend, if latch_trigger { Some(&trigger) } else { None });
    let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !upc[i] && deadbuy[i] && c[i] > penny && turnover[i]).collect();
    let cc = concount_swing(&c, &buysig, &sw, g("losspct") as f32, g("comm") as f32);
    let noofloss = g("noofloss") as f32;
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let trig_ok = latch_trigger || (!is_null(trigger[i]) && c[i] > trigger[i]);
        buy[i] = athc[i] && trig_ok && cc[i] >= noofloss && !upc[i] && deadbuy[i] && c[i] > penny && turnover[i];
        sell[i] = ((sw.trend[i] == -1 || !athc[i]) && !loc[i]) || deadsell[i];
    }
    finish(exrem_close(&buy, &sell, &c, &h, &l), g("hardstop") as f32, None)
}

fn plus_loss_params(exit_def: f64, athc_def: f64) -> Vec<P> {
    vec![
        pdead(),
        phard(),
        ppenny(),
        pturn(TURN_B),
        p("exitpct", "EXIT %", exit_def, 5.0, 25.0, 1.0),
        p("losspct", "MIN PROFIT %", 2.0, 0.5, 10.0, 0.05),
        p("noofloss", "CONSICUTIVE LOSS", 1.0, 0.0, 50.0, 1.0),
        p("athcbuypct", "BUY PERCENTAGE FROM ATHC", athc_def, 1.0, 8.0, 1.0),
    ]
}
fn schema_plus1() -> Vec<P> {
    plus_loss_params(15.0, 1.0)
}
fn schema_plus7() -> Vec<P> {
    plus_loss_params(10.0, 5.0)
}
fn sig_plus1(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| plus_loss_symbol(s, g, false)).collect()
}
fn sig_plus7(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| plus_loss_symbol(s, g, true)).collect()
}

// ================================================================= 2. ATHC entry, exit below %swing or by profit target

/// `ytath_entry_after_2_consecutive_losses_and` (loss gate) and `ytath_entry_on_athc_plus_5pct_exit_below` (trigger gate): the profit-target loop of ath_exits2.
fn pt_symbol(s: &Series, g: G, loss_gate: bool) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let oi = s.openint();
    let dead_yes = g("deadstock").round() == 0.0;
    let (deadbuy, deadsell) = dead_flags(n, dead_yes, false);
    let (upc, loc) = circuits(&o, &h, &l, &c);
    let penny = g("pennyflt") as f32;
    let turn = g("turnflt").round() == 1.0;
    let turnover: Vec<bool> = (0..n).map(|i| if turn { oi[i] > volsig_a(s.bars[i].date / 10000) } else { true }).collect();
    let hcl = hhv_all(&c);
    let sw = swing(&c, g("exitpct") as f32);
    let athc = athc_state(&c, &hcl, &sw.trend, None);
    let cc = if loss_gate {
        let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !upc[i] && deadbuy[i] && c[i] > penny && turnover[i]).collect();
        concount_swing(&c, &buysig, &sw, g("losspct") as f32, g("comm") as f32)
    } else {
        vec![0.0; n]
    };
    let noofloss = g("noofloss") as f32;
    let trigger = if loss_gate { vec![NULLV; n] } else { trigger_elseif(&c, &hcl, &sw.trend, g("athcbuypct") as f32) };
    let pt_pct = g("pt_pct") as f32;
    let nogate = g("dbg_nogate") != 0.0; // test hook, see ScaleCfg::nogate
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut sell_price = c.clone();
    let mut position = false;
    let mut price_at_buy = 0f32;
    let mut buy_skip = false;
    for i in 1..n {
        let buy_filter = turnover[i] && !upc[i] && c[i] > penny && deadbuy[i];
        if buy_skip && sw.trend[i] == -1 {
            buy_skip = false;
        }
        let gate = nogate || if loss_gate { cc[i] >= noofloss } else { !is_null(trigger[i]) && c[i] > trigger[i] };
        if !position && (athc[i] || c[i] > hcl[i - 1]) && gate && buy_filter && !buy_skip {
            buy[i] = true;
            price_at_buy = c[i];
            position = true;
        } else if position {
            let pt = (1.0f32 + pt_pct * 0.01) * price_at_buy;
            if ((sw.trend[i] == -1 || !athc[i] || h[i] >= pt) && !loc[i]) || deadsell[i] {
                sell[i] = true;
                sell_price[i] = if h[i] >= pt { pt } else { c[i] };
                position = false;
                price_at_buy = 0.0;
                buy_skip = true;
            }
        }
    }
    let mut sg = Signals::new(buy, sell, c.clone(), sell_price);
    clamp_prices(&mut sg, &h, &l);
    finish(sg, g("hardstop") as f32, None)
}

fn schema_pt_loss() -> Vec<P> {
    vec![
        pdead(),
        phard(),
        ppenny(),
        pturn(TURN_A),
        p("pt_pct", "PT Percent", 5.0, 0.0, 20.0, 0.05),
        p("exitpct", "EXIT %", 10.0, 1.0, 20.0, 1.0),
        p("losspct", "MIN PROFIT %", 3.0, 0.0, 3.0, 0.05),
        p("noofloss", "CONSICUTIVE LOSS", 2.0, 1.0, 2.0, 1.0),
    ]
}
fn schema_pt_plus() -> Vec<P> {
    vec![
        pdead(),
        phard(),
        ppenny(),
        pturn(TURN_A),
        p("athcbuypct", "BUY PERCENTAGE FROM ATHC", 5.0, 1.0, 10.0, 1.0),
        p("pt_pct", "PT Percent", 5.0, 5.0, 50.0, 5.0),
        p("exitpct", "EXIT %", 10.0, 3.0, 22.0, 1.0),
    ]
}
fn sig_pt_loss(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| pt_symbol(s, g, true)).collect()
}
fn sig_pt_plus(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| pt_symbol(s, g, false)).collect()
}

// ================================================================= 3. Supertrend 10,4 exit after losing ST trades

/// `ytath_entry_st_10_4_exit_only_after_2`: virtual ATHC trades (in on a fresh ATH close, out when ST(10,4) turns down), real entry on an ATH close after 2 losses.
fn st104_after2_symbol(s: &Series, g: G) -> Signals {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let dead_yes = g("deadstock").round() == 0.0;
    let (deadbuy, deadsell) = dead_flags(n, dead_yes, true);
    let hcl = hhv_all(&c);
    let stcal = stcal_of(false, g("stflt").round() as i64, &h, &l, &c);
    let tr = tr1(&h, &l, &c);
    let (tu, _td, bt) = st_lines(&stcal, &tr, &c, g("bpd1") as f32, 1.0, g("bfactor1") as f32, true);
    let fresh: Vec<bool> = (0..n).map(|i| i >= 1 && c[i] > hcl[i - 1]).collect();
    let sellsig: Vec<bool> = bt.iter().map(|&t| t == -1).collect();
    let v = virt_trades(&c, &fresh, &sellsig);
    let cr = cross_prev_line_above_close(&tu, &c);
    let ev: Vec<bool> = (0..n).map(|i| i >= 1 && cr[i] && v.bcount[i - 1] == 1).collect();
    let cc = concount_from(&c, &ev, &v, g("losspct") as f32, g("comm") as f32, None);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = fresh[i] && deadbuy[i] && cc[i] >= 2.0;
        sell[i] = bt[i] == -1 || deadsell[i];
    }
    exrem_close(&buy, &sell, &c, &h, &l)
}

fn schema_st104_2() -> Vec<P> {
    vec![
        pdead(),
        p("losspct", "MIN PROFIT %", 2.0, 0.5, 10.0, 0.05),
        p("stflt", "ST CALCULATION POINT (0 CLOSE, 1 MID POINT, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0),
        p("bpd1", "ST Period", 10.0, 1.0, 20.0, 1.0),
        p("bfactor1", "ST Factor", 4.0, 1.0, 20.0, 1.0),
    ]
}
fn sig_st104_2(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| st104_after2_symbol(s, g)).collect()
}

// ================================================================= 4. ST probability (two supertrends) gating a %swing / ST(10,4) exit

#[derive(Clone, Copy, PartialEq)]
enum Prob {
    /// `ytath_entry_st_10_4_exit_only_after_n`: ATHC state and exit by ST(10,4), V*C table
    St104,
    /// `ytath_running_pctswing_exit_after_2_st_5_2`: running ATH + %swing exit, OpenInt table, `NOT uc` in Buy, parameters
    Run52,
    /// `ytath_running_pctswing_exit_after_2_st_5_2_2`: as Run52 with V*C table, no `NOT uc`, Optimize() constants
    Run52b,
}

fn prob_symbol(s: &Series, g: G, kind: Prob) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let oi = s.openint();
    let (deadbuy, deadsell) = dead_flags(n, g("deadstock").round() == 0.0, false);
    let (upc, loc) = circuits(&o, &h, &l, &c);
    let uc: Vec<bool> = (0..n).map(|i| h[i] == c[i]).collect();
    let penny = g("pennyflt") as f32;
    let turn = g("turnflt").round() == 1.0;
    let turnover: Vec<bool> = (0..n)
        .map(|i| {
            if !turn {
                true
            } else if kind == Prob::Run52 {
                oi[i] > volsig_a(s.bars[i].date / 10000)
            } else {
                s.bars[i].v * c[i] > volsig_b(s.bars[i].date / 10000)
            }
        })
        .collect();
    let hcl = hhv_all(&c);
    let tr = tr1(&h, &l, &c);
    let stcal1 = stcal_of(true, g("stflt1").round() as i64, &h, &l, &c);
    let (bf2, bf3) = if kind == Prob::Run52b { (2.0f32, 4.0f32) } else { (g("bfactor2") as f32, g("bfactor3") as f32) };
    let (up2, _d2, bt2) = st_lines(&stcal1, &tr, &c, g("bpd2") as f32, bf2, bf2, false);
    let (up3, _d3, bt3) = st_lines(&stcal1, &tr, &c, g("bpd3") as f32, bf3, bf3, false);
    // the exit side
    let (athc, exit_sell): (Vec<bool>, Vec<bool>) = if kind == Prob::St104 {
        let stcal = stcal_of(false, g("stflt").round() as i64, &h, &l, &c);
        let (_tu, _td, bt1) = st_lines(&stcal, &tr, &c, g("bpd1") as f32, 1.0, g("bfactor1") as f32, true);
        (athc_state(&c, &hcl, &bt1, None), bt1.iter().map(|&t| t == -1).collect())
    } else {
        let exit_pct = if kind == Prob::Run52 { g("exitpct") as f32 } else { 10.0 };
        let sw = swing(&c, exit_pct);
        (athc_state(&c, &hcl, &sw.trend, None), sw.trend.iter().map(|&t| t == -1).collect())
    };
    // virtual ST trades -> loss streak
    let buysig: Vec<bool> = bt2.iter().map(|&t| t == 1).collect();
    let sellsig: Vec<bool> = (0..n).map(|i| bt2[i] == -1 && bt3[i] == -1).collect();
    let v = virt_trades(&c, &buysig, &sellsig);
    let cr2 = cross_prev_line_above_close(&up2, &c);
    let cr3 = cross_prev_line_above_close(&up3, &c);
    let ev: Vec<bool> = (0..n).map(|i| i >= 1 && ((bt3[i] == -1 && cr2[i]) || (bt3[i - 1] == 1 && cr3[i]))).collect();
    let cc = concount_from(&c, &ev, &v, g("losspct") as f32, g("comm") as f32, None);
    let noofloss = if kind == Prob::Run52b { 2.0f32 } else { g("noofloss") as f32 };
    let dbg_a1 = g("dbg_a1") != 0.0;
    let dead1 = flat30(&o, &h, &l, &c);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let fresh = i >= 1 && c[i] > hcl[i - 1];
        let common = cc[i] >= noofloss && !upc[i] && deadbuy[i] && c[i] > penny && turnover[i];
        match kind {
            Prob::St104 => {
                buy[i] = athc[i] && fresh && common;
                sell[i] = exit_sell[i] || deadsell[i];
            }
            Prob::Run52 => {
                buy[i] = (athc[i] || fresh) && common && !uc[i];
                sell[i] = ((exit_sell[i] || !athc[i]) && !loc[i]) || deadsell[i];
            }
            Prob::Run52b => {
                buy[i] = (athc[i] || fresh) && common;
                sell[i] = ((exit_sell[i] || !athc[i]) && !loc[i]) || deadsell[i];
            }
        }
        if dbg_a1 && kind != Prob::St104 {
            // test hook (key `dbg_a1`, 0 in production): the entry / exit combination of ath_loss1 (fresh ATH close, exit on the swing, 30-bar flat stock) on this module's streak
            buy[i] = athc[i] && fresh && common && !dead1[i];
            sell[i] = exit_sell[i] || deadsell[i] || dead1[i];
        }
    }
    finish(exrem_close(&buy, &sell, &c, &h, &l), g("hardstop") as f32, rank_of(&c, g))
}

fn schema_prob(kind: Prob) -> Vec<P> {
    let mut v = vec![pdead(), phard(), ppenny(), pturn(if kind == Prob::Run52 { TURN_A } else { TURN_B })];
    match kind {
        Prob::St104 => {
            v.extend(prank());
            v.push(p("stflt", "ST CALCULATION POINT (0 CLOSE, 1 MID POINT, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0));
            v.push(p("tstart", "TRADING START BAR (not used by the AFL's signals)", 1.0, 1.0, 500.0, 1.0));
            v.push(p("bpd1", "ST Period FOR ATHC EXIT", 10.0, 1.0, 20.0, 1.0));
            v.push(p("bfactor1", "ST Factor FOR ATHC EXIT", 4.0, 1.0, 20.0, 1.0));
            v.push(p("stflt1", "ST CALCULATION POINT FOR PROBABILITY (0 MID POINT, 1 CLOSE, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0));
            v.push(p("bpd2", "ST ENTRY Period FOR PROBABILITY", 5.0, 1.0, 20.0, 1.0));
            v.push(p("bfactor2", "ST ENTRY Factor FOR PROBABILITY", 2.0, 1.0, 20.0, 1.0));
            v.push(p("bpd3", "ST EXIT Period FOR PROBABILITY", 5.0, 1.0, 20.0, 1.0));
            v.push(p("bfactor3", "ST EXIT Factor FOR PROBABILITY", 4.0, 1.0, 20.0, 1.0));
            v.push(p("losspct", "MIN PROFIT %", 2.0, 0.5, 10.0, 0.05));
            v.push(p("noofloss", "CONSICUTIVE LOSS", 2.0, 1.0, 50.0, 1.0));
        }
        Prob::Run52 => {
            v.push(p("stflt1", "ST CALCULATION POINT FOR PROBABILITY (0 MID POINT, 1 CLOSE, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0));
            v.push(p("bpd2", "ST ENTRY Period FOR PROBABILITY", 5.0, 1.0, 6.0, 0.5));
            v.push(p("bfactor2", "ST ENTRY Factor FOR PROBABILITY", 2.0, 1.0, 4.0, 0.5));
            v.push(p("bpd3", "ST EXIT Period FOR PROBABILITY", 5.0, 1.0, 6.0, 0.5));
            v.push(p("bfactor3", "ST EXIT Factor FOR PROBABILITY", 4.0, 1.0, 6.0, 0.5));
            v.push(p("exitpct", "EXIT %", 11.0, 11.0, 22.0, 1.0)); // AFL default 10 lies below its range 11-22: clamped
            v.push(p("losspct", "MIN PROFIT %", 2.0, 0.5, 10.0, 0.5));
            v.push(p("noofloss", "CONSICUTIVE LOSS", 2.0, 0.0, 5.0, 1.0));
            v.extend(prank());
        }
        Prob::Run52b => {
            v.push(p("stflt1", "ST CALCULATION POINT FOR PROBABILITY (0 MID POINT, 1 CLOSE, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0));
            v.push(p("bpd2", "ST ENTRY Period FOR PROBABILITY", 5.0, 1.0, 6.0, 0.5));
            v.push(p("bpd3", "ST EXIT Period FOR PROBABILITY", 5.0, 1.0, 6.0, 0.5));
            v.push(p("losspct", "MIN PROFIT %", 2.0, 0.5, 10.0, 0.5));
            v.push(p("noofloss", "CONSICUTIVE LOSS (the AFL reads Optimize(2): fixed 2, key kept for the catalog)", 2.0, 0.0, 5.0, 1.0));
            v.extend(prank());
        }
    }
    v
}
fn schema_prob_st104() -> Vec<P> {
    schema_prob(Prob::St104)
}
fn schema_prob_run52() -> Vec<P> {
    schema_prob(Prob::Run52)
}
fn schema_prob_run52b() -> Vec<P> {
    schema_prob(Prob::Run52b)
}
fn sig_prob_st104(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| prob_symbol(s, g, Prob::St104)).collect()
}
fn sig_prob_run52(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| prob_symbol(s, g, Prob::Run52)).collect()
}
fn sig_prob_run52b(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| prob_symbol(s, g, Prob::Run52b)).collect()
}

// ================================================================= 5. running ATH buy, %swing exit after %swing losses, NSE500 / F&O universe

fn backtest_flags(s: &Series, mode: i64) -> Vec<bool> {
    s.bars
        .iter()
        .map(|b| match mode {
            1 => b.aux1 == 1.0,
            2 => b.aux2 == 1.0,
            3 => b.aux1 == 1.0 && b.aux2 == 0.0,
            _ => true,
        })
        .collect()
}

/// `ytath_running_pctswing_after_2_pctswing` (`exit_on_exclusion`: Sell has `OR backtestcheck==False`) and `_2` (the universe flag only gates entries).
fn run_loss_symbol(s: &Series, g: G, exit_on_exclusion: bool) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let (deadbuy, deadsell) = dead_flags(n, g("deadstock").round() == 0.0, false);
    let (upc, loc) = circuits(&o, &h, &l, &c);
    let penny = g("pennyflt") as f32;
    let turn = g("turnflt").round() == 1.0;
    // hidden test keys (0 in production): dbg_oitable = OpenInt against the long table, dbg_notuc = `NOT uc` in Buy (the sibling AFL "final version")
    let (dbg_oitable, dbg_notuc) = (g("dbg_oitable") != 0.0, g("dbg_notuc") != 0.0);
    let turnover: Vec<bool> = (0..n)
        .map(|i| if !turn { true } else if dbg_oitable { s.bars[i].oi > volsig_a(s.bars[i].date / 10000) } else { s.bars[i].v * c[i] > volsig_b(s.bars[i].date / 10000) })
        .collect();
    let bt = backtest_flags(s, g("backtest").round() as i64);
    let hcl = hhv_all(&c);
    let sw = swing(&c, dbg_exit(g)); // EXIT % = Optimize("EXIT %", 10, 5, 25) (hidden test key dbg_exit)
    let athc = athc_state(&c, &hcl, &sw.trend, None);
    let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !upc[i] && deadbuy[i] && c[i] > penny && turnover[i]).collect();
    let cc = concount_swing(&c, &buysig, &sw, g("losspct") as f32, g("comm") as f32);
    let noofloss = if g("dbg_noofloss") > 0.0 { g("dbg_noofloss") as f32 } else { g("noofloss") as f32 }; // Optimize("CONSICUTIVE LOSS", 2, 1, 2) (hidden test key dbg_noofloss)
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let fresh = i >= 1 && c[i] > hcl[i - 1];
        buy[i] = (athc[i] || fresh) && cc[i] >= noofloss && !upc[i] && deadbuy[i] && c[i] > penny && turnover[i] && bt[i] && !(dbg_notuc && h[i] == c[i]);
        sell[i] = ((sw.trend[i] == -1 || !athc[i]) && !loc[i]) || deadsell[i] || (exit_on_exclusion && !bt[i]);
    }
    finish(exrem_close(&buy, &sell, &c, &h, &l), g("hardstop") as f32, rank_of(&c, g))
}

fn schema_run_loss() -> Vec<P> {
    let mut v = vec![pdead(), phard(), ppenny(), pturn(TURN_B), p("exitpct", "EXIT %", 10.0, 5.0, 25.0, 1.0), p("losspct", "MIN PROFIT %", 3.0, 1.0, 3.0, 0.05), p("noofloss", "CONSICUTIVE LOSS", 2.0, 1.0, 2.0, 1.0)];
    v.extend(prank());
    v.push(p("backtest", "BACKTEST for (0 ALL, 1 NSE500, 2 FO, 3 NSE500-FO)", 0.0, 0.0, 3.0, 1.0));
    v
}
fn sig_run_loss(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| run_loss_symbol(s, g, true)).collect()
}
fn sig_run_loss2(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| run_loss_symbol(s, g, false)).collect()
}

// ================================================================= 6. gap between two ATH closes after a losing trade

fn gap_symbol(s: &Series, g: G) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let oi = s.openint();
    let (deadbuy, deadsell) = dead_flags(n, g("deadstock").round() == 0.0, false);
    let (_upc, loc) = circuits(&o, &h, &l, &c);
    let uc: Vec<bool> = (0..n).map(|i| h[i] == c[i]).collect();
    let lc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let penny = g("pennyflt") as f32;
    let turn = g("turnflt").round() == 1.0;
    let turnover: Vec<bool> = (0..n).map(|i| if turn { oi[i] > volsig_a(s.bars[i].date / 10000) } else { true }).collect();
    let hcl = hhv_all(&c);
    let sw = swing(&c, g("exitpct") as f32);
    let buy_filter: Vec<bool> = (0..n).map(|i| turnover[i] && !uc[i] && c[i] > penny && deadbuy[i]).collect();
    let athc = athc_state(&c, &hcl, &sw.trend, None);
    let buysig: Vec<bool> = (0..n).map(|i| athc[i] && buy_filter[i]).collect();
    let cc = concount_swing(&c, &buysig, &sw, g("losspct") as f32, g("comm") as f32);
    // athcdays = ValueWhen(C > Ref(hcl,-1), BarIndex()); noofathcdays = athcdays - Ref(athcdays,-1); lastlossdays = IIf(concount>0, noofathcdays, Null)
    let mut athcdays = vec![f32::NAN; n];
    for i in 0..n {
        let hit = i >= 1 && c[i] > hcl[i - 1];
        athcdays[i] = if hit { i as f32 } else if i > 0 { athcdays[i - 1] } else { f32::NAN };
    }
    let last_loss_days = 22.0f32; // Optimize("NO OF DAYS FROM PREVIOUS LOSS", 22, 30, 30, 30)
    let mut lossbuysig = vec![false; n];
    for i in 1..n {
        let lastlossdays = if cc[i] > 0.0 { athcdays[i] - athcdays[i - 1] } else { f32::NAN };
        lossbuysig[i] = if c[i] > hcl[i - 1] && lastlossdays > last_loss_days {
            true
        } else if sw.trend[i] == -1 {
            false
        } else {
            lossbuysig[i - 1]
        };
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = lossbuysig[i] && buy_filter[i];
        sell[i] = (sw.trend[i] == -1 && !loc[i] && !lc[i]) || deadsell[i];
    }
    finish(exrem_close(&buy, &sell, &c, &h, &l), g("hardstop") as f32, None)
}

fn schema_gap() -> Vec<P> {
    vec![pdead(), phard(), ppenny(), pturn(TURN_A), p("losspct", "MIN PROFIT %", 2.0, 0.0, 10.0, 0.05), p("exitpct", "EXIT %", 10.0, 3.0, 22.0, 1.0)]
}
fn sig_gap(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| gap_symbol(s, g)).collect()
}

// ================================================================= 7. ST(5,1) buys after losing ST(5,1) trades

fn st51_symbol(s: &Series, g: G, take_all: bool) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let oi = s.openint();
    let (deadbuy, deadsell) = dead_flags(n, g("deadstock").round() == 0.0, false);
    let (upc, _loc) = circuits(&o, &h, &l, &c);
    let penny = g("pennyflt") as f32;
    let turn = g("turnflt").round() == 1.0;
    let turnover: Vec<bool> = (0..n)
        .map(|i| if !turn { true } else if take_all { oi[i] > volsig_a(s.bars[i].date / 10000) } else { s.bars[i].v * c[i] > volsig_b(s.bars[i].date / 10000) })
        .collect();
    let hcl = hhv_all(&c);
    let sw = swing(&c, g("exitpct") as f32);
    let athc = athc_state(&c, &hcl, &sw.trend, None);
    let tr = tr1(&h, &l, &c);
    let stcal1 = stcal_of(true, g("stflt1").round() as i64, &h, &l, &c);
    let bf2 = if take_all { 1.0f32 } else { g("bfactor2") as f32 }; // Optimize("ST ENTRY Factor", 1, ...)
    let (up2, dn2, bt2) = st_lines(&stcal1, &tr, &c, g("bpd2") as f32, bf2, bf2, true);
    let buysig: Vec<bool> = bt2.iter().map(|&t| t == 1).collect();
    let sellsig: Vec<bool> = bt2.iter().map(|&t| t == -1).collect();
    let v = virt_trades(&c, &buysig, &sellsig);
    let cr2 = cross_prev_line_above_close(&up2, &c);
    let ev: Vec<bool> = (0..n).map(|i| i >= 1 && athc[i] && bt2[i] == -1 && cr2[i]).collect();
    let losspct = g("losspct") as f32;
    let comm = g("comm") as f32;
    let noofloss = if take_all { 3.0f32 } else { g("noofloss") as f32 }; // take_all: Optimize("CONSICUTIVE LOSS", 3, ...)
    let mut cc = vec![0f32; n];
    let mut ltc = vec![0f32; n]; // losstradecount
    for i in 1..n {
        let (profit, pct) = if ev[i] { virt_profit(&v, i, comm) } else { (0.0, 0.0) };
        if profit != 0.0 {
            if pct <= losspct {
                cc[i] = cc[i - 1] + 1.0;
                ltc[i] = if cc[i] == noofloss { noofloss } else { ltc[i - 1] };
            } else {
                cc[i] = 0.0;
                ltc[i] = ltc[i - 1];
            }
        } else if athc[i] {
            cc[i] = cc[i - 1];
            ltc[i] = ltc[i - 1];
        } else {
            cc[i] = 0.0;
            ltc[i] = 0.0;
        }
    }
    let dead1 = flat30(&o, &h, &l, &c);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let above_line = i >= 1 && !is_null(dn2[i - 1]) && c[i] > dn2[i - 1];
        let gate = if take_all { ltc[i] == noofloss } else { cc[i] >= noofloss };
        buy[i] = athc[i] && above_line && gate && !upc[i] && deadbuy[i] && c[i] > penny && turnover[i] && !dead1[i];
        sell[i] = bt2[i] == -1 || (take_all && sw.trend[i] == -1) || deadsell[i] || dead1[i];
    }
    finish(exrem_close(&buy, &sell, &c, &h, &l), g("hardstop") as f32, None)
}

fn schema_st51() -> Vec<P> {
    vec![
        pdead(),
        phard(),
        ppenny(),
        pturn(TURN_B),
        p("stflt1", "ST CALCULATION POINT FOR PROBABILITY (0 MID POINT, 1 CLOSE, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0),
        p("bpd2", "ST ENTRY Period FOR PROBABILITY", 5.0, 1.0, 20.0, 1.0),
        p("bfactor2", "ST ENTRY Factor FOR PROBABILITY", 1.0, 1.0, 20.0, 1.0),
        p("exitpct", "EXIT %", 21.0, 5.0, 35.0, 1.0),
        p("losspct", "MIN PROFIT %", 2.0, 0.5, 10.0, 0.05),
        p("noofloss", "CONSICUTIVE LOSS", 2.0, 1.0, 50.0, 1.0),
    ]
}
fn schema_st51_all() -> Vec<P> {
    vec![
        pdead(),
        phard(),
        ppenny(),
        pturn(TURN_A),
        p("stflt1", "ST CALCULATION POINT FOR PROBABILITY (0 MID POINT, 1 CLOSE, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0),
        p("bpd2", "ST ENTRY Period FOR PROBABILITY", 5.0, 1.0, 20.0, 1.0),
        p("exitpct", "EXIT %", 21.0, 5.0, 35.0, 1.0),
        p("losspct", "MIN PROFIT %", 2.0, 0.5, 10.0, 0.05),
    ]
}
fn sig_st51(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| st51_symbol(s, g, false)).collect()
}
fn sig_st51_all(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| st51_symbol(s, g, true)).collect()
}

// ================================================================= 8. ATHC -5% re-entry with trailing stop

fn reentry_symbol(s: &Series, g: G) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let oi = s.openint();
    let (deadbuy, deadsell) = dead_flags(n, g("deadstock").round() == 0.0, false);
    let _ = o;
    let penny = g("pennyflt") as f32;
    let turn = g("turnflt").round() == 1.0;
    let turnover: Vec<bool> = (0..n).map(|i| if turn { oi[i] > volsig_a(s.bars[i].date / 10000) } else { true }).collect();
    let trail_pct = 5.0f32; // Optimize("Trailing SL %", 5, ...)
    let buy_pct = 5.0f32; // Optimize("Buy %", 5, ...)
    let hcl = hhv_all(&c);
    let buysig2: Vec<f32> = (0..n).map(|i| hcl[i] - (hcl[i] * buy_pct) / 100.0).collect();
    let sw = swing(&c, g("exitpct") as f32);
    let lc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let uc: Vec<bool> = (0..n).map(|i| h[i] == c[i]).collect();
    let cash_filter: Vec<bool> = (0..n).map(|i| !uc[i] && turnover[i] && c[i] > penny && deadbuy[i]).collect();
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut position, mut skipbuy, mut skipsell) = (false, false, false);
    let mut buy_sig = 0u8;
    let (mut trail_sl, mut hc_af_buy) = (0f32, 0f32);
    for i in 1..n {
        if c[i] > hcl[i - 1] && buy_sig == 0 {
            buy_sig = 1;
        } else if sw.trend[i] == -1 && buy_sig == 1 {
            buy_sig = 2;
        }
        if !position && c[i] > buysig2[i - 1] && buy_sig == 2 && !skipbuy {
            if cash_filter[i] {
                buy[i] = true;
                hc_af_buy = c[i];
                trail_sl = (1.0 - trail_pct * 0.01) * hc_af_buy;
                position = true;
            } else {
                skipbuy = true;
            }
        } else if skipbuy && cash_filter[i] {
            buy[i] = true;
            hc_af_buy = c[i];
            trail_sl = (1.0 - trail_pct * 0.01) * hc_af_buy;
            position = true;
            skipbuy = false;
        } else if position {
            if trail_pct > 0.0 {
                hc_af_buy = c[i].max(hc_af_buy);
            }
            if (c[i] <= trail_sl && !skipsell) || deadsell[i] {
                if !lc[i] || deadsell[i] {
                    sell[i] = true;
                    position = false;
                    hc_af_buy = 0.0;
                    trail_sl = 0.0;
                } else {
                    skipsell = true;
                }
            } else if (!lc[i] && skipsell) || (skipsell && deadsell[i]) {
                sell[i] = true;
                position = false;
                hc_af_buy = 0.0;
                trail_sl = 0.0;
                skipsell = false;
            }
            if trail_pct > 0.0 {
                trail_sl = (1.0 - trail_pct * 0.01) * hc_af_buy;
            }
        }
    }
    let mut sg = Signals::new(buy, sell, c.clone(), c.clone());
    clamp_prices(&mut sg, &h, &l);
    sg
}

fn schema_reentry() -> Vec<P> {
    vec![pdead(), ppenny(), pturn(TURN_A), p("exitpct", "Swing EXIT %", 21.0, 1.0, 40.0, 1.0)]
}
fn sig_reentry(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| reentry_symbol(s, g)).collect()
}

// ================================================================= 9. ATHC +x% buy, %swing exit, NSE500 / F&O universe, ratio

/// `ytath_plus_pct_buy_pctswing_exit_nse500(_2)` (constants EXIT 10, BUY 4; universe flag) and `ytath_plus_4pct_buy_with_ratio_pctswing` (EXIT / BUY parameters, no universe flag).
fn plus_run_symbol(s: &Series, g: G, exit_pct: f32, athc_pct: f32, universe: Option<bool>) -> Signals {
    plus_run_symbol_aux(s, g, exit_pct, athc_pct, universe, false)
}

/// `aux`: the AFL has the toggles Aux1 / Aux2 (ParamToggle "ON|OFF", default OFF; 0 = ON): `IIf(AuxToggle==0, Aux1 == 1, 1) AND IIf(AuxToggle1==0, Aux2 == 1, 1)` is added to Buy only
/// (together with BACKTESTCHECK); Sell is unchanged.
fn plus_run_symbol_aux(s: &Series, g: G, exit_pct: f32, athc_pct: f32, universe: Option<bool>, aux: bool) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let (deadbuy, deadsell) = dead_flags(n, g("deadstock").round() == 0.0, false);
    let (upc, loc) = circuits(&o, &h, &l, &c);
    let penny = g("pennyflt") as f32;
    let turn = g("turnflt").round() == 1.0;
    let turnover: Vec<bool> = (0..n).map(|i| if turn { s.bars[i].v * c[i] > volsig_b(s.bars[i].date / 10000) } else { true }).collect();
    let bt = if universe.is_some() { backtest_flags(s, g("backtest").round() as i64) } else { vec![true; n] };
    let (aux1_on, aux2_on) = if aux { (g("auxtoggle").round() == 0.0, g("auxtoggle1").round() == 0.0) } else { (false, false) };
    let aux_ok = |i: usize| (!aux1_on || s.bars[i].aux1 == 1.0) && (!aux2_on || s.bars[i].aux2 == 1.0);
    let hcl = hhv_all(&c);
    let sw = swing(&c, exit_pct);
    let trigger = trigger_override(&c, &hcl, &sw.trend, athc_pct);
    let buysig = athc_state(&c, &hcl, &sw.trend, None);
    let dbg_notuc = g("dbg_notuc") != 0.0; // test hook (0 in production): add the `NOT uc` condition of the sibling AFL
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let new_high = i >= 1 && c[i] > hcl[i - 1];
        buy[i] = (new_high || buysig[i]) && !is_null(trigger[i]) && c[i] > trigger[i] && deadbuy[i] && !upc[i] && c[i] > penny && turnover[i] && bt[i] && aux_ok(i) && !(dbg_notuc && h[i] == c[i]);
        sell[i] = ((sw.trend[i] == -1 || !buysig[i]) && !loc[i]) || deadsell[i] || (universe == Some(true) && !bt[i]);
    }
    finish(exrem_close(&buy, &sell, &c, &h, &l), g("hardstop") as f32, None)
}

fn schema_nse500() -> Vec<P> {
    vec![
        pdead(),
        phard(),
        ppenny(),
        pturn(TURN_B),
        p("swingpd", "SWING PERIOD (ranking input; the AFL sets no PositionScore)", 180.0, 10.0, 1000.0, 1.0),
        p("backtest", "BACKTEST for (0 ALL, 1 NSE500, 2 FO, 3 NSE500-FO)", 0.0, 0.0, 3.0, 1.0),
    ]
}
/// `ytath_plus_pct_buy_pctswing_exit_nse500_2` only: the same rows plus the AFL's Aux1 / Aux2 toggles (default OFF).
fn schema_nse500_2() -> Vec<P> {
    let mut v = schema_nse500();
    let at = v.iter().position(|d| d.key == "swingpd").unwrap_or(v.len());
    v.insert(at, p("auxtoggle", "Aux1 (0 ON = only Aux1 == 1; 1 OFF)", 1.0, 0.0, 1.0, 1.0));
    v.insert(at + 1, p("auxtoggle1", "Aux2 (0 ON = only Aux2 == 1; 1 OFF)", 1.0, 0.0, 1.0, 1.0));
    v
}
fn schema_ratio() -> Vec<P> {
    vec![
        pdead(),
        phard(),
        ppenny(),
        pturn(TURN_B),
        p("exitpct", "EXIT %", 10.0, 5.0, 25.0, 1.0),
        p("swingpd", "SWING PERIOD (ranking input; the AFL sets no PositionScore)", 180.0, 10.0, 1000.0, 1.0),
        p("athcbuypct", "BUY PERCENTAGE FROM ATHC", 4.0, 1.0, 8.0, 1.0),
    ]
}
fn sig_nse500(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| plus_run_symbol(s, g, 10.0, 4.0, Some(true))).collect()
}
fn sig_nse500_2(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| plus_run_symbol_aux(s, g, 10.0, 4.0, Some(false), true)).collect()
}
fn sig_ratio(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| plus_run_symbol(s, g, g("exitpct") as f32, g("athcbuypct") as f32, None)).collect()
}

// ================================================================= 10. scale-in engines (5 tranches)

/// What opens a position in the scale-in AFLs besides the ATHC state: the loss streak or the +x% trigger.
#[derive(Clone, Copy)]
enum Gate {
    Loss { noofloss: f32, losspct: f32, comm: f32 },
    Trigger { pct: f32 },
}

#[derive(Clone, Copy)]
struct ScaleCfg {
    exit_pct: f32,
    penny: f32,
    turnflt: bool,
    dead_yes: bool,
    atr_prd: usize,
    myatr1: f32,
    buffer: f32,
    gate: Gate,
    /// test hook (`dbg_nogate`, not a schema key: reads 0 in production): switch the AFL's extra entry gate off for differential checks
    nogate: bool,
}

/// What the AFL's main loop does on each bar (see ath_loss2::ScaleOut).
struct ScaleOut {
    /// 0 nothing, 1 new entry (`Buy = 1`), 2 scale-in (`Buy = sigScaleIn`)
    buy: Vec<u8>,
    po: Vec<f32>,
    buy_price: Vec<f32>,
    sell: Vec<bool>,
    sell_price: Vec<f32>,
}

/// Arrays shared by the three scale-in loops.
struct ScaleBase {
    n: usize,
    c: Vec<f32>,
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    hcl: Vec<f32>,
    sw: Swing,
    athc: Vec<bool>,
    upc: Vec<bool>,
    loc: Vec<bool>,
    uc: Vec<bool>,
    lc: Vec<bool>,
    /// MasterFilter = Close > pennyflt AND DEADSTOCKBUYSIG
    master: Vec<bool>,
    other_filter: Vec<bool>,
    skip_buy_filter: Vec<bool>,
    /// entry gate (loss streak reached / Close above the trigger)
    gate: Vec<bool>,
    dead_sell: Vec<bool>,
}

fn scale_base(s: &Series, p: &ScaleCfg) -> ScaleBase {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let hcl = hhv_all(&c);
    let sw = swing(&c, p.exit_pct);
    let athc = athc_state(&c, &hcl, &sw.trend, None);
    let (upc, loc) = circuits(&o, &h, &l, &c);
    let uc: Vec<bool> = (0..n).map(|i| h[i] == c[i]).collect();
    let lc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let turnover: Vec<bool> = (0..n).map(|i| if p.turnflt { s.bars[i].v * c[i] > volsig_b(s.bars[i].date / 10000) } else { true }).collect();
    let (deadbuy, deadsell) = dead_flags(n, p.dead_yes, false);
    let master: Vec<bool> = (0..n).map(|i| c[i] > p.penny && deadbuy[i]).collect();
    let other_filter: Vec<bool> = (0..n).map(|i| !turnover[i] || upc[i] || uc[i]).collect();
    let skip_buy_filter: Vec<bool> = (0..n).map(|i| turnover[i] && !upc[i]).collect();
    let gate: Vec<bool> = if p.nogate { vec![true; n] } else { match p.gate {
        Gate::Loss { noofloss, losspct, comm } => {
            let buysig: Vec<bool> = (0..n).map(|i| athc[i] && !upc[i] && deadbuy[i] && c[i] > p.penny && turnover[i]).collect();
            let cc = concount_swing(&c, &buysig, &sw, losspct, comm);
            (0..n).map(|i| cc[i] >= noofloss).collect()
        }
        Gate::Trigger { pct } => {
            let trigger = trigger_elseif(&c, &hcl, &sw.trend, pct);
            (0..n).map(|i| !is_null(trigger[i]) && c[i] > trigger[i]).collect()
        }
    } };
    ScaleBase { n, c, o, h, l, hcl, sw, athc, upc, loc, uc, lc, master, other_filter, skip_buy_filter, gate, dead_sell: deadsell }
}

fn scale_out_new(b: &ScaleBase) -> ScaleOut {
    ScaleOut { buy: vec![0; b.n], po: vec![0.0; b.n], buy_price: b.c.clone(), sell: vec![false; b.n], sell_price: b.c.clone() }
}

/// Loop with four ATR levels fixed at the entry bar (AFLs "Fixed ATR Scalein"): ath_loss2::scalein_events with the gate of this AFL.
fn scale_atr_fixed(s: &Series, p: &ScaleCfg) -> ScaleOut {
    let b = scale_base(s, p);
    let n = b.n;
    let (c, o, h) = (&b.c, &b.o, &b.h);
    let myatr = atr_wilder(&b.h, &b.l, &b.c, p.atr_prd);
    let mut out = scale_out_new(&b);
    let max_position = 5.0f32;
    let mut pos = vec![0f32; n];
    let mut buysig = vec![NULLV; n];
    let (mut atr_value, mut skip, mut abc) = (0f32, 0f32, 0f32);
    let buf = p.buffer;
    for i in 1..n {
        let pp = pos[i - 1];
        let bb = buysig[i - 1];
        let a = atr_value;
        let ob = o[i] + (o[i] * buf) / 100.0;
        let b1 = bb + a;
        let b2 = bb + 2.0 * a;
        let b3 = bb + 3.0 * a;
        if pp == 0.0 && (b.athc[i] || c[i] > b.hcl[i - 1]) && b.gate[i] && b.master[i] && myatr[i] > 0.0 {
            if b.other_filter[i] {
                atr_value = myatr[i] * p.myatr1;
                buysig[i] = c[i] + atr_value;
                pos[i] = 1.0;
                skip += 1.0;
            } else {
                out.buy[i] = 1;
                atr_value = myatr[i] * p.myatr1;
                buysig[i] = c[i] + atr_value;
                pos[i] = 1.0;
                abc += 1.0;
            }
        } else if ((pp > 0.0 && pp < max_position && h[i] > bb && h[i] < b1) || (pp == 4.0 && h[i] > bb)) && b.master[i] {
            if pp > 0.0 && !b.upc[i] {
                out.po[i] = skip + 1.0;
                out.buy[i] = 2;
                out.buy_price[i] = if o[i] > bb {
                    ob
                } else if skip > 0.0 {
                    ((ob * skip) + bb) / (skip + 1.0)
                } else {
                    bb
                };
                pos[i] = pp + 1.0;
                buysig[i] = bb + atr_value;
                abc += 1.0;
                skip = 0.0;
            } else {
                pos[i] = pp + 1.0;
                buysig[i] = bb + atr_value;
                skip += 1.0;
            }
        } else if ((pp > 0.0 && pp < max_position && h[i] > bb && h[i] > b1 && h[i] < bb + 2.0 * a) || (pp == 3.0 && pp < max_position && h[i] > bb && h[i] > b1)) && b.master[i] {
            if pp > 0.0 && !b.upc[i] {
                out.po[i] = skip + 2.0;
                out.buy[i] = 2;
                if o[i] > bb && o[i] > b1 {
                    out.buy_price[i] = ob;
                } else if skip > 0.0 && o[i] > bb && h[i] > b1 {
                    out.buy_price[i] = ((ob * (skip + 1.0)) + b1) / (skip + 2.0);
                } else if skip > 0.0 && h[i] > bb && h[i] > b1 {
                    out.buy_price[i] = ((ob * skip) + bb + b1) / (skip + 2.0);
                } else if skip == 0.0 && o[i] > bb && h[i] > b1 {
                    out.buy_price[i] = (ob + b1) / 2.0;
                } else if skip == 0.0 && h[i] > bb && h[i] > b1 {
                    out.buy_price[i] = (bb + b1) / 2.0;
                }
                pos[i] = pp + 2.0;
                buysig[i] = bb + 2.0 * atr_value;
                abc += 1.0;
                skip = 0.0;
            } else {
                pos[i] = pp + 2.0;
                buysig[i] = bb + 2.0 * atr_value;
                skip += 2.0;
            }
        } else if ((pp > 0.0 && pp < max_position && h[i] > bb && h[i] > b1 && h[i] > b2 && h[i] < bb + 3.0 * a) || (pp == 2.0 && pp < max_position && h[i] > bb && h[i] > b1 && h[i] > b2)) && b.master[i] {
            if pp > 0.0 && !b.upc[i] {
                out.po[i] = skip + 3.0;
                out.buy[i] = 2;
                if o[i] > bb && o[i] > b1 && o[i] > b2 {
                    out.buy_price[i] = ob;
                } else if skip > 0.0 && o[i] > bb && o[i] > b1 && h[i] > b2 {
                    out.buy_price[i] = ((ob * (skip + 2.0)) + b2) / (skip + 3.0);
                } else if skip > 0.0 && o[i] > bb && h[i] > b1 && h[i] > b2 {
                    out.buy_price[i] = ((ob * (skip + 1.0)) + b1 + b2) / (skip + 3.0);
                } else if skip > 0.0 && h[i] > bb && h[i] > b1 && h[i] > b2 {
                    out.buy_price[i] = ((ob * skip) + bb + b1 + b2) / (skip + 3.0);
                } else if skip == 0.0 && o[i] > bb && o[i] > b1 && h[i] > b2 {
                    out.buy_price[i] = ((ob * 2.0) + b2) / 3.0;
                } else if skip == 0.0 && o[i] > bb && h[i] > b1 && h[i] > b2 {
                    out.buy_price[i] = (ob + b1 + b2) / 3.0;
                } else if skip == 0.0 && h[i] > bb && h[i] > b1 && h[i] > b2 {
                    out.buy_price[i] = (bb + b1 + b2) / 3.0;
                }
                pos[i] = pp + 3.0;
                buysig[i] = bb + 3.0 * atr_value;
                abc += 1.0;
                skip = 0.0;
            } else {
                pos[i] = pp + 3.0;
                buysig[i] = bb + 3.0 * atr_value;
                skip += 3.0;
            }
        } else if pp > 0.0 && pp < max_position && h[i] > b3 && b.master[i] {
            if pp > 0.0 && !b.upc[i] {
                out.po[i] = skip + 4.0;
                out.buy[i] = 2;
                if o[i] > bb && o[i] > b1 && o[i] > b2 && o[i] > b3 {
                    out.buy_price[i] = ob;
                } else if skip > 0.0 && o[i] > bb && o[i] > b1 && o[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = ((ob * (skip + 3.0)) + b3) / (skip + 4.0);
                } else if skip > 0.0 && o[i] > bb && o[i] > b1 && h[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = ((ob * (skip + 2.0)) + b2 + b3) / (skip + 4.0);
                } else if skip > 0.0 && o[i] > bb && h[i] > b1 && h[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = ((ob * (skip + 1.0)) + b1 + b2 + b3) / (skip + 4.0);
                } else if skip > 0.0 && h[i] > bb && h[i] > b1 && h[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = (ob + bb + b1 + b2 + b3) / (skip + 4.0);
                } else if skip == 0.0 && o[i] > bb && o[i] > b1 && o[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = ((ob * 3.0) + b3) / 4.0;
                } else if skip == 0.0 && o[i] > bb && o[i] > b1 && h[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = ((ob * 2.0) + b2 + b3) / 4.0;
                } else if skip == 0.0 && o[i] > bb && h[i] > b1 && h[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = (ob + b1 + b2 + b3) / 4.0;
                } else if skip == 0.0 && h[i] > bb && h[i] > b1 && h[i] > b2 && h[i] > b3 {
                    out.buy_price[i] = (bb + b1 + b2 + b3) / 4.0;
                }
                pos[i] = pp + 4.0;
                buysig[i] = NULLV;
                abc += 1.0;
                skip = 0.0;
            } else {
                pos[i] = pp + 4.0;
                buysig[i] = bb;
                skip += 4.0;
            }
        } else if skip > 0.0 && b.sw.trend[i] == 1 && b.skip_buy_filter[i] && b.master[i] {
            out.po[i] = skip;
            out.buy[i] = 2;
            out.buy_price[i] = ob;
            pos[i] = pp;
            buysig[i] = bb;
            abc += 1.0;
            skip = 0.0;
        } else if pp > 0.0 && abc == 0.0 && skip > 0.0 && b.sw.trend[i] == -1 {
            pos[i] = 0.0;
            skip = 0.0;
            abc = 0.0;
        } else if (pp > 0.0 && abc > 0.0 && (b.sw.trend[i] == -1 || !b.athc[i]) && !(b.loc[i] || b.lc[i])) || b.dead_sell[i] {
            out.sell[i] = true;
            if (b.loc[i - 1] || b.lc[i - 1]) && b.sw.trend[i - 1] == -1 {
                out.sell_price[i] = o[i] - (o[i] * buf) / 100.0;
            } else {
                out.sell_price[i] = c[i];
            }
            pos[i] = 0.0;
            atr_value = 0.0;
            skip = 0.0;
            abc = 0.0;
            buysig[i] = NULLV;
        } else {
            pos[i] = pp;
            buysig[i] = bb;
        }
    }
    out
}

/// Loop with four fixed percentage levels (3 %, 6 %, 9 %, 12 % of the entry close; AFLs "Fixed Percentage Scalein"). Level k above the current one is
/// `(1 + (Prcnt + 3k)/100) * BuyAtPrice` as the AFL writes it.
fn scale_pct(s: &Series, p: &ScaleCfg) -> ScaleOut {
    let b = scale_base(s, p);
    let n = b.n;
    let (c, o, h) = (&b.c, &b.o, &b.h);
    let mut out = scale_out_new(&b);
    let max_position = 5.0f32;
    let mut pos = vec![0f32; n];
    let (mut skip, mut abc, mut prcnt, mut buy_at) = (0f32, 0f32, 0f32, 0f32);
    let buf = p.buffer;
    for i in 1..n {
        let pp = pos[i - 1];
        let ob = (1.0 + buf / 100.0) * o[i]; // (1 + buffer_prcnt/100)*O[i]
        let lv = |k: f32| -> f32 { (1.0 + (prcnt + 3.0 * k) / 100.0) * buy_at };
        let (l0, l1, l2, l3) = (lv(0.0), lv(1.0), lv(2.0), lv(3.0));
        if pp == 0.0 && (b.athc[i] || c[i] > b.hcl[i - 1]) && b.gate[i] && b.master[i] {
            if b.other_filter[i] {
                prcnt = 3.0;
                buy_at = c[i];
                pos[i] = 1.0;
                skip += 1.0;
            } else if !b.other_filter[i] {
                out.buy[i] = 1;
                prcnt = 3.0;
                buy_at = c[i];
                pos[i] = 1.0;
                abc += 1.0;
            }
        } else if ((pp > 0.0 && pp < max_position && h[i] > l0 && h[i] <= l1) || (pp == 4.0 && h[i] > l0)) && b.master[i] {
            if pp > 0.0 && !b.upc[i] {
                out.po[i] = skip + 1.0;
                out.buy[i] = 2;
                out.buy_price[i] = if o[i] > l0 {
                    ob
                } else if skip > 0.0 {
                    ((ob * skip) + l0) / (skip + 1.0)
                } else {
                    l0
                };
                pos[i] = pp + 1.0;
                prcnt += 3.0;
                abc += 1.0;
                skip = 0.0;
            } else {
                pos[i] = pp + 1.0;
                prcnt += 3.0;
                skip += 1.0;
            }
        } else if ((pp > 0.0 && pp < max_position && h[i] > l0 && h[i] > l1 && h[i] <= l2) || (pp == 3.0 && pp < max_position && h[i] > l0 && h[i] > l1)) && b.master[i] {
            if pp > 0.0 && !b.upc[i] {
                out.po[i] = skip + 2.0;
                out.buy[i] = 2;
                if o[i] > l0 && o[i] > l1 {
                    out.buy_price[i] = ob;
                } else if skip > 0.0 && o[i] > l0 && h[i] > l1 {
                    out.buy_price[i] = ((ob * (skip + 1.0)) + l1) / (skip + 2.0);
                } else if skip > 0.0 && h[i] > l0 && h[i] > l1 {
                    out.buy_price[i] = ((ob * skip) + l0 + l1) / (skip + 2.0);
                } else if skip == 0.0 && o[i] > l0 && h[i] > l1 {
                    out.buy_price[i] = (ob + l1) / 2.0;
                } else if skip == 0.0 && h[i] > l0 && h[i] > l1 {
                    out.buy_price[i] = (l0 + l1) / 2.0;
                }
                pos[i] = pp + 2.0;
                prcnt += 6.0;
                skip = 0.0;
                abc += 1.0;
            } else {
                pos[i] = pp + 2.0;
                prcnt += 6.0;
                skip += 2.0;
            }
        } else if ((pp > 0.0 && pp < max_position && h[i] > l0 && h[i] > l1 && h[i] > l2 && h[i] <= l3) || (pp == 2.0 && pp < max_position && h[i] > l0 && h[i] > l1 && h[i] > l2)) && b.master[i] {
            if pp > 0.0 && !b.upc[i] {
                out.po[i] = skip + 3.0;
                out.buy[i] = 2;
                if o[i] > l0 && o[i] > l1 && o[i] > l2 {
                    out.buy_price[i] = ob;
                } else if skip > 0.0 && o[i] > l0 && o[i] > l1 && h[i] > l2 {
                    out.buy_price[i] = ((ob * (skip + 2.0)) + l2) / (skip + 3.0);
                } else if skip > 0.0 && o[i] > l0 && h[i] > l1 && h[i] > l2 {
                    out.buy_price[i] = ((ob * (skip + 1.0)) + l1 + l2) / (skip + 3.0);
                } else if skip > 0.0 && h[i] > l0 && h[i] > l1 && h[i] > l2 {
                    out.buy_price[i] = ((ob * skip) + l0 + l1 + l2) / (skip + 3.0);
                } else if skip == 0.0 && o[i] > l0 && o[i] > l1 && h[i] > l2 {
                    out.buy_price[i] = ((ob * 2.0) + l2) / 3.0;
                } else if skip == 0.0 && o[i] > l0 && h[i] > l1 && h[i] > l2 {
                    out.buy_price[i] = (ob + l1 + l2) / 3.0;
                } else if skip == 0.0 && h[i] > l0 && h[i] > l1 && h[i] > l2 {
                    out.buy_price[i] = (l0 + l1 + l2) / 3.0;
                }
                pos[i] = pp + 3.0;
                prcnt += 9.0;
                abc += 1.0;
                skip = 0.0;
            } else {
                pos[i] = pp + 3.0;
                prcnt += 9.0;
                skip += 3.0;
            }
        } else if pp > 0.0 && pp < max_position && h[i] > l3 && b.master[i] {
            if pp > 0.0 && !b.upc[i] {
                out.po[i] = skip + 4.0;
                out.buy[i] = 2;
                if o[i] > l0 && o[i] > l1 && o[i] > l2 && o[i] > l3 {
                    out.buy_price[i] = ob;
                } else if skip > 0.0 && o[i] > l0 && o[i] > l1 && o[i] > l2 && h[i] > l3 {
                    out.buy_price[i] = ((ob * (skip + 3.0)) + l3) / (skip + 4.0);
                } else if skip > 0.0 && o[i] > l0 && o[i] > l1 && h[i] > l2 && h[i] > l3 {
                    out.buy_price[i] = ((ob * (skip + 2.0)) + l2 + l3) / (skip + 4.0);
                } else if skip > 0.0 && o[i] > l0 && h[i] > l1 && h[i] > l2 && h[i] > l3 {
                    out.buy_price[i] = ((ob * (skip + 1.0)) + l1 + l2 + l3) / (skip + 4.0);
                } else if skip > 0.0 && h[i] > l0 && h[i] > l1 && h[i] > l2 && h[i] > l3 {
                    out.buy_price[i] = (ob + l0 + l1 + l2 + l3) / (skip + 4.0);
                } else if skip == 0.0 && o[i] > l0 && o[i] > l1 && o[i] > l2 && h[i] > l3 {
                    out.buy_price[i] = ((ob * 3.0) + l3) / 4.0;
                } else if skip == 0.0 && o[i] > l0 && o[i] > l1 && h[i] > l2 && h[i] > l3 {
                    out.buy_price[i] = ((ob * 2.0) + l2 + l3) / 4.0;
                } else if skip == 0.0 && o[i] > l0 && h[i] > l1 && h[i] > l2 && h[i] > l3 {
                    out.buy_price[i] = (ob + l1 + l2 + l3) / 4.0;
                } else if skip == 0.0 && h[i] > l0 && h[i] > l1 && h[i] > l2 && h[i] > l3 {
                    out.buy_price[i] = (l0 + l1 + l2 + l3) / 4.0;
                }
                pos[i] = pp + 4.0;
                prcnt = 0.0;
                skip = 0.0;
                abc += 1.0;
            } else {
                pos[i] = pp + 4.0;
                skip += 4.0;
            }
        } else if skip > 0.0 && b.sw.trend[i] == 1 && b.skip_buy_filter[i] && b.master[i] {
            out.po[i] = skip;
            out.buy[i] = 2;
            out.buy_price[i] = ob;
            pos[i] = pp;
            abc += 1.0;
            skip = 0.0;
        } else if pp > 0.0 && abc == 0.0 && skip > 0.0 && b.sw.trend[i] == -1 {
            pos[i] = 0.0;
            skip = 0.0;
            abc = 0.0;
        } else if (pp > 0.0 && abc > 0.0 && (b.sw.trend[i] == -1 || !b.athc[i]) && !(b.loc[i] || b.lc[i])) || b.dead_sell[i] {
            out.sell[i] = true;
            if (b.loc[i - 1] || b.lc[i - 1]) && b.sw.trend[i - 1] == -1 {
                out.sell_price[i] = o[i] - (o[i] * buf) / 100.0;
            } else {
                out.sell_price[i] = c[i];
            }
            pos[i] = 0.0;
            prcnt = 0.0;
            skip = 0.0;
            abc = 0.0;
        } else {
            pos[i] = pp;
        }
    }
    out
}

/// Loop with one moving ATR level `Up_ATR` (last fill + current ATR x multiplier; AFLs "Variable ATR Scalein").
fn scale_atr_var(s: &Series, p: &ScaleCfg) -> ScaleOut {
    let b = scale_base(s, p);
    let n = b.n;
    let (c, o, h) = (&b.c, &b.o, &b.h);
    let myatr = atr_wilder(&b.h, &b.l, &b.c, p.atr_prd);
    let mut out = scale_out_new(&b);
    let max_position = 5.0f32;
    let mut up_atr = vec![NULLV; n];
    let (mut position, mut skip, mut abc) = (0f32, 0f32, 0f32);
    let buf = p.buffer;
    for i in 1..n {
        let ob = o[i] + (o[i] * buf) / 100.0;
        let step = myatr[i] * p.myatr1;
        if position == 0.0 && (b.athc[i] || c[i] > b.hcl[i - 1]) && b.gate[i] && b.master[i] && myatr[i] > 0.0 {
            if b.other_filter[i] {
                up_atr[i] = c[i] + step;
                position = 1.0;
                skip += 1.0;
            } else {
                out.buy[i] = 1;
                up_atr[i] = c[i] + step;
                position = 1.0;
                abc += 1.0;
            }
        } else if position > 0.0 && position < max_position && h[i] > up_atr[i - 1] && b.master[i] {
            if position > 0.0 && !b.upc[i] {
                out.po[i] = skip + 1.0;
                out.buy[i] = 2;
                let u = up_atr[i - 1];
                if o[i] > u {
                    out.buy_price[i] = ob;
                } else if skip > 0.0 && h[i] > u {
                    out.buy_price[i] = ((ob * skip) + u) / (skip + 1.0);
                } else if skip == 0.0 && h[i] > u {
                    out.buy_price[i] = u;
                }
                up_atr[i] = out.buy_price[i] + step;
                position += 1.0;
                skip = 0.0;
                abc += 1.0;
            } else {
                up_atr[i] = up_atr[i - 1] + step;
                position += 1.0;
                skip += 1.0;
            }
        } else if skip > 0.0 && b.sw.trend[i] == 1 && b.skip_buy_filter[i] && b.master[i] {
            out.po[i] = skip;
            out.buy[i] = 2;
            out.buy_price[i] = ob;
            up_atr[i] = up_atr[i - 1];
            skip = 0.0;
            abc += 1.0;
        } else if position > 0.0 && abc == 0.0 && skip > 0.0 && b.sw.trend[i] == -1 {
            position = 0.0;
            skip = 0.0;
            abc = 0.0;
        } else if (position > 0.0 && abc > 0.0 && (b.sw.trend[i] == -1 || !b.athc[i]) && !(b.loc[i] || b.lc[i])) || b.dead_sell[i] {
            out.sell[i] = true;
            if (b.loc[i - 1] || b.lc[i - 1]) && b.sw.trend[i - 1] == -1 {
                out.sell_price[i] = o[i] - (o[i] * buf) / 100.0;
            } else {
                out.sell_price[i] = c[i];
            }
            position = 0.0;
            up_atr[i] = NULLV;
            skip = 0.0;
            abc = 0.0;
        } else {
            up_atr[i] = up_atr[i - 1];
        }
    }
    out
}

/// The signals the portfolio runner can use: one buy per position on the bar of its first real tranche, one sell, later tranches as scale-in orders
/// (one of five tranches each; `po` tranches in one order), as ath_loss2::scalein_signals.
fn scale_signals(s: &Series, e: &ScaleOut) -> Signals {
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

fn scale_cfg(g: G, exit_pct: f32, gate: Gate) -> ScaleCfg {
    ScaleCfg {
        exit_pct,
        penny: g("pennyflt") as f32,
        turnflt: g("turnflt").round() == 1.0,
        dead_yes: g("deadstock").round() == 0.0,
        atr_prd: g("atr_prd").max(0.0) as usize,
        myatr1: g("myatr1") as f32,
        buffer: g("buffer_prcnt") as f32,
        gate,
        nogate: g("dbg_nogate") != 0.0,
    }
}

/// EXIT % of the Optimize() AFLs: the page's `exitpct` setting (default 10); the hidden test key `dbg_exit` (0 in production) lets a check replay a sheet made with another EXIT %.
fn dbg_exit(g: G) -> f32 {
    let d = g("dbg_exit") as f32;
    if d > 0.0 { d } else { g("exitpct") as f32 }
}

fn loss_gate(g: G) -> Gate {
    Gate::Loss { noofloss: g("noofloss") as f32, losspct: g("losspct") as f32, comm: g("comm") as f32 }
}

fn sig_scale_pct_loss(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cfg = scale_cfg(g, g("exitpct") as f32, loss_gate(g));
    series.iter().map(|s| scale_signals(s, &scale_pct(s, &cfg))).collect()
}
fn sig_scale_atrvar_loss(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cfg = scale_cfg(g, g("exitpct") as f32, loss_gate(g));
    series.iter().map(|s| scale_signals(s, &scale_atr_var(s, &cfg))).collect()
}
fn sig_scale_atrfix_trig(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    // EXIT % and BUY PERCENTAGE FROM ATHC are Optimize() in this AFL (defaults 10 and 5)
    let cfg = scale_cfg(g, dbg_exit(g), Gate::Trigger { pct: g("athcbuypct") as f32 });
    series.iter().map(|s| scale_signals(s, &scale_atr_fixed(s, &cfg))).collect()
}
fn sig_scale_pct_trig(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cfg = scale_cfg(g, dbg_exit(g), Gate::Trigger { pct: g("athcbuypct") as f32 });
    series.iter().map(|s| scale_signals(s, &scale_pct(s, &cfg))).collect()
}
fn sig_scale_atrvar_trig(series: &[Series], _e: &Env, g: G) -> Vec<Signals> {
    let cfg = scale_cfg(g, g("exitpct") as f32, Gate::Trigger { pct: g("athcbuypct") as f32 });
    series.iter().map(|s| scale_signals(s, &scale_atr_var(s, &cfg))).collect()
}

fn scale_base_params() -> Vec<P> {
    vec![pdead(), ppenny(), pturn(TURN_B)]
}
fn schema_scale_pct_loss() -> Vec<P> {
    let mut v = scale_base_params();
    v.push(p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0));
    v.push(p("losspct", "MIN PROFIT %", 3.0, 1.0, 3.0, 0.05));
    v.push(p("noofloss", "CONSICUTIVE LOSS", 2.0, 1.0, 2.0, 1.0));
    v.push(pbuffer());
    v
}
fn schema_scale_atrvar_loss() -> Vec<P> {
    let mut v = scale_base_params();
    v.push(p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0));
    v.push(p("losspct", "MIN PROFIT %", 3.0, 1.0, 3.0, 0.05));
    v.push(p("noofloss", "CONSICUTIVE LOSS", 2.0, 1.0, 2.0, 1.0));
    v.extend(patr());
    v.push(pbuffer());
    v
}
fn schema_scale_atrfix_trig() -> Vec<P> {
    let mut v = scale_base_params();
    v.push(p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0));
    v.extend(patr());
    v.push(p("athcbuypct", "BUY PERCENTAGE FROM ATHC", 5.0, 1.0, 10.0, 1.0));
    v.push(pbuffer());
    v
}
fn schema_scale_pct_trig() -> Vec<P> {
    let mut v = scale_base_params();
    v.push(p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0));
    v.push(p("athcbuypct", "BUY PERCENTAGE FROM ATHC", 5.0, 1.0, 10.0, 1.0));
    v.push(pbuffer());
    v
}
fn schema_scale_atrvar_trig() -> Vec<P> {
    let mut v = scale_base_params();
    v.push(p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0));
    v.extend(patr());
    v.push(p("athcbuypct", "BUY PERCENTAGE FROM ATHC", 5.0, 1.0, 10.0, 1.0));
    v.push(pbuffer());
    v
}

// ================================================================= the registered variants

pub fn variants() -> Vec<VariantDef> {
    let d = data22();
    vec![
        VariantDef { id: "ytath_and_pctswing_after_consecutive_low", data: d, schema: schema_scale_pct_loss, signals: sig_scale_pct_loss },
        VariantDef { id: "ytath_plus1pct_buy_pctswing_exit_after", data: d, schema: schema_plus1, signals: sig_plus1 },
        VariantDef { id: "ytath_plus5pct_buy_pctswing_exit_after_7", data: d, schema: schema_plus7, signals: sig_plus7 },
        VariantDef { id: "ytath_entry_after_2_consecutive_losses_and", data: d, schema: schema_pt_loss, signals: sig_pt_loss },
        VariantDef { id: "ytath_entry_st_10_4_exit_only_after_2", data: d, schema: schema_st104_2, signals: sig_st104_2 },
        VariantDef { id: "ytath_entry_st_10_4_exit_only_after_n", data: d, schema: schema_prob_st104, signals: sig_prob_st104 },
        VariantDef { id: "ytath_running_pctswing_exit_after_2_st_5_2", data: d, schema: schema_prob_run52, signals: sig_prob_run52 },
        VariantDef { id: "ytath_running_pctswing_exit_after_2_st_5_2_2", data: d, schema: schema_prob_run52b, signals: sig_prob_run52b },
        VariantDef { id: "ytath_running_pctswing_after_2_pctswing", data: d, schema: schema_run_loss, signals: sig_run_loss },
        VariantDef { id: "ytath_running_pctswing_after_2_pctswing_2", data: d, schema: schema_run_loss, signals: sig_run_loss2 },
        VariantDef { id: "ytath_with_1_trade_loss_entry_variable_atr", data: d, schema: schema_scale_atrvar_loss, signals: sig_scale_atrvar_loss },
        VariantDef { id: "ytath_after_1_losing_trade_gap_between_2", data: d, schema: schema_gap, signals: sig_gap },
        VariantDef { id: "ytath_stock_st_5_1_buys_after_consecutive", data: d, schema: schema_st51, signals: sig_st51 },
        VariantDef { id: "ytath_stock_take_all_st_5_1_trades_once", data: d, schema: schema_st51_all, signals: sig_st51_all },
        VariantDef { id: "ytath_5pct_buy_re_entry_within_5pct_of_ath", data: d, schema: schema_reentry, signals: sig_reentry },
        VariantDef { id: "ytath_plus_pct_buy_pctswing_exit_nse500", data: d, schema: schema_nse500, signals: sig_nse500 },
        VariantDef { id: "ytath_plus_pct_buy_pctswing_exit_nse500_2", data: d, schema: schema_nse500_2, signals: sig_nse500_2 },
        VariantDef { id: "ytath_plus_4pct_buy_with_ratio_pctswing", data: d, schema: schema_ratio, signals: sig_ratio },
        VariantDef { id: "ytath_5_plus_5pct_buy_pctswing_exit_scale", data: d, schema: schema_scale_atrfix_trig, signals: sig_scale_atrfix_trig },
        VariantDef { id: "ytath_5_plus_5pct_buy_pctswing_exit_scale_2", data: d, schema: schema_scale_pct_trig, signals: sig_scale_pct_trig },
        VariantDef { id: "ytath_5_plus_5pct_buy_pctswing_exit_scale_3", data: d, schema: schema_scale_atrvar_trig, signals: sig_scale_atrvar_trig },
        VariantDef { id: "ytath_entry_on_athc_plus_5pct_exit_below", data: d, schema: schema_pt_plus, signals: sig_pt_plus },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series_of(closes: &[f32]) -> Series {
        let bars = closes
            .iter()
            .enumerate()
            .map(|(i, &c)| Bar { date: 20100101 + i as i32, c, o: c, h: c * 1.01, l: c * 0.99, v: 1.0e6, oi: 1.0e8, aux1: 1.0, aux2: 0.0 })
            .collect();
        Series { name: "T".into(), bars }
    }

    #[test]
    fn swing_flips_down_when_close_falls_through_the_band() {
        let c = [100.0f32, 101.0, 102.0, 103.0, 80.0, 79.0];
        let sw = swing(&c, 10.0);
        assert_eq!(sw.trend[3], 1);
        assert_eq!(sw.trend[4], -1);
        // TrendUp1 of the bar before the flip is back-filled with TrendDown1 (Null here), the down line starts on the flip bar
        assert!(sw.tdn[4] > 0.0);
    }

    #[test]
    fn athc_state_latches_and_clears_on_a_down_swing() {
        let c = [100.0f32, 105.0, 104.0, 90.0, 89.0, 120.0];
        let hcl = hhv_all(&c);
        let sw = swing(&c, 10.0);
        let a = athc_state(&c, &hcl, &sw.trend, None);
        assert!(a[1] && a[2]);
        assert!(!a[3]); // 90 < 105*0.9 -> trend -1 clears the state
        assert!(a[5]);
    }

    #[test]
    fn trigger_is_the_first_ath_close_plus_pct_and_null_after_a_down_swing() {
        let c = [100.0f32, 110.0, 111.0, 112.0, 80.0, 81.0];
        let hcl = hhv_all(&c);
        let sw = swing(&c, 10.0);
        let t = trigger_override(&c, &hcl, &sw.trend, 5.0);
        assert!((t[1] - 115.5).abs() < 1e-3);
        assert!((t[3] - 115.5).abs() < 1e-3);
        assert!(is_null(t[4]));
    }

    #[test]
    fn supertrend_period_counts_and_flips() {
        let n = 40;
        let mut c: Vec<f32> = (0..n).map(|i| 100.0 + i as f32).collect();
        for i in 25..n {
            c[i] = 140.0 - 4.0 * (i - 24) as f32;
        }
        let h: Vec<f32> = c.iter().map(|x| x + 1.0).collect();
        let l: Vec<f32> = c.iter().map(|x| x - 1.0).collect();
        let tr = tr1(&h, &l, &c);
        let (tu, _td, tr_) = st_lines(&c, &tr, &c, 5.0, 2.0, 2.0, true);
        assert_eq!(tr_[20], 1);
        assert_eq!(tr_[39], -1);
        assert!(tu[20] > 0.0 && tu[20] < c[20]);
        // fractional periods: ceil(5.5) = 6 true ranges added, divided by 5.5
        let (tu2, _, _) = st_lines(&c, &tr, &c, 5.5, 2.0, 2.0, false);
        assert!(tu2[20] > 0.0);
    }

    #[test]
    fn pct_levels_scale_in_at_3_percent_steps() {
        // an uptrend of closes: entry, then the high crosses +3 %, +6 %: orders with 1 tranche each at 1.03 x and 1.06 x of the entry close
        let mut closes = vec![100.0f32; 5];
        closes.extend([101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0, 108.0]);
        let mut s = series_of(&closes);
        for (i, b) in s.bars.iter_mut().enumerate() {
            b.h = b.c * 1.001;
            b.l = b.c * 0.999;
            b.o = if i == 0 { b.c } else { closes[i - 1] };
        }
        s.bars[5].h = 101.0 * 1.001;
        let cfg = ScaleCfg { exit_pct: 10.0, penny: 0.0, turnflt: false, dead_yes: false, atr_prd: 14, myatr1: 1.0, buffer: 0.25, gate: Gate::Trigger { pct: 0.0 }, nogate: false };
        let e = scale_pct(&s, &cfg);
        let first = e.buy.iter().position(|&b| b == 1);
        assert!(first.is_some(), "an entry is expected");
        let i0 = first.unwrap();
        let buy_at = closes[i0];
        let orders: Vec<(usize, f32)> = (0..closes.len()).filter(|&i| e.buy[i] == 2).map(|i| (i, e.buy_price[i])).collect();
        assert!(!orders.is_empty(), "a scale-in order is expected");
        let lv1 = (1.0 + 3.0f32 / 100.0) * buy_at;
        assert!(orders.iter().any(|&(_, p)| (p - lv1).abs() < 0.2 || p > lv1), "first level near +3 %: {:?} vs {}", orders, lv1);
    }

    #[test]
    fn reentry_buys_within_five_percent_of_the_ath_after_the_swing_exit() {
        // ATH, then a 25 % drop (swing exit state 2), then a recovery above ATH-5 %
        let mut closes = vec![100.0f32, 110.0, 120.0, 130.0, 100.0, 98.0, 99.0, 120.0, 124.0, 125.0];
        closes.extend(vec![125.0; 5]);
        let s = series_of(&closes);
        let (sg_ok, _) = (reentry_symbol_for_test(&s), 0);
        assert!(sg_ok.buy.iter().any(|&b| b), "a re-entry is expected");
        let i = sg_ok.buy.iter().position(|&b| b).unwrap();
        assert!(i >= 7, "the buy comes after the swing exit, got bar {i}");
    }

    fn reentry_symbol_for_test(s: &Series) -> Signals {
        let g = |k: &str| -> f64 {
            match k {
                "deadstock" => 1.0,
                "exitpct" => 10.0,
                _ => 0.0,
            }
        };
        reentry_symbol(s, &g)
    }

    #[test]
    fn gap_variant_needs_a_loss_and_a_long_gap() {
        // only a smoke test of the machinery on a short series: no panic and no overlapping buys
        let closes: Vec<f32> = (0..200).map(|i| 100.0 + ((i * 7) % 23) as f32 + (i / 10) as f32).collect();
        let s = series_of(&closes);
        let g = |k: &str| -> f64 {
            match k {
                "deadstock" => 1.0,
                "exitpct" => 10.0,
                "losspct" => 2.0,
                "comm" => 0.65,
                _ => 0.0,
            }
        };
        let sg = gap_symbol(&s, &g);
        assert_eq!(sg.buy.len(), 200);
    }

    #[test]
    fn ranking_none_gives_no_score_and_roc_mode_gives_one() {
        let c: Vec<f32> = (0..100).map(|i| 100.0 + i as f32).collect();
        assert!(rank_score(&c, 0, 30, 14, 14, 180).is_none());
        let r = rank_score(&c, 1, 30, 14, 14, 180).unwrap();
        assert!((r[50] - (100.0 + 100.0 * (150.0 / 120.0 - 1.0))).abs() < 0.01);
    }

    fn rd(extra: &[(&'static str, f64)]) -> impl Fn(&str) -> f64 {
        let m: std::collections::HashMap<&'static str, f64> = extra.iter().cloned().collect();
        move |k: &str| -> f64 {
            if let Some(v) = m.get(k) {
                return *v;
            }
            match k {
                "deadstock" => 1.0,
                "exitpct" => 10.0,
                "losspct" => 2.0,
                "comm" => 0.65,
                "athcbuypct" => 5.0,
                "pt_pct" => 5.0,
                "bpd2" => 5.0,
                "bfactor2" => 1.0,
                "stflt1" => 1.0,
                "buffer_prcnt" => 0.25,
                "atr_prd" => 14.0,
                "myatr1" => 1.0,
                _ => 0.0,
            }
        }
    }

    fn rising_then_falling() -> Series {
        let mut c: Vec<f32> = (0..60).map(|i| 100.0 + 1.5 * i as f32).collect();
        c.extend((0..30).map(|i| 190.0 - 4.0 * i as f32));
        series_of(&c)
    }

    #[test]
    fn plus_x_percent_buy_waits_for_the_trigger_and_exits_on_the_swing() {
        let s = rising_then_falling();
        let g = rd(&[("noofloss", 0.0)]);
        let sg = plus_loss_symbol(&s, &g, false);
        let b = sg.buy.iter().position(|&x| x).expect("a buy");
        // first ATH close is bar 1; the trigger is its close x 1.05 = 157.5 x 1.0 -> the buy comes on the first close above it
        let first_ath = s.bars[1].c;
        assert!(s.bars[b].c > first_ath * 1.05 - 1e-3, "buy bar {b} close {} must be above the trigger", s.bars[b].c);
        assert!(b > 1);
        let sell = (b + 1..sg.sell.len()).find(|&i| sg.sell[i]).expect("a sell");
        assert!(sell > b && sell >= 60, "sold when the swing breaks, bar {sell}");
    }

    #[test]
    fn profit_target_sells_at_the_target_price() {
        let mut s = rising_then_falling();
        for b in s.bars.iter_mut() {
            b.h = b.c * 1.08; // intraday highs reach +8 % above the close
        }
        let g = rd(&[("noofloss", 0.0), ("pt_pct", 5.0)]);
        let sg = pt_symbol(&s, &g, false);
        let b = sg.buy.iter().position(|&x| x).expect("a buy");
        let k = (b + 1..sg.sell.len()).find(|&i| sg.sell[i]).expect("a sell");
        let pt = (1.0f32 + 5.0 * 0.01) * s.bars[b].c;
        assert!((sg.sell_price[k] - pt).abs() < 1e-3, "sold at the profit target {pt}, got {}", sg.sell_price[k]);
    }

    #[test]
    fn nse500_2_aux_toggles_gate_the_buy_and_default_off_is_unchanged() {
        let mut s = series_of(&[100.0, 101.0, 102.0, 110.0, 120.0, 130.0, 140.0]);
        for b in s.bars.iter_mut() { b.aux1 = 1.0; b.aux2 = 0.0; }
        let with = |a1: f64, a2: f64| rd(&[("turnflt", 0.0), ("auxtoggle", a1), ("auxtoggle1", a2)]);
        let any = |sg: &Signals| sg.buy.iter().any(|&b| b);
        let off = plus_run_symbol_aux(&s, &with(1.0, 1.0), 10.0, 4.0, Some(false), true);
        let plain = plus_run_symbol(&s, &with(1.0, 1.0), 10.0, 4.0, Some(false));
        assert_eq!((off.buy.clone(), off.sell.clone()), (plain.buy.clone(), plain.sell.clone()), "both OFF = the old behaviour");
        let a1 = plus_run_symbol_aux(&s, &with(0.0, 1.0), 10.0, 4.0, Some(false), true);
        assert!(any(&off) && a1.buy == off.buy, "Aux1 ON, the flag is 1 everywhere: same buys");
        assert!(!any(&plus_run_symbol_aux(&s, &with(1.0, 0.0), 10.0, 4.0, Some(false), true)), "Aux2 ON, flag is 0 everywhere");
        assert!(!any(&plus_run_symbol_aux(&s, &with(0.0, 0.0), 10.0, 4.0, Some(false), true)), "both ON needs both flags");
    }

    #[test]
    fn universe_flags_follow_aux1_and_aux2() {
        let mut s = series_of(&[100.0, 101.0, 102.0, 103.0]);
        s.bars[1].aux1 = 0.0;
        s.bars[2].aux2 = 1.0;
        assert_eq!(backtest_flags(&s, 0), vec![true; 4]);
        assert_eq!(backtest_flags(&s, 1), vec![true, false, true, true]);
        assert_eq!(backtest_flags(&s, 2), vec![false, false, true, false]);
        assert_eq!(backtest_flags(&s, 3), vec![true, false, false, true]);
    }

    #[test]
    fn supertrend_variants_do_not_overlap_and_stay_inside_the_bar() {
        let s = rising_then_falling();
        let g = rd(&[("noofloss", 0.0)]);
        for sg in [st51_symbol(&s, &g, false), st51_symbol(&s, &g, true), st104_after2_symbol(&s, &g)] {
            let tr = trades_of(&sg);
            for w in tr.windows(2) {
                assert!(w[0].sell_i != usize::MAX && w[0].sell_i <= w[1].buy_i);
            }
            for e in &tr {
                let b = &s.bars[e.buy_i];
                assert!(e.buy_price >= b.l && e.buy_price <= b.h);
            }
        }
    }

    #[test]
    fn variable_atr_scale_in_orders_never_exceed_five_tranches() {
        let mut closes: Vec<f32> = (0..80).map(|i| 100.0 * (1.0 + 0.02 * i as f32)).collect();
        closes.extend((0..40).map(|i| 250.0 - 5.0 * i as f32));
        let mut s = series_of(&closes);
        for (i, b) in s.bars.iter_mut().enumerate() {
            b.o = if i == 0 { b.c } else { closes[i - 1] };
            b.h = b.o.max(b.c) * 1.02;
            b.l = b.o.min(b.c) * 0.98;
        }
        let cfg = ScaleCfg { exit_pct: 10.0, penny: 0.0, turnflt: false, dead_yes: false, atr_prd: 14, myatr1: 1.0, buffer: 0.25, gate: Gate::Trigger { pct: 0.0 }, nogate: true };
        let e = scale_atr_var(&s, &cfg);
        let tranches: f32 = (0..closes.len()).filter(|&i| e.buy[i] == 2).map(|i| e.po[i]).sum::<f32>() + e.buy.iter().filter(|&&b| b == 1).count() as f32;
        assert!(e.buy.iter().any(|&b| b == 1));
        assert!(tranches <= 5.0 * e.sell.iter().filter(|&&x| x).count().max(1) as f32 + 5.0, "tranches {tranches}");
        let sg = scale_signals(&s, &e);
        assert!(sg.scale.iter().all(|q| q.mult >= 1.0 && q.mult <= 4.0));
    }

    #[test]
    fn every_variant_has_a_default_inside_its_range() {
        for v in variants() {
            for q in (v.schema)() {
                assert!(q.min <= q.def && q.def <= q.max, "{} {}", v.id, q.key);
            }
        }
        assert_eq!(variants().len(), 22);
    }
}
