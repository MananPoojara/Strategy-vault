//! YT Strategies · Inside Bar: the team's AmiBroker AFLs in "Old Topics/Inside Bar", run on an index's daily bars
//! ("intraday" in the file names means an entry and exit on the same daily bar; the weekly AFLs find the inside bar
//! on Monday-Sunday weeks and trade it on daily bars). Coded from the AFLs as written, with their default settings;
//! there is no book PDF and nothing here is checked against the team's workbooks.
//!
//! AmiBroker behaviour followed (see `ytsw.rs`): `Inside()` = high below and low above the previous bar's; the
//! entry is a stop at the inside bar's high (`BuyPrice` = previous high, or the open + 0.25% when the bar opens above
//! it); `SetOption("AllowSameBarExit", True)` lets a position entered on a bar take that bar's exit; the AFLs without
//! `ExRem` rely on the backtester ignoring signals while in a trade / flat; the weekly AFLs apply
//! `ExRem(Buy, Sell)` then `ExRem(Sell, Buy)`. Weekly values reach daily bars with `TimeFrameExpand(expandLast)`
//! (`ytsw::expand_last`). The optional `ApplyStop(stopTypeLoss, percent)` (default 0 = off, stops not active on the
//! entry bar) exits at the stop or at the open when the open is through it, before the regular exit. The template's
//! guards are kept: no entry on bar 0, none on an "upper circuit" bar (open, high and close equal to 2 decimals), no
//! weekly / ST-stop exit on a "lower circuit" bar; turnover filter off. Prices are used as the AFL sets them, then
//! kept inside the bar's low-high range (AmiBroker's price bound check: a gap fill at open +/- 0.25% beyond the bar
//! fills at its high / low). A bar whose sell signal closes one trade does not also close a trade entered on that bar
//! (AmiBroker uses the signal once); a trade entered from flat can still take its entry bar's exit. Both rules were
//! found by comparing with the team's Nifty / BankNifty trade lists (jay-local/Matches/Inside Bar.xlsx).
#![allow(dead_code)]
use crate::books::ytd7::{lv, ma, mk, prec2, Exit};
use crate::books::ytsw::{calc_point, compress, expand_last, st_filter, supertrend, Bars};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_ib";
pub const INTRADAY: &str = "ytib_intraday";
pub const ATHC: &str = "ytib_intraday_athc";
pub const INTRADAY_FLT: &str = "ytib_intraday_filter";
pub const PREV_LOW: &str = "ytib_prev_low_sl";
pub const ST_SL: &str = "ytib_st_sl";
pub const WEEKLY: &str = "ytib_weekly";
pub const WEEKLY_PLOT: &str = "ytib_weekly_plot";
pub const IDS: [&str; 7] = [INTRADAY, ATHC, INTRADAY_FLT, PREV_LOW, ST_SL, WEEKLY, WEEKLY_PLOT];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Inside Bar", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(INTRADAY, "Inside Bar Breakout, Same-Day Exit", &[
            "E|After an inside bar, buy when the next bar's high exceeds the inside bar's high: at that high, or at the open + 0.25% on a gap above it",
            "X|Sell at the same bar's close",
        ], "INSIDE BAR LONG ONLY INTRADAY - FINAL.afl, coded as the AFL is written (daily bars; the optional % hard stop never acts because each trade ends on its entry bar)."),
        def(ATHC, "Inside Bar Breakout, Same-Day Exit, All-Time-High Regime", &[
            "F|Regime on from a close above every earlier close until the daily ST (period 10, factor 4, on the close) turns down; the previous bar's regime must be on",
            "E|As the same-day inside bar breakout (no circuit guard in this AFL)",
            "X|Sell at the same bar's close",
        ], "INSIDE BAR LONG ONLY INTRADAY ON ATHC - Copy.afl, coded as the AFL is written. Its ST upper band uses factor 1 and only the lower band the ST factor. `HHV(C, BarCount)` is taken as the highest close so far (as in ytsw REGIM). The 'trading start bar' parameter is unused by the AFL."),
        def(INTRADAY_FLT, "Inside Bar Breakout, Same-Day Exit, Trend Filter", &[
            "F|Previous bar's filter must be up: the series' own ST (default monthly, period 5, factor 1, mid point) or SMA mode (close vs the previous 200-bar average, daily timeframe only)",
            "E|As the same-day inside bar breakout",
            "X|Sell at the same bar's close",
        ], "INSIDE BAR LONG ONLY INTRADAY with filter - Copy.afl, coded as the AFL is written. Held: the SMA filter on a weekly or monthly timeframe (the AFL uses the compressed array on daily bars without TimeFrameExpand, so its alignment is unresolved) and the S% filter (it reads an undefined trend array, an AFL error); those settings return no result. The swing % parameters are unused."),
        def(PREV_LOW, "Inside Bar Breakout, Previous-Low Stop", &[
            "F|Optional trend filter as in the filter AFL (default none)",
            "E|As the inside bar breakout",
            "X|Sell when the low breaks the previous bar's low: at that low, or the open - 0.25% on a gap below; on the entry bar this is the inside bar's low (same-bar exit allowed)",
        ], "INSIDE BAR LONG ONLY with filter AND previos bar low sl - Copy.afl, coded as the AFL is written. The daily ST for the stop is computed by the AFL but not used here. SMA-on-weekly/monthly and S% filters are held as in the filter AFL."),
        def(ST_SL, "Inside Bar Breakout, ST or Inside-Low Stop", &[
            "F|Optional trend filter as in the filter AFL (default none)",
            "E|As the inside bar breakout",
            "X|If yesterday's daily ST (period 5, factor 1, mid point) was up: sell at the close when it turns down; otherwise sell when the low breaks the latest inside bar's low, at that low or the open - 0.25% on a gap; no exit on a lower-circuit bar",
        ], "INSIDE BAR LONG ONLY with filter AND ST SL.afl, coded as the AFL is written. The stop level follows the latest inside bar, so it moves if a new inside bar forms during the trade. SMA-on-weekly/monthly and S% filters are held as in the filter AFL."),
        def(WEEKLY, "Weekly Inside Bar Breakout", &[
            "E|When the last completed week was an inside week, buy when the day's high exceeds that week's high: at the week's high, or the open + 0.25% on a gap above it",
            "X|If yesterday's daily ST (period 5, factor 1, mid point) was up: sell at the close when it turns down; otherwise sell when the low breaks the last completed week's low, at that low or the open - 0.25% on a gap; no exit on a lower-circuit bar",
        ], "INSIDE BAR LONG ONLY ON WEEKLY - Copy.afl, coded as the AFL is written (ExRem on both signals). The stop low is the last completed week's low, whichever week that is, so it changes weekly during the trade. The AFL's ST here is symmetric (both bands at the ST factor)."),
        def(WEEKLY_PLOT, "Weekly Inside Bar Breakout (plot version)", &[
            "E|Same signal as the weekly AFL, but the buy price is the previous DAY's high (or the open + 0.25% above it)",
            "X|Same signal; the sell price is the close on an ST turn down and otherwise the last completed week's low (no gap handling)",
        ], "INSIDE BAR plot weekly.afl, coded as the AFL is written. Its buy price refers to the previous daily high, not the weekly high, so a fill can be below the weekly trigger; this looks like a bug in the AFL and is kept."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let stop = p("hard_stop_pct", "Hard stop loss % (ApplyStop, 0 off)", 0.0, 0.0, 50.0, 1.0);
    let src = p("st_src", "ST calculation point (0 mid point, 1 close, 2 pivot)", 0.0, 0.0, 2.0, 1.0);
    let flt = |none: bool| {
        vec![
            if none { p("mkt_filter", "Market filter (0 none, 1 ST, 2 SMA)", 0.0, 0.0, 2.0, 1.0) } else { p("mkt_filter", "Market filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0) },
            p("flt_tf", "Filter timeframe (0 monthly, 1 weekly, 2 daily)", 0.0, 0.0, 2.0, 1.0),
            p("flt_st_period", "ST filter period", 5.0, 1.0, 20.0, 1.0),
            p("flt_st_factor", "ST filter factor", 1.0, 0.0, 20.0, 1.0),
            p("flt_sma_period", "SMA filter period", 200.0, 1.0, 300.0, 1.0),
        ]
    };
    let sl_st = || vec![p("sl_st_period", "ST period for the stop", 5.0, 1.0, 20.0, 1.0), p("sl_st_factor", "ST factor for the stop", 1.0, 1.0, 20.0, 1.0)];
    Some(match id {
        INTRADAY => vec![stop],
        ATHC => vec![
            stop,
            p("st_src", "ST calculation point (0 close, 1 mid point, 2 high, 3 pivot, 4 mid/close/pivot average, 5 3-day MA, 6 5-day MA)", 0.0, 0.0, 6.0, 1.0),
            p("st_period", "ST period", 10.0, 1.0, 20.0, 1.0),
            p("st_factor", "ST factor (lower band)", 4.0, 1.0, 20.0, 1.0),
        ],
        INTRADAY_FLT => {
            let mut v = vec![stop, src];
            v.extend(flt(false));
            v
        }
        PREV_LOW | ST_SL => {
            let mut v = vec![stop, src];
            v.extend(flt(true));
            v.extend(sl_st());
            v
        }
        WEEKLY | WEEKLY_PLOT => {
            let mut v = vec![stop, src];
            v.extend(sl_st());
            v
        }
        _ => return None,
    })
}

fn daily_bars(x: &Ctx) -> Bars {
    let n = x.c.len();
    Bars { h: x.h.clone(), l: x.l.clone(), c: x.c.clone(), last: (0..n).collect(), group: (0..n).collect() }
}

/// The previous bar's market filter (`Ref(FLTBuy, -1)`); None when the setting is held.
fn filter(x: &Ctx, kind: i64, g: &dyn Fn(&str) -> f64) -> Option<Vec<bool>> {
    let n = x.c.len();
    let tf = match g("flt_tf").round() as i64 { 1 => 'W', 2 => 'D', _ => 'M' };
    let up: Vec<bool> = match kind {
        0 => vec![true; n],
        1 => {
            let m = st_filter(x, tf, g("st_src").round() as i64 + 1, g("flt_st_period").round().max(1.0) as usize, g("flt_st_factor"));
            m.iter().map(|&t| t == 1).collect()
        }
        2 if tf == 'D' => {
            // smbuysig loop: 1 on a close above the previous average (Null = -1e10 in the loop), 0 below, else carry
            let fm = ma(&x.c, g("flt_sma_period").round().max(1.0) as usize);
            let mut s = vec![false; n];
            for i in 1..n {
                s[i] = if x.c[i] > lv(fm[i - 1]) { true } else if x.c[i] < lv(fm[i - 1]) { false } else { s[i - 1] };
            }
            s
        }
        _ => return None,
    };
    Some((0..n).map(|i| i > 0 && up[i - 1]).collect())
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let upc: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i])).collect();
    let loc: Vec<bool> = (0..n).map(|i| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i])).collect();
    let guard = |i: usize| i != 0 && !upc[i] && c[i] > 0.0;
    let ib: Vec<bool> = (0..n).map(|i| i > 0 && h[i] < h[i - 1] && l[i] > l[i - 1]).collect();
    let trig: Vec<bool> = (0..n).map(|i| i > 0 && ib[i - 1] && h[i] > h[i - 1]).collect();
    let stop_px = |i: usize| if o[i] > h[i - 1] { o[i] * 1.0025 } else { h[i - 1] };
    let calc = calc_point(&daily_bars(x), gi("st_src") + 1);
    let stop = g("hard_stop_pct");
    let mut buy: Vec<Option<f64>> = vec![None; n];
    let mut sell: Vec<Exit> = vec![None; n];
    let same_day = |buy: &mut Vec<Option<f64>>, sell: &mut Vec<Exit>, ok: &dyn Fn(usize) -> bool| {
        for i in 1..n {
            if trig[i] && ok(i) {
                buy[i] = Some(stop_px(i));
            }
            if ib[i - 1] {
                sell[i] = Some((c[i], "Same-bar close"));
            }
        }
    };
    match id {
        INTRADAY => same_day(&mut buy, &mut sell, &|i| guard(i)),
        ATHC => {
            let calc: Vec<f64> = match gi("st_src") {
                1 => (0..n).map(|i| (h[i] + l[i]) / 2.0).collect(),
                2 => h.clone(),
                3 => (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect(),
                4 => (0..n).map(|i| ((h[i] + l[i]) / 2.0 + (h[i] + l[i] + c[i]) / 3.0 + c[i]) / 3.0).collect(),
                5 => ma(c, 3).into_iter().map(|v| v.unwrap_or(f64::NAN)).collect(),
                6 => ma(c, 5).into_iter().map(|v| v.unwrap_or(f64::NAN)).collect(),
                _ => c.clone(),
            };
            let (bt, _) = supertrend(&calc, h, l, c, gu("st_period"), 1.0, g("st_factor"));
            let mut hcl = c.clone();
            for i in 1..n {
                hcl[i] = hcl[i].max(hcl[i - 1]);
            }
            let mut athc = vec![false; n];
            for i in 1..n {
                athc[i] = if c[i] > hcl[i - 1] { true } else if bt[i] == -1 && athc[i - 1] { false } else { athc[i - 1] };
            }
            same_day(&mut buy, &mut sell, &|i| athc[i - 1]);
        }
        INTRADAY_FLT => {
            let f = filter(x, gi("mkt_filter") + 1, &g)?;
            same_day(&mut buy, &mut sell, &|i| guard(i) && f[i]);
        }
        PREV_LOW | ST_SL => {
            let f = filter(x, gi("mkt_filter"), &g)?;
            for i in 1..n {
                if trig[i] && guard(i) && f[i] {
                    buy[i] = Some(stop_px(i));
                }
            }
            if id == PREV_LOW {
                for i in 1..n {
                    if l[i] < l[i - 1] {
                        sell[i] = Some((if o[i] < l[i - 1] { o[i] * 0.9975 } else { l[i - 1] }, "Previous low"));
                    }
                }
            } else {
                let (bt, _) = supertrend(&calc, h, l, c, gu("sl_st_period"), g("sl_st_factor"), g("sl_st_factor"));
                let mut buysl: Option<f64> = None;
                for i in 0..n {
                    if ib[i] {
                        buysl = Some(l[i]);
                    }
                    let was_up = i > 0 && bt[i - 1] == 1;
                    let hit = if was_up { bt[i] == -1 } else { buysl.map_or(false, |s| l[i] < s) };
                    if hit && !loc[i] {
                        let px = if was_up { (c[i], "ST turned down") } else { let s = buysl.unwrap(); (if o[i] < s { o[i] * 0.9975 } else { s }, "Inside-bar low") };
                        sell[i] = Some(px);
                    }
                }
            }
        }
        WEEKLY | WEEKLY_PLOT => {
            let w = compress(x, 'W');
            let ibw: Vec<bool> = (0..w.c.len()).map(|k| k > 0 && w.h[k] < w.h[k - 1] && w.l[k] > w.l[k - 1]).collect();
            let (wib, hi1, lo1) = (expand_last(&ibw, &w), expand_last(&w.h, &w), expand_last(&w.l, &w));
            let (bt, _) = supertrend(&calc, h, l, c, gu("sl_st_period"), g("sl_st_factor"), g("sl_st_factor"));
            let b0: Vec<bool> = (0..n).map(|i| wib[i] && h[i] > hi1[i] && guard(i)).collect();
            let s0: Vec<bool> = (0..n).map(|i| (if i > 0 && bt[i - 1] == 1 { bt[i] == -1 } else { l[i] < lo1[i] }) && !loc[i]).collect();
            let b1 = crate::books::ytsw::exrem(&b0, &s0);
            let s1 = crate::books::ytsw::exrem(&s0, &b1);
            for i in 1..n {
                if b1[i] {
                    let lvl = if id == WEEKLY { hi1[i - 1] } else { h[i - 1] };
                    buy[i] = Some(if o[i] > lvl { o[i] * 1.0025 } else { lvl });
                }
            }
            for i in 0..n {
                if s1[i] {
                    let was = |t: i8| i > 0 && bt[i - 1] == t;
                    sell[i] = Some(if id == WEEKLY {
                        if l[i] < lo1[i] && was(-1) { (if o[i] < lo1[i] { o[i] * 0.9975 } else { lo1[i] }, "Weekly low") } else { (c[i], "ST turned down") }
                    } else if was(1) && bt[i] == -1 {
                        (c[i], "ST turned down")
                    } else {
                        (lo1[i], "Weekly low")
                    });
                }
            }
        }
        _ => return None,
    }
    Some(ib_fills(x, &buy, &sell, from, to, stop))
}

/// The backtest of these AFLs (long only, `AllowSameBarExit`): fills kept inside the bar's range; the exit is tested
/// before the entry on each bar; an entry from flat may exit on its own bar, but not when that bar's exit signal just
/// closed the previous trade; the optional ApplyStop is not active on the entry bar.
fn ib_fills(x: &Ctx, entry: &[Option<f64>], exit: &[Exit], from: usize, to: usize, stop_pct: f64) -> Vec<Trade> {
    let bound = |i: usize, px: f64| px.clamp(x.l[i], x.h[i]);
    let mut out = Vec::new();
    let mut pos: Option<(usize, f64)> = None;
    for i in from..=to.min(x.c.len().saturating_sub(1)) {
        let mut exited = false;
        if let Some((e, epx)) = pos {
            let lvl = epx * (1.0 - stop_pct / 100.0);
            if stop_pct > 0.0 && i > e && x.l[i] <= lvl {
                out.push(mk(1.0, e, epx, "limit", i, bound(i, if x.o[i] < lvl { x.o[i] } else { lvl }), "Hard stop (ApplyStop)", false));
                pos = None;
                exited = true;
            } else if let Some((px, why)) = exit[i] {
                out.push(mk(1.0, e, epx, "limit", i, bound(i, px), why, false));
                pos = None;
                exited = true;
            }
        }
        if pos.is_none() {
            if let Some(px) = entry[i] {
                let px = bound(i, px);
                pos = Some((i, px));
                if !exited {
                    if let Some((spx, why)) = exit[i] {
                        out.push(mk(1.0, i, px, "limit", i, bound(i, spx), why, false));
                        pos = None;
                    }
                }
            }
        }
    }
    if let Some((e, epx)) = pos {
        out.push(mk(1.0, e, epx, "limit", to, x.c[to], "open", true));
    }
    out
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Inside Bar"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytd7::tests::{bars, ps, wave};

    fn go(id: &str, ohlc: &[(f64, f64, f64, f64)], kv: &[(&str, f64)]) -> Vec<(usize, f64, usize, f64, bool)> {
        let r = bars(ohlc);
        let x = Ctx::new(&r, &r, &[]);
        run(id, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.open)).collect()
    }
    const W: (f64, f64, f64, f64) = (100.0, 110.0, 90.0, 101.0);
    const I: (f64, f64, f64, f64) = (100.0, 105.0, 95.0, 101.0); // inside W

    #[test]
    fn same_day_breakout() {
        // bar 2 breaks the inside bar's high 105: buy at 105, sell at its close 107
        let v = [W, I, (102.0, 108.0, 101.0, 107.0), (107.0, 108.0, 106.0, 107.5)];
        assert_eq!(go(INTRADAY, &v, &[]), vec![(2, 105.0, 2, 107.0, false)]);
        // gap above the inside high: open + 0.25%
        let v2 = [W, I, (106.0, 108.0, 105.5, 107.0)];
        assert!((go(INTRADAY, &v2, &[])[0].1 - 106.0 * 1.0025).abs() < 1e-9);
        // the filter AFL with a daily SMA filter (the average is Null early, so the loop's filter is on)
        assert_eq!(go(INTRADAY_FLT, &v, &[("mkt_filter", 1.0), ("flt_tf", 2.0)]).len(), 1);
        // SMA on a monthly timeframe is held
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run(INTRADAY_FLT, &x, 0, 3, &ps(&[("mkt_filter", 1.0)])).is_none());
    }

    #[test]
    fn previous_low_stop_and_same_bar_exit() {
        // bar 2 breaks the inside high 105 and its low 94 breaks the inside low 95: in and out at 95
        let v = [W, I, (100.0, 106.0, 94.0, 99.0), (99.0, 100.0, 98.0, 99.5)];
        assert_eq!(go(PREV_LOW, &v, &[]), vec![(2, 105.0, 2, 95.0, false)]);
        // otherwise held until a low below the previous low
        let v2 = [W, I, (100.0, 106.0, 96.0, 104.0), (104.0, 106.0, 97.0, 105.0), (104.0, 105.0, 96.0, 97.0)];
        assert_eq!(go(PREV_LOW, &v2, &[]), vec![(2, 105.0, 4, 97.0, false)]);
        // a 5% ApplyStop from the next bar: 99.75; bar 3's low 97 hits it first
        assert_eq!(go(PREV_LOW, &v2, &[("hard_stop_pct", 5.0)]), vec![(2, 105.0, 3, 105.0 * 0.95, false)]);
    }

    #[test]
    fn fills_stay_inside_the_bar_and_a_sell_signal_is_used_once() {
        // gap above the inside high: open 108 + 0.25% = 108.27 is above the bar's high 108.1 -> filled at 108.1
        let v = [W, I, (108.0, 108.1, 104.0, 106.0)];
        assert_eq!(go(INTRADAY, &v, &[]), vec![(2, 108.1, 2, 106.0, false)]);
        // previous-low version: bar 4's low breaks bar 3's low (sells the first trade) and bar 4 also breaks the
        // inside high of bar 3 (new entry): the new trade is NOT closed by that same sell signal; it is held
        let v2 = [W, I, (100.0, 106.0, 96.0, 104.0), (103.0, 104.0, 100.0, 101.0), (101.0, 105.0, 99.0, 104.5), (104.5, 106.0, 104.0, 105.5)];
        let t = go(PREV_LOW, &v2, &[]);
        assert_eq!(t[0].2, 4);
        assert!(t.len() == 2 && t[1].0 == 4 && t[1].2 != 4, "{t:?}");
    }

    #[test]
    fn weekly_inside_week_breakout() {
        // week 1 (Mon-Fri) range 90-110, week 2 range 95-105 (inside); Monday of week 3 breaks 105
        let mut v = vec![W; 5];
        v.extend(vec![I; 5]);
        v.push((104.0, 107.0, 103.0, 106.0));
        v.push((106.0, 107.0, 105.5, 106.5));
        let t = go(WEEKLY, &v, &[]);
        assert_eq!((t[0].0, t[0].1), (10, 105.0));
        // the plot AFL buys at the previous day's high (105 as well here) and the same exit
        let tp = go(WEEKLY_PLOT, &v, &[]);
        assert_eq!(tp[0].0, 10);
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            for f in [0.0, 1.0] {
                let t = run(id, &x, 20, r.len() - 1, &ps(&[("mkt_filter", f), ("flt_tf", 2.0), ("hard_stop_pct", 2.0)])).unwrap();
                assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= 20), "{id}");
            }
            assert!(family_of(id).is_some());
        }
    }
}
