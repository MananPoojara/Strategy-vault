//! ATH sub-variant "ATHC WITH ALL ST VARIATION with all ranking.afl" (AFL ATH checking, Aux1/Aux2 version):
//! running ATH close buy (`buysig` until the Supertrend(period, factor) on the chosen calculation point flips down), sell at the close,
//! all 11 RANKING MODES (PositionScore) and all 7 Supertrend calculation points. Default parameters of the AFL.
//! Standard library only (depends on `ath_core` only). Notes: `ATH/checked/ytath_chk_all_st_variations_all_ranking/RESULT.md`.
#![allow(dead_code)]
use crate::ath_core::*;

/// A comparison with Null (NaN here) is false.
fn gt(a: f32, b: f32) -> bool {
    !a.is_nan() && !b.is_nan() && a > b
}

/// The AFL's `volsig` thresholds (compared with Volume * Close), this file's table.
fn volsig(year: i32) -> f32 {
    match year {
        y if y < 2000 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

/// `ATR(1)`: true range, first bar H - L.
fn tr1(h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    (0..h.len()).map(|i| if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect()
}

/// `MA(a, n)`: Null for the first n-1 bars.
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

/// `HHV(C, BarCount)`: highest close from the first bar up to and including the bar.
fn hhv_all(c: &[f32]) -> Vec<f32> {
    let mut h = vec![0f32; c.len()];
    for i in 0..c.len() {
        h[i] = if i == 0 || c[i] > h[i - 1] { c[i] } else { h[i - 1] };
    }
    h
}

/// `HHV(C, n)` with the window shortened at the start.
fn hhv_n(a: &[f32], n: usize) -> Vec<f32> {
    (0..a.len()).map(|i| a[i.saturating_sub(n.saturating_sub(1))..=i].iter().copied().fold(f32::MIN, f32::max)).collect()
}

/// The AFL's `ST()` function: returns `btrend1` (+1 / -1). satr = (ATR(1) + Ref(ATR(1),-1) + ... ) / period (Null while history is short),
/// Up = stcal + 1 * satr (the factor 1 on the upper side is hard-coded), Dn = stcal - factor * satr.
fn supertrend(stcal: &[f32], tr: &[f32], c: &[f32], period: usize, factor: f32) -> Vec<i8> {
    let n = c.len();
    let mut satr = vec![f32::NAN; n];
    for i in 0..n {
        // AmiBroker: ATR(1) is Null on bar 0, so the `period` sum is Null until bar `period` (a new listing's first bar must not count)
        if period >= 1 && i >= period {
            let mut s = tr[i];
            for k in 1..period {
                s += tr[i - k];
            }
            satr[i] = s / period as f32;
        }
    }
    let addm = |a: f32, m: f32, b: f32, sub: bool| -> f32 {
        if a.is_nan() || b.is_nan() {
            f32::NAN
        } else if sub {
            a - m * b
        } else {
            a + m * b
        }
    };
    let mut up: Vec<f32> = (0..n).map(|i| addm(stcal[i], 1.0, satr[i], false)).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| addm(stcal[i], factor, satr[i], true)).collect();
    let mut tdir = vec![1i8; n];
    for i in 1..n {
        tdir[i] = if gt(c[i], up[i - 1]) {
            1
        } else if gt(dn[i - 1], c[i]) {
            -1
        } else {
            tdir[i - 1]
        };
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

/// `STCAL` per the AFL's if / else-if chain on the ParamList (1 CLOSE, 2 MID POINT, 3 HIGH, 4 PIVOT, 5 MP CL PIVOT AVG, 6 "3DAYS MA",
/// 7 5DAYS MA). The AFL tests the misspelt "3DYAS MA", so item 6 matches no branch and STCAL keeps its start value MP = (H+L)/2.
pub fn stcal_of(s: &Series, stflt: i32) -> Vec<f32> {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let mp: Vec<f32> = (0..n).map(|i| (h[i] + l[i]) / 2.0).collect();
    let pivot: Vec<f32> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
    match stflt {
        1 => c,
        2 => mp,
        3 => h,
        4 => pivot,
        5 => (0..n).map(|i| (mp[i] + pivot[i] + c[i]) / 3.0).collect(),
        7 => ma_n(&c, 5),
        _ => mp, // 6: typo in the AFL, falls through to MP
    }
}

/// RSI(n), Wilder form (as the other ATH engines; not verified against AmiBroker's).
fn rsi(c: &[f32], n: usize) -> Vec<f32> {
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
        let (g, l) = if d > 0.0 { (d, 0.0) } else { (0.0, -d) };
        ag = (ag * (n as f32 - 1.0) + g) / n as f32;
        al = (al * (n as f32 - 1.0) + l) / n as f32;
        out[i] = if al == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + ag / al) };
    }
    out
}

/// `StDev(C, n)` (population), Null while the window is short.
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

/// `ROC(C, n)` in percent, Null for the first n bars.
fn roc(c: &[f32], n: usize) -> Vec<f32> {
    (0..c.len()).map(|i| if i >= n && c[i - n] != 0.0 { (c[i] - c[i - n]) / c[i - n] * 100.0 } else { f32::NAN }).collect()
}

/// POCL: the AFL's overlapping count loop copied literally (m from BarCount-(rocpd+1) down to 1, j = 1..rocpd).
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

#[derive(Clone, Copy)]
pub struct StP {
    pub dead_yes: bool,
    pub hardstop: f32,
    pub penny: f32,
    pub turn_on: bool,
    pub aux1_on: bool,
    pub aux2_on: bool,
    pub stflt: i32,
    pub period: usize,
    pub factor: f32,
    pub tstart: usize,
    pub rankmode: i32,
    pub rocpd: usize,
    pub rsipd: usize,
    pub stdpd: usize,
    pub swingpd: usize,
}

/// PositionScore of the AFL (None for RANKING MODE = NONE: the AFL then assigns no PositionScore).
fn position_score(c: &[f32], p: &StP) -> Option<Vec<f32>> {
    let n = c.len();
    let sdiffpct = || -> Vec<f32> {
        let hh = hhv_n(c, p.swingpd);
        (0..n).map(|i| (hh[i] - c[i]) / hh[i] * 100.0).collect()
    };
    let rocstd = || -> Vec<f32> {
        let (r, sd) = (roc(c, p.rocpd), stdev(c, p.stdpd));
        (0..n).map(|i| r[i] / sd[i]).collect()
    };
    Some(match p.rankmode {
        2 => roc(c, p.rocpd).iter().map(|&x| 100.0 + x).collect(),
        3 => roc(c, p.rocpd).iter().map(|&x| 100.0 - x).collect(),
        4 => rsi(c, p.rsipd),
        5 => stdev(c, p.stdpd),
        6 => rocstd().iter().map(|&x| 100.0 + x).collect(),
        7 => rocstd().iter().map(|&x| 100.0 - x).collect(),
        8 => pocl_of(c, p.rocpd),
        9 => sdiffpct().iter().map(|&x| 100.0 - x).collect(),
        10 => {
            let (r, po) = (roc(c, p.rocpd), pocl_of(c, p.rocpd));
            (0..n).map(|i| (100.0 + r[i]) + po[i]).collect()
        }
        11 => {
            let (sd, po) = (sdiffpct(), pocl_of(c, p.rocpd));
            (0..n).map(|i| (100.0 - sd[i]) + po[i]).collect()
        }
        _ => return None,
    })
}

pub fn one(s: &Series, p: &StP) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let stcal = stcal_of(s, p.stflt);
    let tr = tr1(&h, &l, &c);
    let hcl = hhv_all(&c);
    let bt = supertrend(&stcal, &tr, &c, p.period, p.factor);
    // buysig: set on a close above all earlier closes; cleared (the AFL's `buysig[i]==0;` is a no-op, the array holds 0) when the trend is -1
    // and the previous bar had buysig; otherwise carried on
    let mut buysig = vec![false; n];
    for i in 1..n {
        buysig[i] = if gt(c[i], hcl[i - 1]) {
            true
        } else if bt[i] == -1 && buysig[i - 1] {
            false
        } else {
            buysig[i - 1]
        };
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let b = &s.bars[i];
        let upc = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
        let loc = prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
        let (dbuy, dsell) = if p.dead_yes { (i != n - 1, i == n - 1) } else { (i != 0, false) };
        let turnover = if p.turn_on { b.v * b.c > volsig(b.date / 10000) } else { true };
        let prev = if i == 0 { f32::NAN } else { hcl[i - 1] };
        let aux_ok = (!p.aux1_on || b.aux1 == 1.0) && (!p.aux2_on || b.aux2 == 1.0);
        buy[i] = ((gt(c[i], prev) && i >= p.tstart) || buysig[i]) && !upc && dbuy && c[i] > p.penny && turnover && aux_ok;
        sell[i] = (bt[i] == -1 && !loc) || dsell || (!buysig[i] && !loc);
    }
    let b2 = exrem(&buy, &sell);
    let s2 = exrem(&sell, &b2);
    // BuyPrice = ValueWhen(Buy, C), SellPrice = ValueWhen(Sell, C): the close of the signal bar
    let mut sg = Signals::new(b2, s2, c.clone(), c.clone());
    for i in 0..n {
        let b = &s.bars[i];
        sg.buy_price[i] = sg.buy_price[i].min(b.h).max(b.l);
        sg.sell_price[i] = sg.sell_price[i].min(b.h).max(b.l);
    }
    sg.score = position_score(&c, p);
    if p.hardstop > 0.0 {
        sg.stop = Some(StopSpec { loss_pct: p.hardstop, ..StopSpec::default() });
    }
    sg
}

pub fn params_of(g: G) -> StP {
    StP {
        dead_yes: g("deadstock").round() == 1.0,
        hardstop: g("hardstop") as f32,
        penny: g("pennyflt") as f32,
        turn_on: g("turnflt").round() == 1.0,
        aux1_on: g("auxtoggle").round() == 1.0,
        aux2_on: g("auxtoggle1").round() == 1.0,
        stflt: g("stflt").round() as i32,
        period: g("bpd1").round() as usize,
        factor: g("bfactor1") as f32,
        tstart: g("tstart").round() as usize,
        rankmode: g("rankmode").round() as i32,
        rocpd: g("rocpd").round() as usize,
        rsipd: g("rsipd").round() as usize,
        stdpd: g("stdpd").round() as usize,
        swingpd: g("swingpd").round() as usize,
    }
}

fn schema_st() -> Vec<P> {
    vec![
        p("deadstock", "DEAD STOCK BACKTEST (1 YES; 2 NO)", 2.0, 1.0, 2.0, 1.0),
        p("hardstop", "Hard Stoploss %", 0.0, 0.0, 100.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (1 YES; 2 NO)", 1.0, 1.0, 2.0, 1.0),
        p("auxtoggle", "Aux1 (1 ON; 2 OFF)", 2.0, 1.0, 2.0, 1.0),
        p("auxtoggle1", "Aux2 (1 ON; 2 OFF)", 2.0, 1.0, 2.0, 1.0),
        p("stflt", "DAILY ST CALCULATION POINT (1 CLOSE; 2 MID POINTH+L/2; 3 HIGH; 4 PIVOTH+L+C/3; 5 MP CL PIVOT AVG; 6 3DAYS MA; 7 5DAYS MA)", 1.0, 1.0, 7.0, 1.0),
        p("bpd1", "ST Period ", 10.0, 5.0, 10.0, 5.0),
        p("bfactor1", "ST Factor", 4.0, 1.0, 6.0, 1.0),
        p("tstart", "TRADING START BAR", 1.0, 0.0, 500.0, 1.0),
        p("rankmode", "RANKING MODE (1 NONE; 2 POSITIVE ROC; 3 NEGATIVE ROC; 4 RSI; 5 STD; 6 POSITIVE ROC/STD; 7 NEGATIVE ROC/STD; 8 POSITIVE CLOSE; 9 SWING; 10 POSITIVE ROC + POSITIVE CLOSE; 11 POSITIVE CLOSE + SWING)", 1.0, 1.0, 11.0, 1.0),
        p("rocpd", "ROC PERIOD for RANKING/NO OF POSITIVE CLOSE", 10.0, 10.0, 300.0, 10.0),
        p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("stdpd", "STD PERIOD for RANKING", 14.0, 10.0, 300.0, 10.0),
        p("swingpd", "SWING PERIOD", 180.0, 10.0, 500.0, 10.0),
    ]
}

fn signals_st(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let p = params_of(g);
    series.iter().map(|s| one(s, &p)).collect()
}

pub fn variants() -> Vec<VariantDef> {
    vec![VariantDef { id: "ytath_chk_all_st_variations_all_ranking", data: Data::Cash22 { watchlist: "NSE 500 C&P.tls" }, schema: schema_st, signals: signals_st }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(cl: &[f32]) -> Series {
        let bars = cl
            .iter()
            .enumerate()
            .map(|(i, &c)| Bar { date: 20100101 + i as i32, c, o: c - 0.3, h: c + 1.0, l: c - 1.0, v: 1.0e7, oi: 0.0, aux1: 1.0, aux2: 0.0 })
            .collect();
        Series { name: "T".into(), bars }
    }
    fn defaults() -> StP {
        StP { dead_yes: false, hardstop: 0.0, penny: 0.0, turn_on: false, aux1_on: false, aux2_on: false, stflt: 1, period: 10, factor: 4.0, tstart: 1, rankmode: 1, rocpd: 10, rsipd: 14, stdpd: 14, swingpd: 180 }
    }

    #[test]
    fn typo_item_6_is_mid_point() {
        let s = series(&[10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0]);
        assert_eq!(stcal_of(&s, 6), stcal_of(&s, 2));
        assert_eq!(stcal_of(&s, 1), s.close());
    }
    #[test]
    fn aux_toggles_filter() {
        let cl: Vec<f32> = (0..40).map(|i| 10.0 + i as f32).collect();
        let s = series(&cl);
        let mut p = defaults();
        assert!(one(&s, &p).buy.iter().any(|&b| b));
        p.aux2_on = true; // Aux2 == 1 required, the test series has 0
        assert!(!one(&s, &p).buy.iter().any(|&b| b));
        p.aux2_on = false;
        p.aux1_on = true; // Aux1 == 1 present
        assert!(one(&s, &p).buy.iter().any(|&b| b));
    }
    #[test]
    fn pocl_counts_up_closes_of_last_rocpd_bars() {
        let cl: Vec<f32> = (0..30).map(|i| if i % 2 == 0 { 10.0 } else { 11.0 }).collect();
        let po = pocl_of(&cl, 10);
        assert_eq!(po[29], 5.0);
        assert_eq!(po[1], 0.0);
        assert_eq!(po[2], 0.0); // bar 2 is a down close
        assert_eq!(po[3], 1.0);
    }
    #[test]
    fn rank_none_has_no_score_and_modes_have() {
        let cl: Vec<f32> = (0..60).map(|i| 10.0 + (i % 7) as f32).collect();
        let s = series(&cl);
        let mut p = defaults();
        assert!(one(&s, &p).score.is_none());
        for m in 2..=11 {
            p.rankmode = m;
            let sc = one(&s, &p).score.unwrap();
            assert_eq!(sc.len(), 60);
            assert!(sc[59].is_finite(), "mode {m}");
        }
    }
    #[test]
    fn buy_runs_until_supertrend_flip_and_sells_at_close() {
        let mut cl: Vec<f32> = (0..30).map(|i| 10.0 + i as f32).collect();
        cl.extend((0..20).map(|i| 40.0 - 3.0 * i as f32));
        let s = series(&cl);
        let sg = one(&s, &defaults());
        let t = trades_of(&sg);
        assert!(!t.is_empty());
        assert_eq!(sg.buy_price[t[0].buy_i], s.bars[t[0].buy_i].c);
    }
}
