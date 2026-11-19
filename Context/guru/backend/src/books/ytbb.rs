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
//! The three parmar-library AFLs ("Fresh ATHC en-ex", "Bollinger Band entry + fresh ATHC Exit by %swing or BB" and
//! "Bollinger Band entry + ATHC % range + TSL v2", coded by Parth Parmar, Aug 2022) contain only their entry / exit
//! conditions; everything else comes from the include file `parmar/Defaults.afl` (copied from the team's AmiBroker
//! "Formulas/Include" folder to jay-local/Include(X86)/parmar, file dated 18-11-2022, the same file as Include/1 and
//! the 64-bit folder's copy). Coded here from that file as written (`d_*` functions below):
//! - `D_setParamBacktest`: the default buy = not the last bar (Dead Stock ON) AND Close > penny price AND turnover
//!   AND the filter's entry AND not a bad candle (Close = High or High = Low; never on the last bar); the default
//!   sell = the last bar OR the filter's exit. Filter list: OFF | ATHC | SuperTrend | Bull/Bear;
//! - `D_initTrades`: the loop from bar 1. Flat: buy at the close and set the profit level (8%), hard SL (8%) and
//!   trailing SL (5% below the entry, then below the highest High / Close of the bars before the current one). No
//!   exit on the entry bar. In a position, in this order: a deferred exit, High > profit level, Low < hard SL,
//!   Close < trailing SL, the AFL's exit condition. Profit / hard-SL exits are deferred on an Open = Low = Close
//!   bar, the others on a bad candle (Close = Low or High = Low); a deferred exit sells at the close of the next
//!   bar that is not a bad candle, before any other test;
//! - a Null level in the loop compares as -1e10: with the profit % set to 0 the test `High > profit level` is
//!   always true, so the position is sold on the bar after entry (as written; the 8% default is not affected);
//! - Null in array expressions (a band or filter line that does not exist yet) gives no signal.
//! Not offered from Defaults.afl: the foreign (index) filter and the ROC ranking (both OFF by default), position
//! sizing, and absolute (non-%) stop values.
//!
//! Not coded: "BB Master/AFL/BB 100 MA UB 3% LB1%_Index Filter.afl" is a price-plot stub with no trading rules.
#![allow(dead_code)]
use crate::books::ytsw::{backtest, close_swing, compress, sma_filter, st_filter, swing, trade, Bars, Sig};
use crate::books::ytwh::{bound, loop_val, prec2};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::Row;

pub const BOOK: &str = "yt_bb";
pub const PLAIN: &str = "ytbb_plain";
pub const INDEX: &str = "ytbb_index_filter";
pub const V1: &str = "ytbb_index_filter_v1";
pub const V2: &str = "ytbb_index_filter_v2";
pub const MASTER: &str = "ytbb_master";
pub const FATHC: &str = "ytbb_fresh_athc";
pub const BB_FATHC: &str = "ytbb_bb_fresh_athc";
pub const BB_RANGE: &str = "ytbb_bb_athc_range_v2";
pub const IDS: [&str; 8] = [PLAIN, INDEX, V1, V2, MASTER, FATHC, BB_FATHC, BB_RANGE];
/// The AFLs built on parmar/Defaults.afl.
const PARMAR: [&str; 3] = [FATHC, BB_FATHC, BB_RANGE];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Bollinger Band", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const UNIVERSE: &str = "Runs on one stock at a time; the team ran it over NSE 500 / F&O cash stocks (C&P workbooks, 20 open positions, % of equity sizing); portfolio limits not modelled.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(PLAIN, "Bollinger Band Breakout (100, 3 SD / 1 SD)", &[
            "F|Upper band = 100-day MA of the Close + 3 standard deviations (BBandTop)",
            "F|Lower band = 100-day MA − 1 standard deviation (BBandBot)",
            "F|Buy filter: turnover (off by default)",
            "F|Not an upper-circuit bar",
            "F|High ≠ Close",
            "F|Close > penny price (0)",
            "F|Not the first bar",
            "F|Today's Close > upper band",
            "E|Buy at today's Close",
            "X|Sell at the Close when Close < lower band",
            "X|Unless Low = Close (LC bar)",
        ], "BB 100 MA UB 3% LB1%.afl, coded as the AFL is written (Buy / Sell arrays with ExRem, trades at the close). Runs on one stock at a time; the team ran it over NSE 500 / F&O cash stocks (C&P workbooks, 20 open positions, % of equity sizing); portfolio limits not modelled. The AFL's hard stop-loss parameter is declared but never used. Turnover filter: close x volume against the AFL's yearly Volsig table (the team stored turnover in OpenInt); off by default as in the AFL."),
        def(INDEX, "BB Breakout above All-Time-High Close, % Swing Exit (Index Filter)", &[
            "F|Upper band = 100-day MA + 3 SD; lower band = 100-day MA − 1 SD",
            "F|ATHC = highest Close so far",
            "F|21% Close swing",
            "F|Optional market filter (default NONE), in a Bull or Bear setting",
            "F|Filter choices: NIFTY 50 ST (monthly, 20, 1, mid point), SMA (200) or Swing% (bull 44 / bear 23)",
            "F|Flat (no position)",
            "F|Today's Close > previous bar's ATHC",
            "F|Today's Close > upper band",
            "F|Buy filter passes (turnover, circuit, H = C, penny, dead-stock look-ahead)",
            "F|Market filter allows it",
            "E|Buy at today's Close",
            "X|Sell at the Close when any of these holds:",
            "X|21% swing turns down",
            "X|Close < lower band",
            "X|Close at or below the hard stop (0 = off)",
            "X|Next 30 bars identical (dead stock)",
            "X|On an LC bar the sell waits for the next non-LC bar",
        ], "BB 100 MA UB 3% LB1%_Index Filter.afl, coded as the AFL is written (its loop; no ExRem). Runs on one stock at a time; the team ran it over NSE 500 / F&O cash stocks (C&P workbooks, 20 open positions); portfolio limits not modelled. Quirks kept: no entry before the 100-bar band exists (the team's trade lists never buy earlier), while a Null lower band never triggers an exit; the sell test `athcbuysig[i]=0` is an assignment (value 0), so the ATHC exit never fires; the dead-stock test uses Ref(.., +1..+30), i.e. it looks 30 bars ahead; the AFL's BB period default 100 lies outside its Param range (the default is used). The market filter uses NIFTY 50 (the market benchmark) in place of the AFL's NSENIFTY / NSE500; with it on, a buy needs FLTShort, which in the Bull setting means the index ST / Swing% is UP and in the Bear setting DOWN; with SMA it means the index close is below the previous period's average. The Swing% filter is taken on daily index closes (the AFL expands it with inDaily). Turnover = close x volume (team stored it in OpenInt), off by default."),
        def(V1, "BB Breakout, % Swing Exit, RSI / ADX / ROC Options (Index Filter V1)", &[
            "F|Upper band = 100-day MA + 3 SD",
            "F|Lower band toggle (default OFF: the lower band is 1)",
            "F|21% Close swing",
            "F|Optional swing-ATHC flag (default OFF)",
            "F|Optional filters (all off by default): RSI(14) > 30, ADX(8) > 20, ROC(1) > 1 or < −1",
            "F|Market filter as in the Index Filter AFL (default NONE)",
            "F|Flat (no position)",
            "F|Today's Close > upper band",
            "F|Buy filter passes",
            "F|Swing-ATHC flag is set",
            "E|Buy at today's Close",
            "X|Sell at the Close when any of these holds:",
            "X|21% swing turns down",
            "X|Close < lower band",
            "X|Close at or below the hard stop",
            "X|Next 30 bars identical",
        ], "BB 100 MA UB 3% LB1%_Index Filter_V1.afl, coded as the AFL is written. With the AFL's defaults the swing-ATHC option is OFF, so AthcBuySig stays 0 and the AFL never buys (set athc = 1 to trade). Runs on one stock at a time; the team ran it over NSE 500 / F&O cash stocks (20 open positions); portfolio limits not modelled. The optional PositionScore rank type (default Off) only orders stocks in a portfolio and is not offered. Quirks kept: the first sell test reads the dead-stock toggle (a number, 1 by default) as `Deadstock[i]`, so an LC bar does not delay the sell unless the dead-stock backtest is set to YES; `athcbuysig[i]=0` is an assignment (never true); the dead-stock test looks 30 bars ahead; no entry before the band exists (as in the team's trade lists). Market filter: NIFTY 50 in place of the AFL's index. ADX is Wilder's. Turnover = close x volume, off by default."),
        def(V2, "BB Breakout, Near-ATHC Flag and Trailing Stop (Index Filter V2)", &[
            "F|As the Master AFL",
            "F|Swing-ATHC flag (default OFF) is also set while the Close is within 21% below the highest Close",
            "F|Optional trailing stop (default 0 = off): highest High (or Close) since entry − trail %",
            "F|Flat (no position)",
            "F|Today's Close > upper band",
            "F|Buy filter passes",
            "F|Swing-ATHC flag is set",
            "E|Buy at today's Close",
            "X|Sell at the Close when any of these holds:",
            "X|21% swing turns down",
            "X|Close < lower band",
            "X|Close at or below the hard or trailing stop",
            "X|Next 30 bars identical",
        ], "BB 100 MA UB 3% LB1%_Index Filter_V2.afl, coded as the AFL is written. With the AFL's defaults the swing-ATHC option is OFF, so AthcBuySig stays 0 and the AFL never buys (set athc = 1 to trade). Runs on one stock at a time; the team ran it over cash stocks (Optim_Cash workbooks, 20 open positions); portfolio limits not modelled. The trailing stop is checked against the previous bar's level and raised only on bars without an exit signal, as the loop does; the AFL tests TS == \"Cl\" against the list item \"CL\" (taken as the close trail). RSI / ADX / ROC are evaluated on daily bars (the AFL computes the conditions inside TimeFrameSet without expanding them). Market filter: NIFTY 50 in place of the AFL's index. Quirks as in the Master AFL. Turnover = close x volume, off by default."),
        def(MASTER, "BB Master (Index Filter)", &[
            "F|Upper band = 100-day MA + 3 SD",
            "F|Lower band toggle (default OFF: the lower band is 1)",
            "F|21% Close swing",
            "F|Optional swing-ATHC flag (default OFF)",
            "F|Optional filters (all off by default): RSI(14) > 30, ADX(8) > 20, ROC(1) > 1 or < −1",
            "F|Market filter as in the Index Filter AFL (default NONE)",
            "F|Flat (no position)",
            "F|Today's Close > upper band",
            "F|Buy filter passes",
            "F|Swing-ATHC flag is set",
            "E|Buy at today's Close",
            "X|Sell at the Close when any of these holds:",
            "X|21% swing turns down",
            "X|Close < lower band",
            "X|Close at or below the hard stop",
            "X|Next 30 bars identical",
            "X|On an LC bar the sell waits",
        ], "BB_Master_IndexFilter.afl, coded as the AFL is written. With the AFL's defaults the swing-ATHC option is OFF, so AthcBuySig stays 0 and the AFL never buys (set athc = 1 to trade). Runs on one stock at a time; the team ran it over cash stocks (Optim_Cash workbooks, 20 open positions); portfolio limits not modelled. RSI / ADX / ROC are evaluated on daily bars (the AFL computes the conditions inside TimeFrameSet(monthly by default) without expanding them). Quirks kept: `athcbuysig[i]=0` is an assignment (never true); the dead-stock test looks 30 bars ahead; no entry before the band exists (as in the team's trade lists). Market filter: NIFTY 50 in place of the AFL's index. Turnover = close x volume, off by default."),
        def(FATHC, "Fresh All-Time-High Close Entry (parmar Defaults)", &[
            "F|ATHC = highest Close so far; swing line = ATHC − 17%",
            "F|Fresh ATHC run: the last new ATHC is more recent than the last Close cross below the swing line",
            "F|No run before the first cross below the swing line",
            "F|Flat (no position)",
            "F|First bar of a fresh ATHC run (a new ATHC after a 17% swing down)",
            "F|Not a bad candle: Close ≠ High and High ≠ Low",
            "F|Not the last bar; Close > penny price (0); turnover filter (off)",
            "E|Buy at today's Close",
            "X|From the bar after entry, the first of these (in this order):",
            "X|High > entry + 8%: sell at that level, or at the Open − 0.25% when the Open is at or above it",
            "X|Low < entry − 8%: sell at that level, or near the Open when the Open is below it",
            "X|Close < trailing SL (5% below the highest High since entry, up to the previous bar): sell at the Close",
            "X|Close < swing line (ATHC − 17%): sell at the Close",
            "X|An exit on an Open = Low = Close bar (profit / hard SL) or a Close = Low / High = Low bar (others) waits for the next bar that is not one",
        ], concat!("Fresh ATHC en-ex.afl, coded as the AFL is written, with the rules of its include file parmar/Defaults.afl (see the module notes; the include folder was supplied on 17-11-2026). The AFL sets the Defaults filter to ATHC; with another filter choice its fresh-ATHC array stays empty and it never buys. ", "Runs on one stock at a time; the team's universe for this AFL is not recorded (Defaults.afl: 20 open positions, 5% of equity, 0.65 brokerage); portfolio limits and the optional ROC ranking are not modelled. With Profit % 0 the profit level is Null; inside the loop a comparison with Null is false (AmiBroker, SV-Audit), so the profit exit does not fire. Not compared with the team's results.")),
        def(BB_FATHC, "Bollinger Band Entry, Exit below Lower Band or by Defaults Filter (parmar Defaults)", &[
            "F|Upper band = 20-day MA of the Close + 3 standard deviations; lower band = 20-day MA − 1 SD",
            "F|Defaults filter (default OFF): ATHC (fresh ATHC run / 17% swing), SuperTrend or Bull/Bear swing",
            "F|Flat (no position)",
            "F|Today's Close > upper band",
            "F|Not a bad candle: Close ≠ High and High ≠ Low",
            "F|Not the last bar; Close > penny price (0); turnover filter (off); filter entry",
            "E|Buy at today's Close",
            "X|From the bar after entry, the first of these (in this order):",
            "X|High > entry + 8%: sell at that level, or at the Open − 0.25% when the Open is at or above it",
            "X|Low < entry − 8%: sell at that level, or near the Open when the Open is below it",
            "X|Close < trailing SL (5% below the highest High since entry, up to the previous bar): sell at the Close",
            "X|Close < lower band (BB Bottom exit ON), or the filter's exit: sell at the Close",
            "X|An exit on an Open = Low = Close bar (profit / hard SL) or a Close = Low / High = Low bar (others) waits for the next bar that is not one",
        ], concat!("Bollinger Band entry + fresh ATHC Exit by %swing or BB .afl, coded as the AFL is written, with the rules of its include file parmar/Defaults.afl (see the module notes). The Defaults filter is OFF by default in this AFL (set d_filter = 1 for the ATHC filter its name refers to). Quirk kept: the AFL's 'ATHC Filter' exit toggle calls D_disableFilterExit() after the default sell array is already built, so it changes nothing and is not offered. ", "Runs on one stock at a time; the team's universe for this AFL is not recorded (Defaults.afl: 20 open positions, 5% of equity, 0.65 brokerage); portfolio limits and the optional ROC ranking are not modelled. With Profit % 0 the position is sold on the bar after entry (Null level, as written). Not compared with the team's results.")),
        def(BB_RANGE, "Bollinger Band Entry within ATHC % Range (v2, parmar Defaults)", &[
            "F|Upper band = 20-day MA of the Close + 3 standard deviations; lower band = 20-day MA − 1 SD",
            "F|ATHC = highest Close so far; range line = ATHC − 20%",
            "F|Flat (no position)",
            "F|Today's Close > upper band",
            "F|Today's Close > range line",
            "F|Not a bad candle: Close ≠ High and High ≠ Low",
            "F|Not the last bar; Close > penny price (0); turnover filter (off); Defaults filter (OFF)",
            "E|Buy at today's Close",
            "X|From the bar after entry, the first of these (in this order):",
            "X|High > entry + 8%: sell at that level, or at the Open − 0.25% when the Open is at or above it",
            "X|Low < entry − 8%: sell at that level, or near the Open when the Open is below it",
            "X|Close < trailing SL (5% below the highest High since entry, up to the previous bar): sell at the Close",
            "X|Close < range line, or Close < lower band (BB Bottom exit ON), or the filter's exit: sell at the Close",
            "X|An exit on an Open = Low = Close bar (profit / hard SL) or a Close = Low / High = Low bar (others) waits for the next bar that is not one",
        ], concat!("Bollinger Band entry + ATHC % range + TSL v2.afl. Declared adaptation: this AFL (22-08-2022) includes <Defaults.afl>, <UtilityExtension.afl> and <PlotExtension.afl> from the root of the AmiBroker include folder, and neither supplied include folder has them there; the parmar/ copies (Defaults.afl of 18-11-2022, the earliest kept version is 07-09-2022) are used, so the Defaults rules the team ran in August 2022 may have differed. The AFL's own conditions are coded as written. An ATHC range of 0 here stands for the AFL's empty string (range test off). ", "Runs on one stock at a time; the team's universe for this AFL is not recorded; portfolio limits and the optional ROC ranking are not modelled. With Profit % 0 the position is sold on the bar after entry (Null level, as written). Not compared with the team's results.")),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if PARMAR.contains(&id) {
        let mut v = Vec::new();
        if id != FATHC {
            v.extend([
                p("bb_period", "Band average period", 20.0, 1.0, 200.0, 1.0),
                p("ub_sd", "Upper band SD multiplier", 3.0, 0.5, 3.5, 0.5),
                p("lb_sd", "Lower band SD multiplier", 1.0, 0.5, 3.5, 0.5),
                p("exit_bbb", "Exit below the lower band (0 off, 1 on)", 1.0, 0.0, 1.0, 1.0),
            ]);
        }
        if id == BB_RANGE {
            v.push(p("athc_range", "ATHC range % (0 off)", 20.0, 0.0, 100.0, 1.0));
        }
        v.extend([
            p("pb_pct", "Profit booking % (0: sold on the next bar, see notes)", 8.0, 0.0, 500.0, 0.5),
            p("hsl_pct", "Hard stop-loss % (0 off)", 8.0, 0.0, 100.0, 0.5),
            p("tsl_pct", "Trailing stop-loss % (0 off)", 5.0, 0.0, 100.0, 0.5),
            p("tsl_close", "Trail from (0 high, 1 close)", 0.0, 0.0, 1.0, 1.0),
            p("exit_buff", "Exit buffer %", 0.25, 0.0, 10.0, 0.05),
            p("d_filter", "Defaults filter (0 off, 1 ATHC, 2 SuperTrend, 3 Bull/Bear)", if id == FATHC { 1.0 } else { 0.0 }, 0.0, 3.0, 1.0),
            p("athc_buff", "ATHC plus % buy (0 off)", 0.0, 0.0, 50.0, 0.25),
            p("athc_swing", "ATHC swing %", 17.0, 1.0, 100.0, 1.0),
            p("dst_input", "ST input (0 O, 1 H, 2 L, 3 C, 4 (O+C)/2, 5 (H+L)/2, 6 (HLC)/3, 7 (OHLC)/4)", 5.0, 0.0, 7.0, 1.0),
            p("dst_tf", "ST timeframe (0 chart = daily, 1 daily, 2 weekly, 3 monthly)", 0.0, 0.0, 3.0, 1.0),
            p("dst_period", "ST period", 10.0, 1.0, 200.0, 1.0),
            p("dst_mult", "ST multiplier", 3.0, 0.1, 20.0, 0.1),
            p("bbs_bull", "Bull/Bear swing: bull %", 16.0, 1.0, 100.0, 1.0),
            p("bbs_bear", "Bull/Bear swing: bear %", 16.0, 1.0, 100.0, 1.0),
            p("penny", "Penny stock price", 0.0, 0.0, 100.0, 1.0),
            p("turnover", "Turnover filter (0 no, 1 yes: close x volume)", 0.0, 0.0, 1.0, 1.0),
            p("dead_bt", "Dead stock (0 off, 1 on: no buy on the last bar)", 1.0, 0.0, 1.0, 1.0),
        ]);
        return Some(v);
    }
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

// ---------------------------------------------------------------- parmar/Defaults.afl (D_ functions)

/// AFL Null as a loop value.
const NUL: f64 = f64::NAN; // AFL Null in a loop: comparisons false (SV-Audit)
fn nul_opt(v: f64) -> Option<f64> {
    if v <= -1e9 || !v.is_finite() { None } else { Some(v) }
}

/// Bars of the timeframe `tf` ('D', 'W', 'M') as rows (with their opens), and the daily-to-period map.
pub(crate) fn period_rows(x: &Ctx, tf: char) -> (Vec<Row>, Bars) {
    let b = compress(x, tf);
    let mut first = vec![usize::MAX; b.c.len()];
    for (i, &g) in b.group.iter().enumerate() {
        if first[g] == usize::MAX {
            first[g] = i;
        }
    }
    let rows = (0..b.c.len())
        .map(|g| Row { ticker: x.r[b.last[g]].ticker.clone(), date: x.r[b.last[g]].date.clone(), open: x.o[first[g]], high: b.h[g], low: b.l[g], close: b.c[g], volume: None })
        .collect();
    (rows, b)
}

/// The AFL input list "Open|High|Low|Close|(Open+Close)/2|(High+Low)/2|(HLC)/3|(OHLC)/4" (index 0-7).
pub(crate) fn input_array(r: &[Row], code: i64) -> Vec<f64> {
    r.iter()
        .map(|b| match code {
            0 => b.open,
            1 => b.high,
            2 => b.low,
            3 => b.close,
            4 => (b.open + b.close) / 2.0,
            6 => (b.high + b.low + b.close) / 3.0,
            7 => (b.open + b.high + b.low + b.close) / 4.0,
            _ => (b.high + b.low) / 2.0,
        })
        .collect()
}

/// The Supertrend loop of D_SuperTrend / SupertrendV2: lineUp = calc + m x ATR, lineDown = calc - m x ATR, trend
/// up from the start; from bar `period`: up when C > lineUp[i-1], down when C < lineDown[i-1] (a Null line is
/// -1e10 in the loop); in an up trend lineDown only rises, in a down trend lineUp only falls. Returns the ST line
/// (lineDown in an up trend, lineUp in a down trend; None while it is Null).
pub(crate) fn st_loop(calc: &[f64], atr: &[Option<f64>], period: usize, mult: f64, c: &[f64]) -> Vec<Option<f64>> {
    let n = c.len();
    let mut up: Vec<f64> = (0..n).map(|i| atr[i].map_or(NUL, |a| calc[i] + mult * a)).collect();
    let mut dn: Vec<f64> = (0..n).map(|i| atr[i].map_or(NUL, |a| calc[i] - mult * a)).collect();
    let mut is_up = vec![true; n];
    for i in period.max(1)..n {
        is_up[i] = if c[i] > up[i - 1] { true } else if c[i] < dn[i - 1] { false } else { is_up[i - 1] };
        if is_up[i] {
            dn[i] = dn[i].max(dn[i - 1]);
        } else {
            up[i] = up[i].min(up[i - 1]);
        }
    }
    (0..n).map(|i| nul_opt(if is_up[i] { dn[i] } else { up[i] })).collect()
}

/// D_freshRunATHC / D_freshATHC and the swing line: (fresh-run ATHC + buffer, the same frozen at the run's first
/// value, ATHC - swing %).
fn d_athc(c: &[f64], swing_pct: f64, buff_pct: f64) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<f64>) {
    let n = c.len();
    let mut ath = c.to_vec();
    for i in 1..n {
        ath[i] = ath[i].max(ath[i - 1]);
    }
    let athcs: Vec<f64> = ath.iter().map(|a| a - a * swing_pct / 100.0).collect();
    let (mut last_high, mut last_cross): (Option<usize>, Option<usize>) = (None, None);
    let mut run = vec![None; n];
    for i in 1..n {
        if ath[i] > ath[i - 1] {
            last_high = Some(i);
        }
        if athcs[i] > c[i] && athcs[i - 1] <= c[i - 1] {
            last_cross = Some(i);
        }
        // BarsSince(new ATHC) < BarsSince(cross below the swing line); Null (never yet) fails the test
        if let (Some(a), Some(b)) = (last_high, last_cross) {
            if a > b {
                run[i] = Some(ath[i] + ath[i] * buff_pct / 100.0);
            }
        }
    }
    let mut frozen = run.clone();
    for i in 1..n {
        if frozen[i].is_some() && frozen[i - 1].is_some() {
            frozen[i] = frozen[i - 1];
        }
    }
    (run, frozen, athcs)
}

/// D_BullBearSwing: the Bull/Bear swing line.
fn d_bbs(c: &[f64], bull: f64, bear: f64) -> Vec<Option<f64>> {
    let n = c.len();
    let mut up: Vec<f64> = (0..n).map(|i| if i == 0 { NUL } else { c[i - 1] + c[i - 1] * bull / 100.0 }).collect();
    let mut dn: Vec<f64> = (0..n).map(|i| if i == 0 { NUL } else { c[i - 1] - c[i - 1] * bear / 100.0 }).collect();
    let mut is_up = vec![true; n];
    for i in 1..n {
        is_up[i] = if c[i - 1] > up[i - 1] { true } else if c[i - 1] < dn[i - 1] { false } else { is_up[i - 1] };
        if is_up[i] {
            dn[i] = dn[i].max(dn[i - 1]);
        } else {
            up[i] = up[i].min(up[i - 1]);
        }
    }
    (0..n).map(|i| nul_opt(if is_up[i] { dn[i] } else { up[i] })).collect()
}

/// What D_setParamBacktest leaves: the default buy and sell arrays (the last-bar dead-stock sell is left to the
/// run's end mark) and D_ARR_FATHC (empty unless the filter is ATHC).
struct DDefaults {
    buy: Vec<bool>,
    sell: Vec<bool>,
    fathc: Vec<Option<f64>>,
}

fn d_defaults(x: &Ctx, g: &dyn Fn(&str) -> f64) -> DDefaults {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let mut fathc = vec![None; n];
    let (f_in, f_out): (Vec<bool>, Vec<bool>) = match gi("d_filter") {
        1 => {
            let buff = g("athc_buff");
            let (run, frozen, athcs) = d_athc(c, g("athc_swing"), buff);
            let cs: Vec<Option<f64>> = c.iter().map(|&v| Some(v)).collect();
            let fin = if buff != 0.0 { crate::books::yteng::cross(&cs, &frozen) } else { run.iter().map(|v| v.is_some()).collect() };
            fathc = frozen;
            (fin, (0..n).map(|i| c[i] < athcs[i]).collect())
        }
        2 => {
            let tf = match gi("dst_tf") { 2 => 'W', 3 => 'M', _ => 'D' };
            let (rows, b) = period_rows(x, tf);
            let period = g("dst_period").round().max(1.0) as usize;
            let atr: Vec<Option<f64>> = crate::connors::atr(&rows, period).into_iter().map(|v| if v.is_finite() { Some(v) } else { None }).collect();
            let pc: Vec<f64> = rows.iter().map(|r| r.close).collect();
            let st = st_loop(&input_array(&rows, gi("dst_input")), &atr, period, g("dst_mult"), &pc);
            // Ref(.., -1) on the filter timeframe, expandFirst: every bar reads the previous period's close and ST
            let prev = |i: usize| -> Option<(f64, f64)> { let k = b.group[i]; if k == 0 { None } else { st[k - 1].map(|s| (pc[k - 1], s)) } };
            ((0..n).map(|i| prev(i).map_or(false, |(pc, s)| pc > s)).collect(), (0..n).map(|i| prev(i).map_or(false, |(pc, s)| pc < s)).collect())
        }
        3 => {
            let bbs = d_bbs(c, g("bbs_bull"), g("bbs_bear"));
            ((0..n).map(|i| bbs[i].map_or(false, |b| c[i] > b)).collect(), (0..n).map(|i| bbs[i].map_or(false, |b| c[i] < b)).collect())
        }
        _ => (vec![true; n], vec![false; n]),
    };
    let dead_on = gi("dead_bt") == 1;
    let turn = turnover(x, gi("turnover") == 1);
    let penny = g("penny");
    let buy = (0..n).map(|i| !(dead_on && i + 1 == n) && c[i] > penny && turn[i] && f_in[i] && !d_bad_candle(h, l, c, i, true)).collect();
    DDefaults { buy, sell: f_out, fathc }
}

/// D_isBadCandle: Close = High (buy side) or Close = Low (sell side), or High = Low; never on the last bar.
fn d_bad_candle(h: &[f64], l: &[f64], c: &[f64], i: usize, buy: bool) -> bool {
    i + 1 != c.len() && ((if buy { c[i] == h[i] } else { c[i] == l[i] }) || h[i] == l[i])
}

/// D_initTrades (long side): `le` / `lx` are the AFL's entry and exit conditions (lx with its reason).
fn d_init_trades(x: &Ctx, le: &[bool], lx: &[Option<&'static str>], g: &dyn Fn(&str) -> f64) -> Sig4 {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let (pb_pct, hsl_pct, tsl_pct, buff) = (g("pb_pct"), g("hsl_pct"), g("tsl_pct"), g("exit_buff"));
    let tsl_close = g("tsl_close").round() as i64 == 1;
    let mut s = Sig4::new(n);
    let (mut pos, mut pend, mut sold_prev) = (false, false, false);
    let (mut pb, mut hsl, mut tsl, mut top, mut bp) = (NUL, NUL, NUL, NUL, 0.0f64);
    for i in 1..n {
        if sold_prev {
            pb = NUL;
            hsl = NUL;
            tsl = NUL;
            sold_prev = false;
        }
        if !pos {
            if le[i] {
                s.buy[i] = true;
                bp = c[i];
                if pb_pct != 0.0 {
                    pb = bp + bp * pb_pct / 100.0;
                }
                if hsl_pct != 0.0 {
                    hsl = bp - bp * hsl_pct / 100.0;
                }
                if tsl_pct != 0.0 {
                    top = bp;
                    tsl = top - top * tsl_pct / 100.0;
                }
                pos = true;
            }
            continue;
        }
        if !tsl.is_nan() {
            let prev = if tsl_close { c[i - 1] } else { h[i - 1] };
            if prev > top {
                top = prev;
                tsl = top - top * tsl_pct / 100.0;
            }
        }
        let circuit = o[i] == l[i] && l[i] == c[i];
        let bad = d_bad_candle(h, l, c, i, false);
        let mut sell: Option<(f64, &'static str)> = None;
        if pend {
            if !bad {
                sell = Some((c[i], "Deferred exit"));
                pend = false;
            }
        } else if h[i] > pb {
            if circuit {
                pend = true;
            } else {
                sell = Some((if o[i] < pb { pb } else { o[i] - o[i] * buff / 100.0 }, "Profit booking"));
            }
        } else if l[i] < hsl {
            if circuit {
                pend = true;
            } else {
                let px = if o[i] > hsl {
                    hsl
                } else if o[i] < hsl && o[i] == l[i] {
                    o[i]
                } else if o[i] < hsl && l[i] < o[i] - bp * buff / 100.0 {
                    o[i] - o[i] * buff / 100.0
                } else {
                    l[i]
                };
                sell = Some((px, "Hard SL"));
            }
        } else if c[i] < tsl {
            if bad { pend = true } else { sell = Some((c[i], "Trailing SL")) }
        } else if let Some(r) = lx[i] {
            if bad { pend = true } else { sell = Some((c[i], r)) }
        }
        if sell.is_some() {
            s.sell[i] = sell;
            pos = false;
            sold_prev = true;
        }
    }
    s
}

fn parmar_signals(x: &Ctx, id: &str, g: &dyn Fn(&str) -> f64) -> Sig4 {
    let c = &x.c;
    let n = c.len();
    let d = d_defaults(x, g);
    let filter_exit = |i: usize| if d.sell[i] { Some("Filter exit") } else { None };
    if id == FATHC {
        let le: Vec<bool> = (0..n).map(|i| d.buy[i] && i > 0 && d.fathc[i].is_some() && d.fathc[i - 1].is_none()).collect();
        let lx: Vec<Option<&'static str>> = (0..n).map(filter_exit).collect();
        return d_init_trades(x, &le, &lx, g);
    }
    let per = g("bb_period").round().max(1.0) as usize;
    let ub = bband(c, per, g("ub_sd"), true);
    let lb = bband(c, per, g("lb_sd"), false);
    let exit_bbb = g("exit_bbb").round() as i64 == 1;
    let range = if id == BB_RANGE { g("athc_range") } else { 0.0 };
    let mut ath = c.clone();
    for i in 1..n {
        ath[i] = ath[i].max(ath[i - 1]);
    }
    let line = |i: usize| ath[i] - ath[i] * range / 100.0;
    let le: Vec<bool> = (0..n).map(|i| d.buy[i] && ub[i].map_or(false, |u| c[i] > u) && (range == 0.0 || c[i] > line(i))).collect();
    let lx: Vec<Option<&'static str>> = (0..n)
        .map(|i| {
            filter_exit(i)
                .or(if range != 0.0 && c[i] < line(i) { Some("Below ATHC range") } else { None })
                .or(if exit_bbb && lb[i].map_or(false, |b| c[i] < b) { Some("Below lower band") } else { None })
        })
        .collect();
    d_init_trades(x, &le, &lx, g)
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
        FATHC | BB_FATHC | BB_RANGE => amibt(x, &parmar_signals(x, id, &g), from, to),
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
        FATHC => "ytbb_fresh_athc",
        BB_FATHC => "ytbb_bb_fresh_athc",
        BB_RANGE => "ytbb_bb_athc_range_v2",
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
    fn fresh_athc_run_starts_after_a_swing_down() {
        let c = [90.0, 95.0, 100.0, 82.0, 85.0, 101.0, 103.0, 99.0];
        let (run, frozen, athcs) = d_athc(&c, 17.0, 0.0);
        // no run before the first cross below ATHC - 17% (bar 3); the new ATHC on bar 5 starts one
        assert_eq!(run.iter().map(|v| v.is_some()).collect::<Vec<_>>(), vec![false, false, false, false, false, true, true, true]);
        assert_eq!((run[6], frozen[6], athcs[6]), (Some(103.0), Some(101.0), 103.0 * 0.83));
    }

    #[test]
    fn parmar_fresh_athc_entry_and_stops() {
        let v: Vec<_> = [90.0, 95.0, 100.0, 82.0, 85.0, 101.0, 103.0, 110.0, 111.0].iter().map(|&c| bar(c)).collect();
        let r = rows(&v);
        // buy on bar 5 at 101 (first bar of the run); profit level 109.08; bar 7 opens at 110, above it: Open - 0.25%
        let t = go(FATHC, &r, &[]);
        assert_eq!((t[0].0, t[0].1, t[0].2), (5, 101.0, 7));
        assert!((t[0].3 - (110.0 - 110.0 * 0.0025)).abs() < 1e-9);
        // profit % 0: the profit level stays Null and a comparison with Null is false (AFL loop), so profit booking
        // never fires; the trade is still open on the last bar
        let t = go(FATHC, &r, &[("pb_pct", 0.0)]);
        assert_eq!((t[0].0, t[0].2, t[0].4), (5, 8, true));
        // a first run bar closing at its high is a bad candle: no entry for that run
        let mut w = v.clone();
        w[5] = (100.0, 101.0, 99.0, 101.0);
        assert!(go(FATHC, &rows(&w), &[]).is_empty());
        // another Defaults filter leaves the fresh-ATHC array empty: no buys
        assert!(go(FATHC, &r, &[("d_filter", 0.0)]).is_empty());
        // hard SL: entry 101, level 92.92; a bar opening below it with Open = Low sells at the Open
        let mut w = v.clone();
        w[6] = (92.0, 93.0, 92.0, 92.5);
        let t = go(FATHC, &rows(&w), &[]);
        assert_eq!((t[0].2, t[0].3), (6, 92.0));
        // trailing SL 5% from the previous bars' highest High: bar 6 high 104 -> 98.8; bar 7 closes at 98
        let mut w = v.clone();
        w[7] = (99.0, 100.0, 97.0, 98.0);
        let t = go(FATHC, &rows(&w), &[]);
        assert_eq!((t[0].2, t[0].3), (7, 98.0));
    }

    #[test]
    fn parmar_bb_entry_and_deferred_exit() {
        let mut v: Vec<_> = (0..6).map(|_| bar(100.0)).collect();
        v.extend([bar(110.0), bar(111.0), bar(95.0), bar(96.0), bar(97.0)]);
        let kv = [("bb_period", 5.0), ("ub_sd", 1.0), ("lb_sd", 1.0), ("pb_pct", 500.0), ("hsl_pct", 0.0), ("tsl_pct", 0.0)];
        for id in [BB_FATHC, BB_RANGE] {
            let t = go(id, &rows(&v), &kv);
            assert_eq!(t[0], (6, 110.0, 8, 95.0, false), "{id}");
            // the exit bar closes at its low (bad candle): the sell waits for the next bar's close
            let mut w = v.clone();
            w[8] = (96.0, 97.0, 95.0, 95.0);
            let t = go(id, &rows(&w), &kv);
            assert_eq!((t[0].2, t[0].3), (9, 96.0), "{id}");
            // no exit below the lower band when that exit is off
            let mut off = kv.to_vec();
            off.push(("exit_bbb", 0.0));
            off.push(("athc_range", 0.0));
            assert!(go(id, &rows(&v), &off)[0].4, "{id}");
        }
        // v2: a breakout more than 20% below the highest close is not bought
        let mut w: Vec<_> = vec![bar(200.0)];
        w.extend(v.clone());
        assert!(go(BB_RANGE, &rows(&w), &kv).is_empty());
        assert_eq!(go(BB_RANGE, &rows(&w), &[kv[0], kv[1], kv[2], kv[3], kv[4], kv[5], ("athc_range", 0.0)])[0].0, 7);
        // Defaults Bull/Bear swing line
        let d = d_bbs(&[100.0, 100.0, 120.0, 121.0, 90.0, 91.0], 16.0, 16.0);
        assert_eq!(d[0], None);
        // up from the start; bar 3 follows a close above the bull line: the bear line rises to 120 x 0.84
        assert!((d[3].unwrap() - 100.8).abs() < 1e-9);
        // bar 5 follows a close (90) below the bear line: down, the line is the bull line 90 x 1.16
        assert!((d[5].unwrap() - 104.4).abs() < 1e-9);
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &PARMAR, &[("d_filter", 2.0), ("dst_tf", 3.0), ("dst_period", 3.0)]);
        smoke(run, &PARMAR, &[("d_filter", 3.0), ("tsl_close", 1.0), ("athc_buff", 1.0)]);
        smoke(run, &PARMAR, &[("d_filter", 1.0), ("athc_buff", 1.0), ("dead_bt", 0.0), ("turnover", 1.0)]);
        smoke(run, &IDS, &[("athc", 1.0), ("lb_on", 1.0), ("mkt_flt", 1.0), ("tsl_pct", 5.0), ("rsi_on", 1.0), ("hard_stop", 5.0)]);
        smoke(run, &[INDEX, V2], &[("mkt_flt", 1.0), ("iflt", 2.0), ("flt_tf", 3.0)]);
        smoke(run, &[INDEX, MASTER], &[("athc", 1.0), ("mkt_flt", 1.0), ("iflt", 3.0), ("scene", 2.0)]);
    }
}
