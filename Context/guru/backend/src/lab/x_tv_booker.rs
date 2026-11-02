//! TradingView built-in indicators the lab did not have (2026-10-30 gap check against TradingView's built-in list): the six
//! Rob Booker built-ins. All six ids already exist as listed (not computed) entries in listed.rs, with rows in sources.rs.
//! TradingView publishes the Pine source of two of them (pine-facade, open_no_auth, retrieved 2026-10-30):
//! "Rob Booker — ADX breakout" (STD;Rob%1Booker%1ADX%1Breakout%1Strategy v24.0) and "Rob Booker — Ziv ghost pivots"
//! (STD;Rob%1Booker%1Ghost%1Pivots%1v2 v30.0). Rob Booker Reversal is closed, but its help page states the rule, OANDA Japan
//! lists its inputs and defaults, and Rob Booker's own open-source RB_ReversalTabs (same developer) fixes the MACD and %K maths.
//! These three are computed here in READY. They are not in DEFS, because the same ids are still listed in listed.rs.
//! Swapping them in means removing those three listed.rs rows, and moving READY into DEFS (see the READY doc comment).
//! Knoxville Divergence, Intraday Pivot Points and Missed Pivot Points stay listed: the help pages give no formula, the
//! built-ins are closed source, and only their names are in the chart bundle (no source).
#![allow(dead_code)]
use super::bars::Bars;
use super::catalogue::{int, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::resample::period_key;
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str],
             placement: Placement, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs: Needs::NONE, definition, first, sparse: false, compute: Some(f), note: "" }
}
fn flag(b: bool) -> f64 { if b { 1. } else { 0. } }

/// The published dirmov/adx of the built-in: DM is 0 on bar 0 (Pine na comparisons are false), TR is na on bar 0,
/// DI = fixnan(100·rma(DM)/rma(TR)), ADX = 100·rma(|+DI − −DI| / (sum == 0 ? 1 : sum), adx_len). First value at bar di_len + adx_len − 1.
fn pine_adx(b: &Bars, dl: usize, al: usize) -> Vec<f64> {
    let m = b.len();
    let dm = |plus: bool| -> Vec<f64> { (0..m).map(|i| if i == 0 { 0. } else {
        let (up, dn) = (b.high[i] - b.high[i - 1], b.low[i - 1] - b.low[i]);
        let (a, o) = if plus { (up, dn) } else { (dn, up) };
        if a > o && a > 0. { a } else { 0. }
    }).collect() };
    let trs = ta::rma(&ta::tr(b, false), dl);
    let di = |d: Vec<f64>| -> Vec<f64> {
        let r = ta::rma(&d, dl);
        let mut prev = NA; // fixnan: a 0/0 or x/0 bar keeps the previous DI
        (0..m).map(|i| { let v = 100. * r[i] / trs[i]; if v.is_finite() { prev = v; } prev }).collect()
    };
    let (pl, mi) = (di(dm(true)), di(dm(false)));
    let x: Vec<f64> = (0..m).map(|i| { let s = pl[i] + mi[i]; (pl[i] - mi[i]).abs() / if s == 0. { 1. } else { s } }).collect();
    ta::rma(&x, al).iter().map(|v| 100. * v).collect()
}

const ADX: &[ParamSpec] = &[int("adx_smoothing", "ADX Smoothing Period (the DI length in the published code)", 14., 1., 500.),
    int("adx_period", "ADX Period (the ADX smoothing in the published code)", 14., 1., 500.), real("adx_lower", "ADX Lower Level", 18., 0., 100., 1.),
    int("box_lookback", "BreakoutBox Lookback Period", 20., 1., 500.)];
fn adx_breakout(b: &Bars, prm: &Params) -> Output {
    let (dl, al, lvl, lb, m) = (n(prm, "adx_smoothing"), n(prm, "adx_period"), p(prm, "adx_lower"), n(prm, "box_lookback"), b.len());
    let c = &b.close;
    let adx = pine_adx(b, dl, al);
    let shift = |v: Vec<f64>| -> Vec<f64> { (0..m).map(|i| if i == 0 { NA } else { v[i - 1] }).collect() };
    let (up, lo) = (shift(ta::highest(&b.high, lb)), shift(ta::lowest(&b.low, lb)));
    let width: Vec<f64> = (0..m).map(|i| up[i] - lo[i]).collect();
    let low: Vec<f64> = adx.iter().map(|a| if a.is_nan() { NA } else { flag(*a < lvl) }).collect();
    // ta.cross = crossover or crossunder; NaN comparisons are false, as in Pine
    let cross = |x: &[f64], t: usize| (c[t] > x[t] && c[t - 1] <= x[t - 1]) || (c[t] < x[t] && c[t - 1] >= x[t - 1]);
    let warm = (dl + al - 1).max(lb);
    let (mut buy, mut sell) = (vec![NA; m], vec![NA; m]);
    for t in warm.max(1)..m {
        let quiet = adx[t] < lvl;
        buy[t] = flag(quiet && cross(&up, t));
        sell[t] = flag(quiet && cross(&lo, t));
    }
    Ok(vec![adx, low, up, lo, width, buy, sell])
}

const REV: &[ParamSpec] = &[int("fast", "Fast MA Period", 12., 1., 500.), int("slow", "Slow MA Period", 26., 1., 500.),
    int("k_period", "KPeriod", 70., 1., 500.), int("slowing", "Slowing", 10., 1., 500.),
    real("upper", "Stochastic Upper", 70., 0., 100., 1.), real("lower", "Stochastic Lower", 30., 0., 100., 1.)];
fn reversal(b: &Bars, prm: &Params) -> Output {
    let (f, s, kp, sl, m) = (n(prm, "fast"), n(prm, "slow"), n(prm, "k_period"), n(prm, "slowing"), b.len());
    let (ub, lb, c) = (p(prm, "upper"), p(prm, "lower"), &b.close);
    let (ef, es) = (ta::ema(c, f), ta::ema(c, s));
    let macd: Vec<f64> = (0..m).map(|i| ef[i] - es[i]).collect();
    let (hh, ll) = (ta::highest(&b.high, kp), ta::lowest(&b.low, kp));
    let fast_k: Vec<f64> = (0..m).map(|i| { let r = hh[i] - ll[i]; if r == 0. { NA } else { 100. * (c[i] - ll[i]) / r } }).collect();
    let k = ta::sma(&fast_k, sl);
    let warm = f.max(s).max(kp + sl - 2);
    let (mut buy, mut sell) = (vec![NA; m], vec![NA; m]);
    for t in warm.max(1)..m {
        buy[t] = flag(macd[t] > 0. && macd[t - 1] <= 0. && k[t] < lb);
        sell[t] = flag(macd[t] < 0. && macd[t - 1] >= 0. && k[t] > ub);
    }
    Ok(vec![buy, sell, macd, k])
}

/// (high so far, low so far, close)/3 of the bar's developing week (Monday–Sunday) and calendar month.
fn ghost_pivots(b: &Bars, _: &Params) -> Output {
    let m = b.len();
    let mut out = vec![b.hlc3(), vec![NA; m], vec![NA; m]];
    for (col, per) in [(1, 1), (2, 2)] {
        let (mut key, mut h, mut l) = (None, NA, NA);
        for i in 0..m {
            let k = period_key(&b.date[i], per);
            if key != Some(k) { key = Some(k); h = b.high[i]; l = b.low[i]; } else { h = h.max(b.high[i]); l = l.min(b.low[i]); }
            out[col][i] = (h + l + b.close[i]) / 3.;
        }
    }
    Ok(out)
}

/// Computed Booker built-ins, ready to replace the listed.rs rows of the same ids. To activate: delete
/// rob_booker_adx_breakout, rob_booker_reversal and rob_booker_ziv_ghost from listed.rs, make DEFS = READY, and
/// lower the catalogue_matches_the_phase6_master_list base counts by 3. Their sources.rs rows stay, so SOURCES gets no new rows here.
pub const READY: &[IndicatorDef] = &[
    def("rob_booker_adx_breakout", "Rob Booker - ADX Breakout", "Trend", ADX, &["adx", "consolidation", "box_upper", "box_lower", "box_width", "buy", "sell"], Placement::Price,
        "TradingView built-in Rob Booker — ADX breakout, published Pine source (strategy, developer Andrew Palladino, 9/29/2017). dirmov(len): up = high - high[1], down = low[1] - low; +DM = up > down and up > 0 ? up : 0, -DM likewise (both 0 on bar 0, where Pine's na comparisons are false); truerange = rma(tr, len) with tr na on bar 0; +DI = fixnan(100*rma(+DM, len)/truerange), -DI likewise (a zero range keeps the previous DI). adx = 100*rma(|+DI - -DI| / (sum == 0 ? 1 : sum), adx_len). rma is Wilder's average (alpha 1/n) seeded with the SMA of the first n values. The published call is adx(adxSmoothPeriod, adxPeriod), so adx_smoothing (14) is the DI length and adx_period (14) smooths the ADX. The first ADX value is on bar adx_smoothing + adx_period - 1. consolidation = 1 when adx < adx_lower (18), else 0 (TradingView's purple background). box_upper = highest(high, box_lookback)[1] and box_lower = lowest(low, box_lookback)[1]: the extremes of the box_lookback bars before this one (lookback 20). box_width = box_upper - box_lower. buy = 1 when ta.cross(close, box_upper) and adx < adx_lower. sell = 1 when ta.cross(close, box_lower) and adx < adx_lower. Else 0, and blank before bar max(adx_smoothing + adx_period - 1, box_lookback). ta.cross is either direction: close > level now and close[1] <= level[1], or close < level now and close[1] >= level[1]. As published, a close back under a box that the previous bar's close broke out of also counts as a buy cross (and the mirror for sell). Trade management in the published strategy: the order fills on the next bar's open, stop = entry -/+ 0.5*box_width and target = entry +/- 1.0*box_width (box_width of the signal bar), one trade at a time, direction both. Adaptation: this is the strategy's signal layer, with the box always recomputed as if flat. The strategy freezes the box and suppresses signals while a position is open, and that state belongs to the lab's backtester. The first bar after an exit can differ, because there ta.cross compares against the frozen box of the previous bar. Stop and target are not outputs: they need the fill price. Sources: TradingView built-in source, pine-facade STD;Rob%1Booker%1ADX%1Breakout%1Strategy v24.0 (retrieved 2026-10-30); TradingView help, Rob Booker - ADX Breakout (https://www.tradingview.com/support/solutions/43000591333-rob-booker-adx-breakout/), which gives no formula. Cross-check: the open-source v5 port by Powerscooter has the same logic (https://www.tradingview.com/script/qoaam9SI-Rob-Booker-ADX-Breakout-updated-to-pinescript-V5/).",
        &[27, 27, 20, 20, 20, 27, 27], adx_breakout),
    def("rob_booker_reversal", "Rob Booker Reversal", "Momentum", REV, &["buy", "sell", "macd", "k"], Placement::Pane,
        "Rule, from the TradingView help page: a red triangle (sell) when the MACD line has crossed 0 from above and stochastic %K is greater than the overbought level; a green triangle (buy) when the MACD line has crossed 0 from below and %K is lower than the oversold level. Inputs and defaults, from OANDA Japan's TradingView guide: Fast MA Period 12, Slow MA Period 26, KPeriod 70, Slowing 10, Stochastic Upper 70, Stochastic Lower 30. macd = ema(close, fast) - ema(close, slow); the EMA is seeded with the SMA of its first n values (Pine ta.ema). fast %K = 100*(close - lowest(low, k_period))/(highest(high, k_period) - lowest(low, k_period)), na on a zero range. k = sma(fast %K, slowing). buy = 1 when macd > 0 and macd[1] <= 0 (ta.crossover) and k < lower. sell = 1 when macd < 0 and macd[1] >= 0 (ta.crossunder) and k > upper. Else 0; a na k gives 0. Signals are blank before bar max(fast, slow, k_period + slowing - 2), which is 78 for the defaults. Causal: bar t uses bars <= t. Adaptation: the built-in is closed source. The MACD line as an EMA difference and Slowing as an SMA of the raw %K are taken from Rob Booker's own open-source RB_ReversalTabs (developer Andrew Palladino, 11/25/2018), whose signal lines are the same with %K Period in place of Slowing. They are not confirmed against TradingView output. Sources: TradingView help, Rob Booker Reversal (https://www.tradingview.com/support/solutions/43000599840-rob-booker-reversal/); OANDA Japan, how to set Rob Booker Reversal on TradingView (https://www.oanda.jp/lab-education/tradingviewstrategy/basic/tradingview_rob_booker_reversal_setting); RB_ReversalTabs by robbooker (https://www.tradingview.com/script/sCFViUbr-Rob-Booker-Reversal-Tabs/, PUB;8JP4qLYYdKtdlCZ1fC9wCyb3ZHVOsXog).",
        &[78, 78, 25, 78], reversal),
    def("rob_booker_ziv_ghost", "Rob Booker Ziv Ghost Pivots", "Periods & calendar", &[], &["day", "week", "month"], Placement::Price,
        "TradingView built-in Rob Booker — Ziv ghost pivots, published Pine source (creator Rob Booker). Each mark is request.security(tickerid, TF, hlc3[0], gaps_off, lookahead_on) for TF = 240, D, W and M. That is the pivot (H+L+C)/3 of the current, still-forming higher-timeframe bar: where that period's pivot would sit if the period ended now (help page: 'where the target would exist if today were the first day of next week'). On daily bars: day = (high + low + close)/3 of the bar. week = (highest high so far this week + lowest low so far this week + today's close)/3, weeks Monday to Sunday as TradingView's W bars. month is the same over the calendar month. Adaptation (causality): with lookahead_on, TradingView's historical bars show the period's FINAL hlc3 on every bar of the period, which repaints. Here each bar shows the value as it stood at that bar's close, which is what a live chart displayed. The 240-minute mark ('8') needs intraday bars and is not reproduced; the published code also drives it from the next-day switch. Sources: TradingView built-in source, pine-facade STD;Rob%1Booker%1Ghost%1Pivots%1v2 v30.0 (retrieved 2026-10-30); TradingView help, Rob Booker Ziv Ghost Pivots (https://www.tradingview.com/support/solutions/43000594679-rob-booker-ziv-ghost-pivots/).",
        &[0, 0, 0], ghost_pivots),
];
pub const DEFS: &[IndicatorDef] = READY;
/// (id, source, licence/notes), one per entry.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("rob_booker_adx_breakout", "Rob Booker - ADX Breakout", ""),
    ("rob_booker_reversal", "Rob Booker Reversal", ""),
    ("rob_booker_ziv_ghost", "Rob Booker Ziv Ghost Pivots", ""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::{get, run};
    use crate::lab::testdata::{assert_series, close_eq, cut, d1, from_ohlc, walk};
    const N: f64 = f64::NAN;
    fn ready(id: &str) -> IndicatorDef { *READY.iter().find(|d| d.id == id).unwrap() }
    fn go(id: &str, b: &Bars, kv: &[(&str, f64)]) -> Vec<Vec<f64>> {
        run(&ready(id), b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()).unwrap_or_else(|e| panic!("{id}: {e}"))
    }

    #[test] fn ready_ids_are_the_listed_ones_waiting_for_the_swap() {
        for d in READY {
            let listed = get(d.id).unwrap_or_else(|| panic!("{}: no listed entry", d.id));
            assert!(listed.compute.is_none() || DEFS.iter().any(|x| x.id == d.id), "{}", d.id);
            assert!(!crate::lab::sources::names(d.id).0.is_empty(), "{}: sources row", d.id);
            assert_eq!(d.first.len(), d.outputs.len());
        }
        for id in ["rob_booker_knoxville", "rob_booker_missed_pivots", "rob_booker_intraday_pivots"] { assert!(get(id).unwrap().compute.is_none(), "{id}"); }
    }
    #[test] fn ghost_pivots_are_the_developing_period_hlc3() {
        // d1: 2024-01-01 (Monday) .. 01-08; bar 7 opens week 2
        let o = go("rob_booker_ziv_ghost", &d1(), &[]);
        assert_series(&o[0], &[(11. + 9. + 10.5) / 3., (12. + 10. + 11.5) / 3., (12.5 + 11. + 12.) / 3., (12.2 + 10.8 + 11.) / 3., (11.5 + 10. + 10.2) / 3., (11. + 9.5 + 10.8) / 3., (12. + 10.5 + 11.9) / 3., (13. + 11.5 + 12.6) / 3.]);
        let wk = [(11. + 9. + 10.5) / 3., (12. + 9. + 11.5) / 3., (12.5 + 9. + 12.) / 3., (12.5 + 9. + 11.) / 3., (12.5 + 9. + 10.2) / 3., (12.5 + 9. + 10.8) / 3., (12.5 + 9. + 11.9) / 3.];
        let mut w = wk.to_vec(); w.push((13. + 11.5 + 12.6) / 3.);
        assert_series(&o[1], &w);
        let mut mo = wk.to_vec(); mo.push((13. + 9. + 12.6) / 3.);
        assert_series(&o[2], &mo);
    }
    // box_lookback 2, DI 1, ADX 1, lower 1000 (above the input maximum, so called directly): adx is defined from bar 1 and
    // always below it, so the box crosses alone decide
    #[test] fn adx_breakout_box_and_both_way_cross() {
        let h = [10., 11., 10.5, 12.5, 12., 11., 9.6];
        let l = [9., 9.5, 9.8, 10.5, 10.8, 9.2, 8.];
        let c = [9.5, 10.5, 10., 12., 11., 9.4, 8.5];
        let b = from_ohlc(&(0..7).map(|i| (c[i], h[i], l[i], c[i])).collect::<Vec<_>>());
        let q = |lvl: f64| -> Params { [("box_lookback", 2.), ("adx_smoothing", 1.), ("adx_period", 1.), ("adx_lower", lvl)].iter().map(|(k, v)| (k.to_string(), *v)).collect() };
        let o = adx_breakout(&b, &q(1000.)).unwrap();
        assert_series(&o[2], &[N, N, 11., 11., 12.5, 12.5, 12.]);
        assert_series(&o[3], &[N, N, 9., 9.5, 9.8, 10.5, 9.2]);
        // bar 3 closes 12 > 11 (bar 2: 10 <= 11) → buy; bar 4 closes 11 < 12.5 after 12 >= 11 → a second (published) buy cross
        assert_series(&o[5], &[N, N, 0., 1., 1., 0., 0.]);
        // bar 5 closes 9.4 < 10.5 (bar 4: 11 >= 9.8) → sell; bar 6 closes 8.5 < 9.2 but 9.4 < 10.5 before → no cross
        assert_series(&o[6], &[N, N, 0., 0., 0., 1., 0.]);
        // a high level switches every signal off but keeps the box
        let q = adx_breakout(&b, &q(0.)).unwrap();
        assert_series(&q[5], &[N, N, 0., 0., 0., 0., 0.]);
        assert_series(&q[1], &[N, 0., 0., 0., 0., 0., 0.]);
    }
    #[test] fn pine_adx_hand_values() {
        // DI 1, ADX 1: rma(·,1) is the value itself; bar 1: up 1, down -0.5 → +DM 1, TR max(1.5, 1, .5) = 1.5 → +DI 66.67, -DI 0 → ADX 100
        let b = from_ohlc(&[(9.5, 10., 9., 9.5), (10.5, 11., 9.5, 10.5), (10., 10.5, 8., 9.)]);
        let a = pine_adx(&b, 1, 1);
        assert_series(&a, &[N, 100., 100.]);
        // DI 2: +DM [0, 1, 0], rma2 seeds at bar 1 = 0.5, bar 2 = 0.25; -DM [0, 0, 1.5] → 0, 0.75; TR [na, 1.5, 2.5] → seeds bar 2 = 2
        // bar 2: +DI 12.5, -DI 37.5 → x = 0.5 → ADX(1) = 50
        assert_series(&pine_adx(&b, 2, 1), &[N, N, 50.]);
    }
    #[test] fn reversal_hand_values() {
        // fast 1 (ema = close), slow 2, k_period 2, slowing 1; macd = c - ema2(c)
        let c = [10., 8., 7., 9., 12., 11.];
        let h = [10.5, 9., 7.5, 9.5, 12.5, 12.];
        let l = [9.5, 7.5, 6.5, 8.5, 11., 10.];
        let b = from_ohlc(&(0..6).map(|i| (c[i], h[i], l[i], c[i])).collect::<Vec<_>>());
        let kv = [("fast", 1.), ("slow", 2.), ("k_period", 2.), ("slowing", 1.), ("upper", 70.), ("lower", 60.)];
        let o = go("rob_booker_reversal", &b, &kv);
        // ema2: seed bar 1 = 9, bar 2 = 9 + 2/3(7-9) = 7.667, bar 3 = 8.556, bar 4 = 10.852, bar 5 = 10.951
        let e = [N, 9., 9. + (7. - 9.) * 2. / 3.];
        let e3 = e[2] + (9. - e[2]) * 2. / 3.; let e4 = e3 + (12. - e3) * 2. / 3.; let e5 = e4 + (11. - e4) * 2. / 3.;
        assert_series(&o[2], &[N, -1., 7. - e[2], 9. - e3, 12. - e4, 11. - e5]);
        // %K(2): bar 1 (8-7.5)/(10.5-7.5) = 16.67, bar 2 (7-6.5)/(9-6.5) = 20, bar 3 (9-6.5)/(9.5-6.5) = 83.33, bar 4 (12-8.5)/4 = 87.5, bar 5 (11-10)/2.5 = 40
        assert_series(&o[3], &[N, 100. / 6., 20., 250. / 3., 87.5, 40.]);
        // macd crosses 0 up on bar 3 (k 83.3 not < 60 → no buy), stays up; no sell (bar 5 macd 0.049 > 0)
        assert_series(&o[0], &[N, N, 0., 0., 0., 0.]);
        let o = go("rob_booker_reversal", &b, &[kv[0], kv[1], kv[2], kv[3], ("upper", 70.), ("lower", 90.)]);
        assert_series(&o[0], &[N, N, 0., 1., 0., 0.]);
        // the mirror: negate prices → macd crosses down on bar 3 with %K 16.7 → sell when upper < 16.7
        let neg = |x: &[f64]| x.iter().map(|v| 200. - v).collect::<Vec<_>>();
        let (nc, nh, nl) = (neg(&c), neg(&l), neg(&h));
        let nb = from_ohlc(&(0..6).map(|i| (nc[i], nh[i], nl[i], nc[i])).collect::<Vec<_>>());
        let o = go("rob_booker_reversal", &nb, &[kv[0], kv[1], kv[2], kv[3], ("upper", 10.), ("lower", 0.)]);
        assert_series(&o[1], &[N, N, 0., 1., 0., 0.]);
    }
    // the catalogue-wide checks, run here on READY until it is registered
    #[test] fn warmup_matches_first_and_no_gaps() {
        let w = walk(1000, 7, true);
        for d in READY {
            for (k, s) in run(d, &w, &Params::new()).unwrap().iter().enumerate() {
                let f = s.iter().position(|v| !v.is_nan()).unwrap_or(usize::MAX);
                assert_eq!(f, d.first[k], "{}.{}", d.id, d.outputs[k]);
                assert!(s[f..].iter().all(|v| v.is_finite()), "{}.{} gap", d.id, d.outputs[k]);
            }
        }
        assert!(go("rob_booker_adx_breakout", &walk(3000, 2, false), &[])[5].iter().any(|v| *v == 1.), "an ADX breakout occurs");
        let r = go("rob_booker_reversal", &walk(3000, 2, false), &[]);
        assert!(r[0].iter().chain(&r[1]).any(|v| *v == 1.), "a reversal occurs");
    }
    #[test] fn every_output_is_causal_at_many_cut_points() {
        let w = walk(500, 11, false);
        let small = [("box_lookback", 3.), ("adx_smoothing", 3.), ("adx_period", 2.), ("adx_lower", 40.), ("k_period", 5.), ("slowing", 2.), ("fast", 3.), ("slow", 6.)];
        for d in READY {
            for q in [Params::new(), d.params.iter().filter_map(|p| small.iter().find(|(k, _)| *k == p.key).map(|(k, v)| (k.to_string(), *v))).collect()] {
                let full = run(d, &w, &q).unwrap();
                for k in (30..500).step_by(7) {
                    for (f, g) in full.iter().zip(run(d, &cut(&w, k), &q).unwrap()) { assert!(g.iter().zip(&f[..k]).all(|(a, b)| close_eq(*a, *b)), "{} at cut {k}", d.id); }
                }
            }
        }
    }
    #[test] fn flat_short_and_extreme_inputs_are_safe() {
        let mut flat = walk(300, 5, false);
        for i in 0..300 { flat.open[i] = 100.; flat.high[i] = 100.; flat.low[i] = 100.; flat.close[i] = 100.; }
        for d in READY {
            for b in [&flat, &walk(0, 1, false), &walk(1, 1, false), &walk(3, 1, false), &walk(300, 1, false)] {
                for hi in [None, Some(false), Some(true)] {
                    let q: Params = hi.map_or(Params::new(), |hi| d.params.iter().map(|p| (p.key.to_string(), if hi { p.max } else { p.min })).collect());
                    let o = run(d, b, &q).unwrap();
                    assert!(o.len() == d.outputs.len() && o.iter().all(|s| s.len() == b.len() && s.iter().all(|v| !v.is_infinite())), "{}", d.id);
                }
            }
        }
    }
    /// BOOKER_BARS=<csv date,open,high,low,close> BOOKER_OUT=<json> cargo test --bin guru lab::x_tv_booker::tests::booker_dump -- --ignored --exact
    #[test] #[ignore] fn booker_dump() {
        let text = std::fs::read_to_string(std::env::var("BOOKER_BARS").expect("BOOKER_BARS")).expect("read BOOKER_BARS");
        let mut b = Bars::default();
        for (i, line) in text.lines().skip(1).filter(|s| !s.trim().is_empty()).enumerate() {
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            let x = |k: usize| f[k].parse::<f64>().unwrap_or(f64::NAN);
            b.date.push(f[0].to_string());
            b.open.push(x(1)); b.high.push(x(2)); b.low.push(x(3)); b.close.push(x(4)); b.end.push(i);
        }
        let num = |v: f64| if v.is_finite() { format!("{v:?}") } else { "null".into() };
        let small: Params = [("adx_lower", 25.)].iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let body: Vec<String> = READY.iter().flat_map(|d| [("", Params::new()), ("_l25", small.clone())].map(|(tag, q)| {
            let q: Params = q.into_iter().filter(|(k, _)| d.params.iter().any(|p| p.key == k)).collect();
            let out = run(d, &b, &q).unwrap_or_else(|e| panic!("{}: {e}", d.id));
            let cols: Vec<String> = d.outputs.iter().zip(&out).map(|(k, s)| format!("\"{k}\":[{}]", s.iter().map(|&v| num(v)).collect::<Vec<_>>().join(","))).collect();
            format!("\"{}{tag}\":{{{}}}", d.id, cols.join(","))
        })).collect();
        std::fs::write(std::env::var("BOOKER_OUT").expect("BOOKER_OUT"), format!("{{{}}}", body.join(","))).expect("write BOOKER_OUT");
    }
}
