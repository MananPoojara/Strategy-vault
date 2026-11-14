//! YT Strategies · USD INR: the team's AmiBroker AFLs in "Old Topics/USD INR". Every AFL trades one index (NSENIFTY,
//! or BANKNIFTY in some runs) from signals on USDINR: a Supertrend on USDINR (SetForeign), or the price relative
//! USDINR / index against a ratio of their moving averages. Coded from the AFLs as written, with their default
//! settings; there is no book.
//!
//! Data: the external series USDINR (extra_data `USDINR.csv`: Date, Open, High, Low, Close -> keys USDINR.Open ..
//! USDINR.Close). Like AmiBroker's Foreign / SetForeign with the default fix-up, USDINR is aligned to the index's
//! dates: an index day without a USDINR bar takes the previous USDINR close as its Open, High, Low and Close. Index
//! bars before the first USDINR bar have no USDINR value; no signal is taken there. Without the series the strategies
//! give no trades (needs_series).
//!
//! AmiBroker conventions as in `ytsw.rs` / `ytwh.rs`: Supertrend = the AFL loop (`ytsw::supertrend`: mean of
//! ATR(1) over the period, Close against the previous bar's bands, band ratchets and flip resets, trend 1 during
//! warm-up), TimeFrameSet / TimeFrameExpand(expandLast) for weekly and monthly bars, ExRem'd array signals filled at
//! the close with stop-and-reverse for the PR AFLs, loop trades for the others. Live stop fills are kept inside the
//! bar's range (price bound checking). EMA starts from the simple average of its first n values.
#![allow(dead_code)]
use crate::books::ytmr::day_number;
use crate::books::ytsw::{backtest, calc_point, exrem, expand_last, true_range, Bars, Sig};
use crate::books::ytwh::{bound, collect, Ev};
use crate::connors::{wilder, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_usd";

// USDINR Supertrend AFLs ("Long", "Short", "Others" folders)
pub const ST_NIFTY: &str = "ytusd_st_nifty_close";
pub const ST_DOUBLE: &str = "ytusd_st_nifty_double_exit";
pub const ST_LIVE: &str = "ytusd_st_nifty_live";
pub const ST_SHORT_LIVE: &str = "ytusd_st_short_nifty_live";
pub const ST_ATR: &str = "ytusd_st_atr_trail";
pub const ST_PCT: &str = "ytusd_st_pct_trail_close";
pub const ST_PCT_LIVE: &str = "ytusd_st_pct_trail_live";
pub const ST_SHORT_PCT: &str = "ytusd_st_short_pct_trail_close";
pub const ST2_PT: &str = "ytusd_2st_pt_close";
pub const ST2_LIVE: &str = "ytusd_2st_live";
// "NIFTY USD PR TRADING IN NIFTY ONLY" AFLs
pub const PR_BASE: &str = "ytusd_pr_base";
pub const PR_PRF: &str = "ytusd_pr_prfilter";
pub const PR_PRF_R2: &str = "ytusd_pr_prfilter_regime2";
pub const PR_PRF_BB: &str = "ytusd_pr_prfilter_bullbear";
pub const PR_STF: &str = "ytusd_pr_stfilter";
pub const PR_STF_R2: &str = "ytusd_pr_stfilter_regime2";
pub const PR_STF_BB: &str = "ytusd_pr_stfilter_bullbear";
pub const PR_COMBO_LS: &str = "ytusd_pr_prfilter_combo_ls";
pub const PR_COMBO_SCAN: &str = "ytusd_pr_prfilter_combo_scan";
// "Long Only with USDINR & NSENIFTY Slow and Fast PR indicator" and "Long Only in NSENIFTY with PR indicator with
// Monthly ST as a Filter and Daily ST as a Indicator"
pub const SF_HSL: &str = "ytusd_sf_hsl";
pub const SF_ST: &str = "ytusd_sf_stflt";
pub const MST: &str = "ytusd_mst_dst";

pub const IDS: [&str; 22] = [
    ST_NIFTY, ST_DOUBLE, ST_LIVE, ST_SHORT_LIVE, ST_ATR, ST_PCT, ST_PCT_LIVE, ST_SHORT_PCT, ST2_PT, ST2_LIVE, PR_BASE, PR_PRF,
    PR_PRF_R2, PR_PRF_BB, PR_STF, PR_STF_R2, PR_STF_BB, PR_COMBO_LS, PR_COMBO_SCAN, SF_HSL, SF_ST, MST,
];

const BIG: f64 = 9_999_999.0;

/// The "Nifty ST" / "Filter ST" blocks that follow a USDINR block (TimeFrameRestore with no RestorePriceArrays)
/// are computed on the traded index, as AmiBroker documents TimeFrameRestore restoring the original symbol.
const SECOND_ST_ON_INDEX: bool = true;

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · USD INR", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &["USDINR.Close"], book: BOOK, rules, notes, claim: None }
}

const ST_NOTE: &str = "USDINR ST = the AFL Supertrend on USDINR (SetForeign), on the chosen time frame (1 daily, 2 weekly, 3 monthly) and calculation point (1 mid point, 2 close, 3 pivot). ST 1 = USDINR up (rupee weak), -1 = down. Position size, commission (0.65 %) and dead-stock / penny / turnover settings are stock-template options at their defaults (off); no forced exit on the last bar.";
const PR_NOTE: &str = "PR = USDINR close / index close; its line = MA(USDINR) / MA(index), a ratio of two moving averages (not an MA of the ratio). Average types: 0 EMA, 1 simple, 2 double, 3 triple, 4 'triangular' (as written: EMA of a simple average, both over Round((n+1.1)/2)), 5 weighted. Signals are ExRem'd arrays filled at the signal bar's close; a long and a short reverse on the same bar. Commission 0.65 % and sizing are template settings.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(ST_NIFTY, "Long Nifty when USDINR ST is down and Nifty ST is up", "Long", &[
            "F|USDINR ST (10, 2) is down and the Nifty ST (5, 4) is up",
            "F|Close > 0; not on the first bar",
            "E|Buy at the close",
            "X|Close < hard SL (2 % below entry), or the USDINR ST turns up, or the Nifty ST turns down: sell at the close",
        ], "LONG in Nifty if USDINR is in Sell (Without Filter).afl (Single confirmation in entry & single in exit). The Nifty ST is computed on the traded index. Buffer % is not used by this AFL."),
        def(ST_DOUBLE, "Long Nifty on USDINR ST down and Nifty ST up, exit on both", "Long", &[
            "F|USDINR ST down and Nifty ST up; the bar is not an upper circuit (High = Close)",
            "E|Buy at the close",
            "X|Close <= hard SL, or both the USDINR ST is up and the Nifty ST is down",
            "X|Sell at the close; on a lower-circuit bar (Low = Close) the sale waits for the next bar that is not",
        ], "LONG in Nifty ST if USDINR ST is in Sell (Double Confirmation, Long only).afl (ANUSHREE)."),
        def(ST_LIVE, "Long Nifty on USDINR ST down and Nifty ST up, hard SL live", "Long", &[
            "F|USDINR ST down and Nifty ST up; not an upper-circuit bar",
            "E|Buy at the close",
            "X|Low <= hard SL: sell at the SL, or at Open - buffer when the bar opens below it",
            "X|USDINR ST up or Nifty ST down: sell at the close",
            "X|On a lower-circuit bar the exit waits; it is taken on the next bar that is not a lower circuit, at Open - buffer",
        ], "LONG in Nifty if USDINR is in Sell (Without Filter) Hardsl @live.afl. The AFL's Nifty ST factor default (4) is above its Param maximum (3); the default is kept. The AFL does not expand the USDINR ST to daily bars; it is expanded here (no difference on the default daily time frame)."),
        def(ST_SHORT_LIVE, "Short Nifty on USDINR ST up and Nifty ST down, hard SL live", "Short", &[
            "F|USDINR ST up and Nifty ST down; not a lower-circuit bar",
            "E|Short at the close",
            "X|High >= hard SL (2 % above entry): cover at the SL, or at Open + buffer when the bar opens above it",
            "X|USDINR ST down or Nifty ST up: cover at the close",
            "X|On an upper-circuit bar the cover waits for the next bar that is not, at Open + buffer",
        ], "Short in Nifty if USDINR is in Buy (Without Filter).afl. The AFL's USDINR factor default (2) is below its Param minimum (3); the default is kept."),
        def(ST_ATR, "Long Nifty on USDINR ST down, ATR trailing stop", "Long", &[
            "F|USDINR ST down; not an upper-circuit bar",
            "E|Buy at the close; the ATR trail starts at Close - ATR(5) x 5",
            "X|Close <= hard SL, or USDINR ST up and Close <= the previous bar's ATR trail: sell at the close",
            "X|The trail = highest close - ATR (taken on the bar of that highest close) x multiplier, and never falls",
            "X|On a lower-circuit bar the sale waits for the next bar that is not",
        ], "Long only Single Confirmation in Entry & Double in exit.afl (DARSHIL + vikas). ATR = Wilder ATR on the index. The AFL's USDINR factor default (2) is below its Param minimum (3); the default is kept."),
        def(ST_PCT, "Long Nifty on USDINR ST down, % trailing stop at close", "Long", &[
            "F|USDINR ST down; not an upper-circuit bar",
            "E|Buy at the close",
            "X|Close <= hard SL (2 %), or Close <= the trail (5 % below the highest high since entry, today's high included), or USDINR ST up: sell at the close",
            "X|On a lower-circuit bar the sale is deferred; as written, the deferred sale fires on a later lower-circuit bar (Low = Close)",
        ], "LONG in Nifty if USDINR is in Sell (Without Filter) - Ver 2 @close.afl, - Close.afl and Others/... - Ver 2.afl (identical apart from Param steps). The deferred-sale condition is inverted in the AFL; kept as written. A % of 0 turns that stop off."),
        def(ST_PCT_LIVE, "Long Nifty on USDINR ST down, % trailing stop live", "Long", &[
            "F|USDINR ST down; not an upper-circuit bar",
            "E|Buy at the close",
            "X|Low <= hard SL: at the SL (or Open - buffer below it); Low <= the previous bar's trail: at the trail (or Open - buffer); USDINR ST up: at the close",
            "X|The trail = 5 % below the highest high since entry, updated after the exit check",
            "X|On a lower-circuit bar the exit waits for the next bar that is not, at Open - buffer",
        ], "LONG in Nifty if USDINR is in Sell (Without Filter) with trail & hard SL live.afl (HRISHIT)."),
        def(ST_SHORT_PCT, "Short Nifty on USDINR ST up, % trailing stop at close", "Short", &[
            "F|USDINR ST up; not a lower-circuit bar",
            "E|Short at the close",
            "X|Close >= hard SL (2 % above), or Close >= the trail (5 % above the lowest low since entry), or USDINR ST down: cover at the close",
            "X|On an upper-circuit bar the cover is deferred; as written, it fires on a later upper-circuit bar",
        ], "Short in Nifty if USDINR is in Buy (Without Filter) - Ver 2 @close.afl and Others/... - Ver 2.afl. USDINR ST is 1 during its warm-up, so a short can open there (as in the AFL)."),
        def(ST2_PT, "Long Nifty on two USDINR STs, PT / HSL / TSL at close", "Long", &[
            "F|USDINR ST 1 (5, 1) and ST 2 (5, 2) are both down; optional filter ST (20, 1, monthly) down; not an upper-circuit bar",
            "E|Buy at the close",
            "X|Close <= hard SL (2 %), Close <= trail (5 % below the highest high, previous bar's), Close >= profit target (10 %), or ST 1 up: sell at the close",
            "X|With the filter on, the stops are tested on the Low (filled at the close) and the filter ST turning up also exits",
            "X|No sale on a lower-circuit bar (it is re-tested next bar)",
        ], "LONG in Nifty if USDINR is in Sell in 2 ST(51 & 52) (With & Without Filter) -PT,HSL,TSL @ Close.afl. Late entry (both STs), early exit (ST 1). The filter ST follows TimeFrameRestore without SetForeign, so it is computed on the traded index; its time frame is weekly when chosen, otherwise monthly (as written)."),
        def(ST2_LIVE, "Long Nifty on two USDINR STs, HSL / TSL live", "Long", &[
            "F|USDINR ST 1 (5, 1) and ST 2 (5, 2) are both down; optional filter ST down; not an upper-circuit bar",
            "E|Buy at the close",
            "X|Low <= hard SL: at the SL (or Open - buffer); Low <= the previous bar's trail: at the trail (or Open - buffer); ST 1 up (or the filter ST up): at the close",
            "X|No sale on a lower-circuit bar (it is re-tested next bar)",
        ], "LONG in Nifty if USDINR is in Sell in 2 ST(51 & 52) (With & Without Filter) - Ver 3.afl."),
        def(PR_BASE, "USDINR price relative, stop and reverse", "Long/Short", &[
            "E|Long when PR < its MA ratio (minus the buy buffer); short when PR > its MA ratio (minus the sell buffer)",
            "X|Each signal reverses the position at the close",
        ], "NIFTY USD PR TRADING IN NIFTY ONLY.afl and _Buy & sell buffer.afl (buffers in % of the MA ratio, 0 by default). The Nifty leg of NIFTY USD PR TRADING NIFTY USD COMBO.afl gives the same trades; its USDINR leg needs USDINR as a traded symbol and is not modelled."),
        def(PR_PRF, "USDINR price relative with PR filter", "Long/Short", &[
            "F|Filter PR: a second PR line (period 80); long only when PR < it, short only when PR > it (filter on)",
            "E|Long when PR < its MA ratio and the filter allows; short when PR > its MA ratio and the filter allows",
            "X|Sell when PR > its MA ratio; cover when PR < it (filter not needed)",
        ], "NIFTY USD PR TRADING IN NIFTY ONLY WITH PR FILTER.afl (three copies: 60/80, 60/80, 200/200 by default; the 60/80 file's defaults are used). The filter's 'double MA' takes its inner period from the trading period (as written)."),
        def(PR_PRF_R2, "USDINR price relative, PR filter on shorts only (Regime 2)", "Long/Short", &[
            "E|Long when PR < its MA ratio (no filter); short when PR > it and the filter PR is above its line",
            "X|Sell when PR > its MA ratio; cover when PR < it",
        ], "NIFTY USD PR TRADING IN NIFTY ONLY WITH PR FILTER REGIM2.afl."),
        def(PR_PRF_BB, "USDINR price relative, PR filter, bull / bear regime", "Long/Short", &[
            "E|Bull (long only): long when PR < its MA ratio and the filter PR is below its line",
            "X|Bull: sell when PR > its MA ratio or the filter PR crosses above its line",
            "E|Bear: long when PR < its MA ratio and the filter PR is above its line; short when PR > its MA ratio and the filter PR is above its line",
            "X|Bear: sell when PR > its MA ratio or the filter PR crosses below its line; cover when PR < its MA ratio or the filter PR crosses below its line",
        ], "NIFTY USD PR TRADING IN NIFTY ONLY WITH PR FILTER REGIM 2 WITH BULL BEAR OPTION.afl (three copies; defaults 2 / 10, as in the Optimize copy). The cross exits apply even with the filter off (as written)."),
        def(PR_STF, "USDINR price relative with index ST / SMA filter", "Long/Short", &[
            "F|Filter on the index: ST (20, 5) trend, or Close above / below its SMA (previous bar), on the chosen time frame",
            "E|Long when PR < its MA ratio and the filter is up; short when PR > its MA ratio and the filter is down",
            "X|Sell when PR > its MA ratio; cover when PR < it",
        ], "NIFTY USD PR TRADING IN NIFTY ONLY WITH ST FILTER.afl (Regime 1; two copies, PR period 20 or 200)."),
        def(PR_STF_R2, "USDINR price relative, index ST filter on shorts only (Regime 2)", "Long/Short", &[
            "E|Long when PR < its MA ratio (no filter); short when PR > it and the index ST is down",
            "X|Sell when PR > its MA ratio; cover when PR < it or the index close crosses above the ST down line",
        ], "NIFTY USD PR TRADING IN NIFTY ONLY WITH ST FILTER REGIM 2.afl."),
        def(PR_STF_BB, "USDINR price relative, index ST filter, bull / bear regime", "Long/Short", &[
            "E|Bull (long only): long when PR < its MA ratio and the index ST is up",
            "X|Bull: sell when PR > its MA ratio or the previous bar's ST up line crosses above the index close",
            "E|Bear: long when PR < its MA ratio and the index ST is down; short when PR > its MA ratio and the index ST is down",
            "X|Bear: sell / cover when PR is past its MA ratio or the index close crosses above the ST down line",
        ], "NIFTY USD PR TRADING IN NIFTY ONLY WITH ST FILTER REGIM 2 WITH BULL BEAR OPTION.afl (also the BANK NIFTY copy)."),
        def(PR_COMBO_LS, "USDINR price relative with PR filter, combo long / short (Nifty leg)", "Long/Short", &[
            "E|Long when PR < its MA ratio and the filter PR is below its line; short when PR > its MA ratio and above",
            "X|Sell when PR > its MA ratio or the filter turns short; cover when PR < its MA ratio or the filter turns long",
        ], "NIFTY USD PR TRADING IN NIFTY ONLY WITH PR FILTER_COMBO (L&S).afl, Nifty leg; the mirrored USDINR leg needs USDINR as a traded symbol and is not modelled. Default trading period 60 (above its Param maximum 20; kept)."),
        def(PR_COMBO_SCAN, "USDINR price relative with PR filter, combo scan (Nifty leg)", "Long/Short", &[
            "E|As the combo L&S, but with the filter off neither filter state is set, and the short entry uses the filter's long state (as written)",
            "X|Sell when PR > its MA ratio or the filter turns short; cover when PR < its MA ratio or the filter turns long",
        ], "NIFTY USD PR TRADING IN NIFTY ONLY WITH PR FILTER_combo_scan.afl (Yashesh), Nifty leg. Signals are ExRem'd before and after the symbol mapping, as in the AFL."),
        def(SF_HSL, "Long on USDINR PR fast / slow lines, running entry, HSL live", "Long", &[
            "E|Filter off: buy at the close when PR crosses below the fast line",
            "E|Filter on: a PR cross below the fast line is remembered; buy at the close on that or a later bar where the filter (index ST, or the filter PR below its line) is bullish",
            "X|Low < hard SL (0 = off): at the SL, or Open - 0.25 % when the bar opens below it",
            "X|PR above both the fast and the slow line: sell at the close",
        ], "Long Only with USDINR & NSENIFTY Slow and Fast PR indicator with & Without ST & PR Filter(Running Entry) Live HSL(FINAL).afl and its three copies. The filter's short exit compares a 0/1 value with -1 and never fires (as written). The fast / slow period defaults (200) are above the Param maximums; kept. The loop starts at bar 'fast period'."),
        def(SF_ST, "Long on USDINR PR fast / slow lines, running entry, ST filter", "Long", &[
            "E|As the HSL version with the index ST filter, no hard SL",
            "X|PR above both lines, or (filter on) the index ST turning down: sell at the close",
        ], "Long Only with USDINR & NSENIFTY Slow and Fast PR indicator with & Without SuperTrend Filter(Running Entry).afl (14-04-2022); the only version whose filter exit works."),
        def(MST, "Long on USDINR PR with monthly index ST filter, daily ST exit", "Long", &[
            "E|Filter on: buy at the close when PR < its MA ratio and the monthly index ST is up (a level, not a cross)",
            "E|Filter off: buy when PR < its MA ratio",
            "X|The daily index ST turns down (fresh flip), or (filter on) the monthly ST is down: sell at the close",
        ], "Long Only in NSENIFTY with PR indicator with Monthly ST as a Filter and Daily ST as a Indicator.afl (three copies). Re-entry can follow on the next bar after an exit (the entry is a level)."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def: f64, min: f64, max: f64, step| ParamDef { key, label, def, min: min.min(def), max: max.max(def), step };
    let tf = |key, label, def| p(key, label, def, 1.0, 3.0, 1.0);
    let pt = |key, label| p(key, label, 1.0, 1.0, 3.0, 1.0);
    let usd = |per: f64, fac: f64, fmin: f64, fmax: f64, fstep: f64| vec![
        tf("usd_tf", "USDINR ST time frame (1 daily, 2 weekly, 3 monthly)", 1.0),
        pt("usd_point", "USDINR ST calculation point (1 mid, 2 close, 3 pivot)"),
        p("usd_period", "USDINR ST period", per, 5.0, 25.0, 1.0),
        p("usd_factor", "USDINR ST factor", fac, fmin, fmax, fstep),
    ];
    let nifty = |fac: f64, fmax: f64, fstep: f64| vec![
        tf("n_tf", "Index ST time frame (1 daily, 2 weekly, 3 monthly)", 1.0),
        pt("n_point", "Index ST calculation point (1 mid, 2 close, 3 pivot)"),
        p("n_period", "Index ST period", 5.0, 5.0, 25.0, 1.0),
        p("n_factor", "Index ST factor", fac, 1.0, fmax, fstep),
    ];
    let avg = |key, label| p(key, label, 0.0, 0.0, 5.0, 1.0);
    let mut v: Vec<ParamDef> = Vec::new();
    match id {
        ST_NIFTY | ST_DOUBLE => {
            v.push(p("hsl_pct", "Hard SL % (0 off)", 2.0, 0.0, 10.0, 0.01));
            v.extend(usd(10.0, 2.0, 1.0, if id == ST_NIFTY { 4.0 } else { 6.0 }, 1.0));
            v.extend(nifty(4.0, 7.0, 1.0));
        }
        ST_LIVE | ST_SHORT_LIVE => {
            v.push(p("hsl_pct", "Hard SL % (0 off)", 2.0, 0.0, 10.0, 0.01));
            v.push(p("buffer", "Buffer %", 0.25, 0.0, 10.0, 0.05));
            if id == ST_LIVE {
                v.extend(usd(10.0, 2.0, 1.0, 3.0, 0.5));
                v.extend(nifty(4.0, 3.0, 0.5));
            } else {
                v.extend(usd(10.0, 2.0, 3.0, 6.0, 1.0));
                v.extend(nifty(4.0, 7.0, 1.0));
            }
        }
        ST_ATR => {
            v.push(p("hsl_pct", "Hard SL % (0 off)", 2.0, 0.0, 10.0, 0.01));
            v.extend(usd(10.0, 2.0, 3.0, 6.0, 1.0));
            v.push(p("atr_period", "ATR periods", 5.0, 1.0, 100.0, 1.0));
            v.push(p("atr_mult", "ATR trail SL multiplier", 5.0, 0.0, 20.0, 1.0));
        }
        ST_PCT | ST_PCT_LIVE | ST_SHORT_PCT => {
            v.push(p("hsl_pct", "Hard SL % (0 off)", 2.0, 0.0, 10.0, 0.5));
            v.push(p("tsl_pct", "Trailing SL % (0 off)", 5.0, 0.0, 15.0, 0.5));
            if id == ST_PCT_LIVE {
                v.push(p("buffer", "Buffer %", 0.25, 0.0, 10.0, 0.05));
            }
            v.extend(usd(10.0, 2.0, 1.0, 3.0, if id == ST_PCT_LIVE { 0.5 } else { 1.0 }));
        }
        ST2_PT | ST2_LIVE => {
            v.push(p("filter_on", "Filter type (0 off, 1 ST)", 0.0, 0.0, 1.0, 1.0));
            if id == ST2_PT {
                v.push(p("pb_pct", "Profit booking % (0 off)", 10.0, 0.0, 30.0, 0.1));
            }
            v.push(p("hsl_pct", "Hard SL % (0 off)", 2.0, 0.0, 30.0, 0.01));
            v.push(p("tsl_pct", "Trailing SL % (0 off)", 5.0, 0.0, 30.0, 0.01));
            v.push(p("buffer", "Buffer %", 0.25, 0.0, 10.0, 0.05));
            v.push(tf("usd_tf", "USDINR ST time frame (1 daily, 2 weekly, 3 monthly)", 1.0));
            v.push(pt("usd_point", "USDINR ST calculation point (1 mid, 2 close, 3 pivot)"));
            v.push(p("st1_period", "1st ST period", 5.0, 1.0, 20.0, 1.0));
            v.push(p("st1_factor", "1st ST factor", 1.0, 1.0, 20.0, 1.0));
            v.push(p("st2_period", "2nd ST period", 5.0, 1.0, 20.0, 1.0));
            v.push(p("st2_factor", "2nd ST factor", 2.0, 1.0, 20.0, 1.0));
            v.push(tf("f_tf", "Filter ST time frame (2 weekly, otherwise monthly)", 1.0));
            v.push(pt("f_point", "Filter ST calculation point (1 mid, 2 close, 3 pivot)"));
            v.push(p("f_period", "Filter ST period", 20.0, 1.0, 20.0, 1.0));
            v.push(p("f_factor", "Filter ST factor", 1.0, 1.0, 20.0, 1.0));
        }
        PR_BASE => {
            v.push(p("pr_period", "PR average period", 200.0, 1.0, 200.0, 1.0));
            v.push(avg("pr_avg", "PR average type (0 EMA, 1 simple, 2 double, 3 triple, 4 triangular, 5 weighted)"));
            v.push(p("buy_buf", "Buy buffer % of the MA ratio", 0.0, 0.0, 10.0, 0.1));
            v.push(p("sell_buf", "Sell buffer % of the MA ratio", 0.0, 0.0, 10.0, 0.1));
        }
        PR_PRF | PR_PRF_R2 | PR_PRF_BB | PR_COMBO_LS | PR_COMBO_SCAN => {
            let (a, b) = match id {
                PR_PRF | PR_COMBO_LS | PR_COMBO_SCAN => (60.0, 80.0),
                PR_PRF_BB => (2.0, 10.0),
                _ => (200.0, 200.0),
            };
            v.push(p("pr_period", "PR average period", a, 2.0, 300.0, 1.0));
            v.push(avg("pr_avg", "PR average type (0 EMA, 1 simple, 2 double, 3 triple, 4 triangular, 5 weighted)"));
            v.push(p("filter_on", "Filter (1 on, 0 off)", 1.0, 0.0, 1.0, 1.0));
            v.push(p("f_period", "Filter PR average period", b, 2.0, 300.0, 5.0));
            v.push(avg("f_avg", "Filter PR average type (0 EMA .. 5 weighted)"));
            if id == PR_PRF_BB {
                v.push(p("regime", "Regime (0 bull, 1 bear)", 0.0, 0.0, 1.0, 1.0));
            }
        }
        PR_STF | PR_STF_R2 | PR_STF_BB => {
            let (a, f) = if id == PR_STF { (20.0, 5.0) } else { (200.0, 1.0) };
            v.push(p("pr_period", "PR average period", a, 1.0, 300.0, 1.0));
            v.push(avg("pr_avg", "PR average type (0 EMA, 1 simple, 2 double, 3 triple, 4 triangular, 5 weighted)"));
            if id == PR_STF {
                v.push(p("filter_type", "Filter type (1 ST, 2 SMA, 0 off)", 1.0, 0.0, 2.0, 1.0));
            }
            v.push(tf("n_tf", "Filter time frame (1 daily, 2 weekly, 3 monthly)", 1.0));
            v.push(pt("n_point", "Filter ST calculation point (1 mid, 2 close, 3 pivot)"));
            v.push(p("n_period", "Filter ST period", 20.0, 1.0, 25.0, 1.0));
            v.push(p("n_factor", "Filter ST factor", f, 1.0, 20.0, 1.0));
            if id == PR_STF {
                v.push(p("sma_period", "Filter SMA period", 200.0, 1.0, 2000.0, 1.0));
            }
            if id == PR_STF_BB {
                v.push(p("regime", "Regime (0 bull, 1 bear)", 0.0, 0.0, 1.0, 1.0));
            }
        }
        SF_HSL | SF_ST => {
            v.push(p("fast", "Fast PR average period", 200.0, 2.0, 200.0, 1.0));
            v.push(p("slow", "Slow PR average period", 200.0, 2.0, 200.0, 1.0));
            v.push(avg("pr_avg", "PR average type (0 EMA, 1 simple, 2 double, 3 triple, 4 triangular, 5 weighted)"));
            v.push(p("filter_on", "Filter (1 on, 0 off)", 0.0, 0.0, 1.0, 1.0));
            if id == SF_HSL {
                v.push(p("hsl_pct", "Hard SL % (0 off)", 0.0, 0.0, 5.0, 1.0));
                v.push(p("filter_type", "Filter type (1 ST, 2 PR)", 1.0, 1.0, 2.0, 1.0));
                v.push(p("f_period", "Filter PR average period", 200.0, 1.0, 1000.0, 5.0));
                v.push(avg("f_avg", "Filter PR average type (0 EMA .. 5 weighted)"));
            }
            v.push(tf("n_tf", "Filter ST time frame (1 daily, 2 weekly, 3 monthly)", 3.0));
            v.push(pt("n_point", "Filter ST calculation point (1 mid, 2 close, 3 pivot)"));
            v.push(p("n_period", "Filter ST period", 5.0, 1.0, 20.0, 1.0));
            v.push(p("n_factor", "Filter ST factor", 4.0, 1.0, 20.0, 1.0));
        }
        MST => {
            v.push(p("pr_period", "PR average period", 200.0, 5.0, 120.0, 5.0));
            v.push(avg("pr_avg", "PR average type (0 EMA, 1 simple, 2 double, 3 triple, 4 triangular, 5 weighted)"));
            v.push(p("filter_on", "Filter (1 on, 0 off)", 1.0, 0.0, 1.0, 1.0));
            v.push(tf("n_tf", "Filter ST time frame (1 daily, 2 weekly, 3 monthly)", 3.0));
            v.push(pt("n_point", "Filter ST calculation point (1 mid, 2 close, 3 pivot)"));
            v.push(p("n_period", "Filter ST period", 5.0, 5.0, 20.0, 5.0));
            v.push(p("n_factor", "Filter ST factor", 4.0, 1.0, 8.0, 1.0));
            v.push(pt("d_point", "Daily ST calculation point (1 mid, 2 close, 3 pivot)"));
            v.push(p("d_period", "Daily ST period", 5.0, 5.0, 20.0, 5.0));
            v.push(p("d_factor", "Daily ST factor", 4.0, 1.0, 4.0, 1.0));
        }
        _ => return None,
    }
    Some(v)
}

// ---------------------------------------------------------------- data and indicators

/// USDINR aligned to the index's dates (AmiBroker Foreign fix-up): a missing day repeats the previous close as
/// O = H = L = C. NaN before the first USDINR bar. None without the series.
fn usd(x: &Ctx) -> Option<[Vec<f64>; 4]> {
    let c = x.series("USDINR.Close")?;
    let get = |k: &str| x.series(k).map(|v| v.to_vec()).unwrap_or_else(|| c.to_vec());
    let (mut o, mut h, mut l, mut c) = (get("USDINR.Open"), get("USDINR.High"), get("USDINR.Low"), c.to_vec());
    for i in 1..c.len() {
        if !c[i].is_finite() && c[i - 1].is_finite() {
            c[i] = c[i - 1];
            o[i] = c[i];
            h[i] = c[i];
            l[i] = c[i];
        }
        for v in [&mut o, &mut h, &mut l] {
            if !v[i].is_finite() {
                v[i] = c[i];
            }
        }
    }
    Some([o, h, l, c])
}

/// AFL Supertrend trend of a series on time frame `tf` (1 daily, 2 weekly, 3 monthly), expanded to daily bars
/// (expandLast), with the up line (ITrendUp) and down line (ITrendDown) expanded the same way. Bars before the
/// series starts get trend 0 (no signal) and no lines.
fn st_full(x: &Ctx, h: &[f64], l: &[f64], c: &[f64], tf: i64, point: i64, period: usize, factor: f64) -> (Vec<i8>, Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = c.len();
    let Some(s) = (0..n).find(|&i| c[i].is_finite() && h[i].is_finite() && l[i].is_finite()) else {
        return (vec![0; n], vec![None; n], vec![None; n], vec![None; n]);
    };
    let key = |i: usize| -> i64 {
        match tf {
            3 => x.r[i].date.get(..7).map_or(i as i64, |s| s.replace('-', "").parse().unwrap_or(i as i64)),
            2 => day_number(&x.r[i].date).map_or(i as i64, |d| (d + 3).div_euclid(7)),
            _ => i as i64,
        }
    };
    let mut b = Bars { h: vec![], l: vec![], c: vec![], last: vec![], group: vec![0; n - s] };
    let mut prev = None;
    for i in s..n {
        let k = key(i);
        if prev != Some(k) {
            b.h.push(h[i]);
            b.l.push(l[i]);
            b.c.push(c[i]);
            b.last.push(i - s);
            prev = Some(k);
        }
        let g = b.c.len() - 1;
        b.h[g] = b.h[g].max(h[i]);
        b.l[g] = b.l[g].min(l[i]);
        b.c[g] = c[i];
        b.last[g] = i - s;
        b.group[i - s] = g;
    }
    let (tr, lu, ld) = supertrend_lines(&calc_point(&b, point), &b.h, &b.l, &b.c, period, factor);
    let cl: Vec<Option<f64>> = b.c.iter().map(|&v| Some(v)).collect();
    let pad = |v: Vec<Option<f64>>| { let mut o = vec![None; s]; o.extend(expand_last(&v, &b)); o };
    let mut t = vec![0i8; s];
    t.extend(expand_last(&tr, &b));
    (t, pad(lu), pad(ld), pad(cl))
}

/// `ytsw::supertrend` with both plotted lines: (trend, ITrendUp, ITrendDown).
fn supertrend_lines(calc: &[f64], h: &[f64], l: &[f64], c: &[f64], period: usize, f: f64) -> (Vec<i8>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = c.len();
    let trr = true_range(h, l, c);
    let p = period.max(1);
    let satr: Vec<Option<f64>> = (0..n)
        .map(|i| if i + 1 < p { None } else { trr[i + 1 - p..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)).map(|s| s / p as f64) })
        .collect();
    let mut up: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| calc[i] + f * s)).collect();
    let mut dn: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| calc[i] - f * s)).collect();
    let mut tr = vec![1i8; n];
    let (mut lu, mut ld) = (vec![None; n], vec![None; n]);
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
            up[i] = satr[i].map(|s| calc[i] + f * s);
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = satr[i].map(|s| calc[i] - f * s);
        }
        if tr[i] == 1 {
            lu[i] = dn[i];
            if tr[i - 1] == -1 {
                lu[i - 1] = ld[i - 1];
            }
        } else {
            ld[i] = up[i];
            if tr[i - 1] == 1 {
                ld[i - 1] = lu[i - 1];
            }
        }
    }
    (tr, lu, ld)
}

fn st_dir(x: &Ctx, h: &[f64], l: &[f64], c: &[f64], tf: i64, point: i64, period: usize, factor: f64) -> Vec<i8> {
    st_full(x, h, l, c, tf, point, period, factor).0
}

fn sma(v: &[f64], n: usize) -> Vec<f64> {
    let n = n.max(1);
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}

/// AFL EMA(v, n), started from the simple average of the first n values (NaN gaps restart it).
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
            prev += k * (v[i] - prev);
            out[i] = prev;
        }
    }
    out
}

/// AFL WMA(v, n): linear weights 1..n, the latest bar weighted n.
fn wma(v: &[f64], n: usize) -> Vec<f64> {
    let n = n.max(1);
    let den = (n * (n + 1)) as f64 / 2.0;
    (0..v.len())
        .map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().enumerate().map(|(k, x)| x * (k + 1) as f64).sum::<f64>() / den })
        .collect()
}

/// The AFL's PR average types. `inner` is the first-stage period of the double MA (the filter blocks take it from
/// the trading period, as written).
fn avg(v: &[f64], n: usize, kind: i64, inner: usize) -> Vec<f64> {
    match kind {
        1 => sma(v, n),
        2 => sma(&sma(v, inner), n),
        3 => sma(&sma(&sma(v, n), n), n),
        4 => {
            let q = ((n as f64 + 1.1) / 2.0).round().max(1.0) as usize;
            ema(&sma(v, q), q)
        }
        5 => wma(v, n),
        _ => ema(v, n),
    }
}

/// (PR, its line): USDINR close / index close, MA(USDINR) / MA(index).
fn pr_lines(x: &Ctx, u: &[f64], n: usize, kind: i64, inner: usize) -> (Vec<f64>, Vec<f64>) {
    let (mu, mi) = (avg(u, n, kind, inner), avg(&x.c, n, kind, inner));
    ((0..u.len()).map(|i| u[i] / x.c[i]).collect(), (0..u.len()).map(|i| mu[i] / mi[i]).collect())
}

fn lt(a: f64, b: f64) -> bool {
    a.is_finite() && b.is_finite() && a < b
}
fn gt(a: f64, b: f64) -> bool {
    a.is_finite() && b.is_finite() && a > b
}
/// AFL Cross(a, b): a crosses above b on this bar.
fn cross(a: &[f64], b: &[f64], i: usize) -> bool {
    i > 0 && gt(a[i], b[i]) && a[i - 1].is_finite() && b[i - 1].is_finite() && a[i - 1] <= b[i - 1]
}
/// AFL Prec(v, 2): truncated to 2 decimals.
fn prec2(v: f64) -> f64 {
    (v * 100.0).trunc() / 100.0
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let Some(u) = usd(x) else { return Some(Vec::new()) };
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    Some(match id {
        ST_NIFTY | ST_DOUBLE | ST_LIVE | ST_SHORT_LIVE | ST_ATR | ST_PCT | ST_PCT_LIVE | ST_SHORT_PCT | ST2_PT | ST2_LIVE => {
            collect(x, st_family(id, x, &u, &g, &gi, &gu), from, to)
        }
        PR_BASE | PR_PRF | PR_PRF_R2 | PR_PRF_BB | PR_STF | PR_STF_R2 | PR_STF_BB | PR_COMBO_LS | PR_COMBO_SCAN => {
            let (sig, side) = pr_family(id, x, &u[3], &g, &gi, &gu);
            backtest(x, sig, side, from, to)
        }
        _ => collect(x, sf_family(id, x, &u[3], &g, &gi, &gu), from, to),
    })
}

// ---------------------------------------------------------------- USDINR Supertrend AFLs

fn st_family(id: &str, x: &Ctx, u: &[Vec<f64>; 4], g: &dyn Fn(&str) -> f64, gi: &dyn Fn(&str) -> i64, gu: &dyn Fn(&str) -> usize) -> Vec<Ev> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let two = id == ST2_PT || id == ST2_LIVE;
    let (im, st2) = if two {
        (st_dir(x, &u[1], &u[2], &u[3], gi("usd_tf"), gi("usd_point"), gu("st1_period"), g("st1_factor")),
         st_dir(x, &u[1], &u[2], &u[3], gi("usd_tf"), gi("usd_point"), gu("st2_period"), g("st2_factor")))
    } else {
        (st_dir(x, &u[1], &u[2], &u[3], gi("usd_tf"), gi("usd_point"), gu("usd_period"), g("usd_factor")), vec![0; n])
    };
    // the second ST: on the index (or on USDINR when SECOND_ST_ON_INDEX is false)
    let (sh, sl, sc): (&[f64], &[f64], &[f64]) = if SECOND_ST_ON_INDEX { (h, l, c) } else { (&u[1], &u[2], &u[3]) };
    let mt = match id {
        ST_NIFTY | ST_DOUBLE | ST_LIVE | ST_SHORT_LIVE => st_dir(x, sh, sl, sc, gi("n_tf"), gi("n_point"), gu("n_period"), g("n_factor")),
        ST2_PT | ST2_LIVE => st_dir(x, sh, sl, sc, if gi("f_tf") == 2 { 2 } else { 3 }, gi("f_point"), gu("f_period"), g("f_factor")),
        _ => vec![0; n],
    };
    let atr = if id == ST_ATR {
        let tr: Vec<f64> = true_range(h, l, c).into_iter().map(|v| v.unwrap_or(f64::NAN)).collect();
        wilder(&tr, gu("atr_period"))
    } else {
        vec![f64::NAN; n]
    };
    let hp = g("hsl_pct") / 100.0;
    let tp = g("tsl_pct") / 100.0;
    let buf = g("buffer") / 100.0;
    let mult = g("atr_mult");
    let pb = g("pb_pct") / 100.0;
    let flt = gi("filter_on") == 1;
    let short = id == ST_SHORT_LIVE || id == ST_SHORT_PCT;
    let uc = |i: usize| h[i] == c[i];
    let lc = |i: usize| l[i] == c[i];
    let up_circ = |i: usize| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
    let lo_circ = |i: usize| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);

    let mut evs: Vec<Ev> = Vec::new();
    let mut pos = false;
    let mut skip = false;
    let (mut hsl, mut trail, mut ext, mut pt, mut atrv) = (0.0, 0.0, 0.0, BIG, 0.0);
    let close = |evs: &mut Vec<Ev>, i: usize, px: f64, why: &'static str| {
        if let Some(v) = evs.last_mut() {
            v.exit = Some((i, bound(x, i, px), i, why));
        }
    };
    for i in 1..n {
        let ok = c[i] > 0.0;
        if !pos {
            let enter = match id {
                ST_NIFTY => im[i] == -1 && mt[i] == 1,
                ST_DOUBLE | ST_LIVE => im[i] == -1 && mt[i] == 1 && !uc(i),
                ST_SHORT_LIVE => im[i] == 1 && mt[i] == -1 && !lc(i),
                ST_ATR | ST_PCT | ST_PCT_LIVE => im[i] == -1 && !uc(i),
                ST_SHORT_PCT => im[i] == 1 && !lc(i),
                _ => im[i] == -1 && st2[i] == -1 && (!flt || mt[i] == -1) && !uc(i),
            } && ok;
            if enter {
                pos = true;
                skip = false;
                evs.push(Ev { dir: if short { -1.0 } else { 1.0 }, e: i, epx: c[i], exit: None });
                hsl = if short {
                    if hp > 0.0 { c[i] * (1.0 + hp) } else { BIG }
                } else if hp > 0.0 {
                    c[i] * (1.0 - hp)
                } else {
                    0.0
                };
                match id {
                    ST_ATR => {
                        ext = c[i];
                        trail = ext - atr[i] * mult;
                        atrv = 0.0;
                    }
                    ST_SHORT_PCT => {
                        ext = l[i];
                        trail = if tp > 0.0 { ext * (1.0 + tp) } else { BIG };
                    }
                    ST_PCT | ST_PCT_LIVE | ST2_PT | ST2_LIVE => {
                        ext = h[i];
                        trail = if tp > 0.0 { ext * (1.0 - tp) } else { 0.0 };
                        pt = if pb > 0.0 { c[i] * (1.0 + pb) } else { BIG };
                    }
                    _ => {}
                }
            }
            continue;
        }
        if skip {
            // deferred exit after a circuit bar
            let (go, px) = match id {
                ST_DOUBLE | ST_ATR => (!lc(i), c[i]),
                ST_PCT => (lc(i), c[i]),        // as written: NOT LC == 0
                ST_SHORT_PCT => (uc(i), c[i]),  // as written: NOT UC == 0
                ST_LIVE | ST_PCT_LIVE => (!lo_circ(i), o[i] * (1.0 - buf)),
                ST_SHORT_LIVE => (!up_circ(i), o[i] * (1.0 + buf)),
                _ => (false, c[i]),
            };
            if go {
                close(&mut evs, i, px, "Deferred exit after a circuit bar");
                pos = false;
                skip = false;
            }
            continue;
        }
        match id {
            ST_NIFTY => {
                if c[i] < hsl || im[i] == 1 || mt[i] == -1 {
                    close(&mut evs, i, c[i], if c[i] < hsl { "Hard SL" } else { "ST exit" });
                    pos = false;
                }
            }
            ST_DOUBLE => {
                if c[i] <= hsl || (im[i] == 1 && mt[i] == -1) {
                    if !lc(i) {
                        close(&mut evs, i, c[i], if c[i] <= hsl { "Hard SL" } else { "Both STs exit" });
                        pos = false;
                    } else {
                        skip = true;
                    }
                }
            }
            ST_LIVE => {
                let hit = l[i] <= hsl;
                if hit || im[i] == 1 || mt[i] == -1 {
                    if !lc(i) {
                        let px = if hit { if o[i] < hsl { o[i] * (1.0 - buf) } else { hsl } } else { c[i] };
                        close(&mut evs, i, px, if hit { "Hard SL" } else { "ST exit" });
                        pos = false;
                    } else {
                        skip = true;
                    }
                }
            }
            ST_SHORT_LIVE => {
                let hit = h[i] >= hsl;
                if hit || im[i] == -1 || mt[i] == 1 {
                    if !uc(i) {
                        let px = if hit { if o[i] > hsl { o[i] * (1.0 + buf) } else { hsl } } else { c[i] };
                        close(&mut evs, i, px, if hit { "Hard SL" } else { "ST exit" });
                        pos = false;
                    } else {
                        skip = true;
                    }
                }
            }
            ST_ATR => {
                ext = ext.max(c[i]);
                if c[i] == ext {
                    atrv = atr[i];
                }
                if c[i] <= hsl || (im[i] == 1 && c[i] <= trail) {
                    if !lc(i) {
                        close(&mut evs, i, c[i], if c[i] <= hsl { "Hard SL" } else { "ATR trail with USDINR ST up" });
                        pos = false;
                        continue;
                    }
                    skip = true;
                }
                let t = if atrv > 0.0 { ext - atrv * mult } else { trail };
                trail = if t < trail { trail } else { t };
            }
            ST_PCT => {
                ext = ext.max(h[i]);
                if tp > 0.0 {
                    trail = ext * (1.0 - tp);
                }
                if c[i] <= hsl || c[i] <= trail || im[i] == 1 {
                    if !lc(i) {
                        close(&mut evs, i, c[i], if c[i] <= hsl { "Hard SL" } else if c[i] <= trail { "Trailing SL" } else { "USDINR ST up" });
                        pos = false;
                    } else {
                        skip = true;
                    }
                }
            }
            ST_SHORT_PCT => {
                ext = ext.min(l[i]);
                if tp > 0.0 {
                    trail = ext * (1.0 + tp);
                }
                if c[i] >= hsl || c[i] >= trail || im[i] == -1 {
                    if !uc(i) {
                        close(&mut evs, i, c[i], if c[i] >= hsl { "Hard SL" } else if c[i] >= trail { "Trailing SL" } else { "USDINR ST down" });
                        pos = false;
                    } else {
                        skip = true;
                    }
                }
            }
            ST_PCT_LIVE => {
                ext = ext.max(h[i]);
                let (a, b) = (l[i] <= hsl, l[i] <= trail);
                if a || b || im[i] == 1 {
                    if !lc(i) {
                        let px = if a { if o[i] < hsl { o[i] * (1.0 - buf) } else { hsl } } else if b { if o[i] < trail { o[i] * (1.0 - buf) } else { trail } } else { c[i] };
                        close(&mut evs, i, px, if a { "Hard SL" } else if b { "Trailing SL" } else { "USDINR ST up" });
                        pos = false;
                        continue;
                    }
                    skip = true;
                }
                if tp > 0.0 {
                    trail = ext * (1.0 - tp);
                }
            }
            _ => {
                // ST2_PT / ST2_LIVE: no deferral; a blocked exit is tested again on the next bar
                ext = ext.max(h[i]);
                let px_l = if flt || id == ST2_LIVE { l[i] } else { c[i] };
                let (a, b) = (px_l <= hsl, px_l <= trail);
                let tgt = id == ST2_PT && c[i] >= pt;
                let st_exit = im[i] == 1 || (flt && mt[i] == 1);
                if (a || b || tgt || st_exit) && !lc(i) {
                    let px = if id == ST2_LIVE {
                        if a { if o[i] < hsl { o[i] * (1.0 - buf) } else { hsl } } else if b { if o[i] < trail { o[i] * (1.0 - buf) } else { trail } } else { c[i] }
                    } else {
                        c[i]
                    };
                    close(&mut evs, i, px, if a { "Hard SL" } else if b { "Trailing SL" } else if tgt { "Profit booking" } else { "ST exit" });
                    pos = false;
                    continue;
                }
                if tp > 0.0 {
                    trail = ext * (1.0 - tp);
                }
            }
        }
    }
    evs
}

// ---------------------------------------------------------------- PR AFLs (array signals)

fn pr_family(id: &str, x: &Ctx, u: &[f64], g: &dyn Fn(&str) -> f64, gi: &dyn Fn(&str) -> i64, gu: &dyn Fn(&str) -> usize) -> (Sig, i64) {
    let n = u.len();
    let np = gu("pr_period");
    let (pr, line) = pr_lines(x, u, np, gi("pr_avg"), np);
    let dn: Vec<bool> = (0..n).map(|i| lt(pr[i], line[i])).collect();
    let up: Vec<bool> = (0..n).map(|i| gt(pr[i], line[i])).collect();
    let f = |v: bool| vec![v; n];
    match id {
        PR_BASE => {
            let (bb, sb) = (g("buy_buf") / 100.0, g("sell_buf") / 100.0);
            let buy: Vec<bool> = (0..n).map(|i| lt(pr[i], line[i] * (1.0 + bb))).collect();
            let sell: Vec<bool> = (0..n).map(|i| gt(pr[i], line[i] * (1.0 - sb))).collect();
            (Sig { buy: buy.clone(), sell: sell.clone(), short: sell, cover: buy }, 3)
        }
        PR_PRF | PR_PRF_R2 | PR_PRF_BB | PR_COMBO_LS | PR_COMBO_SCAN => {
            let on = gi("filter_on") == 1;
            let (p1, l1) = pr_lines(x, u, gu("f_period"), gi("f_avg"), np);
            let (fb, fs): (Vec<bool>, Vec<bool>) = if on {
                ((0..n).map(|i| lt(p1[i], l1[i])).collect(), (0..n).map(|i| gt(p1[i], l1[i])).collect())
            } else if id == PR_COMBO_SCAN {
                (f(false), f(false))
            } else {
                (f(true), f(true))
            };
            let and = |a: &[bool], b: &[bool]| -> Vec<bool> { (0..n).map(|i| a[i] && b[i]).collect() };
            let or = |a: &[bool], b: &[bool]| -> Vec<bool> { (0..n).map(|i| a[i] || b[i]).collect() };
            let x_up: Vec<bool> = (0..n).map(|i| cross(&p1, &l1, i)).collect();
            let x_dn: Vec<bool> = (0..n).map(|i| cross(&l1, &p1, i)).collect();
            match id {
                PR_PRF => (Sig { buy: and(&dn, &fb), sell: up.clone(), short: and(&up, &fs), cover: dn }, 3),
                PR_PRF_R2 => (Sig { buy: dn.clone(), sell: up.clone(), short: and(&up, &fs), cover: dn }, 3),
                PR_PRF_BB if gi("regime") == 1 => {
                    (Sig { buy: and(&dn, &fs), sell: or(&up, &x_dn), short: and(&up, &fs), cover: or(&dn, &x_dn) }, 3)
                }
                PR_PRF_BB => (Sig { buy: and(&dn, &fb), sell: or(&up, &x_up), short: f(false), cover: f(false) }, 1),
                PR_COMBO_LS => (Sig { buy: and(&dn, &fb), sell: or(&up, &fs), short: and(&up, &fs), cover: or(&dn, &fb) }, 3),
                _ => {
                    let (b1, s1, sh1, c1) = (and(&dn, &fb), or(&up, &fs), and(&up, &fb), or(&dn, &fb));
                    let b = exrem(&b1, &s1);
                    let s = exrem(&s1, &b1);
                    let sh = exrem(&sh1, &c1);
                    let cv = exrem(&c1, &sh1);
                    (Sig { buy: b, sell: s, short: sh, cover: cv }, 3)
                }
            }
        }
        _ => {
            // index ST / SMA filter on the filter time frame
            let tf = gi("n_tf");
            let (tr, lu, ld, ic) = st_full(x, &x.h, &x.l, &x.c, tf, gi("n_point"), gu("n_period"), g("n_factor"));
            let ftype = if id == PR_STF { gi("filter_type") } else { 1 };
            let (fb, fs): (Vec<bool>, Vec<bool>) = match ftype {
                1 => ((0..n).map(|i| tr[i] == 1).collect(), (0..n).map(|i| tr[i] == -1).collect()),
                2 => {
                    let b = compress_index(x, tf);
                    let m = sma(&b.c, gu("sma_period"));
                    let above: Vec<i8> = (0..b.c.len()).map(|k| if k > 0 && gt(b.c[k], m[k - 1]) { 1 } else if k > 0 && lt(b.c[k], m[k - 1]) { -1 } else { 0 }).collect();
                    let e = expand_last(&above, &b);
                    ((0..n).map(|i| e[i] == 1).collect(), (0..n).map(|i| e[i] == -1).collect())
                }
                _ => (f(true), f(true)),
            };
            let o = |v: &[Option<f64>]| -> Vec<f64> { v.iter().map(|z| z.unwrap_or(f64::NAN)).collect() };
            let (lu, ld, ic) = (o(&lu), o(&ld), o(&ic));
            let lu_prev: Vec<f64> = (0..n).map(|i| if i == 0 { f64::NAN } else { lu[i - 1] }).collect();
            let and = |a: &[bool], b: &[bool]| -> Vec<bool> { (0..n).map(|i| a[i] && b[i]).collect() };
            let ic_over_dn: Vec<bool> = (0..n).map(|i| cross(&ic, &ld, i)).collect();
            let up_line_over_c: Vec<bool> = (0..n).map(|i| cross(&lu_prev, &ic, i)).collect();
            match id {
                PR_STF => (Sig { buy: and(&dn, &fb), sell: up.clone(), short: and(&up, &fs), cover: dn }, 3),
                PR_STF_R2 => {
                    let cover = (0..n).map(|i| dn[i] || ic_over_dn[i]).collect();
                    (Sig { buy: dn.clone(), sell: up.clone(), short: and(&up, &fs), cover }, 3)
                }
                _ if gi("regime") == 1 => {
                    let ex: Vec<bool> = (0..n).map(|i| up[i] || ic_over_dn[i]).collect();
                    let cv: Vec<bool> = (0..n).map(|i| dn[i] || ic_over_dn[i]).collect();
                    (Sig { buy: and(&dn, &fs), sell: ex, short: and(&up, &fs), cover: cv }, 3)
                }
                _ => {
                    let sell = (0..n).map(|i| up[i] || up_line_over_c[i]).collect();
                    (Sig { buy: and(&dn, &fb), sell, short: f(false), cover: f(false) }, 1)
                }
            }
        }
    }
}

/// The index compressed to the time frame (closes only are used).
fn compress_index(x: &Ctx, tf: i64) -> Bars {
    crate::books::ytsw::compress(x, match tf {
        3 => 'M',
        2 => 'W',
        _ => 'D',
    })
}

// ---------------------------------------------------------------- slow / fast PR and monthly-ST AFLs (loops)

fn sf_family(id: &str, x: &Ctx, u: &[f64], g: &dyn Fn(&str) -> f64, gi: &dyn Fn(&str) -> i64, gu: &dyn Fn(&str) -> usize) -> Vec<Ev> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let on = gi("filter_on") == 1;
    let mtrend = st_dir(x, h, l, c, gi("n_tf"), gi("n_point"), gu("n_period"), g("n_factor"));
    let mut evs: Vec<Ev> = Vec::new();
    let mut pos = false;
    let close = |evs: &mut Vec<Ev>, i: usize, px: f64, why: &'static str| {
        if let Some(v) = evs.last_mut() {
            v.exit = Some((i, bound(x, i, px), i, why));
        }
    };
    if id == MST {
        let np = gu("pr_period");
        let (pr, line) = pr_lines(x, u, np, gi("pr_avg"), np);
        let st = st_dir(x, h, l, c, 1, gi("d_point"), gu("d_period"), g("d_factor"));
        for i in 1..n {
            let below = lt(pr[i], line[i]);
            if !pos && below && (!on || mtrend[i] == 1) {
                pos = true;
                evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
            } else if pos && ((st[i] == -1 && st[i - 1] == 1) || (on && mtrend[i] == -1)) {
                close(&mut evs, i, c[i], if st[i] == -1 && st[i - 1] == 1 { "Daily ST sell" } else { "Monthly ST filter down" });
                pos = false;
            }
        }
        return evs;
    }
    let (fast, slow) = (gu("fast"), gu("slow"));
    let kind = gi("pr_avg");
    let (pr, ef) = pr_lines(x, u, fast, kind, fast);
    let (_, es) = pr_lines(x, u, slow, kind, slow);
    let hp = if id == SF_HSL { g("hsl_pct") } else { 0.0 };
    let pr_filter = id == SF_HSL && gi("filter_type") == 2;
    let (fbuy, fexit): (Vec<bool>, Vec<bool>) = if pr_filter {
        let fp = gu("f_period");
        let (p1, l1) = pr_lines(x, u, fp, gi("f_avg"), fast);
        ((0..n).map(|i| lt(p1[i], l1[i])).collect(), vec![false; n])
    } else {
        // SF_HSL: FLTSHORT is a 0/1 value compared with -1, so its exit never fires (as written)
        ((0..n).map(|i| mtrend[i] == 1).collect(), (0..n).map(|i| id == SF_ST && mtrend[i] == -1).collect())
    };
    let mut abc = false;
    let mut hsl = 0.0;
    for i in fast.max(1)..n {
        let crossed = lt(pr[i], ef[i]) && gt(pr[i - 1], ef[i - 1]);
        if on && crossed {
            abc = true;
        }
        let above = gt(pr[i], ef[i]) && gt(pr[i], es[i]);
        if !pos && (if on { abc && fbuy[i] } else { crossed }) {
            pos = true;
            evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
            hsl = if hp > 0.0 { c[i] * (1.0 - hp / 100.0) } else { 0.0 };
        } else if pos && l[i] < hsl {
            close(&mut evs, i, if o[i] < hsl { o[i] * (1.0 - 0.0025) } else { hsl }, "Hard SL");
            pos = false;
            hsl = 0.0;
        } else if pos && (above || (on && fexit[i])) {
            close(&mut evs, i, c[i], if above { "PR above both lines" } else { "ST filter down" });
            pos = false;
            abc = false;
            hsl = 0.0;
        }
    }
    evs
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    let fam: &'static str = IDS.iter().copied().find(|&k| k == id)?;
    Some((fam, name, "USD INR"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    fn with_usd(r: &[crate::Row], usd: &[f64]) -> Ctx<'static> {
        let r: &'static [crate::Row] = Box::leak(r.to_vec().into_boxed_slice());
        let mut x = Ctx::new(r, r, &[]);
        for k in ["Open", "High", "Low", "Close"] {
            x.extra.insert(format!("USDINR.{k}"), usd.to_vec());
        }
        x
    }

    #[test]
    fn no_usdinr_no_trades() {
        let r = bars(&(0..60).map(|k| { let c = 100.0 + k as f64; (c, c + 1.0, c - 1.0, c) }).collect::<Vec<_>>());
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            assert!(run(id, &x, 0, r.len() - 1, &Params::new()).unwrap().is_empty(), "{id}");
        }
    }

    #[test]
    fn usdinr_gaps_repeat_the_previous_close() {
        let r = bars(&[(1.0, 1.0, 1.0, 1.0); 4]);
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert("USDINR.Close".into(), vec![f64::NAN, 70.0, f64::NAN, 72.0]);
        x.extra.insert("USDINR.High".into(), vec![f64::NAN, 71.0, f64::NAN, 73.0]);
        let [o, h, _, c] = usd(&x).unwrap();
        assert!(c[0].is_nan());
        assert_eq!((c[2], h[2], o[2]), (70.0, 70.0, 70.0));
        assert_eq!((h[1], o[1]), (71.0, 70.0));
    }

    #[test]
    fn lines_match_the_shared_supertrend() {
        let r = wave(300);
        let x = Ctx::new(&r, &r, &[]);
        let b = crate::books::ytsw::compress(&x, 'D');
        let (t1, d1) = crate::books::ytsw::supertrend(&calc_point(&b, 1), &b.h, &b.l, &b.c, 7, 2.0, 2.0);
        let (t2, _, d2) = supertrend_lines(&calc_point(&b, 1), &b.h, &b.l, &b.c, 7, 2.0);
        assert_eq!(t1, t2);
        assert_eq!(d1, d2);
    }

    #[test]
    fn averages() {
        let v: Vec<f64> = (1..=10).map(|k| k as f64).collect();
        assert_eq!(wma(&v, 3)[2], (1.0 + 4.0 + 9.0) / 6.0);
        assert_eq!(ema(&v, 3)[2], 2.0);
        assert_eq!(ema(&v, 3)[3], 3.0);
        // triangular: q = Round((4 + 1.1) / 2) = 3 -> EMA(SMA(v, 3), 3), first value at bar 4
        let t = avg(&v, 4, 4, 4);
        assert!(t[3].is_nan());
        assert_eq!(t[4], 3.0);
    }

    #[test]
    fn pr_base_reverses_on_the_ratio() {
        // index flat at 100; USDINR rises then falls: PR above its EMA ratio -> short, then below -> long
        let r = bars(&[(100.0, 101.0, 99.0, 100.0); 40]);
        let u: Vec<f64> = (0..40).map(|k| if k < 20 { 70.0 + k as f64 * 0.1 } else { 72.0 - (k - 20) as f64 * 0.2 }).collect();
        let x = with_usd(&r, &u);
        let t = run(PR_BASE, &x, 0, 39, &[("pr_period".to_string(), 3.0)].into_iter().collect()).unwrap();
        assert_eq!(t[0].dir, -1.0);
        assert_eq!(t[1].dir, 1.0);
        assert_eq!(t[0].exit_i, t[1].legs[0].i);
    }

    #[test]
    fn st_nifty_needs_usdinr_down_and_index_up() {
        // index rising (its ST up); USDINR falls sharply after bar 30 (its ST turns down) -> long at that close
        let rows: Vec<(f64, f64, f64, f64)> = (0..60).map(|k| { let c = 100.0 + k as f64; (c - 0.2, c + 0.5, c - 0.5, c) }).collect();
        let r = bars(&rows);
        let u: Vec<f64> = (0..60).map(|k| if k < 30 { 75.0 } else { 75.0 - (k - 29) as f64 }).collect();
        let x = with_usd(&r, &u);
        let t = run(ST_NIFTY, &x, 0, 59, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert!(t[0].legs[0].i >= 30 && t[0].open);
        assert_eq!(t[0].legs[0].px, r[t[0].legs[0].i].close);
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(500);
        let u: Vec<f64> = (0..r.len()).map(|k| 70.0 + (k as f64 / 9.0).sin() * 3.0 + k as f64 * 0.01).collect();
        let x = with_usd(&r, &u);
        for id in IDS {
            for (from, to) in [(0, r.len() - 1), (50, 300)] {
                for t in run(id, &x, from, to, &Params::new()).unwrap() {
                    assert!(t.legs[0].i >= from && t.exit_i <= to && t.legs[0].i <= t.exit_i, "{id}");
                }
            }
        }
    }
}
