//! YT Strategies · Gann HiLo: the team's AmiBroker AFLs in "Old Topics/Gann HiLo" that trade individual stocks on
//! daily bars, long and short by market regime. Coded from the AFLs as written, with the AFLs' own default
//! settings; there is no book PDF. Each strategy runs on one stock at a time; the team ran them over a cash-stock
//! universe ("C&P - 0.65" workbooks, 20 open positions, 0.65% cost); portfolio limits and sizing are not modelled.
//!
//! Conventions as in `ytsw.rs` / `ytwh.rs` and the stock template in `ytbb.rs` (Buy / Sell / Short / Cover arrays
//! read by AmiBroker's backtester without ExRem, UC / LC bars delaying exits, turnover = close x volume, dead-stock
//! toggle default NO). In addition:
//! - Gann HiLo: Hld = 1 when C > Ref(MA(H, n), -1), -1 when C < Ref(MA(L, n), -1), else 0; Hlv = the last non-zero
//!   Hld; HiLo = MA(H, n) while Hlv = -1, otherwise MA(L, n);
//! - weekly / monthly values reach the daily bars with TimeFrameExpand(expandLast) (`ytsw::expand_last`);
//! - inside the loops a Null value is -1e10 (so `C > HiLo` holds and `C < HiLo` fails before HiLo exists).
#![allow(dead_code)]
use crate::books::ytbb::{amibt, dead_sigs, turnover, Sig4};
use crate::books::ytsw::{calc_point, compress, expand_last, true_range, Bars};
use crate::books::ytwh::loop_val;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_gann";
pub const R1_GANN: &str = "ytgann_regim1_gann_filter";
pub const R1_ST: &str = "ytgann_regim1_st_filter";
pub const R2_GANN: &str = "ytgann_regim2_gann_filter";
pub const R2_ST_SQUARE: &str = "ytgann_regim2_st_square_off";
pub const R2_ST_CONT: &str = "ytgann_regim2_st_continue";
pub const IDS: [&str; 5] = [R1_GANN, R1_ST, R2_GANN, R2_ST_SQUARE, R2_ST_CONT];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Gann HiLo", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(R1_GANN, "Gann HiLo, Regime 1, Monthly Gann HiLo Filter", &[
            "F|Daily Gann HiLo (period 3); filter: Gann HiLo (period 3) of the stock's own monthly bars against the monthly close, expanded to daily bars",
            "E|Buy at the close when flat, the close is above the daily HiLo, the monthly close is above the monthly HiLo, high not equal to close, close above the penny price (0)",
            "X|Sell when the close is below the daily HiLo or the monthly close below the monthly HiLo (an LC bar waits for the next non-LC bar)",
            "E|Short at the close when flat, the close is below the daily HiLo and the monthly close below the monthly HiLo, low not equal to close",
            "X|Cover when the close is above the daily HiLo or the monthly close above the monthly HiLo (a UC bar waits)",
        ], "GANN HiLo EntryorExit with REGIM1 with Gann HiLo Monthly Filter.afl (REGIME 1 GANN HILO-Filter Optim), coded as the AFL is written (its loop from bar = HiLo period). Runs on one stock at a time; the team ran it over a cash-stock universe (C&P - 0.65 workbooks, 20 open positions); portfolio limits not modelled. A sell and a short can fall on the same bar (exit, then reverse). Turnover = close x volume (team stored it in OpenInt), off by default; dead-stock backtest default NO."),
        def(R1_ST, "Gann HiLo, Regime 1, Monthly ST Filter", &[
            "F|Daily Gann HiLo (period 3); filter: Supertrend of the stock's own monthly bars (period 5, factor 1, mid point; ATR = mean of ATR(1)), expanded to daily bars",
            "E|Buy at the close when flat, the ST has produced a line at least once, the close is above the daily HiLo, the monthly ST is up, high not equal to close",
            "X|Sell when the close is below the daily HiLo or the monthly ST is down (an LC bar waits)",
            "E|Short at the close when flat, the close is below the daily HiLo and the monthly ST is down, low not equal to close",
            "X|Cover when the close is above the daily HiLo or the monthly ST is up (a UC bar waits)",
        ], "GANN HiLo EntryorExit with REGIM1 with ST Filter.afl (REGIME 1 ST Filter Optim), coded as the AFL is written (its header says long only; the code also shorts, as coded). Runs on one stock at a time; the team ran it over a cash-stock universe (C&P - 0.65 workbooks, 20 open positions); portfolio limits not modelled. Turnover = close x volume, off by default; dead-stock backtest default NO."),
        def(R2_GANN, "Gann HiLo, Regime 2, Monthly Gann HiLo Filter", &[
            "F|Daily Gann HiLo (period 3); monthly Gann HiLo filter (period 3) on the stock's own monthly bars",
            "E|Bull (monthly close above the monthly HiLo): buy when the close is above the daily HiLo; sell when below it or the filter turns bearish",
            "E|Bear (monthly close below the monthly HiLo): buy when the close is above the daily HiLo, short when it is below (each reverses the other)",
            "X|Bear trades: sell when the close is below the daily HiLo or the filter turns bullish; cover when the close is above the daily HiLo or the filter turns bullish; a bull long carries into the bear book (and back) without a trade",
        ], "GANN HiLo EntryorExit with REGIM2 and Gann HiLo Monthly Filter.afl, coded as the AFL is written: two position states (bull long-only and bear long-short) write one set of Buy / Sell / Short / Cover arrays, which AmiBroker's backtester reads as one position (so a sell from either state closes it, a buy while long is ignored). Filter timeframe list MONTHLY | WEEKLY. Runs on one stock at a time; the team ran it over a cash-stock universe (C&P - 0.65 workbooks, 20 open positions); portfolio limits not modelled. Quirks kept: some bear-book exits do not reset the skip-sell / skip-cover flags. Turnover = close x volume, off by default; dead-stock backtest default NO."),
        def(R2_ST_SQUARE, "Gann HiLo, Regime 2, ST Filter (Square Off in Transition)", &[
            "F|Daily Gann HiLo (period 3); Supertrend of the stock's own bars (default daily, period 5, factor 1, on the close)",
            "E|ST up: buy when the close is above the daily HiLo (only when neither book holds a position); sell when below it or the ST turns down",
            "E|ST down: buy when the close is above the daily HiLo, short when below it (each reverses the other)",
            "X|When the ST turns down a bull long passes to the bear book; when the ST turns up a bear long is sold (unless LC) and a short covered (unless UC)",
        ], "GANN HiLo EntryExit with REGIM2 and ST Filter(Square Off and Re-enter in Transition Phase).afl, coded as the AFL is written. Quirks kept: the calculation point list items (MID POINT(H+L/2) ...) are compared with \"MID_POINT(H+L/2)\" / \"PIVOT_(H+L+C/3)\", so the ST is always on the close; the long-short ST uses the long-only ST's period and factor, so both STs are the same. Runs on one stock at a time; the team ran it over a cash-stock universe (C&P - 0.65 workbooks, 20 open positions); portfolio limits not modelled. Turnover = close x volume, off by default; dead-stock backtest default NO."),
        def(R2_ST_CONT, "Gann HiLo, Regime 2, ST Filter (Continue in Transition)", &[
            "F|Daily Gann HiLo (period 3); Supertrend of the stock's own bars (default daily, period 5, factor 1, on the close)",
            "E|ST up: buy when the close is above the daily HiLo (only when neither book holds a position); sell when below it or the ST turns down",
            "E|ST down: buy when the close is above the daily HiLo, short when below it (each reverses the other)",
            "X|When the ST turns up a bear long above the HiLo passes to the bull book (no trade); below the HiLo it is sold (unless LC); a short is covered (unless UC)",
        ], "GANN HiLo EntryorExit with REGIM2 and ST Filter(Continue Trades in Transition Phase).afl, coded as the AFL is written; same quirks as the square-off version (ST always on the close; one ST period and factor for both books). Runs on one stock at a time; the team ran it over a cash-stock universe (C&P - 0.65 workbooks, 20 open positions); portfolio limits not modelled. Turnover = close x volume, off by default; dead-stock backtest default NO."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![p("prd", "Gann HiLo period", 3.0, 1.0, 100.0, 1.0)];
    match id {
        R1_GANN => v.extend([p("flt_tf", "Filter timeframe (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0), p("flt_prd", "Filter Gann HiLo period", 3.0, 1.0, 100.0, 1.0)]),
        R2_GANN => v.extend([p("flt_tf", "Filter timeframe (1 monthly, 2 weekly)", 1.0, 1.0, 2.0, 1.0), p("flt_prd", "Filter Gann HiLo period", 3.0, 1.0, 100.0, 1.0)]),
        R1_ST => v.extend([
            p("flt_tf", "ST timeframe (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0),
            p("st_period", "ST period", 5.0, 1.0, 20.0, 1.0),
            p("st_factor", "ST factor", 1.0, 0.5, 20.0, 0.5),
            p("st_point", "ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
        ]),
        R2_ST_SQUARE | R2_ST_CONT => v.extend([
            p("flt_tf", "ST timeframe (1 daily, 2 weekly, 3 monthly)", 1.0, 1.0, 3.0, 1.0),
            p("st_period", "ST period", 5.0, 1.0, 20.0, 1.0),
            p("st_factor", "ST factor", 1.0, 1.0, 20.0, 0.5),
        ]),
        _ => return None,
    }
    v.extend([
        p("penny", "Penny stock price", 0.0, 0.0, 100.0, 1.0),
        p("turnover", "Turnover filter (0 no, 1 yes: close x volume)", 0.0, 0.0, 1.0, 1.0),
        p("dead_bt", "Dead stock backtest (0 no, 1 yes)", 0.0, 0.0, 1.0, 1.0),
    ]);
    Some(v)
}

// ---------------------------------------------------------------- indicators

fn ma(v: &[f64], n: usize) -> Vec<Option<f64>> {
    let n = n.max(1);
    (0..v.len()).map(|i| if i + 1 < n { None } else { Some(v[i + 1 - n..=i].iter().sum::<f64>() / n as f64) }).collect()
}

/// AFL Gann HiLo (see the header).
pub(crate) fn gann_hilo(h: &[f64], l: &[f64], c: &[f64], n: usize) -> Vec<Option<f64>> {
    let (mh, ml) = (ma(h, n), ma(l, n));
    let mut hlv: Option<i8> = None;
    (0..c.len())
        .map(|i| {
            let (rh, rl) = if i == 0 { (None, None) } else { (mh[i - 1], ml[i - 1]) };
            let hld = if rh.map_or(false, |v| c[i] > v) { 1 } else if rl.map_or(false, |v| c[i] < v) { -1 } else { 0 };
            if hld != 0 {
                hlv = Some(hld);
            }
            if hlv == Some(-1) { mh[i] } else { ml[i] }
        })
        .collect()
}

/// The AFLs' ST loop (ATR = mean of the last `period` ATR(1), Null until it exists): trend and the line drawn
/// (lower band in an up trend, upper band in a down trend; Null while the ATR is Null).
fn st_lines(calc: &[f64], b: &Bars, period: usize, factor: f64) -> (Vec<i8>, Vec<Option<f64>>) {
    let (h, l, c) = (&b.h, &b.l, &b.c);
    let n = c.len();
    let trr = true_range(h, l, c);
    let p = period.max(1);
    let satr: Vec<Option<f64>> = (0..n)
        .map(|i| if i + 1 < p { None } else { trr[i + 1 - p..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)).map(|s| s / p as f64) })
        .collect();
    let mut up: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| calc[i] + factor * s)).collect();
    let mut dn: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| calc[i] - factor * s)).collect();
    let mut tr = vec![1i8; n];
    let mut line = vec![None; n];
    if n > 0 {
        line[0] = dn[0];
    }
    for i in 1..n {
        tr[i] = match (up[i - 1], dn[i - 1]) {
            (Some(u), _) if c[i] > u => 1,
            (_, Some(d)) if c[i] < d => -1,
            _ => tr[i - 1],
        };
        if let (true, Some(a), Some(b)) = (tr[i] > 0, dn[i], dn[i - 1]) {
            if a < b {
                dn[i] = Some(b);
            }
        }
        if let (true, Some(a), Some(b)) = (tr[i] < 0, up[i], up[i - 1]) {
            if a > b {
                up[i] = Some(b);
            }
        }
        if tr[i] < 0 && tr[i - 1] > 0 {
            up[i] = satr[i].map(|s| calc[i] + factor * s);
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = satr[i].map(|s| calc[i] - factor * s);
        }
        line[i] = if tr[i] == 1 { dn[i] } else { up[i] };
    }
    (tr, line)
}

/// ST trend of the stock's own bars on a timeframe, expanded, and the "abc" flag: set from the first daily bar whose
/// expanded ST line is above 0.
fn st_expanded(x: &Ctx, tf: char, point: i64, period: usize, factor: f64) -> (Vec<i8>, Vec<bool>) {
    let b = compress(x, tf);
    let (tr, line) = st_lines(&calc_point(&b, point), &b, period, factor);
    let (tr, line) = (expand_last(&tr, &b), expand_last(&line, &b));
    let mut on = false;
    let abc = line.iter().map(|v| { on |= v.map_or(false, |v| v > 0.0); on }).collect();
    (tr, abc)
}

/// Gann HiLo of the stock's own bars on a timeframe, expanded with that timeframe's close (MyClose, F_HiLo).
fn gann_filter(x: &Ctx, tf: char, n: usize) -> (Vec<f64>, Vec<Option<f64>>) {
    let b = compress(x, tf);
    let fh = gann_hilo(&b.h, &b.l, &b.c, n);
    (expand_last(&b.c, &b), expand_last(&fh, &b))
}

// ---------------------------------------------------------------- the loops

struct Common {
    hilo: Vec<f64>,
    bcf: Vec<bool>,
    scf: Vec<bool>,
    uc: Vec<bool>,
    lc: Vec<bool>,
    dss: Vec<bool>,
}

fn common(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Common {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    let hilo: Vec<f64> = gann_hilo(h, l, c, g("prd").round().max(1.0) as usize).into_iter().map(loop_val).collect();
    let (dbs, dss) = dead_sigs(n, g("dead_bt").round() as i64 == 1, false);
    let turn = turnover(x, g("turnover").round() as i64 == 1);
    let penny = g("penny");
    let uc: Vec<bool> = (0..n).map(|i| h[i] == c[i]).collect();
    let lc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let bcf = (0..n).map(|i| !uc[i] && turn[i] && c[i] > penny && dbs[i]).collect();
    let scf = (0..n).map(|i| !lc[i] && turn[i] && c[i] > penny && dbs[i]).collect();
    Common { hilo, bcf, scf, uc, lc, dss }
}

/// Regime 1: one position, long when the filter is bullish, short when bearish. `bull[i]` / `bear[i]` are the
/// filter's entry states (also its exit triggers for the other side); `abc` gates buys (ST version).
fn regim1(x: &Ctx, k: &Common, start: usize, bull: &[bool], bear: &[bool], abc: &[bool]) -> Sig4 {
    let c = &x.c;
    let n = c.len();
    let mut s = Sig4::new(n);
    let (mut pos, mut skip_s, mut skip_c) = (0i8, false, false);
    for i in start.max(1)..n {
        let hl = k.hilo[i];
        if pos == 0 && abc[i] && c[i] > hl && bull[i] && k.bcf[i] {
            s.buy[i] = true;
            pos = 1;
        } else if pos > 0 && !skip_s {
            if c[i] < hl || bear[i] || k.dss[i] {
                if !k.lc[i] || k.dss[i] {
                    s.sell[i] = Some((c[i], if c[i] < hl { "Below HiLo" } else { "Filter bearish" }));
                    pos = 0;
                } else {
                    skip_s = true;
                }
            }
        } else if skip_s && (!k.lc[i] || k.dss[i]) {
            s.sell[i] = Some((c[i], "Sell after LC bar"));
            pos = 0;
            skip_s = false;
        }
        if pos == 0 && c[i] < hl && bear[i] && k.scf[i] {
            s.short[i] = true;
            pos = -1;
        } else if pos < 0 && !skip_c {
            if c[i] > hl || bull[i] || k.dss[i] {
                if !k.uc[i] || k.dss[i] {
                    s.cover[i] = Some((c[i], if c[i] > hl { "Above HiLo" } else { "Filter bullish" }));
                    pos = 0;
                } else {
                    skip_c = true;
                }
            }
        } else if skip_c && (!k.uc[i] || k.dss[i]) {
            s.cover[i] = Some((c[i], "Cover after UC bar"));
            pos = 0;
            skip_c = false;
        }
    }
    s
}

/// Regime 2 with the monthly Gann HiLo filter, as written.
fn regim2_gann(x: &Ctx, k: &Common, mc: &[f64], fh: &[f64]) -> Sig4 {
    let c = &x.c;
    let n = c.len();
    let mut s = Sig4::new(n);
    let (mut pos, mut pos1) = (0i8, 0i8);
    let (mut skip_s, mut skip_s1, mut skip_c1) = (false, false, false);
    for i in 1..n {
        let hl = k.hilo[i];
        let (fbull, fbear) = (mc[i] > fh[i], mc[i] < fh[i]);
        if pos > 0 && fbear {
            pos = 0;
            pos1 = 1;
        } else if pos1 < 0 && fbull && !k.uc[i] {
            s.cover[i] = Some((c[i], "Filter bullish"));
            pos1 = 0;
        } else if pos1 > 0 && fbull {
            pos1 = 0;
            pos = 1;
        }
        // bull book (long only)
        if pos == 0 && c[i] > hl && fbull && k.bcf[i] {
            s.buy[i] = true;
            pos = 1;
        } else if pos > 0 && !skip_s {
            if c[i] < hl || fbear || k.dss[i] {
                if !k.lc[i] {
                    s.sell[i] = Some((c[i], if c[i] < hl { "Below HiLo" } else { "Filter bearish" }));
                    pos = 0;
                } else {
                    skip_s = true;
                }
            }
        } else if skip_s && (!k.lc[i] || k.dss[i]) {
            s.sell[i] = Some((c[i], "Sell after LC bar"));
            pos = 0;
            skip_s = false;
        }
        // bear book (long and short)
        if pos1 <= 0 && c[i] > hl && fbear && k.bcf[i] {
            s.buy[i] = true;
            if pos1 == -1 {
                s.cover[i] = Some((c[i], "Reversed by buy"));
            }
            pos1 = 1;
        } else if pos1 >= 0 && c[i] < hl && fbear && k.scf[i] {
            s.short[i] = true;
            if pos1 == 1 {
                s.sell[i] = Some((c[i], "Reversed by short"));
            }
            pos1 = -1;
        } else if pos1 > 0 && !skip_s1 {
            if c[i] < hl || fbull || k.dss[i] {
                if !k.lc[i] {
                    s.sell[i] = Some((c[i], if c[i] < hl { "Below HiLo" } else { "Filter bullish" }));
                    pos1 = 0;
                } else {
                    skip_s1 = true;
                }
            }
        } else if skip_s1 && (!k.lc[i] || k.dss[i]) {
            s.sell[i] = Some((c[i], "Sell after LC bar"));
            pos1 = 0;
            skip_s1 = false;
        } else if pos1 < 0 && !skip_c1 {
            if c[i] > hl || fbull || k.dss[i] {
                if !k.uc[i] {
                    s.cover[i] = Some((c[i], if c[i] > hl { "Above HiLo" } else { "Filter bullish" }));
                    pos1 = 0;
                } else {
                    skip_c1 = true;
                }
            }
        } else if skip_c1 && (!k.uc[i] || k.dss[i]) {
            s.cover[i] = Some((c[i], "Cover after UC bar"));
            pos1 = 0;
            skip_c1 = false;
        }
    }
    s
}

/// Regime 2 with the ST filter, square-off (`cont` false) or continue (`cont` true) in the transition.
fn regim2_st(x: &Ctx, k: &Common, st: &[i8], abc: &[bool], cont: bool) -> Sig4 {
    let c = &x.c;
    let n = c.len();
    let mut s = Sig4::new(n);
    let (mut pos, mut pos1) = (0i8, 0i8);
    let (mut skip_s, mut skip_s1, mut skip_c1) = (false, false, false);
    for i in 1..n {
        let hl = k.hilo[i];
        let (up, down) = (st[i] == 1, st[i] == -1);
        if pos > 0 && down && st[i - 1] == 1 {
            pos = 0;
            pos1 = 1;
        } else if cont && pos1 > 0 && up && st[i - 1] == -1 && c[i] > hl {
            pos = 1;
            pos1 = 0;
        } else if pos1 > 0 && up && (!cont || c[i] < hl) && !k.lc[i] {
            s.sell[i] = Some((c[i], "ST up"));
            pos1 = 0;
            skip_s1 = false;
        } else if pos1 < 0 && up && !k.uc[i] {
            s.cover[i] = Some((c[i], "ST up"));
            pos1 = 0;
            skip_c1 = false;
        }
        // bull book
        if pos == 0 && pos1 == 0 && abc[i] && c[i] > hl && up && k.bcf[i] {
            s.buy[i] = true;
            pos = 1;
        } else if pos > 0 && !skip_s {
            if c[i] < hl || down || k.dss[i] {
                if !k.lc[i] || k.dss[i] {
                    s.sell[i] = Some((c[i], if c[i] < hl { "Below HiLo" } else { "ST down" }));
                    pos = 0;
                } else {
                    skip_s = true;
                }
            }
        } else if skip_s && (!k.lc[i] || k.dss[i]) {
            s.sell[i] = Some((c[i], "Sell after LC bar"));
            pos = 0;
            skip_s = false;
        }
        // bear book
        if pos1 <= 0 && abc[i] && c[i] > hl && down && k.bcf[i] {
            s.buy[i] = true;
            if pos1 == -1 {
                s.cover[i] = Some((c[i], "Reversed by buy"));
                skip_c1 = false;
            }
            pos1 = 1;
        } else if pos1 >= 0 && c[i] < hl && down && k.scf[i] {
            s.short[i] = true;
            if pos1 == 1 {
                s.sell[i] = Some((c[i], "Reversed by short"));
                skip_s1 = false;
            }
            pos1 = -1;
        } else if pos1 > 0 && !skip_s1 {
            if c[i] < hl || up || k.dss[i] {
                if !k.lc[i] || k.dss[i] {
                    s.sell[i] = Some((c[i], if c[i] < hl { "Below HiLo" } else { "ST up" }));
                    pos1 = 0;
                } else {
                    skip_s1 = true;
                }
            }
        } else if skip_s1 && (!k.lc[i] || k.dss[i]) {
            s.sell[i] = Some((c[i], "Sell after LC bar"));
            pos1 = 0;
            skip_s1 = false;
        } else if pos1 < 0 && !skip_c1 {
            if c[i] > hl || up || k.dss[i] {
                if !k.uc[i] || k.dss[i] {
                    s.cover[i] = Some((c[i], if c[i] > hl { "Above HiLo" } else { "ST up" }));
                    pos1 = 0;
                } else {
                    skip_c1 = true;
                }
            }
        } else if skip_c1 && (!k.uc[i] || k.dss[i]) {
            s.cover[i] = Some((c[i], "Cover after UC bar"));
            pos1 = 0;
            skip_c1 = false;
        }
    }
    s
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let k = common(x, &g);
    let s = match id {
        R1_GANN | R2_GANN => {
            let tf = match (id, gi("flt_tf")) { (_, 2) => 'W', (R1_GANN, 3) => 'D', _ => 'M' };
            let (mc, fh) = gann_filter(x, tf, gu("flt_prd"));
            let fh: Vec<f64> = fh.into_iter().map(loop_val).collect();
            if id == R1_GANN {
                let bull: Vec<bool> = (0..n).map(|i| mc[i] > fh[i]).collect();
                let bear: Vec<bool> = (0..n).map(|i| mc[i] < fh[i]).collect();
                regim1(x, &k, gu("prd"), &bull, &bear, &vec![true; n])
            } else {
                regim2_gann(x, &k, &mc, &fh)
            }
        }
        R1_ST => {
            let tf = match gi("flt_tf") { 2 => 'W', 3 => 'D', _ => 'M' };
            let (st, abc) = st_expanded(x, tf, gi("st_point"), gu("st_period"), g("st_factor"));
            let bull: Vec<bool> = st.iter().map(|&t| t == 1).collect();
            let bear: Vec<bool> = st.iter().map(|&t| t == -1).collect();
            regim1(x, &k, gu("prd"), &bull, &bear, &abc)
        }
        R2_ST_SQUARE | R2_ST_CONT => {
            let tf = match gi("flt_tf") { 2 => 'W', 3 => 'M', _ => 'D' };
            // the AFL's calculation point never matches its list, so the ST is on the close (calc_point 2)
            let (st, abc) = st_expanded(x, tf, 2, gu("st_period"), g("st_factor"));
            regim2_st(x, &k, &st, &abc, id == R2_ST_CONT)
        }
        _ => return None,
    };
    Some(amibt(x, &s, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        R1_GANN => "ytgann_regim1_gann_filter",
        R1_ST => "ytgann_regim1_st_filter",
        R2_GANN => "ytgann_regim2_gann_filter",
        R2_ST_SQUARE => "ytgann_regim2_st_square_off",
        R2_ST_CONT => "ytgann_regim2_st_continue",
        _ => return None,
    }, name, "Gann HiLo"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytbb::stocktest::{rows, smoke};
    use crate::books::ytwh::testbars::ps;
    use crate::Row;

    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<(f64, usize, usize, f64, bool)> {
        let x = Ctx::new(r, r, &[]);
        run(id, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.dir, t.legs[0].i, t.exit_i, t.exit_px, t.open)).collect()
    }
    fn bar(c: f64) -> (f64, f64, f64, f64) {
        (c, c + 1.0, c - 1.0, c)
    }

    #[test]
    fn hilo_switches_bands() {
        // n = 2: rising closes keep HiLo = MA(L); a close below the previous MA(L) switches to MA(H)
        let h = [11.0, 12.0, 13.0, 14.0, 10.0];
        let l = [9.0, 10.0, 11.0, 12.0, 8.0];
        let c = [10.0, 11.0, 12.0, 13.0, 9.0];
        let v = gann_hilo(&h, &l, &c, 2);
        assert_eq!(v[0], None);
        assert_eq!(v[3], Some(11.5));
        assert_eq!(v[4], Some(12.0)); // 9 < MA(L)[3] = 11.5 -> Hlv = -1 -> MA(H) = (14 + 10) / 2
    }

    #[test]
    fn regim1_daily_filter_goes_long_then_short() {
        // daily filter = the same HiLo on the same bars: up-move buys, a drop sells and shorts on the same bar
        let v: Vec<_> = [100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 95.0, 94.0, 93.0, 104.0].iter().map(|&c| bar(c)).collect();
        let t = go(R1_GANN, &rows(&v), &[("flt_tf", 3.0)]);
        assert_eq!(t[0].0, 1.0);
        assert_eq!(t[0].1, 3);
        assert_eq!((t[0].2, t[0].3), (6, 95.0));
        assert_eq!((t[1].0, t[1].1, t[1].2), (-1.0, 6, 9));
    }

    #[test]
    fn regim2_st_books_share_one_position() {
        let v: Vec<_> = (0..60).map(|k| bar(100.0 + 10.0 * ((k as f64) / 6.0).sin())).collect();
        for id in [R2_ST_SQUARE, R2_ST_CONT, R2_GANN] {
            let t = go(id, &rows(&v), &[]);
            for w in t.windows(2) {
                assert!(w[1].1 >= w[0].2, "{id}");
            }
        }
    }

    #[test]
    fn defaults_trade_on_a_wave() {
        let r = crate::books::ytbb::stocktest::with_volume(crate::books::ytwh::testbars::wave(420), &[1.0e5, 1.6e5, 0.9e5, 2.2e5, 1.2e5]);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            eprintln!("{id}: {} trades", t.len());
            assert!(!t.is_empty(), "{id}");
        }
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS, &[("flt_tf", 2.0), ("dead_bt", 1.0), ("turnover", 1.0)]);
        smoke(run, &[R1_GANN, R1_ST, R2_ST_CONT], &[("flt_tf", 3.0), ("prd", 5.0)]);
    }
}
