//! TrendLine Trader (trendlinetrader.com) — the TradeStation strategy "TrendLine Trader" with its db* functions (open code).
//! Source (research drive, read in place, read-only): `USB HDD 1/TradeStation Systems & Indicators/TrendLine Trader (trendlinetrader.com)/
//! OpenCode/Open Code.rar` -> `TrendLine Trader.strategy.txt`, `dbPivot.txt`, `dbPivotHighVSBar.txt`, `dbPivotLowVSBar.txt`,
//! `dbDynamicTrendFnc.txt`, `dbLinearReg.txt`, `dbmomdirfast.txt`, `dbmomdirslow.txt`, `dbExtremes.txt`, `dbxaverage.txt`,
//! `dbavgtruerange.txt`, `dbTrueRange/High/Low.txt`, `dbAverage.txt`, `dbSummation.txt` (extracted as data), and the manual
//! `TLT_TradingManual.pdf` (stop defaults p.10). Book id `tltdr`. Notes: library/tltdr/README.md. Engine: books::rrsim.
use crate::books::rrsim::{g, o, pd, run_ts, sdef, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tltdr";
const ID: &str = "tltdr_trendline";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef(BOOK, ID, "TrendLine Trader (default inputs: ATR stops on, momentum filter on)", "TrendLine Trader (trendlinetrader.com)", "Long/Short", Status::Live, &[
        "F|Trendlines: when \"dbPivotHighVSBar (1, HIGH, 1, 1, 2) = 1\" the high line is reset to run from the previous pivot high to (DATE[1], HIGH[1]) and extended right (TL_SETBEGIN/TL_SETEND/TL_SETEXTRIGHT); same for lows. dbPivot: bar 1 is a pivot high if H[1] >= H[2], H[1] >= C[2], H[1] >= C[3] and H[1] > H[0] (low: mirror)",
        "F|Variable23 = dbDynamicTrendFnc(9, 7) = dbLinearReg(CLOSE, 9, 7) value (least-squares line of the last 9 closes evaluated 7 bars ago, code formula)",
        "E|Buy this bar on close when not long and either (a) \"CLOSE > TL_GETVALUE(high line) AND CLOSE > TL_GETVALUE(low line)\" or (b) close between the lines and both lines rising (\"TL_GETBEGINVAL < TL_GETENDVAL\"), AND \"LOW > Variable23 AND CLOSE >= OPEN AND HIGH > HIGH[1] AND HIGH > HIGH[2]\" AND the momentum filter (Momentum_Filter, MF_Fast_Direction, MF_Slow_Direction TRUE by default): dbmomdirfast(7,21,1,H,L) <> its value 1 bar ago AND dbmomdirslow(7,21,1,H,L) <> its value 1 bar ago",
        "E|Sell short this bar on close (mirror): when not short and either close below both lines, or between the lines with both lines falling, AND \"HIGH < Variable23 AND CLOSE <= OPEN AND LOW < LOW[1] AND LOW < LOW[2]\" AND the same momentum filter",
        "X|Stops (Stops_On TRUE, Dynamic_Stops TRUE; manual p.10 defaults ATR_Length 14, Initial_Stop_ATRs 4, Profit_Threshold_ATRs 4, Tightened_Stop_ATRs 4): on the bar after entry stop = entry close - 4 x dbavgtruerange(14)[1]; later bars candidate = HIGH[1] - 4 x ATR[1] (Tightened when HIGH[1] - entry >= 4 x ATR[1]); the stop only rises. \"IF Variable28 AND LOW < Variable32 ... THEN SELL THIS BAR CLOSE\" (short mirror with HIGH > stop, BUYTOCOVER this bar close)",
        "F|Trading Range, Entry Speed, Gap, Trend Strength filters and MF_Strength default FALSE (their text contains a garbled '/2Variables' that the defaults never evaluate); with Stops_On TRUE the trendline 'Cash' exits are off",
    ], "Code literal. TL_GETVALUE: a TradeStation trendline is a straight chart line, so its value at the current bar is extrapolated linearly in bar \
number from its begin and end points. Warm-up: the code starts at CurrentBar 1 = bar mbb (all lookbacks exist); the first pivot of each kind sets a \
line begin of date 0 / price 0, so the entry tests that read a line are taken as false until that line has two real pivots. All orders fill at the \
signal bar's close (\"THIS BAR CLOSE\"); an entry reverses an opposite position. The manual says the product works 'in most time frames' (examples \
on 1-minute charts, 'even if you are an end-of-day trader'); run on daily bars. India: instrument swap; index shorts hypothetical.")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some(("tltdr", "TrendLine Trader", "default inputs"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![
        pd("atr_len", "ATR_Length", 14., 5., 50., 1.),
        pd("init_atr", "Initial_Stop_ATRs", 4., 0.5, 10., 0.5),
        pd("thresh_atr", "Profit_Threshold_ATRs", 4., 0.5, 10., 0.5),
        pd("tight_atr", "Tightened_Stop_ATRs", 4., 0., 10., 0.5),
        pd("mf_fast", "MF_Fast_Length", 7., 2., 50., 1.),
        pd("mf_slow", "MF_Slow_Length", 21., 5., 100., 1.),
        pd("mbb", "Bars before CurrentBar 1 (MaxBarsBack)", 21., 21., 100., 1.),
    ])
}

/// dbLinearReg value (code formula) of the last n closes evaluated `tgt` bars ago.
pub fn db_linreg(c: &[f64], i: usize, n: usize, tgt: f64) -> f64 {
    let nf = n as f64;
    let v3 = nf * (nf - 1.) * 0.5;
    let v4 = nf * (nf - 1.) * (2. * nf - 1.) / 6.;
    let v6 = v3 * v3 - nf * v4;
    let v1: f64 = (0..n).map(|k| k as f64 * c[i - k]).sum();
    let v2: f64 = (0..n).map(|k| c[i - k]).sum();
    let slope = (nf * v1 - v3 * v2) / v6;
    let icpt = (v2 - slope * v3) / nf;
    icpt + slope * (nf - 1. - tgt)
}

#[derive(Clone, Copy)]
struct Line { b: Option<(usize, f64)>, e: Option<(usize, f64)> }
impl Line {
    fn val(&self, i: usize) -> Option<f64> {
        let ((bi, bv), (ei, ev)) = (self.b?, self.e?);
        if ei <= bi { return None; }
        Some(ev + (ev - bv) / (ei - bi) as f64 * (i - ei) as f64)
    }
    fn rising(&self) -> Option<bool> { Some(self.b?.1 < self.e?.1) }
    fn falling(&self) -> Option<bool> { Some(self.b?.1 > self.e?.1) }
    /// TL_SETBEGIN(previous pivot) / TL_SETEND(new pivot at bar j)
    fn push(&mut self, prev: Option<(usize, f64)>, j: usize, v: f64) { self.b = prev; self.e = Some((j, v)); }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let atr_len = g(p, "atr_len", 14.).max(1.) as usize;
    let (k_init, k_thr, k_tight) = (g(p, "init_atr", 4.), g(p, "thresh_atr", 4.), g(p, "tight_atr", 4.));
    let (nf, ns) = (g(p, "mf_fast", 7.).max(1.) as usize, g(p, "mf_slow", 21.).max(1.) as usize);
    let mbb = (g(p, "mbb", 21.) as usize).max(atr_len + 1).max(ns).max(nf).max(9).max(3);
    let (o_, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    // dbTrueRange = dbTrueHigh - dbTrueLow (C[1] vs H/L); dbavgtruerange = dbAverage(dbTrueRange, len)
    let tr: Vec<f64> = (0..n).map(|i| if i == 0 { h[0] - l[0] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect();
    let atr = |i: usize| (0..atr_len).map(|k| tr[i - k]).sum::<f64>() / atr_len as f64;
    let mid = |i: usize, len: usize| {
        let hh = (0..len).map(|k| h[i - k]).fold(f64::NEG_INFINITY, f64::max);
        let ll = (0..len).map(|k| l[i - k]).fold(f64::INFINITY, f64::min);
        ll + (hh - ll) / 2.
    };
    let (mut hl, mut ll) = (Line { b: None, e: None }, Line { b: None, e: None });
    let (mut ph, mut pl): (Option<(usize, f64)>, Option<(usize, f64)>) = (None, None);
    let (mut v28, mut v29, mut v35, mut v40) = (false, false, false, false);
    let (mut v32, mut v33, mut v36, mut v37, mut v38, mut v41) = (0f64, 0f64, 0f64, 0f64, 0f64, 0f64);
    Some(run_ts(x, from, to, |i, _st, now, _next| {
        if i < mbb { return; }
        let v23 = db_linreg(c, i, 9, 7.);
        // dynamic stops (ATR of the previous bar)
        let a1 = atr(i - 1);
        if v28 {
            if v35 { v32 = v33 - k_init * a1; }
            if !v35 && h[i - 1] - v33 < k_thr * a1 { v36 = h[i - 1] - k_init * a1; }
            if !v35 && h[i - 1] - v33 >= k_thr * a1 { v36 = h[i - 1] - k_tight * a1; }
            if v36 > v32 && !v35 { v32 = v36; }
        }
        if v29 {
            if v40 { v37 = v38 + k_init * a1; }
            if !v40 && v38 - l[i - 1] < k_thr * a1 { v41 = l[i - 1] + k_init * a1; }
            if !v40 && v38 - l[i - 1] >= k_thr * a1 { v41 = l[i - 1] + k_tight * a1; }
            if v41 < v37 && v41 != 0. && !v40 { v37 = v41; }
        }
        if v28 && l[i] < v32 && !v35 { now.push(o(A::Xl, P::Mkt, "LX stop")); v28 = false; }
        if v29 && h[i] > v37 && !v40 { now.push(o(A::Xs, P::Mkt, "SX stop")); v29 = false; }
        // BARSTATUS(1) = 2 (every historical bar)
        if h[i - 1] >= h[i - 2] && h[i - 1] >= c[i - 2] && h[i - 1] >= c[i - 3] && h[i - 1] > h[i] {
            hl.push(ph, i - 1, h[i - 1]); ph = Some((i - 1, h[i - 1]));
        }
        if l[i - 1] <= l[i - 2] && l[i - 1] <= c[i - 2] && l[i - 1] <= c[i - 3] && l[i - 1] < l[i] {
            ll.push(pl, i - 1, l[i - 1]); pl = Some((i - 1, l[i - 1]));
        }
        v35 = false;
        v40 = false;
        let mf = mid(i, nf) != mid(i - 1, nf) && mid(i, ns) != mid(i - 1, ns);
        let (th, tl) = (hl.val(i), ll.val(i));
        let both = |f: &dyn Fn(f64, f64) -> bool| matches!((th, tl), (Some(a), Some(b)) if f(a, b));
        let (cc, oo) = (c[i], o_[i]);
        let long_bar = cc >= oo && h[i] > h[i - 1] && h[i] > h[i - 2] && mf;
        let short_bar = cc <= oo && l[i] < l[i - 1] && l[i] < l[i - 2] && mf;
        let enter_long = |v28: bool| !v28 && long_bar && l[i] > v23 && (both(&|a, b| cc > a && cc > b)
            || (both(&|a, b| cc < a && cc > b) && hl.rising() == Some(true) && ll.rising() == Some(true)));
        if enter_long(v28) { now.push(o(A::Buy, P::Mkt, "B")); v28 = true; v29 = false; v33 = cc; v35 = true; }
        let enter_short = !v29 && short_bar && h[i] < v23 && (both(&|a, b| cc < a && cc < b)
            || (both(&|a, b| cc < a && cc > b) && hl.falling() == Some(true) && ll.falling() == Some(true)));
        if enter_short { now.push(o(A::Sell, P::Mkt, "S")); v28 = false; v29 = true; v38 = cc; v40 = true; }
    }))
}

#[cfg(test)]
mod tests {
    use super::db_linreg;
    #[test]
    fn linreg_value_7_bars_ago_on_a_straight_line() {
        // closes rise 1 per bar: the fitted line is exact, value 7 bars ago = close[7]
        let c: Vec<f64> = (0..20).map(|k| 100. + k as f64).collect();
        assert!((db_linreg(&c, 19, 9, 7.) - c[12]).abs() < 1e-9);
        assert!((db_linreg(&c, 19, 9, 0.) - c[19]).abs() < 1e-9);
    }
}
