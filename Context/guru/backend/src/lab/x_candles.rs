//! Cross-platform indicators, Candlestick patterns and Patterns (2026-10-19 build list: TradingView, AmiBroker, MetaTrader, cTrader, NinjaTrader, TradeStation, thinkorswim, Sierra Chart, MetaStock). Formulas: see each definition string.
//! The TA-Lib candles are line-by-line ports of ta-lib/ta-lib src/ta_func/ta_CDL*.c with the default TA_CandleSettings; they are not part of `cdl_all`.
use super::bars::Bars;
use super::catalogue::{int, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CDL: &str = "Candlestick patterns";
const PAT: &str = "Patterns";
const PN: Placement = Placement::Pane;
const PR: Placement = Placement::Price;
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs: Needs::NONE, definition, first, sparse, compute: Some(f), note: "" }
}

// ---- TA-Lib candle settings (ta_global.c TA_CANDLE_DEFAULT_SETTINGS) and ta_utility.h macros ----
#[derive(Clone, Copy)]
enum Rt { Body, Hl, Shadows }
/// (range type, average period, factor)
#[derive(Clone, Copy)]
pub(super) struct S(Rt, usize, f64);
pub(super) const BODY_LONG: S = S(Rt::Body, 10, 1.);
pub(super) const BODY_SHORT: S = S(Rt::Body, 10, 1.);
const BODY_DOJI: S = S(Rt::Hl, 10, 0.1);
const SHADOW_LONG: S = S(Rt::Body, 0, 1.);
const SHADOW_VERY_LONG: S = S(Rt::Body, 0, 2.);
const SHADOW_SHORT: S = S(Rt::Shadows, 10, 1.);
pub(super) const SHADOW_VERY_SHORT: S = S(Rt::Hl, 10, 0.1);
const NEAR: S = S(Rt::Hl, 5, 0.2);
const FAR: S = S(Rt::Hl, 5, 0.6);
const EQUAL: S = S(Rt::Hl, 5, 0.05);

pub(super) struct K<'a> { o: &'a [f64], h: &'a [f64], l: &'a [f64], c: &'a [f64] }
impl K<'_> {
    pub(super) fn new(b: &Bars) -> K<'_> { K { o: &b.open, h: &b.high, l: &b.low, c: &b.close } }
    pub(super) fn rb(&self, i: usize) -> f64 { (self.c[i] - self.o[i]).abs() }
    pub(super) fn top(&self, i: usize) -> f64 { if self.c[i] >= self.o[i] { self.c[i] } else { self.o[i] } }
    pub(super) fn bot(&self, i: usize) -> f64 { if self.c[i] >= self.o[i] { self.o[i] } else { self.c[i] } }
    pub(super) fn us(&self, i: usize) -> f64 { self.h[i] - self.top(i) }
    pub(super) fn ls(&self, i: usize) -> f64 { self.bot(i) - self.l[i] }
    /// TA_CANDLECOLOR: close >= open is white (+1).
    pub(super) fn col(&self, i: usize) -> i32 { if self.c[i] >= self.o[i] { 1 } else { -1 } }
    pub(super) fn gap_up(&self, i2: usize, i1: usize) -> bool { self.bot(i2) > self.top(i1) }
    pub(super) fn gap_dn(&self, i2: usize, i1: usize) -> bool { self.top(i2) < self.bot(i1) }
    fn range(&self, s: S, i: usize) -> f64 {
        match s.0 { Rt::Body => self.rb(i), Rt::Hl => self.h[i] - self.l[i], Rt::Shadows => self.us(i) + self.ls(i) }
    }
    /// TA_CANDLEAVERAGE for candle `i`: factor × (sum of the range over the `period` bars before `i`) / period, or the candle's own range when period is 0; halved for Shadows.
    // shortcut: a direct window sum where TA-Lib rolls a running total, so equality cases can differ in the last bit; acceptable for 0/1 signals.
    pub(super) fn avg(&self, s: S, i: usize) -> f64 {
        let base = if s.1 == 0 { self.range(s, i) } else { (i - s.1..i).map(|j| self.range(s, j)).sum::<f64>() / s.1 as f64 };
        s.2 * base / if matches!(s.0, Rt::Shadows) { 2. } else { 1. }
    }
}
/// TA-Lib integer output (±100/±200, 0) for every bar; bars before the function's lookback stay 0, as python TA-Lib reports them.
fn scan(k: &K, lookback: usize, f: impl Fn(usize) -> i32) -> Vec<i32> { (0..k.c.len()).map(|i| if i >= lookback { f(i) } else { 0 }).collect() }
fn hund(hit: bool, sign: i32) -> i32 { if hit { sign * 100 } else { 0 } }

fn advance_block(k: &K) -> Vec<i32> {
    scan(k, 12, |i| hund(k.col(i - 2) == 1 && k.col(i - 1) == 1 && k.col(i) == 1
        && k.c[i] > k.c[i - 1] && k.c[i - 1] > k.c[i - 2]
        && k.o[i - 1] > k.o[i - 2] && k.o[i - 1] <= k.c[i - 2] + k.avg(NEAR, i - 2)
        && k.o[i] > k.o[i - 1] && k.o[i] <= k.c[i - 1] + k.avg(NEAR, i - 1)
        && k.rb(i - 2) > k.avg(BODY_LONG, i - 2)
        && k.us(i - 2) < k.avg(SHADOW_SHORT, i - 2)
        && ((k.rb(i - 1) < k.rb(i - 2) - k.avg(FAR, i - 2) && k.rb(i) < k.rb(i - 1) + k.avg(NEAR, i - 1))
            || k.rb(i) < k.rb(i - 1) - k.avg(FAR, i - 1)
            || (k.rb(i) < k.rb(i - 1) && k.rb(i - 1) < k.rb(i - 2) && (k.us(i) > k.avg(SHADOW_SHORT, i) || k.us(i - 1) > k.avg(SHADOW_SHORT, i - 1)))
            || (k.rb(i) < k.rb(i - 1) && k.us(i) > k.avg(SHADOW_LONG, i))), -1))
}
fn belt_hold(k: &K) -> Vec<i32> {
    scan(k, 10, |i| hund(k.rb(i) > k.avg(BODY_LONG, i)
        && ((k.col(i) == 1 && k.ls(i) < k.avg(SHADOW_VERY_SHORT, i)) || (k.col(i) == -1 && k.us(i) < k.avg(SHADOW_VERY_SHORT, i))), k.col(i)))
}
fn breakaway(k: &K) -> Vec<i32> {
    scan(k, 14, |i| hund(k.col(i - 4) == k.col(i - 3) && k.col(i - 3) == k.col(i - 1) && k.col(i - 1) == -k.col(i)
        && k.rb(i - 4) > k.avg(BODY_LONG, i - 4)
        && ((k.col(i - 4) == -1 && k.gap_dn(i - 3, i - 4) && k.h[i - 2] < k.h[i - 3] && k.l[i - 2] < k.l[i - 3] && k.h[i - 1] < k.h[i - 2] && k.l[i - 1] < k.l[i - 2]
                && k.c[i] > k.o[i - 3] && k.c[i] < k.c[i - 4])
            || (k.col(i - 4) == 1 && k.gap_up(i - 3, i - 4) && k.h[i - 2] > k.h[i - 3] && k.l[i - 2] > k.l[i - 3] && k.h[i - 1] > k.h[i - 2] && k.l[i - 1] > k.l[i - 2]
                && k.c[i] < k.o[i - 3] && k.c[i] > k.c[i - 4])), k.col(i)))
}
fn closing_marubozu(k: &K) -> Vec<i32> {
    scan(k, 10, |i| hund(k.rb(i) > k.avg(BODY_LONG, i)
        && ((k.col(i) == 1 && k.us(i) < k.avg(SHADOW_VERY_SHORT, i)) || (k.col(i) == -1 && k.ls(i) < k.avg(SHADOW_VERY_SHORT, i))), k.col(i)))
}
fn conceal_baby_swallow(k: &K) -> Vec<i32> {
    scan(k, 13, |i| hund((0..4).all(|j| k.col(i - j) == -1)
        && k.ls(i - 3) < k.avg(SHADOW_VERY_SHORT, i - 3) && k.us(i - 3) < k.avg(SHADOW_VERY_SHORT, i - 3)
        && k.ls(i - 2) < k.avg(SHADOW_VERY_SHORT, i - 2) && k.us(i - 2) < k.avg(SHADOW_VERY_SHORT, i - 2)
        && k.gap_dn(i - 1, i - 2) && k.us(i - 1) > k.avg(SHADOW_VERY_SHORT, i - 1) && k.h[i - 1] > k.c[i - 2]
        && k.h[i] > k.h[i - 1] && k.l[i] < k.l[i - 1], 1))
}
fn counterattack(k: &K) -> Vec<i32> {
    scan(k, 11, |i| hund(k.col(i - 1) == -k.col(i) && k.rb(i - 1) > k.avg(BODY_LONG, i - 1) && k.rb(i) > k.avg(BODY_LONG, i)
        && k.c[i] <= k.c[i - 1] + k.avg(EQUAL, i - 1) && k.c[i] >= k.c[i - 1] - k.avg(EQUAL, i - 1), k.col(i)))
}
fn xside_gap_3_methods(k: &K) -> Vec<i32> {
    scan(k, 2, |i| hund(k.col(i - 2) == k.col(i - 1) && k.col(i - 1) == -k.col(i)
        && k.o[i] < k.top(i - 1) && k.o[i] > k.bot(i - 1) && k.c[i] < k.top(i - 2) && k.c[i] > k.bot(i - 2)
        && ((k.col(i - 2) == 1 && k.gap_up(i - 1, i - 2)) || (k.col(i - 2) == -1 && k.gap_dn(i - 1, i - 2))), k.col(i - 2)))
}
fn high_wave(k: &K) -> Vec<i32> {
    scan(k, 10, |i| hund(k.rb(i) < k.avg(BODY_SHORT, i) && k.us(i) > k.avg(SHADOW_VERY_LONG, i) && k.ls(i) > k.avg(SHADOW_VERY_LONG, i), k.col(i)))
}
/// CDLHIKKAKE / CDLHIKKAKEMOD state machine; `start` is where TA-Lib's priming loop begins (lookback − 3), outputs from `lookback`.
fn hikkake_like(k: &K, start: usize, lookback: usize, setup: impl Fn(usize) -> Option<i32>) -> Vec<i32> {
    let mut out = vec![0; k.c.len()];
    let (mut result, mut count, mut hi, mut lo) = (0, 0, 0., 0.);
    for i in start..k.c.len() {
        let v = if let Some(r) = setup(i) {
            (result, hi, lo, count) = (r, k.h[i - 1], k.l[i - 1], 4);
            r
        } else if count > 0 && ((result > 0 && k.c[i] > hi) || (result < 0 && k.c[i] < lo)) {
            count = 0;
            result + 100 * result.signum()
        } else { 0 };
        if i >= lookback { out[i] = v; }
        if count > 0 { count -= 1; }
    }
    out
}
fn hikkake(k: &K) -> Vec<i32> {
    hikkake_like(k, 2, 5, |i| (k.h[i - 1] < k.h[i - 2] && k.l[i - 1] > k.l[i - 2]
        && ((k.h[i] < k.h[i - 1] && k.l[i] < k.l[i - 1]) || (k.h[i] > k.h[i - 1] && k.l[i] > k.l[i - 1])))
        .then(|| if k.h[i] < k.h[i - 1] { 100 } else { -100 }))
}
fn hikkake_mod(k: &K) -> Vec<i32> {
    hikkake_like(k, 7, 10, |i| (k.h[i - 2] < k.h[i - 3] && k.l[i - 2] > k.l[i - 3] && k.h[i - 1] < k.h[i - 2] && k.l[i - 1] > k.l[i - 2]
        && ((k.h[i] < k.h[i - 1] && k.l[i] < k.l[i - 1] && k.c[i - 2] <= k.l[i - 2] + k.avg(NEAR, i - 2))
            || (k.h[i] > k.h[i - 1] && k.l[i] > k.l[i - 1] && k.c[i - 2] >= k.h[i - 2] - k.avg(NEAR, i - 2))))
        .then(|| if k.h[i] < k.h[i - 1] { 100 } else { -100 }))
}
fn homing_pigeon(k: &K) -> Vec<i32> {
    scan(k, 11, |i| hund(k.col(i - 1) == -1 && k.col(i) == -1 && k.rb(i - 1) > k.avg(BODY_LONG, i - 1) && k.rb(i) <= k.avg(BODY_SHORT, i)
        && k.o[i] < k.o[i - 1] && k.c[i] > k.c[i - 1], 1))
}
fn identical_3_crows(k: &K) -> Vec<i32> {
    scan(k, 12, |i| hund((0..3).all(|j| k.col(i - j) == -1 && k.ls(i - j) < k.avg(SHADOW_VERY_SHORT, i - j))
        && k.c[i - 2] > k.c[i - 1] && k.c[i - 1] > k.c[i]
        && k.o[i - 1] <= k.c[i - 2] + k.avg(EQUAL, i - 2) && k.o[i - 1] >= k.c[i - 2] - k.avg(EQUAL, i - 2)
        && k.o[i] <= k.c[i - 1] + k.avg(EQUAL, i - 1) && k.o[i] >= k.c[i - 1] - k.avg(EQUAL, i - 1), -1))
}
fn in_neck(k: &K) -> Vec<i32> {
    scan(k, 11, |i| hund(k.col(i - 1) == -1 && k.rb(i - 1) > k.avg(BODY_LONG, i - 1) && k.col(i) == 1 && k.o[i] < k.l[i - 1]
        && k.c[i] <= k.c[i - 1] + k.avg(EQUAL, i - 1) && k.c[i] >= k.c[i - 1], -1))
}
fn ladder_bottom(k: &K) -> Vec<i32> {
    scan(k, 14, |i| hund(k.col(i - 4) == -1 && k.col(i - 3) == -1 && k.col(i - 2) == -1
        && k.o[i - 4] > k.o[i - 3] && k.o[i - 3] > k.o[i - 2] && k.c[i - 4] > k.c[i - 3] && k.c[i - 3] > k.c[i - 2]
        && k.col(i - 1) == -1 && k.us(i - 1) > k.avg(SHADOW_VERY_SHORT, i - 1)
        && k.col(i) == 1 && k.o[i] > k.o[i - 1] && k.c[i] > k.h[i - 1], 1))
}
fn long_legged_doji(k: &K) -> Vec<i32> {
    scan(k, 10, |i| hund(k.rb(i) <= k.avg(BODY_DOJI, i) && (k.ls(i) > k.avg(SHADOW_LONG, i) || k.us(i) > k.avg(SHADOW_LONG, i)), 1))
}
fn long_line(k: &K) -> Vec<i32> {
    scan(k, 10, |i| hund(k.rb(i) > k.avg(BODY_LONG, i) && k.us(i) < k.avg(SHADOW_SHORT, i) && k.ls(i) < k.avg(SHADOW_SHORT, i), k.col(i)))
}
fn mat_hold(k: &K) -> Vec<i32> {
    const PEN: f64 = 0.5; // optInPenetration default
    scan(k, 14, |i| hund(k.col(i - 4) == 1 && k.col(i - 3) == -1 && k.col(i) == 1 && k.gap_up(i - 3, i - 4)
        && k.bot(i - 2) < k.c[i - 4] && k.bot(i - 1) < k.c[i - 4]
        && k.bot(i - 2) > k.c[i - 4] - k.rb(i - 4) * PEN && k.bot(i - 1) > k.c[i - 4] - k.rb(i - 4) * PEN
        && k.top(i - 2) < k.o[i - 3] && k.top(i - 1) < k.top(i - 2)
        && k.o[i] > k.c[i - 1] && k.c[i] > k.h[i - 3].max(k.h[i - 2]).max(k.h[i - 1])
        && k.rb(i - 4) > k.avg(BODY_LONG, i - 4) && (1..4).all(|j| k.rb(i - j) < k.avg(BODY_SHORT, i - j)), 1))
}
fn matching_low(k: &K) -> Vec<i32> {
    scan(k, 6, |i| hund(k.col(i - 1) == -1 && k.col(i) == -1 && k.c[i] <= k.c[i - 1] + k.avg(EQUAL, i - 1) && k.c[i] >= k.c[i - 1] - k.avg(EQUAL, i - 1), 1))
}
fn rickshaw_man(k: &K) -> Vec<i32> {
    scan(k, 10, |i| {
        let mid = k.l[i] + (k.h[i] - k.l[i]) / 2.;
        hund(k.rb(i) <= k.avg(BODY_DOJI, i) && k.ls(i) > k.avg(SHADOW_LONG, i) && k.us(i) > k.avg(SHADOW_LONG, i)
            && k.bot(i) <= mid + k.avg(NEAR, i) && k.top(i) >= mid - k.avg(NEAR, i), 1)
    })
}
fn separating_lines(k: &K) -> Vec<i32> {
    scan(k, 11, |i| hund(k.col(i - 1) == -k.col(i) && k.o[i] <= k.o[i - 1] + k.avg(EQUAL, i - 1) && k.o[i] >= k.o[i - 1] - k.avg(EQUAL, i - 1)
        && k.rb(i) > k.avg(BODY_LONG, i)
        && ((k.col(i) == 1 && k.ls(i) < k.avg(SHADOW_VERY_SHORT, i)) || (k.col(i) == -1 && k.us(i) < k.avg(SHADOW_VERY_SHORT, i))), k.col(i)))
}
fn short_line(k: &K) -> Vec<i32> {
    scan(k, 10, |i| hund(k.rb(i) < k.avg(BODY_SHORT, i) && k.us(i) < k.avg(SHADOW_SHORT, i) && k.ls(i) < k.avg(SHADOW_SHORT, i), k.col(i)))
}
fn gap_side_side_white(k: &K) -> Vec<i32> {
    scan(k, 7, |i| hund(((k.gap_up(i - 1, i - 2) && k.gap_up(i, i - 2)) || (k.gap_dn(i - 1, i - 2) && k.gap_dn(i, i - 2)))
        && k.col(i - 1) == 1 && k.col(i) == 1
        && k.rb(i) >= k.rb(i - 1) - k.avg(NEAR, i - 1) && k.rb(i) <= k.rb(i - 1) + k.avg(NEAR, i - 1)
        && k.o[i] >= k.o[i - 1] - k.avg(EQUAL, i - 1) && k.o[i] <= k.o[i - 1] + k.avg(EQUAL, i - 1), if k.gap_up(i - 1, i - 2) { 1 } else { -1 }))
}
fn stalled_pattern(k: &K) -> Vec<i32> {
    scan(k, 12, |i| hund(k.col(i - 2) == 1 && k.col(i - 1) == 1 && k.col(i) == 1 && k.c[i] > k.c[i - 1] && k.c[i - 1] > k.c[i - 2]
        && k.rb(i - 2) > k.avg(BODY_LONG, i - 2) && k.rb(i - 1) > k.avg(BODY_LONG, i - 1) && k.us(i - 1) < k.avg(SHADOW_VERY_SHORT, i - 1)
        && k.o[i - 1] > k.o[i - 2] && k.o[i - 1] <= k.c[i - 2] + k.avg(NEAR, i - 2)
        && k.rb(i) < k.avg(BODY_SHORT, i) && k.o[i] >= k.c[i - 1] - k.rb(i) - k.avg(NEAR, i - 1), -1))
}
fn stick_sandwich(k: &K) -> Vec<i32> {
    scan(k, 7, |i| hund(k.col(i - 2) == -1 && k.col(i - 1) == 1 && k.col(i) == -1 && k.l[i - 1] > k.c[i - 2]
        && k.c[i] <= k.c[i - 2] + k.avg(EQUAL, i - 2) && k.c[i] >= k.c[i - 2] - k.avg(EQUAL, i - 2), 1))
}
fn takuri(k: &K) -> Vec<i32> {
    scan(k, 10, |i| hund(k.rb(i) <= k.avg(BODY_DOJI, i) && k.us(i) < k.avg(SHADOW_VERY_SHORT, i) && k.ls(i) > k.avg(SHADOW_VERY_LONG, i), 1))
}
fn three_inside(k: &K) -> Vec<i32> {
    scan(k, 12, |i| hund(k.top(i - 1) < k.top(i - 2) && k.bot(i - 1) > k.bot(i - 2)
        && ((k.col(i - 2) == 1 && k.col(i) == -1 && k.c[i] < k.o[i - 2]) || (k.col(i - 2) == -1 && k.col(i) == 1 && k.c[i] > k.o[i - 2]))
        && k.rb(i - 2) > k.avg(BODY_LONG, i - 2) && k.rb(i - 1) <= k.avg(BODY_SHORT, i - 1), -k.col(i - 2)))
}
fn three_line_strike(k: &K) -> Vec<i32> {
    scan(k, 8, |i| hund(k.col(i - 3) == k.col(i - 2) && k.col(i - 2) == k.col(i - 1) && k.col(i) == -k.col(i - 1)
        && k.o[i - 2] >= k.bot(i - 3) - k.avg(NEAR, i - 3) && k.o[i - 2] <= k.top(i - 3) + k.avg(NEAR, i - 3)
        && k.o[i - 1] >= k.bot(i - 2) - k.avg(NEAR, i - 2) && k.o[i - 1] <= k.top(i - 2) + k.avg(NEAR, i - 2)
        && ((k.col(i - 1) == 1 && k.c[i - 1] > k.c[i - 2] && k.c[i - 2] > k.c[i - 3] && k.o[i] > k.c[i - 1] && k.c[i] < k.o[i - 3])
            || (k.col(i - 1) == -1 && k.c[i - 1] < k.c[i - 2] && k.c[i - 2] < k.c[i - 3] && k.o[i] < k.c[i - 1] && k.c[i] > k.o[i - 3])), k.col(i - 1)))
}
fn three_outside(k: &K) -> Vec<i32> {
    scan(k, 3, |i| hund((k.col(i - 1) == 1 && k.col(i - 2) == -1 && k.c[i - 1] > k.o[i - 2] && k.o[i - 1] < k.c[i - 2] && k.c[i] > k.c[i - 1])
        || (k.col(i - 1) == -1 && k.col(i - 2) == 1 && k.o[i - 1] > k.c[i - 2] && k.c[i - 1] < k.o[i - 2] && k.c[i] < k.c[i - 1]), k.col(i - 1)))
}
fn three_stars_in_south(k: &K) -> Vec<i32> {
    scan(k, 12, |i| hund(k.col(i - 2) == -1 && k.col(i - 1) == -1 && k.col(i) == -1
        && k.rb(i - 2) > k.avg(BODY_LONG, i - 2) && k.ls(i - 2) > k.avg(SHADOW_LONG, i - 2)
        && k.rb(i - 1) < k.rb(i - 2) && k.o[i - 1] > k.c[i - 2] && k.o[i - 1] <= k.h[i - 2] && k.l[i - 1] < k.c[i - 2] && k.l[i - 1] >= k.l[i - 2]
        && k.ls(i - 1) > k.avg(SHADOW_VERY_SHORT, i - 1)
        && k.rb(i) < k.avg(BODY_SHORT, i) && k.ls(i) < k.avg(SHADOW_VERY_SHORT, i) && k.us(i) < k.avg(SHADOW_VERY_SHORT, i)
        && k.l[i] > k.l[i - 1] && k.h[i] < k.h[i - 1], 1))
}
fn thrusting(k: &K) -> Vec<i32> {
    scan(k, 11, |i| hund(k.col(i - 1) == -1 && k.rb(i - 1) > k.avg(BODY_LONG, i - 1) && k.col(i) == 1 && k.o[i] < k.l[i - 1]
        && k.c[i] > k.c[i - 1] + k.avg(EQUAL, i - 1) && k.c[i] <= k.rb(i - 1).mul_add(0.5, k.c[i - 1]), -1))
}
fn two_crows(k: &K) -> Vec<i32> {
    scan(k, 12, |i| hund(k.col(i - 2) == 1 && k.rb(i - 2) > k.avg(BODY_LONG, i - 2) && k.col(i - 1) == -1 && k.gap_up(i - 1, i - 2)
        && k.col(i) == -1 && k.o[i] < k.o[i - 1] && k.o[i] > k.c[i - 1] && k.c[i] > k.o[i - 2] && k.c[i] < k.c[i - 2], -1))
}
fn unique_3_river(k: &K) -> Vec<i32> {
    scan(k, 12, |i| hund(k.col(i - 2) == -1 && k.col(i - 1) == -1 && k.col(i) == 1
        && k.c[i - 1] > k.c[i - 2] && k.o[i - 1] <= k.o[i - 2] && k.l[i - 1] < k.l[i - 2] && k.o[i] > k.l[i - 1]
        && k.rb(i - 2) > k.avg(BODY_LONG, i - 2) && k.rb(i) < k.avg(BODY_SHORT, i), 1))
}
fn upside_gap_2_crows(k: &K) -> Vec<i32> {
    scan(k, 12, |i| hund(k.col(i - 2) == 1 && k.rb(i - 2) > k.avg(BODY_LONG, i - 2) && k.col(i - 1) == -1 && k.rb(i - 1) <= k.avg(BODY_SHORT, i - 1)
        && k.gap_up(i - 1, i - 2) && k.col(i) == -1 && k.o[i] > k.o[i - 1] && k.c[i] < k.c[i - 1] && k.c[i] > k.c[i - 2], -1))
}

/// Keeps one sign of a TA-Lib output: `+1` positive, `-1` negative, `0` either; the value is |output|/100 (1, or 2 for a hikkake confirmation).
fn pick(v: Vec<i32>, sign: i32) -> Vec<f64> {
    v.into_iter().map(|x| if (sign == 0 && x != 0) || x * sign > 0 { (x.abs() / 100) as f64 } else { 0. }).collect()
}
macro_rules! talib {
    ($($run:ident = $f:ident, $sign:expr;)*) => { $(fn $run(b: &Bars, _: &Params) -> Output { Ok(vec![pick($f(&K::new(b)), $sign)]) })* };
}
talib! {
    r_advance_block = advance_block, -1; r_belt_hold_bear = belt_hold, -1; r_belt_hold_bull = belt_hold, 1; r_breakaway_bear = breakaway, -1; r_breakaway_bull = breakaway, 1;
    r_closing_marubozu_black = closing_marubozu, -1; r_closing_marubozu_white = closing_marubozu, 1; r_concealing_baby_swallow = conceal_baby_swallow, 1;
    r_counterattack_bear = counterattack, -1; r_counterattack_bull = counterattack, 1; r_downside_gap_three_methods = xside_gap_3_methods, -1;
    r_upside_gap_three_methods = xside_gap_3_methods, 1; r_high_wave = high_wave, 0; r_hikkake_bear = hikkake, -1; r_hikkake_bull = hikkake, 1;
    r_hikkake_mod_bear = hikkake_mod, -1; r_hikkake_mod_bull = hikkake_mod, 1; r_homing_pigeon = homing_pigeon, 1; r_identical_three_crows = identical_3_crows, -1;
    r_in_neck = in_neck, -1; r_ladder_bottom = ladder_bottom, 1; r_long_legged_doji = long_legged_doji, 0; r_long_line = long_line, 0; r_mat_hold = mat_hold, 1;
    r_matching_low = matching_low, 1; r_rickshaw_man = rickshaw_man, 0; r_separating_lines_bear = separating_lines, -1; r_separating_lines_bull = separating_lines, 1;
    r_short_line = short_line, 0; r_side_by_side_white_down = gap_side_side_white, -1; r_side_by_side_white_up = gap_side_side_white, 1;
    r_stalled_pattern = stalled_pattern, -1; r_stick_sandwich = stick_sandwich, 1; r_takuri = takuri, 0; r_three_inside_down = three_inside, -1;
    r_three_inside_up = three_inside, 1; r_three_line_strike_bear = three_line_strike, -1; r_three_line_strike_bull = three_line_strike, 1;
    r_three_outside_down = three_outside, -1; r_three_outside_up = three_outside, 1; r_three_stars_south = three_stars_in_south, 1; r_thrusting = thrusting, -1;
    r_two_crows = two_crows, -1; r_unique_three_river = unique_3_river, 1; r_upside_gap_two_crows = upside_gap_2_crows, -1;
}

// ---- thinkorswim One Black Crow / One White Soldier ----
const TREND: &[ParamSpec] = &[int("trend", "Trend rule (0 SMA50, 1 SMA50+SMA200, 2 none)", 0., 0., 2.)];
/// thinkScript IsLongWhite/IsLongBlack(20): colour, body longer than each shadow, body above the average body of the 20 preceding candles.
fn tos_long(k: &K, i: usize, white: bool) -> bool {
    const LEN: usize = 20;
    i >= LEN && (if white { k.c[i] > k.o[i] } else { k.c[i] < k.o[i] }) && k.rb(i) > k.us(i) && k.rb(i) > k.ls(i)
        && k.rb(i) > (i - LEN..i).map(|j| k.rb(j)).sum::<f64>() / LEN as f64
}
/// candles.rs trend rule on close (0 SMA50, 1 SMA50 and SMA200, 2 off): (up, down) per bar.
fn trend(b: &Bars, rule: usize) -> Vec<(bool, bool)> {
    let (c, s50, s200) = (&b.close, ta::sma(&b.close, 50), ta::sma(&b.close, 200));
    (0..b.len()).map(|i| match rule {
        0 => (c[i] > s50[i], c[i] < s50[i]),
        1 => (c[i] > s50[i] && s50[i] > s200[i], c[i] < s50[i] && s50[i] < s200[i]),
        _ => (true, true),
    }).collect()
}
fn one_crow_or_soldier(b: &Bars, prm: &Params, bull: bool) -> Output {
    let (k, t) = (K::new(b), trend(b, n(prm, "trend")));
    Ok(vec![(0..b.len()).map(|i| {
        let hit = i >= 1 && if bull {
            t[i - 1].1 && tos_long(&k, i - 1, false) && tos_long(&k, i, true) && k.o[i] > k.c[i - 1] && k.o[i] < k.o[i - 1] && k.c[i] > k.o[i - 1]
        } else {
            t[i - 1].0 && tos_long(&k, i - 1, true) && tos_long(&k, i, false) && k.o[i] < k.c[i - 1] && k.o[i] > k.o[i - 1] && k.c[i] < k.o[i - 1]
        };
        if hit { 1. } else { 0. }
    }).collect()])
}
fn one_black_crow(b: &Bars, prm: &Params) -> Output { one_crow_or_soldier(b, prm, false) }
fn one_white_soldier(b: &Bars, prm: &Params) -> Output { one_crow_or_soldier(b, prm, true) }

// ---- Patterns ----
const DARVAS: &[ParamSpec] = &[int("box_length", "Box length", 5., 3., 2000.)];
fn darvas(b: &Bars, prm: &Params) -> Output {
    let (bl, len) = (n(prm, "box_length"), b.len());
    let (ll, k1, k2, k3) = (ta::lowest(&b.low, bl), ta::highest(&b.high, bl), ta::highest(&b.high, bl - 1), ta::highest(&b.high, bl - 2));
    let (mut top, mut bottom, mut brk) = (vec![NA; len], vec![NA; len], vec![0.; len]);
    let (mut nh, mut since, mut t, mut bo) = (NA, None::<usize>, NA, NA);
    for i in 0..len {
        if i >= 1 && b.high[i] > k1[i - 1] { nh = b.high[i]; since = Some(0); } else if let Some(s) = since.as_mut() { *s += 1; }
        if since == Some(bl - 2) && k3[i] < k2[i] { t = nh; bo = ll[i]; }
        (top[i], bottom[i]) = (t, bo);
        // Pine crossover/crossunder: false while either side is na
        if i >= 1 && b.close[i] > top[i] && b.close[i - 1] <= top[i - 1] { brk[i] = 1.; }
        if i >= 1 && b.close[i] < bottom[i] && b.close[i - 1] >= bottom[i - 1] { brk[i] = -1.; }
    }
    Ok(vec![top, bottom, brk])
}

const FVG: &[ParamSpec] = &[real("min_gap", "Min gap (price units)", 0., 0., 1e9, 0.01), int("max_zones", "Tracked zones", 10., 1., 100.)];
fn fvg(b: &Bars, prm: &Params) -> Output {
    let (min_gap, max_zones, len) = (p(prm, "min_gap"), n(prm, "max_zones"), b.len());
    let mut out = vec![vec![0.; len], vec![0.; len], vec![NA; len], vec![NA; len], vec![NA; len], vec![NA; len]];
    let (mut bull, mut bear): (Vec<(f64, f64)>, Vec<(f64, f64)>) = (vec![], vec![]); // (bottom, top), oldest first
    for i in 0..len {
        // NinjaTrader: a gap is filled once a bar closes through the whole zone
        bull.retain(|z| b.close[i] >= z.0);
        bear.retain(|z| b.close[i] <= z.1);
        if i >= 2 {
            let (up, dn) = (b.low[i] - b.high[i - 2], b.low[i - 2] - b.high[i]);
            if up > 0. && up >= min_gap { bull.push((b.high[i - 2], b.low[i])); out[0][i] = 1.; }
            if dn > 0. && dn >= min_gap { bear.push((b.high[i], b.low[i - 2])); out[1][i] = 1.; }
            for z in [&mut bull, &mut bear] { if z.len() > max_zones { z.remove(0); } }
        }
        if let Some(z) = bull.last() { (out[2][i], out[3][i]) = (z.1, z.0); }
        if let Some(z) = bear.last() { (out[4][i], out[5][i]) = (z.1, z.0); }
    }
    Ok(out)
}

const GAPS: &[ParamSpec] = &[real("min_dev", "Min gap % of avg range", 30., 0., 1000., 1.), int("range_length", "Average range length", 14., 1., 2000.), int("partial", "Close gaps partially (0/1)", 0., 0., 1.)];
fn gaps(b: &Bars, prm: &Params) -> Output {
    let (dev, partial, len) = (p(prm, "min_dev") / 100., n(prm, "partial") == 1, b.len());
    let rng: Vec<f64> = (0..len).map(|i| b.high[i] - b.low[i]).collect();
    let avg = ta::sma(&rng, n(prm, "range_length"));
    let mut out = vec![vec![0.; len], vec![0.; len], vec![0.; len], vec![0.; len], vec![NA; len], vec![NA; len], vec![NA; len], vec![NA; len]];
    let (mut ups, mut dns): (Vec<(f64, f64)>, Vec<(f64, f64)>) = (vec![], vec![]); // (bottom, top), oldest first
    for i in 0..len {
        let (h, l) = (b.high[i], b.low[i]);
        let (mut fu, mut fd) = (false, false);
        // a bar entering an open gap closes it (partial: closes only the covered part)
        ups.retain_mut(|z| if l >= z.1 { true } else if partial && l > z.0 { z.1 = l; true } else { fu = true; false });
        dns.retain_mut(|z| if h <= z.0 { true } else if partial && h < z.1 { z.0 = h; true } else { fd = true; false });
        if fu { out[2][i] = 1.; }
        if fd { out[3][i] = 1.; }
        if i >= 1 {
            let min = dev * avg[i - 1]; // declared: the average range excludes the gap bar
            if l > b.high[i - 1] && l - b.high[i - 1] >= min { ups.push((b.high[i - 1], l)); out[0][i] = 1.; }
            if h < b.low[i - 1] && b.low[i - 1] - h >= min { dns.push((h, b.low[i - 1])); out[1][i] = 1.; }
        }
        if let Some(z) = ups.last() { (out[4][i], out[5][i]) = (z.1, z.0); }
        if let Some(z) = dns.last() { (out[6][i], out[7][i]) = (z.1, z.0); }
    }
    Ok(out)
}

const ISLAND: &[ParamSpec] = &[int("max_bars", "Max island length", 10., 1., 2000.)];
fn island_reversal(b: &Bars, prm: &Params) -> Output {
    let (m, len, h, l) = (n(prm, "max_bars"), b.len(), &b.high, &b.low);
    let mut out = vec![vec![0.; len]; 2];
    for t in 2..len {
        let (gap_dn, gap_up) = (h[t] < l[t - 1], l[t] > h[t - 1]);
        if !gap_dn && !gap_up { continue; }
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY); // extremes of the island bars s..t-1
        for s in (t.saturating_sub(m).max(1)..t).rev() {
            (lo, hi) = (lo.min(l[s]), hi.max(h[s]));
            if gap_dn && lo > h[s - 1] && lo > h[t] { out[0][t] = 1.; }
            if gap_up && hi < l[s - 1] && hi < l[t] { out[1][t] = 1.; }
        }
    }
    Ok(out)
}

const RALLY: &[ParamSpec] = &[int("with_volume", "Require volume confirmation (0/1)", 0., 0., 1.)];
fn rally_reaction(b: &Bars, prm: &Params) -> Output {
    let with_vol = n(prm, "with_volume") == 1;
    if with_vol && b.volume.is_none() { return Err("needs volume".into()); }
    let len = b.len();
    let vol = |i: usize| b.volume.as_ref().map_or(0., |v| v[i]);
    let mut out = vec![vec![0.; len]; 2];
    let mut r = 0; // the previous rally or reaction day; bar 0 seeds it (declared)
    for i in 1..len {
        let rally = b.high[i] > b.high[r] && b.low[i] >= b.low[r];
        let reaction = b.high[i] <= b.high[r] && b.low[i] < b.low[r];
        let vol_ok = !with_vol || vol(i) > vol(r);
        if rally && vol_ok { out[0][i] = 1.; }
        if reaction && vol_ok { out[1][i] = 1.; }
        if rally || reaction { r = i; }
    }
    Ok(out)
}

const TD: &[ParamSpec] = &[int("lookback", "Setup comparison lag", 4., 1., 2000.), int("setup", "Setup length", 9., 1., 2000.), int("countdown", "Countdown length", 13., 1., 2000.)];
fn td_sequential(b: &Bars, prm: &Params) -> Output {
    let (lb, su, cd, len, c) = (n(prm, "lookback"), n(prm, "setup"), n(prm, "countdown"), b.len(), &b.close);
    let mut out = vec![vec![0.; len]; 4];
    let (mut bs, mut ss) = (0, 0); // setup counts
    let (mut bc, mut sc): (Option<usize>, Option<usize>) = (None, None); // active countdowns
    for i in 0..len {
        let (pb, ps) = (bs, ss);
        bs = if i >= lb && c[i] < c[i - lb] { (bs + 1).min(su) } else { 0 };
        ss = if i >= lb && c[i] > c[i - lb] { (ss + 1).min(su) } else { 0 };
        // a completed setup starts its countdown (that bar may count) and cancels the opposite one; a repeat setup while counting is ignored
        if bs == su && pb < su { sc = None; bc.get_or_insert(0); }
        if ss == su && ps < su { bc = None; sc.get_or_insert(0); }
        if let Some(x) = bc.as_mut() { if i >= 2 && c[i] <= b.low[i - 2] { *x += 1; } }
        if let Some(x) = sc.as_mut() { if i >= 2 && c[i] >= b.high[i - 2] { *x += 1; } }
        (out[0][i], out[1][i]) = (bs as f64, ss as f64);
        (out[2][i], out[3][i]) = (bc.unwrap_or(0) as f64, sc.unwrap_or(0) as f64);
        if bc == Some(cd) { bc = None; }
        if sc == Some(cd) { sc = None; }
    }
    Ok(out)
}

macro_rules! cdl {
    ($id:expr, $name:expr, $func:expr, $lb:expr, $rule:expr, $run:ident) => {
        def($id, $name, CDL, &[], &["signal"], PN,
            concat!($name, ": TA-Lib ", $func, " (port of ta_func/ta_", $func, ".c): ", $rule, ". Default TA-Lib CandleSettings, averaged over the bars before each tested candle: BodyLong/BodyShort = avg body (10); BodyDoji = 0.1 × avg range (10); ShadowLong = own body; ShadowVeryLong = 2 × own body; ShadowShort = avg (upper + lower)/2 (10); ShadowVeryShort = 0.1 × avg range (10); Near 0.2, Far 0.6, Equal 0.05 × avg range (5); white = close ≥ open; 1 on the completing bar, else 0 (0 before TA-Lib lookback ", $lb, ")"),
            &[0], false, $run)
    };
}

pub const DEFS: &[IndicatorDef] = &[
    cdl!("cdl_advance_block", "Advance Block - Bearish", "CDLADVANCEBLOCK", 12, "three white, rising closes, 2 and 3 open above the prior open and ≤ prior close + Near; 1 long with upper shadow < ShadowShort; weakening: (2 < 1 − Far and 3 < 2 + Near) or 3 < 2 − Far or (3 < 2 < 1 and 3 or 2 upper shadow > ShadowShort) or (3 < 2 and 3 upper shadow > its body); TA-Lib −100", r_advance_block),
    cdl!("cdl_belt_hold_bear", "Belt-Hold - Bearish", "CDLBELTHOLD", 10, "long black body (BodyLong) with upper shadow < ShadowVeryShort; TA-Lib −100", r_belt_hold_bear),
    cdl!("cdl_belt_hold_bull", "Belt-Hold - Bullish", "CDLBELTHOLD", 10, "long white body (BodyLong) with lower shadow < ShadowVeryShort; no trend or gap rule; TA-Lib +100", r_belt_hold_bull),
    cdl!("cdl_breakaway_bear", "Breakaway - Bearish", "CDLBREAKAWAY", 14, "1 long white; 2 white body gapping up; 3 and 4 each with a higher high and higher low; 4 white; 5 black closing below 2's open and above 1's close; TA-Lib −100", r_breakaway_bear),
    cdl!("cdl_breakaway_bull", "Breakaway - Bullish", "CDLBREAKAWAY", 14, "1 long black; 2 black body gapping down; 3 and 4 each with a lower high and lower low; 4 black; 5 white closing above 2's open and below 1's close; TA-Lib +100", r_breakaway_bull),
    cdl!("cdl_closing_marubozu_black", "Closing Marubozu - Black", "CDLCLOSINGMARUBOZU", 10, "long black body (BodyLong) with lower shadow < ShadowVeryShort; TA-Lib −100", r_closing_marubozu_black),
    cdl!("cdl_closing_marubozu_white", "Closing Marubozu - White", "CDLCLOSINGMARUBOZU", 10, "long white body (BodyLong) with upper shadow < ShadowVeryShort; TA-Lib +100", r_closing_marubozu_white),
    cdl!("cdl_concealing_baby_swallow", "Concealing Baby Swallow - Bullish", "CDLCONCEALBABYSWALL", 13, "four black; 1 and 2 with both shadows < ShadowVeryShort; 3 body gapping below 2's with upper shadow > ShadowVeryShort and high above 2's close; 4 with high above and low below 3's; TA-Lib +100", r_concealing_baby_swallow),
    cdl!("cdl_counterattack_bear", "Counterattack (Meeting Lines) - Bearish", "CDLCOUNTERATTACK", 11, "1 long white, 2 long black (BodyLong each), 2's close within ± Equal of 1's close; TA-Lib −100", r_counterattack_bear),
    cdl!("cdl_counterattack_bull", "Counterattack (Meeting Lines) - Bullish", "CDLCOUNTERATTACK", 11, "1 long black, 2 long white (BodyLong each), 2's close within ± Equal of 1's close; TA-Lib +100", r_counterattack_bull),
    cdl!("cdl_downside_gap_three_methods", "Downside Gap Three Methods - Bearish", "CDLXSIDEGAP3METHODS", 2, "1 and 2 black with 2's body gapping below 1's; 3 white opening strictly inside 2's body and closing strictly inside 1's body; TA-Lib −100", r_downside_gap_three_methods),
    cdl!("cdl_high_wave", "High-Wave Candle", "CDLHIGHWAVE", 10, "body < BodyShort with upper and lower shadows each > ShadowVeryLong (2 × body); either colour (TA-Lib ±100)", r_high_wave),
    cdl!("cdl_hikkake_bear", "Hikkake - Bearish", "CDLHIKKAKE", 5, "2 inside 1 (lower high, higher low); 3 with a higher high and higher low than 2 → 1 on bar 3 (TA-Lib −100); a close below 2's low within the next 3 bars → 2 (TA-Lib −200), unless a newer hikkake replaces it", r_hikkake_bear),
    cdl!("cdl_hikkake_bull", "Hikkake - Bullish", "CDLHIKKAKE", 5, "2 inside 1 (lower high, higher low); 3 with a lower high and lower low than 2 → 1 on bar 3 (TA-Lib +100); a close above 2's high within the next 3 bars → 2 (TA-Lib +200), unless a newer hikkake replaces it", r_hikkake_bull),
    cdl!("cdl_hikkake_mod_bear", "Modified Hikkake - Bearish", "CDLHIKKAKEMOD", 10, "2 inside 1, 3 inside 2, 2's close ≥ 2's high − Near; 4 with a higher high and higher low than 3 → 1 (TA-Lib −100); a close below 3's low within the next 3 bars → 2 (TA-Lib −200)", r_hikkake_mod_bear),
    cdl!("cdl_hikkake_mod_bull", "Modified Hikkake - Bullish", "CDLHIKKAKEMOD", 10, "2 inside 1, 3 inside 2, 2's close ≤ 2's low + Near; 4 with a lower high and lower low than 3 → 1 (TA-Lib +100); a close above 3's high within the next 3 bars → 2 (TA-Lib +200)", r_hikkake_mod_bull),
    cdl!("cdl_homing_pigeon", "Homing Pigeon - Bullish", "CDLHOMINGPIGEON", 11, "1 long black (BodyLong); 2 black with body ≤ BodyShort, open below 1's open and close above 1's close; TA-Lib +100", r_homing_pigeon),
    cdl!("cdl_identical_three_crows", "Identical Three Crows - Bearish", "CDLIDENTICAL3CROWS", 12, "three black with lower shadows < ShadowVeryShort and falling closes; 2 and 3 open within ± Equal of the prior close; TA-Lib −100", r_identical_three_crows),
    cdl!("cdl_in_neck", "In-Neck - Bearish", "CDLINNECK", 11, "1 long black; 2 white opening below 1's low and closing between 1's close and 1's close + Equal; TA-Lib −100", r_in_neck),
    cdl!("cdl_ladder_bottom", "Ladder Bottom - Bullish", "CDLLADDERBOTTOM", 14, "1–3 black with falling opens and closes; 4 black with upper shadow > ShadowVeryShort; 5 white opening above 4's open and closing above 4's high; TA-Lib +100", r_ladder_bottom),
    cdl!("cdl_long_legged_doji", "Long-Legged Doji", "CDLLONGLEGGEDDOJI", 10, "body ≤ BodyDoji with the lower or the upper shadow longer than the body (ShadowLong); TA-Lib +100", r_long_legged_doji),
    cdl!("cdl_long_line", "Long Line Candle", "CDLLONGLINE", 10, "body > BodyLong with upper and lower shadows < ShadowShort; either colour (TA-Lib ±100)", r_long_line),
    cdl!("cdl_mat_hold", "Mat Hold - Bullish", "CDLMATHOLD", 14, "1 long white; 2 black with body gapping above 1's; 2–4 bodies < BodyShort; 3 and 4 body bottoms below 1's close but above 1's close − 0.5 × 1's body; body tops falling (3's below 2's open, 4's below 3's); 5 white opening above 4's close and closing above the highest high of 2–4; penetration 0.5; TA-Lib +100", r_mat_hold),
    cdl!("cdl_matching_low", "Matching Low - Bullish", "CDLMATCHINGLOW", 6, "two black, 2's close within ± Equal of 1's close; TA-Lib +100", r_matching_low),
    cdl!("cdl_rickshaw_man", "Rickshaw Man", "CDLRICKSHAWMAN", 10, "body ≤ BodyDoji, both shadows longer than the body, body within ± Near of the range midpoint; TA-Lib +100", r_rickshaw_man),
    cdl!("cdl_separating_lines_bear", "Separating Lines - Bearish", "CDLSEPARATINGLINES", 11, "1 white; 2 long black opening within ± Equal of 1's open with upper shadow < ShadowVeryShort; TA-Lib −100", r_separating_lines_bear),
    cdl!("cdl_separating_lines_bull", "Separating Lines - Bullish", "CDLSEPARATINGLINES", 11, "1 black; 2 long white opening within ± Equal of 1's open with lower shadow < ShadowVeryShort; TA-Lib +100", r_separating_lines_bull),
    cdl!("cdl_short_line", "Short Line Candle", "CDLSHORTLINE", 10, "body < BodyShort with upper and lower shadows < ShadowShort; either colour (TA-Lib ±100)", r_short_line),
    cdl!("cdl_side_by_side_white_down", "Down-Gap Side-by-Side White Lines - Bearish", "CDLGAPSIDESIDEWHITE", 7, "2 and 3 white, both bodies gapping below 1's body; 3's body within ± Near of 2's body and 3's open within ± Equal of 2's open; TA-Lib −100", r_side_by_side_white_down),
    cdl!("cdl_side_by_side_white_up", "Up-Gap Side-by-Side White Lines - Bullish", "CDLGAPSIDESIDEWHITE", 7, "2 and 3 white, both bodies gapping above 1's body; 3's body within ± Near of 2's body and 3's open within ± Equal of 2's open; TA-Lib +100", r_side_by_side_white_up),
    cdl!("cdl_stalled_pattern", "Stalled Pattern (Deliberation) - Bearish", "CDLSTALLEDPATTERN", 12, "three white, rising closes; 1 and 2 long, 2's upper shadow < ShadowVeryShort, 2 opens above 1's open and ≤ 1's close + Near; 3 body < BodyShort opening ≥ 2's close − 3's body − Near; TA-Lib −100", r_stalled_pattern),
    cdl!("cdl_stick_sandwich", "Stick Sandwich - Bullish", "CDLSTICKSANDWICH", 7, "1 black; 2 white with low above 1's close; 3 black closing within ± Equal of 1's close; TA-Lib +100", r_stick_sandwich),
    cdl!("cdl_takuri", "Takuri - Bullish", "CDLTAKURI", 10, "body ≤ BodyDoji, upper shadow < ShadowVeryShort, lower shadow > ShadowVeryLong (2 × body); TA-Lib +100", r_takuri),
    cdl!("cdl_three_inside_down", "Three Inside Down - Bearish", "CDL3INSIDE", 12, "1 long white; 2 body ≤ BodyShort strictly inside 1's body; 3 black closing below 1's open; TA-Lib −100", r_three_inside_down),
    cdl!("cdl_three_inside_up", "Three Inside Up - Bullish", "CDL3INSIDE", 12, "1 long black; 2 body ≤ BodyShort strictly inside 1's body; 3 white closing above 1's open; TA-Lib +100", r_three_inside_up),
    cdl!("cdl_three_line_strike_bear", "Three-Line Strike - Bearish", "CDL3LINESTRIKE", 8, "1–3 black with falling closes, 2 and 3 opening within the prior body ± Near; 4 white opening below 3's close and closing above 1's open; TA-Lib −100 (signed by the colour of 1–3)", r_three_line_strike_bear),
    cdl!("cdl_three_line_strike_bull", "Three-Line Strike - Bullish", "CDL3LINESTRIKE", 8, "1–3 white with rising closes, 2 and 3 opening within the prior body ± Near; 4 black opening above 3's close and closing below 1's open; TA-Lib +100 (signed by the colour of 1–3)", r_three_line_strike_bull),
    cdl!("cdl_three_outside_down", "Three Outside Down - Bearish", "CDL3OUTSIDE", 3, "1 white; 2 black opening above 1's close and closing below 1's open; 3 closes below 2's close; TA-Lib −100", r_three_outside_down),
    cdl!("cdl_three_outside_up", "Three Outside Up - Bullish", "CDL3OUTSIDE", 3, "1 black; 2 white opening below 1's close and closing above 1's open; 3 closes above 2's close; TA-Lib +100", r_three_outside_up),
    cdl!("cdl_three_stars_south", "Three Stars in the South - Bullish", "CDL3STARSINSOUTH", 12, "three black; 1 long with lower shadow longer than its body; 2 smaller body, opening above 1's close and ≤ 1's high, low below 1's close and ≥ 1's low, lower shadow > ShadowVeryShort; 3 body < BodyShort, both shadows < ShadowVeryShort, inside 2's high-low range; TA-Lib +100", r_three_stars_south),
    cdl!("cdl_thrusting", "Thrusting - Bearish", "CDLTHRUSTING", 11, "1 long black; 2 white opening below 1's low and closing above 1's close + Equal but at or below 1's body midpoint; TA-Lib −100", r_thrusting),
    cdl!("cdl_two_crows", "Two Crows - Bearish", "CDL2CROWS", 12, "1 long white; 2 black with body gapping above 1's; 3 black opening inside 2's body and closing inside 1's body; TA-Lib −100", r_two_crows),
    cdl!("cdl_unique_three_river", "Unique Three River Bottom - Bullish", "CDLUNIQUE3RIVER", 12, "1 long black; 2 black with close above 1's close, open ≤ 1's open and a lower low; 3 white with body < BodyShort opening above 2's low; TA-Lib +100", r_unique_three_river),
    cdl!("cdl_upside_gap_three_methods", "Upside Gap Three Methods - Bullish", "CDLXSIDEGAP3METHODS", 2, "1 and 2 white with 2's body gapping above 1's; 3 black opening strictly inside 2's body and closing strictly inside 1's body; TA-Lib +100", r_upside_gap_three_methods),
    cdl!("cdl_upside_gap_two_crows", "Upside Gap Two Crows - Bearish", "CDLUPSIDEGAP2CROWS", 12, "1 long white; 2 black, body ≤ BodyShort, gapping above 1's body; 3 black opening above 2's open, closing below 2's close and above 1's close; TA-Lib −100", r_upside_gap_two_crows),
    def("cdl_one_black_crow", "One Black Crow - Bearish", CDL, TREND, &["signal"], PN, "One Black Crow: thinkorswim OneBlackCrow (TD Ameritrade 2019 thinkScript): bar 1 IsLongWhite(20) (close > open, body longer than each shadow and above the average body of the 20 preceding bars) in an uptrend; bar 2 IsLongBlack(20) with open < 1's close, open > 1's open and close < 1's open; declared adaptation: the uptrend on bar 1 is our TradingView trend rule (thinkorswim's IsAscending(open, 3) slope test is not published in full); 1 on bar 2, else 0", &[0], false, one_black_crow),
    def("cdl_one_white_soldier", "One White Soldier - Bullish", CDL, TREND, &["signal"], PN, "One White Soldier: thinkorswim OneWhiteSoldier (TD Ameritrade 2019 thinkScript): bar 1 IsLongBlack(20) (close < open, body longer than each shadow and above the average body of the 20 preceding bars) in a downtrend; bar 2 IsLongWhite(20) with open > 1's close, open < 1's open and close > 1's open; declared adaptation: the downtrend on bar 1 is our TradingView trend rule (thinkorswim's IsDescending(open, 3) slope test is not published in full); 1 on bar 2, else 0", &[0], false, one_white_soldier),
    def("darvas", "Darvas Box", PAT, DARVAS, &["top", "bottom", "breakout"], PR, "Kivanc Ozbilgic DARVAS BOX (TradingView Pine v3): ll = lowest(low, b); k1/k2/k3 = highest(high, b / b−1 / b−2); nh = valuewhen(high > k1[1], high); top = valuewhen(barssince(high > k1[1]) == b−2 and k3 < k2, nh); bottom = valuewhen(same condition, ll); NaN before the first box; breakout (lab signal) = +1 when close crosses over top, −1 when it crosses under bottom (Pine crossover semantics), else 0", &[usize::MAX, usize::MAX, 0], true, darvas),
    def("fvg", "Fair Value Gap", PAT, FVG, &["bull", "bear", "bull_top", "bull_bottom", "bear_top", "bear_bottom"], PR, "Three-bar fair value gap (NinjaTrader Fair Value Gap; pandas-ta smc imbalance): bull at bar t if low − high[2] > 0 and ≥ min_gap → zone [high[2], low]; bear if low[2] − high > 0 and ≥ min_gap → zone [high, low[2]]; bull/bear = 1 on that bar, else 0; a zone is filled when a later bar closes through it (bull: close < bottom; bear: close > top; NinjaTrader's published rule); the newest max_zones zones per side are tracked and *_top/*_bottom show the newest unfilled one (NaN if none)", &[0, 0, usize::MAX, usize::MAX, usize::MAX, usize::MAX], true, fvg),
    def("gaps", "Gaps (with fill tracking)", PAT, GAPS, &["gap_up", "gap_down", "filled_up", "filled_down", "open_up_top", "open_up_bottom", "open_down_top", "open_down_bottom"], PR, "TradingView Gaps: up gap low > high[1] (zone [high[1], low]), down gap high < low[1] (zone [high, low[1]]), kept if its height ≥ min_dev% × sma(high − low, range_length) (declared: the average ends on the bar before the gap); a later bar entering a gap closes it (partial = 1: only the covered part closes, the rest stays open); gap_* = 1 on the gap bar, filled_* = 1 on the bar a gap fully closes; open_* = zone of the newest open gap (NaN if none)", &[0, 0, 0, 0, usize::MAX, usize::MAX, usize::MAX, usize::MAX], true, gaps),
    def("island_reversal", "Island Reversal", PAT, ISLAND, &["top", "bottom"], PN, "Island reversal (Sierra Chart; TradeStation/MultiCharts): top = 1 on bar t if high[t] < low[t−1] (gap down) and, for some s with t − max_bars ≤ s < t, every bar s..t−1 has its low above high[s−1] (the gap up into the island) and above high[t]; bottom mirrors (gap down into the island; island highs below low[s−1] and low[t]); else 0", &[0, 0], false, island_reversal),
    def("rally_reaction", "Rally / Reaction Days", PAT, RALLY, &["rally", "reaction"], PN, "MetaStock Rally()/Reaction() as corrected by Equis: rally = high > the high of the previous rally-or-reaction day and low ≥ its low; reaction = high ≤ its high and low < its low; with_volume = 1 (RallyWithVol/ReactionWithVol) also needs volume > that day's volume; bar 0 seeds the reference (declared); 1/0", &[0, 0], false, rally_reaction),
    def("td_sequential", "TD Sequential Setup and Countdown", PAT, TD, &["buy_setup", "sell_setup", "buy_countdown", "sell_countdown"], PN, "DeMark TD Sequential (Sierra Chart, thinkorswim, pandas-ta exhc): buy setup count = consecutive bars with close < close[lookback], 0 otherwise, capped at setup; sell mirrors with close > close[lookback]; when a setup reaches setup its countdown starts (that bar may count) and the opposite countdown is cancelled; buy countdown counts bars with close ≤ low[2], sell with close ≥ high[2], and shows its count until it reaches countdown (then 0); a repeat setup during a countdown is ignored (declared); perfection and TDST are not built", &[0, 0, 0, 0], false, td_sequential),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("cdl_advance_block", "TA-Lib: CDLADVANCEBLOCK; thinkorswim: AdvanceBlock; TradeStation: Advance Block", ""),
    ("cdl_belt_hold_bear", "TA-Lib: CDLBELTHOLD; thinkorswim: BeltHold; NinjaTrader 8: Belt Hold", ""),
    ("cdl_belt_hold_bull", "TA-Lib: CDLBELTHOLD; thinkorswim: BeltHold; NinjaTrader 8: Belt Hold", ""),
    ("cdl_breakaway_bear", "TA-Lib: CDLBREAKAWAY; thinkorswim: Breakaway", ""),
    ("cdl_breakaway_bull", "TA-Lib: CDLBREAKAWAY; thinkorswim: Breakaway", ""),
    ("cdl_closing_marubozu_black", "TA-Lib: CDLCLOSINGMARUBOZU", ""),
    ("cdl_closing_marubozu_white", "TA-Lib: CDLCLOSINGMARUBOZU", ""),
    ("cdl_concealing_baby_swallow", "TA-Lib: CDLCONCEALBABYSWALL; thinkorswim: ConcealingBabySwallow", ""),
    ("cdl_counterattack_bear", "TA-Lib: CDLCOUNTERATTACK; thinkorswim: MeetingLines", ""),
    ("cdl_counterattack_bull", "TA-Lib: CDLCOUNTERATTACK; thinkorswim: MeetingLines", ""),
    ("cdl_downside_gap_three_methods", "TA-Lib: CDLXSIDEGAP3METHODS; thinkorswim: DownsideGapThreeMethods", ""),
    ("cdl_high_wave", "TA-Lib: CDLHIGHWAVE", ""),
    ("cdl_hikkake_bear", "TA-Lib: CDLHIKKAKE", ""),
    ("cdl_hikkake_bull", "TA-Lib: CDLHIKKAKE", ""),
    ("cdl_hikkake_mod_bear", "TA-Lib: CDLHIKKAKEMOD", ""),
    ("cdl_hikkake_mod_bull", "TA-Lib: CDLHIKKAKEMOD", ""),
    ("cdl_homing_pigeon", "TA-Lib: CDLHOMINGPIGEON; thinkorswim: HomingPigeon", ""),
    ("cdl_identical_three_crows", "TA-Lib: CDLIDENTICAL3CROWS; thinkorswim: IdenticalThreeCrows", ""),
    ("cdl_in_neck", "TA-Lib: CDLINNECK; thinkorswim: InNeck", ""),
    ("cdl_ladder_bottom", "TA-Lib: CDLLADDERBOTTOM", ""),
    ("cdl_long_legged_doji", "TA-Lib: CDLLONGLEGGEDDOJI; thinkorswim: LongLeggedDoji; MetaStock: Long-Legged Doji", ""),
    ("cdl_long_line", "TA-Lib: CDLLONGLINE; MetaStock: Big Black/White Candle", ""),
    ("cdl_mat_hold", "TA-Lib: CDLMATHOLD; thinkorswim: MatHold", ""),
    ("cdl_matching_low", "TA-Lib: CDLMATCHINGLOW; thinkorswim: MatchingLow", ""),
    ("cdl_one_black_crow", "thinkorswim: OneBlackCrow", ""),
    ("cdl_one_white_soldier", "thinkorswim: OneWhiteSoldier", ""),
    ("cdl_rickshaw_man", "TA-Lib: CDLRICKSHAWMAN", ""),
    ("cdl_separating_lines_bear", "TA-Lib: CDLSEPARATINGLINES; thinkorswim: SeparatingLines; MetaStock: Separating Lines", ""),
    ("cdl_separating_lines_bull", "TA-Lib: CDLSEPARATINGLINES; thinkorswim: SeparatingLines; MetaStock: Separating Lines", ""),
    ("cdl_short_line", "TA-Lib: CDLSHORTLINE", ""),
    ("cdl_side_by_side_white_down", "TA-Lib: CDLGAPSIDESIDEWHITE; thinkorswim: SideBySideWhiteLines", ""),
    ("cdl_side_by_side_white_up", "TA-Lib: CDLGAPSIDESIDEWHITE; thinkorswim: SideBySideWhiteLines", ""),
    ("cdl_stalled_pattern", "TA-Lib: CDLSTALLEDPATTERN; thinkorswim: StalledPattern", ""),
    ("cdl_stick_sandwich", "TA-Lib: CDLSTICKSANDWICH; thinkorswim: StickSandwich; NinjaTrader 8: Stick Sandwich", ""),
    ("cdl_takuri", "TA-Lib: CDLTAKURI", ""),
    ("cdl_three_inside_down", "TA-Lib: CDL3INSIDE; thinkorswim: ThreeInsideDown; TA4J: ThreeInsideDown", ""),
    ("cdl_three_inside_up", "TA-Lib: CDL3INSIDE; thinkorswim: ThreeInsideUp; TA4J: ThreeInsideUp", ""),
    ("cdl_three_line_strike_bear", "TA-Lib: CDL3LINESTRIKE; thinkorswim: ThreeLineStrike", ""),
    ("cdl_three_line_strike_bull", "TA-Lib: CDL3LINESTRIKE; thinkorswim: ThreeLineStrike", ""),
    ("cdl_three_outside_down", "TA-Lib: CDL3OUTSIDE; thinkorswim: ThreeOutsideDown", ""),
    ("cdl_three_outside_up", "TA-Lib: CDL3OUTSIDE; thinkorswim: ThreeOutsideUp", ""),
    ("cdl_three_stars_south", "TA-Lib: CDL3STARSINSOUTH; thinkorswim: ThreeStarsInTheSouth; TradeStation: Three Stars in the South", ""),
    ("cdl_thrusting", "TA-Lib: CDLTHRUSTING; thinkorswim: Thrusting", ""),
    ("cdl_two_crows", "TA-Lib: CDL2CROWS; thinkorswim: TwoCrows", ""),
    ("cdl_unique_three_river", "TA-Lib: CDLUNIQUE3RIVER; thinkorswim: UniqueThreeRiverBottom", ""),
    ("cdl_upside_gap_three_methods", "TA-Lib: CDLXSIDEGAP3METHODS; thinkorswim: UpsideGapThreeMethods", ""),
    ("cdl_upside_gap_two_crows", "TA-Lib: CDLUPSIDEGAP2CROWS; thinkorswim: UpsideGapTwoCrows; NinjaTrader 8: Upside Gap Two Crows", ""),
    ("darvas", "NinjaTrader 8: Darvas; thinkorswim: DarvasBox; MetaStock: Darvas Box; Zerodha Kite (ChartIQ): Darvas Box; Dhan: Darvas Box; TradingView: DARVAS BOX (KivancOzbilgic)", ""),
    ("fvg", "NinjaTrader 8: Fair Value Gap; pandas-ta: smc", ""),
    ("gaps", "TradingView: Gaps", ""),
    ("island_reversal", "Sierra Chart: Island Reversal; TradeStation: Island Reversal; MultiCharts: Island Reversal", ""),
    ("rally_reaction", "MetaStock: Rally/RallyWithVol/Reaction/ReactionWithVol", ""),
    ("td_sequential", "Sierra Chart: Trading the Nines; thinkorswim: SequenceCounter; pandas-ta: exhc", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, compiles, from_ohlc, go};
    type R = (f64, f64, f64, f64);
    const N: f64 = f64::NAN;
    /// The LCG bar generator mirrored in the python TA-Lib check (integer moves × tick, so both sides build identical bars).
    fn lcg_bars(n: usize, seed: u64, tick: f64, ob: u64, cb: u64, wb: u64) -> crate::lab::bars::Bars {
        let mut s = seed;
        let mut r = |m: u64| { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (s >> 33) % m };
        let (mut c, mut rows) = (100.0f64, Vec::with_capacity(n));
        for _ in 0..n {
            let o = c + (r(2 * ob + 1) as f64 - ob as f64) * tick;
            let cl = o + (r(2 * cb + 1) as f64 - cb as f64) * tick;
            let h = o.max(cl) + r(wb) as f64 * tick;
            let l = o.min(cl) - r(wb) as f64 * tick;
            rows.push((o, h, l, cl));
            c = if cl < 30. { cl + 50. } else { cl };
        }
        from_ohlc(&rows)
    }
    /// (fires, sum of fire indices, sum of values) from python TA-Lib 0.8.2 (C 0.8.1) on lcg_bars(20000, 1, 1, 3, 4, 3) and lcg_bars(20000, 2, 0.25, 2, 8, 4).
    const TALIB: &[(&str, [(usize, usize, usize); 2])] = &[
        ("cdl_advance_block", [(8, 79505, 8), (74, 725299, 74)]),
        ("cdl_belt_hold_bear", [(1605, 16086032, 1605), (1250, 12550793, 1250)]),
        ("cdl_belt_hold_bull", [(1668, 16701565, 1668), (1220, 12196768, 1220)]),
        ("cdl_closing_marubozu_black", [(1621, 16627931, 1621), (1293, 12949097, 1293)]),
        ("cdl_closing_marubozu_white", [(1616, 15813106, 1616), (1250, 12627549, 1250)]),
        ("cdl_counterattack_bear", [(62, 589882, 62), (0, 0, 0)]),
        ("cdl_counterattack_bull", [(93, 926159, 93), (0, 0, 0)]),
        ("cdl_downside_gap_three_methods", [(19, 222533, 19), (59, 591697, 59)]),
        ("cdl_high_wave", [(942, 9761321, 942), (743, 7326756, 743)]),
        ("cdl_hikkake_bear", [(499, 5012222, 600), (700, 7147253, 837)]),
        ("cdl_hikkake_bull", [(517, 5185371, 617), (639, 6291471, 766)]),
        ("cdl_hikkake_mod_bear", [(0, 0, 0), (4, 28824, 4)]),
        ("cdl_hikkake_mod_bull", [(0, 0, 0), (2, 21251, 2)]),
        ("cdl_homing_pigeon", [(143, 1325898, 143), (49, 538778, 49)]),
        ("cdl_identical_three_crows", [(0, 0, 0), (3, 26067, 3)]),
        ("cdl_in_neck", [(158, 1561758, 158), (49, 498983, 49)]),
        ("cdl_ladder_bottom", [(15, 144748, 15), (7, 54647, 7)]),
        ("cdl_long_legged_doji", [(1907, 19430197, 1907), (1058, 10708902, 1058)]),
        ("cdl_long_line", [(2585, 25790386, 2585), (2522, 25242902, 2522)]),
        ("cdl_matching_low", [(400, 3892225, 400), (233, 2504113, 233)]),
        ("cdl_rickshaw_man", [(938, 9702853, 938), (607, 5968631, 607)]),
        ("cdl_separating_lines_bear", [(113, 1180282, 113), (45, 480923, 45)]),
        ("cdl_separating_lines_bull", [(92, 1015698, 92), (32, 366006, 32)]),
        ("cdl_short_line", [(2607, 26037360, 2607), (2535, 25726957, 2535)]),
        ("cdl_side_by_side_white_down", [(27, 275448, 27), (6, 63524, 6)]),
        ("cdl_side_by_side_white_up", [(49, 494770, 49), (31, 278146, 31)]),
        ("cdl_stalled_pattern", [(37, 331251, 37), (53, 503689, 53)]),
        ("cdl_stick_sandwich", [(84, 810320, 84), (35, 395208, 35)]),
        ("cdl_takuri", [(513, 5160426, 513), (211, 2213536, 211)]),
        ("cdl_three_inside_down", [(96, 905213, 96), (143, 1452226, 143)]),
        ("cdl_three_inside_up", [(119, 1212343, 119), (147, 1419951, 147)]),
        ("cdl_three_line_strike_bear", [(0, 0, 0), (2, 6335, 2)]),
        ("cdl_three_line_strike_bull", [(0, 0, 0), (2, 25515, 2)]),
        ("cdl_three_outside_down", [(175, 1827183, 175), (317, 3167999, 317)]),
        ("cdl_three_outside_up", [(58, 562196, 58), (244, 2431044, 244)]),
        ("cdl_thrusting", [(168, 1784901, 168), (139, 1400750, 139)]),
        ("cdl_unique_three_river", [(7, 80694, 7), (3, 26351, 3)]),
        ("cdl_upside_gap_three_methods", [(18, 211387, 18), (57, 606522, 57)]),
        ("cdl_upside_gap_two_crows", [(3, 38535, 3), (0, 0, 0)]),
    ];
    #[test] fn talib_candles_match_talib() {
        let sets = [lcg_bars(20000, 1, 1., 3, 4, 3), lcg_bars(20000, 2, 0.25, 2, 8, 4)];
        for (id, want) in TALIB {
            for (b, w) in sets.iter().zip(want) {
                let s = go(id, b, &[]).remove(0);
                let got = s.iter().enumerate().filter(|(_, v)| **v != 0.).fold((0, 0, 0), |a, (i, v)| (a.0 + 1, a.1 + i, a.2 + *v as usize));
                assert_eq!(got, *w, "{id}");
            }
        }
    }
    const NEUTRAL: R = (10., 11.5, 9.5, 11.); // body 1, range 2
    fn after_neutral(n: usize, rows: &[R]) -> crate::lab::bars::Bars { let mut all = vec![NEUTRAL; n]; all.extend_from_slice(rows); from_ohlc(&all) }
    /// Rare patterns: hand-built after 14 neutral bars; python TA-Lib reports each exactly once, on the last bar.
    const HAND: &[(&str, &[R])] = &[
        ("cdl_breakaway_bull", &[(13., 13.1, 10.9, 11.), (10.5, 10.6, 9.9, 10.), (10., 10.4, 9.5, 9.7), (9.6, 9.9, 9.0, 9.2), (9.3, 10.9, 9.2, 10.8)]),
        ("cdl_breakaway_bear", &[(10., 12.1, 9.9, 12.), (12.5, 13.1, 12.4, 13.), (13., 13.5, 12.6, 13.2), (13.3, 14., 13.2, 13.8), (13.7, 13.8, 12.1, 12.2)]),
        ("cdl_concealing_baby_swallow", &[(13., 13., 11., 11.), (11., 11., 9., 9.), (8.5, 9.3, 8., 8.2), (9.4, 9.5, 7.5, 7.6)]),
        ("cdl_mat_hold", &[(10., 12.1, 9.9, 12.), (12.8, 13., 12.4, 12.5), (12.3, 12.4, 11.6, 11.8), (12.1, 12.2, 11.4, 11.5), (11.6, 13.3, 11.5, 13.2)]),
        ("cdl_three_stars_south", &[(12., 12.1, 7.9, 10.), (11., 11.1, 8.5, 9.5), (9.5, 9.55, 8.9, 8.95)]),
        ("cdl_two_crows", &[(10., 12.1, 9.9, 12.), (13., 13.2, 12.4, 12.5), (12.8, 12.9, 10.9, 11.)]),
    ];
    fn once_on_last(s: &[f64]) -> bool { s.iter().enumerate().all(|(i, v)| *v == if i + 1 == s.len() { 1. } else { 0. }) }
    #[test] fn rare_patterns_fire_once_on_the_last_bar() {
        for (id, rows) in HAND { assert!(once_on_last(&go(id, &after_neutral(14, rows), &[])[0]), "{id}"); }
        // the mirror ids stay silent on the same bars
        assert!(go("cdl_breakaway_bear", &after_neutral(14, HAND[0].1), &[])[0].iter().all(|v| *v == 0.));
    }
    #[test] fn nothing_fires_before_the_talib_lookback() {
        // a mat hold ending on bar 13 (lookback 14) is not reported, as TA-Lib reports nothing there
        assert!(go("cdl_mat_hold", &after_neutral(9, HAND[3].1), &[])[0].iter().all(|v| *v == 0.));
    }
    #[test] fn hikkake_reports_two_on_confirmation() {
        // 1 = inside bar's mother, 2 inside, 3 lower high and lower low → 1; close above 2's high next bar → 2
        let b = after_neutral(6, &[(10., 12., 8., 11.), (10.5, 11.5, 9., 10.), (9.5, 11., 8.5, 9.), (9.5, 12., 9.4, 11.8)]);
        let s = go("cdl_hikkake_bull", &b, &[]).remove(0);
        assert_eq!(&s[s.len() - 2..], &[1., 2.]);
        assert!(go("cdl_hikkake_bear", &b, &[])[0].iter().all(|v| *v == 0.));
    }
    #[test] fn one_black_crow_and_one_white_soldier() {
        let crow = after_neutral(21, &[(10., 12.1, 9.9, 12.), (11.8, 11.9, 9.3, 9.4)]);
        assert!(once_on_last(&go("cdl_one_black_crow", &crow, &[("trend", 2.)])[0]));
        assert_eq!(*go("cdl_one_black_crow", &crow, &[])[0].last().unwrap(), 0.); // default trend rule: no SMA50 yet
        let soldier = after_neutral(21, &[(12., 12.1, 9.9, 10.), (10.2, 12.6, 10.1, 12.5)]);
        assert!(once_on_last(&go("cdl_one_white_soldier", &soldier, &[("trend", 2.)])[0]));
        // the second candle must close beyond the first one's open
        assert_eq!(*go("cdl_one_white_soldier", &after_neutral(21, &[(12., 12.1, 9.9, 10.), (10.2, 11.95, 10.1, 11.9)]), &[("trend", 2.)])[0].last().unwrap(), 0.);
    }
    fn hl(h: &[f64]) -> crate::lab::bars::Bars { from_ohlc(&h.iter().map(|&x| (x - 0.5, x, x - 1., x - 0.5)).collect::<Vec<_>>()) }
    #[test] fn darvas_box_confirms_b_minus_2_bars_after_the_new_high() {
        let mut b = hl(&[10., 10., 10., 10., 10., 12., 11., 11., 11.]);
        b.open.push(11.); b.high.push(12.6); b.low.push(10.9); b.close.push(12.5); b.date.push("2024-01-10".into()); b.end.push(9);
        let o = go("darvas", &b, &[]);
        assert_series(&o[0], &[N, N, N, N, N, N, N, N, 12., 12.]);
        assert_series(&o[1], &[N, N, N, N, N, N, N, N, 9., 9.]);
        assert_series(&o[2], &[0., 0., 0., 0., 0., 0., 0., 0., 0., 1.]);
    }
    #[test] fn fair_value_gap_zone_until_closed_through() {
        let b = from_ohlc(&[(9.5, 10., 9., 9.5), (10.5, 11.5, 10., 11.), (11.2, 12., 11., 11.8), (11., 11.2, 10.2, 10.5), (10.4, 10.6, 9.5, 9.9)]);
        let o = go("fvg", &b, &[]);
        assert_series(&o[0], &[0., 0., 1., 0., 0.]);
        assert_series(&o[2], &[N, N, 11., 11., N]); // bull_top
        assert_series(&o[3], &[N, N, 10., 10., N]); // bull_bottom; close 9.9 < 10 fills it
        assert_series(&go("fvg", &b, &[("min_gap", 1.5)])[0], &[0.; 5]);
    }
    #[test] fn gaps_kept_against_average_range_and_filled_on_entry() {
        let rows = [(10., 12., 8., 11.), (11., 13., 9., 12.), (15.5, 16., 15., 15.8), (15., 15.5, 14.5, 15.), (13.5, 14., 12.9, 13.)];
        let o = go("gaps", &from_ohlc(&rows), &[("range_length", 2.)]); // average range 4, so 30% is 1.2 and the gap of 2 is kept
        assert_series(&o[0], &[0., 0., 1., 0., 0.]);
        assert_series(&o[2], &[0., 0., 0., 1., 0.]);
        assert_series(&o[4], &[N, N, 15., N, N]);
        let p = go("gaps", &from_ohlc(&rows), &[("range_length", 2.), ("partial", 1.)]);
        assert_series(&p[4], &[N, N, 15., 14.5, N]); // partial: the covered part closes
        assert_series(&p[5], &[N, N, 13., 13., N]);
        assert_series(&p[2], &[0., 0., 0., 0., 1.]);
        let small = [(10., 12., 8., 11.), (11., 13., 9., 12.), (14.5, 15., 14., 14.8)]; // gap of 1 < 1.2
        assert_series(&go("gaps", &from_ohlc(&small), &[("range_length", 2.)])[0], &[0., 0., 0.]);
    }
    #[test] fn island_top_on_the_gap_down_bar() {
        let b = from_ohlc(&[(10., 11., 9., 10.5), (12.5, 13., 12., 12.8), (12.8, 13.2, 12.2, 13.), (11., 11.5, 10.5, 11.)]);
        assert_series(&go("island_reversal", &b, &[])[0], &[0., 0., 0., 1.]);
        assert_series(&go("island_reversal", &b, &[("max_bars", 1.)])[0], &[0., 0., 0., 0.]); // the island is 2 bars long
        let m = from_ohlc(&[(10., 11., 9., 10.5), (8., 8.5, 7.5, 8.), (7.8, 8.4, 7.6, 8.), (9.5, 10., 9., 9.8)]);
        assert_series(&go("island_reversal", &m, &[])[1], &[0., 0., 0., 1.]);
    }
    #[test] fn rally_and_reaction_compare_with_the_last_rally_or_reaction_day() {
        let mut b = from_ohlc(&[(10., 11., 9., 10.), (10.5, 11.5, 9.5, 11.), (11., 11.4, 9., 10.), (10., 11.4, 9., 10.), (10., 11.6, 9., 11.)]);
        let o = go("rally_reaction", &b, &[]);
        assert_series(&o[0], &[0., 1., 0., 0., 1.]); // bar 4 is compared with reaction day 2, not rally day 1
        assert_series(&o[1], &[0., 0., 1., 0., 0.]);
        assert!(crate::lab::catalogue::run(&crate::lab::catalogue::get("rally_reaction").unwrap(), &b, &[("with_volume".to_string(), 1.)].into_iter().collect()).is_err());
        b.volume = Some(vec![100., 90., 120., 50., 130.]);
        let v = go("rally_reaction", &b, &[("with_volume", 1.)]);
        assert_series(&v[0], &[0., 0., 0., 0., 1.]); // 90 < 100 fails; 130 > 120
        assert_series(&v[1], &[0., 0., 1., 0., 0.]);
    }
    #[test] fn td_setup_and_countdown() {
        let closes = [20., 19., 18., 17., 16., 15., 14., 13., 12., 11., 10., 9., 8., 7., 6.];
        let b = from_ohlc(&closes.iter().map(|&c| (c, c + 0.5, c - 0.5, c)).collect::<Vec<_>>());
        let o = go("td_sequential", &b, &[("countdown", 2.)]);
        assert_series(&o[0], &[0., 0., 0., 0., 1., 2., 3., 4., 5., 6., 7., 8., 9., 9., 9.]);
        assert_series(&o[2], &[0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 1., 2., 0.]);
        assert!(o[1].iter().chain(&o[3]).all(|v| *v == 0.));
    }
    #[test] fn rules_reach_new_patterns() {
        for (id, out) in [("cdl_hikkake_bull", "signal"), ("cdl_one_black_crow", "signal"), ("darvas", "breakout"), ("gaps", "open_up_top"), ("td_sequential", "buy_setup")] { compiles(id, out).unwrap(); }
    }
}
