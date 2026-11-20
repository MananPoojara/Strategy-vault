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
/// The 6-hour F&O intraday versions (1-minute ieodfo data; admitted by `instrument_policy::intraday_rule`, not as cash
/// stock rules, so they are kept out of `IDS`).
pub const INTRADAY_IDS: [&str; 2] = [FO6H_ROC_ALL, FO6H_TRIGGER];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Nick Radge", session: 0, dir, status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const FLT: &str = "F|Setup bar: Close between 10 and 5,000 (short AFLs: 60,000)";
const FLT_2: &str = "F|Setup bar: 50-bar EMA of volume > 500,000";
const FLT_3: &str = "F|Setup bar: not a lower-circuit bar closing less than 6% down (short: not an upper-circuit bar, Open = High = Close to 2 decimals, less than 6% up)";
const FLT_4: &str = "F|Optional: NIFTY 50 above its 100-day average (off)";
const FLT_5: &str = "F|Optional: turnover filter (off)";
const LONG_E: &str = "E|Next bar, not a lower-circuit bar";
const LONG_E_2: &str = "E|Buy limit = setup Low − 0.75 × ATR(5), rounded to the tick (0.01; 0.005 below 2; 0.001 below 0.10)";
const LONG_E_3: &str = "E|Filled only if the bar's Low reaches it";
const LONG_E_4: &str = "E|Fill at Open + 0.25% when the Open is below the limit, else at the limit";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(LONG_BASIC, "Nick Radge Stretch Long, Exit at Close", "Long", &[
            "F|Setup bar: Close in the lower half of the day's range", "F|Setup bar: ADX(5) > 40", "F|Setup bar: Close > its 200-day average", FLT, FLT_2, FLT_3, FLT_4, FLT_5, LONG_E, LONG_E_2, LONG_E_3, LONG_E_4,
            "X|Sell at the entry bar's Close",
        ], concat!("INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct.afl (Nick radge analysis/LONG_ with Circuit filter), coded as the AFL is written. Same per-stock rules: NICK RADGE_LONG ONLY_Without ROC (only the ranking is removed) and the 'trash' copy.",
            " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(LONG_NO_ADX, "Nick Radge Stretch Long, No ADX, Exit at Close", "Long", &[
            "F|Setup bar: Close in the lower half of the day's range", "F|Setup bar: Close > its 200-day average", "F|No ADX condition", FLT, FLT_2, FLT_3, FLT_4, FLT_5, LONG_E, LONG_E_2, LONG_E_3, LONG_E_4,
            "X|Sell at the entry bar's Close",
        ], concat!("NICK RADGE_LONG ONLY_Without ADX & ROC/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct without adx n roc.afl, coded as the AFL is written.",
            " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; ranking and portfolio limits not modelled (it has no PositionScore; Maximum Open Positions 10). The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(LONG_BUFFER, "Nick Radge Stretch Long, Exit Buffer + Hard SL", "Long", &[
            "F|Setup bar: Close in the lower half of the day's range", "F|Setup bar: ADX(5) > 40", "F|Setup bar: Close > its 200-day average", FLT, FLT_2, FLT_3, FLT_4, FLT_5, LONG_E, LONG_E_2, LONG_E_3, LONG_E_4,
            "X|Sell at the entry bar's Close − exit buffer % (0.4)",
            "X|On a bar closing at its Low: sell at Close + 1% (kept within the bar's High)",
            "X|Optional hard stop % (default 0 = off), live on the entry bar",
        ], concat!("NICK RADGE LONG only_With Hard SL & exit buffer/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct n hard sl N EXIT BUFFER.afl, coded as the AFL is written. Same per-stock rules: the copy at the folder root and the three copies under Nick radge analysis (fo / NSE 500C&P; one sets the hard SL with Optimize, default 0).", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table. A hard stop and the close exit on the same bar: the stop is taken first.")),
        def(LONG_NO_ADX_BUFFER, "Nick Radge Stretch Long, No ADX, Exit Buffer + Hard SL", "Long", &[
            "F|Setup bar: Close in the lower half of the day's range", "F|Setup bar: Close > its 200-day average", "F|No ADX condition", FLT, FLT_2, FLT_3, FLT_4, FLT_5, LONG_E, LONG_E_2, LONG_E_3, LONG_E_4,
            "X|As the exit-buffer version: Close − 0.4%",
            "X|On a bar closing at its Low: Close + 1%",
            "X|Optional hard stop % (off)",
        ], concat!("NICK RADGE_LONG only_Without ADX & with ROC/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct without ADX.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(LONG_SPS, "Nick Radge Stretch Long, Low-Close Exit + Hard SL (SPS)", "Long", &[
            "F|Setup bar: Close in the lower half of the day's range", "F|Setup bar: ADX(5) > 40", "F|Setup bar: Close > its 200-day average", FLT, FLT_2, FLT_3, FLT_4, FLT_5, LONG_E, LONG_E_2, LONG_E_3, LONG_E_4,
            "X|Sell at the entry bar's Close",
            "X|On a bar closing at its Low: sell at Close + 1% (kept within the bar's High)",
            "X|Optional hard stop % (default 0 = off), live on the entry bar",
        ], concat!("Nick radge analysis/LONG_ with Circuit filter/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct n hard sl (With SPS).afl, coded as the AFL is written (same per-stock rules: its '- Copy' and the NSE 500C&P/WITHOUT EXIT BUFFER copy). This AFL does not set AllowSameBarExit itself (it is commented out); the same-bar exit is taken as enabled in the team's settings, as in every other AFL of the family.", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(LONG_BEAR, "Nick Radge Stretch Long for Bear Market, Exit at Close", "Long", &[
            "F|Setup bar: Close in the lower half of the day's range", "F|Setup bar: ADX(5) > 40", "F|Setup bar: Close < its 200-day average (BELOW)", FLT, FLT_2, FLT_3, FLT_4, FLT_5, LONG_E, LONG_E_2, LONG_E_3, LONG_E_4,
            "X|Sell at the entry bar's Close",
        ], concat!("NICK RADGE LONG only_For BEAR mkt_With Hard SL & exit buffer/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct for bear mkt.afl, coded as the AFL is written: despite the folder name the AFL has no hard stop or exit buffer (both commented out) and sells at the close.", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(LONG_ATHC, "Nick Radge Stretch Long on All-Time-High Stocks", "Long", &[
            "F|ATHC regime: on from a Close above every earlier Close",
            "F|ATHC regime ends when the 10% Close swing turns down",
            "F|The setup bar's regime must be on",
            "F|Setup bar: Close in the lower half of the day's range", "F|Setup bar: ADX(5) > 40", "F|Setup bar: Close > its 200-day average", FLT, FLT_2, FLT_3, FLT_4, FLT_5, LONG_E, LONG_E_2, LONG_E_3, LONG_E_4,
            "X|As the exit-buffer version: Close − 0.4%",
            "X|On a bar closing at its Low: Close + 1%",
            "X|Optional hard stop % (off)",
        ], concat!("NICK RADGE on ATH stocks_LONG only/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct FOR ATHC STOCK (With Hrd SL & Exit buffer)..afl, coded as the AFL is written (same rules: FO ATH AFL Check/..._v1 and the NSE 500C&P ATH-basket copy).", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(LONG_ATHC_OLD, "Nick Radge Stretch Long on All-Time-High Stocks (Old Version)", "Long", &[
            "F|ATHC regime as in the ATH AFL, but read one bar before the setup bar",
            "F|Setup bar: Close in the lower half of the day's range", "F|Setup bar: ADX(5) > 40", "F|Setup bar: Close > its 200-day average", FLT, FLT_2, FLT_3, FLT_4, FLT_5, LONG_E, LONG_E_2, LONG_E_3, LONG_E_4,
            "X|As the exit-buffer version: Close − 0.4%",
            "X|On a bar closing at its Low: Close + 1%",
            "X|Optional hard stop % (off)",
        ], concat!("FO ATH AFL Check/INTRADAY LONG SYSTEM FOR INDIAN MKT with circuit flt 5pct FOR ATHC STOCK (With Hrd SL & Exit buffer)_Old Version..afl, coded as the AFL is written; it differs from v1 only in `Ref(ATHCBUYSIG, -1)`.", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 - 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default) uses NIFTY 50 (the market benchmark) for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(SHORT_BASIC, "Nick Radge Stretch Short, Cover at Close", "Short", &[
            "F|Setup bar: Close in the UPPER half of the day's range (as written)", "F|Setup bar: ADX(5) > 40", "F|Setup bar: Close > its 200-day average (as written)", FLT, FLT_2, FLT_3, FLT_4, FLT_5,
            "E|Next bar, not an upper-circuit bar",
            "E|Sell-short limit = setup High + 0.75 × ATR(5), rounded to 0.01",
            "E|Filled only if the bar's High reaches it",
            "E|Fill at Open − 0.25% when the Open is above the limit, else at the limit",
            "X|Cover at the entry bar's Close",
        ], concat!("Nick radge analysis/SHORT_With circuit filter/INTRADAY SHORT SYSTEM FOR INDIAN MKT with circuit  flt 5 pct.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 + 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default; long-side condition index above its average, as written) uses NIFTY 50 for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table.")),
        def(SHORT_BUFFER, "Nick Radge Stretch Short, Exit Buffer + Hard SL", "Short", &[
            "F|Setup bar: Close in the UPPER half of the day's range (as written)", "F|Setup bar: ADX(5) > 40", "F|Setup bar: Close > its 200-day average (as written)", FLT, FLT_2, FLT_3, FLT_4, FLT_5,
            "E|Next bar, not an upper-circuit bar",
            "E|Sell-short limit = setup High + 0.75 × ATR(5), rounded to 0.01",
            "E|Filled only if the bar's High reaches it",
            "E|Fill at Open − open sell buffer % (0.25) when the Open is above the limit, else at the limit",
            "X|Cover at the entry bar's Close + exit buffer % (0.4)",
            "X|On a bar closing at its High: cover at Close − 1% (kept within the bar's Low)",
            "X|Optional hard stop % (off), live on the entry bar",
        ], concat!("NICK RADGE_SHORT only_With Hard SL & exit buffer/INTRADAY SHORT SYSTEM FOR INDIAN MKT with circuit  flt 5 pct n hard sl n EXIT BUFFER.afl, coded as the AFL is written.", " Runs on one stock at a time; the team ran it over NSE 500 constituents (cash) and F&O stocks; the PositionScore ranking (100 + 3-day ROC of the setup bar), Maximum Open Positions (10) and the custom backtester's daily order cap are not modelled. The index filter (off by default; index above its average, as written) uses NIFTY 50 for the AFL's NSENIFTY. The turnover filter (off) is close x volume against the AFL's year table. A hard stop and the close exit on the same bar: the stop is taken first.")),
    ]
    .into_iter()
    .chain(fo6h_catalogue())
    .collect()
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
    if id == FO6H_ROC_ALL || id == FO6H_TRIGGER {
        return Some(fo6h_params());
    }
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
    if id == FO6H_ROC_ALL || id == FO6H_TRIGGER {
        return Some(fo6h_run(x, from, to, p, &mut Vec::new()));
    }
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

// ---- 6 hr F&O: ROC on all setup condition stocks (added 20-11-2026)
//
// "NICK RADGE 6 HR time frame  database_LONG only/INTRADAY LONG SYSTEM FOR INDIAN MKT on intraday data_FO_6 hr_ROC on
// all setup condition stocks.afl" (the copy under Nick radge analysis/.../WITHOUT EXIT BUFFER_INTRADAY 6 HR time
// frame_FO differs only in `S_Dist = Optimize("Stretch", 0.75, 0.75, 1.75, 0.25)`; same default 0.75).
//
// The daily setup (TimeFrameSet(inDaily)): Range = (C - L) / (H - L) < 0.5, ADX(5) > 40, C > MA(C, 200) and the
// price / average-volume / turnover filters; BuyLimit = round((L - ATR(5) x Stretch) / Tick) x Tick. It reaches the
// 6-hour candles through TimeFrameExpand (expandLast) and `Ref(.., -1)`, so every candle of day D reads day D-1's
// setup and limit. Buy = that setup AND TimeNum() < 151500; Sell = TimeNum() == 151459 OR the day's last candle;
// BuyPrice = Open + 0.25% when the candle opens below the limit, else the limit; SellPrice = Close; same-bar exit on;
// the custom backtester drops an entry whose limit is below the candle's low. No circuit filter (commented out).
//
// Candles (how AmiBroker built the team's 6-hour bars): 360-minute buckets anchored at the 09:15 trading start (09:15-
// 15:14 and 15:15-15:29), each stamped with the END time of its interval. Evidence: the team's workbook "FO
// Intraday_6 hr_Comm 0.05-Backtest.xlsx" (same folder, Old Topics XLSX) lists all 12,641 trades of its six runs with
// entry and exit at 15:14:59 on the same bar (# bars 1), and the AFL tests TimeNum() == 151459. So `TimeNum()` here is
// the candle's end time (start + 360 min - 1 s): the 09:15 candle reads 151459 (buy allowed, and sold at its 15:14
// close), the 15:15 candle 211459 (no buy). `bar_align` 0 (midnight grid: 06:00-11:59 / 12:00-17:59, end stamps
// 115959 / 175959) is offered; there the trade is held to the day's last candle.
pub const FO6H_ROC_ALL: &str = "ytnr_fo_6hr_roc_all";
// "...FO_6 hr_updated_Trigger first then ROC.afl" (same folder) is this AFL with `L <= Ref(BuyLimit, -1)` moved into
// Buy and the custom backtester commented out. On one symbol the trades are the same (the backtester's only per-symbol
// effect was dropping an entry whose limit is below the candle's low, which Buy now tests); the difference is which
// stocks are ranked for the 10 positions (only triggered ones), which is not modelled. So it shares this code.
pub const FO6H_TRIGGER: &str = "ytnr_fo_6h_trigger_roc";

fn fo6h_catalogue() -> Vec<StrategyDef> {
    let roc_all = StrategyDef {
        id: FO6H_ROC_ALL,
        name: "Nick Radge Stretch Long on 6-Hour F&O Bars, Exit at Close",
        group: "YT Strategies · Nick Radge",
        session: 0,
        dir: "Long",
        status: Status::Live,
        needs_volume: true,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "F|Daily setup bar: Close in the lower half of the day's range",
            "F|Daily setup bar: ADX(5) > 40",
            "F|Daily setup bar: Close > its 200-day average",
            "F|Close between 10 and 5,000; 50-bar EMA of 6-hour volume > 500,000 (on the day's last 6-hour candle)",
            "F|Optional: turnover filter (off)",
            "E|Next day's 09:15-15:14 candle (TimeNum < 151500)",
            "E|Buy limit = setup Low − 0.75 × ATR(5), rounded to the tick (0.01; 0.005 below 2; 0.001 below 0.10)",
            "E|Filled only if the candle's Low reaches it",
            "E|Fill at Open + 0.25% when the Open is below the limit, else at the limit",
            "X|Sell at the same candle's Close (15:14), TimeNum = 151459",
        ],
        notes: concat!(
            "NICK RADGE 6 HR time frame  database_LONG only/INTRADAY LONG SYSTEM FOR INDIAN MKT on intraday data_FO_6 hr_ROC on all setup condition stocks.afl, coded as the AFL is written with its Param defaults (the copy in Nick radge analysis/.../WITHOUT EXIT BUFFER_INTRADAY 6 HR time frame_FO differs only in Stretch by Optimize, same default 0.75).",
            " Runs on 6-hour candles of the selected ieodfo F&O futures (1-minute IEOD data): 360-minute bars from the 09:15 start (09:15-15:14 and 15:15-15:29), TimeNum read as the bar's END time, as the team's workbook FO Intraday_6 hr_Comm 0.05-Backtest.xlsx shows (every trade enters and exits at 15:14:59 on one bar). The daily setup uses the daily bars of the same minutes and reaches the candles with TimeFrameExpand's expandLast and Ref(-1) (the previous day's setup and limit).",
            " Not modelled (they only choose among stocks): PositionScore = Ref(100 - ROC(C, 3), -1) of the daily ROC (the 'ROC on all setup condition stocks' ranking), Maximum Open Positions (10) and the custom backtester's daily order cap. The index filter (off by default) is not offered: it is a 6-hour-bar NSENIFTY average computed outside the daily TimeFrameSet.",
            " Declared adaptation: the price, average-volume and turnover filters are computed before TimeFrameSet(inDaily) on 6-hour bars but used inside the daily timeframe, where AmiBroker pairs arrays of different timeframes by position; they are kept as 6-hour-bar values (close, EMA(V, 50) of 6-hour volume, V x C) and read on each day's last 6-hour candle (as TimeFrameCompress with compressLast would). ATR(5) is AmiBroker's (ab::atr); limit arithmetic in float32.",
            " Compared 20-11-2026, per stock, with the team's workbook FO Intraday_6 hr_Comm 0.05-Backtest.xlsx (trade list TL1: the Optimize copy at Stretch 1.25, run with ADX level 20 and the price / volume filters off, since MRF at 79,000 with 12,000 volume is traded): with those settings, on MRF, DIVISLAB, HCLTECH and SBIN, 154 of the 155 sheet trades are found on the same dates, 152 exits match to 0.01, and entries match to 0.01 in 108 and within 0.1% in 151. The remaining differences come from the minute data (for example SBIN 16-01-2018, 15:14 close 296.5 in the sheet against 296.4 here). The AFL defaults themselves (ADX 40, Stretch 0.75, filters on) are not in any team run."
        ),
        claim: None,
    };
    let trigger = StrategyDef {
        id: FO6H_TRIGGER,
        name: "Nick Radge Stretch Long on 6-Hour F&O Bars, Trigger First then ROC",
        notes: concat!(
            "NICK RADGE 6 HR time frame  database_LONG only/INTRADAY LONG SYSTEM FOR INDIAN MKT on intraday data_FO_6 hr_updated_Trigger first then ROC.afl, coded as the AFL is written with its Param defaults (the copy in Nick radge analysis/.../WITHOUT EXIT BUFFER_INTRADAY 6 HR time frame_FO is identical).",
            " It is the 'ROC on all setup condition stocks' AFL with L <= Ref(BuyLimit, -1) moved into Buy and the custom backtester commented out, so only stocks whose limit was reached are ranked by PositionScore = Ref(100 - ROC(C, 3), -1) for the 10 positions. On one symbol its trades are the same as that AFL's, and SV runs it with the same code: 6-hour candles from 09:15 (09:15-15:14 and 15:15-15:29), TimeNum read as the candle's END time, buy on the 09:15 candle at the limit (or Open + 0.25% when it opens below), sell at that candle's 15:14 close.",
            " Not modelled: the ROC ranking, Maximum Open Positions (10) and sizing. The index filter (off by default) is not offered. Same declared adaptation as the sibling id for the price / volume / turnover filters (6-hour-bar values read on each day's last candle).",
            " Not compared: no team workbook names this AFL; the sibling id's comparison (FO Intraday_6 hr_Comm 0.05-Backtest.xlsx) applies to the same per-stock trades. The AFL defaults (ADX 40, Stretch 0.75, price 10-5,000, volume filter on) give no trades on NIFTY / BANKNIFTY futures, whose prices are above 5,000."
        ),
        group: roc_all.group,
        session: 0,
        dir: "Long",
        status: Status::Live,
        needs_volume: true,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: roc_all.rules,
        claim: None,
    };
    vec![roc_all, trigger]
}

fn fo6h_params() -> Vec<ParamDef> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    vec![
        p("s_look", "Stretch lookback (ATR period)", 5.0, 1.0, 100.0, 1.0),
        p("s_dist", "Stretch (ATR multiple)", 0.75, 0.0, 100.0, 0.01),
        p("adx_n", "ADX period", 5.0, 5.0, 60.0, 5.0),
        p("adx_lvl", "ADX level", 40.0, 20.0, 60.0, 5.0),
        p("price_flt", "Price filter (0 off, 1 on)", 1.0, 0.0, 1.0, 1.0),
        p("min_sp", "Minimum share price", 10.0, 0.0, 1000.0, 0.01),
        p("max_sp", "Maximum share price", 5000.0, 0.0, 6_000_000.0, 0.01),
        p("vol_flt", "Average volume filter (0 off, 1 on)", 1.0, 0.0, 1.0, 1.0),
        p("min_avg_vol", "Minimum average volume (6-hour bars)", 500_000.0, 0.0, 10_000_000.0, 1000.0),
        p("vol_ema", "Volume EMA period (6-hour bars)", 50.0, 1.0, 200.0, 1.0),
        p("turnover_flt", "Turnover filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        // the team's 6-hour bars run from 09:15 (workbook trade stamps 15:14:59), so the default is the session grid
        ParamDef { def: 1.0, ..crate::intraday::align_param() },
    ]
}

/// AFL TimeNum() of a 6-hour candle stamped with the END time of its interval (start + 360 min - 1 s), HHMMSS.
fn fo6h_timenum(start_hhmm: u16) -> u32 {
    let start = (start_hhmm / 100) as u32 * 60 + (start_hhmm % 100) as u32;
    let end = ((start + 360) * 60 + 86_399) % 86_400; // seconds of day, minus one second
    end / 3600 * 10_000 + end % 3600 / 60 * 100 + end % 60
}

/// The daily limit price `round((L - Stretch * S_Dist) / Tick) * Tick` in AmiBroker's float32 arithmetic.
fn fo6h_limit(low: f64, stretch: f64, s_dist: f64) -> f64 {
    let tick: f32 = if low < 0.10 { 0.001 } else if low < 2.0 { 0.005 } else { 0.01 };
    let bl = (low as f32) - (stretch as f32) * (s_dist as f32);
    ((bl / tick).round() * tick) as f64
}

/// The per-candle rules on built candles `b` (`di[k]` = daily index of candle k; `setup` / `limit` per daily bar);
/// returns (entry candle, exit candle, entry price, open at end).
fn fo6h_signals(b: &crate::intraday::Candles, di: &[usize], setup: &[bool], limit: &[Option<f64>], from: usize, to: usize) -> Vec<(usize, usize, f64, bool)> {
    // TimeFrameExpand(x, inDaily) (expandLast) at candle k: the day's value on its last candle, else the previous day's
    let expanded = |k: usize| -> Option<usize> { if b.last_of_day(k) { Some(di[k]) } else { di[k].checked_sub(1) } };
    let mut out = Vec::new();
    let mut armed = true; // ExRem(Buy, Sell)
    let mut pos: Option<(usize, f64)> = None;
    for k in 0..b.len() {
        if di[k] > to {
            break;
        }
        let tn = fo6h_timenum(b.hhmm[k]);
        let prev = k.checked_sub(1).and_then(expanded); // Ref(TimeFrameExpand(x), -1)
        let buy = prev.is_some_and(|d| setup[d]) && tn < 151_500;
        // Sell = TimeNum() == 151459 OR MyDay != Ref(MyDay, 1) (the last candle's Ref(.., 1) is Null: true)
        let sell = tn == 151_459 || b.last_of_day(k);
        let buy_ex = buy && armed;
        if buy_ex {
            armed = false;
        }
        if sell {
            armed = true;
        }
        if pos.is_none() && buy_ex && di[k] >= from {
            if let Some(lim) = prev.and_then(|d| limit[d]) {
                // custom backtester: sig.Price = -1 when LimitPrice < Low (float32 prices)
                if !((lim as f32) < (b.l[k] as f32)) {
                    let o = b.o[k];
                    let px = if (o as f32) < (lim as f32) { crate::books::ab::addmul(o, 0.25 / 100.0) } else { lim };
                    pos = Some((k, px));
                }
            }
        }
        if let Some((e, px)) = pos {
            if sell {
                out.push((e, k, px, false));
                pos = None;
            }
        }
    }
    if let Some((e, px)) = pos {
        if let Some(z) = (e..b.len()).rev().find(|&z| di[z] <= to) {
            out.push((e, z, px, true));
        }
    }
    out
}

fn fo6h_run(x: &Ctx, from: usize, to: usize, p: &Params, capture: &mut Vec<(usize, usize)>) -> Vec<Trade> {
    let schema = fo6h_params();
    let g = |k: &str| p.get(k).copied().filter(|v| v.is_finite()).unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Vec::new();
    }
    // admission (instrument_policy / main) already requires an ieodfo series with a minute file
    let Some(m) = crate::intraday::minutes(&format!("{}{}", crate::intraday::BASKET_PREFIX, x.r[0].ticker)) else { return Vec::new() };
    let align = if g(crate::intraday::ALIGN_KEY) >= 0.5 { crate::intraday::Align::Session } else { crate::intraday::Align::Midnight };
    let (b, di) = crate::books::ytsti::candles_for(&m, x, 360, align);
    // price / volume / turnover filters on 6-hour bars, read on each day's last candle
    let ev = ema(&b.v, g("vol_ema").round().max(1.0) as usize);
    let (pflt, vflt, tflt) = (g("price_flt").round() as i64 == 1, g("vol_flt").round() as i64 == 1, g("turnover_flt").round() as i64 == 1);
    let (min_sp, max_sp, min_vol) = (g("min_sp"), g("max_sp"), g("min_avg_vol"));
    let mut opt = vec![false; n];
    for k in 0..b.len() {
        if b.last_of_day(k) {
            let c = b.c[k];
            opt[di[k]] = (!pflt || (c >= min_sp && c <= max_sp)) && (!vflt || ev[k] > min_vol) && (!tflt || b.v[k] * c > volsig(&x.r[di[k]].date));
        }
    }
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let adx = dmi(x.r, g("adx_n").round().max(1.0) as usize).1;
    let adx_lvl = g("adx_lvl");
    let ma200 = ma(c, 200);
    let stretch = crate::books::ab::atr(h, l, c, g("s_look").round().max(1.0) as usize);
    let s_dist = g("s_dist");
    let setup: Vec<bool> = (0..n)
        .map(|i| (c[i] - l[i]) / (h[i] - l[i]) < 0.5 && adx[i] > adx_lvl && ma200[i].is_some_and(|m| c[i] > m) && opt[i])
        .collect();
    let limit: Vec<Option<f64>> = (0..n).map(|i| stretch[i].map(|a| fo6h_limit(l[i], a, s_dist))).collect();
    let mut trades = Vec::new();
    for (e, z, px, open) in fo6h_signals(&b, &di, &setup, &limit, from, to) {
        let reason = if open { "Open position at end of selected period".to_string() } else { format!("6-hour candle {:04}: sell at close", b.hhmm[z]) };
        trades.push(Trade { dir: 1.0, legs: vec![Leg { i: di[e], px, w: 1.0 }], exit_i: di[z], exit_px: b.c[z], reason, entry_kind: "limit", signal_i: di[e], open, exits: Vec::new() });
        capture.push((e, z));
    }
    crate::intraday::record(&m, &b, 360, align, &trades, capture);
    trades
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
        assert!(crate::books::ab::near_ab(t[0].legs[0].px, epx));
        assert!(crate::books::ab::near_ab(t[0].exit_px, (lim + 1.0) * (1.0 - 0.004)));
        // hard stop 1%: the low (limit - 3) is below entry x 0.99
        let t = go(LONG_NO_ADX_BUFFER, &r, &[("hard_sl", 1.0)]);
        assert!(crate::books::ab::near_ab(t[0].exit_px, epx * 0.99));
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
        assert_eq!(catalogue().len(), IDS.len() + INTRADAY_IDS.len());
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

    // ---- 6 hr F&O: ROC on all setup condition stocks (added 20-11-2026)
    fn six(hhmm: &[(u32, u16)], ohlc: &[(f64, f64, f64, f64)]) -> crate::intraday::Candles {
        crate::intraday::Candles {
            day: hhmm.iter().map(|v| v.0).collect(),
            hhmm: hhmm.iter().map(|v| v.1).collect(),
            o: ohlc.iter().map(|v| v.0).collect(),
            h: ohlc.iter().map(|v| v.1).collect(),
            l: ohlc.iter().map(|v| v.2).collect(),
            c: ohlc.iter().map(|v| v.3).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn fo6h_timenum_is_the_interval_end() {
        assert_eq!(fo6h_timenum(915), 151_459);
        assert_eq!(fo6h_timenum(1515), 211_459);
        assert_eq!(fo6h_timenum(600), 115_959);
        assert_eq!(fo6h_timenum(1200), 175_959);
        assert_eq!(fo6h_timenum(1800), 235_959);
        // limit rounded to the tick in float32
        assert!((fo6h_limit(100.0, 2.0, 0.75) - 98.5).abs() < 1e-5);
        assert!((fo6h_limit(1.5, 0.0101, 1.0) - 1.49).abs() < 1e-6);
        assert!((fo6h_limit(1.5, 0.0124, 1.0) - 1.49).abs() < 1e-6);
    }

    #[test]
    fn fo6h_buys_the_0915_candle_and_sells_its_close() {
        let days = [(0, 915), (0, 1515), (1, 915), (1, 1515), (2, 915), (2, 1515)];
        let bars = |d1: (f64, f64, f64, f64)| six(&days, &[(100., 101., 99., 100.), (100., 100., 100., 100.), d1, (97., 97., 97., 97.), (100., 101., 99., 100.), (100., 100., 100., 100.)]);
        let di = [0, 0, 1, 1, 2, 2];
        let setup = [true, false, false];
        let limit = [Some(98.0), None, None];
        // the low reaches the limit, open above it: filled at the limit, sold at the 15:14 close of the same candle
        let t = fo6h_signals(&bars((99., 100., 97.5, 99.5)), &di, &setup, &limit, 0, 2);
        assert_eq!(t, vec![(2, 2, 98.0, false)]);
        // opening below the limit: Open + 0.25% (AFL shape O + O * .25 / 100)
        let t = fo6h_signals(&bars((97., 99., 96.5, 98.)), &di, &setup, &limit, 0, 2);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].2, crate::books::ab::addmul(97., 0.0025));
        // the low stays above the limit: the backtester drops the entry
        assert!(fo6h_signals(&bars((99., 100., 98.5, 99.5)), &di, &setup, &limit, 0, 2).is_empty());
        // the 15:15 candle (TimeNum 211459) never buys, even with the setup expanded onto it
        let setup = [false, true, false];
        let limit = [None, Some(200.0), None];
        assert!(fo6h_signals(&bars((99., 100., 97.5, 99.5)), &di, &setup, &limit, 0, 2).iter().all(|t| t.0 == 4));
        // entries before the selected start are not taken
        assert!(fo6h_signals(&bars((99., 100., 97.5, 99.5)), &di, &[true, false, false], &[Some(98.0), None, None], 2, 2).is_empty());
    }

    #[test]
    fn fo6h_midnight_grid_holds_to_the_last_candle() {
        let b = six(&[(0, 600), (0, 1200), (1, 600), (1, 1200)], &[(100., 101., 99., 100.); 4]);
        let t = fo6h_signals(&b, &[0, 0, 1, 1], &[true, false], &[Some(99.5), None], 0, 1);
        assert_eq!(t, vec![(2, 3, 99.5, false)]);
    }

    #[test]
    fn fo6h_registered_as_intraday() {
        let d = catalogue().into_iter().find(|d| d.id == FO6H_ROC_ALL).unwrap();
        assert!(crate::instrument_policy::intraday_rule(&d));
        assert!(!IDS.contains(&FO6H_ROC_ALL));
        assert_eq!(family_of(FO6H_ROC_ALL).unwrap().2, "Nick Radge");
        let defs = params(FO6H_ROC_ALL).unwrap();
        let v = |k: &str| defs.iter().find(|p| p.key == k).unwrap().def;
        assert_eq!((v("s_look"), v("s_dist"), v("adx_n"), v("adx_lvl"), v("max_sp"), v("min_avg_vol"), v("bar_align")), (5., 0.75, 5., 40., 5000., 500_000., 1.));
        // no minute file for this ticker: no trades (readiness reports why before a run)
        let r = with_vol(wave(30), 1.0);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run(FO6H_ROC_ALL, &x, 0, r.len() - 1, &ps(&[])).unwrap().is_empty());
    }

    #[test]
    fn fo6h_trigger_first_shares_the_roc_all_rules() {
        let d = catalogue().into_iter().find(|d| d.id == FO6H_TRIGGER).unwrap();
        assert!(crate::instrument_policy::intraday_rule(&d));
        assert!(d.notes.contains("Trigger first then ROC.afl"));
        assert_eq!(family_of(FO6H_TRIGGER).map(|f| f.2), family_of(FO6H_ROC_ALL).map(|f| f.2));
        let key = |p: Vec<ParamDef>| p.into_iter().map(|p| (p.key, p.def.to_bits())).collect::<Vec<_>>();
        assert_eq!(key(params(FO6H_TRIGGER).unwrap()), key(params(FO6H_ROC_ALL).unwrap()));
        let r = with_vol(wave(30), 1.0);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run(FO6H_TRIGGER, &x, 0, r.len() - 1, &ps(&[])).unwrap().is_empty());
    }

    /// Opt-in check on the local data: TEST_NR6H_DAILY (a daily ieodfo CSV, file stem = symbol) and INTRADAY_DATA_DIR;
    /// prints the trades (filters off, Stretch / ADX level from TEST_NR6H_STRETCH / TEST_NR6H_ADX) for comparison with the
    /// team's workbook.
    #[test]
    fn fo6h_local_trades_when_configured() {
        let Ok(csv) = std::env::var("TEST_NR6H_DAILY") else { return };
        if std::env::var_os("INTRADAY_DATA_DIR").is_none() {
            return;
        }
        let sym = std::path::Path::new(&csv).file_stem().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&csv).unwrap();
        let rows: Vec<Row> = text.lines().skip(1).map(|line| {
            let v: Vec<_> = line.split(',').collect();
            Row { ticker: sym.clone(), date: v[0].into(), open: v[1].parse().unwrap(), high: v[2].parse().unwrap(), low: v[3].parse().unwrap(), close: v[4].parse().unwrap(), volume: v[5].parse().ok() }
        }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let st: f64 = std::env::var("TEST_NR6H_STRETCH").ok().and_then(|v| v.parse().ok()).unwrap_or(0.75);
        let adx: f64 = std::env::var("TEST_NR6H_ADX").ok().and_then(|v| v.parse().ok()).unwrap_or(40.0);
        let t = run(FO6H_ROC_ALL, &x, 0, rows.len() - 1, &ps(&[("s_dist", st), ("adx_lvl", adx), ("price_flt", 0.0), ("vol_flt", 0.0)])).unwrap();
        for t in &t {
            eprintln!("NR6H {sym} {} {:.4} {:.4}", rows[t.legs[0].i].date, t.legs[0].px, t.exit_px);
        }
    }
}
