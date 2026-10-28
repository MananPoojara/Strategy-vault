//! YT Strategies · Double 7: the team's AmiBroker AFLs in "Old Topics/Double 7 Strategy" (the 7-day lower low /
//! higher high studies and the generic "Lowest Low + Highest High" engine), run on an index's daily bars. Coded from
//! the AFLs as written, with each AFL's own default settings (an `Optimize()` uses its default); there is no book PDF
//! and nothing here is checked against the team's workbooks.
//!
//! AmiBroker behaviour the code follows (see `ytsw.rs` for the shared conventions):
//! - the AFLs run their own bar loop and set Buy / Sell (Short / Cover) and their prices; the backtester then takes
//!   an entry only when flat, ignores exits while flat, and on one bar exits first and then enters;
//! - inside a loop an array element that is Null is the number -1e10, so `C[i] > MA[i]` is true while the average
//!   is still Null and `C[i] < Ref(LLV(L, n), -1)[i]` is false there; whole-array comparisons are false before an
//!   array's first value (`ytsw.rs`);
//! - the template's guards, kept as written: no long entry on a bar whose high equals its close (no short entry when
//!   the low equals the close), no entry on bar 0, a price above the 0 "penny" level, turnover filter off, "dead
//!   stock" backtest off; an exit that falls on a bar whose low equals its close (for a short: high equals close) is
//!   deferred to the close of the first later bar where it does not;
//! - the SuperTrend filter is the team's ST loop (`ytsw::supertrend`, both bands at the ST factor) on the filter
//!   timeframe (daily / weekly Monday-Sunday / calendar month), brought back with `TimeFrameExpand(expandLast)`;
//! - prices are used as the AFL sets them (no clipping to the bar's range).
#![allow(dead_code)]
use crate::books::ytsw::st_filter;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_d7";
pub const LONG: &str = "ytd7_long";
pub const LONG_OLD: &str = "ytd7_long_old";
pub const LONG_CD: &str = "ytd7_long_cd";
pub const SHORT: &str = "ytd7_short";
pub const LONG_MA: &str = "ytd7_long_ma_st";
pub const LONG_MA_OLD: &str = "ytd7_long_ma_st_old";
pub const SHORT_MA: &str = "ytd7_short_ma_st";
pub const LLHH: &str = "ytd7_lowest_low_highest_high";
pub const IDS: [&str; 8] = [LONG, LONG_OLD, LONG_CD, SHORT, LONG_MA, LONG_MA_OLD, SHORT_MA, LLHH];

/// AFL Null inside a bar loop.
pub(crate) const NULL: f64 = -1e10;
const BIG: f64 = 9_999_999.0;

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Double 7", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(LONG, "Double 7 Long (current version)", "Long", &[
            "F|Filter version: off (default), ST (daily ST of the series, period 5, factor 4, mid point, must be up) or MA (close above its 200-day average)",
            "E|Buy at the close when the close is below the lowest low of the previous 7 days",
            "X|Sell on the 5% hard stop (low), the 10% profit target (high), 2 consecutive lower closes, a close above the previous 7 days' highest high after the wait, the ST turning down (ST mode) or a close below the MA (MA mode)",
            "X|Stop / target fills at the level, or at the open less the 0.25% buffer when the open is already through it; the stop is tested before the target",
        ], "7 Days Lower Low and Higher High Based Buy & Sell (With & Without Filter) - Long only.afl (folder root), coded as the AFL is written. As written the higher-high exit's wait counter only counts once it has reached the period (`Count1 >= HHV_Prd`, starting at 1), so with the default period 7 the higher-high exit never fires (with period 1 it does); in ST mode the exit tests `Count1 == HHV_Prd` instead of `>`. The buffer is subtracted also on a gap up through the target. The duplicate copy in AFL/ (…Long only_Old AFL.afl) differs only in brokerage and a parameter range and is not coded separately."),
        def(LONG_OLD, "Double 7 Long (old work)", "Long", &[
            "F|Filter version: off (default), ST (daily, period 5, factor 4, mid point) or MA (close above the 200-day average)",
            "E|Buy at the close when the close is below the lowest low of the previous 7 days",
            "X|Sell on the 5% hard stop, the 10% target, 2 consecutive lower closes (ST / MA modes only), a close above the previous 7 days' highest high from the 7th bar after entry, the ST turning down or a close below the MA",
        ], "7 Day Lower Low and Higher High_Old Work/…(With & Without Filter) - Long only.afl, coded as the AFL is written. In filter-off mode the AFL writes the down-close test as `Count2 = 0` (an assignment, which is false), so with the default filter off the consecutive-lower-close exit never fires. Stop before target when both are hit."),
        def(LONG_CD, "Double 7 Long (LL, HH, CD version)", "Long", &[
            "F|Filter version: off (default), ST (daily, period 5, factor 4, mid point) or MA (close above the 200-day average)",
            "E|Buy at the close when the close is below the lowest low of the previous 7 days",
            "X|Sell on the 10% target, the 5% hard stop, 2 consecutive lower closes, a close above the previous 7 days' highest high from the 7th bar after entry, the ST turning down or a close below the MA",
        ], "7 Day LL, HH, CD/…(With & Without Filter) - Long only with MA Filter.afl, coded as the AFL is written. When the stop and the target are both hit on one bar the AFL prices the exit at the target (target tested first). The MA period default 200 lies outside its own slider range (20-80); the default is used."),
        def(SHORT, "Double 7 Short", "Short", &[
            "F|Filter version: off (default), ST (daily, period 5, factor 4, mid point, must be down) or MA (close below the 200-day average)",
            "E|Short at the close when the close is above the highest high of the previous 7 days",
            "X|Cover on the 10% target (low), the 5% hard stop (high), 2 consecutive higher closes, a close below the previous 7 days' lowest low from the 7th bar after entry, the ST turning up or a close above the MA",
        ], "7 Day Lower Low and Higher High_Old Work/…(With & Without Filter) - Short only.afl, coded as the AFL is written (its Optimize() defaults). Target tested before the stop for the cover price; a cover on a bar whose high equals its close is deferred. The entry warm-up counter counts to the exit period but the entry tests it against the entry period, as written."),
        def(LONG_MA, "Double 7 Long with MA Filter, optional ST (LL, HH, CD)", "Long", &[
            "F|Close above its 200-day average in both modes; ST mode (off by default) also needs the daily ST (5, 4, mid point) up",
            "E|Buy at the close when the close is below the lowest low of the previous 7 days",
            "X|Sell on the 10% target, the 5% hard stop, 2 consecutive lower closes, a close above the previous 7 days' highest high from the 7th bar after entry, or the ST turning down (ST mode); no MA exit",
        ], "7 Day LL, HH, CD/…(With & Without ST Filter) - Long only.afl, coded as the AFL is written (its Optimize() defaults 7 / 7 / 2). The MA filter applies with the filter off too; target tested before the stop. While the 200-day average is still Null the AFL's loop treats it as -1e10, so the MA test passes during the first 199 bars."),
        def(LONG_MA_OLD, "Double 7 Long with MA Filter, optional ST (old work)", "Long", &[
            "F|Close above its 200-day average in both modes; ST mode (off by default) also needs the daily ST (5, 4, mid point) up",
            "E|Buy at the close when the close is below the lowest low of the previous 7 days",
            "X|Sell on the 5% hard stop, the 10% target, 2 consecutive lower closes, any close above the previous 7 days' highest high (no wait), or the ST turning down (ST mode)",
        ], "7 Day Lower Low and Higher High_Old Work/…(With & Without ST Filter) - Long only.afl, coded as the AFL is written: besides the waited higher-high exit it also exits on any close above the previous 7-day high, so the wait has no effect. Stop tested before the target."),
        def(SHORT_MA, "Double 7 Short with MA Filter, optional ST", "Short", &[
            "F|Close below its 200-day average in both modes; ST mode (off by default) also needs the daily ST (5, 4, mid point) down",
            "E|Short at the close when the close is above the highest high of the previous 7 days",
            "X|Cover on the 5% hard stop, the 10% target, 2 consecutive higher closes, a close below the previous 7 days' lowest low from the 7th bar after entry, or the ST turning up (ST mode); no MA exit",
        ], "7 Day Lower Low and Higher High_Old Work/…(With & Without ST Filter) - Short only.afl, coded as the AFL is written. Stop tested before the target for the cover price. While the average is Null the loop's MA test (close below -1e10) fails, so no short before the 200-day average exists."),
        def(LLHH, "Lowest Low + Highest High (generic)", "Long", &[
            "E|Buy at the close when the close (or low, by entry type) is below the previous N days' lowest low (or lowest close); N default 1",
            "X|Sell at the close when the close (or high) is above the previous N days' highest high (or highest close); N default 1",
            "X|Optional: N-candle / green-candle / up-close count exit, % profit booking, hard and trailing stops; an exit on a bar with a new entry signal keeps the position",
        ], "Double 7 Strategy/AFL/LowestLow+HighestHigh.afl, coded as the AFL is written. Its stop, target and trailing levels are stored only on the entry bar and never carried forward, so later bars compare with Null: the hard and trailing stops never fire and a profit-booking % above 0 exits on the very next bar (at the open, or the low if the buffer takes it below). The count exit's default N = 0 lies below its slider minimum and never fires. The AFL exits everything on the last data bar, shown here as an open position."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let st = || vec![
        p("st_tf", "ST filter timeframe (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
        p("st_src", "ST calculation point (0 mid point, 1 close, 2 pivot)", 0.0, 0.0, 2.0, 1.0),
        p("st_period", "ST period", 5.0, 5.0, 25.0, 1.0),
        p("st_factor", "ST factor", 4.0, 1.0, 7.0, 1.0),
    ];
    let common = |short: bool, three: bool| {
        let mut v = vec![if three {
            p("filter_version", "Filter version (0 off, 1 ST, 2 MA)", 0.0, 0.0, 2.0, 1.0)
        } else {
            p("filter_version", "Filter version (0 off, 1 ST)", 0.0, 0.0, 1.0, 1.0)
        }];
        if short {
            v.push(p("hhv_prd", "Highest high period for entry", 7.0, 1.0, 1000.0, 1.0));
            v.push(p("llv_prd", "Lowest low period for exit", 7.0, 1.0, 1000.0, 1.0));
            v.push(p("upclose_prd", "Consecutive up closes for exit (0 off)", 2.0, 0.0, 1000.0, 1.0));
        } else {
            v.push(p("llv_prd", "Lowest low period for entry", 7.0, 1.0, 1000.0, 1.0));
            v.push(p("hhv_prd", "Highest high period for exit", 7.0, 1.0, 1000.0, 1.0));
            v.push(p("dnclose_prd", "Consecutive down closes for exit (0 off)", 2.0, 0.0, 1000.0, 1.0));
        }
        v.extend([
            p("buffer_pct", "Buffer % on a gap through the stop / target", 0.25, 0.0, 10.0, 0.05),
            p("profit_pct", "Profit booking % (0 off)", 10.0, 0.0, 50.0, 0.1),
            p("hard_sl_pct", "Hard SL % (0 off)", 5.0, 0.0, 50.0, 0.01),
            p("ma_prd", "MA period", 200.0, 1.0, 600.0, 1.0),
        ]);
        v.extend(st());
        v
    };
    Some(match id {
        LONG | LONG_OLD | LONG_CD => common(false, true),
        SHORT => common(true, true),
        LONG_MA | LONG_MA_OLD => common(false, false),
        SHORT_MA => common(true, false),
        LLHH => vec![
            p("entry_type", "Entry (0 close below lowest low, 1 close below lowest close, 2 low below lowest low)", 0.0, 0.0, 2.0, 1.0),
            p("lookback_ll", "Last N days for entry", 1.0, 1.0, 100.0, 1.0),
            p("exit_type", "Exit (0 close above highest high, 1 close above highest close, 2 high above highest high)", 0.0, 0.0, 2.0, 1.0),
            p("lookback_hh", "Last N days for exit", 1.0, 1.0, 100.0, 1.0),
            p("exit_candle_type", "Count exit candle (0 every candle, 1 green candle, 2 up close)", 0.0, 0.0, 2.0, 1.0),
            p("n_exit_candles", "N candles for the count exit", 0.0, 0.0, 5.0, 1.0),
            p("exit_at", "Stop / target test (0 live: high / low, 1 close)", 0.0, 0.0, 1.0, 1.0),
            p("pb_pct", "Profit booking % (0 off)", 0.0, 0.0, 7.0, 1.0),
            p("hsl_pct", "Hard SL % (0 off)", 0.0, 0.0, 7.0, 1.0),
            p("tsl_pct", "Trailing SL % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("buffer_pct", "Buffer %", 0.0, 0.0, 10.0, 0.1),
        ],
        _ => return None,
    })
}

// ---------------------------------------------------------------- shared helpers (also used by yteng / ytib)

/// `Ref(LLV(v, n), -1)` / `Ref(HHV(v, n), -1)`: None until n earlier bars exist.
pub(crate) fn prev_ext(v: &[f64], n: usize, high: bool) -> Vec<Option<f64>> {
    let n = n.max(1);
    (0..v.len())
        .map(|i| {
            if i < n {
                return None;
            }
            let w = v[i - n..i].iter().copied();
            Some(if high { w.fold(f64::MIN, f64::max) } else { w.fold(f64::MAX, f64::min) })
        })
        .collect()
}

/// AFL `MA(C, n)`: None until n bars exist.
pub(crate) fn ma(c: &[f64], n: usize) -> Vec<Option<f64>> {
    crate::books::ytsw::sma(c, n)
}

/// An array element as a bar loop reads it (Null = -1e10).
pub(crate) fn lv(v: Option<f64>) -> f64 {
    v.unwrap_or(NULL)
}

/// AFL `Prec(v, 2)` (truncates to 2 decimals).
pub(crate) fn prec2(v: f64) -> i64 {
    (v * 100.0 + 1e-7).floor() as i64
}

pub(crate) type Exit = Option<(f64, &'static str)>;

pub(crate) fn mk(dir: f64, e: usize, px: f64, kind: &'static str, exit: usize, exit_px: f64, reason: &str, open: bool) -> Trade {
    Trade { dir, legs: vec![Leg { i: e, px, w: 1.0 }], exit_i: exit, exit_px, reason: reason.into(), entry_kind: kind, signal_i: exit, open, exits: Vec::new() }
}

/// AmiBroker backtest of entry / exit signals that carry their own prices, one position at a time over from..=to.
/// On a bar the open position exits first (an `ApplyStop` percent loss stop, when `stop_pct` > 0, before the exit
/// signal; from the bar after entry, at the stop or the open if it gapped through); then a flat book enters; with
/// `same_bar` (AllowSameBarExit) a position entered on the bar also takes that bar's exit signal. An exit signal
/// named "Last bar" (the AFL's forced exit on the final data bar) is reported as an open position.
pub(crate) fn fills(x: &Ctx, dir: f64, entry: &[Option<f64>], exit: &[Exit], from: usize, to: usize, kind: &'static str, same_bar: bool, stop_pct: f64) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut pos: Option<(usize, f64)> = None;
    let close = |out: &mut Vec<Trade>, e: usize, epx: f64, i: usize, px: f64, why: &str| {
        let open = why == "Last bar";
        out.push(mk(dir, e, epx, kind, i, if open { x.c[i] } else { px }, if open { "open" } else { why }, open));
    };
    for i in from..=to.min(x.c.len().saturating_sub(1)) {
        if let Some((e, epx)) = pos {
            let lvl = epx * (1.0 - stop_pct / 100.0);
            if stop_pct > 0.0 && dir > 0.0 && i > e && x.l[i] <= lvl {
                close(&mut out, e, epx, i, if x.o[i] < lvl { x.o[i] } else { lvl }, "Hard stop (ApplyStop)");
                pos = None;
            } else if let Some((px, why)) = exit[i] {
                close(&mut out, e, epx, i, px, why);
                pos = None;
            }
        }
        if pos.is_none() {
            if let Some(px) = entry[i] {
                pos = Some((i, px));
                if same_bar {
                    if let Some((spx, why)) = exit[i] {
                        close(&mut out, i, px, i, spx, why);
                        pos = None;
                    }
                }
            }
        }
    }
    if let Some((e, epx)) = pos {
        out.push(mk(dir, e, epx, kind, to, x.c[to], "open", true));
    }
    out
}

pub(crate) fn tf_char(code: i64) -> char {
    match code {
        1 => 'W',
        2 => 'M',
        _ => 'D',
    }
}

// ---------------------------------------------------------------- the 7-day lower low / higher high loops

#[derive(Clone, Copy)]
struct Var {
    short: bool,
    /// "Off|ST|MA" list (MA mode with its MA exit); false = "Off|ST" with the MA filter always on
    three: bool,
    /// the higher-high wait counter increments while `Count1 >= HHV_Prd` (current file) instead of `<=`
    count_ge: bool,
    /// ST mode tests `Count1 == HHV_Prd` for the higher-high exit (current file)
    st_eq: bool,
    /// also exits on any close above the previous highest high (old-work ST long)
    hh_always: bool,
    /// the target is tested before the stop for the exit price
    pt_first: bool,
    /// filter-off mode's `Count2 = 0` assignment (old-work long)
    off_count2_bug: bool,
}

fn variant(id: &str) -> Option<Var> {
    let v = Var { short: false, three: true, count_ge: false, st_eq: false, hh_always: false, pt_first: false, off_count2_bug: false };
    Some(match id {
        LONG => Var { count_ge: true, st_eq: true, ..v },
        LONG_OLD => Var { off_count2_bug: true, ..v },
        LONG_CD => Var { pt_first: true, ..v },
        SHORT => Var { short: true, pt_first: true, ..v },
        LONG_MA => Var { three: false, pt_first: true, ..v },
        LONG_MA_OLD => Var { three: false, hh_always: true, ..v },
        SHORT_MA => Var { short: true, three: false, ..v },
        _ => return None,
    })
}

fn double7(x: &Ctx, v: Var, g: &dyn Fn(&str) -> f64) -> (Vec<Option<f64>>, Vec<Exit>) {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (llv, hhv) = (gu("llv_prd"), gu("hhv_prd"));
    let run_n = g(if v.short { "upclose_prd" } else { "dnclose_prd" }).round();
    let buf = g("buffer_pct");
    let (pp, sp) = (g("profit_pct"), g("hard_sl_pct"));
    let mode = (g("filter_version").round() as i64).clamp(0, if v.three { 2 } else { 1 });
    let ll: Vec<f64> = prev_ext(l, llv, false).into_iter().map(lv).collect();
    let hh: Vec<f64> = prev_ext(h, hhv, true).into_iter().map(lv).collect();
    let mav: Vec<f64> = ma(c, gu("ma_prd")).into_iter().map(lv).collect();
    let mt: Vec<i8> = if mode == 1 {
        st_filter(x, tf_char(g("st_tf").round() as i64), g("st_src").round() as i64 + 1, gu("st_period"), g("st_factor"))
    } else {
        vec![1; n]
    };
    let mut buy = vec![None; n];
    let mut sell: Vec<Exit> = vec![None; n];
    let (mut pos, mut skip) = (false, false);
    let (mut cnt, mut cnt1, mut cnt2) = (1usize, 0usize, 0usize);
    // long: HSL 0, PT 9999999, Dn_Close 0; short: PT 0, HSL 9999999, Up_Close 9999999
    let (mut hsl, mut pt, mut run) = if v.short { (BIG, 0.0, BIG) } else { (0.0, BIG, 0.0) };
    let reset = |hsl: &mut f64, pt: &mut f64, run: &mut f64| {
        if v.short {
            *hsl = BIG;
            *pt = 0.0;
            *run = BIG;
        } else {
            *hsl = 0.0;
            *pt = BIG;
            *run = 0.0;
        }
    };
    for i in 1..n {
        if !pos && cnt <= llv {
            cnt += 1;
        }
        let gate = cnt > if v.short { hhv } else { llv };
        let side_ma = if v.short { c[i] < mav[i] } else { c[i] > mav[i] };
        let filt = match mode {
            1 => mt[i] == if v.short { -1 } else { 1 },
            2 => side_ma,
            _ => true,
        } && (v.three || side_ma);
        let brk = if v.short { c[i] > hh[i] } else { c[i] < ll[i] };
        let guard = c[i] > 0.0 && if v.short { l[i] != c[i] } else { h[i] != c[i] };
        let stuck = if v.short { h[i] == c[i] } else { l[i] == c[i] };
        if !pos && gate && brk && filt && guard {
            buy[i] = Some(c[i]);
            if pp > 0.0 {
                pt = c[i] * if v.short { 1.0 - pp * 0.01 } else { 1.0 + pp * 0.01 };
            }
            if sp > 0.0 {
                hsl = c[i] * if v.short { 1.0 + sp * 0.01 } else { 1.0 - sp * 0.01 };
            }
            pos = true;
            cnt1 = 1;
        } else if pos && !skip {
            // consecutive lower (short: higher) closes; an equal close keeps the count
            let worse = |a: f64, b: f64| if v.short { a > b } else { a < b };
            if v.off_count2_bug && mode == 0 {
                cnt2 = 0;
            } else if worse(c[i], c[i - 1]) && cnt2 == 0 {
                run = c[i];
                cnt2 = 1;
            } else if worse(c[i], run) && cnt2 > 0 {
                run = c[i];
                cnt2 += 1;
            } else if worse(run, c[i]) && cnt2 > 0 {
                run = if v.short { BIG } else { 0.0 };
                cnt2 = 0;
            }
            let lim = if v.short { llv } else { hhv };
            if if v.count_ge { cnt1 >= lim } else { cnt1 <= lim } {
                cnt1 += 1;
            }
            let waited = if mode == 1 && v.st_eq { cnt1 == lim } else { cnt1 > lim };
            let (stop_hit, tgt_hit, swing) = if v.short {
                (h[i] >= hsl, l[i] <= pt, (waited && c[i] < ll[i]) || (v.hh_always && c[i] < ll[i]))
            } else {
                (l[i] <= hsl, h[i] >= pt, (waited && c[i] > hh[i]) || (v.hh_always && c[i] > hh[i]))
            };
            let streak = run_n > 0.0 && cnt2 as f64 == run_n;
            let st_out = mode == 1 && mt[i] == if v.short { 1 } else { -1 };
            let ma_out = mode == 2 && if v.short { c[i] > mav[i] } else { c[i] < mav[i] };
            if stop_hit || tgt_hit || swing || streak || st_out || ma_out {
                if !stuck {
                    let b = o[i] * buf / 100.0;
                    let (stop_px, tgt_px) = if v.short {
                        (if o[i] > hsl { o[i] + b } else { hsl }, if o[i] < pt { o[i] + b } else { pt })
                    } else {
                        (if o[i] < hsl { o[i] - b } else { hsl }, if o[i] > pt { o[i] - b } else { pt })
                    };
                    let e = if v.pt_first {
                        if tgt_hit { (tgt_px, "Profit target") } else if stop_hit { (stop_px, "Hard SL") } else { (c[i], "Exit rule") }
                    } else if stop_hit {
                        (stop_px, "Hard SL")
                    } else if tgt_hit {
                        (tgt_px, "Profit target")
                    } else {
                        (c[i], "Exit rule")
                    };
                    sell[i] = Some(e);
                    pos = false;
                    cnt1 = 0;
                    cnt2 = 0;
                    reset(&mut hsl, &mut pt, &mut run);
                } else {
                    skip = true;
                }
            }
        } else if skip && !stuck {
            sell[i] = Some((c[i], "Deferred exit"));
            pos = false;
            skip = false;
            cnt1 = 0;
            cnt2 = 0;
            reset(&mut hsl, &mut pt, &mut run);
        }
    }
    (buy, sell)
}

// ---------------------------------------------------------------- Lowest Low + Highest High

fn llhh(x: &Ctx, g: &dyn Fn(&str) -> f64) -> (Vec<Option<f64>>, Vec<Exit>) {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let (ne, nx) = (gi("lookback_ll").max(1) as usize, gi("lookback_hh").max(1) as usize);
    let (r_ll, r_lc, r_hh, r_hc) = (prev_ext(l, ne, false), prev_ext(c, ne, false), prev_ext(h, nx, true), prev_ext(c, nx, true));
    let lt = |a: f64, b: Option<f64>| b.map_or(false, |b| a < b);
    let gt = |a: f64, b: Option<f64>| b.map_or(false, |b| a > b);
    let entry: Vec<bool> = (0..n).map(|i| match gi("entry_type") { 1 => lt(c[i], r_lc[i]), 2 => lt(l[i], r_ll[i]), _ => lt(c[i], r_ll[i]) }).collect();
    let exit: Vec<bool> = (0..n).map(|i| match gi("exit_type") { 1 => gt(c[i], r_hc[i]), 2 => gt(h[i], r_hh[i]), _ => gt(c[i], r_hh[i]) }).collect();
    let (ctype, nexit, at_close) = (gi("exit_candle_type"), g("n_exit_candles").round(), gi("exit_at") == 1);
    let (pbp, hslp, tslp, buf) = (g("pb_pct"), g("hsl_pct"), g("tsl_pct"), g("buffer_pct"));
    // the AFL's level arrays: written on the entry bar only (Null elsewhere)
    let (mut pb, mut hsl, mut tsl) = (vec![NULL; n], vec![NULL; n], vec![NULL; n]);
    let mut buy = vec![None; n];
    let mut sell: Vec<Exit> = vec![None; n];
    let (mut pos, mut flag, mut count) = (false, false, 0.0);
    for i in ne..n {
        if pos {
            let lo = if at_close { c[i] } else { l[i] };
            let hi = if at_close { c[i] } else { h[i] };
            let hsl_hit = hslp > 0.0 && lo <= hsl[i];
            let pb_hit = pbp > 0.0 && hi >= pb[i];
            let tsl_hit = tslp > 0.0 && lo <= tsl[i];
            match ctype {
                1 if o[i] < c[i] => count += 1.0,
                2 if i > 0 && c[i] > c[i - 1] => count += 1.0,
                0 => count += 1.0,
                _ => {}
            }
            let cnt_exit = count == nexit;
            let last = i == n - 1;
            if last || exit[i] || hsl_hit || pb_hit || tsl_hit || cnt_exit {
                if entry[i] && !flag {
                    flag = true;
                } else {
                    let gap = |lvl: f64, above: bool| if (above && o[i] > lvl) || (!above && o[i] < lvl) { l[i].max(o[i] * (1.0 - buf * 0.01)) } else { lvl };
                    let mut px = (c[i], if last && !(exit[i] || hsl_hit || pb_hit || tsl_hit || cnt_exit) { "Last bar" } else { "Exit rule" });
                    if pb_hit && !at_close {
                        px = (gap(pb[i], true), "Profit booking");
                    }
                    if hsl_hit && !at_close {
                        px = (gap(hsl[i], false), "Hard SL");
                    }
                    if tsl_hit && !at_close {
                        px = (gap(tsl[i], false), "Trailing SL");
                    }
                    sell[i] = Some(px);
                    pos = false;
                    pb[i] = NULL;
                    hsl[i] = NULL;
                    tsl[i] = NULL;
                }
            }
            if tslp > 0.0 {
                tsl[i] = if tsl[i] > 0.0 { (h[i] * (1.0 - tslp * 0.01)).max(tsl[i - 1]) } else { NULL };
            }
        }
        if (!pos || flag) && entry[i] {
            if !flag {
                buy[i] = Some(c[i]);
            }
            hsl[i] = if hslp > 0.0 { c[i] * (1.0 - hslp * 0.01) } else { NULL };
            pb[i] = if pbp > 0.0 { c[i] * (1.0 + pbp * 0.01) } else { NULL };
            tsl[i] = if tslp > 0.0 { c[i] * (1.0 - tslp * 0.01) } else { NULL };
            pos = true;
            count = 0.0;
            flag = false;
        }
    }
    (buy, sell)
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    Some(if id == LLHH {
        let (b, s) = llhh(x, &g);
        fills(x, 1.0, &b, &s, from, to, "close", false, 0.0)
    } else {
        let v = variant(id)?;
        let (b, s) = double7(x, v, &g);
        fills(x, if v.short { -1.0 } else { 1.0 }, &b, &s, from, to, "close", false, 0.0)
    })
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !IDS.contains(&id) {
        return None;
    }
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        LONG => "ytd7_long",
        LONG_OLD => "ytd7_long_old",
        LONG_CD => "ytd7_long_cd",
        SHORT => "ytd7_short",
        LONG_MA => "ytd7_long_ma_st",
        LONG_MA_OLD => "ytd7_long_ma_st_old",
        SHORT_MA => "ytd7_short_ma_st",
        _ => "ytd7_llhh",
    }, name, "Double 7 Strategy"))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::books::ytmr::day_number;
    use crate::Row;

    pub(crate) fn civil(z: i64) -> String {
        let z = z + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{:04}-{:02}-{:02}", yoe + era * 400 + i64::from(m <= 2), m, d)
    }
    /// Consecutive weekdays from Monday 2024-01-01; bars are (open, high, low, close).
    pub(crate) fn bars(ohlc: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        let mut d = day_number("2024-01-01").unwrap();
        ohlc.iter()
            .map(|&(open, high, low, close)| {
                let z = d;
                d += if (z + 4).rem_euclid(7) == 5 { 3 } else { 1 };
                Row { ticker: "T".into(), date: civil(z), open, high, low, close, volume: None }
            })
            .collect()
    }
    pub(crate) fn wave(n: usize) -> Vec<Row> {
        let v: Vec<(f64, f64, f64, f64)> = (0..n)
            .map(|k| {
                let c = 100.0 + 12.0 * ((k as f64) / 6.0).sin() + 4.0 * ((k as f64) / 2.3).cos() + k as f64 * 0.03;
                let o = c - 1.5 * ((k as f64) / 1.7).sin();
                (o, o.max(c) + 1.0 + (k % 3) as f64 * 0.4, o.min(c) - 1.0 - (k % 4) as f64 * 0.3, c)
            })
            .collect();
        bars(&v)
    }
    pub(crate) fn ps(kv: &[(&str, f64)]) -> Params {
        kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }
    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<(usize, f64, usize, f64, bool)> {
        let x = Ctx::new(r, r, &[]);
        run(id, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.open)).collect()
    }
    /// A flat bar around `c` (high and low 1 away, so neither equals the close).
    fn b(c: f64) -> (f64, f64, f64, f64) {
        (c, c + 1.0, c - 1.0, c)
    }

    #[test]
    fn long_buys_below_seven_day_low_and_exits_on_two_lower_closes() {
        // bars 0-7 at 100 (lows 99); bar 8 closes 97 < 99 -> buy; bar 9 98 (up), bar 10 96, bar 11 95 -> 2 lower closes
        let mut v: Vec<_> = (0..8).map(|_| b(100.0)).collect();
        v.extend([b(97.0), b(98.0), b(96.0), b(95.0), b(95.5)]);
        let r = bars(&v);
        assert_eq!(go(LONG, &r, &[]), vec![(8, 97.0, 11, 95.0, false)]);
        // the old-work AFL's filter-off down-close test never counts: still open at the end
        assert_eq!(go(LONG_OLD, &r, &[]), vec![(8, 97.0, 12, 95.5, true)]);
    }

    #[test]
    fn higher_high_exit_dead_in_current_version_but_works_in_others() {
        // buy at bar 8 (97); then closes rise above the previous 7-day high; no lower closes
        let mut v: Vec<_> = (0..8).map(|_| b(100.0)).collect();
        v.push(b(97.0));
        for k in 0..9 {
            v.push(b(98.0 + k as f64 * 0.5));
        }
        v.push(b(104.0)); // bar 18: 104 > highest high of bars 11-17 (103)
        let r = bars(&v);
        let cur = go(LONG, &r, &[("profit_pct", 0.0)]);
        assert!(cur[0].4, "current version: higher-high exit never fires");
        let cd = go(LONG_CD, &r, &[("profit_pct", 0.0)]);
        assert_eq!((cd[0].0, cd[0].2, cd[0].4), (8, 18, false));
        // with period 1 the current version's counter runs
        let cur1 = go(LONG, &r, &[("profit_pct", 0.0), ("hhv_prd", 1.0)]);
        assert_eq!((cur1[0].0, cur1[0].2, cur1[0].4), (8, 18, false));
    }

    #[test]
    fn stop_and_target_pricing_and_lc_deferral() {
        let mut v: Vec<_> = (0..8).map(|_| b(100.0)).collect();
        v.push(b(97.0)); // buy 97: stop 92.15, target 106.7
        v.push((90.0, 91.0, 89.0, 90.5)); // opens below the stop: 90 - 0.25%
        let r = bars(&v);
        let t = go(LONG, &r, &[]);
        assert!((t[0].3 - 90.0 * 0.9975).abs() < 1e-9 && t[0].2 == 9);
        // stop and target both hit: stop first in the current file, target first in the CD file
        let mut v2 = v[..9].to_vec();
        v2.push((100.0, 110.0, 90.0, 100.5));
        let r2 = bars(&v2);
        assert!((go(LONG, &r2, &[])[0].3 - 97.0 * 0.95).abs() < 1e-9);
        assert!((go(LONG_CD, &r2, &[])[0].3 - 97.0 * 1.10).abs() < 1e-9);
        // an exit bar whose low equals its close is deferred to the next bar's close
        let mut v3 = v[..9].to_vec();
        v3.extend([(97.0, 98.0, 91.0, 91.0), b(93.0)]);
        let t3 = go(LONG, &bars(&v3), &[]);
        assert_eq!((t3[0].2, t3[0].3), (10, 93.0));
    }

    #[test]
    fn short_mirror() {
        let mut v: Vec<_> = (0..8).map(|_| b(100.0)).collect();
        v.extend([b(103.0), b(102.0), b(104.0), b(105.0)]);
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(SHORT, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (-1.0, 8, 11, 105.0));
    }

    #[test]
    fn llhh_defaults_and_unforwarded_profit_level() {
        // close below the previous low buys; close above the previous high sells
        let r = bars(&[b(100.0), b(98.5), b(99.0), b(100.5), b(100.0)]);
        assert_eq!(go(LLHH, &r, &[]), vec![(1, 98.5, 3, 100.5, false)]);
        // PB % > 0 compares with a Null level on the next bar: exits at bar 2's open
        let t = go(LLHH, &r, &[("pb_pct", 3.0)]);
        assert_eq!((t[0].0, t[0].2, t[0].3), (1, 2, 99.0));
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            for fv in [0.0, 1.0, 2.0] {
                let t = run(id, &x, 50, r.len() - 1, &ps(&[("filter_version", fv), ("st_tf", fv)])).unwrap();
                assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= 50), "{id}");
            }
            assert!(family_of(id).is_some());
        }
        assert_eq!(catalogue().len(), IDS.len());
    }
}
