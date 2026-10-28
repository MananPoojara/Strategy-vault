//! YT Strategies · Nick Radge: the team's AmiBroker AFLs in "Old Topics/Nick Radge" (the "INTRADAY ... SYSTEM FOR
//! INDIAN MKT" family) that trade individual stocks on daily bars. Coded from the AFLs as written, with the AFLs'
//! own default settings; there is no book PDF and nothing here is checked against the team's workbooks.
//!
//! The system (Nick Radge's mean-reversion "stretch" entry): a setup bar closes in the lower half of its range
//! (short: upper half) with ADX(5) above 40, the close above its 200-day average and the price / average-volume
//! filters passing; on the next bar a limit order sits at the setup bar's low less 0.75 x ATR(5) (short: high plus),
//! rounded to the tick. "Intraday" in the file names means the trade is closed at that same bar's close
//! (`Sell = Buy`, `SetOption("AllowSameBarExit", True)`, no trade delays).
//!
//! AmiBroker behaviour followed (see `ytsw.rs` / `ytwh.rs`):
//! - the AFLs' custom backtester drops an entry whose limit price is below the bar's low (short: above its high),
//!   which is a per-stock fill rule and is coded; its order cap (Maximum Open Positions) and the PositionScore
//!   ranking (`Ref(100 - ROC(C, 3), -1)`) only choose among stocks and are not modelled;
//! - entry price as written: the open + 0.25% when the open is below the limit (short: the open less the
//!   "OPEN SELL BUFF" % when above it), else the limit; exit price as written; both are kept inside the bar's
//!   low-high range (AmiBroker's price bound check);
//! - the optional `ApplyStop(stopTypeLoss, percent)` (default 0 = off) runs with `ActivateStopsImmediately`, so it
//!   is live on the entry bar: a low at or below entry x (1 - SL%) exits at that level before the close exit;
//! - ATR / ADX are Wilder's (ATR(1) Null on the first bar); `EMA(V, 50)` starts from the simple average of the
//!   first 50 bars; `Prec(x, 2)` truncates to 2 decimals; a missing volume fails the average-volume filter;
//! - `HHV(C, BarCount)` (ATHC AFLs) is the highest close so far (as in ytsw REGIM / ytib).
#![allow(dead_code)]
use crate::books::ytd7::{ma, prec2};
use crate::books::ytsw::close_swing;
use crate::books::ytwh::bound;
use crate::connors::{dmi, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_nr";
pub const LONG_BASIC: &str = "ytnr_long_basic";
pub const LONG_NO_ADX: &str = "ytnr_long_no_adx";
pub const LONG_BUFFER: &str = "ytnr_long_hsl_buffer";
pub const LONG_NO_ADX_BUFFER: &str = "ytnr_long_no_adx_hsl_buffer";
pub const LONG_SPS: &str = "ytnr_long_lc_exit_sps";
pub const LONG_BEAR: &str = "ytnr_long_bear";
pub const LONG_ATHC: &str = "ytnr_long_athc";
pub const LONG_ATHC_OLD: &str = "ytnr_long_athc_old";
pub const SHORT_BASIC: &str = "ytnr_short_basic";
pub const SHORT_BUFFER: &str = "ytnr_short_hsl_buffer";
pub const IDS: [&str; 10] = [LONG_BASIC, LONG_NO_ADX, LONG_BUFFER, LONG_NO_ADX_BUFFER, LONG_SPS, LONG_BEAR, LONG_ATHC, LONG_ATHC_OLD, SHORT_BASIC, SHORT_BUFFER];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Nick Radge", session: 0, dir, status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const FLT: &str = "F|Setup-bar filters: close between 10 and 5,000 (short AFLs 60,000); 50-bar EMA of volume above 500,000; no lower-circuit bar closing less than 6% down (short: upper circuit, open = high = close to 2 decimals, less than 6% up); optional NIFTY 50 above its 100-day average (off) and turnover filter (off)";
const LONG_E: &str = "E|Next bar, not a lower-circuit bar: buy limit at the setup low less 0.75 x ATR(5), rounded to the tick (0.01; 0.005 below 2; 0.001 below 0.10); filled only if the bar's low reaches it, at the open + 0.25% when the open is below the limit, else at the limit";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(LONG_BASIC, "Nick Radge Stretch Long, Exit at Close", "Long", &[
            "F|Setup bar: close in the lower half of the day's range, ADX(5) above 40, close above its 200-day average", FLT, LONG_E,
            "X|Sell at the entry bar's close",
        ], concat!("INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct.afl (Nick radge analysis/LONG_ with Circuit filter), coded as the AFL is written. Same per-stock rules: NICK RADGE_LONG ONLY_Without ROC (only the ranking is removed) and the 'trash' copy.",
            " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(LONG_NO_ADX, "Nick Radge Stretch Long, No ADX, Exit at Close", "Long", &[
            "F|Setup bar: close in the lower half of the day's range, close above its 200-day average (no ADX condition)", FLT, LONG_E,
            "X|Sell at the entry bar's close",
        ], concat!("NICK RADGE_LONG ONLY_Without ADX & ROC/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct without adx n roc.afl, coded as the AFL is written.",
            " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; ranking and portfolio limits not modelled (it has no PositionScore; Maximum Open Positions 10). The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(LONG_BUFFER, "Nick Radge Stretch Long, Exit Buffer + Hard SL", "Long", &[
            "F|Setup bar: close in the lower half of the day's range, ADX(5) above 40, close above its 200-day average", FLT, LONG_E,
            "X|Sell at the entry bar's close less the exit buffer % (0.4); on a bar closing at its low, at the close + 1% (kept within the bar's high); optional hard stop % (default 0 = off), live on the entry bar",
        ], concat!("NICK RADGE LONG only_With Hard SL & exit buffer/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct n hard sl N EXIT BUFFER.afl, coded as the AFL is written. Same per-stock rules: the copy at the folder root and the three copies under Nick radge analysis (fo / NSE 500C&P; one sets the hard SL with Optimize, default 0).", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table. A hard stop and the close exit on the same bar: the stop is taken first.")),
        def(LONG_NO_ADX_BUFFER, "Nick Radge Stretch Long, No ADX, Exit Buffer + Hard SL", "Long", &[
            "F|Setup bar: close in the lower half of the day's range, close above its 200-day average (no ADX condition)", FLT, LONG_E,
            "X|As the exit-buffer version: close less 0.4%, close + 1% on a bar closing at its low, optional hard stop % (off)",
        ], concat!("NICK RADGE_LONG only_Without ADX & with ROC/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct without ADX.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(LONG_SPS, "Nick Radge Stretch Long, Low-Close Exit + Hard SL (SPS)", "Long", &[
            "F|Setup bar: close in the lower half of the day's range, ADX(5) above 40, close above its 200-day average", FLT, LONG_E,
            "X|Sell at the entry bar's close; on a bar closing at its low, at the close + 1% (kept within the bar's high); optional hard stop % (default 0 = off), live on the entry bar",
        ], concat!("Nick radge analysis/LONG_ with Circuit filter/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct n hard sl (With SPS).afl, coded as the AFL is written (same per-stock rules: its '- Copy' and the NSE 500C&P/WITHOUT EXIT BUFFER copy). This AFL does not set AllowSameBarExit itself (it is commented out); the same-bar exit is taken as enabled in the team's settings, as in every other AFL of the family.", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(LONG_BEAR, "Nick Radge Stretch Long for Bear Market, Exit at Close", "Long", &[
            "F|Setup bar: close in the lower half of the day's range, ADX(5) above 40, close BELOW its 200-day average", FLT, LONG_E,
            "X|Sell at the entry bar's close",
        ], concat!("NICK RADGE LONG only_For BEAR mkt_With Hard SL & exit buffer/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct for bear mkt.afl, coded as the AFL is written: despite the folder name the AFL has no hard stop or exit buffer (both commented out) and sells at the close.", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(LONG_ATHC, "Nick Radge Stretch Long on All-Time-High Stocks", "Long", &[
            "F|ATHC regime: on from a close above every earlier close until the 10% close swing turns down; the setup bar's regime must be on",
            "F|Setup bar: close in the lower half of the day's range, ADX(5) above 40, close above its 200-day average", FLT, LONG_E,
            "X|As the exit-buffer version: close less 0.4%, close + 1% on a bar closing at its low, optional hard stop % (off)",
        ], concat!("NICK RADGE on ATH stocks_LONG only/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct FOR ATHC STOCK (With Hrd SL & Exit buffer)..afl, coded as the AFL is written (same rules: FO ATH AFL Check/..._v1 and the NSE 500C&P ATH-basket copy).", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(LONG_ATHC_OLD, "Nick Radge Stretch Long on All-Time-High Stocks (Old Version)", "Long", &[
            "F|ATHC regime as in the ATH AFL, but read one bar before the setup bar",
            "F|Setup bar: close in the lower half of the day's range, ADX(5) above 40, close above its 200-day average", FLT, LONG_E,
            "X|As the exit-buffer version: close less 0.4%, close + 1% on a bar closing at its low, optional hard stop % (off)",
        ], concat!("FO ATH AFL Check/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct FOR ATHC STOCK (With Hrd SL & Exit buffer)_Old Version..afl, coded as the AFL is written; it differs from v1 only in `Ref(ATHCBUYSIG, -1)`.", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(SHORT_BASIC, "Nick Radge Stretch Short, Cover at Close", "Short", &[
            "F|Setup bar: close in the UPPER half of the day's range, ADX(5) above 40, close above its 200-day average (as written)", FLT,
            "E|Next bar, not an upper-circuit bar: sell-short limit at the setup high plus 0.75 x ATR(5), rounded to 0.01; filled only if the bar's high reaches it, at the open less 0.25% when the open is above the limit, else at the limit",
            "X|Cover at the entry bar's close",
        ], concat!("Nick radge analysis/SHORT_With circuit filter/INTRADAY SHORT SYSTEM FOR INDIAN MKT with circuit  flt 5 pct.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 + 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default; long-side condition index above its average, as written) uses NIFTY 50 for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(SHORT_BUFFER, "Nick Radge Stretch Short, Exit Buffer + Hard SL", "Short", &[
            "F|Setup bar: close in the UPPER half of the day's range, ADX(5) above 40, close above its 200-day average (as written)", FLT,
            "E|Next bar, not an upper-circuit bar: sell-short limit at the setup high plus 0.75 x ATR(5), rounded to 0.01; filled only if the bar's high reaches it, at the open less the open sell buffer % (0.25) when the open is above the limit, else at the limit",
            "X|Cover at the entry bar's close plus the exit buffer % (0.4); on a bar closing at its high, at the close - 1% (kept within the bar's low); optional hard stop % (off), live on the entry bar",
        ], concat!("NICK RADGE_SHORT only_With Hard SL & exit buffer/INTRADAY SHORT SYSTEM FOR INDIAN MKT with circuit  flt 5 pct n hard sl n EXIT BUFFER.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 + 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default; index above its average, as written) uses NIFTY 50 for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table. A hard stop and the close exit on the same bar: the stop is taken first.")),
    ]
}

#[derive(Clone, Copy)]
struct Var {
    short: bool,
    adx: bool,
    above_ma: bool,
    /// exit price: 0 close, 1 close + 1% on L == C else close (SPS), 2 buffer + low-close rule
    exit: u8,
    stop: bool,
    athc: Option<usize>,
}

fn var(id: &str) -> Option<Var> {
    let v = Var { short: false, adx: true, above_ma: true, exit: 0, stop: false, athc: None };
    Some(match id {
        LONG_BASIC => v,
        LONG_NO_ADX => Var { adx: false, ..v },
        LONG_BUFFER => Var { exit: 2, stop: true, ..v },
        LONG_NO_ADX_BUFFER => Var { adx: false, exit: 2, stop: true, ..v },
        LONG_SPS => Var { exit: 1, stop: true, ..v },
        LONG_BEAR => Var { above_ma: false, ..v },
        LONG_ATHC => Var { exit: 2, stop: true, athc: Some(0), ..v },
        LONG_ATHC_OLD => Var { exit: 2, stop: true, athc: Some(1), ..v },
        SHORT_BASIC => Var { short: true, ..v },
        SHORT_BUFFER => Var { short: true, exit: 2, stop: true, ..v },
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let v = var(id)?;
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut out = vec![
        p("s_look", "Stretch lookback (ATR period)", 5.0, 1.0, 100.0, 1.0),
        p("s_dist", "Stretch (ATR multiple)", 0.75, 0.0, 100.0, 0.01),
    ];
    if v.adx {
        out.push(p("adx_n", "ADX period", 5.0, 1.0, 60.0, 1.0));
        out.push(p("adx_lvl", "ADX level", 40.0, 0.0, 60.0, 5.0));
    }
    out.extend([
        p("price_flt", "Price filter (0 off, 1 on)", 1.0, 0.0, 1.0, 1.0),
        p("min_sp", "Minimum share price", 10.0, 0.0, 1000.0, 0.01),
        p("max_sp", "Maximum share price", if v.short { 60000.0 } else { 5000.0 }, 0.0, 9_000_000.0, 0.01),
        p("vol_flt", "Average volume filter (0 off, 1 on)", 1.0, 0.0, 1.0, 1.0),
        p("min_avg_vol", "Minimum average volume", 500_000.0, 0.0, 10_000_000.0, 1000.0),
        p("vol_ema", "Volume EMA period", 50.0, 1.0, 200.0, 1.0),
        p("index_flt", "Index filter, NIFTY 50 above its average (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("index_ma", "Index filter MA period", 100.0, 1.0, 300.0, 1.0),
        p("turnover_flt", "Turnover filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
    ]);
    if v.short {
        out.push(p("open_buff", "Open sell buffer %", 0.25, 0.0, 5.0, 0.05));
    }
    if v.exit == 2 {
        out.push(p("exit_buff", "Exit buffer %", 0.4, 0.0, 2.0, 0.01));
    }
    if v.stop {
        out.push(p("hard_sl", "Hard stop loss % (0 off)", 0.0, 0.0, 50.0, 0.5));
    }
    if v.athc.is_some() {
        out.push(p("athc_exit", "ATHC exit swing %", 10.0, 5.0, 35.0, 1.0));
    }
    Some(out)
}

// ---------------------------------------------------------------- indicators

/// AFL ATR(n): Wilder average of the true range, the first bar's range being Null.
fn atr(x: &Ctx, n: usize) -> Vec<f64> {
    let tr: Vec<f64> = (0..x.c.len())
        .map(|i| if i == 0 { f64::NAN } else { (x.h[i] - x.l[i]).max((x.h[i] - x.c[i - 1]).abs()).max((x.l[i] - x.c[i - 1]).abs()) })
        .collect();
    wilder(&tr, n.max(1))
}

/// AFL EMA(v, n) started from the simple average of the first n values (NaN gaps restart it).
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let n = n.max(1);
    let k = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    let (mut run, mut sum, mut prev) = (0usize, 0.0, f64::NAN);
    for i in 0..v.len() {
        if !v[i].is_finite() {
            run = 0;
            sum = 0.0;
            continue;
        }
        run += 1;
        if run < n {
            sum += v[i];
        } else if run == n {
            prev = (sum + v[i]) / n as f64;
            out[i] = prev;
        } else {
            prev = v[i] * k + prev * (1.0 - k);
            out[i] = prev;
        }
    }
    out
}

/// The AFLs' turnover thresholds by calendar year.
fn volsig(date: &str) -> f64 {
    let y: i64 = date.get(..4).and_then(|s| s.parse().ok()).unwrap_or(0);
    match y {
        ..=1999 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

/// Benchmark close carried forward over missing days (AmiBroker's Foreign fills holes with the previous bar).
pub(crate) fn bench_ffill(x: &Ctx, name: &str) -> Option<Vec<f64>> {
    let s = x.series(name)?;
    let mut last = f64::NAN;
    Some(s.iter().map(|&v| { if v.is_finite() { last = v; } last }).collect())
}

fn trade(dir: f64, i: usize, px: f64, exit_px: f64, why: &str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px, reason: why.into(), entry_kind: "limit", signal_i: i, open: false, exits: Vec::new() }
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let v = var(id)?;
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let index_up: Vec<bool> = if g("index_flt").round() as i64 == 1 {
        let Some(ix) = bench_ffill(x, "BENCH.Close") else { return Some(Vec::new()) };
        let m = ma(&ix, g("index_ma").round().max(1.0) as usize);
        (0..n).map(|i| m[i].map_or(false, |m| ix[i] > m)).collect()
    } else {
        vec![true; n]
    };
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let ev = ema(&vol, g("vol_ema").round().max(1.0) as usize);
    let (min_sp, max_sp, min_vol) = (g("min_sp"), g("max_sp"), g("min_avg_vol"));
    let (pflt, vflt, tflt) = (g("price_flt").round() as i64 == 1, g("vol_flt").round() as i64 == 1, g("turnover_flt").round() as i64 == 1);
    let opt = |i: usize| {
        (!pflt || (c[i] >= min_sp && c[i] <= max_sp)) && (!vflt || ev[i] > min_vol) && (!tflt || vol[i] * c[i] > volsig(&x.r[i].date))
    };
    let adx = if v.adx { dmi(x.r, g("adx_n").round().max(1.0) as usize).1 } else { vec![f64::NAN; n] };
    let adx_lvl = g("adx_lvl");
    let ma200 = ma(c, 200);
    let stretch = atr(x, g("s_look").round().max(1.0) as usize);
    let s_dist = g("s_dist");
    let upc = |i: usize| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
    let loc = |i: usize| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
    // ATHC regime (ATH AFLs): on after a close above every earlier close, off when the close swing turns down
    let athc: Vec<bool> = match v.athc {
        None => vec![true; n],
        Some(_) => {
            let tr = close_swing(c, g("athc_exit"));
            let mut a = vec![false; n];
            let mut hcl = c[0];
            for i in 1..n {
                a[i] = if c[i] > hcl { true } else if tr[i] == -1 && a[i - 1] { false } else { a[i - 1] };
                hcl = hcl.max(c[i]);
            }
            a
        }
    };
    let setup: Vec<bool> = (0..n)
        .map(|i| {
            let range = (c[i] - l[i]) / (h[i] - l[i]);
            let half = if v.short { range > 0.5 } else { range < 0.5 };
            let trend = ma200[i].map_or(false, |m| if v.above_ma { c[i] > m } else { c[i] < m });
            let adx_ok = !v.adx || adx[i] > adx_lvl;
            // circuit5: a circuit bar that moved less than 6% (the AFL's "5% circuit" stocks)
            let circuit5 = i > 0
                && if v.short { upc(i) && (c[i] - c[i - 1]) / c[i] * 100.0 < 6.0 } else { loc(i) && (c[i - 1] - c[i]) / c[i - 1] * 100.0 < 6.0 };
            let athc_ok = match v.athc {
                Some(0) => athc[i],
                Some(_) => i > 0 && athc[i - 1],
                None => true,
            };
            half && adx_ok && trend && index_up[i] && opt(i) && !circuit5 && athc_ok
        })
        .collect();
    // limit price of the order placed after bar i
    let limit = |i: usize| -> Option<f64> {
        let a = stretch[i];
        if !a.is_finite() {
            return None;
        }
        if v.short {
            Some((((h[i] + a * s_dist) / 0.01).round()) * 0.01)
        } else {
            let tick = if l[i] < 0.10 { 0.001 } else if l[i] < 2.0 { 0.005 } else { 0.01 };
            Some(((l[i] - a * s_dist) / tick).round() * tick)
        }
    };
    let (buff, obuff, hs) = (g("exit_buff"), g("open_buff"), if v.stop { g("hard_sl") } else { 0.0 });
    let mut out = Vec::new();
    for i in from.max(1)..=to {
        if !setup[i - 1] {
            continue;
        }
        let Some(lim) = limit(i - 1) else { continue };
        if v.short {
            if upc(i) || lim > h[i] {
                continue;
            }
            // compared at AmiBroker's 32-bit price precision: an open equal to the tick-rounded limit fills at the limit
            let epx = bound(x, i, if (o[i] as f32) > (lim as f32) { o[i] - o[i] * obuff / 100.0 } else { lim });
            let stop = epx * (1.0 + hs / 100.0);
            let (px, why) = if hs > 0.0 && h[i] >= stop {
                (stop, "Hard stop (ApplyStop)")
            } else {
                match v.exit {
                    2 if h[i] == c[i] => (c[i] - c[i] / 100.0, "Cover at close - 1% (closed at high)"),
                    2 => (c[i] + c[i] * buff / 100.0, "Cover at close + exit buffer"),
                    _ => (c[i], "Cover at close"),
                }
            };
            out.push(trade(-1.0, i, epx, bound(x, i, px), why));
        } else {
            if loc(i) || lim < l[i] {
                continue;
            }
            // compared at AmiBroker's 32-bit price precision: an open equal to the tick-rounded limit fills at the limit
            let epx = bound(x, i, if (o[i] as f32) < (lim as f32) { o[i] + o[i] * 0.25 / 100.0 } else { lim });
            let stop = epx * (1.0 - hs / 100.0);
            let (px, why) = if hs > 0.0 && l[i] <= stop {
                (stop, "Hard stop (ApplyStop)")
            } else {
                match v.exit {
                    1 | 2 if l[i] == c[i] => (c[i] + c[i] / 100.0, "Sell at close + 1% (closed at low)"),
                    2 => (c[i] - c[i] * buff / 100.0, "Sell at close - exit buffer"),
                    _ => (c[i], "Sell at close"),
                }
            };
            out.push(trade(1.0, i, epx, bound(x, i, px), why));
        }
    }
    Some(out)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Nick Radge"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytd7::tests::{bars, ps, wave};
    use crate::Row;

    fn with_vol(mut r: Vec<Row>, v: f64) -> Vec<Row> {
        for x in r.iter_mut() {
            x.volume = Some(v);
        }
        r
    }
    /// 230 slowly rising bars (close above its 200-day average), a setup bar closing low (short: high) in its range,
    /// then the bar built from the order's limit price.
    fn scene(short: bool, next: impl Fn(f64) -> (f64, f64, f64, f64)) -> (Vec<Row>, f64) {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..230).map(|k| { let c = 100.0 + k as f64 * 0.05; (c - 0.5, c + 1.0, c - 1.0, c) }).collect();
        let c = 100.0 + 230.0 * 0.05;
        v.push(if short { (c, c + 2.0, c - 2.0, c + 1.8) } else { (c, c + 2.0, c - 2.0, c - 1.8) });
        v.push((c, c, c, c));
        let r = with_vol(bars(&v), 1_000_000.0);
        let x = Ctx::new(&r, &r, &[]);
        let i = r.len() - 2;
        let a = atr(&x, 5)[i];
        let lim = if short { ((x.h[i] + a * 0.75) / 0.01).round() * 0.01 } else { ((x.l[i] - a * 0.75) / 0.01).round() * 0.01 };
        *v.last_mut().unwrap() = next(lim);
        (with_vol(bars(&v), 1_000_000.0), lim)
    }
    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<Trade> {
        let x = Ctx::new(r, r, &[]);
        run(id, &x, 0, r.len() - 1, &ps(kv)).unwrap()
    }

    #[test]
    fn long_limit_fill_and_close_exit() {
        let (r, lim) = scene(false, |m| (m + 1.0, m + 3.0, m - 1.0, m + 2.0));
        let t = go(LONG_NO_ADX, &r, &[]);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (r.len() - 1, r.len() - 1));
        assert!((t[0].legs[0].px - lim).abs() < 1e-9); // open above the limit: filled at the limit
        assert!((t[0].exit_px - (lim + 2.0)).abs() < 1e-9);
        // the low does not reach the limit: no trade
        let (r, _) = scene(false, |m| (m + 1.0, m + 3.0, m + 0.5, m + 2.0));
        assert!(go(LONG_NO_ADX, &r, &[]).is_empty());
    }

    #[test]
    fn gap_below_limit_buys_open_plus_quarter_and_buffer_exit() {
        let (r, lim) = scene(false, |m| (m - 2.0, m + 3.0, m - 3.0, m + 1.0));
        let t = go(LONG_NO_ADX_BUFFER, &r, &[]);
        assert_eq!(t.len(), 1);
        let epx = (lim - 2.0) * 1.0025;
        assert!((t[0].legs[0].px - epx).abs() < 1e-9);
        assert!((t[0].exit_px - (lim + 1.0) * (1.0 - 0.004)).abs() < 1e-9);
        // hard stop 1%: the low (limit - 3) is below entry x 0.99
        let t = go(LONG_NO_ADX_BUFFER, &r, &[("hard_sl", 1.0)]);
        assert!((t[0].exit_px - epx * 0.99).abs() < 1e-9);
        // closing at the low: sell at close + 1% (kept within the high)
        let (r, lim) = scene(false, |m| (m, m + 3.0, m - 1.0, m - 1.0));
        let t = go(LONG_BUFFER, &r, &[("adx_lvl", 0.0)]);
        assert_eq!(t.len(), 1);
        assert!((t[0].exit_px - (lim - 1.0) * 1.01).abs() < 1e-9);
        // a lower-circuit bar (open = low = close) takes no entry
        let (r, _) = scene(false, |m| (m - 1.0, m + 2.0, m - 1.0, m - 1.0));
        assert!(go(LONG_NO_ADX_BUFFER, &r, &[]).is_empty());
    }

    #[test]
    fn short_limit_above_high() {
        let (r, lim) = scene(true, |m| (m - 1.0, m + 1.0, m - 3.0, m - 2.0));
        let t = go(SHORT_BUFFER, &r, &[("adx_lvl", 0.0)]);
        assert_eq!(t.len(), 1, "limit {lim}");
        assert_eq!(t[0].dir, -1.0);
        assert!((t[0].legs[0].px - lim).abs() < 1e-9);
        assert!((t[0].exit_px - (lim - 2.0) * 1.004).abs() < 1e-9);
        let (r, _) = scene(true, |m| (m - 1.0, m - 0.5, m - 3.0, m - 2.0));
        assert!(go(SHORT_BASIC, &r, &[("adx_lvl", 0.0)]).is_empty());
    }

    #[test]
    fn volume_filter_needs_volume() {
        let (r, _) = scene(false, |m| (m + 1.0, m + 3.0, m - 1.0, m + 2.0));
        let r: Vec<Row> = r.into_iter().map(|mut b| { b.volume = Some(100.0); b }).collect();
        assert!(go(LONG_NO_ADX, &r, &[]).is_empty());
        assert_eq!(go(LONG_NO_ADX, &r, &[("vol_flt", 0.0)]).len(), 1);
        // the bear version needs the close below its 200-day average
        assert!(go(LONG_BEAR, &r, &[("vol_flt", 0.0), ("adx_lvl", 0.0)]).is_empty());
    }

    #[test]
    fn every_strategy_runs() {
        let r = with_vol(wave(400), 2_000_000.0);
        for bench in [false, true] {
            let mut x = Ctx::new(&r, &r, &[]);
            if bench {
                for k in ["Open", "High", "Low", "Close"] {
                    x.extra.insert(format!("BENCH.{k}"), r.iter().map(|b| b.close * 0.9).collect());
                }
            }
            for id in IDS {
                for kv in [vec![], vec![("index_flt", 1.0), ("adx_lvl", 0.0)]] {
                    let t = run(id, &x, 50, r.len() - 1, &ps(&kv)).unwrap();
                    assert!(t.iter().all(|t| t.exit_i == t.legs[0].i && t.legs[0].i >= 50), "{id}");
                }
            }
        }
        assert_eq!(catalogue().len(), IDS.len());
        for id in IDS {
            assert!(params(id).is_some() && family_of(id).is_some());
        }
    }

    #[test]
    fn open_equal_to_the_limit_fills_at_the_limit() {
        // the limit rounds to 104.10 but carries a double-precision residue; the open is 104.10: AmiBroker sees them
        // equal (32-bit prices) and fills at the limit, not at the open + 0.25%
        let lim = (104.1f64 / 0.01).round() * 0.01;
        assert!((104.1f64 as f32) == (lim as f32));
        assert!(!((104.1f64 as f32) < (lim as f32)));
    }
}
