//! Street Smarts: High Probability Short-Term Trading Strategies (Linda Bradford Raschke & Laurence A. Connors, 1995).
//! Book id "ss". Rules are transcribed from the book (library/ss/extract-*.md). The book gives entries exactly but most
//! exits only as "trail a stop"; every exit that is not in the chapter is a parameter and is tagged
//! "[Not from the book]" or "[From elsewhere in the book]" in the rules text.
//!
//! Daily-bar conventions (no lookahead beyond the bar being filled):
//!   * a stop entry fills at the level, or at the open if the bar opens through it;
//!   * a stop placed AFTER an intraday break (Turtle Soup, 80-20) fills at the level when the bar trades back to it.
//!     A bar that OPENED beyond the level is ambiguous on daily data (did the break come before the return?):
//!     param `fill_rule` 0 (default) = skip such bars (only bars that opened inside the level count; unbiased),
//!     1 = count them all as fills at the level (pessimistic bound), 2 = count them only if the close is back
//!     beyond the level (the first-round rule; it keeps the winners and drops the losers, so it is biased upward);
//!   * a protective stop on the entry bar is hit only if the bar trades strictly through it (a stop at "today's low"
//!     was set by the low itself); on later bars a gap through the stop fills at the open;
//!   * when two stop entries could fill on one bar, the one nearer the open fills first.
use crate::connors::*;
use crate::Row;

pub const BOOK: &str = "ss";

/// Daily rules admitted after the 2026-10-21 source review (connors/ss-admission/SOURCE-CONTRACT.md) and an
/// independent chronological Python ledger check. Declared application of futures/stock studies to actual
/// index/ETF prices (and actual stocks for `STOCK_SHOWN`); no futures dollars or book performance claimed.
/// Held (not listed): Turtle Soup, 80-20's, Momentum Pinball, ADX no-gap variant, chart patterns, news,
/// Smart Money / TICK / Arms indicators and the gate-withheld Fresh ADX studies.
pub const REVIEWED_DAILY: &[&str] = &[
    "ss_turtle_soup_plus_one", "ss_2_period_roc", "ss_anti", "ss_anti_3_10", "ss_anti_trigger", "ss_holy_grail",
    "ss_adx_gapper", "ss_whiplash", "ss_three_day_unfilled_gap", "ss_id_nr4", "ss_hist_vol_crabel",
    "ss_ad_market_buster", "ss_ad_market_buster_nse",
    "ss_study_close_upper_90", "ss_study_close_upper_80", "ss_study_close_lower_10", "ss_study_close_lower_20",
    "ss_study_wr7_high_close", "ss_study_wr7_low_close", "ss_study_rocrsi_buy", "ss_study_rocrsi_sell",
    "ss_study_fresh_roc_buy", "ss_study_fresh_roc_sell", "ss_study_oops_buy", "ss_study_oops_sell",
    "ss_study_oops_adx_gapper_buy", "ss_study_oops_adx_gapper_sell", "ss_study_gap_failure_buy",
    "ss_study_gap_failure_sell", "ss_study_channel_2period_sar",
];
pub fn reviewed_daily(id: &str) -> bool { REVIEWED_DAILY.contains(&id) }
/// Methods the book shows on individual stocks (MU p.64, "all equities" p.77, Motorola p.99-101): actual stocks
/// plus any actual index/ETF (user rule 21 Oct 2026). The others are futures studies: index timing only.
pub const STOCK_SHOWN: &[&str] = &["ss_2_period_roc", "ss_anti", "ss_anti_3_10", "ss_anti_trigger", "ss_three_day_unfilled_gap"];
/// Non-stock symbols admitted for the futures studies (1 tick = % of price, so scale-free). The A/D Market Buster
/// times the S&P on NYSE breadth (SPX/SPY) and NIFTY 50 on NSE breadth.
pub fn admitted_symbol(id: &str, ticker: &str) -> bool {
    match id {
        "ss_ad_market_buster" => matches!(ticker, "SPX" | "SPY"),
        "ss_ad_market_buster_nse" => ticker == "NIFTY 50",
        _ if STOCK_SHOWN.contains(&id) => true,
        _ => reviewed_daily(id) && matches!(ticker, "SPX" | "NDX" | "DJI" | "SPY" | "NIFTY 50" | "NIFTYBANK"),
    }
}
/// Every admitted rule except the close-only 2-period ROC fills or exits at the open (stop gaps, next-open orders).
pub fn reads_open(id: &str) -> bool { reviewed_daily(id) && id != "ss_2_period_roc" }

// ---------------------------------------------------------------- indicators

/// Window mean that is NaN until n finite values are available (connors::sma never recovers from a NaN).
fn sma_w(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len())
        .map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 })
        .collect()
}

/// Exponential moving average (alpha 2/(n+1)), seeded with the first finite value (TradeStation XAverage).
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    let mut prev = f64::NAN;
    for i in 0..v.len() {
        if v[i].is_finite() {
            prev = if prev.is_finite() { prev + a * (v[i] - prev) } else { v[i] };
        }
        out[i] = prev;
    }
    out
}

/// Wilder directional system: (+DI, -DI, ADX), same construction as connors::dmi.
fn dmi3(r: &[Row], n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = r.len();
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    let trs = true_range(r);
    for i in 1..len {
        let up = r[i].high - r[i - 1].high;
        let dn = r[i - 1].low - r[i].low;
        pdm[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        mdm[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&tr, n));
    let (mut p, mut m, mut dx) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 0..len {
        if st[i] > 0.0 {
            p[i] = 100.0 * sp[i] / st[i];
            m[i] = 100.0 * sm[i] / st[i];
            dx[i] = if p[i] + m[i] > 0.0 { 100.0 * (p[i] - m[i]).abs() / (p[i] + m[i]) } else { 0.0 };
        }
    }
    (p, m, wilder(&dx, n))
}

/// Raw stochastic %K over n bars (0-100).
fn fast_k(x: &Ctx, n: usize) -> Vec<f64> {
    (0..x.c.len())
        .map(|i| {
            if i + 1 < n {
                return f64::NAN;
            }
            let (ll, hh) = (min_of(&x.l[i + 1 - n..=i]), max_of(&x.h[i + 1 - n..=i]));
            if hh > ll { 100.0 * (x.c[i] - ll) / (hh - ll) } else { 50.0 }
        })
        .collect()
}

/// Wilder RSI of a series that may have missing values; NaN wherever today's or yesterday's value is missing.
fn rsi_nan(v: &[f64], n: usize) -> Vec<f64> {
    // start after any leading NaNs: connors::rsi turns the NaN first change into a 0 change (NaN.max(0) = 0)
    let k = v.iter().position(|x| x.is_finite()).unwrap_or(v.len());
    let mut r = vec![f64::NAN; k];
    r.extend(rsi(&v[k..], n));
    for i in 0..v.len() {
        if !v[i].is_finite() || (i > 0 && !v[i - 1].is_finite()) {
            r[i] = f64::NAN;
        }
    }
    r
}

/// LBR/RSI (ch.7): an n-period RSI of the one-period rate of change (today's close - yesterday's close).
fn lbr_rsi(c: &[f64], n: usize) -> Vec<f64> {
    let roc: Vec<f64> = (0..c.len()).map(|i| if i == 0 { f64::NAN } else { c[i] - c[i - 1] }).collect();
    rsi_nan(&roc, n)
}

/// 2-period ROC "go home" state (ch.8): +1 when today's close is above yesterday's short-term pivot
/// (pivot = yesterday's close + yesterday's ROC, ROC = C - C[n]), i.e. the ROC rose; -1 below; unchanged if equal.
fn roc_state(c: &[f64], n: usize) -> Vec<f64> {
    let mut s = vec![0.0; c.len()];
    for i in n + 1..c.len() {
        let (r0, r1) = (c[i] - c[i - n], c[i - 1] - c[i - 1 - n]);
        s[i] = if r0 > r1 { 1.0 } else if r0 < r1 { -1.0 } else { s[i - 1] };
    }
    s
}

/// Forward-fill holes inside a series (never past its last value, so a series that stops does not keep signalling).
// ponytail: a missing breadth day repeats the previous one; fine for 3-5 day averages.
fn ffill_inner(v: &[f64]) -> Vec<f64> {
    let mut out = v.to_vec();
    let Some(last) = v.iter().rposition(|x| x.is_finite()) else { return out };
    for i in 1..=last {
        if !out[i].is_finite() {
            out[i] = out[i - 1];
        }
    }
    out
}

fn max_of(v: &[f64]) -> f64 {
    v.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
}
fn min_of(v: &[f64]) -> f64 {
    v.iter().cloned().fold(f64::INFINITY, f64::min)
}

// ---------------------------------------------------------------- local simulator

#[derive(Clone, Copy)]
enum Fill {
    Close,          // on the close of bar `from`
    Open,           // at the open of bar `from`
    Stop(f64),      // resting stop: fills at the level, or at the open if the bar opens through it
    StopAfter(f64, u8), // stop placed after an intraday break of the opposite side; u8 = fill_rule (see module doc)
}

#[derive(Clone, Copy)]
struct Ord {
    dir: f64,
    fill: Fill,
    setup: usize,
    from: usize,
    last: usize,
    stop: f64,     // initial protective stop (NaN = none)
    rev: f64,      // entry-day stop-and-reverse level (NaN = none)
    target: f64,   // profit target (limit), NaN = none
    stop_pct: f64, // protective stop as a fraction of the fill price (NaN = none)
}

fn ord(dir: f64, fill: Fill, setup: usize, from: usize, last: usize) -> Ord {
    Ord { dir, fill, setup, from, last, stop: f64::NAN, rev: f64::NAN, target: f64::NAN, stop_pct: f64::NAN }
}

struct Pos {
    dir: f64,
    i: usize,
    px: f64,
    lv: f64,
    setup: usize,
    stop: f64,
    rev: f64,
    target: f64,
    trailed: bool,
    half: Option<(usize, f64)>, // (bar, close) of the half taken off
    kind: &'static str,
}

enum Act {
    Close(String),
    NextOpen(String),
    SetStop(f64),
    SetTarget(f64),
}

type Rule<'a> = &'a dyn Fn(usize, &Pos) -> Option<Act>;

struct Ex<'a> {
    trail: usize,    // n-bar channel trailing stop (0 = off), from the bar after entry, ratchets only
    half_bar: usize, // take half off on the close of the bar this many bars after entry (0 = off)
    rule: Rule<'a>,  // close-of-bar rule
    gate: Option<&'a dyn Fn(&Ord, Option<&Trade>) -> bool>, // may an order fill, given the last closed trade
    // Some = after a losing stop-out, a stop at the original entry price stays live while this says so (order, bar)
    reentry: Option<&'a dyn Fn(&Ord, usize) -> bool>,
}

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Normal,
    EntryStop,  // the bar a stop order filled on
    EntryClose, // filled on the close (only the close rule runs)
}

fn push(out: &mut Vec<Trade>, p: Pos, j: usize, sig: usize, px: f64, why: String, open: bool) {
    let exit_px = p.half.map_or(px, |(_, hp)| 0.5 * hp + 0.5 * px);
    let exits = p.half.map_or(Vec::new(), |(hi, hp)| vec![(hi, hp, 0.5), (j, px, 0.5)]);
    let reason = if p.half.is_some() { format!("half off on an earlier close; rest: {why}") } else { why };
    out.push(Trade { dir: p.dir, legs: vec![Leg { i: p.i, px: p.px, w: 1.0 }], exit_i: j, exit_px, reason, entry_kind: p.kind, signal_i: sig, open, exits });
}

fn manage(x: &Ctx, ex: &Ex, j: usize, p: &mut Pos, mode: Mode, exit_open: &mut Option<String>, to: usize) -> Option<(f64, String)> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let long = p.dir > 0.0;
    if mode != Mode::EntryClose {
        let n = ex.trail;
        if mode == Mode::Normal && n > 0 && j > p.i && j >= n {
            let lv = if long { min_of(&l[j - n..j]) } else { max_of(&h[j - n..j]) };
            let ns = if long { p.stop.max(lv) } else { p.stop.min(lv) };
            if ns != p.stop {
                p.trailed = true;
                p.stop = ns;
            }
        }
        let entry = mode == Mode::EntryStop;
        if p.stop.is_finite() {
            let hit = if entry { if long { l[j] < p.stop } else { h[j] > p.stop } } else if long { l[j] <= p.stop } else { h[j] >= p.stop };
            if hit {
                let gap = !entry && if long { o[j] <= p.stop } else { o[j] >= p.stop };
                let what = if p.trailed { format!("trailing stop ({n}-bar channel)") } else { "protective stop".to_string() };
                return Some(if gap { (o[j], format!("{what}, gapped: out on the open")) } else { (p.stop, what) });
            }
        }
        if p.target.is_finite() {
            let t = p.target;
            if if long { h[j] >= t } else { l[j] <= t } {
                // entry bar: a target the fill already gapped past (re-entry / stop filled at a gap open) is
                // marketable at once -> out at the fill price, never at an untraded target level
                let px = if entry { if long { t.max(p.px) } else { t.min(p.px) } } else if long { o[j].max(t) } else { o[j].min(t) };
                return Some((px, "profit target".into()));
            }
        }
    }
    if ex.half_bar > 0 && p.half.is_none() && j - p.i >= ex.half_bar {
        p.half = Some((j, c[j]));
    }
    match (ex.rule)(j, p) {
        Some(Act::Close(w)) => return Some((c[j], w)),
        Some(Act::NextOpen(w)) if j < to => *exit_open = Some(w),
        Some(Act::SetStop(s)) => p.stop = s,
        Some(Act::SetTarget(t)) => p.target = t,
        _ => {}
    }
    None
}

/// One position at a time over bars [from, to].
fn sim(x: &Ctx, mut orders: Vec<Ord>, ex: &Ex, from: usize, to: usize) -> Vec<Trade> {
    orders.sort_by_key(|o| o.from);
    let w = orders.iter().map(|o| o.last - o.from).max().unwrap_or(0);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out: Vec<Trade> = Vec::new();
    let mut pos: Option<Pos> = None;
    let mut exit_open: Option<String> = None;
    let mut first = 0;
    let (mut re, mut tmpl): (Option<Ord>, Option<Ord>) = (None, None); // live re-entry order; re-entry template of the open position
    let to_re = |out: &Vec<Trade>, why: &str, tmpl: Option<Ord>, j: usize| -> Option<Ord> {
        let lost = out.last().map_or(false, |t| t.ret(0.0) <= 0.0);
        (ex.reentry.is_some() && why.contains("stop") && lost).then(|| tmpl.map(|t| Ord { from: j + 1, last: to, ..t })).flatten()
    };
    for j in from..=to {
        let mut phase = 0; // 1 = a position closed on this open, 2 = closed intrabar or on the close
        if let Some(mut p) = pos.take() {
            if let Some(why) = exit_open.take() {
                push(&mut out, p, j, j - 1, o[j], why, false);
                phase = 1;
            } else if let Some((px, why)) = manage(x, ex, j, &mut p, Mode::Normal, &mut exit_open, to) {
                push(&mut out, p, j, j, px, why.clone(), false);
                re = to_re(&out, &why, tmpl, j);
                phase = 2;
            } else {
                pos = Some(p);
                continue;
            }
        }
        while first < orders.len() && orders[first].from + w < j {
            first += 1;
        }
        let mut best: Option<(f64, f64, Ord)> = None;
        for od in &orders[first..] {
            if od.from > j {
                break;
            }
            if j > od.last {
                continue;
            }
            if let Some(gate) = ex.gate {
                if !gate(od, out.last()) {
                    continue;
                }
            }
            let long = od.dir > 0.0;
            let f: Option<f64> = match od.fill {
                Fill::Close => (j == od.from).then(|| c[j]),
                Fill::Open => (j == od.from && phase <= 1).then(|| o[j]),
                Fill::Stop(lv) if phase <= 1 => {
                    if long { (h[j] >= lv).then(|| o[j].max(lv)) } else { (l[j] <= lv).then(|| o[j].min(lv)) }
                }
                Fill::StopAfter(lv, fr) if phase <= 1 => {
                    let (touch, beyond, back) = if long { (h[j] >= lv, o[j] >= lv, c[j] >= lv) } else { (l[j] <= lv, o[j] <= lv, c[j] <= lv) };
                    (touch && (!beyond || fr == 1 || (fr == 2 && back))).then_some(lv)
                }
                _ => None,
            };
            let Some(px) = f else { continue };
            let pri = if matches!(od.fill, Fill::Close) { f64::INFINITY } else { (px - o[j]).abs() };
            if best.as_ref().map_or(true, |b| pri < b.0) {
                best = Some((pri, px, *od));
            }
        }
        let mut from_re = false;
        if let (Some(r), Some(live)) = (re, ex.reentry) {
            if j >= r.from {
                if !live(&r, j) {
                    re = None;
                } else if let (Fill::Stop(lv), true) = (r.fill, phase <= 1) {
                    let f = if r.dir > 0.0 { (h[j] >= lv).then(|| o[j].max(lv)) } else { (l[j] <= lv).then(|| o[j].min(lv)) };
                    if let Some(px) = f {
                        if best.as_ref().map_or(true, |b| (px - o[j]).abs() < b.0) {
                            best = Some(((px - o[j]).abs(), px, r));
                            from_re = true;
                        }
                    }
                }
            }
        }
        let Some((_, px, od)) = best else { continue };
        re = None;
        tmpl = Some(if from_re { od } else { Ord { fill: Fill::Stop(px), ..od } });
        let kind = if from_re { "re-entry" } else { match od.fill { Fill::Close => "close", Fill::Open => "open", _ => "stop" } };
        let stop = if od.stop_pct.is_finite() { px * (1.0 - od.dir * od.stop_pct) } else { od.stop };
        let mut p = Pos { dir: od.dir, i: j, px, lv: px, setup: od.setup, stop, rev: od.rev, target: od.target, trailed: false, half: None, kind };
        let mut mode = match od.fill { Fill::Close => Mode::EntryClose, Fill::Open => Mode::Normal, _ => Mode::EntryStop };
        if mode == Mode::EntryStop && p.rev.is_finite() && if p.dir > 0.0 { l[j] <= p.rev } else { h[j] >= p.rev } {
            let rv = p.rev;
            let q = Pos { dir: -p.dir, i: j, px: rv, lv: rv, setup: p.setup, stop: p.lv, rev: f64::NAN, target: f64::NAN, trailed: false, half: None, kind: "reverse" };
            push(&mut out, p, j, j, rv, "stop and reverse (entry day)".into(), false);
            p = q;
            mode = Mode::EntryClose;
        }
        match manage(x, ex, j, &mut p, mode, &mut exit_open, to) {
            Some((xp, why)) => {
                push(&mut out, p, j, j, xp, why.clone(), false);
                re = to_re(&out, &why, tmpl, j);
            }
            None => pos = Some(p),
        }
    }
    if let Some(p) = pos {
        push(&mut out, p, to, to, c[to], "open".into(), true);
    }
    out
}

/// One order simulated on its own (Moore studies with overlap = 1).
fn go_alone(x: &Ctx, od: Ord, rule: &dyn Fn(usize, &Pos) -> Option<Act>, to: usize) -> Vec<Trade> {
    if od.from > to {
        return vec![];
    }
    sim(x, vec![od], &Ex { trail: 0, half_bar: 0, rule, gate: None, reentry: None }, od.from, (od.last + 6).min(to))
}

/// Moore Research exits (appendix tables): 0 = close of the entry day, 1 = next open, 2 = next close, 3 = 2nd close, ...
fn moore_exit(k: usize) -> impl Fn(usize, &Pos) -> Option<Act> {
    move |j, p| {
        let b = j - p.i;
        match k {
            0 => (b == 0).then(|| Act::Close("exit on the close of the entry day".into())),
            1 => (b == 0).then(|| Act::NextOpen("exit on the next open".into())),
            _ => (b + 1 >= k).then(|| Act::Close(if k == 2 { "exit on the next close".into() } else { format!("exit on close #{} after entry", k - 1) })),
        }
    }
}

/// Moore 2-period channel stop & reverse (Exhibit A.20): always in the market.
fn channel_sar(x: &Ctx, n: usize, from: usize, to: usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mk = |d: f64, i: usize, px: f64, j: usize, xp: f64, why: &str, open: bool| Trade {
        dir: d, legs: vec![Leg { i, px, w: 1.0 }], exit_i: j, exit_px: xp, reason: why.into(), entry_kind: "stop", signal_i: j, open, exits: Vec::new()
    };
    let mut out = Vec::new();
    let mut pos: Option<(f64, usize, f64)> = None;
    for j in from.max(n)..=to {
        let (hi, lo) = (max_of(&h[j - n..j]), min_of(&l[j - n..j]));
        match pos {
            None => {
                let (up, dn) = (h[j] >= hi, l[j] <= lo);
                let d = if up && dn { if o[j].max(hi) - o[j] <= o[j] - o[j].min(lo) { 1.0 } else { -1.0 } } else if up { 1.0 } else if dn { -1.0 } else { 0.0 };
                if d != 0.0 {
                    pos = Some((d, j, if d > 0.0 { o[j].max(hi) } else { o[j].min(lo) }));
                }
            }
            Some((d, i, px)) if j > i => {
                if if d > 0.0 { l[j] <= lo } else { h[j] >= hi } {
                    let xp = if d > 0.0 { o[j].min(lo) } else { o[j].max(hi) };
                    out.push(mk(d, i, px, j, xp, if d > 0.0 { "stop & reverse: 2-bar low broken" } else { "stop & reverse: 2-bar high taken out" }, false));
                    pos = Some((-d, j, xp));
                }
            }
            _ => {}
        }
    }
    if let Some((d, i, px)) = pos {
        out.push(mk(d, i, px, to, c[to], "open", true));
    }
    out
}

// ---------------------------------------------------------------- catalogue

const TRAIL2: &str = "X|[From elsewhere in the book] \"trail a stop\" = a 2-period channel (Linda, Appendix p.232: \"I have also used a 2-period channel as a way to trail a stop in a trending market\"): stop at the lowest low (highest high) of the prior 2 bars, ratcheting only";
const INDEX: &str = "The book traded futures and stocks; run here on the indices / ETFs. 1 tick = tick_pct % of price (0.01% default, about one S&P futures tick in 1995). India: NIFTY indices daily, no night session, same rules. NIFTY index opens before about Nov 2010 are stale (16-34% of days open within 0.01% of the prior close; NSE's pre-open call auction began late 2010): for rules that use the open (gaps, next-open entries or exits) set From to 2010-11-01 or later on NSE indices.";
const MOORE: &str = "Moore Research Center tendency study (Appendix): 25 US futures 1986-1995, \"we are not testing mechanical systems\", no stops, costs or slippage; Win(%) counts positive outcomes, averages are $ per contract. Implemented as a mechanical rule with the table's own exit; the only data overlap is SPY 1993-1995 against the S&P row, so the comparison is an approximation (index ETF vs. rolled futures contracts; compare win % and frequency, not dollars). By default one position at a time: a setup that arrives while the previous one is still held is skipped; overlap = 1 counts every setup independently as Moore did (use it for the table comparison; the equity curve is then not meaningful because the trades overlap). India: NIFTY index opens before about Nov 2010 are stale, so for next-open exits and gap setups on NSE use From 2010-11-01 or later.";

const FILLRULE: &str = "E|[Not from the book: daily-bar fill rule] the stop is placed only after the break, so a bar that opened beyond the level is ambiguous. fill_rule 0 (default, unbiased) = only bars that opened inside the level can fill; 1 = every bar that touches the level fills (pessimistic bound); 2 = an opened-beyond bar fills only if it closed back beyond the level (biased upward: keeps the winners, drops the losers)";
const TS: &[&str] = &[
    "F|\"Today must make a new 20-day low-the lower the better.\" (low below the lowest low of the prior 20 days)",
    "F|\"The previous 20-day low must have occurred at least four trading sessions earlier. This is very important.\"",
    "E|\"After the market falls below the prior 20-day low, place an entry buy stop 5-10 ticks above the previous 20-day low. This buy stop is good for today only.\" (5 ticks)",
    FILLRULE,
    "X|\"If the buy stop is filled, immediately place an initial good-till-canceled sell stop-loss one tick under today's low.\"",
    "X|\"As the position becomes profitable, use a trailing stop to prevent giving back profits.\"",
    TRAIL2,
    "X|[Not from the book: TradeStation code inputs Bars2Exit 6 / ExitPct 50] half_bar = 6 takes half off on the close of bar 6 and trails the rest; default 0 = the chapter's trailing stop only",
    "F|\"FOR BUYS (SELLS ARE REVERSED)\": new 20-day high, sell stop 5 ticks under the previous 20-day high, stop one tick above today's high",
];
const TSP1: &[&str] = &[
    "F|\"The market makes a new 20-day low. The previous 20-bar low must have been made at least three trading sessions earlier.\"",
    "F|\"The close of the new low (day one) must be at or below the previous 20-bar low\"",
    "E|\"An entry buy stop is placed the next day (day two) at the earlier 20 day low. If you are not filled on day two, the trade is cancelled.\" (an open beyond it fills at the open)",
    "X|\"If filled, place a protective sell stop one tick under the lower of the day-one low or the day-two low.\" (day-one low on the entry day, the lower of the two from day three)",
    "X|\"Take partial profits within two to six bars and trail a stop on the balance of your position.\" Half off on the close of bar 6 (TradeStation code input Bars2Exit 6, ExitPct 50)",
    TRAIL2,
    "F|\"FOR BUYS (SELLS ARE REVERSED)\"",
];
const E8020: &[&str] = &[
    "F|\"Yesterday the market opened in the top 20 percent of its daily range and closed in the lower 20 percent of its daily range.\"",
    "F|\"Today the market must trade at least 5-15 ticks below yesterday's low\" (10 ticks)",
    "E|\"An entry buy stop is then placed at yesterday's low.\"",
    FILLRULE,
    "X|\"Upon being filled, place an initial protective stop near the low extreme of today.\" (today's low: on daily bars it was made before the entry)",
    "X|\"Move the stop up to lock in accrued profits. This trade is a day trade only.\" Out on the close",
    "F|\"FOR BUYS (SELLS ARE REVERSED)\"",
];
const PINBALL: &[&str] = &[
    "F|\"Plot a three-period RSI of a one-period rate of change (the daily net change). We refer to this as an LBR/RSI.\"",
    "F|\"Day one is determined by an LBR/RSI value of less than 30.\" (sells: greater than 70)",
    "E|\"On day two, place a buy stop above the HIGH of the first hour's trading range.\" (series OPEN60_HIGH + 1 tick; sells: below OPEN60_LOW)",
    "X|\"Upon being filled, place a resting sell stop at the low of the first hour's range to protect your trade.\" (a low under OPEN60_LOW later in the day counts as hit)",
    "X|\"If the trade closes with a profit, carry it overnight.\" Otherwise out on the day-two close",
    "X|\"Exit on morning follow-through the next day (day three). Be sure to exit this trade by the close of the next day. Taylor would look to exit a long trade 'just above the high of the previous day.'\" Day 3: sell limit 1 tick above the day-two high, else out on the close",
];
const ROC2: &[&str] = &[
    "F|\"close (day one) - close (day three) equals the 2-period rate of change.\" (worksheet p.61: ROC = C - C[2])",
    "F|\"Add this number to yesterday's closing price (day two). This will be our short-term pivot number.\"",
    "E|\"We want to go home long if we have been on a sell signal and the price then closes above this pivot number.\" Buy the close of a fresh flip",
    "E|\"We will look to short if the 2-period rate of change flips from a buy to a sell and the price is going to close below the short-term pivot number.\"",
    "X|\"If you had entered on the close of a fresh 'flip' and exited on the close the following day\" (p.62)",
];
const ANTI: &[&str] = &[
    "F|\"Use a seven-period %K stochastic (the 'fast' line). If your program allows for an adjustment of the smoothing of this parameter, default to four.\" Fast = 4-bar SMA of 7-bar %K",
    "F|\"Use a 10-period %D stochastic (the 'slow' line).\" Slow = TradeStation SlowD(10): 10-bar %K smoothed twice by 3-bar SMAs",
    "F|\"The slow line (%D) has established a definite upward-trend.\" (TradeStation 'Anti basic 2': slow > slow[1] > slow[2])",
    "E|\"Enter when the price action causes the fast line to turn up once again in the direction of the slow line (forming a hook).\" (fast crosses over fast[1])",
    "E|\"We enter the trade on the opening the next morning\" (examples)",
    "X|\"The initial stop should be placed just below the bar of entry.\" (1 tick under the entry bar's low, from its close)",
    "X|\"Once you are in a winning trade, look to exit on a buying or selling climax within three to four bars.\" Out on the close of day 4",
    "F|\"BUY SETUP (SELLS ARE REVERSED)\"",
];
const ANTI310: &[&str] = &[
    "F|\"a 3-10 moving-average oscillator with a simple 16-period moving average of itself\" (fast = SMA3 - SMA10, slow = SMA16 of fast)",
    "F|(TradeStation 'Anti basic 1') slow line rising two bars: slow > slow[1] > slow[2]",
    "E|(TradeStation) \"FASTLINE CROSSES OVER FASTLINE[1]\": the hook; buy on the next open (sells reversed)",
    "X|\"The initial stop should be placed just below the bar of entry.\"",
    "X|\"look to exit on a buying or selling climax within three to four bars.\" Out on the close of day 4",
];
const ANTITRIG: &[&str] = &[
    "F|\"When the %K and %D have formed opposing slopes for at least three days, creating a tension between them,\" (buys: %D rising, %K falling)",
    "E|\"place a buy stop one tick above the previous day's bar. (This will be to go long when %D has returned to a positive slope.)\"",
    "E|\"If the buy stop is not hit, keep on trailing it down to the high of each previous day's bar.\" (while %K keeps falling)",
    "X|\"The initial stop should be placed just below the bar of entry.\"",
    "X|\"look to exit on a buying or selling climax within three to four bars.\" Out on the close of day 4",
];
const GRAIL: &[&str] = &[
    "F|\"A 14-period ADX must initially be greater than 30 and rising. This will identify a strongly trending market.\" (ADX > 30 on the touch bar; trend side from +DI vs -DI; \"rising\" = ADX > 30 and above the prior bar on some bar of the 10 bars up to the touch, [Not from the book] the 10-bar window, param adx_rising, 0 = ADX > 30 only)",
    "F|\"Look for a retracement in price to the 20-period exponential moving average.\" (yesterday's close above the EMA)",
    "E|\"When the price touches the 20-period exponential moving average, put a buy stop above the high of the previous bar.\" (1 tick above the touch bar's high, next bar only; each new touch re-arms)",
    "X|\"Once filled, enter a protective sell stop at the newly formed swing low.\" (1 tick under the touch bar's low)",
    "X|\"Trail the stop as profits accrue and look to exit at the most recent swing high.\" Target = highest high of the 20 bars before the touch ([Not from the book]: the 20-bar window)",
    TRAIL2,
    "F|\"After a successful trade, the ADX must once again turn up above 30 before another retracement to the moving average can be traded.\"",
    "E|\"If stopped out, re-enter this trade by placing a new buy stop at the original entry price.\" (param reentry = 1: after a losing stop-out, a buy stop at the first entry's fill price with the same protective stop and target; re-armed after each stop-out)",
    "E|[Book silent: order life] the re-entry stop stays live until filled, until a new touch of the EMA (a new setup) or until the ADX closes at or below 30",
    "F|\"FOR BUYS (SELLS ARE REVERSED)\"",
];
const GAPPER: &[&str] = &[
    "F|\"We will use a 12-period ADX and a 28-period +DI/-DI.\" \"The ADX must be greater than 30.\" (yesterday's values)",
    "F|\"For buys, the +DI must be greater than the -DI; for sells, the -DI must be greater than the +DI.\"",
    "F|\"Today's open must gap below yesterday's low.\"",
    "E|\"A buy stop is placed in the area of yesterday's low.\"",
    "X|\"If filled, a protective sell stop is placed at today's low.\"",
    "X|\"either exit the position before the close or carry it into the following day if it closes strongly.\" \"Back testing of this strategy shows profit improvement holding these trades into the next morning.\" Exit mode 2 (default) = carry to the next open only if it closes strongly, else out on the close (\"strongly\" = close in the favourable half of the range, [Not from the book]); 0 = always out on the close; 1 = [Not from the book] always carry to the next open (\"Back testing ... shows profit improvement\")",
    "F|\"FOR BUYS (SELLS ARE REVERSED)\"",
];
const NOGAP: &[&str] = &[
    "F|\"Let's assume the ADX is above 30 and the trend is up.\" (ADX(12) > 30, +DI(28) > -DI(28), yesterday)",
    "F|\"I will look for some early morning weakness in the market, perhaps a down opening.\" (open below yesterday's close)",
    "E|\"If the market begins to recover and moves above yesterday's close, I will buy.\" (buy stop 1 tick above yesterday's close)",
    "X|\"Usually at today's morning low.\" (today's low)",
    "X|\"otherwise, I will just scratch the trade.\" Same exit modes as the ADX Gapper (default 2: carry to the next open only on a strong close; the chapter has no overnight rule for this variant)",
];
const WHIP: &[&str] = &[
    "F|\"The market must gap lower than the previous day's low.\"",
    "F|\"The close must be higher than the opening and also in the top 50 percent of the day's trading range.\"",
    "E|\"If rules 1 and 2 are met, buy MOC.\"",
    "X|\"If tomorrow opens below today's close (indicating a loss on the position), sell immediately! Take the loss!\"",
    "X|\"If tomorrow opens with a profit, trail a stop to protect profits.\" Stop at the entry price (\"do not let this profit turn into a loss\"); an open exactly at the entry price is neither a loss nor a profit: held without a stop; [Not from the book] out on the close of day 1 (exit mode 1). Exit mode 0 = always out on the next open (the \"head start\")",
    "F|\"BUY SETUP (SELLS ARE REVERSED)\"",
];
const GAP3: &[&str] = &[
    "F|\"Today the market must gap lower and not fill the gap.\" (the whole bar below yesterday's low)",
    "E|\"Over the next three trading sessions, have in place a buy stop one tick above the high of the gap-down day.\"",
    "X|\"If filled, place a protective sell stop at the low of the gap-down day.\" (optional cap: max_risk_pct, book: \"risk in the range of two to three points\")",
    "X|\"Protect any accrued profits with a trailing stop.\"",
    TRAIL2,
    "E|\"If not filled after three trading sessions, cancel the initial buy stop.\"",
    "F|\"BUY SETUP (SELLS ARE REVERSED)\"",
];
const NR4: &[&str] = &[
    "F|\"An NR4 is a trading day with the narrowest daily range of the last four days. An inside day has a higher low than the previous day's low and a lower high than the previous day's high.\"",
    "E|\"The next day only, place a buy-stop one tick above and a sell-stop one tick below the ID/NR4 bar.\"",
    "X|\"On entry day only, if we are filled on the buy side, enter an additional sell-stop one tick below the ID/NR4 bar. ... we will reverse and go short.\" [Not from the book] after the entry day the protective stop stays at the other side of the ID/NR4 bar",
    "X|\"Trail a stop to lock in accrued profits.\"",
    TRAIL2,
    "X|\"If the position is not profitable within two days and you have not been stopped out, exit the trade MOC (market on close.)\" (close of the 2nd day after the setup, as in the p.155 example; the older TradeStation ELA S0014 uses BarsSinceEntry = 2, which is time_days = 3)",
];
const HVC: &[&str] = &[
    "F|\"compare the six-day historical volatility reading to the 100-day historical volatility reading. We are looking for the 6/100 reading to be under 50 percent\" (HV = sample stdev of log returns, Appendix p.203)",
    "F|\"If rule one is met, today (day one) must be either an inside day or an NR4 day.\"",
    "E|\"On day two, place a buy-stop one tick above the day-one high and a sell-stop one tick below the day-one low.\"",
    "X|\"If your buy-stop is filled, place an additional sell-stop one tick below the day-one low. ... This additional sell-stop is done on the entry day only\" (stop-and-reverse) [Not from the book] after the entry day the protective stop stays at the other side of day one",
    "X|\"A trailing stop should be used to lock in profits on a winning trade.\"",
    TRAIL2,
    "X|Larry (p.169): \"I will stay in the position as long as it continues to move in my favour. Usually though, I am out within one to four days.\" Out on the close of day 4",
];
const NEWS: &[&str] = &[
    "F|\"Wait for an economic news event to be released at 8:30 EST.\" (series NEWS_DAY = 1 on release days)",
    "F|\"If the report immediately lifts the bond market at least four ticks above the previous day's high,\" (the open, which comes after the 8:30 release for index ETFs)",
    "E|\"place a sell-stop one to three ticks underneath the previous day's high.\" (2 ticks; buys reversed at the previous day's low)",
    "X|\"If filled, place an initial protective stop one tick above today's high (or for buys, one tick below today's low).\"",
    "X|\"As the position becomes profitable, immediately move the stop to breakeven.\" (cannot be sequenced on daily bars) [Not from the book] out on the close",
];
const SMI: &[&str] = &[
    "F|\"Take the net change for the first hour in the Dow. Multiply it by -1.\" (OPEN60_CLOSE - yesterday's close)",
    "F|\"Take the net change for the last hour in the Dow and add it to the final number in rule 1.\" (close - LAST60_OPEN)",
    "E|\"If the day's sum reading is greater than 20, look to be a buyer the next day. If the day's sum reading is less than [-]20, look to be a seller the next day.\" (20 Dow points ≈ 0.4% in 1995)",
    "E|[Not from the book] buy (sell) the next day's open",
    "X|[Not from the book] out on that day's close",
];
const TICKX: &[&str] = &[
    "F|\"If day one registers a reading of ticks greater than 400, then on day two we will look to short a morning rally.\" (series NYSE_TICK.High)",
    "F|\"If day one registers a reading of lower than -400, then on day two we will look to buy a morning pull-back.\" (NYSE_TICK.Low)",
    "E|[Not from the book] short (buy) the day-two open (\"look to short a morning rally\" is discretionary)",
    "X|\"Please remember to always use stops when entering on a reversal in the S&P.\" [Not from the book] out on the day-two close",
];
const TRIN: &[&str] = &[
    "F|\"This is a ratio of two ratios: advances/declines divided by up volume/down-volume.\" \"We will use a five-day moving average of the Arms Index.\"",
    "E|\"A reading greater than 1.20 indicates a potential intermediate-term bottom or oversold condition.\" [Not from the book] buy the close of the day the 5-day average first rises above 1.20 (the book gives no entry)",
    "E|\"A reading less than .80 indicates a potential short-term top or overbought condition.\" [Not from the book] sell the close of the day it first falls below 0.80",
    "X|\"These are short-term trading opportunities only (one to three days).\" [Not from the book] out on the close of day 3",
];
const ADB: &[&str] = &[
    "F|\"Take a three-day average of the advances minus declines on the NYSE.\"",
    "F|\"Plot the three-period RSI (Welles Wilder's Relative Strength Index)\" of that average",
    "E|\"Sell the market when the RSI downticks from the above 70 level and the following rules are met: a. The RSI change (thrust) is less than 40 and b. The market today is under its 100-day exponential moving average.\" (RSI[1] > 70 and today's RSI lower: a change of direction, p.226; cross = 1 also requires it back at or under 70 [reading fitted to p.227, not in the text])",
    "E|\"Buy the market when the RSI upticks from the below 30 level and the uptick is less than 40 and the market is trading above its 100-day exponential moving average.\" (RSI[1] < 30 and today's RSI higher: \"the change in direction of the RSI\", p.226; cross = 1 also requires it back at or over 30 [reading fitted to p.227, not in the text])",
    "E|[Not from the book] the entry price is not stated: next open (entry_mode 1, default; a SuperCharts \"buy at market\" on a daily bar fills on the next open); 0 = the signal close",
    "X|\"Exit in three days or on a 2 percent stop.\" (close of the 3rd day after entry; stop 2% from the entry price)",
];

fn profile(what: &'static str, side: &'static str) -> Vec<&'static str> {
    vec![what, side, "E|[Moore columns] \"Opened Up/Down\" = enter on the setup close, exit on the next open (exit 1, default); \"Closed Up/Down\" = exit on the next close (exit 2)", "X|\"no money management stops\": exit by the table column only"]
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id: &'static str, name: &'static str, group: &'static str, dir: &'static str, status: Status, series: &'static [&'static str], rules: &'static [&'static str], notes: &'static str, claim: Option<Claim>| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: series, book: BOOK, rules, notes, claim,
    };
    let leak = |v: Vec<&'static str>| -> &'static [&'static str] { Box::leak(v.into_boxed_slice()) };
    let disc = NotTestable("discretionary");
    let mut v = vec![
        d("ss_turtle_soup", "Turtle Soup", "Ch 4 · Turtle Soup", "Long/Short", Live, &[], TS, INDEX, claim("15-20 / month (30 futures)", "-", "-", "hours to a few days", "frequency only, 1995 examples")),
        d("ss_turtle_soup_plus_one", "Turtle Soup Plus One", "Ch 5 · Turtle Soup Plus One", "Long/Short", Live, &[], TSP1, INDEX, None),
        d("ss_80_20", "80-20's", "Ch 6 · 80-20's", "Long/Short", Live, &[], E8020,
          "Day trade built for intraday data; run on daily bars: filled when today's low is at least 10 ticks under yesterday's low and the bar trades back to yesterday's low. Bars that opened above yesterday's low are ambiguous: skipped by default (fill_rule 0); fill_rule 1 counts them all (pessimistic), 2 = close-confirmed (biased upward, the first-round default). The protective stop at today's low cannot be hit after the fill on daily bars. India: NIFTY day session only, as the book asks.", None),
        d("ss_momentum_pinball", "Momentum Pinball", "Ch 7 · Momentum Pinball", "Long/Short", Live, &["OPEN60_HIGH", "OPEN60_LOW"], PINBALL,
          "Needs per-ticker series OPEN60_HIGH / OPEN60_LOW = high / low of the first 60 minutes of the session (US 9:30-10:30 ET, NSE 9:15-10:15 IST), from intraday bars. Re-entry after a stop-out is not modelled.", None),
        d("ss_2_period_roc", "2-Period ROC", "Ch 8 · 2-Period ROC", "Long/Short", Live, &[], ROC2,
          "ROC = C - C[2] (TradeStation MOMENTUM(C,2)); the prose has the subtraction backwards, the worksheet is right (unit test reproduces every row). Close above the pivot <=> the ROC rose. India: all indices.",
          claim("11", "8 of 11", "-", "1 day", "S&P Dec-94 futures, ~Oct 24 - Dec 1 1994")),
        d("ss_anti", "The Anti (7 %K / 10 %D)", "Ch 9 · The Anti", "Long/Short", Live, &[], ANTI, INDEX, None),
        d("ss_anti_3_10", "The Anti (3-10 oscillator)", "Ch 9 · The Anti", "Long/Short", Live, &[], ANTI310, INDEX, None),
        d("ss_anti_trigger", "The Anti · trigger entry", "Ch 9 · The Anti", "Long/Short", Live, &[], ANTITRIG, INDEX, None),
        d("ss_holy_grail", "The Holy Grail", "Ch 10 · Holy Grail", "Long/Short", Live, &[], GRAIL, INDEX, None),
        d("ss_adx_gapper", "ADX Gapper", "Ch 11 · ADX Gapper", "Long/Short", Live, &[], GAPPER,
          "ADX(12) and DI(28) read on yesterday's bar (known before the open). India: NIFTY index opens before about Nov 2010 are stale (NSE pre-open auction began late 2010): use From 2010-11-01 or later on NSE indices.",
          claim("2-4 / week (all markets)", "-", "-", "intraday or overnight", "not stated")),
        d("ss_adx_gapper_no_gap", "ADX trend reversal without a gap", "Ch 11 · ADX Gapper", "Long", Live, &[], NOGAP,
          "Larry calls this variant \"more subjective\"; daily-bar reading of the dialogue. The morning-low stop cannot be hit after the fill on daily bars.", None),
        d("ss_whiplash", "Whiplash", "Ch 12 · Whiplash", "Long/Short", Live, &[], WHIP, INDEX,
          claim("-", "\"almost 60 percent head start\" (next open)", "-", "overnight", "not stated (S&Ps, bonds)")),
        d("ss_three_day_unfilled_gap", "Three-Day Unfilled Gap Reversal", "Ch 13 · Three-Day Unfilled Gap Reversals", "Long/Short", Live, &[], GAP3, INDEX, None),
        d("ss_spike_and_ledge", "Spike and Ledge", "Ch 14 · A Picture's Worth a Thousand Words", "Long/Short", disc, &[],
          &["F|\"a buying climax forms a spike and ledge pattern\"", "E|\"A short trade is entered on the breakdown out of the ledge ... and an initial stop is placed on the other side of the ledge.\"", "X|\"wait until the market begins to give back any gains or stalls in its movement; then exit at-the-market\""],
          "\"These patterns are a purely subjective form of pattern recognition. This means that it is impossible to do any form of back testing on them.\" (p.109)", None),
        d("ss_fakeout_shakeout", "Fakeout-Shakeout", "Ch 14 · A Picture's Worth a Thousand Words", "Long/Short", disc, &[],
          &["F|\"Once a market breaks from a ledge or a triangle, it should not come back to the breakout point ... If it does, it sets up an excellent trade in the other direction.\"", "E|\"a buy stop is placed just above the point from which the market broke down.\"", "X|\"The initial stop is entered at the most recent swing low. The stop is quickly moved up to breakeven, and then trailed\""],
          "Discretionary chart pattern (ch.14: impossible to back test).", None),
        d("ss_three_little_indians", "Three Little Indians", "Ch 14 · A Picture's Worth a Thousand Words", "Long/Short", disc, &[],
          &["F|\"a climax pattern formed by three symmetrical peaks.\"", "E|\"must wait for the price to reverse off the last peak before we enter. ... We enter at-the-market\"", "X|\"The stop should quickly be moved to breakeven.\""],
          "Discretionary chart pattern (ch.14: impossible to back test).", None),
        d("ss_wolfe_wave", "Wolfe Waves", "Ch 15 · Wolfe Waves", "Long/Short", disc, &[],
          &["F|\"Number 2 wave is a top. Number 3 wave is the bottom of a first decline. Number 1 wave is the bottom prior to wave 2. Point 3 must be lower than point 1. ... The wave 4 point should be higher than the wave 1 bottom.\"", "E|\"A trend line is drawn from point 1 to point 3. The extension of this line projects to the anticipated reversal point ... wave 5. this is the entry point\"", "X|\"The Estimated Price at Arrival (EPA) is the trend line drawn from points 1 to 4. ... Our initial stop is placed just beneath the newly formed reversal at point 5.\""],
          "The book gives no swing definition (\"Point 2 must be a significant swing low or high\"); any zigzag threshold would be invented.", claim("3-6 setups / week", "-", "-", "-", "S&P 5-minute, 1995")),
        d("ss_morning_news_reversal", "Morning News Reversals", "Ch 17 · Morning News Reversals", "Long/Short", Live, &["NEWS_DAY"], NEWS,
          "Needs series NEWS_DAY (market-wide, 1 = a major scheduled release before the open: US 8:30 ET employment, CPI, PPI, GDP, retail sales; India: RBI policy, CPI). Book traded bonds/currencies; daily-bar approximation on index ETFs.", None),
        d("ss_big_picture_news_reversal", "Big Picture News Reversals", "Ch 18 · Big Picture News Reversals", "Long", disc, &[],
          &["F|\"We are looking for an extraordinary event to occur which causes a market to move dramatically.\"", "E|\"Place a resting stop order to enter the market at this previous closing price level.\"", "X|\"Risk with a stop up to the lowest level the stock reached after the sell off.\""],
          "Choosing an \"extraordinary event\" is judgement.", None),
        d("ss_id_nr4", "ID/NR4", "Ch 19 · Range Contraction", "Long/Short", Live, &[], NR4, INDEX, None),
        d("ss_hist_vol_crabel", "Historical Volatility Meets Toby Crabel", "Ch 20 · Historical Volatility", "Long/Short", Live, &[], HVC, INDEX, None),
        d("ss_smart_money_index", "Smart Money Index", "Ch 21 · Smart Money Indicators", "Long/Short", Live, &["OPEN60_CLOSE", "LAST60_OPEN"], SMI,
          "Needs per-ticker series OPEN60_CLOSE = price at the end of the first 60 minutes and LAST60_OPEN = price 60 minutes before the close (book uses the Dow; any index works). Threshold in % of price (20 Dow points at Dow 5,000).",
          claim("4-5 signals / month", "-", "-", "next day", "1995, Dow")),
        d("ss_tick_divergence", "Tick divergence", "Ch 21 · Smart Money Indicators", "Long/Short", NeedsData("5-minute S&P bars + NYSE TICK"), &[],
          &["F|\"The S&P must make a morning low where the ticks give a reading of less than -350.\"", "F|\"The S&P then must make an equal or lower low in price at least 90 minutes later. The tick must make a higher low\"", "E|\"If the ticks improve from this second low point by 100 ticks, we will enter at-the-market.\"", "X|\"A protective sell-stop is then entered at the low of the day.\" \"protect at least 50 percent of any equity gained\""],
          "Purely intraday (two lows 90 minutes apart on 5-minute bars, TICK path); no daily formulation. NSE publishes no TICK.", None),
        d("ss_tick_extreme_next_day", "Extreme TICK next-day reversal", "Ch 21 · Smart Money Indicators", "Long/Short", Live, &["NYSE_TICK.High", "NYSE_TICK.Low"], TICKX,
          "Needs NYSE_TICK.csv (columns High, Low = the day's highest / lowest NYSE TICK reading). US only (no NSE TICK).", None),
        d("ss_arms_index_5ma", "Arms Index (TRIN) 5-day average · NYSE", "Ch 21 · Smart Money Indicators", "Long/Short", Live,
          &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume"], TRIN,
          "TRIN computed from NYSE_ADV_DEC.csv (Advances, Declines, AdvVolume, DecVolume per day). \"We have found the buy signals to be more reliable than the sell signals.\"", None),
        d("ss_arms_index_5ma_nse", "Arms Index (TRIN) 5-day average · NSE", "Ch 21 · Smart Money Indicators", "Long/Short", Live,
          &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines", "NSE_ADV_DEC.AdvVolume", "NSE_ADV_DEC.DecVolume"], TRIN,
          "India mapping: NSE_ADV_DEC.csv with the same columns (NSE advancing/declining issues and their traded volume, built from bhavcopies). Same rules.", None),
        d("ss_ad_market_buster", "A/D Market Buster · NYSE", "Special Report · A/D Market Buster", "Long/Short", Live,
          &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines"], ADB,
          "Series NYSE_ADV_DEC.csv (NYSE advancing / declining issues). The report traded S&P futures (SuperCharts, $500 multiplier); run on SPY.",
          claim("114", "60%", "$736.80 / contract", "3 bars", "1/1/90 - 10/1/97, S&P 500 futures, PF 1.67")),
        d("ss_ad_market_buster_nse", "A/D Market Buster · NSE", "Special Report · A/D Market Buster", "Long/Short", Live,
          &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines"], ADB,
          "India mapping: NSE_ADV_DEC.csv (NSE advancing / declining issues); trade NIFTY with the 100-day EMA filter. With entry_mode 1 (next open) use From 2010-11-01 or later: NIFTY index opens before then are stale.", None),
    ];
    let m = |id, name, dir, rules: Vec<&'static str>, c| d(id, name, "Appendix · Moore Research studies", dir, Live, &[], leak(rules), MOORE, c);
    v.extend([
        m("ss_study_close_upper_90", "Close in top 10% of range", "Long", profile("F|\"the market closed in the upper 10 percent of its range\"", "E|Long = follow-through (table: Opened Up / Closed Up; a fade wins the complement)"),
          claim("408 setup days (17%)", "opened up 48%, closed up 48%", "penetrated high 85%, avg 2.00 pts", "next day", "86H-95M, S&P futures (A.1)")),
        m("ss_study_close_upper_80", "Close in top 20% of range", "Long", profile("F|\"days closing in the upper 20 percent of the range\"", "E|Long = follow-through (Opened Up / Closed Up)"),
          claim("720 setup days (30%)", "opened up 49%, closed up 50%", "penetrated high 80%, avg 2.06 pts", "next day", "86H-95M, S&P futures (A.2)")),
        m("ss_study_close_lower_10", "Close in bottom 10% of range", "Short", profile("F|\"day closing in the lower 10 percent of its range\"", "E|Short = follow-through (Opened Down / Closed Down)"),
          claim("215 setup days (9%)", "opened down 41%, closed down 43%", "penetrated low 88%, avg 3.05 pts", "next day", "86H-95M, S&P futures (A.3)")),
        m("ss_study_close_lower_20", "Close in bottom 20% of range", "Short", profile("F|\"day closing in the lower 20 percent of its range\"", "E|Short = follow-through (Opened Down / Closed Down)"),
          claim("375 setup days (15%)", "opened down 47%, closed down 42%", "penetrated low 83%, avg 2.79 pts", "next day", "86H-95M, S&P futures (A.4)")),
        m("ss_study_wr7_high_close", "WR7 with a high close", "Long", profile("F|\"the day following a WR7, which is the widest range of the last seven days ... WR7s with a high close\" (close in the upper half; close_def 1 = above yesterday's close)", "E|Long = follow-through (Opened Up / Closed Up)"),
          claim("143 setup days (6%)", "opened up 48%, closed up 54%", "penetrated high 74%, avg 2.05 pts", "next day", "86H-95M, S&P futures (A.5)")),
        m("ss_study_wr7_low_close", "WR7 with a low close", "Short", profile("F|\"WR7s with a low close\" (close in the lower half; close_def 1 = below yesterday's close)", "E|Short = follow-through (Opened Down / Closed Down)"),
          claim("174 setup days (7%)", "opened down 45%, closed down 45%", "penetrated low 68%, avg 3.32 pts", "next day", "86H-95M, S&P futures (A.6)")),
        m("ss_study_rocrsi_buy", "ROC/RSI buy (LBR/RSI < 30)", "Long", vec!["F|\"A three-period RSI is run on a one-period rate of change.\"", "E|\"If the reading is less than 30, the next day's opening is bought.\"", "X|\"We then examine the initial entry at the following morning's open and the following day's close. No protective stops are used.\" Exit 1 = next open (default), 2 = next close = the close of the day after the entry open (t+2 from the setup; reading t+1 = the entry day's close is exit 0)"],
          claim("≈269", "52% (next open), 53% (next close)", "$148 / $211", "1 day", "86H-95U, S&P futures (A.7)")),
        m("ss_study_rocrsi_sell", "ROC/RSI sell (LBR/RSI > 70)", "Short", vec!["F|\"A three-period RSI is run on a one-period rate of change.\"", "E|\"If the reading is greater than 70, the next day's opening is sold.\"", "X|Exit 1 = next open (default), 2 = next close = the close of the day after the entry open (t+2 from the setup; reading t+1 = exit 0). No protective stops"],
          claim("≈253", "49% (next open), 42% (next close)", "$-123 / $-269", "1 day", "86H-95U, S&P futures (A.8)")),
        m("ss_study_fresh_roc_buy", "Fresh 2-period ROC buy", "Long", vec!["F|\"a reversal in the 2-period rate of change\" (ROC = C - C[2] turns up: fresh flip to a buy, ch.8)", "E|\"entries taken on the close\"", "X|\"exiting on the next day's opening and the next day's close\" Exit 1 = next open (default), 2 = next close"],
          claim("≈595", "57% (next open), 57% (next close)", "$83 / $168", "1 day", "86H-95U, S&P futures (A.9)")),
        m("ss_study_fresh_roc_sell", "Fresh 2-period ROC sell", "Short", vec!["F|\"a reversal in the 2-period rate of change\" (fresh flip to a sell)", "E|\"entries taken on the close\"", "X|Exit 1 = next open (default), 2 = next close"],
          claim("≈602", "54% (next open), 50% (next close)", "$42 / $-66", "1 day", "86H-95U, S&P futures (A.10)")),
        m("ss_study_oops_buy", "Oops buy", "Long", vec!["F|\"the market opened below the previous day's low\"", "E|\"and then rallied back above that low ... a trade is entered at the previous day's low\"", "X|\"exited on the close of the day of entry, the next day's open or close, and the following day's close\" Exit 0 = close, 1 = next open (default: \"the optimal window for exit tends to be the next day's opening\"), 2 = next close, 3 = 2nd close"],
          claim("≈99", "70% / 69% / 56% / 49% (C / NO / NC / 2C)", "$354 / $308 / $229 / $-62", "0-2 days", "89H-95U, S&P futures (A.12)")),
        m("ss_study_oops_sell", "Oops sell", "Short", vec!["F|\"the market must open above the previous day's high\"", "E|\"and then trade back down through that high\": entered at the previous day's high", "X|Exit 0 = close, 1 = next open (default), 2 = next close, 3 = 2nd close"],
          claim("≈163", "52% / 53% / 48% / 43% (C / NO / NC / 2C)", "$51 / $119 / $-2 / $-335", "0-2 days", "89H-95U, S&P futures (A.13)")),
        m("ss_study_oops_adx_gapper_buy", "Oops (ADX Gapper) buy", "Long", vec!["F|\"Filters: Only ADX(12)>30 & w/ADX(28) trend\" (+DI(28) > -DI(28), yesterday)", "F|opened below the previous day's low", "E|\"Enter at: PD Low\"", "X|Exit 0 = close, 1 = next open (default), 2 = next close, 3 = 2nd close"],
          claim("≈13", "69% / 62% / 54% / 46% (C / NO / NC / 2C)", "$237 / $525 / $310 / $485", "0-2 days", "89H-95U, S&P futures (A.14)")),
        m("ss_study_oops_adx_gapper_sell", "Oops (ADX Gapper) sell", "Short", vec!["F|\"Filters: Only ADX(12)>30 & w/ADX(28) trend\" (-DI(28) > +DI(28))", "F|opened above the previous day's high", "E|\"Enter at: PD High\"", "X|Exit 0 = close, 1 = next open (default), 2 = next close, 3 = 2nd close"],
          claim("≈16", "69% / 50% / 56% / 50% (C / NO / NC / 2C)", "$292 / $408 / $378 / $136", "0-2 days", "89H-95U, S&P futures (A.15)")),
        m("ss_study_fresh_adx_buy", "Fresh ADX buy", "Long", vec!["F|\"periods in which the ADX is above 30 and rising. Trades are taken only in the direction of the trend.\" (ADX(14), +DI > -DI)", "E|\"buying a close that is less than the close of two days ago\" (fresh: yesterday was not such a close)", "X|Exit 2 = next close (default), 3 = 2nd, 4 = 3rd, 5 = 4th close. \"No protective stops\""],
          claim("≈61", "49% / 51% / 56% / 61% (NC / 2C / 3C / 4C)", "$110 / $48 / $119 / $354", "1-4 days", "86H-95U, S&P futures (A.16)")),
        m("ss_study_fresh_adx_sell", "Fresh ADX sell", "Short", vec!["F|\"ADX is above 30 and rising\", downtrend (-DI > +DI)", "E|\"selling a close that is greater than the close of two days ago\" (fresh)", "X|Exit 2 = next close (default), 3 = 2nd, 4 = 3rd, 5 = 4th close"],
          claim("≈34", "44% / 50% / 41% / 29% (NC / 2C / 3C / 4C)", "$-51 / $10 / $-18 / $-252", "1-4 days", "86H-95U, S&P futures (A.17)")),
        m("ss_study_gap_failure_buy", "Gap failure buy (50%)", "Long", vec!["F|\"the market opens below the previous day's low and closes in the upper 50 percent of its range ... The gap does not necessarily have to be filled\"", "E|\"Enter on: Close\"", "X|\"Exit on Next Open\""],
          claim("≈113", "62%", "$96", "overnight", "86H-95U, S&P futures (A.18)")),
        m("ss_study_gap_failure_sell", "Gap failure sell (50%)", "Short", vec!["F|\"the market opens above the previous day's high and closes in the lower 50 percent of its range\"", "E|\"Enter on: Close\"", "X|\"Exit on Next Open\""],
          claim("≈116", "62%", "$84", "overnight", "86H-95U, S&P futures (A.19)")),
        m("ss_study_channel_2period_sar", "2-period channel stop & reverse", "Long/Short", vec!["E|\"buying the highest high of the last two bars (buy stop)\"", "X|\"then exiting and going short when the market breaks below the lowest low of the last two bars (stop and reverse)\"", "X|\"This study is not intended to be an entry technique or an exit strategy!\""],
          claim("705 (351 long / 354 short)", "39% (long 43%, short 35%)", "$79 (long $256, short $-97)", "-", "86H-95U, S&P futures (A.20)")),
    ]);
    v
}

/// (family id, family name, variant label)
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ss_turtle_soup" => ("ss_turtle", "Turtle Soup (Street Smarts)", "Turtle Soup"),
        "ss_turtle_soup_plus_one" => ("ss_turtle", "Turtle Soup (Street Smarts)", "Turtle Soup Plus One"),
        "ss_anti" => ("ss_anti", "The Anti", "7 %K / 10 %D hook"),
        "ss_anti_3_10" => ("ss_anti", "The Anti", "3-10 oscillator hook"),
        "ss_anti_trigger" => ("ss_anti", "The Anti", "Trigger: stop above the prior bar"),
        "ss_adx_gapper" => ("ss_adx_gapper", "ADX Gapper", "Gap below yesterday's low"),
        "ss_adx_gapper_no_gap" => ("ss_adx_gapper", "ADX Gapper", "No gap: down open, back above yesterday's close"),
        "ss_spike_and_ledge" => ("ss_ch14", "Climax chart patterns (ch.14)", "Spike and Ledge"),
        "ss_fakeout_shakeout" => ("ss_ch14", "Climax chart patterns (ch.14)", "Fakeout-Shakeout"),
        "ss_three_little_indians" => ("ss_ch14", "Climax chart patterns (ch.14)", "Three Little Indians"),
        "ss_morning_news_reversal" => ("ss_news", "News Reversals", "Morning (8:30 report)"),
        "ss_big_picture_news_reversal" => ("ss_news", "News Reversals", "Big Picture"),
        "ss_tick_divergence" => ("ss_tick", "NYSE TICK", "Divergence"),
        "ss_tick_extreme_next_day" => ("ss_tick", "NYSE TICK", "Extreme reading, next-day reversal"),
        "ss_arms_index_5ma" => ("ss_trin", "Arms Index (TRIN) 5-day average", "NYSE"),
        "ss_arms_index_5ma_nse" => ("ss_trin", "Arms Index (TRIN) 5-day average", "NSE"),
        "ss_ad_market_buster" => ("ss_adb", "A/D Market Buster", "NYSE"),
        "ss_ad_market_buster_nse" => ("ss_adb", "A/D Market Buster", "NSE"),
        "ss_study_close_upper_90" => ("ss_moore_profile", "Moore: daily bar profiles", "Close in top 10%"),
        "ss_study_close_upper_80" => ("ss_moore_profile", "Moore: daily bar profiles", "Close in top 20%"),
        "ss_study_close_lower_10" => ("ss_moore_profile", "Moore: daily bar profiles", "Close in bottom 10%"),
        "ss_study_close_lower_20" => ("ss_moore_profile", "Moore: daily bar profiles", "Close in bottom 20%"),
        "ss_study_wr7_high_close" => ("ss_moore_profile", "Moore: daily bar profiles", "WR7, high close"),
        "ss_study_wr7_low_close" => ("ss_moore_profile", "Moore: daily bar profiles", "WR7, low close"),
        "ss_study_rocrsi_buy" => ("ss_moore_rocrsi", "Moore: ROC/RSI reports", "Buy (< 30)"),
        "ss_study_rocrsi_sell" => ("ss_moore_rocrsi", "Moore: ROC/RSI reports", "Sell (> 70)"),
        "ss_study_fresh_roc_buy" => ("ss_moore_froc", "Moore: Fresh 2-period ROC", "Buy"),
        "ss_study_fresh_roc_sell" => ("ss_moore_froc", "Moore: Fresh 2-period ROC", "Sell"),
        "ss_study_oops_buy" => ("ss_moore_oops", "Moore: Oops reports", "Buy"),
        "ss_study_oops_sell" => ("ss_moore_oops", "Moore: Oops reports", "Sell"),
        "ss_study_oops_adx_gapper_buy" => ("ss_moore_oops", "Moore: Oops reports", "ADX Gapper buy"),
        "ss_study_oops_adx_gapper_sell" => ("ss_moore_oops", "Moore: Oops reports", "ADX Gapper sell"),
        "ss_study_fresh_adx_buy" => ("ss_moore_fadx", "Moore: Fresh ADX", "Buy"),
        "ss_study_fresh_adx_sell" => ("ss_moore_fadx", "Moore: Fresh ADX", "Sell"),
        "ss_study_gap_failure_buy" => ("ss_moore_gapf", "Moore: Gap failure (50%)", "Buy"),
        "ss_study_gap_failure_sell" => ("ss_moore_gapf", "Moore: Gap failure (50%)", "Sell"),
        "ss_study_channel_2period_sar" => ("ss_moore_ch2", "Moore: 2-period channel stop & reverse", ""),
        "ss_80_20" => ("ss_80_20", "80-20's", ""),
        "ss_momentum_pinball" => ("ss_pinball", "Momentum Pinball", ""),
        "ss_2_period_roc" => ("ss_roc2", "2-Period ROC", ""),
        "ss_holy_grail" => ("ss_grail", "The Holy Grail", ""),
        "ss_whiplash" => ("ss_whiplash", "Whiplash", ""),
        "ss_three_day_unfilled_gap" => ("ss_gap3", "Three-Day Unfilled Gap Reversal", ""),
        "ss_wolfe_wave" => ("ss_wolfe", "Wolfe Waves", ""),
        "ss_id_nr4" => ("ss_idnr4", "ID/NR4", ""),
        "ss_hist_vol_crabel" => ("ss_hvc", "Historical Volatility Meets Toby Crabel", ""),
        "ss_smart_money_index" => ("ss_smi", "Smart Money Index", ""),
        _ => return None,
    })
}

// ---------------------------------------------------------------- parameters

fn p(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef {
    ParamDef { key, label, def, min, max, step }
}
fn tick() -> ParamDef {
    p("tick_pct", "1 tick = % of price", 0.01, 0.0, 0.1, 0.01)
}
fn side() -> ParamDef {
    p("side", "Side: 0 both, 1 long only, -1 short only", 0.0, -1.0, 1.0, 1.0)
}
fn trail() -> ParamDef {
    p("trail", "[Not from the chapter] trailing stop: N-bar channel (0 = off)", 2.0, 0.0, 10.0, 1.0)
}
fn mexit(def: f64, min: f64, max: f64) -> ParamDef {
    p("exit", "Exit: 0 entry-day close, 1 next open, 2 next close, 3 2nd close, 4 3rd, 5 4th", def, min, max, 1.0)
}

fn rexit() -> ParamDef {
    p("exit", "Exit: 0 close of the entry day (t+1), 1 next open, 2 next close (t+2 from the setup)", 1.0, 0.0, 2.0, 1.0)
}
fn overlap() -> ParamDef {
    p("overlap", "Count every setup independently, as Moore did (1; equity curve not meaningful) or one position at a time (0)", 0.0, 0.0, 1.0, 1.0)
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    family_of(id)?;
    let ex_mode = || p("exit_mode", "Exit: 0 the close, 1 next open, 2 carry only on a strong close (book p.88)", 2.0, 0.0, 2.0, 1.0);
    Some(match id {
        "ss_turtle_soup" => vec![p("lookback", "New N-day low/high", 20.0, 10.0, 40.0, 5.0), p("prev", "Prior extreme at least N sessions earlier", 4.0, 2.0, 8.0, 1.0),
            p("entry_ticks", "Entry: ticks beyond the prior extreme", 5.0, 1.0, 10.0, 1.0), p("stop_ticks", "Stop: ticks beyond today's extreme", 1.0, 0.0, 5.0, 1.0),
            p("half_bar", "[TS code, not the chapter] half off on the close of bar N (0 = off, book: trailing stop only)", 0.0, 0.0, 10.0, 1.0), p("fill_rule", "[Not from the book] bar that opened beyond the level: 0 skip (unbiased), 1 fill (pessimistic), 2 fill if it closed back (biased)", 0.0, 0.0, 2.0, 1.0), trail(), side(), tick()],
        "ss_turtle_soup_plus_one" => vec![p("lookback", "New N-day low/high", 20.0, 10.0, 40.0, 5.0), p("prev", "Prior extreme at least N sessions earlier", 3.0, 2.0, 8.0, 1.0),
            p("stop_ticks", "Stop: ticks beyond the day-1/day-2 extreme", 1.0, 0.0, 5.0, 1.0), p("half_bar", "Half off on the close of bar N (book 2-6; 0 = off)", 6.0, 0.0, 10.0, 1.0), trail(), side(), tick()],
        "ss_80_20" => vec![p("open_pct", "Opened in the top/bottom % of range", 20.0, 5.0, 40.0, 5.0), p("close_pct", "Closed in the bottom/top % of range", 20.0, 5.0, 40.0, 5.0),
            p("pen_ticks", "Today trades N ticks beyond yesterday's extreme", 10.0, 0.0, 30.0, 5.0), p("fill_rule", "[Not from the book] bar that opened beyond the level: 0 skip (unbiased), 1 fill (pessimistic), 2 fill if it closed back (biased)", 0.0, 0.0, 2.0, 1.0), side(), tick()],
        "ss_momentum_pinball" => vec![p("rsi_len", "LBR/RSI period", 3.0, 2.0, 6.0, 1.0), p("buy_level", "Buy setup: LBR/RSI below", 30.0, 10.0, 40.0, 5.0), p("sell_level", "Sell setup: LBR/RSI above", 70.0, 60.0, 90.0, 5.0), side(), tick()],
        "ss_2_period_roc" => vec![p("roc_len", "ROC period", 2.0, 1.0, 5.0, 1.0), p("hold", "Exit on the close N days later", 1.0, 1.0, 5.0, 1.0), side()],
        "ss_anti" => vec![p("k_len", "Fast line: %K period", 7.0, 5.0, 14.0, 1.0), p("k_smooth", "Fast line: smoothing", 4.0, 1.0, 5.0, 1.0), p("d_len", "Slow line: %D (stochastic) period", 10.0, 5.0, 20.0, 1.0),
            p("trend_bars", "Slow line rising N bars", 2.0, 1.0, 5.0, 1.0), p("hold", "Out on the close of day N", 4.0, 2.0, 8.0, 1.0), side(), tick()],
        "ss_anti_3_10" => vec![p("fast_ma", "Oscillator fast SMA", 3.0, 2.0, 5.0, 1.0), p("slow_ma", "Oscillator slow SMA", 10.0, 6.0, 20.0, 1.0), p("signal_ma", "Signal SMA of the oscillator", 16.0, 8.0, 25.0, 1.0),
            p("trend_bars", "Slow line rising N bars", 2.0, 1.0, 5.0, 1.0), p("hold", "Out on the close of day N", 4.0, 2.0, 8.0, 1.0), side(), tick()],
        "ss_anti_trigger" => vec![p("k_len", "Fast line: %K period", 7.0, 5.0, 14.0, 1.0), p("k_smooth", "Fast line: smoothing", 4.0, 1.0, 5.0, 1.0), p("d_len", "Slow line: %D (stochastic) period", 10.0, 5.0, 20.0, 1.0),
            p("opp_days", "Opposing slopes for N days", 3.0, 2.0, 6.0, 1.0), p("hold", "Out on the close of day N", 4.0, 2.0, 8.0, 1.0), side(), tick()],
        "ss_holy_grail" => vec![p("adx_len", "ADX period", 14.0, 7.0, 21.0, 1.0), p("adx_min", "ADX above", 30.0, 20.0, 40.0, 5.0), p("ema_len", "EMA period", 20.0, 10.0, 50.0, 5.0),
            p("swing_n", "[Not from the book] swing high = highest high of N bars before the touch", 20.0, 5.0, 40.0, 5.0),
            p("adx_rising", "Rule 1 \"and rising\": ADX above the level and rising on some bar of the last N bars up to the touch (0 = off; N = 10 not from the book)", 10.0, 0.0, 30.0, 1.0),
            p("reentry", "Rule 5: after a stop-out, re-enter on a stop at the original entry price (1 = book, 0 = off)", 1.0, 0.0, 1.0, 1.0), trail(), side(), tick()],
        "ss_adx_gapper" => vec![p("adx_len", "ADX period", 12.0, 7.0, 21.0, 1.0), p("di_len", "+DI/-DI period", 28.0, 10.0, 40.0, 2.0), p("adx_min", "ADX above", 30.0, 20.0, 40.0, 5.0),
            p("entry_ticks", "Entry: ticks beyond yesterday's extreme", 0.0, 0.0, 5.0, 1.0), ex_mode(), side(), tick()],
        "ss_adx_gapper_no_gap" => vec![p("adx_len", "ADX period", 12.0, 7.0, 21.0, 1.0), p("di_len", "+DI/-DI period", 28.0, 10.0, 40.0, 2.0), p("adx_min", "ADX above", 30.0, 20.0, 40.0, 5.0), ex_mode(), tick()],
        "ss_whiplash" => vec![p("close_loc", "Close in the top fraction of range (0.5 = top 50%)", 0.5, 0.2, 0.9, 0.1),
            p("exit_mode", "Exit: 0 next open, 1 book (out if it opens at a loss, else breakeven stop)", 1.0, 0.0, 1.0, 1.0), p("hold", "[Not from the book] mode 1: out on the close of day N", 1.0, 1.0, 5.0, 1.0), side()],
        "ss_three_day_unfilled_gap" => vec![p("window", "Buy stop live for N sessions", 3.0, 1.0, 5.0, 1.0), p("entry_ticks", "Entry: ticks beyond the gap-day extreme", 1.0, 0.0, 5.0, 1.0),
            p("max_risk_pct", "Cap the risk at % of entry (0 = gap-day extreme)", 0.0, 0.0, 6.0, 0.5), trail(), side(), tick()],
        "ss_id_nr4" => vec![p("nr", "Narrowest range of N days", 4.0, 3.0, 8.0, 1.0), p("entry_ticks", "Entry: ticks beyond the setup bar", 1.0, 0.0, 5.0, 1.0),
            p("time_days", "Out MOC if not profitable N days after the setup (book p.155: 2; ELA S0014: 3)", 2.0, 1.0, 5.0, 1.0), trail(), side(), tick()],
        "ss_hist_vol_crabel" => vec![p("hv_short", "Short HV days", 6.0, 3.0, 10.0, 1.0), p("hv_long", "Long HV days", 100.0, 50.0, 150.0, 10.0), p("hv_ratio", "HV ratio below", 0.5, 0.3, 0.8, 0.05),
            p("nr", "NR days", 4.0, 3.0, 8.0, 1.0), p("entry_ticks", "Entry: ticks beyond day one", 1.0, 0.0, 5.0, 1.0), p("max_days", "Out on the close of day N (Larry: 1-4)", 4.0, 1.0, 8.0, 1.0), trail(), side(), tick()],
        "ss_morning_news_reversal" => vec![p("pen_ticks", "Open N ticks beyond yesterday's extreme", 4.0, 1.0, 20.0, 1.0), p("entry_ticks", "Entry stop N ticks back inside", 2.0, 1.0, 5.0, 1.0), side(), tick()],
        "ss_smart_money_index" => vec![p("threshold_pct", "Day's reading beyond ± % (20 Dow pts ≈ 0.4%)", 0.4, 0.1, 1.0, 0.1), side()],
        "ss_tick_extreme_next_day" => vec![p("level", "TICK beyond ±", 400.0, 300.0, 1000.0, 50.0), side()],
        "ss_arms_index_5ma" | "ss_arms_index_5ma_nse" => vec![p("avg_len", "TRIN moving average days", 5.0, 3.0, 10.0, 1.0), p("buy_level", "Buy: average above", 1.2, 1.0, 1.6, 0.05),
            p("sell_level", "Sell: average below", 0.8, 0.5, 1.0, 0.05), p("hold", "Out on the close N days later", 3.0, 1.0, 5.0, 1.0), side()],
        "ss_ad_market_buster" | "ss_ad_market_buster_nse" => vec![p("ad_avg", "A-D average days", 3.0, 2.0, 5.0, 1.0), p("rsi_len", "RSI period", 3.0, 2.0, 5.0, 1.0), p("ob", "Overbought", 70.0, 60.0, 85.0, 5.0),
            p("os", "Oversold", 30.0, 15.0, 40.0, 5.0), p("thrust", "Max one-day RSI change", 40.0, 20.0, 60.0, 5.0), p("ema_len", "Trend EMA", 100.0, 50.0, 200.0, 10.0),
            p("hold", "Exit on the close N days after entry", 3.0, 1.0, 5.0, 1.0), p("stop_pct", "Stop %", 2.0, 1.0, 4.0, 0.5), p("entry_mode", "[Not from the book] entry: 0 signal close, 1 next open", 1.0, 0.0, 1.0, 1.0), p("cross", "Uptick must also cross back over 30 (downtick under 70): 0 no (book text, default), 1 yes (reading fitted to p.227's win% / PF)", 0.0, 0.0, 1.0, 1.0), side()],
        "ss_study_close_upper_90" | "ss_study_close_lower_10" => vec![p("pct", "Close within % of the range extreme", 10.0, 5.0, 30.0, 5.0), mexit(1.0, 1.0, 2.0), overlap()],
        "ss_study_close_upper_80" | "ss_study_close_lower_20" => vec![p("pct", "Close within % of the range extreme", 20.0, 5.0, 30.0, 5.0), mexit(1.0, 1.0, 2.0), overlap()],
        "ss_study_wr7_high_close" | "ss_study_wr7_low_close" => vec![p("wr", "Widest range of N days", 7.0, 4.0, 10.0, 1.0), p("close_def", "High/low close: 0 = upper/lower half of range, 1 = vs yesterday's close", 0.0, 0.0, 1.0, 1.0), mexit(1.0, 1.0, 2.0), overlap()],
        "ss_study_rocrsi_buy" => vec![p("rsi_len", "LBR/RSI period", 3.0, 2.0, 6.0, 1.0), p("level", "LBR/RSI below", 30.0, 10.0, 40.0, 5.0), rexit(), overlap()],
        "ss_study_rocrsi_sell" => vec![p("rsi_len", "LBR/RSI period", 3.0, 2.0, 6.0, 1.0), p("level", "LBR/RSI above", 70.0, 60.0, 90.0, 5.0), rexit(), overlap()],
        "ss_study_fresh_roc_buy" | "ss_study_fresh_roc_sell" => vec![p("roc_len", "ROC period", 2.0, 1.0, 5.0, 1.0), mexit(1.0, 1.0, 2.0), overlap()],
        "ss_study_oops_buy" | "ss_study_oops_sell" => vec![mexit(1.0, 0.0, 3.0), overlap()],
        "ss_study_oops_adx_gapper_buy" | "ss_study_oops_adx_gapper_sell" => vec![p("adx_len", "ADX period", 12.0, 7.0, 21.0, 1.0), p("di_len", "Trend +DI/-DI period", 28.0, 10.0, 40.0, 2.0), p("adx_min", "ADX above", 30.0, 20.0, 40.0, 5.0), mexit(1.0, 0.0, 3.0), overlap()],
        "ss_study_fresh_adx_buy" | "ss_study_fresh_adx_sell" => vec![p("adx_len", "ADX period (not stated; 14 as in A.11)", 14.0, 7.0, 21.0, 1.0), p("adx_min", "ADX above", 30.0, 20.0, 40.0, 5.0),
            p("fresh", "Fresh only: 1 = yesterday was not a setup", 1.0, 0.0, 1.0, 1.0), mexit(2.0, 2.0, 5.0), overlap()],
        "ss_study_gap_failure_buy" | "ss_study_gap_failure_sell" => vec![p("loc", "Close in the favourable % of range", 50.0, 30.0, 80.0, 10.0), mexit(1.0, 1.0, 2.0), overlap()],
        "ss_study_channel_2period_sar" => vec![p("ch", "Channel bars", 2.0, 1.0, 10.0, 1.0)],
        _ => vec![],
    })
}

// ---------------------------------------------------------------- rules

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, prm: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| prm.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let u0 = |k: &str| g(k).round().max(0.0) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let to = to.min(n.saturating_sub(1));
    let tick = |i: usize| c[i] * g("tick_pct") / 100.0;
    let lo_ok = || g("side") >= 0.0;
    let sh_ok = || g("side") <= 0.0;
    let none = |_: usize, _: &Pos| -> Option<Act> { None };
    let day_close = |j: usize, p: &Pos| (j == p.i).then(|| Act::Close("day trade: out on the close".into()));
    let go = |v: Vec<Ord>, trail: usize, half_bar: usize, rule: Rule| sim(x, v, &Ex { trail, half_bar, rule, gate: None, reentry: None }, from, to);
    let mut v: Vec<Ord> = Vec::new();

    let trades = match id {
        "ss_turtle_soup" => {
            let (nb, prev, et, st, fr) = (u("lookback"), u("prev"), g("entry_ticks"), g("stop_ticks"), u0("fill_rule") as u8);
            for j in nb..=to {
                let (mut lo, mut kl, mut hi, mut kh) = (f64::INFINITY, 0, f64::NEG_INFINITY, 0);
                for m in j - nb..j {
                    if l[m] <= lo { lo = l[m]; kl = m; }
                    if h[m] >= hi { hi = h[m]; kh = m; }
                }
                if lo_ok() && l[j] < lo && j - kl >= prev {
                    v.push(Ord { stop: l[j] - st * tick(j), ..ord(1.0, Fill::StopAfter(lo + et * tick(j), fr), j, j, j) });
                }
                if sh_ok() && h[j] > hi && j - kh >= prev {
                    v.push(Ord { stop: h[j] + st * tick(j), ..ord(-1.0, Fill::StopAfter(hi - et * tick(j), fr), j, j, j) });
                }
            }
            go(v, u0("trail"), u0("half_bar"), &none)
        }
        "ss_turtle_soup_plus_one" => {
            let (nb, prev, st) = (u("lookback"), u("prev"), g("stop_ticks"));
            for j in nb + 1..=to {
                let d = j - 1;
                let (mut lo, mut kl, mut hi, mut kh) = (f64::INFINITY, 0, f64::NEG_INFINITY, 0);
                for m in d - nb..d {
                    if l[m] <= lo { lo = l[m]; kl = m; }
                    if h[m] >= hi { hi = h[m]; kh = m; }
                }
                if lo_ok() && l[d] < lo && d - kl >= prev && c[d] <= lo {
                    v.push(Ord { stop: l[d] - st * tick(d), ..ord(1.0, Fill::Stop(lo), d, j, j) });
                }
                if sh_ok() && h[d] > hi && d - kh >= prev && c[d] >= hi {
                    v.push(Ord { stop: h[d] + st * tick(d), ..ord(-1.0, Fill::Stop(hi), d, j, j) });
                }
            }
            let rule = |j: usize, p: &Pos| (j == p.i).then(|| Act::SetStop(if p.dir > 0.0 { l[p.setup].min(l[j]) - st * tick(j) } else { h[p.setup].max(h[j]) + st * tick(j) }));
            go(v, u0("trail"), u0("half_bar"), &rule)
        }
        "ss_80_20" => {
            let (op, cp, pen, fr) = (g("open_pct") / 100.0, g("close_pct") / 100.0, g("pen_ticks"), u0("fill_rule") as u8);
            for j in 1..=to {
                let d = j - 1;
                let r = h[d] - l[d];
                if r <= 0.0 { continue; }
                let (ol, cl) = ((o[d] - l[d]) / r, (c[d] - l[d]) / r);
                if lo_ok() && ol >= 1.0 - op && cl <= cp && l[j] <= l[d] - pen * tick(j) {
                    v.push(Ord { stop: l[j], ..ord(1.0, Fill::StopAfter(l[d], fr), d, j, j) });
                }
                if sh_ok() && ol <= op && cl >= 1.0 - cp && h[j] >= h[d] + pen * tick(j) {
                    v.push(Ord { stop: h[j], ..ord(-1.0, Fill::StopAfter(h[d], fr), d, j, j) });
                }
            }
            go(v, 0, 0, &day_close)
        }
        "ss_momentum_pinball" => {
            let (Some(fh), Some(fl)) = (x.series("OPEN60_HIGH"), x.series("OPEN60_LOW")) else { return Some(vec![]) };
            let (lb, bl, sl) = (lbr_rsi(c, u("rsi_len")), g("buy_level"), g("sell_level"));
            for j in 1..=to {
                let d = j - 1;
                if !(fh[j].is_finite() && fl[j].is_finite()) { continue; }
                if lo_ok() && lb[d] < bl {
                    v.push(Ord { stop: fl[j], ..ord(1.0, Fill::Stop(fh[j] + tick(j)), d, j, j) });
                }
                if sh_ok() && lb[d] > sl {
                    v.push(Ord { stop: fh[j], ..ord(-1.0, Fill::Stop(fl[j] - tick(j)), d, j, j) });
                }
            }
            let rule = |j: usize, p: &Pos| {
                if j > p.i {
                    Some(Act::Close("day 3: out by the close".into()))
                } else if p.dir * (c[j] - p.px) <= 0.0 {
                    Some(Act::Close("no profit on the close: out (no losers overnight)".into()))
                } else {
                    Some(Act::SetTarget(if p.dir > 0.0 { h[j] + tick(j) } else { l[j] - tick(j) }))
                }
            };
            go(v, 0, 0, &rule)
        }
        "ss_2_period_roc" => {
            let (st, hold) = (roc_state(c, u("roc_len")), u("hold"));
            for i in 1..=to {
                if st[i] != 0.0 && st[i - 1] != 0.0 && st[i] != st[i - 1] && (if st[i] > 0.0 { lo_ok() } else { sh_ok() }) {
                    v.push(ord(st[i], Fill::Close, i, i, i));
                }
            }
            let rule = move |j: usize, p: &Pos| (j - p.i >= hold).then(|| Act::Close(format!("{hold} day(s) held: out on the close")));
            go(v, 0, 0, &rule)
        }
        "ss_anti" | "ss_anti_3_10" | "ss_anti_trigger" => {
            let (fast, slow) = if id == "ss_anti_3_10" {
                let (a, b) = (sma_w(c, u("fast_ma")), sma_w(c, u("slow_ma")));
                let f: Vec<f64> = a.iter().zip(&b).map(|(a, b)| a - b).collect();
                let s = sma_w(&f, u("signal_ma"));
                (f, s)
            } else {
                (sma_w(&fast_k(x, u("k_len")), u("k_smooth")), sma_w(&sma_w(&fast_k(x, u("d_len")), 3), 3))
            };
            if id == "ss_anti_trigger" {
                let od = u("opp_days");
                let ol = streak(n, |i| i > 0 && slow[i] > slow[i - 1] && fast[i] < fast[i - 1]);
                let os = streak(n, |i| i > 0 && slow[i] < slow[i - 1] && fast[i] > fast[i - 1]);
                let (mut al, mut as_) = (false, false);
                for i in 1..=to {
                    al = ol[i] >= od || (al && fast[i] <= fast[i - 1]);
                    as_ = os[i] >= od || (as_ && fast[i] >= fast[i - 1]);
                    if al && lo_ok() { v.push(ord(1.0, Fill::Stop(h[i] + tick(i)), i, i + 1, i + 1)); }
                    if as_ && sh_ok() { v.push(ord(-1.0, Fill::Stop(l[i] - tick(i)), i, i + 1, i + 1)); }
                }
            } else {
                let tb = u("trend_bars");
                let rising = |i: usize, d: f64| i >= tb && (0..tb).all(|k| d * (slow[i - k] - slow[i - k - 1]) > 0.0);
                for i in 2..=to {
                    if lo_ok() && fast[i] > fast[i - 1] && fast[i - 1] <= fast[i - 2] && rising(i, 1.0) { v.push(ord(1.0, Fill::Open, i, i + 1, i + 1)); }
                    if sh_ok() && fast[i] < fast[i - 1] && fast[i - 1] >= fast[i - 2] && rising(i, -1.0) { v.push(ord(-1.0, Fill::Open, i, i + 1, i + 1)); }
                }
            }
            let hold = u("hold");
            let rule = move |j: usize, p: &Pos| {
                if j + 1 - p.i >= hold {
                    Some(Act::Close(format!("day {hold}: out on the close")))
                } else if j == p.i {
                    Some(Act::SetStop(if p.dir > 0.0 { l[j] - tick(j) } else { h[j] + tick(j) }))
                } else {
                    None
                }
            };
            go(v, 0, 0, &rule)
        }
        "ss_holy_grail" => {
            let (pdi, mdi, adx) = dmi3(x.r, u("adx_len"));
            let (em, am, sw) = (ema(c, u("ema_len")), g("adx_min"), u("swing_n"));
            let ar = u0("adx_rising");
            for j in sw.max(1)..=to {
                if !(adx[j] > am) { continue; }
                if ar > 0 && !(j.saturating_sub(ar).max(1)..=j).any(|m| adx[m] > am && adx[m] > adx[m - 1]) { continue; }
                if lo_ok() && pdi[j] > mdi[j] && c[j - 1] > em[j - 1] && l[j] <= em[j] {
                    let (lv, tg) = (h[j] + tick(j), max_of(&h[j - sw..j]));
                    v.push(Ord { stop: l[j] - tick(j), target: if tg > lv { tg } else { f64::NAN }, ..ord(1.0, Fill::Stop(lv), j, j + 1, j + 1) });
                }
                if sh_ok() && mdi[j] > pdi[j] && c[j - 1] < em[j - 1] && h[j] >= em[j] {
                    let (lv, tg) = (l[j] - tick(j), min_of(&l[j - sw..j]));
                    v.push(Ord { stop: h[j] + tick(j), target: if tg < lv { tg } else { f64::NAN }, ..ord(-1.0, Fill::Stop(lv), j, j + 1, j + 1) });
                }
            }
            // rule 6: after a winner, the ADX must turn up again above the threshold before the next setup
            let gate = |od: &Ord, last: Option<&Trade>| match last {
                Some(t) if t.ret(0.0) > 0.0 => (t.exit_i + 1..=od.setup).any(|m| m >= 2 && adx[m] > am && adx[m] > adx[m - 1] && adx[m - 1] <= adx[m - 2]),
                _ => true,
            };
            // rule 5: after a losing stop-out a stop at the original entry price stays live [Book silent: order life]
            // until it fills, a new setup (touch) appears, or the ADX is no longer above the level
            let mut is_setup = vec![false; n];
            v.iter().for_each(|o| is_setup[o.setup] = true);
            let live = |r: &Ord, j: usize| adx[j - 1] > am && !(r.from - 1..j).any(|m| is_setup[m]);
            let reentry: Option<&dyn Fn(&Ord, usize) -> bool> = if g("reentry") >= 0.5 { Some(&live) } else { None };
            sim(x, v, &Ex { trail: u0("trail"), half_bar: 0, rule: &none, gate: Some(&gate), reentry }, from, to)
        }
        "ss_adx_gapper" | "ss_adx_gapper_no_gap" => {
            let (pdi, mdi, _) = dmi3(x.r, u("di_len"));
            let (_, _, adx) = dmi3(x.r, u("adx_len"));
            let am = g("adx_min");
            for j in 1..=to {
                let d = j - 1;
                if !(adx[d] > am) { continue; }
                if id == "ss_adx_gapper" {
                    let et = g("entry_ticks");
                    if lo_ok() && pdi[d] > mdi[d] && o[j] < l[d] { v.push(Ord { stop: l[j], ..ord(1.0, Fill::Stop(l[d] + et * tick(j)), d, j, j) }); }
                    if sh_ok() && mdi[d] > pdi[d] && o[j] > h[d] { v.push(Ord { stop: h[j], ..ord(-1.0, Fill::Stop(h[d] - et * tick(j)), d, j, j) }); }
                } else if pdi[d] > mdi[d] && o[j] < c[d] {
                    v.push(Ord { stop: l[j], ..ord(1.0, Fill::Stop(c[d] + tick(j)), d, j, j) });
                }
            }
            let mode = u0("exit_mode");
            let rule = move |j: usize, p: &Pos| {
                if j > p.i { return None; }
                let loc = x.close_loc(j);
                let strong = if p.dir > 0.0 { loc >= 0.5 } else { loc <= 0.5 };
                match mode {
                    0 => Some(Act::Close("out on the close".into())),
                    2 if !strong => Some(Act::Close("weak close: out on the close".into())),
                    _ => Some(Act::NextOpen("carried overnight: out on the next open".into())),
                }
            };
            go(v, 0, 0, &rule)
        }
        "ss_whiplash" => {
            let loc = g("close_loc");
            for i in 1..=to {
                let cl = x.close_loc(i);
                if lo_ok() && o[i] < l[i - 1] && c[i] > o[i] && cl >= loc { v.push(ord(1.0, Fill::Close, i, i, i)); }
                if sh_ok() && o[i] > h[i - 1] && c[i] < o[i] && cl <= 1.0 - loc { v.push(ord(-1.0, Fill::Close, i, i, i)); }
            }
            let (mode, hold) = (u0("exit_mode"), u("hold"));
            let rule = move |j: usize, p: &Pos| {
                if mode == 0 {
                    (j == p.i).then(|| Act::NextOpen("out on the next open".into()))
                } else if j - p.i >= hold {
                    Some(Act::Close(format!("{hold} day(s): out on the close")))
                } else if j == p.i && j < to {
                    // orders for tomorrow's open, placed on the entry close: a loss at the open -> out on the open;
                    // a profit -> breakeven stop; opening exactly at the entry is neither (rules 4/5): hold, no stop
                    let gain = p.dir * (o[j + 1] - p.px);
                    if gain < 0.0 { Some(Act::NextOpen("opened at a loss: out on the open".into())) } else if gain > 0.0 { Some(Act::SetStop(p.px)) } else { None }
                } else {
                    None
                }
            };
            go(v, 0, 0, &rule)
        }
        "ss_three_day_unfilled_gap" => {
            let (wd, et, mr) = (u("window"), g("entry_ticks"), g("max_risk_pct") / 100.0);
            for d in 1..=to {
                if lo_ok() && h[d] < l[d - 1] {
                    let lv = h[d] + et * tick(d);
                    v.push(Ord { stop: if mr > 0.0 { l[d].max(lv * (1.0 - mr)) } else { l[d] }, ..ord(1.0, Fill::Stop(lv), d, d + 1, d + wd) });
                }
                if sh_ok() && l[d] > h[d - 1] {
                    let lv = l[d] - et * tick(d);
                    v.push(Ord { stop: if mr > 0.0 { h[d].min(lv * (1.0 + mr)) } else { h[d] }, ..ord(-1.0, Fill::Stop(lv), d, d + 1, d + wd) });
                }
            }
            go(v, u0("trail"), 0, &none)
        }
        "ss_id_nr4" | "ss_hist_vol_crabel" => {
            let (nr, et) = (u("nr"), g("entry_ticks"));
            let ratio: Vec<f64> = if id == "ss_hist_vol_crabel" {
                let (a, b) = (hv_ddof(c, u("hv_short"), 1.0), hv_ddof(c, u("hv_long"), 1.0));
                a.iter().zip(&b).map(|(a, b)| a / b).collect()
            } else {
                vec![]
            };
            for s in nr.max(2)..=to {
                let inside = h[s] < h[s - 1] && l[s] > l[s - 1];
                let narrow = (s + 1 - nr..s).all(|k| h[s] - l[s] < h[k] - l[k]);
                let ok = if ratio.is_empty() { inside && narrow } else { ratio[s] < g("hv_ratio") && (inside || narrow) };
                if !ok { continue; }
                let (bl, sl) = (h[s] + et * tick(s), l[s] - et * tick(s));
                let both = lo_ok() && sh_ok();
                if lo_ok() { v.push(Ord { stop: sl, rev: if both { sl } else { f64::NAN }, ..ord(1.0, Fill::Stop(bl), s, s + 1, s + 1) }); }
                if sh_ok() { v.push(Ord { stop: bl, rev: if both { bl } else { f64::NAN }, ..ord(-1.0, Fill::Stop(sl), s, s + 1, s + 1) }); }
            }
            if id == "ss_id_nr4" {
                let td = u("time_days");
                let rule = move |j: usize, p: &Pos| (j == p.setup + td && p.dir * (c[j] - p.px) <= 0.0).then(|| Act::Close(format!("not profitable {td} days after the setup: out MOC")));
                go(v, u0("trail"), 0, &rule)
            } else {
                let md = u("max_days");
                let rule = move |j: usize, p: &Pos| (j + 1 - p.i >= md).then(|| Act::Close(format!("day {md}: out on the close")));
                go(v, u0("trail"), 0, &rule)
            }
        }
        "ss_morning_news_reversal" => {
            let Some(nw) = x.series("NEWS_DAY") else { return Some(vec![]) };
            let (pen, et) = (g("pen_ticks"), g("entry_ticks"));
            for j in 1..=to {
                if !(nw[j] > 0.5) { continue; }
                let d = j - 1;
                if sh_ok() && o[j] >= h[d] + pen * tick(j) { v.push(Ord { stop: h[j] + tick(j), ..ord(-1.0, Fill::Stop(h[d] - et * tick(j)), d, j, j) }); }
                if lo_ok() && o[j] <= l[d] - pen * tick(j) { v.push(Ord { stop: l[j] - tick(j), ..ord(1.0, Fill::Stop(l[d] + et * tick(j)), d, j, j) }); }
            }
            go(v, 0, 0, &day_close)
        }
        "ss_smart_money_index" => {
            let (Some(fc), Some(lh)) = (x.series("OPEN60_CLOSE"), x.series("LAST60_OPEN")) else { return Some(vec![]) };
            let thr = g("threshold_pct");
            for i in 1..=to {
                let s = (-(fc[i] - c[i - 1]) + (c[i] - lh[i])) / c[i - 1] * 100.0;
                if s > thr && lo_ok() { v.push(ord(1.0, Fill::Open, i, i + 1, i + 1)); }
                if s < -thr && sh_ok() { v.push(ord(-1.0, Fill::Open, i, i + 1, i + 1)); }
            }
            go(v, 0, 0, &day_close)
        }
        "ss_tick_extreme_next_day" => {
            let (Some(th), Some(tl)) = (x.series("NYSE_TICK.High"), x.series("NYSE_TICK.Low")) else { return Some(vec![]) };
            let lv = g("level");
            for i in 0..=to {
                let (up, dn) = (th[i] > lv, tl[i] < -lv);
                if up && !dn && sh_ok() { v.push(ord(-1.0, Fill::Open, i, i + 1, i + 1)); }
                if dn && !up && lo_ok() { v.push(ord(1.0, Fill::Open, i, i + 1, i + 1)); }
            }
            go(v, 0, 0, &day_close)
        }
        "ss_arms_index_5ma" | "ss_arms_index_5ma_nse" | "ss_ad_market_buster" | "ss_ad_market_buster_nse" => {
            let pre = if id.ends_with("_nse") { "NSE_ADV_DEC" } else { "NYSE_ADV_DEC" };
            let get = |k: &str| x.series(&format!("{pre}.{k}")).map(ffill_inner);
            let (Some(a), Some(dc)) = (get("Advances"), get("Declines")) else { return Some(vec![]) };
            let hold = u("hold");
            if id.starts_with("ss_arms") {
                let (Some(av), Some(dv)) = (get("AdvVolume"), get("DecVolume")) else { return Some(vec![]) };
                let trin: Vec<f64> = (0..n).map(|i| (a[i] / dc[i]) / (av[i] / dv[i])).collect();
                let ma = sma_w(&trin, u("avg_len"));
                let (b, s) = (g("buy_level"), g("sell_level"));
                for i in 1..=to {
                    if lo_ok() && ma[i] > b && ma[i - 1] <= b { v.push(ord(1.0, Fill::Close, i, i, i)); }
                    if sh_ok() && ma[i] < s && ma[i - 1] >= s { v.push(ord(-1.0, Fill::Close, i, i, i)); }
                }
            } else {
                let ad: Vec<f64> = (0..n).map(|i| a[i] - dc[i]).collect();
                let r = rsi_nan(&sma_w(&ad, u("ad_avg")), u("rsi_len"));
                let em = ema(c, u("ema_len"));
                let (ob, os, th, sp, nxt, cr) = (g("ob"), g("os"), g("thrust"), g("stop_pct") / 100.0, u0("entry_mode") == 1, u0("cross") == 1);
                for i in 1..=to {
                    let d = if lo_ok() && r[i - 1] < os && r[i] > r[i - 1] && r[i] - r[i - 1] < th && c[i] > em[i] && (!cr || r[i] >= os) {
                        1.0
                    } else if sh_ok() && r[i - 1] > ob && r[i] < r[i - 1] && r[i - 1] - r[i] < th && c[i] < em[i] && (!cr || r[i] <= ob) {
                        -1.0
                    } else {
                        continue;
                    };
                    let od = if nxt { ord(d, Fill::Open, i, i + 1, i + 1) } else { ord(d, Fill::Close, i, i, i) };
                    v.push(Ord { stop_pct: sp, ..od });
                }
            }
            let rule = move |j: usize, p: &Pos| (j - p.i >= hold).then(|| Act::Close(format!("{hold} days: out on the close")));
            go(v, 0, 0, &rule)
        }
        "ss_study_channel_2period_sar" => channel_sar(x, u("ch"), from, to),
        _ if id.starts_with("ss_study_") => {
            let buy = id.ends_with("_buy") || id.contains("upper") || id.ends_with("high_close");
            let dir = if buy { 1.0 } else { -1.0 };
            if id.contains("_close_") {
                let t = g("pct") / 100.0;
                for i in 0..=to {
                    let cl = x.close_loc(i);
                    if if buy { cl >= 1.0 - t } else { cl <= t } { v.push(ord(dir, Fill::Close, i, i, i)); }
                }
            } else if id.contains("wr7") {
                let (wr, cd) = (u("wr"), u0("close_def"));
                for i in wr..=to {
                    let widest = (i + 1 - wr..i).all(|k| h[i] - l[i] >= h[k] - l[k]);
                    let cl = if cd == 0 { if buy { x.close_loc(i) >= 0.5 } else { x.close_loc(i) < 0.5 } } else if buy { c[i] > c[i - 1] } else { c[i] < c[i - 1] };
                    if widest && cl { v.push(ord(dir, Fill::Close, i, i, i)); }
                }
            } else if id.contains("rocrsi") {
                let (lb, lv) = (lbr_rsi(c, u("rsi_len")), g("level"));
                for i in 0..=to {
                    if if buy { lb[i] < lv } else { lb[i] > lv } { v.push(ord(dir, Fill::Open, i, i + 1, i + 1)); }
                }
            } else if id.contains("fresh_roc") {
                let st = roc_state(c, u("roc_len"));
                for i in 1..=to {
                    if st[i] == dir && st[i - 1] == -dir { v.push(ord(dir, Fill::Close, i, i, i)); }
                }
            } else if id.contains("oops") {
                let filt: Option<(Vec<f64>, Vec<f64>, Vec<f64>, f64)> = id.contains("adx").then(|| {
                    let (pd, md, _) = dmi3(x.r, u("di_len"));
                    (pd, md, dmi3(x.r, u("adx_len")).2, g("adx_min"))
                });
                for j in 1..=to {
                    let d = j - 1;
                    if let Some((pd, md, adx, am)) = &filt {
                        if !(adx[d] > *am && dir * (pd[d] - md[d]) > 0.0) { continue; }
                    }
                    if buy && o[j] < l[d] && h[j] >= l[d] { v.push(ord(1.0, Fill::Stop(l[d]), d, j, j)); }
                    if !buy && o[j] > h[d] && l[j] <= h[d] { v.push(ord(-1.0, Fill::Stop(h[d]), d, j, j)); }
                }
            } else if id.contains("fresh_adx") {
                let (pd, md, adx) = dmi3(x.r, u("adx_len"));
                let (am, fresh) = (g("adx_min"), u0("fresh") == 1);
                let set = |i: usize| dir * (c[i - 2] - c[i]) > 0.0;
                for i in 3..=to {
                    if adx[i] > am && adx[i] > adx[i - 1] && dir * (pd[i] - md[i]) > 0.0 && set(i) && !(fresh && set(i - 1)) { v.push(ord(dir, Fill::Close, i, i, i)); }
                }
            } else if id.contains("gap_failure") {
                let t = g("loc") / 100.0;
                for i in 1..=to {
                    let ok = if buy { o[i] < l[i - 1] && x.close_loc(i) >= 1.0 - t } else { o[i] > h[i - 1] && x.close_loc(i) <= t };
                    if ok { v.push(ord(dir, Fill::Close, i, i, i)); }
                }
            }
            let rule = moore_exit(u0("exit"));
            if u0("overlap") == 1 {
                // every setup on its own (Moore): each order simulated alone over the few bars its exit needs (max 5)
                let mut all: Vec<Trade> = v.into_iter().flat_map(|od| go_alone(x, od, &rule, to)).collect();
                all.sort_by_key(|t| (t.legs[0].i, t.exit_i));
                all
            } else {
                go(v, 0, 0, &rule)
            }
        }
        _ => Vec::new(), // listed but not testable
    };
    Some(trades)
}

// ---------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    fn row(i: usize, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: format!("2020-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }
    }

    #[test]
    fn holy_grail_rule5_reentry_at_the_original_entry_price() {
        // buy stop 101 on bar 1 fills; bar 2 hits the 99 stop (loss); bar 3 does not reach 101; bar 4 re-enters at 101
        let r = vec![row(0, 100.0, 100.5, 99.5, 100.0), row(1, 100.0, 102.0, 100.0, 101.5), row(2, 101.0, 101.5, 98.0, 98.5),
                     row(3, 98.5, 100.0, 98.0, 99.0), row(4, 99.0, 103.0, 99.5, 102.5), row(5, 102.5, 104.0, 102.0, 103.0)];
        let x = Ctx::new(&r, &r, &[]);
        let none = |_: usize, _: &Pos| -> Option<Act> { None };
        let od = Ord { stop: 99.0, ..ord(1.0, Fill::Stop(101.0), 0, 1, 1) };
        let run = |re: Option<&dyn Fn(&Ord, usize) -> bool>| sim(&x, vec![od], &Ex { trail: 0, half_bar: 0, rule: &none, gate: None, reentry: re }, 0, 5);
        let t = run(Some(&|_, _| true));
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (1, 2, 99.0));
        assert_eq!((t[1].legs[0].i, t[1].legs[0].px, t[1].entry_kind, t[1].open), (4, 101.0, "re-entry", true));
        assert_eq!(run(None).len(), 1); // reentry = 0
        assert_eq!(run(Some(&|_, j| j < 4)).len(), 1); // order cancelled (new setup / ADX fails) before bar 4
    }

    #[test]
    fn target_already_gapped_past_on_the_entry_bar_fills_at_the_fill() {
        // sell stop 100 with target 98; bar 1 gaps open at 97 (through both) -> short 97, and the buy-limit 98 is
        // marketable at once: out at 97, not at 98 (above the bar's 97.5 high; SPX/NDX/SPY 2024-08-02)
        let r = vec![row(0, 101.0, 102.0, 100.5, 101.0), row(1, 97.0, 97.5, 96.0, 96.5), row(2, 96.5, 97.0, 95.0, 95.5)];
        let x = Ctx::new(&r, &r, &[]);
        let none = |_: usize, _: &Pos| -> Option<Act> { None };
        let od = Ord { stop: 102.0, target: 98.0, ..ord(-1.0, Fill::Stop(100.0), 0, 1, 1) };
        let t = sim(&x, vec![od], &Ex { trail: 0, half_bar: 0, rule: &none, gate: None, reentry: None }, 0, 2);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (97.0, 1, 97.0));
    }

    #[test]
    fn two_period_roc_worksheet_p61() {
        // S&P closes 10-30 .. 11-8 and the book's ROC, pivot and go-home columns (printed p.61)
        let c: [f64; 8] = [586.70, 583.85, 588.25, 592.35, 592.50, 591.30, 588.20, 594.10];
        let roc: [f64; 6] = [1.55, 8.50, 4.25, -1.05, -4.30, 2.80];
        let piv: [f64; 6] = [585.40, 596.75, 596.60, 591.45, 587.00, 591.00];
        for k in 0..6 {
            let i = k + 2;
            assert!((c[i] - c[i - 2] - roc[k]).abs() < 1e-9);
            assert!((c[i - 1] + (c[i] - c[i - 2]) - piv[k]).abs() < 1e-9);
        }
        // go home: 11-2 long, 11-3 short, 11-6 short, 11-7 short, 11-8 long (11-1 needs a 10-27 close)
        let st = roc_state(&c, 2);
        assert_eq!(&st[3..], &[1.0, -1.0, -1.0, -1.0, 1.0]);
        for i in 3..8 {
            assert_eq!(st[i] > 0.0, c[i] > c[i - 2] + (c[i - 1] - c[i - 3]), "close vs yesterday's pivot, row {i}");
        }
    }

    #[test]
    fn e80_20_fill_rule_on_a_bar_that_opened_above_the_level() {
        // bar 1 opens near its high and closes near its low; bar 2 opens ABOVE yesterday's low (100), breaks to 99,
        // trades back to 100 and closes at 99.5. Skipped by rule 0, filled at 100 by rule 1, skipped by rule 2 (close below).
        let r = vec![row(0, 105.0, 110.0, 100.0, 105.0), row(1, 109.0, 110.0, 100.0, 101.0), row(2, 101.0, 101.5, 99.0, 99.5)];
        let x = Ctx::new(&r, &r, &[]);
        let n = |fr: f64| {
            let mut p = Params::new();
            p.insert("fill_rule".into(), fr);
            run("ss_80_20", &x, 0, 2, &p).unwrap()
        };
        assert!(n(0.0).is_empty() && n(2.0).is_empty());
        let t = n(1.0);
        assert_eq!(t.len(), 1);
        assert!((t[0].legs[0].px - 100.0).abs() < 1e-9 && (t[0].exit_px - 99.5).abs() < 1e-9);
    }

    #[test]
    fn id_nr4_buys_then_stops_and_reverses_on_the_entry_day() {
        // bars 0-3 wide, bar 4 = inside day with the narrowest range of 4, bar 5 opens 103, trades up to 107
        // (buy stop 105.01) then down to 94 (sell stop 94.99): long 105.01 -> reversed short 94.99, still open
        let mut r: Vec<Row> = (0..4).map(|i| row(i, 100.0, 110.0, 90.0, 100.0)).collect();
        r.push(row(4, 100.0, 105.0, 95.0, 100.0));
        r.push(row(5, 103.0, 107.0, 94.0, 96.0));
        r.push(row(6, 96.0, 97.0, 93.0, 94.0));
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ss_id_nr4", &x, 0, 6, &Params::new()).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].dir, 1.0);
        assert!((t[0].legs[0].px - 105.01).abs() < 1e-9 && (t[0].exit_px - 94.99).abs() < 1e-9);
        assert_eq!((t[1].dir, t[1].legs[0].i), (-1.0, 5));
        assert!((t[1].legs[0].px - 94.99).abs() < 1e-9);
        assert!(t[1].open);
    }
}
