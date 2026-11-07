//! ATH engines: ATHC %swing exit with turnover ranking and market-cap class selection.
//! One `VariantDef` per sub-variant (see `ath_core::VariantDef`); registered in `ath_registry.rs`. Standard library only: depends on
//! `ath_core` only, so a verification driver can include it with `#[path]`.
//!
//! The AFLs (`ATHC EXIT WITH %SWING with AVG turnover ranking VER2.afl`, `... with turnover ranking VER1.afl`) rank every symbol of the
//! watchlist per bar by turnover (`StaticVarGenerateRanks(..., mode 0)`: highest value = rank 1) and buy only while the rank is inside
//! the band of the chosen class. Verification: `ATH/verify/<variant id>/RESULT.md`.
#![allow(dead_code)]
#[allow(unused_imports)]
use crate::ath_core::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// VER2: ranked value = MA(C*V, period); turnover filter (V*C) > Volsig(year); circuits UC = H==C, LC = L==C
    AvgTurnover,
    /// VER1: ranked value = C*V of the day; volume filter V > Ref(MA(V, volfltday), -1); circuits by Prec(.,2)
    DayTurnover,
}

#[derive(Clone, Copy, Debug)]
pub struct RankParams {
    pub kind: Kind,
    pub exit_pct: f32,
    pub penny: f32,
    pub filter: bool,
    pub period: usize,
    pub volfltday: usize,
    pub dead_stock: bool,
    pub rank_lo: f64,
    pub rank_hi: f64,
}

/// `BACKTEST MARKET`: 1 LARGECAP, 2 MIDCAP, 3 SMALLCAP, 4 MICROCAP -> (RANKHELD1, RANKHELD2); rank > lo AND rank < hi.
pub fn band(k: i32) -> (f64, f64) {
    match k {
        2 => (100.0, 251.0),
        3 => (250.0, 801.0),
        4 => (800.0, 80000.0),
        _ => (0.0, 101.0),
    }
}

/// VER2 `Volsig` (V*C > Volsig)
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

/// AmiBroker `MA(a, n)`; the first n-1 bars are the average of the bars so far.
fn ma_f(a: &[f32], n: usize) -> Vec<f32> {
    let n = n.max(1);
    let mut out = vec![f32::NAN; a.len()];
    let mut sum = 0f64;
    for i in 0..a.len() {
        sum += a[i] as f64;
        if i >= n {
            sum -= a[i - n] as f64;
        }
        // AmiBroker's MA averages the bars that exist at the start of an array (the team sheets only fit this way)
        out[i] = (sum / (i + 1).min(n) as f64) as f32;
    }
    out
}

/// The 30-bar "deadstock" test of the AFL: O,H,L,C of the bar equal those of each of the NEXT 30 bars (`Ref(x, k)`, k = 1..30,
/// looks forward; beyond the last bar it is Null and the comparison is false).
fn dead30(s: &Series) -> Vec<bool> {
    let n = s.bars.len();
    let same = |i: usize| {
        let (a, b) = (&s.bars[i], &s.bars[i + 1]);
        a.o == b.o && a.h == b.h && a.l == b.l && a.c == b.c
    };
    let mut out = vec![false; n];
    // run[i] = number of consecutive equal-to-next pairs starting at i
    let mut run = 0usize;
    for i in (0..n).rev() {
        if i + 1 < n && same(i) {
            run += 1;
        } else {
            run = 0;
        }
        // bar i equals bars i+1..i+30 only if all of the pairs i..i+29 are equal (the bar is then equal to every one of them)
        out[i] = run >= 30;
    }
    out
}

/// Per-symbol arrays the cross-sectional ranking needs, plus everything the Buy/Sell expressions use.
struct Sym {
    trend: Vec<i8>,
    hcl: Vec<f32>,
    dead1: Vec<bool>,
    /// value that is ranked (TURNOVER = IIf(deadstock==0, value, 0)); NaN = Null
    val: Vec<f32>,
}

fn prepare(s: &Series, p: &RankParams) -> Sym {
    let n = s.bars.len();
    let c: Vec<f32> = s.bars.iter().map(|b| b.c).collect();
    let v: Vec<f32> = s.bars.iter().map(|b| b.v).collect();
    let mut hcl = vec![0f32; n];
    for i in 0..n {
        hcl[i] = if i == 0 || c[i] > hcl[i - 1] { c[i] } else { hcl[i - 1] };
    }
    let exit: Vec<f32> = c.iter().map(|&x| (x * p.exit_pct) / 100.0).collect();
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
    let dead1 = dead30(s);
    let to: Vec<f32> = (0..n).map(|i| c[i] * v[i]).collect();
    let base = match p.kind {
        Kind::AvgTurnover => ma_f(&to, p.period),
        Kind::DayTurnover => to,
    };
    let val: Vec<f32> = (0..n).map(|i| if dead1[i] { 0.0 } else { base[i] }).collect();
    Sym { trend, hcl, dead1, val }
}

/// StaticVarGenerateRanks(mode 0) per date over the universe: rank 1 = highest value; Null values get no rank (NaN).
/// Ties keep the order of the universe (the AFL's tie-break is a seeded shuffle that cannot be reproduced).
fn ranks(series: &[Series], syms: &[Sym]) -> Vec<Vec<f32>> {
    let mut by_date: HashMap<i32, Vec<(f32, usize, usize)>> = HashMap::new();
    for (k, s) in series.iter().enumerate() {
        for (i, b) in s.bars.iter().enumerate() {
            let x = syms[k].val[i];
            if x.is_finite() {
                by_date.entry(b.date).or_default().push((x, k, i));
            }
        }
    }
    let mut out: Vec<Vec<f32>> = series.iter().map(|s| vec![f32::NAN; s.bars.len()]).collect();
    for (_, mut list) in by_date {
        list.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
        for (r, &(_, k, i)) in list.iter().enumerate() {
            out[k][i] = (r + 1) as f32;
        }
    }
    out
}

fn one(s: &Series, sy: &Sym, rank: &[f32], p: &RankParams) -> Signals {
    let n = s.bars.len();
    let c: Vec<f32> = s.bars.iter().map(|b| b.c).collect();
    let o: Vec<f32> = s.bars.iter().map(|b| b.o).collect();
    let h: Vec<f32> = s.bars.iter().map(|b| b.h).collect();
    let l: Vec<f32> = s.bars.iter().map(|b| b.l).collect();
    let v: Vec<f32> = s.bars.iter().map(|b| b.v).collect();
    // running ATH state
    let mut buysig = vec![false; n];
    for i in 1..n {
        buysig[i] = if c[i] > sy.hcl[i - 1] {
            true
        } else if sy.trend[i] == -1 && buysig[i - 1] {
            false
        } else {
            buysig[i - 1]
        };
    }
    let mavol = ma_f(&v, p.volfltday);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let turnover = if !p.filter {
            true
        } else {
            match p.kind {
                Kind::AvgTurnover => (v[i] * c[i]) > volsig(s.bars[i].date / 10000),
                Kind::DayTurnover => i > 0 && v[i] > mavol[i - 1],
            }
        };
        let (uc, lc) = match p.kind {
            Kind::AvgTurnover => (h[i] == c[i], l[i] == c[i]),
            Kind::DayTurnover => (
                prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]),
                prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]),
            ),
        };
        let dead_buy = if p.dead_stock { i != n - 1 } else { i != 0 };
        let dead_sell = p.dead_stock && i == n - 1;
        let r = rank[i] as f64;
        let in_band = r.is_finite() && r > p.rank_lo && r < p.rank_hi;
        let ath = buysig[i] || (i > 0 && c[i] > sy.hcl[i - 1]);
        buy[i] = ath && !uc && dead_buy && c[i] > p.penny && turnover && in_band && !sy.dead1[i];
        sell[i] = (sy.trend[i] == -1 && !lc) || dead_sell || sy.dead1[i];
    }
    let buy = exrem(&buy, &sell);
    let sell = exrem(&sell, &buy);
    // BuyPrice = ValueWhen(Buy, C), SellPrice = ValueWhen(Sell, C): the close
    let px: Vec<f32> = (0..n).map(|i| c[i].min(h[i]).max(l[i])).collect();
    let mut sg = Signals::new(buy, sell, px.clone(), px);
    sg.score = Some(match p.kind {
        // PositionScore = turn = MA(C*V, period)  /  V*C
        Kind::AvgTurnover => {
            let to: Vec<f32> = (0..n).map(|i| c[i] * v[i]).collect();
            ma_f(&to, p.period)
        }
        Kind::DayTurnover => (0..n).map(|i| v[i] * c[i]).collect(),
    });
    sg
}

/// The AFL's arrays begin on the first bar of the Analysis range (default 2000-01-01), so the ATHC line, the trend and the cross-sectional
/// turnover rank are all built from that bar, not from the first bar of the stored history. `rangefrom` (yyyymmdd) is that start; only the
/// day-turnover ranking variant has it.
fn run(series: &[Series], g: G, kind: Kind) -> Vec<Signals> {
    let from = if kind == Kind::DayTurnover { g("rangefrom") as i32 } else { 0 };
    if from <= 0 {
        return run_inner(series, g, kind);
    }
    let ks: Vec<usize> = series.iter().map(|s| s.bars.partition_point(|b| b.date < from)).collect();
    let sub: Vec<Series> = series.iter().zip(&ks).map(|(s, &k)| Series { name: s.name.clone(), bars: s.bars[k..].to_vec() }).collect();
    run_inner(&sub, g, kind)
        .into_iter()
        .zip(&ks)
        .map(|(mut sg, &k)| {
            let pad_b = |v: &Vec<bool>| [vec![false; k], v.clone()].concat();
            let pad_f = |v: &Vec<f32>| [vec![0f32; k], v.clone()].concat();
            sg.buy = pad_b(&sg.buy);
            sg.sell = pad_b(&sg.sell);
            sg.buy_price = pad_f(&sg.buy_price);
            sg.sell_price = pad_f(&sg.sell_price);
            sg.score = sg.score.take().map(|x| pad_f(&x));
            sg
        })
        .collect()
}

fn run_inner(series: &[Series], g: G, kind: Kind) -> Vec<Signals> {
    let (lo, hi) = band(g("bkmkt").round() as i32);
    let p = RankParams {
        kind,
        exit_pct: g("exitpct") as f32,
        penny: g("pennyflt") as f32,
        filter: g("turnflt").round() == 1.0,
        period: if kind == Kind::AvgTurnover { g("turnoverpd") as usize } else { 1 },
        volfltday: if kind == Kind::DayTurnover { g("volfltday") as usize } else { 10 },
        dead_stock: g("deadstock").round() == 0.0,
        rank_lo: lo,
        rank_hi: hi,
    };
    let syms: Vec<Sym> = series.iter().map(|s| prepare(s, &p)).collect();
    let rk = ranks(series, &syms);
    let hard = g("hardstop") as f32;
    series
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let mut sg = one(s, &syms[k], &rk[k], &p);
            // ApplyStop(stopTypeLoss, stopModePercent, hardstop) with ActivateStopsImmediately False
            if hard > 0.0 {
                sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
            }
            sg
        })
        .collect()
}

fn common(mut v: Vec<P>) -> Vec<P> {
    let mut out = vec![
        p("tstart", "TRADING START BAR (not used by the AFL's signals)", 0.0, 0.0, 500.0, 1.0),
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "Volume Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0),
    ];
    out.append(&mut v);
    out
}

fn schema_avg() -> Vec<P> {
    common(vec![
        p("exitpct", "EXIT %", 10.0, 10.0, 22.0, 1.0),
        p("turnoverpd", "TURNOVER AVG PERIOD", 60.0, 1.0, 200.0, 10.0),
        p("bkmkt", "BACKTEST MARKET AS PER VOLUME (1 LARGECAP, 2 MIDCAP, 3 SMALLCAP, 4 MICROCAP)", 1.0, 1.0, 4.0, 1.0),
    ])
}

fn schema_day() -> Vec<P> {
    common(vec![
        p("volfltday", "Volume Average Days", 10.0, 0.0, 100.0, 1.0),
        p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0),
        p("rangefrom", "Analysis range start (yyyymmdd): the arrays and the turnover rank begin on this date", 20000101.0, 19000101.0, 20300101.0, 1.0),
        p("bkmkt", "BACKTEST MARKET AS PER VOLUME (1 LARGECAP, 2 MIDCAP, 3 SMALLCAP, 4 MICROCAP)", 1.0, 1.0, 4.0, 1.0),
        p("minperiods", "Min Periods (not used by the signals)", 9.0, 1.0, 100.0, 1.0),
        p("maxperiods", "Max Periods (not used by the signals)", 40.0, 1.0, 200.0, 1.0),
    ])
}

fn signals_avg(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    run(series, g, Kind::AvgTurnover)
}
fn signals_day(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    run(series, g, Kind::DayTurnover)
}

// ---------------------------------------------------------------- Fresh ATHC + Market cap (Parth Parmar)
// Ported from `Fresh ATHC + Market cap.afl` and the include files it uses (parmar/Defaults.afl `D_freshATHC`, `D_checkFilter`,
// `D_setParamBacktest`, `D_initTrades`, `D_isBadCandle`, `D_isPaddedDeadStock`; parmar/TechnicalAnalysis.afl `TA_setMarketCapRank`).

/// BarsSince(cond): bars since the last true (0 on the bar itself); None = never (Null).
fn bars_since(c: &[bool]) -> Vec<Option<usize>> {
    let mut last: Option<usize> = None;
    (0..c.len())
        .map(|i| {
            if c[i] {
                last = Some(i);
            }
            last.map(|l| i - l)
        })
        .collect()
}

/// `(pct * x) / 100` with float32 arithmetic = `D_paramStrToNum("<pct>%", x)`
fn pc(pct: f32, x: f32) -> f32 {
    (pct * x) / 100.0
}

struct FreshP {
    swing: f32,
    pb: f32,
    hsl: f32,
    tsl: f32,
    exit_buf: f32,
    penny: f32,
    dead_ticker: bool,
    /// false = the AFL as written (entry only on the first bar of a fresh run); true = entry on any bar of the run while flat
    run_entry: bool,
    period: usize,
    lo: f64,
    hi: f64,
}

fn fresh_one(s: &Series, rank: &[f32], ma_t: &[f32], p: &FreshP) -> Signals {
    let n = s.bars.len();
    let c: Vec<f32> = s.bars.iter().map(|b| b.c).collect();
    let o: Vec<f32> = s.bars.iter().map(|b| b.o).collect();
    let h: Vec<f32> = s.bars.iter().map(|b| b.h).collect();
    let l: Vec<f32> = s.bars.iter().map(|b| b.l).collect();
    // D_ATHC = Highest(Close); D_ATHCS = ATHC - swing % of ATHC
    let mut athc = vec![0f32; n];
    for i in 0..n {
        athc[i] = if i == 0 || c[i] > athc[i - 1] { c[i] } else { athc[i - 1] };
    }
    let athcs: Vec<f32> = athc.iter().map(|&a| a - pc(p.swing, a)).collect();
    // D_freshRunATHC: BarsSince(ATHC > Ref(ATHC,-1)) < BarsSince(Cross(ATHCS, Close))
    let newath: Vec<bool> = (0..n).map(|i| i > 0 && athc[i] > athc[i - 1]).collect();
    let cross: Vec<bool> = (0..n).map(|i| i > 0 && athcs[i] > c[i] && athcs[i - 1] <= c[i - 1]).collect();
    let (ba, bc) = (bars_since(&newath), bars_since(&cross));
    // BarsSince of an event that never happened is Null, which compares as "later than anything": the run is on
    let fr: Vec<bool> = (0..n).map(|i| match (ba[i], bc[i]) { (Some(a), Some(b)) => a < b, (Some(_), None) => true, _ => false }).collect();
    // candles
    let bad_buy: Vec<bool> = (0..n).map(|i| i != n - 1 && (c[i] == h[i] || h[i] == l[i])).collect();
    let bad_sell: Vec<bool> = (0..n).map(|i| i != n - 1 && (c[i] == l[i] || h[i] == l[i])).collect();
    // D_isPaddedDeadStock(30): the bar equals the next 29 bars
    let mut padded = vec![false; n];
    {
        let mut run = 0usize;
        for i in (0..n).rev() {
            if i + 1 < n && o[i] == o[i + 1] && h[i] == h[i + 1] && l[i] == l[i + 1] && c[i] == c[i + 1] {
                run += 1;
            } else {
                run = 0;
            }
            padded[i] = run >= 29;
        }
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut buy_price, mut sell_price) = (c.clone(), c.clone());
    let mut long = false;
    let mut pend = false;
    let (mut pb, mut hsl, mut tsl, mut high): (Option<f32>, Option<f32>, Option<f32>, f32) = (None, None, None, 0.0);
    for i in 1..n {
        let last = i == n - 1;
        if !long {
            let class_ok = {
                let r = rank[i] as f64;
                r.is_finite() && r > p.lo && r <= p.hi
            };
            let ma_ok = !last && ma_t[i].is_finite() && ma_t[i] != 0.0;
            let cond_le = fr[i] && (p.run_entry || !fr[i - 1]) && ma_ok
                && (!p.dead_ticker || !last)
                && !(c[i] <= p.penny)
                && !bad_buy[i]
                && class_ok;
            if cond_le {
                buy[i] = true;
                buy_price[i] = c[i];
                long = true;
                pb = if p.pb != 0.0 { Some(c[i] + pc(p.pb, c[i])) } else { None };
                hsl = if p.hsl != 0.0 { Some(c[i] - pc(p.hsl, c[i])) } else { None };
                if p.tsl != 0.0 {
                    high = c[i];
                    tsl = Some(high - pc(p.tsl, high));
                } else {
                    tsl = None;
                }
                continue; // D_FLAG_SAME_CANDLE_ENTRY_EXIT is False: no exit test on the entry bar
            }
        }
        if long {
            if tsl.is_some() && h[i - 1] > high {
                high = h[i - 1];
                tsl = Some(high - pc(p.tsl, high));
            }
            let cond_lx = (p.dead_ticker && last) || c[i] < athcs[i] || padded[i];
            let mut done = false;
            if pend {
                if !bad_sell[i] {
                    sell[i] = true;
                    sell_price[i] = c[i];
                    done = true;
                }
            } else if pb.map_or(false, |x| h[i] > x) {
                let x = pb.unwrap();
                if o[i] != l[i] || l[i] != c[i] {
                    sell[i] = true;
                    sell_price[i] = if o[i] < x { x } else { o[i] - pc(p.exit_buf, o[i]) };
                    done = true;
                } else {
                    pend = true;
                }
            } else if hsl.map_or(false, |x| l[i] < x) {
                let x = hsl.unwrap();
                if o[i] != l[i] || l[i] != c[i] {
                    sell[i] = true;
                    sell_price[i] = if o[i] > x {
                        x
                    } else if o[i] < x && o[i] == l[i] {
                        o[i]
                    } else if o[i] < x && l[i] < (o[i] - pc(p.exit_buf, buy_price_of(&buy, &buy_price, i))) {
                        o[i] - pc(p.exit_buf, o[i])
                    } else {
                        l[i]
                    };
                    done = true;
                } else {
                    pend = true;
                }
            } else if tsl.map_or(false, |x| c[i] < x) {
                if !bad_sell[i] {
                    sell[i] = true;
                    sell_price[i] = c[i];
                    done = true;
                } else {
                    pend = true;
                }
            } else if cond_lx {
                if !bad_sell[i] {
                    sell[i] = true;
                    sell_price[i] = c[i];
                    done = true;
                } else {
                    pend = true;
                }
            }
            if done {
                long = false;
                pend = false;
            }
        }
    }
    for i in 0..n {
        sell_price[i] = sell_price[i].min(h[i]).max(l[i]);
        buy_price[i] = buy_price[i].min(h[i]).max(l[i]);
    }
    let mut sg = Signals::new(buy, sell, buy_price, sell_price);
    // PositionScore = the market-cap value (MA of turnover; 0 on the last bar)
    sg.score = Some(ma_t.to_vec());
    sg
}

/// BuyPrice of the open trade (the AFL reads the carried `BuyPrice[i]`)
fn buy_price_of(buy: &[bool], bp: &[f32], i: usize) -> f32 {
    (0..=i).rev().find(|&k| buy[k]).map(|k| bp[k]).unwrap_or(bp[i])
}

fn signals_fresh(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let (lo, hi) = match g("backtestmarket").round() as i32 {
        2 => (100.0, 250.0),
        3 => (250.0, 800.0),
        4 => (800.0, f64::INFINITY),
        _ => (0.0, 100.0),
    };
    let p = FreshP {
        swing: g("swing") as f32,
        pb: g("pb") as f32,
        hsl: g("hsl") as f32,
        tsl: g("tsl") as f32,
        exit_buf: g("exitbuf") as f32,
        penny: g("pennyflt") as f32,
        dead_ticker: g("deadticker").round() == 1.0,
        run_entry: g("runentry").round() == 1.0,
        period: g("turnoverpd").round().max(1.0) as usize,
        lo,
        hi,
    };
    // TA_setMarketCapRank: MarketCap = IIf(D_isDeadStock(), 0, MA(Close*Volume, period)); rank 1 = highest (StaticVarGenerateRanks mode 0)
    let ma_t: Vec<Vec<f32>> = series
        .iter()
        .map(|s| {
            let to: Vec<f32> = s.bars.iter().map(|b| b.c * b.v).collect();
            let mut m = ma_f(&to, p.period);
            if let Some(x) = m.last_mut() {
                *x = 0.0;
            }
            m
        })
        .collect();
    let syms: Vec<Sym> = (0..series.len())
        .map(|k| Sym { trend: vec![], hcl: vec![], dead1: vec![], val: ma_t[k].clone() })
        .collect();
    let rk = ranks(series, &syms);
    // The Analysis range starts at `rangefrom`: ATHC, BarsSince and the trade state start there, while the market-cap MA / rank are
    // computed (SetForeign) on the whole history (the team sheets only fit this way).
    let from = g("rangefrom") as i32;
    let cut: Vec<usize> = series.iter().map(|s| s.bars.partition_point(|b| b.date < from)).collect();
    let tr: Vec<Series> = series.iter().zip(&cut).map(|(s, &k)| Series { name: s.name.clone(), bars: s.bars[k..].to_vec() }).collect();
    let series = &tr[..];
    let rk: Vec<Vec<f32>> = rk.iter().zip(&cut).map(|(r, &k)| r[k..].to_vec()).collect();
    let ma_t: Vec<Vec<f32>> = ma_t.iter().zip(&cut).map(|(r, &k)| r[k..].to_vec()).collect();
    series
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let sg = fresh_one(s, &rk[k], &ma_t[k], &p);
            // pad back to the full series
            let z = cut[k];
            let pad_b = |v: Vec<bool>| [vec![false; z], v].concat();
            let pad_f = |v: Vec<f32>| [vec![0f32; z], v].concat();
            let mut out = Signals::new(pad_b(sg.buy), pad_b(sg.sell), pad_f(sg.buy_price), pad_f(sg.sell_price));
            out.score = sg.score.map(pad_f);
            out
        })
        .collect()
}

fn schema_fresh() -> Vec<P> {
    vec![
        p("backtestmarket", "Market (1 Large rank<=100, 2 Mid <=250, 3 Small <=800, 4 Micro)", 1.0, 1.0, 4.0, 1.0),
        p("turnoverpd", "[TA M] Turnover Avg Period", 60.0, 1.0, 500.0, 1.0),
        p("swing", "[ATHC] Swing % below the all-time-high close (exit line)", 17.0, 1.0, 50.0, 0.5),
        p("pb", "Profit book % (0 = off)", 8.0, 0.0, 100.0, 0.5),
        p("hsl", "Hard SL % (0 = off)", 8.0, 0.0, 100.0, 0.5),
        p("tsl", "Trail SL % (0 = off)", 5.0, 0.0, 100.0, 0.5),
        p("exitbuf", "Exit buffer %", 0.25, 0.0, 10.0, 0.05),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("rangefrom", "Analysis range start (yyyymmdd): the arrays begin on this date", 20000101.0, 19000101.0, 20300101.0, 1.0),
        p("runentry", "ASSUMPTION (not in the AFL): 1 = enter on any bar of a fresh run while flat, 0 = only on its first bar (AFL as written)", 0.0, 0.0, 1.0, 1.0),
        p("deadticker", "Dead Stock (0 OFF, 1 ON: no buy on, sell on the last bar)", 1.0, 0.0, 1.0, 1.0),
    ]
}

pub fn variants() -> Vec<VariantDef> {
    vec![
        VariantDef { id: "ytath_pctswing_exit_with_average_turnover_4", data: Data::Cash22 { watchlist: "NSE 500 C&P.tls" }, schema: schema_avg, signals: signals_avg },
        VariantDef { id: "ytath_pctswing_exit_with_turnover_ranking", data: Data::Cash22 { watchlist: "NSE 500 C&P.tls" }, schema: schema_day, signals: signals_day },
        VariantDef { id: "ytath_fresh_athc_with_market_cap_class", data: Data::Cash22 { watchlist: "NSE 500 C&P.tls" }, schema: schema_fresh, signals: signals_fresh },
    ]
}
