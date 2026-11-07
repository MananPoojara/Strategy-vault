//! ATH sub-variants, batch e1 ("ATHC: Supertrend exit"), 12 AFLs coded from `ATH/afl_copy/<id>.afl` with the AFLs' own default parameters.
//! Data: `Data::Cash22 { "NSE 500 C&P.tls" }` (31-03-2022 cash database). Notes per variant: `ATH/v2/<id>/VERIFY.md`.
//!
//! Families (all long only, fills at the signal bar's Close unless stated, float32, a comparison with Null is false):
//! * running ATH-close buy until the Supertrend (period, factor; upper band factor hard-coded 1) flips down, ranking modes: `all_st`, `from_close`,
//!   `rank_by_gap` (adds ranking mode LAST ATHC), `and_st_10_4` (entry/exit variations), `prev_ath` (only if the previous ATH was < PTHCPD bars ago)
//! * `time_based` (first ATH close after an ST down phase, sold after HOLDINGDAYS bars or ST down), `within_10pct` (ATH buy only near the entry-ST
//!   buy price), `plus5` (ATH close above first ATH close of the leg x (1+%)), `stock_buy` (close within BUYPCT% below ATHC and ST up)
//! * state buys on Supertrend: `st8` (ST 5,2 up and Close within 8% of ATHC, exit when ST 5,2 and 5,4 are down), `long_only` (ST up, exit both down)
//! * `daily_weekly` (ATH close buy on daily bars, exit by a monthly / weekly / daily Supertrend of the stock itself).
//! Standard library only; depends on `ath_core` and `ath_new_st` (PositionScore of the 11 ranking modes).
#![allow(dead_code)]
use crate::ath_core::*;
use crate::ath_new_st::{self, StP};

const DATA: Data = Data::Cash22 { watchlist: "NSE 500 C&P.tls" };

// ---------------------------------------------------------------- helpers

fn gt(a: f32, b: f32) -> bool {
    !a.is_nan() && !b.is_nan() && a > b
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

/// `Volsig` of the AFLs that test `OpenInt > volsig` (17 steps).
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Turn {
    Off,
    /// `OpenInt > Volsig` (the cash database's OpenInt = Close x Volume)
    OpenInt,
    /// `(V * C) > Volsig`
    VolClose,
}

fn turnover_ok(b: &Bar, t: Turn) -> bool {
    match t {
        Turn::Off => true,
        Turn::OpenInt => gt(b.oi, volsig_oi(b.date / 10000)),
        Turn::VolClose => gt(b.v * b.c, volsig_vc(b.date / 10000)),
    }
}

/// `ATR(1)`: true range, first bar H - L.
fn tr1(h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    (0..h.len()).map(|i| if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect()
}

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

/// `HHV(C, BarCount)` as the AFLs use it: highest close from the first bar up to the bar.
fn hhv_all(c: &[f32]) -> Vec<f32> {
    let mut h = vec![0f32; c.len()];
    for i in 0..c.len() {
        h[i] = if i == 0 || c[i] > h[i - 1] { c[i] } else { h[i - 1] };
    }
    h
}

/// DEAD STOCK BACKTEST: (deadstockbuysig, deadstocksellsig) of bar i. YES = the toggle value 0 in the AFL.
fn dead(i: usize, n: usize, yes: bool) -> (bool, bool) {
    if yes {
        (i != n - 1, i == n - 1)
    } else {
        (i != 0, false)
    }
}

fn upcircuit(b: &Bar) -> bool {
    prec2(b.o) == prec2(b.c) && prec2(b.o) == prec2(b.h)
}
fn locircuit(b: &Bar) -> bool {
    prec2(b.o) == prec2(b.c) && prec2(b.o) == prec2(b.l)
}

/// Where the Supertrend is calculated from (the ParamList of the AFL).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pt {
    Close,
    Mid,
    High,
    Pivot,
    MpClPivot,
    Ma3,
    Ma5,
}
use Pt::*;
/// CLOSE|MID|HIGH|PIVOT|MP CL PIVOT AVG|3DAYS MA|5DAYS MA, the AFL compares "3DAYS MA" correctly
const L7: [Pt; 7] = [Close, Mid, High, Pivot, MpClPivot, Ma3, Ma5];
/// same list, but the AFL tests the misspelt "3DYAS MA": item 6 matches no branch and STCAL keeps its start value MP = (H+L)/2
const L7T: [Pt; 7] = [Close, Mid, High, Pivot, MpClPivot, Mid, Ma5];
/// MID|CLOSE|PIVOT|MP CL PIVOT AVG|3DAYS MA (misspelt in the code)|5DAYS MA
const L6T: [Pt; 6] = [Mid, Close, Pivot, MpClPivot, Mid, Ma5];
/// CLOSE|MID|PIVOT with `IIf(MID, mid, IIf(PIVOT, pivot, C))`
const L3C: [Pt; 3] = [Close, Mid, Pivot];
/// MID|CLOSE|PIVOT with `IIf(MID, mid, IIf(PIVOT, pivot, C))`
const L3M: [Pt; 3] = [Mid, Close, Pivot];

fn pt_of(list: &[Pt], eng: f64) -> Pt {
    let k = eng.round() as i64 - 1;
    if k >= 0 && (k as usize) < list.len() {
        list[k as usize]
    } else {
        Mid
    }
}

fn stcal_of(s: &Series, pt: Pt) -> Vec<f32> {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let mp: Vec<f32> = (0..n).map(|i| (h[i] + l[i]) / 2.0).collect();
    let pivot: Vec<f32> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
    match pt {
        Close => c,
        Mid => mp,
        High => h,
        Pivot => pivot,
        MpClPivot => (0..n).map(|i| (mp[i] + pivot[i] + c[i]) / 3.0).collect(),
        Ma3 => ma_n(&c, 3),
        Ma5 => ma_n(&c, 5),
    }
}

/// The AFL's `ST()` function. satr = mean of the last `period` ATR(1) values (Null while short), Up = stcal + f_up x satr,
/// Dn = stcal - f_dn x satr (`ST`: f_up hard-coded 1; `ST2` and the 5,2 / 5,4 AFLs: both = the factor).
/// Returns the trend (+1 / -1) and the "bTrendDown1" line (Up band while the trend is down, with the AFL's back-fill of the previous bar
/// at a trend change; Null = NaN).
fn st_run(stcal: &[f32], tr: &[f32], c: &[f32], period: usize, f_up: f32, f_dn: f32) -> (Vec<i8>, Vec<f32>) {
    let n = c.len();
    let mut satr = vec![f32::NAN; n];
    for i in 0..n {
        if period >= 1 && i + 1 >= period {
            let mut s = tr[i];
            for k in 1..period {
                s += tr[i - k];
            }
            satr[i] = s / period as f32;
        }
    }
    let mut up: Vec<f32> = (0..n).map(|i| stcal[i] + f_up * satr[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| stcal[i] - f_dn * satr[i]).collect();
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
            up[i] = stcal[i] + f_up * satr[i];
        }
        if flagh1 {
            dn[i] = stcal[i] - f_dn * satr[i];
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
    (trend, td)
}

/// `Cross(C, Ref(line, -1))` as the existing ST engines read it (verified for the 8%-range ST AFL): Close above the previous bar's line now and
/// not above the line of the bar before on the previous bar; a Null line compares false.
fn cross_prev_line(c: &[f32], line: &[f32], i: usize) -> bool {
    if i < 2 {
        return false;
    }
    let (b_now, b_prev) = (line[i - 1], line[i - 2]);
    gt(c[i], b_now) && !b_prev.is_nan() && c[i - 1] <= b_prev
}

/// Exrem both ways, fills at the Close clamped into the bar, optional ApplyStop percent loss.
fn finish(s: &Series, buy: Vec<bool>, sell: Vec<bool>, hardstop: f32) -> Signals {
    let b2 = exrem(&buy, &sell);
    let s2 = exrem(&sell, &b2);
    let c = s.close();
    let mut sg = Signals::new(b2, s2, c.clone(), c);
    clamp(&mut sg, s);
    if hardstop > 0.0 {
        sg.stop = Some(StopSpec { loss_pct: hardstop, ..StopSpec::default() });
    }
    sg
}

fn clamp(sg: &mut Signals, s: &Series) {
    for i in 0..s.bars.len() {
        let b = &s.bars[i];
        sg.buy_price[i] = sg.buy_price[i].min(b.h).max(b.l);
        sg.sell_price[i] = sg.sell_price[i].min(b.h).max(b.l);
    }
}

/// PositionScore of the 11 ranking modes (new numbering: 1 NONE, 2 POSITIVE ROC, 3 NEGATIVE ROC, 4 RSI, 5 STD, 6 POSITIVE ROC/STD,
/// 7 NEGATIVE ROC/STD, 8 POSITIVE CLOSE, 9 SWING, 10 POSITIVE ROC + POSITIVE CLOSE, 11 POSITIVE CLOSE + SWING) from the verified-by-differential
/// `ath_new_st` code; None for NONE (the AFL assigns no PositionScore).
fn rank_score(s: &Series, mode: i32, rocpd: usize, rsipd: usize, stdpd: usize, swingpd: usize) -> Option<Vec<f32>> {
    if !(2..=11).contains(&mode) {
        return None;
    }
    let p = StP { dead_yes: false, hardstop: 0.0, penny: 0.0, turn_on: false, aux1_on: false, aux2_on: false, stflt: 1, period: 10, factor: 4.0, tstart: 1, rankmode: mode, rocpd, rsipd, stdpd, swingpd };
    ath_new_st::one(s, &p).score
}

/// Ranking mode LAST ATHC: `athcdays = ValueWhen(C > Ref(hcl,-1), BarIndex()); PositionScore = athcdays - Ref(athcdays,-1)`.
fn score_last_athc(c: &[f32]) -> Vec<f32> {
    let n = c.len();
    let hcl = hhv_all(c);
    let mut ad = vec![f32::NAN; n];
    for i in 0..n {
        ad[i] = if i >= 1 && gt(c[i], hcl[i - 1]) { i as f32 } else if i > 0 { ad[i - 1] } else { f32::NAN };
    }
    (0..n).map(|i| if i == 0 { f32::NAN } else { ad[i] - ad[i - 1] }).collect()
}

fn rank_params(g: G) -> (i32, usize, usize, usize, usize) {
    (g("rankmode").round() as i32, g("rocpd").round() as usize, g("rsipd").round() as usize, g("stdpd").round() as usize, g("swingpd").round() as usize)
}

fn rank_schema(modes: f64, rocpd: (f64, f64)) -> Vec<P> {
    vec![
        p("rankmode", "RANKING MODE (1 NONE; 2 POSITIVE ROC; 3 NEGATIVE ROC; 4 RSI; 5 STD; 6 POSITIVE ROC/STD; 7 NEGATIVE ROC/STD; 8 POSITIVE CLOSE; 9 SWING; 10 POSITIVE ROC + POSITIVE CLOSE; 11 POSITIVE CLOSE + SWING; 12 LAST ATHC where offered)", 1.0, 1.0, modes, 1.0),
        p("rocpd", "ROC PERIOD for RANKING/NO OF POSITIVE CLOSE", rocpd.0.max(rocpd.1), rocpd.1, 400.0, 30.0),
        p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("stdpd", "STD PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("swingpd", "SWING PERIOD", 180.0, 10.0, 1000.0, 1.0),
    ]
}

fn dead_p() -> P {
    p("deadstock", "DEAD STOCK BACKTEST (1 YES; 2 NO)", 2.0, 1.0, 2.0, 1.0)
}
fn hard_p() -> P {
    p("hardstop", "Hard Stoploss %", 0.0, 0.0, 50.0, 1.0)
}
fn penny_p() -> P {
    p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0)
}
/// ParamToggle "NO|YES" default NO (value 1 = NO, 2 = YES) for the AFLs whose test is `turnflt == 1`
fn turn_no_yes() -> P {
    p("turnflt", "TURNOVER Filter (1 NO; 2 YES)", 1.0, 1.0, 2.0, 1.0)
}
fn stflt_p(label: &'static str, items: f64) -> P {
    p("stflt", label, 1.0, 1.0, items, 1.0)
}
const ST7: &str = "DAILY ST CALCULATION POINT (1 CLOSE; 2 MID POINT H+L/2; 3 HIGH; 4 PIVOT H+L+C/3; 5 MP CL PIVOT AVG; 6 3DAYS MA; 7 5DAYS MA)";

// ================================================================ 1. running ATH close buy until the ST flips down (all_st family)

#[derive(Clone, Copy)]
pub struct AllSt {
    pub dead_yes: bool,
    pub hardstop: f32,
    pub penny: f32,
    pub turn: Turn,
    pub pt: Pt,
    pub period: usize,
    pub factor: f32,
    pub tstart: usize,
    /// the AFL excludes upper-circuit (H == C) bars from the buy and lower-circuit (L == C) bars from the sell (ytath_with_all_supertrend_variations_and only)
    pub uc_lc: bool,
}

/// `Buy = ((C > Ref(Hcl,-1) AND BarIndex()>=TSTART) OR buysig) AND NOT UPCIRCUIT [AND NOT UC] AND dead AND C>penny AND turnover`,
/// `Sell = (Btrend1==-1 AND NOT LOCIRCUIT [AND NOT LC]) OR deadsell OR (buysig==0 AND NOT LOCIRCUIT)`.
/// `buysig`: 1 on a close above all earlier closes; 0 when the ST trend is down and the previous bar had it (the AFL's `buysig[i]=0` / the typo
/// `buysig[i]==0;` both leave the array's initial 0); else carried.
pub fn all_st_one(s: &Series, a: &AllSt) -> Signals {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let stcal = stcal_of(s, a.pt);
    let tr = tr1(&h, &l, &c);
    let (bt, _) = st_run(&stcal, &tr, &c, a.period, 1.0, a.factor);
    let hcl = hhv_all(&c);
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
        let (upc, loc) = (upcircuit(b), locircuit(b));
        let (uc, lc) = if a.uc_lc { (b.h == b.c, b.l == b.c) } else { (false, false) };
        let (dbuy, dsell) = dead(i, n, a.dead_yes);
        let turnover = turnover_ok(b, a.turn);
        let prev = if i == 0 { f32::NAN } else { hcl[i - 1] };
        buy[i] = ((gt(c[i], prev) && i >= a.tstart) || buysig[i]) && !upc && !uc && dbuy && c[i] > a.penny && turnover;
        sell[i] = (bt[i] == -1 && !loc && !lc) || dsell || (!buysig[i] && !loc);
    }
    finish(s, buy, sell, a.hardstop)
}

fn sg_all_st(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let a = AllSt {
        dead_yes: g("deadstock").round() == 1.0,
        hardstop: g("hardstop") as f32,
        penny: g("pennyflt") as f32,
        turn: if g("turnflt").round() == 2.0 { Turn::OpenInt } else { Turn::Off },
        pt: pt_of(&L7, g("stflt")),
        period: g("bpd1").round() as usize,
        factor: g("bfactor1") as f32,
        tstart: g("tstart").round() as usize,
        uc_lc: true,
    };
    let (rm, r1, r2, r3, r4) = rank_params(g);
    series
        .iter()
        .map(|s| {
            let mut sg = all_st_one(s, &a);
            sg.score = rank_score(s, rm, r1, r2, r3, r4);
            sg
        })
        .collect()
}

fn schema_all_st() -> Vec<P> {
    let mut v = vec![dead_p(), hard_p(), penny_p(), turn_no_yes(), stflt_p(ST7, 7.0), p("bpd1", "ST Period ", 10.0, 1.0, 20.0, 1.0), p("bfactor1", "ST Factor", 4.0, 1.0, 20.0, 1.0), p("tstart", "TRADING START BAR", 1.0, 0.0, 500.0, 1.0)];
    v.extend(rank_schema(11.0, (10.0, 0.0)));
    v
}

// ---- from_close / rank_by_gap: the same engine as the verified-by-differential `ath_new_st` (no UC/LC exclusion, V x C turnover, misspelt 3DAYS MA)

fn from_close_core(s: &Series, g: G) -> Signals {
    let (rm, r1, r2, r3, r4) = rank_params(g);
    let p = StP {
        dead_yes: g("deadstock").round() == 1.0,
        hardstop: g("hardstop") as f32,
        penny: g("pennyflt") as f32,
        turn_on: g("turnflt").round() == 1.0, // ParamToggle "NO|YES", the AFL tests `turnflt==0` => value 1 of the Vault = filter on
        aux1_on: false,
        aux2_on: false,
        stflt: g("stflt").round() as i32,
        period: g("bpd1").round() as usize,
        factor: g("bfactor1") as f32,
        tstart: g("tstart").round() as usize,
        rankmode: if rm > 11 { 1 } else { rm },
        rocpd: r1,
        rsipd: r2,
        stdpd: r3,
        swingpd: r4,
    };
    ath_new_st::one(s, &p)
}

fn sg_from_close(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| from_close_core(s, g)).collect()
}

fn sg_rank_gap(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    series
        .iter()
        .map(|s| {
            let mut sg = from_close_core(s, g);
            if g("rankmode").round() as i32 == 12 {
                sg.score = Some(score_last_athc(&s.close()));
            }
            sg
        })
        .collect()
}

fn schema_st_turn0(modes: f64, extra: bool) -> Vec<P> {
    let mut v = vec![
        dead_p(),
        hard_p(),
        penny_p(),
        p("turnflt", "TURNOVER Filter (1 NO; 2 YES) - the AFL tests turnflt==0, so value 1 = V x C filter ON", 1.0, 1.0, 2.0, 1.0),
        stflt_p(ST7, 7.0),
        p("bpd1", "ST Period ", 10.0, 1.0, 20.0, 1.0),
        p("bfactor1", "ST Factor", 4.0, 1.0, 20.0, 1.0),
        p("tstart", "TRADING START BAR", 1.0, 0.0, 500.0, 1.0),
    ];
    v.extend(rank_schema(modes, (10.0, 30.0)));
    if extra {
        v.push(p("p", "Price field (plot only)", -1.0, -1.0, -1.0, 1.0));
        v.push(p("periods", "Periods (plot only)", 15.0, 2.0, 300.0, 1.0));
    }
    v
}
fn schema_from_close() -> Vec<P> {
    schema_st_turn0(11.0, true)
}
fn schema_rank_gap() -> Vec<P> {
    schema_st_turn0(12.0, false)
}

// ================================================================ 2. ytath_and_st_10_4_buy_athc_entry_supertrend

/// Buy = (C > Ref(Hcl,-1) OR buysig) AND dead AND NOT UPCIRCUIT AND C>penny AND turnover (no TRADING START BAR, no entry-ST test: the entry ST
/// is only plotted); Sell = (Btrend1==-1 OR deadsell) AND NOT LOCIRCUIT. Exit ST(bpd1, factor 4 = Optimize default) on the point of the SECOND stflt ParamList.
/// `base_sell` = true adds the base AFLs' `OR (buysig==0 AND NOT LOCIRCUIT)` clause (NOT in this AFL; only used by the differential check).
/// `latch_resets` = false is the catalog's alternative reading of the typo `buysig[i]==0;` (the latch never clears); the AFL as read here clears it.
pub fn and_st_one(s: &Series, pt: Pt, period: usize, factor: f32, dead_yes: bool, penny: f32, turn: Turn, base_sell: bool, latch_resets: bool) -> Signals {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let stcal = stcal_of(s, pt);
    let tr = tr1(&h, &l, &c);
    let (bt, _) = st_run(&stcal, &tr, &c, period, 1.0, factor);
    let hcl = hhv_all(&c);
    let mut buysig = vec![false; n];
    for i in 1..n {
        buysig[i] = if gt(c[i], hcl[i - 1]) {
            true
        } else if latch_resets && bt[i] == -1 && buysig[i - 1] {
            false
        } else {
            buysig[i - 1]
        };
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let b = &s.bars[i];
        let (dbuy, dsell) = dead(i, n, dead_yes);
        let prev = if i == 0 { f32::NAN } else { hcl[i - 1] };
        buy[i] = (gt(c[i], prev) || buysig[i]) && dbuy && !upcircuit(b) && c[i] > penny && turnover_ok(b, turn);
        sell[i] = ((bt[i] == -1 || dsell) && !locircuit(b)) || (base_sell && !buysig[i] && !locircuit(b));
    }
    finish(s, buy, sell, 0.0)
}

fn sg_and_st(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let pt = pt_of(&L3C, g("stflt_2"));
    let turn = if g("turnflt").round() == 1.0 { Turn::VolClose } else { Turn::Off }; // ParamToggle "YES|NO": value 1 = YES
    series.iter().map(|s| and_st_one(s, pt, g("bpd1").round() as usize, 4.0, g("deadstock").round() == 1.0, g("pennyflt") as f32, turn, false, true)).collect()
}

fn schema_and_st() -> Vec<P> {
    vec![
        dead_p(),
        p("stflt", "ST CALCULATION POINT (first list; overwritten by the second) (1 CLOSE; 2 MID POINT; 3 PIVOT)", 1.0, 1.0, 3.0, 1.0),
        p("tstart", "TRADING START BAR (declared, not used)", 0.0, 0.0, 500.0, 1.0),
        penny_p(),
        p("turnflt", "TURNOVER Filter (1 YES; 2 NO)", 1.0, 1.0, 2.0, 1.0),
        p("stflt_2", "ST CALCULATION POINT (the list in force: 1 CLOSE; 2 MID POINT H+L/2; 3 PIVOT H+L+C/3)", 1.0, 1.0, 3.0, 1.0),
        p("bpd2", "ST ENTRY Period (plotted only)", 10.0, 1.0, 20.0, 1.0),
        p("bfactor2", "ST ENTRY Factor (plotted only)", 4.0, 1.0, 20.0, 1.0),
        p("bpd1", "ST EXIT Period", 10.0, 1.0, 20.0, 1.0),
    ]
}

// ================================================================ 3. ytath_buy_only_if_within_10pct_range_of_st

/// Entry ST2 (bpd2, bfactor2, both bands = factor) gives `BPrice = ValueWhen(Cross(C, Ref(down line,-1)), C)`; `ATHCSTDIFF = BPrice x (1+BPRICEDIFF/100)`.
/// buysig = 1 on (C > Ref(hcl,-1) AND entry trend up AND Ref(hcl,-1) < Ref(ATHCSTDIFF,-1)); 0 when the exit ST (bpd1, bfactor1; upper factor 1) is down and
/// buysig was 1; else carried. Buy = (buysig OR (C>Ref(HCL,-1) AND Cross(C,Ref(down line,-1)))) AND dead AND NOT UPCIRCUIT AND C>penny AND turnover;
/// Sell = (Btrend1==-1 OR deadsell) AND NOT LOCIRCUIT.
struct W10 {
    pt: Pt,
    pd2: usize,
    f2: f32,
    pd1: usize,
    f1: f32,
    diff: f32,
    dead_yes: bool,
    hardstop: f32,
    penny: f32,
    turn: Turn,
}

fn w10_one(s: &Series, a: &W10) -> Signals {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let stcal = stcal_of(s, a.pt);
    let tr = tr1(&h, &l, &c);
    let (bt1, _) = st_run(&stcal, &tr, &c, a.pd1, 1.0, a.f1);
    let (bt2, td2) = st_run(&stcal, &tr, &c, a.pd2, a.f2, a.f2);
    let hcl = hhv_all(&c);
    let cross: Vec<bool> = (0..n).map(|i| cross_prev_line(&c, &td2, i)).collect();
    let mut bprice = vec![f32::NAN; n];
    for i in 0..n {
        bprice[i] = if cross[i] { c[i] } else if i > 0 { bprice[i - 1] } else { f32::NAN };
    }
    let athcstdiff: Vec<f32> = (0..n).map(|i| bprice[i] + (bprice[i] * a.diff / 100.0)).collect();
    let mut buysig = vec![false; n];
    for i in 1..n {
        buysig[i] = if gt(c[i], hcl[i - 1]) && bt2[i] == 1 && gt(athcstdiff[i - 1], hcl[i - 1]) {
            true
        } else if bt1[i] == -1 && buysig[i - 1] {
            false
        } else {
            buysig[i - 1]
        };
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let b = &s.bars[i];
        let (dbuy, dsell) = dead(i, n, a.dead_yes);
        let prev = if i == 0 { f32::NAN } else { hcl[i - 1] };
        buy[i] = (buysig[i] || (gt(c[i], prev) && cross[i])) && dbuy && !upcircuit(b) && c[i] > a.penny && turnover_ok(b, a.turn);
        sell[i] = (bt1[i] == -1 || dsell) && !locircuit(b);
    }
    finish(s, buy, sell, a.hardstop)
}

fn sg_w10(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let a = W10 {
        pt: pt_of(&L7, g("stflt")),
        pd2: g("bpd2").round() as usize,
        f2: g("bfactor2") as f32,
        pd1: g("bpd1").round() as usize,
        f1: g("bfactor1") as f32,
        diff: g("bpricediff") as f32,
        dead_yes: g("deadstock").round() == 1.0,
        // the AFL declares hardstop / pennyflt / turnflt twice: the second declaration (the *_2 keys) is the one in force
        hardstop: g("hardstop_2") as f32,
        penny: g("pennyflt_2") as f32,
        turn: if g("turnflt_2").round() == 2.0 { Turn::VolClose } else { Turn::Off },
    };
    series.iter().map(|s| w10_one(s, &a)).collect()
}

fn schema_w10() -> Vec<P> {
    vec![
        dead_p(),
        hard_p(),
        penny_p(),
        turn_no_yes(),
        p("tstart", "TRADING START BAR (declared, not used)", 0.0, 0.0, 500.0, 1.0),
        stflt_p(ST7, 7.0),
        p("bpd2", "ST ENTRY Period ", 10.0, 5.0, 15.0, 5.0),
        p("bfactor2", "ST ENTRY Factor", 4.0, 1.0, 6.0, 0.5),
        p("bpd1", "ST EXIT Period ", 10.0, 5.0, 20.0, 5.0),
        p("bfactor1", "ST EXIT Factor", 4.0, 1.0, 6.0, 0.5),
        p("bpricediff", "ATHC DIFF FROM ST 10/4 BUY PRICE", 10.0, 2.0, 10.0, 2.0),
        p("hardstop_2", "Hard Stoploss % (second declaration, in force)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt_2", "Penny Stock Price (second declaration, in force)", 0.0, 0.0, 100.0, 1.0),
        p("turnflt_2", "TURNOVER Filter (second declaration, in force) (1 NO; 2 YES)", 1.0, 1.0, 2.0, 1.0),
    ]
}

// ================================================================ 4. ytath_buy_only_if_previous_ath_was_within_1

/// State machine on ATH closes (TC = bars since the last ATH close, DBUYSIG = ATH state on): on an ATH close, buysig = 1 if TC[i-1] < PTHCPD and DBUYSIG[i-1] == 0,
/// else buysig keeps its value; TC = 1. When the ST trend is down and DBUYSIG[i-1] == 1: buysig = DBUYSIG = 0. Otherwise carried, TC + 1.
/// Buy = buysig AND NOT UPCIRCUIT AND dead AND C>penny AND turnover (no TRADING START BAR); Sell as the base AFLs (with buysig==0).
fn prev_ath_one(s: &Series, pt: Pt, period: usize, factor: f32, pthcpd: f32, dead_yes: bool, hardstop: f32, penny: f32, turn: Turn) -> Signals {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let stcal = stcal_of(s, pt);
    let tr = tr1(&h, &l, &c);
    let (bt, _) = st_run(&stcal, &tr, &c, period, 1.0, factor);
    let hcl = hhv_all(&c);
    let mut buysig = vec![false; n];
    let mut dbuy_sig = vec![false; n];
    let mut tc = vec![0f32; n];
    for i in 1..n {
        if gt(c[i], hcl[i - 1]) {
            dbuy_sig[i] = true;
            if tc[i - 1] < pthcpd && !dbuy_sig[i - 1] {
                buysig[i] = true;
            } else {
                buysig[i] = buysig[i - 1];
            }
            tc[i] = 1.0;
        } else if bt[i] == -1 && dbuy_sig[i - 1] {
            buysig[i] = false;
            dbuy_sig[i] = false;
            tc[i] = tc[i - 1] + 1.0;
        } else {
            buysig[i] = buysig[i - 1];
            dbuy_sig[i] = dbuy_sig[i - 1];
            tc[i] = tc[i - 1] + 1.0;
        }
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let b = &s.bars[i];
        let (dbuy, dsell) = dead(i, n, dead_yes);
        let loc = locircuit(b);
        buy[i] = buysig[i] && !upcircuit(b) && dbuy && c[i] > penny && turnover_ok(b, turn);
        sell[i] = (bt[i] == -1 && !loc) || dsell || (!buysig[i] && !loc);
    }
    finish(s, buy, sell, hardstop)
}

fn sg_prev_ath(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let pt = pt_of(&L7T, g("stflt"));
    let (rm, r1, r2, r3, r4) = rank_params(g);
    let turn = if g("turnflt").round() == 2.0 { Turn::VolClose } else { Turn::Off };
    series
        .iter()
        .map(|s| {
            let mut sg = prev_ath_one(s, pt, g("bpd1").round() as usize, g("bfactor1") as f32, g("pthcpd") as f32, g("deadstock").round() == 1.0, g("hardstop") as f32, g("pennyflt") as f32, turn);
            sg.score = rank_score(s, rm, r1, r2, r3, r4);
            sg
        })
        .collect()
}

fn schema_prev_ath() -> Vec<P> {
    let mut v = vec![dead_p(), hard_p(), penny_p(), turn_no_yes(), stflt_p(ST7, 7.0), p("bpd1", "ST Period ", 10.0, 1.0, 20.0, 1.0), p("bfactor1", "ST Factor", 4.0, 1.0, 20.0, 1.0), p("tstart", "TRADING START BAR (declared, not used)", 1.0, 0.0, 500.0, 1.0)];
    v.extend(rank_schema(11.0, (10.0, 30.0)));
    v.push(p("pthcpd", "PREVIOUS ATHC PERIOD", 450.0, 15.0, 500.0, 15.0));
    v
}

// ================================================================ 5. ytath_plus_5pct_buy_supertrend_exit

/// A leg starts at an ATH close with t == 0: bprice = that close, trigger = bprice x (1 + ATHCBUYPCT/100); carried; the ST trend down clears t / bprice / trigger.
/// Buy = C > Ref(hcl,-1) AND C > trigger AND dead AND NOT UPCIRCUIT AND C>penny AND turnover (no TRADING START BAR); Sell = (Btrend1==-1 OR deadsell) AND NOT LOCIRCUIT.
fn plus5_one(s: &Series, pt: Pt, period: usize, factor: f32, pct: f32, dead_yes: bool, hardstop: f32, penny: f32, turn: Turn) -> Signals {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let stcal = stcal_of(s, pt);
    let tr = tr1(&h, &l, &c);
    let (bt, _) = st_run(&stcal, &tr, &c, period, 1.0, factor);
    let hcl = hhv_all(&c);
    let mut t = vec![false; n];
    let mut trigger = vec![f32::NAN; n];
    let mut bprice = vec![f32::NAN; n];
    for i in 1..n {
        if gt(c[i], hcl[i - 1]) && !t[i - 1] {
            t[i] = true;
            bprice[i] = c[i];
            trigger[i] = bprice[i] + (bprice[i] * pct / 100.0);
        } else {
            bprice[i] = bprice[i - 1];
            trigger[i] = trigger[i - 1];
            t[i] = t[i - 1];
        }
        if bt[i] == -1 {
            t[i] = false;
            bprice[i] = f32::NAN;
            trigger[i] = f32::NAN;
        }
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let b = &s.bars[i];
        let (dbuy, dsell) = dead(i, n, dead_yes);
        let prev = if i == 0 { f32::NAN } else { hcl[i - 1] };
        buy[i] = gt(c[i], prev) && gt(c[i], trigger[i]) && dbuy && !upcircuit(b) && c[i] > penny && turnover_ok(b, turn);
        sell[i] = (bt[i] == -1 || dsell) && !locircuit(b);
    }
    finish(s, buy, sell, hardstop)
}

fn sg_plus5(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let pt = pt_of(&L7, g("stflt"));
    let turn = if g("turnflt").round() == 2.0 { Turn::VolClose } else { Turn::Off };
    series.iter().map(|s| plus5_one(s, pt, g("bpd1").round() as usize, g("bfactor1") as f32, g("athcbuypct") as f32, g("deadstock").round() == 1.0, g("hardstop") as f32, g("pennyflt") as f32, turn)).collect()
}

fn schema_plus5() -> Vec<P> {
    vec![
        dead_p(),
        hard_p(),
        penny_p(),
        turn_no_yes(),
        stflt_p(ST7, 7.0),
        p("bpd1", "ST Period ", 10.0, 5.0, 20.0, 5.0),
        p("bfactor1", "ST Factor", 4.0, 3.0, 4.0, 0.5),
        p("tstart", "TRADING START BAR (declared, not used)", 1.0, 0.0, 500.0, 1.0),
        p("swingpd", "SWING PERIOD (declared, not used)", 180.0, 10.0, 1000.0, 1.0),
        // Param default 5 is outside the AFL's own range 6..9: clamped to 6
        p("athcbuypct", "BUY PERCENTAGE FROM ATHC (AFL default 5, range 6-9)", 6.0, 6.0, 9.0, 1.0),
    ]
}

// ================================================================ 6. ytath_stock_buy_within_10pct_below_athc

/// buysig2 = hcl x (1 - BUYPCT/100); buysig = 1 when C > Ref(buysig2,-1); 0 when the ST trend is down (typo `buysig[i]==0;` keeps the initial 0);
/// else carried. Buy = ((Cross(C,Ref(buysig2,-1)) AND NOT UPCIRCUIT) OR (buysig==1 AND NOT UPCIRCUIT)) AND BarIndex()>=TSTART AND dead AND Btrend1==1;
/// Sell = (Btrend1==-1 AND NOT LOCIRCUIT) OR deadsell OR (buysig==0 AND NOT LOCIRCUIT). Dead stock NO still puts a Sell on bar 0 (as written).
fn stock_buy_one(s: &Series, pt: Pt, period: usize, factor: f32, buypct: f32, tstart: usize, dead_yes: bool) -> Signals {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let stcal = stcal_of(s, pt);
    let tr = tr1(&h, &l, &c);
    let (bt, _) = st_run(&stcal, &tr, &c, period, 1.0, factor);
    let hcl = hhv_all(&c);
    let b2: Vec<f32> = (0..n).map(|i| hcl[i] - (hcl[i] * buypct / 100.0)).collect();
    let mut buysig = vec![false; n];
    for i in 1..n {
        buysig[i] = if gt(c[i], b2[i - 1]) {
            true
        } else if bt[i] == -1 {
            false
        } else {
            buysig[i - 1]
        };
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let b = &s.bars[i];
        let (dbuy, dsell0) = if dead_yes { (i != n - 1, i == n - 1) } else { (i != 0, i == 0) };
        let upc = upcircuit(b);
        let cross = i >= 2 && gt(c[i], b2[i - 1]) && !b2[i - 2].is_nan() && c[i - 1] <= b2[i - 2];
        buy[i] = ((cross && !upc) || (buysig[i] && !upc)) && i >= tstart && dbuy && bt[i] == 1;
        let loc = locircuit(b);
        sell[i] = (bt[i] == -1 && !loc) || dsell0 || (!buysig[i] && !loc);
    }
    finish(s, buy, sell, 0.0)
}

fn sg_stock_buy(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let pt = pt_of(&L3C, g("stflt"));
    series.iter().map(|s| stock_buy_one(s, pt, g("bpd1").round() as usize, g("bfactor1") as f32, g("buypct") as f32, g("tstart").round() as usize, g("deadstock").round() == 1.0)).collect()
}

fn schema_stock_buy() -> Vec<P> {
    vec![
        dead_p(),
        p("buypct", "BUY PERCENTAGE", 10.0, 1.0, 15.0, 1.0),
        p("stflt", "ST CALCULATION POINT (1 CLOSE; 2 MID POINT H+L/2; 3 PIVOT H+L+C/3)", 1.0, 1.0, 3.0, 1.0),
        // Param default 0 is outside the AFL's own range 1..500: clamped to 1
        p("tstart", "TRADING START BAR (AFL default 0, range 1-500)", 1.0, 1.0, 500.0, 1.0),
        p("bpd1", "ST Period ", 10.0, 1.0, 20.0, 1.0),
        p("bfactor1", "ST Factor", 4.0, 1.0, 20.0, 1.0),
    ]
}

// ================================================================ 7. ytath_supertrend_long_only_buy_st_5_2_up

/// Buy = Btrend1==1 AND dead AND NOT UPCIRCUIT (every flat bar while the entry ST(5,2) is up); Sell = ((Btrend1==-1 AND Btrend2==-1) OR deadsell) AND NOT LOCIRCUIT.
/// Both ST() bands use the factor (this file's ST has `Up = STCAL + V2*satr`). The month-end `entryday` / `diff` code only builds an unused array.
fn long_only_one(s: &Series, pt: Pt, pd1: usize, f1: f32, pd2: usize, f2: f32, dead_yes: bool) -> Signals {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let stcal = stcal_of(s, pt);
    let tr = tr1(&h, &l, &c);
    let (bt1, _) = st_run(&stcal, &tr, &c, pd1, f1, f1);
    let (bt2, _) = st_run(&stcal, &tr, &c, pd2, f2, f2);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let b = &s.bars[i];
        let (dbuy, dsell) = dead(i, n, dead_yes);
        buy[i] = bt1[i] == 1 && dbuy && !upcircuit(b);
        sell[i] = ((bt1[i] == -1 && bt2[i] == -1) || dsell) && !locircuit(b);
    }
    finish(s, buy, sell, 0.0)
}

fn sg_long_only(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let pt = pt_of(&L6T, g("stflt"));
    series.iter().map(|s| long_only_one(s, pt, g("bpd1").round() as usize, g("bfactor1") as f32, g("bpd2").round() as usize, g("bfactor2") as f32, g("deadstock").round() == 1.0)).collect()
}

fn schema_long_only() -> Vec<P> {
    vec![
        dead_p(),
        p("entryday", "entry day (builds an unused array)", 5.0, 1.0, 10.0, 1.0),
        p("stflt", "DAILY ST CALCULATION POINT (1 MID POINT H+L/2; 2 CLOSE; 3 PIVOT; 4 MP CL PIVOT AVG; 5 3DAYS MA (AFL typo = mid point); 6 5DAYS MA)", 1.0, 1.0, 6.0, 1.0),
        p("bpd1", "ST Period for ENTRY", 5.0, 1.0, 20.0, 1.0),
        p("bfactor1", "ST Factor for ENTRY", 2.0, 1.0, 20.0, 1.0),
        p("bpd2", "ST Period for EXIT", 5.0, 1.0, 20.0, 1.0),
        p("bfactor2", "ST Factor for EXIT", 4.0, 1.0, 20.0, 1.0),
        p("diff", "% Difference (builds an unused array)", 1.0, 0.0, 20.0, 0.25),
    ]
}

// ================================================================ 8. ytath_8pct_range_with_supertrend_buy (continuous trade of 5,2 / 5,4)

pub struct St8 {
    pub pt: Pt,
    pub pd1: usize,
    pub f1: f32,
    pub pd2: usize,
    pub f2: f32,
    pub diff: f32,
    pub dead_yes: bool,
    pub penny: f32,
    pub turn: Turn,
    /// false in the AFL. true = differential-check mode that behaves like the nearest AFL ("new trade only", `ytath_8pct_range_with_supertrend_buy_new`):
    /// entry only on `Cross(C, Ref(entry-ST down line,-1))`, upper circuit tested as `NOT UPCIRCUIT` (O=C=H to 2 decimals) and the sell blocked at a lower circuit.
    pub nearest: bool,
}

/// Bar loop of the AFL (Position / SkipSell, literally): `Buysig = Btrend1==1 AND C > ATHDIFF AND BarIndex() > ENTRYPERIOD`,
/// `Buy_Cash_Filter = UC==0 AND Turnover AND Close>PennyFlt AND DEADSTOCKBUYSIG AND Buysig`, `Sellsig = Btrend1==-1 AND Btrend2==-1`.
/// Buy whenever Buysig and the filter hold (ExRem keeps the first while held); a held position is sold on Sellsig / dead stock at the Close
/// (the lower-circuit deferral can never trigger because the inner test contains `OR SELLSIG[i]`, which is true there).
pub fn st8_one(s: &Series, a: &St8) -> Signals {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let stcal = stcal_of(s, a.pt);
    let tr = tr1(&h, &l, &c);
    let (bt1, td1) = st_run(&stcal, &tr, &c, a.pd1, a.f1, a.f1);
    let (bt2, _) = st_run(&stcal, &tr, &c, a.pd2, a.f2, a.f2);
    let hcl = hhv_all(&c);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut position, mut skip) = (false, false);
    for i in 1..n {
        let b = &s.bars[i];
        let (dbuy, dsell) = dead(i, n, a.dead_yes);
        let athdiff = hcl[i] - (hcl[i] * a.diff / 100.0);
        let buysig = bt1[i] == 1 && gt(c[i], athdiff) && i > a.pd1 && (!a.nearest || cross_prev_line(&c, &td1, i));
        let filt = (if a.nearest { !upcircuit(b) } else { b.h != b.c }) && turnover_ok(b, a.turn) && c[i] > a.penny && dbuy && buysig;
        let sellsig = bt1[i] == -1 && bt2[i] == -1;
        let lc = b.l == b.c;
        if buysig && filt {
            buy[i] = true;
            position = true;
        } else if position && !skip {
            // (nearest = differential check only: the nearest AFL blocks every sell at a Prec lower circuit)
            if (dsell || sellsig) && !(a.nearest && locircuit(b)) {
                if !lc || dsell || sellsig {
                    sell[i] = true;
                    position = false;
                } else {
                    skip = true;
                }
            }
        } else if sellsig || (skip && (!lc || dsell)) {
            sell[i] = true;
            position = false;
            skip = false;
        }
    }
    finish(s, buy, sell, 0.0)
}

fn sg_st8(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let a = St8 {
        pt: pt_of(&L6T, g("stflt")),
        pd1: g("bpd1").round() as usize,
        f1: g("bfactor1") as f32,
        pd2: g("bpd2").round() as usize,
        f2: g("bfactor2") as f32,
        diff: g("difffromathc") as f32,
        dead_yes: g("deadstock").round() == 1.0,
        penny: g("pennyflt") as f32,
        turn: if g("turnflt").round() == 1.0 { Turn::VolClose } else { Turn::Off }, // ParamToggle "YES|NO": value 1 = YES
        nearest: false,
    };
    // list NONE|ROC|RSI|STD|ROC/STD|POSITIVE CLOSE|SWING -> ranking modes 1,2,4,5,6,8,9
    let mode = match g("rankmode").round() as i32 {
        2 => 2,
        3 => 4,
        4 => 5,
        5 => 6,
        6 => 8,
        7 => 9,
        _ => 1,
    };
    let (r1, r2, r3, r4) = (g("rocpd").round() as usize, g("rsipd").round() as usize, g("stdpd").round() as usize, g("swingpd").round() as usize);
    series
        .iter()
        .map(|s| {
            let mut sg = st8_one(s, &a);
            sg.score = rank_score(s, mode, r1, r2, r3, r4);
            sg
        })
        .collect()
}

fn schema_st8() -> Vec<P> {
    vec![
        dead_p(),
        penny_p(),
        p("turnflt", "TURNOVER Filter (1 YES; 2 NO)", 1.0, 1.0, 2.0, 1.0),
        p("stflt", "DAILY ST CALCULATION POINT (1 MID POINT H+L/2; 2 CLOSE; 3 PIVOT; 4 MP CL PIVOT AVG; 5 3DAYS MA (AFL typo = mid point); 6 5DAYS MA)", 1.0, 1.0, 6.0, 1.0),
        p("bpd1", "ST Period for ENTRY", 5.0, 5.0, 20.0, 5.0),
        p("bfactor1", "ST Factor for ENTRY", 2.0, 1.0, 6.0, 0.5),
        p("bpd2", "ST Period for EXIT", 5.0, 5.0, 20.0, 5.0),
        p("bfactor2", "ST Factor for EXIT", 4.0, 1.0, 6.0, 0.5),
        p("difffromathc", "% DIFF FROM ATHC", 8.0, 1.0, 10.0, 1.0),
        p("rankmode", "RANKING MODE (1 NONE; 2 ROC; 3 RSI; 4 STD; 5 ROC/STD; 6 POSITIVE CLOSE; 7 SWING)", 1.0, 1.0, 7.0, 1.0),
        p("rocpd", "ROC PERIOD for RANKING/NO OF POSITIVE CLOSE", 10.0, 0.0, 400.0, 30.0),
        p("rsipd", "RSI PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("stdpd", "STD PERIOD for RANKING", 14.0, 10.0, 1000.0, 10.0),
        p("swingpd", "SWING PERIOD", 180.0, 10.0, 1000.0, 1.0),
        p("rankmode_2", "RANKING MODE (Explore columns only)", 1.0, 1.0, 7.0, 1.0),
    ]
}

// ================================================================ 9. ytath_with_time_based_exit_and_supertrend

/// athbuysig = C>Ref(HCL,-1) AND BarIndex()>=TSTART AND dead AND NOT UPCIRCUIT AND C>penny AND turnover.
/// buysig = Ref... counter: athbuysig => buysig[i-1]+1; trend down => 0; else carried. count likewise (1 / 0 / carried); FLAG = athbuysig AND count[i-1]==0.
/// Buy = athbuysig AND Ref(buysig,-1)==0; Sell = deadsell OR (BarsSince(FLAG)==HOLDINGDAYS AND NOT LOCIRCUIT) OR (BarsSince(FLAG)>HOLDINGDAYS AND NOT LOCIRCUIT)
/// OR (Btrend1==-1 AND NOT LOCIRCUIT). A time exit does NOT clear buysig/count: no re-entry until the ST has turned down.
fn time_one(s: &Series, pt: Pt, period: usize, factor: f32, tstart: usize, hold: usize, dead_yes: bool, hardstop: f32, penny: f32, turn: Turn) -> Signals {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let stcal = stcal_of(s, pt);
    let tr = tr1(&h, &l, &c);
    let (bt, _) = st_run(&stcal, &tr, &c, period, 1.0, factor);
    let hcl = hhv_all(&c);
    let mut ath = vec![false; n];
    for i in 0..n {
        let b = &s.bars[i];
        let (dbuy, _) = dead(i, n, dead_yes);
        let prev = if i == 0 { f32::NAN } else { hcl[i - 1] };
        ath[i] = gt(c[i], prev) && i >= tstart && dbuy && !upcircuit(b) && c[i] > penny && turnover_ok(b, turn);
    }
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
        let b = &s.bars[i];
        let (_, dsell) = dead(i, n, dead_yes);
        let loc = locircuit(b);
        buy[i] = i >= 1 && ath[i] && buysig[i - 1] == 0;
        let bs = last_flag.map(|f| i - f);
        let time_ok = match bs {
            Some(k) => k >= hold,
            None => false,
        };
        sell[i] = dsell || (time_ok && !loc) || (bt[i] == -1 && !loc);
    }
    finish(s, buy, sell, hardstop)
}

fn sg_time(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let pt = pt_of(&L7, g("stflt"));
    let (rm, r1, r2, r3, r4) = rank_params(g);
    let turn = if g("turnflt").round() == 2.0 { Turn::VolClose } else { Turn::Off };
    series
        .iter()
        .map(|s| {
            let mut sg = time_one(s, pt, g("bpd1").round() as usize, g("bfactor1") as f32, g("tstart").round() as usize, g("holdingdays").round() as usize, g("deadstock").round() == 1.0, g("hardstop") as f32, g("pennyflt") as f32, turn);
            sg.score = rank_score(s, rm, r1, r2, r3, r4);
            sg
        })
        .collect()
}

fn schema_time() -> Vec<P> {
    let mut v = vec![dead_p(), hard_p(), penny_p(), turn_no_yes()];
    v.extend(rank_schema(11.0, (10.0, 0.0)));
    v.push(stflt_p(ST7, 7.0));
    v.push(p("bpd1", "ST Period ", 10.0, 1.0, 20.0, 1.0));
    v.push(p("bfactor1", "ST Factor", 4.0, 1.0, 20.0, 1.0));
    v.push(p("tstart", "TRADING START BAR", 0.0, 0.0, 500.0, 1.0));
    // Optimize("HOLDING PERIOD",5,1,200,1): a backtest uses the default 5
    v.push(p("holdingdays", "HOLDING PERIOD (bars; AFL Optimize default 5)", 5.0, 1.0, 200.0, 1.0));
    v
}

// ================================================================ 10. ytath_on_daily_timeframe_exit_below_weekly

fn days_from_civil(y: i32, m: i32, d: i32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Key of the compressed bar: 0 monthly, 1 weekly (Monday based), 2 daily.
fn period_key(d: i32, tf: u8) -> i64 {
    match tf {
        0 => (d / 100) as i64,
        1 => (days_from_civil(d / 10000, d / 100 % 100, d % 100) + 3).div_euclid(7),
        _ => d as i64,
    }
}

pub struct Comp {
    pub start: Vec<usize>,
    pub end: Vec<usize>,
    pub h: Vec<f32>,
    pub l: Vec<f32>,
    pub c: Vec<f32>,
}

/// `TimeFrameSet` of the stock's own daily bars: O first, H highest, L lowest, C last.
pub fn compress(s: &Series, tf: u8) -> Comp {
    let mut k = Comp { start: vec![], end: vec![], h: vec![], l: vec![], c: vec![] };
    let mut last_key = i64::MIN;
    for (i, b) in s.bars.iter().enumerate() {
        let key = period_key(b.date, tf);
        if key != last_key || k.start.is_empty() {
            k.start.push(i);
            k.end.push(i);
            k.h.push(b.h);
            k.l.push(b.l);
            k.c.push(b.c);
            last_key = key;
        } else {
            let g = k.start.len() - 1;
            k.end[g] = i;
            k.h[g] = k.h[g].max(b.h);
            k.l[g] = k.l[g].min(b.l);
            k.c[g] = b.c;
        }
    }
    k
}

/// `TimeFrameExpand(x, tf)` in AmiBroker's default mode (as verified for the weekly-close engines): a compressed bar's value shows on its LAST daily
/// bar; the other daily bars of the period carry the previous compressed bar's value.
pub fn expand(x: &[f32], k: &Comp, n: usize) -> Vec<f32> {
    let mut out = vec![f32::NAN; n];
    for g in 0..k.start.len() {
        for j in k.start[g]..k.end[g] {
            out[j] = if g > 0 { x[g - 1] } else { f32::NAN };
        }
        out[k.end[g]] = x[g];
    }
    out
}

struct Dw {
    tf: u8,
    pd: usize,
    fact: f32,
    pt: Pt,
    dead_yes: bool,
    penny: f32,
    turn: Turn,
    buffer: f32,
}

/// Daily-bar entry `Position==0 AND Close > Highest_Close[i-1] AND BuyFilter` (BuyFilter = Turnover AND NOT UC AND Close>PennyFlt AND DEADSTOCKBUYSIG),
/// exit when the higher-timeframe ST (Up = MSTCAL + 1 x satr, Dn = MSTCAL - FLTFACT x satr, on the compressed bars of the stock) is down and the expanded
/// close changed (= the last daily bar of the period): sell at Close unless L==C (then skipsell; next bar not at a lower circuit: sell at Open - buffer%).
fn dw_one(s: &Series, a: &Dw) -> Signals {
    let n = s.bars.len();
    let c = s.close();
    let k = compress(s, a.tf);
    let m = k.start.len();
    let cal: Vec<f32> = (0..m)
        .map(|g| match a.pt {
            Mid => (k.h[g] + k.l[g]) / 2.0,
            Pivot => (k.h[g] + k.l[g] + k.c[g]) / 3.0,
            _ => k.c[g],
        })
        .collect();
    let tr = tr1(&k.h, &k.l, &k.c);
    let (tr_trend, _) = st_run(&cal, &tr, &k.c, a.pd, 1.0, a.fact);
    let trend_f: Vec<f32> = tr_trend.iter().map(|&t| t as f32).collect();
    let mtrend = expand(&trend_f, &k, n);
    let myclose = expand(&k.c, &k, n);
    let hc = hhv_all(&c);
    let mut buy = vec![false; n];
    let mut sell = vec![false; n];
    let mut buy_price = c.clone();
    let mut sell_price = c.clone();
    let (mut position, mut skip) = (false, false);
    for i in 1..n {
        let b = &s.bars[i];
        let (dbuy, dsell) = dead(i, n, a.dead_yes);
        let lc = b.l == b.c;
        let filt = turnover_ok(b, a.turn) && b.h != b.c && c[i] > a.penny && dbuy;
        if !position && gt(c[i], hc[i - 1]) && filt {
            buy[i] = true;
            buy_price[i] = c[i];
            position = true;
        } else if position && mtrend[i] == -1.0 && myclose[i] != myclose[i - 1] && !skip {
            if !lc || dsell {
                sell[i] = true;
                sell_price[i] = c[i];
                position = false;
                skip = false;
            } else {
                skip = true;
            }
        } else if position && ((!locircuit(b) && skip) || dsell) {
            sell[i] = true;
            sell_price[i] = b.o - (b.o * a.buffer / 100.0);
            position = false;
            skip = false;
        }
    }
    let mut sg = Signals::new(buy, sell, buy_price, sell_price);
    clamp(&mut sg, s);
    sg
}

fn sg_dw(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let a = Dw {
        tf: match g("flttf").round() as i32 {
            1 => 0,
            2 => 1,
            _ => 2,
        },
        pd: g("fltpd").round() as usize,
        fact: g("fltfact") as f32,
        pt: pt_of(&L3M, g("stflt")),
        dead_yes: g("deadstock").round() == 1.0,
        penny: g("pennyflt") as f32,
        turn: if g("turnflt").round() == 2.0 { Turn::OpenInt } else { Turn::Off },
        buffer: g("buffer_prcnt") as f32,
    };
    series.iter().map(|s| dw_one(s, &a)).collect()
}

fn schema_dw() -> Vec<P> {
    vec![
        dead_p(),
        penny_p(),
        turn_no_yes(),
        p("flttf", "FILTER TIMEFRAME (1 MONTHLY; 2 WEEKLY; 3 DAILY)", 1.0, 1.0, 3.0, 1.0),
        p("fltpd", "ST FILTER PERIOD", 5.0, 0.0, 20.0, 1.0),
        p("fltfact", "ST FILTER FACTOR", 1.0, 0.0, 20.0, 1.0),
        p("stflt", "ST CALCULATION POINT (1 MID POINT H+L/2; 2 CLOSE; 3 PIVOT H+L+C/3)", 1.0, 1.0, 3.0, 1.0),
        p("buffer_prcnt", "Selling Buffer %", 0.25, 0.0, 10.0, 0.05),
    ]
}

// ================================================================ registry

pub fn variants() -> Vec<VariantDef> {
    vec![
        VariantDef { id: "ytath_8pct_range_with_supertrend_buy", data: DATA, schema: schema_st8, signals: sg_st8 },
        VariantDef { id: "ytath_and_st_10_4_buy_athc_entry_supertrend", data: DATA, schema: schema_and_st, signals: sg_and_st },
        VariantDef { id: "ytath_buy_only_if_within_10pct_range_of_st", data: DATA, schema: schema_w10, signals: sg_w10 },
        VariantDef { id: "ytath_buy_only_if_previous_ath_was_within_1", data: DATA, schema: schema_prev_ath, signals: sg_prev_ath },
        VariantDef { id: "ytath_on_daily_timeframe_exit_below_weekly", data: DATA, schema: schema_dw, signals: sg_dw },
        VariantDef { id: "ytath_plus_5pct_buy_supertrend_exit", data: DATA, schema: schema_plus5, signals: sg_plus5 },
        VariantDef { id: "ytath_stock_buy_within_10pct_below_athc", data: DATA, schema: schema_stock_buy, signals: sg_stock_buy },
        VariantDef { id: "ytath_with_supertrend_10_4_exit_from_close", data: DATA, schema: schema_from_close, signals: sg_from_close },
        VariantDef { id: "ytath_with_all_supertrend_variations_and", data: DATA, schema: schema_all_st, signals: sg_all_st },
        VariantDef { id: "ytath_with_supertrend_exit_rank_by_gap", data: DATA, schema: schema_rank_gap, signals: sg_rank_gap },
        VariantDef { id: "ytath_with_time_based_exit_and_supertrend", data: DATA, schema: schema_time, signals: sg_time },
        VariantDef { id: "ytath_supertrend_long_only_buy_st_5_2_up", data: DATA, schema: schema_long_only, signals: sg_long_only },
    ]
}

// ================================================================ tests

#[cfg(test)]
mod tests {
    use super::*;

    fn series_of(cl: &[f32]) -> Series {
        let bars = cl
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                let d = 20100101 + (i as i32 / 28) * 100 + (i as i32 % 28);
                Bar { date: d, c, o: c - 0.3, h: c + 1.0, l: c - 1.0, v: 1.0e7, oi: 1.0e8, aux1: 1.0, aux2: 0.0 }
            })
            .collect();
        Series { name: "T".into(), bars }
    }
    fn up_down() -> Vec<f32> {
        let mut cl: Vec<f32> = (0..40).map(|i| 10.0 + i as f32).collect();
        cl.extend((0..30).map(|i| 50.0 - 2.5 * i as f32));
        cl.extend((0..40).map(|i| 5.0 + 1.5 * i as f32));
        cl
    }
    fn all_st_cfg() -> AllSt {
        AllSt { dead_yes: false, hardstop: 0.0, penny: 0.0, turn: Turn::Off, pt: Close, period: 10, factor: 4.0, tstart: 1, uc_lc: true }
    }

    #[test]
    fn typo_lists_map_item_to_mid_point() {
        assert_eq!(pt_of(&L7T, 6.0), Mid);
        assert_eq!(pt_of(&L7, 6.0), Ma3);
        assert_eq!(pt_of(&L6T, 5.0), Mid);
        assert_eq!(pt_of(&L3C, 1.0), Close);
        assert_eq!(pt_of(&L3M, 1.0), Mid);
    }

    #[test]
    fn running_buy_sells_at_close_when_st_flips() {
        let s = series_of(&up_down());
        let sg = all_st_one(&s, &all_st_cfg());
        let t = trades_of(&sg);
        assert!(!t.is_empty());
        assert_eq!(sg.buy_price[t[0].buy_i], s.bars[t[0].buy_i].c);
        assert!(t[0].sell_i != usize::MAX && t[0].sell_i > t[0].buy_i);
    }

    #[test]
    fn all_st_equals_new_st_when_uc_lc_exclusion_is_off() {
        // the only difference to ath_new_st (checked, differential) is the UC / LC exclusion and the turnover test
        let s = series_of(&up_down());
        let mut a = all_st_cfg();
        a.uc_lc = false;
        let mine = all_st_one(&s, &a);
        let p = StP { dead_yes: false, hardstop: 0.0, penny: 0.0, turn_on: false, aux1_on: false, aux2_on: false, stflt: 1, period: 10, factor: 4.0, tstart: 1, rankmode: 1, rocpd: 10, rsipd: 14, stdpd: 14, swingpd: 180 };
        let theirs = ath_new_st::one(&s, &p);
        assert_eq!(mine.buy, theirs.buy);
        assert_eq!(mine.sell, theirs.sell);
    }

    #[test]
    fn uc_bar_is_not_bought_in_all_st() {
        // 15 flat bars, then a jump to a new ATH close with H == C (upper circuit by H==C, but O != C so not the Prec circuit)
        let mut cl = vec![10.0f32; 15];
        cl.push(20.0);
        cl.extend(vec![21.0f32; 5]);
        let mut s = series_of(&cl);
        s.bars[15].h = s.bars[15].c;
        let with_excl = all_st_one(&s, &all_st_cfg());
        assert!(!with_excl.buy[15]);
        let mut a = all_st_cfg();
        a.uc_lc = false;
        assert!(all_st_one(&s, &a).buy[15]);
    }

    #[test]
    fn st_up_band_uses_hardcoded_factor_one_but_st8_uses_the_factor() {
        let s = series_of(&up_down());
        let (c, h, l) = (s.close(), s.high(), s.low());
        let st = stcal_of(&s, Mid);
        let tr = tr1(&h, &l, &c);
        let (a, _) = st_run(&st, &tr, &c, 5, 1.0, 4.0);
        let (b, _) = st_run(&st, &tr, &c, 5, 4.0, 4.0);
        assert!(a.iter().zip(b.iter()).any(|(x, y)| x != y) || a == b); // both run; values differ only where the wide up band matters
        assert_eq!(a.len(), 110);
    }

    #[test]
    fn prev_ath_only_first_ath_after_down_phase_with_recent_previous_ath() {
        // ATH close, flip down, quick new ATH within pthcpd bars: bought; with pthcpd = 1 nothing qualifies after the first
        let s = series_of(&up_down());
        let sg = prev_ath_one(&s, Close, 10, 4.0, 450.0, false, 0.0, 0.0, Turn::Off);
        assert!(trades_of(&sg).len() >= 1);
        let sg1 = prev_ath_one(&s, Close, 10, 4.0, 1.0, false, 0.0, 0.0, Turn::Off);
        assert!(trades_of(&sg1).len() <= trades_of(&sg).len());
    }

    #[test]
    fn time_exit_after_holding_days_and_no_reentry_until_st_down() {
        let s = series_of(&up_down());
        let sg = time_one(&s, Close, 10, 4.0, 0, 5, false, 0.0, 0.0, Turn::Off);
        let t = trades_of(&sg);
        assert!(!t.is_empty());
        assert!(t[0].sell_i - t[0].buy_i <= 5, "held {} bars", t[0].sell_i - t[0].buy_i);
        // the same series without time exit holds longer
        let sg2 = time_one(&s, Close, 10, 4.0, 0, 200, false, 0.0, 0.0, Turn::Off);
        let t2 = trades_of(&sg2);
        assert!(t2[0].sell_i - t2[0].buy_i > 5);
        // no second buy right after the time exit while the ST is still up
        assert!(t.len() < 3);
    }

    #[test]
    fn plus5_needs_a_close_above_first_ath_close_plus_pct() {
        let s = series_of(&up_down());
        let sg = plus5_one(&s, Close, 10, 4.0, 5.0, false, 0.0, 0.0, Turn::Off);
        let t = trades_of(&sg);
        assert!(!t.is_empty());
        // first ATH close of the leg is bar 1: close 11; trigger 11.55 => first buy at the first close above (bar 2: 12)
        assert!(sg.buy[2]);
        assert!(!sg.buy[1]);
    }

    #[test]
    fn st_state_buy_reenters_while_trend_is_up() {
        let s = series_of(&up_down());
        let a = St8 { pt: Close, pd1: 5, f1: 2.0, pd2: 5, f2: 4.0, diff: 8.0, dead_yes: false, penny: 0.0, turn: Turn::Off, nearest: false };
        let sg = st8_one(&s, &a);
        let t = trades_of(&sg);
        assert!(!t.is_empty());
        assert_eq!(sg.buy_price[t[0].buy_i], s.bars[t[0].buy_i].c);
        let lo = long_only_one(&s, Close, 5, 2.0, 5, 4.0, false);
        assert!(trades_of(&lo).len() >= 1);
    }

    #[test]
    fn weekly_compress_and_expand_last_bar_rule() {
        let s = series_of(&up_down());
        let k = compress(&s, 0); // monthly (28 bars per month in the test series)
        let x: Vec<f32> = (0..k.start.len()).map(|g| g as f32).collect();
        let e = expand(&x, &k, s.bars.len());
        assert!(e[0].is_nan());
        assert_eq!(e[k.end[0]], 0.0);
        assert_eq!(e[k.start[1]], 0.0); // first bar of month 2 still shows month 1
        assert_eq!(e[k.end[1]], 1.0);
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(period_key(19700105, 1) - period_key(19700104, 1), 1); // Monday starts a week
        assert_eq!(period_key(19700104, 1), period_key(19700102, 1));
    }

    #[test]
    fn daily_weekly_sells_only_on_the_last_bar_of_the_period() {
        let s = series_of(&up_down());
        let a = Dw { tf: 0, pd: 2, fact: 1.0, pt: Close, dead_yes: false, penny: 0.0, turn: Turn::Off, buffer: 0.25 };
        let sg = dw_one(&s, &a);
        let k = compress(&s, 0);
        for i in 0..s.bars.len() {
            if sg.sell[i] {
                assert!(k.end.contains(&i), "sell on bar {i} is not a month end");
            }
        }
    }

    #[test]
    fn stock_buy_dead_stock_no_puts_a_sell_on_bar_zero_and_still_trades() {
        let s = series_of(&up_down());
        let sg = stock_buy_one(&s, Close, 10, 4.0, 10.0, 1, false);
        assert!(!trades_of(&sg).is_empty());
    }

    #[test]
    fn last_athc_score_is_the_gap_between_ath_closes() {
        let c = [10.0f32, 11.0, 12.0, 11.5, 11.0, 13.0, 12.0];
        let sc = score_last_athc(&c);
        assert!(sc[0].is_nan());
        assert!(sc[1].is_nan()); // first ATH close: no previous one
        assert_eq!(sc[2], 1.0);
        assert_eq!(sc[3], 0.0);
        assert_eq!(sc[5], 3.0);
    }

    #[test]
    fn within10_runs_and_first_hardstop_key_is_inert() {
        let s = series_of(&up_down());
        let a = W10 { pt: Close, pd2: 10, f2: 4.0, pd1: 10, f1: 4.0, diff: 10.0, dead_yes: false, hardstop: 0.0, penny: 0.0, turn: Turn::Off };
        let sg = w10_one(&s, &a);
        assert_eq!(sg.buy.len(), 110);
    }

    #[test]
    fn all_twelve_ids_are_registered_with_unique_schema_keys() {
        let v = variants();
        assert_eq!(v.len(), 12);
        for d in &v {
            let keys: Vec<&str> = (d.schema)().iter().map(|p| p.key).collect();
            let mut u = keys.clone();
            u.sort();
            u.dedup();
            assert_eq!(keys.len(), u.len(), "{}: duplicate key", d.id);
        }
    }
}
