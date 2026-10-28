//! YT Strategies · Bollinger Band: the team's AmiBroker AFLs in "Old Topics/Bollinger band" that trade individual
//! stocks on daily bars. Coded from the AFLs as written, with the AFLs' own default settings; there is no book PDF.
//! Each strategy runs on one stock at a time; the team ran them over cash-stock universes (NSE 500 and F&O cash
//! stocks, "C&P" workbooks) with up to 20 open positions; the portfolio limits and position sizing are not modelled.
//!
//! The AmiBroker conventions are those in `ytsw.rs` / `ytwh.rs`; in addition, for these stock AFLs:
//! - `BBandTop(C, n, k)` / `BBandBot` = MA(C, n) +/- k x the population standard deviation of the last n closes
//!   (Null until n bars exist; the AFL's "SD %" parameter is that multiplier k);
//! - the loop AFLs write Buy / Sell / Short / Cover arrays that AmiBroker's backtester then reads without ExRem: an
//!   exit signal closes the open position (at the AFL's price, kept inside the bar's range), an opposite entry
//!   reverses it (ReverseSignalForcesExit), an entry while already in that position is ignored, and exits come
//!   before entries on the same bar (`amibt`);
//! - the team's stock template: turnover filter `OpenInt > Volsig(year)` (the team stored turnover in OpenInt; here
//!   close x volume is the turnover), upper circuit `Prec(O,2) == Prec(C,2) == Prec(H,2)` (Prec truncates to 2
//!   decimals), `UC = H == C`, `LC = L == C` (a sell on an LC bar waits for the next non-LC bar), penny price
//!   `C > PennyFlt`, and the dead-stock backtest toggle (default NO);
//! - SetForeign(index) filters use NIFTY 50 (the market benchmark, `BENCH.*`) in place of the AFL's NSENIFTY /
//!   NSE500 list, aligned to the stock's dates and carried forward over missing days.
//!
//! Not coded: "Bollinger Band entry + ATHC % range + TSL v2", "Bollinger Band entry + fresh ATHC Exit by %swing or
//! BB" and "Fresh ATHC en-ex" are built on the missing parmar include library (Defaults.afl, UtilityExtension.afl:
//! D_initTrades, D_getDefaultBuyVar, D_ARR_FATHC ...), so their rules cannot be resolved; "BB Master/AFL/BB 100 MA
//! UB 3% LB1%_Index Filter.afl" is a price-plot stub with no trading rules.
#![allow(dead_code)]
use crate::books::ytsw::{backtest, close_swing, sma_filter, st_filter, swing, trade, Sig};
use crate::books::ytwh::{bound, loop_val, prec2};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::Row;

pub const BOOK: &str = "yt_bb";
pub const PLAIN: &str = "ytbb_plain";
pub const INDEX: &str = "ytbb_index_filter";
pub const V1: &str = "ytbb_index_filter_v1";
pub const V2: &str = "ytbb_index_filter_v2";
pub const MASTER: &str = "ytbb_master";
pub const IDS: [&str; 5] = [PLAIN, INDEX, V1, V2, MASTER];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Bollinger Band", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const UNIVERSE: &str = "Runs on one stock at a time; the team ran it over NSE 500 / F&O cash stocks (C&P workbooks, 20 open positions, % of equity sizing); portfolio limits not modelled.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(PLAIN, "Bollinger Band Breakout (100, 3 SD / 1 SD)", &[
            "F|Bands on the close: upper = 100-day MA + 3 standard deviations, lower = 100-day MA + 1 SD below (BBandTop / BBandBot)",
            "F|Buy filter: turnover (off by default), not an upper-circuit bar, high not equal to close, close above the penny price (0), not the first bar",
            "E|Buy at the close when the close is above the upper band",
            "X|Sell at the close when the close is below the lower band, unless low = close (LC bar)",
        ], "BB 100 MA UB 3% LB1%.afl, coded as the AFL is written (Buy / Sell arrays with ExRem, trades at the close). Runs on one stock at a time; the team ran it over NSE 500 / F&O cash stocks (C&P workbooks, 20 open positions, % of equity sizing); portfolio limits not modelled. The AFL's hard stop-loss parameter is declared but never used. Turnover filter: close x volume against the AFL's yearly Volsig table (the team stored turnover in OpenInt); off by default as in the AFL."),
        def(INDEX, "BB Breakout above All-Time-High Close, % Swing Exit (Index Filter)", &[
            "F|Bands: 100-day MA + 3 SD (upper), 100-day MA - 1 SD (lower); ATHC = highest close so far; 21% close swing",
            "F|Optional market filter (default NONE): NIFTY 50 ST (monthly, 20, 1, mid point), SMA (200) or Swing% (bull 44 / bear 23) in a Bull or Bear setting",
            "E|Buy at the close when flat, the close is above the previous bar's ATHC and the upper band, the buy filter passes (turnover, circuit, H = C, penny, dead-stock look-ahead) and the market filter allows it",
            "X|Sell at the close when the 21% swing turns down, the close is below the lower band, at or below the hard stop (0 = off), or the next 30 bars are identical (dead stock); on an LC bar the sell waits for the next non-LC bar",
        ], "BB 100 MA UB 3% LB1%_Index Filter.afl, coded as the AFL is written (its loop; no ExRem). Runs on one stock at a time; the team ran it over NSE 500 / F&O cash stocks (C&P workbooks, 20 open positions); portfolio limits not modelled. Quirks kept: no entry before the 100-bar band exists (the team's trade lists never buy earlier), while a Null lower band never triggers an exit; the sell test `athcbuysig[i]=0` is an assignment (value 0), so the ATHC exit never fires; the dead-stock test uses Ref(.., +1..+30), i.e. it looks 30 bars ahead; the AFL's BB period default 100 lies outside its Param range (the default is used). The market filter uses NIFTY 50 (the market benchmark) in place of the AFL's NSENIFTY / NSE500; with it on, a buy needs FLTShort, which in the Bull setting means the index ST / Swing% is UP and in the Bear setting DOWN; with SMA it means the index close is below the previous period's average. The Swing% filter is taken on daily index closes (the AFL expands it with inDaily). Turnover = close x volume (team stored it in OpenInt), off by default."),
        def(V1, "BB Breakout, % Swing Exit, RSI / ADX / ROC Options (Index Filter V1)", &[
            "F|Bands: 100-day MA + 3 SD (upper); lower band toggle (default OFF: the lower band is 1); 21% close swing; optional swing-ATHC flag (default OFF)",
            "F|Optional filters (all off by default): RSI(14) > 30, ADX(8) > 20, ROC(1) above 1 or below -1; market filter as in the Index Filter AFL (default NONE)",
            "E|Buy at the close when flat, the close is above the upper band, the buy filter passes and the swing-ATHC flag is set",
            "X|Sell at the close when the 21% swing turns down, the close is below the lower band, at or below the hard stop, or the next 30 bars are identical",
        ], "BB 100 MA UB 3% LB1%_Index Filter_V1.afl, coded as the AFL is written. With the AFL's defaults the swing-ATHC option is OFF, so AthcBuySig stays 0 and the AFL never buys (set athc = 1 to trade). Runs on one stock at a time; the team ran it over NSE 500 / F&O cash stocks (20 open positions); portfolio limits not modelled. The optional PositionScore rank type (default Off) only orders stocks in a portfolio and is not offered. Quirks kept: the first sell test reads the dead-stock toggle (a number, 1 by default) as `Deadstock[i]`, so an LC bar does not delay the sell unless the dead-stock backtest is set to YES; `athcbuysig[i]=0` is an assignment (never true); the dead-stock test looks 30 bars ahead; no entry before the band exists (as in the team's trade lists). Market filter: NIFTY 50 in place of the AFL's index. ADX is Wilder's. Turnover = close x volume, off by default."),
        def(V2, "BB Breakout, Near-ATHC Flag and Trailing Stop (Index Filter V2)", &[
            "F|As the Master AFL; the swing-ATHC flag (default OFF) is also set while the close is within 21% below the highest close",
            "F|Optional trailing stop (default 0 = off): highest high (or close) since entry less the trail %",
            "E|Buy at the close when flat, the close is above the upper band, the buy filter passes and the swing-ATHC flag is set",
            "X|Sell at the close when the 21% swing turns down, the close is below the lower band, at or below the hard or trailing stop, or the next 30 bars are identical",
        ], "BB 100 MA UB 3% LB1%_Index Filter_V2.afl, coded as the AFL is written. With the AFL's defaults the swing-ATHC option is OFF, so AthcBuySig stays 0 and the AFL never buys (set athc = 1 to trade). Runs on one stock at a time; the team ran it over cash stocks (Optim_Cash workbooks, 20 open positions); portfolio limits not modelled. The trailing stop is checked against the previous bar's level and raised only on bars without an exit signal, as the loop does; the AFL tests TS == \"Cl\" against the list item \"CL\" (taken as the close trail). RSI / ADX / ROC are evaluated on daily bars (the AFL computes the conditions inside TimeFrameSet without expanding them). Market filter: NIFTY 50 in place of the AFL's index. Quirks as in the Master AFL. Turnover = close x volume, off by default."),
        def(MASTER, "BB Master (Index Filter)", &[
            "F|Bands: 100-day MA + 3 SD (upper); lower band toggle (default OFF: the lower band is 1); 21% close swing; optional swing-ATHC flag (default OFF)",
            "F|Optional filters (all off by default): RSI(14) > 30, ADX(8) > 20, ROC(1) above 1 or below -1; market filter as in the Index Filter AFL (default NONE)",
            "E|Buy at the close when flat, the close is above the upper band, the buy filter passes and the swing-ATHC flag is set",
            "X|Sell at the close when the 21% swing turns down, the close is below the lower band, at or below the hard stop, or the next 30 bars are identical; on an LC bar the sell waits",
        ], "BB_Master_IndexFilter.afl, coded as the AFL is written. With the AFL's defaults the swing-ATHC option is OFF, so AthcBuySig stays 0 and the AFL never buys (set athc = 1 to trade). Runs on one stock at a time; the team ran it over cash stocks (Optim_Cash workbooks, 20 open positions); portfolio limits not modelled. RSI / ADX / ROC are evaluated on daily bars (the AFL computes the conditions inside TimeFrameSet(monthly by default) without expanding them). Quirks kept: `athcbuysig[i]=0` is an assignment (never true); the dead-stock test looks 30 bars ahead; no entry before the band exists (as in the team's trade lists). Market filter: NIFTY 50 in place of the AFL's index. Turnover = close x volume, off by default."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![
        p("bb_period", "Band average period", 100.0, 1.0, 1000.0, 1.0),
        p("ub_sd", "Upper band SD multiplier", 3.0, 0.25, 30.0, 0.25),
        p("lb_sd", "Lower band SD multiplier", 1.0, 0.5, 10.0, 0.5),
        p("penny", "Penny stock price", 0.0, 0.0, 100.0, 1.0),
        p("turnover", "Turnover filter (0 no, 1 yes: close x volume)", 0.0, 0.0, 1.0, 1.0),
        p("dead_bt", "Dead stock backtest (0 no, 1 yes)", 0.0, 0.0, 1.0, 1.0),
    ];
    match id {
        PLAIN => {}
        INDEX | V1 | V2 | MASTER => {
            v.push(p("exit_pct", "Swing exit %", 21.0, 2.0, 25.0, 1.0));
            v.push(p("hard_stop", "Hard stop-loss % (0 off)", 0.0, 0.0, 50.0, 1.0));
            if id != INDEX {
                v.push(p("lb_on", "Lower band (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0));
                v.push(p("athc", "Swing ATHC flag (0 off: no buys, 1 ATHC)", 0.0, 0.0, 1.0, 1.0));
                v.extend([
                    p("rsi_on", "RSI filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
                    p("rsi_period", "RSI period", 14.0, 1.0, 50.0, 1.0),
                    p("rsi_val", "RSI value", 30.0, 10.0, 90.0, 1.0),
                    p("adx_on", "ADX filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
                    p("adx_period", "ADX period", 8.0, 1.0, 30.0, 1.0),
                    p("adx_val", "ADX value", 20.0, 5.0, 90.0, 5.0),
                    p("roc_on", "ROC filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
                    p("roc_side", "ROC threshold side (0 above, 1 below)", 0.0, 0.0, 1.0, 1.0),
                    p("roc_days", "ROC days", 1.0, 1.0, 20.0, 1.0),
                    p("roc_thr", "ROC threshold", 1.0, 0.0, 100.0, 1.0),
                ]);
            }
            if id == V2 {
                v.push(p("tsl_pct", "Trailing stop % (0 off)", 0.0, 0.0, 100.0, 1.0));
                v.push(p("tsl_close", "Trail from (0 high, 1 close)", 0.0, 0.0, 1.0, 1.0));
            }
            v.extend([
                p("mkt_flt", "Market filter (0 none, 1 NIFTY 50 index)", 0.0, 0.0, 1.0, 1.0),
                p("flt_tf", "Index filter timeframe (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0),
                p("scene", "Market condition (1 bull, 2 bear)", 1.0, 1.0, 2.0, 1.0),
                p("iflt", "Index filter (1 ST, 2 SMA, 3 Swing%)", 1.0, 1.0, 3.0, 1.0),
                p("ist_point", "Index ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
                p("ist_period", "Index ST period", 20.0, 1.0, 20.0, 1.0),
                p("ist_factor", "Index ST factor", 1.0, 0.5, 20.0, 0.5),
                p("isma", "Index SMA period", 200.0, 1.0, 300.0, 1.0),
                p("bear_pct", "Index swing % for bear market", 23.0, 2.0, 50.0, 1.0),
                p("bull_pct", "Index swing % for bull market", 44.0, 2.0, 50.0, 1.0),
            ]);
        }
        _ => return None,
    }
    Some(v)
}

// ---------------------------------------------------------------- shared by the yt_bb / yt_gann / yt_cr stock AFLs

/// Signal arrays as an AFL loop leaves them: entries at the close, exits with their price and reason.
pub(crate) struct Sig4 {
    pub buy: Vec<bool>,
    pub sell: Vec<Option<(f64, &'static str)>>,
    pub short: Vec<bool>,
    pub cover: Vec<Option<(f64, &'static str)>>,
}
impl Sig4 {
    pub(crate) fn new(n: usize) -> Self {
        Sig4 { buy: vec![false; n], sell: vec![None; n], short: vec![false; n], cover: vec![None; n] }
    }
}

/// AmiBroker's regular backtester on loop signals (no ExRem): exits before entries on a bar; an opposite entry
/// reverses (at the close); redundant entries and exits while flat are ignored. Entries only in from..=to; a
/// position still open at `to` is marked there at the close.
pub(crate) fn amibt(x: &Ctx, s: &Sig4, from: usize, to: usize) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut pos: Option<(f64, usize)> = None;
    for i in from..=to {
        if let Some((d, e)) = pos {
            let ex = if d > 0.0 {
                s.sell[i].or(if s.short[i] { Some((x.c[i], "Reversed by short")) } else { None })
            } else {
                s.cover[i].or(if s.buy[i] { Some((x.c[i], "Reversed by buy")) } else { None })
            };
            if let Some((px, r)) = ex {
                out.push(trade(d, e, x.c[e], i, bound(x, i, px), r, false));
                pos = None;
            }
        }
        if pos.is_none() {
            if s.buy[i] {
                pos = Some((1.0, i));
            } else if s.short[i] {
                pos = Some((-1.0, i));
            }
        }
    }
    if let Some((d, e)) = pos {
        out.push(trade(d, e, x.c[e], to, x.c[to], "open", true));
    }
    out
}

/// The team's yearly turnover threshold (Volsig).
pub(crate) fn volsig(year: i64) -> f64 {
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

/// Turnover filter: `OpenInt > Volsig` with close x volume as the turnover (no volume = fails); all true when off.
pub(crate) fn turnover(x: &Ctx, on: bool) -> Vec<bool> {
    (0..x.c.len())
        .map(|i| {
            if !on {
                return true;
            }
            let y: i64 = x.r[i].date.get(..4).and_then(|s| s.parse().ok()).unwrap_or(0);
            x.r[i].volume.map_or(false, |v| x.c[i] * v > volsig(y))
        })
        .collect()
}

/// Dead-stock backtest toggle: YES = no buy on the last bar and a sell there; NO = no buy on the first bar and
/// (`sell_bar0`, the BB AFLs) a sell signal on the first bar, or none (the Gann / Candle Range AFLs).
pub(crate) fn dead_sigs(n: usize, yes: bool, sell_bar0: bool) -> (Vec<bool>, Vec<bool>) {
    if yes {
        ((0..n).map(|i| i + 1 != n).collect(), (0..n).map(|i| i + 1 == n).collect())
    } else {
        ((0..n).map(|i| i != 0).collect(), (0..n).map(|i| sell_bar0 && i == 0).collect())
    }
}

/// Upper circuit: Prec(O,2) == Prec(C,2) AND Prec(O,2) == Prec(H,2).
pub(crate) fn upper_circuit(x: &Ctx, i: usize) -> bool {
    prec2(x.o[i]) == prec2(x.c[i]) && prec2(x.o[i]) == prec2(x.h[i])
}

/// Dead stock as the AFLs write it: O, H, L, C equal to Ref(.., k) for k = 1..30 (the next 30 bars).
fn dead30(x: &Ctx) -> Vec<bool> {
    let n = x.c.len();
    let mut run = vec![0usize; n];
    for i in (0..n.saturating_sub(1)).rev() {
        let same = x.o[i] == x.o[i + 1] && x.h[i] == x.h[i + 1] && x.l[i] == x.l[i + 1] && x.c[i] == x.c[i + 1];
        run[i] = if same { run[i + 1] + 1 } else { 0 };
    }
    run.iter().map(|&k| k >= 30).collect()
}

/// BBandTop / BBandBot: MA(C, n) +/- k x population SD of the last n closes; Null until n bars.
fn bband(c: &[f64], n: usize, k: f64, top: bool) -> Vec<Option<f64>> {
    let n = n.max(1);
    (0..c.len())
        .map(|i| {
            if i + 1 < n {
                return None;
            }
            let w = &c[i + 1 - n..=i];
            let m = w.iter().sum::<f64>() / n as f64;
            let sd = (w.iter().map(|v| (v - m).powi(2)).sum::<f64>() / n as f64).sqrt();
            Some(if top { m + k * sd } else { m - k * sd })
        })
        .collect()
}

/// The NIFTY 50 benchmark as a context of its own over the stock's dates (from its first value, carried forward
/// over missing days). Returns the context rows and the first bar it covers.
fn bench_rows(x: &Ctx) -> Option<(Vec<Row>, usize)> {
    let c = x.series("BENCH.Close")?;
    let first = c.iter().position(|v| v.is_finite())?;
    let get = |k: &str| x.extra.get(k).map(|v| v.as_slice());
    let (o, h, l) = (get("BENCH.Open"), get("BENCH.High"), get("BENCH.Low"));
    let mut rows = Vec::with_capacity(c.len() - first);
    let mut last: (f64, f64, f64, f64) = (c[first], c[first], c[first], c[first]);
    for i in first..c.len() {
        if c[i].is_finite() {
            let pick = |s: Option<&[f64]>| s.map(|v| v[i]).filter(|v| v.is_finite()).unwrap_or(c[i]);
            last = (pick(o), pick(h).max(c[i]), pick(l).min(c[i]), c[i]);
        }
        rows.push(Row { ticker: "BENCH".into(), date: x.r[i].date.clone(), open: last.0, high: last.1, low: last.2, close: last.3, volume: None });
    }
    Some((rows, first))
}

/// The AFL's FLTShort from its INDEX market filter (all true with the filter NONE; all false without index data).
fn index_filter(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Vec<bool> {
    let n = x.c.len();
    if g("mkt_flt").round() as i64 != 1 {
        return vec![true; n];
    }
    let Some((rows, first)) = bench_rows(x) else { return vec![false; n] };
    let bx = Ctx::new(&rows, &rows, &[]);
    let tf = match g("flt_tf").round() as i64 { 2 => 'W', 3 => 'D', _ => 'M' };
    let bull = g("scene").round() as i64 != 2;
    let part: Vec<bool> = match g("iflt").round() as i64 {
        2 => sma_filter(&bx, tf, g("isma").round().max(1.0) as usize, true).1,
        3 => {
            let up: Vec<f64> = bx.c.iter().map(|v| v * (1.0 + g("bull_pct") / 100.0)).collect();
            let dn: Vec<f64> = bx.c.iter().map(|v| v * (1.0 - g("bear_pct") / 100.0)).collect();
            let (tr, _) = swing(&bx.c, &up, &dn, 0.0);
            tr.iter().map(|&t| if bull { t == 1 } else { t == -1 }).collect()
        }
        _ => {
            let m = st_filter(&bx, tf, g("ist_point").round() as i64, g("ist_period").round().max(1.0) as usize, g("ist_factor"));
            m.iter().map(|&t| if bull { t == 1 } else { t == -1 }).collect()
        }
    };
    (0..n).map(|i| i >= first && part[i - first]).collect()
}

/// ROC(C, n) in %; Null for the first n bars.
fn roc(c: &[f64], n: usize) -> Vec<Option<f64>> {
    (0..c.len()).map(|i| if i < n || c[i - n] == 0.0 { None } else { Some((c[i] - c[i - n]) / c[i - n] * 100.0) }).collect()
}

// ---------------------------------------------------------------- the index-filter loop AFLs

fn bb_loop(x: &Ctx, id: &str, g: &dyn Fn(&str) -> f64) -> Sig4 {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let per = g("bb_period").round().max(1.0) as usize;
    let later = id != INDEX;
    let ub = bband(c, per, g("ub_sd"), true);
    let lb = if !later || gi("lb_on") == 1 { bband(c, per, g("lb_sd"), false) } else { vec![Some(1.0); n] };
    let exit_pct = g("exit_pct");
    let tr = close_swing(c, exit_pct);
    let mut hc = c.clone();
    for i in 1..n {
        hc[i] = hc[i].max(hc[i - 1]);
    }
    let dead = dead30(x);
    let dead_yes = gi("dead_bt") == 1;
    let (dbs, dss) = dead_sigs(n, dead_yes, true);
    let turn = turnover(x, gi("turnover") == 1);
    let penny = g("penny");
    let bf: Vec<bool> = (0..n).map(|i| turn[i] && !upper_circuit(x, i) && h[i] != c[i] && c[i] > penny && dbs[i] && !dead[i]).collect();
    let lc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let fs = index_filter(x, g);
    // V1 / V2 / Master options (all on by default = true)
    let opt = |on: bool, v: Vec<Option<f64>>, f: &dyn Fn(f64) -> bool| -> Vec<bool> { if on { v.iter().map(|z| z.map_or(false, f)).collect() } else { vec![true; n] } };
    let (s1, s2, rc) = if later {
        let rv = g("rsi_val");
        let av = g("adx_val");
        let thr = g("roc_thr");
        let below = gi("roc_side") == 1;
        let adx: Vec<Option<f64>> = crate::connors::dmi(x.r, g("adx_period").round().max(1.0) as usize).1.into_iter().map(|v| if v.is_finite() { Some(v) } else { None }).collect();
        (
            opt(gi("rsi_on") == 1, crate::books::yteng::rsi(c, g("rsi_period").round().max(1.0) as usize), &|v| v > rv),
            opt(gi("adx_on") == 1, adx, &|v| v > av),
            opt(gi("roc_on") == 1, roc(c, g("roc_days").round().max(1.0) as usize), &|v| if below { v < -thr } else { v > thr }),
        )
    } else {
        (vec![true; n], vec![true; n], vec![true; n])
    };
    // swing ATHC flag (V1 / V2 / Master; off by default = 0 everywhere)
    let mut athc = vec![false; n];
    if later && gi("athc") == 1 {
        for i in 1..n {
            let near = id == V2 && c[i] < hc[i - 1] && c[i] > hc[i - 1] * (1.0 - exit_pct / 100.0);
            athc[i] = if c[i] > hc[i - 1] || near { true } else if tr[i] == -1 && athc[i - 1] { false } else { athc[i - 1] };
        }
    }
    // the dead-stock toggle read as Deadstock[i] in V1's first sell test (YES = 0, NO = 1)
    let v1_toggle = !dead_yes;
    let hard = g("hard_stop");
    let (tsl_pct, tsl_close) = if id == V2 { (g("tsl_pct"), gi("tsl_close") == 1) } else { (0.0, false) };
    let mut s = Sig4::new(n);
    let (mut pos, mut skip) = (false, false);
    let (mut hsl, mut tsl, mut hcab) = (0.0f64, 0.0f64, 0.0f64);
    for i in 1..n {
        let ubv = loop_val(ub[i]);
        let lbv = loop_val(lb[i]);
        // no entry before the band exists: the team's trade lists never buy in a stock's first band-period bars
        // (jay-local/Matches/Bollinger Band (stocks).xlsx), although a Null band reads as -1e10 in the loop
        let entry = ub[i].is_some() && if later {
            bf[i] && !dead[i] && c[i] > ubv && fs[i] && s1[i] && s2[i] && rc[i] && athc[i]
        } else {
            bf[i] && c[i] > hc[i - 1] && !dead[i] && c[i] > ubv && fs[i]
        };
        if !pos && entry {
            s.buy[i] = true;
            if hard > 0.0 {
                hsl = (1.0 - hard * 0.01) * c[i];
            }
            if tsl_pct > 0.0 {
                hcab = if tsl_close { c[i] } else { h[i] };
                tsl = hcab - hcab * tsl_pct / 100.0;
            }
            pos = true;
        } else if pos && !skip {
            if tsl_pct > 0.0 {
                hcab = hcab.max(if tsl_close { c[i] } else { h[i] });
            }
            let hit = c[i] <= hsl || c[i] <= tsl || dss[i] || tr[i] == -1 || dead[i] || c[i] < lbv;
            if hit {
                let go = !lc[i] || dss[i] || if id == V1 { v1_toggle } else { dead[i] };
                if go {
                    let r = if tr[i] == -1 { "Swing down" } else if c[i] < lbv { "Below lower band" } else if c[i] <= hsl { "Hard stop" } else if c[i] <= tsl { "Trailing stop" } else { "Dead stock" };
                    s.sell[i] = Some((c[i], r));
                    pos = false;
                } else {
                    skip = true;
                }
            } else if tsl_pct > 0.0 {
                tsl = hcab - hcab * tsl_pct / 100.0;
            }
        } else if skip && (!lc[i] || dss[i] || dead[i]) {
            s.sell[i] = Some((c[i], "Sell after LC bar"));
            pos = false;
            skip = false;
        }
    }
    s
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    Some(match id {
        PLAIN => {
            let per = g("bb_period").round().max(1.0) as usize;
            let ub = bband(c, per, g("ub_sd"), true);
            let lb = bband(c, per, g("lb_sd"), false);
            let (dbs, dss) = dead_sigs(n, g("dead_bt").round() as i64 == 1, true);
            let turn = turnover(x, g("turnover").round() as i64 == 1);
            let penny = g("penny");
            let buy = (0..n).map(|i| ub[i].map_or(false, |u| c[i] > u) && turn[i] && !upper_circuit(x, i) && h[i] != c[i] && c[i] > penny && dbs[i]).collect();
            let sell = (0..n).map(|i| (lb[i].map_or(false, |b| c[i] < b) && l[i] != c[i]) || dss[i]).collect();
            backtest(x, Sig { buy, sell, short: vec![false; n], cover: vec![false; n] }, 1, from, to)
        }
        INDEX | V1 | V2 | MASTER => amibt(x, &bb_loop(x, id, &g), from, to),
        _ => return None,
    })
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        PLAIN => "ytbb_plain",
        INDEX => "ytbb_index_filter",
        V1 => "ytbb_index_filter_v1",
        V2 => "ytbb_index_filter_v2",
        MASTER => "ytbb_master",
        _ => return None,
    }, name, "Bollinger band"))
}

/// Test helpers shared by the yt_bb / yt_gann / yt_cr tests.
#[cfg(test)]
pub(crate) mod stocktest {
    use crate::books::ytwh::testbars::{bars, wave};
    use crate::connors::{Ctx, Params, Trade};
    use crate::Row;

    pub fn with_volume(mut r: Vec<Row>, v: &[f64]) -> Vec<Row> {
        for (k, row) in r.iter_mut().enumerate() {
            row.volume = Some(v[k % v.len()]);
        }
        r
    }
    pub fn rows(ohlc: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        bars(ohlc)
    }
    pub fn bench(x: &mut Ctx, r: &[Row]) {
        let c: Vec<f64> = (0..r.len()).map(|k| if k < 5 { f64::NAN } else { 10000.0 + 800.0 * ((k as f64) / 40.0).sin() + k as f64 }).collect();
        x.extra.insert("BENCH.Close".into(), c.clone());
        x.extra.insert("BENCH.High".into(), c.iter().map(|v| v + 30.0).collect());
        x.extra.insert("BENCH.Low".into(), c.iter().map(|v| v - 30.0).collect());
        x.extra.insert("BENCH.Open".into(), c.clone());
    }
    /// Every id on a few hundred weekday bars, with and without the benchmark, full range and a sub-range.
    pub fn smoke(run: fn(&str, &Ctx, usize, usize, &Params) -> Option<Vec<Trade>>, ids: &[&str], extra: &[(&str, f64)]) {
        let r = with_volume(wave(420), &[1.0e5, 1.6e5, 0.9e5, 2.2e5, 1.2e5]);
        for with_bench in [false, true] {
            let mut x = Ctx::new(&r, &r, &[]);
            if with_bench {
                bench(&mut x, &r);
            }
            for id in ids {
                let mut sets: Vec<Params> = vec![Params::new()];
                sets.push(extra.iter().map(|(k, v)| (k.to_string(), *v)).collect());
                for p in &sets {
                    for (from, to) in [(0, r.len() - 1), (60, 330)] {
                        let t = run(id, &x, from, to, p).unwrap();
                        let mut last_exit = 0usize;
                        for t in &t {
                            assert!(t.legs[0].i >= from && t.legs[0].i <= to && t.exit_i >= t.legs[0].i && t.exit_i <= to, "{id}");
                            assert!(t.legs[0].i >= last_exit, "{id}: overlapping trades");
                            last_exit = t.exit_i;
                            assert!(t.exit_px.is_finite() && t.exit_px >= x.l[t.exit_i] - 1e-9 && t.exit_px <= x.h[t.exit_i] + 1e-9, "{id}");
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::stocktest::*;
    use super::*;
    use crate::books::ytwh::testbars::ps;

    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<(usize, f64, usize, f64, bool)> {
        let x = Ctx::new(r, r, &[]);
        run(id, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.open)).collect()
    }
    fn bar(c: f64) -> (f64, f64, f64, f64) {
        (c, c + 1.0, c - 1.0, c)
    }

    #[test]
    fn bands_use_population_sd() {
        let b = bband(&[1.0, 2.0, 3.0], 3, 2.0, true);
        assert_eq!(b[1], None);
        assert!((b[2].unwrap() - (2.0 + 2.0 * (2.0f64 / 3.0).sqrt())).abs() < 1e-12);
    }

    #[test]
    fn plain_breaks_out_and_exits_below_lower_band() {
        // 5-bar bands, 1 SD each side: flat 100s, a jump to 110 buys, a fall to 95 sells
        let mut v: Vec<_> = (0..6).map(|_| bar(100.0)).collect();
        v.extend([bar(110.0), bar(111.0), bar(95.0), bar(96.0)]);
        let t = go(PLAIN, &rows(&v), &[("bb_period", 5.0), ("ub_sd", 1.0), ("lb_sd", 1.0)]);
        assert_eq!(t, vec![(6, 110.0, 8, 95.0, false)]);
        // a breakout bar whose high equals its close (UC) is not bought
        let mut w = v.clone();
        w[6] = (100.0, 110.0, 99.0, 110.0);
        assert_eq!(go(PLAIN, &rows(&w), &[("bb_period", 5.0), ("ub_sd", 1.0), ("lb_sd", 1.0)])[0].0, 7);
    }

    #[test]
    fn index_filter_needs_new_high_and_swing_exit() {
        // 5-bar bands: buys need a close above the previous highest close; a 10% swing exit sells
        let mut v: Vec<_> = (0..6).map(|_| bar(100.0)).collect();
        v.extend([bar(110.0), bar(112.0), bar(100.0), bar(99.0)]);
        let r = rows(&v);
        let t = go(INDEX, &r, &[("bb_period", 5.0), ("ub_sd", 1.0), ("exit_pct", 10.0)]);
        // bar 6: 110 > 100 (ATHC) and > upper band -> buy; bar 8: 100 < 112 x 0.9 = 100.8 -> swing down -> sell
        assert_eq!(t[0], (6, 110.0, 8, 100.0, false));
        // on an LC bar (low = close) the sell waits for the next bar
        let mut w = v.clone();
        w[8] = (101.0, 102.0, 100.0, 100.0);
        let t = go(INDEX, &rows(&w), &[("bb_period", 5.0), ("ub_sd", 1.0), ("exit_pct", 10.0)]);
        assert_eq!((t[0].2, t[0].3), (9, 99.0));
        // index filter on without benchmark data: no buys
        assert!(go(INDEX, &r, &[("bb_period", 5.0), ("ub_sd", 1.0), ("mkt_flt", 1.0)]).is_empty());
    }

    #[test]
    fn later_versions_need_the_athc_flag() {
        let mut v: Vec<_> = (0..6).map(|_| bar(100.0)).collect();
        v.extend([bar(110.0), bar(112.0), bar(100.0), bar(99.0)]);
        let r = rows(&v);
        let kv = [("bb_period", 5.0), ("ub_sd", 1.0), ("exit_pct", 10.0)];
        for id in [V1, V2, MASTER] {
            assert!(go(id, &r, &kv).is_empty(), "{id}: defaults never buy");
            let t = go(id, &r, &[kv[0], kv[1], kv[2], ("athc", 1.0)]);
            assert_eq!((t[0].0, t[0].2), (6, 8), "{id}");
        }
        // V2 trailing stop of 2% from the high: bar 7 high 113 -> trail 110.74, raised after bar 7; bar 8 sells
        let mut w = v.clone();
        w[8] = (111.0, 111.5, 110.0, 110.5);
        let t = go(V2, &rows(&w), &[kv[0], kv[1], kv[2], ("athc", 1.0), ("tsl_pct", 2.0)]);
        assert_eq!((t[0].2, t[0].3), (8, 110.5));
    }

    #[test]
    fn amibt_reverses_and_ignores_redundant_signals() {
        let r = rows(&(0..6).map(|k| bar(100.0 + k as f64)).collect::<Vec<_>>());
        let x = Ctx::new(&r, &r, &[]);
        let mut s = Sig4::new(6);
        s.buy[1] = true;
        s.buy[2] = true;
        s.short[3] = true;
        s.cover[4] = Some((x.c[4], "Cover"));
        s.sell[5] = Some((x.c[5], "Sell"));
        let t = amibt(&x, &s, 0, 5);
        assert_eq!(t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(1.0, 1, 3), (-1.0, 3, 4)]);
    }

    #[test]
    fn dead_stock_looks_30_bars_ahead() {
        let mut v: Vec<_> = (0..5).map(|k| bar(100.0 + k as f64)).collect();
        v.extend((0..31).map(|_| bar(50.0)));
        let r = rows(&v);
        let x = Ctx::new(&r, &r, &[]);
        let d = dead30(&x);
        assert!(d[5] && !d[6] && !d[4]);
    }

    #[test]
    fn defaults_trade_on_a_breakout() {
        // 150 quiet bars around 100, a rally to new highs, then a 30% fall
        let mut v: Vec<_> = (0..150).map(|k| bar(100.0 + ((k as f64) / 3.0).sin())).collect();
        v.extend((0..40).map(|k| bar(102.0 + 1.5 * k as f64)));
        v.extend((0..40).map(|k| bar(160.0 - 1.5 * k as f64)));
        let r = rows(&v);
        let x = Ctx::new(&r, &r, &[]);
        for id in [PLAIN, INDEX] {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            eprintln!("{id}: {} trades", t.len());
            assert!(!t.is_empty(), "{id}");
        }
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS, &[("athc", 1.0), ("lb_on", 1.0), ("mkt_flt", 1.0), ("tsl_pct", 5.0), ("rsi_on", 1.0), ("hard_stop", 5.0)]);
        smoke(run, &[INDEX, V2], &[("mkt_flt", 1.0), ("iflt", 2.0), ("flt_tf", 3.0)]);
        smoke(run, &[INDEX, MASTER], &[("athc", 1.0), ("mkt_flt", 1.0), ("iflt", 3.0), ("scene", 2.0)]);
    }
}
