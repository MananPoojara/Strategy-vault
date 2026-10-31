//! Courtney D. Smith, "How to Make a Living Trading Foreign Exchange: A Guaranteed Income for Life" (Wiley, 2010; book id `csmlt`).
//! Reading: claude-continuation-20261020/authors/csmith/agents/. Live: trend analysis with three-bar swings (Ch.2), the 55/20 channel
//! breakout with the rejection rule, last-bar stop, Bishop and ADX filter (Ch.3), the Conqueror I and II (Ch.4), the stochastic 50
//! line (Ch.5), reversal days with the lemonade trade and riding the rejection rule (Ch.6). The book trades currency pairs; runs on
//! indices, ETFs and stocks are a declared application. Daily bars; orders at a level fill at that level or at the open if the bar
//! opens beyond it.
use crate::connors::{dmi, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "csmlt";
pub const REVIEWED: &[&str] = &["csmlt_trend3", "csmlt_cb55", "csmlt_conq1", "csmlt_conq2", "csmlt_stoch50", "csmlt_revday", "csmlt_rrr"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const TREND_NOTE: &str = "[Declared readings] A swing high is a bar whose high is above the next bar's high and above the highs of at least the three bars just before it (the count stops at the first bar with an equal or higher high; Smith: three-bar highs and lows are significant, p.17-18); swing lows mirror. A swing is known at the close of the bar after it. Long: the last two swing lows are rising and the price trades above the latest swing high (the higher high is then 'guaranteed', p.21-25); short: the last two swing highs are falling and the price trades below the latest swing low. Fill at that level, or at the open if the bar opens beyond it; if both levels are hit on a bar that opens between them the order is unknown on daily bars and no trade is taken. Stop: the latest swing low (long) / swing high (short), moved as new swings form, not checked on the entry bar. A newly known swing against the position (a lower high or lower low while long, a higher low or higher high while short) makes the market neutral: exit at the next open (p.23). No new entry on a stop-out bar. Options: adx=1 takes entries only when yesterday's 14-day ADX was above the day before (p.27-28, no look-ahead); bishop=1 exits at the close when the ADX was above 40 and turns down (p.29). Book: currency pairs; indices, ETFs and stocks are a declared application.";
const CB_NOTE: &str = "[Declared readings] Breakout = trade beyond the highest high (lowest low) of the previous 55 bars; filled at that level or at a gapped open (the book's 3-pip offset is not used). ADX filter: entry only if yesterday's 14-day ADX was above the day before. Five-day condition: the 55-day level has not risen (breakdowns: fallen) on any of the five days up to the breakout. Rejection rule (with the five-day condition): exit at the close of the breakout day or the next day if the close is back below the breakout level. Last-bar stop = the low (high) of the breakout bar, from the day after entry (with the rejection rule: from the third day). 20-day exit = trade beyond the 20-bar low (high), always on from the day after entry; the effective stop is the tighter of the active stops. Bishop: exit at the close when the ADX was above 40 and ticks down. Stops are not checked on the entry bar; no new entry on a stop-out bar; entries are both long and short; if both channel sides are hit on a bar that opens inside the channel no trade is taken. Options switch each enhancement off (all off = the plain 55/20, p.36-38).";
const CONQ_NOTE: &str = "[Declared readings] Conditions at the close: (1) close minus the 10-day simple average, (2) the 10-day average minus its value 10 days ago, (3) close minus the close 40 days ago. Enter at the close of the day all three turn positive (negative); a fresh alignment is needed, so after a stop-out with the three still aligned one condition must flip and return first (Smith's stated preference, p.65). ATR = simple 40-day average of the true range. Stop = the best close since entry minus (plus) k x ATR x (conditions still agreeing / 3): each condition that turns against the trade cuts a third (p.63-64); recomputed at every close, used on the next bar, filled at the stop or a gapped open. The $2,000 money stop (per FX contract) is not used. k = 2 for Conqueror I, 12 for Conqueror II. A fresh alignment at the close of a stop-out day can enter again.";
const STOCH_NOTE: &str = "[Declared readings] Raw K = (close - lowest low of 14 bars) / (highest high - lowest low) x 100; %K = 3-day average of raw K (p.68). Buy at the close when %K crosses above 50, sell short when it crosses below; always in the market after the first cross (p.81). Daily bars (the book uses daily and weekly).";
const REV_NOTE: &str = "[Declared readings] Bullish reversal day = lower low than the previous day and a close above the previous close; bearish = higher high and a close below the previous close (p.97). Enter at its close; exit at the next day's close; stop at the reversal day's low (high) - the book's text says 'the low of the inside day' in this section, read as the reversal day. If stopped, reverse at the stop (or a gapped open) and exit that lemonade trade at the same close (p.97-98). The short-term trend filter is not coded (the book does not define it and says taking all reversal bars is also profitable, p.99).";
const RRR_NOTE: &str = "[Declared readings] Runs the default csmlt_cb55 system and trades each rejection-rule exit the other way: enter at that close, stop at the other end of that bar, exit at the next day's close (p.101-103).";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    let nt = |id, name, group, why, rules| d(id, name, group, NotTestable(why), rules, "Held after the 2026-10-24 reading (authors/csmith/agents).");
    vec![
        d("csmlt_trend3", "Trend analysis with three-bar swings (higher highs and lows)", "Ch.2 Trend analysis (p.13-30)", Live,
          &["F|\"three-bar highs and lows are significant and that two- and one-bar highs and lows are not\" (p.18); a swing high needs \"one bar to the right that has a lower high\" and bars to the left with lower highs (p.16-17)",
            "E|\"Go long the moment the market moves into a pattern of higher highs and higher lows. Note that all four points don't have to exist to get long but three of them do have to exist and the fourth must be in the process of being formed\" (p.25)",
            "X|\"The stop on short positions is the most recent three-bar high. The protective stop on long positions is always the most recent significant swing low\" (p.22)",
            "X|\"we must be on the sidelines whenever there is a neutral market. We exit the position on the open the next day\" (p.23)"], TREND_NOTE),
        d("csmlt_cb55", "55/20 channel breakout with the rejection rule, last bar, Bishop and ADX filter", "Ch.3 Channel breakouts (p.31-53)", Live,
          &["E|\"Entries are made using 55 days but exits are made with 20 days\" (p.37)",
            "E|ADX filter: \"takes ... trades only when the ADX is higher than it was the prior day\" (p.28, p.43)",
            "X|Rejection rule: \"If a five-day condition is in place, liquidate a 55-day buy signal if the price of the market closes below the initial breakout level on the day of the breakout or the next day\" (p.47)",
            "X|Last bar: \"the last bar with any portion still under the 55-day level. We set our stop loss at the bottom of this bar\" (p.51)",
            "X|Tactics: \"First look at the rejection rule, then the last bar, then the 20-day low, in that order. We are looking at the Bishop every day\" (p.52)",
            "X|Bishop: \"I look for the ADX line to be above 40 and then turn down. I then exit\" (p.29)"], CB_NOTE),
        d("csmlt_conq1", "The Conqueror I (three time frames, 2 x ATR adaptive stop)", "Ch.4 The Conqueror (p.55-65)", Live,
          &["E|\"1. Today's close minus the 10-day moving average (MA) of the close 2. Today's 10-day MA minus the 10-day MA 10 days ago 3. Today's close minus the close 40 days ago\"; \"Go long if all three conditions are positive. Go short if all three conditions are negative\" (p.57-59)",
            "E|\"We go short on the close of trading on the day that the third condition triggers\" (p.61)",
            "X|\"Conqueror I will create a stop two times the ATR away from the current close\"; \"a protective stop loss 480 pips above the lowest close in the move\" (p.61-62)",
            "X|\"Every time a factor changes sign (positive or negative), we will reduce the size of the ATR stop by a third\" (p.63)"], CONQ_NOTE),
        d("csmlt_conq2", "The Conqueror II (12 x ATR adaptive stop)", "Ch.4 The Conqueror (p.55-65)", Live,
          &["E|Same three conditions as the Conqueror I (p.57-59)",
            "X|\"Conqueror II will create a stop 12 times the ATR away from the current close\" (p.62)"], CONQ_NOTE),
        d("csmlt_stoch50", "Stochastic %K crossing the 50 line (always in)", "Ch.5 Stochastics (p.67-90)", Live,
          &["F|\"K = (C - L)/(H - L) * 100 ... %K is a three-day MA of K\"; 14 days (p.68-69)",
            "E|\"The basic rule is to buy when the %K stochastic crosses above the 50 line and sell when it crosses below the 50 line\" (p.81)",
            "X|\"You are always in the market\": a cross the other way closes the position and reverses it (p.81)"], STOCH_NOTE),
        d("csmlt_revday", "Reversal day with the lemonade trade", "Ch.6 Pattern recognition (p.91-103)", Live,
          &["F|\"A Reversal Day occurs when a pair makes a lower low than the previous day and closes higher. On the bear side ... a higher high than the previous day and closes lower\" (p.97)",
            "E|\"buy at the close of trading ... and hold the position until the close of trading the next day\" (p.97)",
            "X|\"Our protective stop loss is just below the low ... We will also lemonade that stop loss and go short if we get stopped out. The lemonade trade will still be liquidated on the close of the day following\" (p.97-98)"], REV_NOTE),
        d("csmlt_rrr", "Riding the rejection rule (RRR)", "Ch.6 Pattern recognition (p.101-103)", Live,
          &["E|\"Our entry rule is to sell (or buy) whenever we are stopped out of our entry trades using the channel breakout technique\" by the rejection rule (p.102)",
            "X|\"Our protective stop is the other end of our entry bar. We exit the trade at the close of trading\" (p.102)"], RRR_NOTE),
        nt("csmlt_cb20", "20/20 channel breakout (Donchian four-week rule)", "Ch.3 Channel breakouts (p.32-35)",
           "the same rule is already in the vault (Donchian four-week rule, e.g. kftsm and covel modules); Smith: 'I don't recommend using this system' (p.35); csmlt_cb55 with p.entry=20 and all enhancements off is a stop-only version", &["E|\"You buy the currency pair when it makes a new four-week high or sell when it makes a new four-week low\" (p.31)"]),
        nt("csmlt_inside_day", "Inside day breakout with the double whammy", "Ch.6 Pattern recognition (p.91-96)",
           "the exit is the close of the breakout day and a stopped trade reverses; when one day breaks both sides of the inside day, daily bars do not show which came first (needs intraday bars; the 20-pip multiunit exit is in pips)",
           &["E|\"place a buy stop order just above the high of the inside day and place a sell stop order just under the low of the inside day\"; \"The exit of the position will always be the close of trading on the breakout day\" (p.92-94)"]),
        nt("csmlt_slingshot", "Slingshot and mini-slingshot", "Ch.8 Slingshot (p.119-130)",
           "the key high ('any high that is higher than the previous high') and the two/three-bar confirmations are not defined consistently, and the profit plan halves a multi-contract position at each target",
           &["E|\"We now look to sell on a break of the key low ... Your first protective stop loss is the slingshot high\" (p.127)"]),
        nt("csmlt_stoch_div", "Stochastic divergences, failures and crossovers", "Ch.5 Stochastics (p.70-84)",
           "divergences have 'no specific entry and exit rules' (p.76); overbought/oversold and %K/%D crossover rules are described as money-losing and not recommended",
           &["F|\"There are no specific entry and exit rules but I'll show you how to use stochastic divergences\" (p.76)"]),
        StrategyDef { book: "csops", ..nt("csops_spreads", "Option strategies (spreads, straddles, ratio and calendar spreads, covered writes)", "Option Strategies (2nd ed., 1996) · Parts One-Two",
           "strategies are picked to fit an expected market view and priced from option quotes; no entry or exit rule on the underlying (read by contents, scanned copy)",
           &["F|Bull/bear, butterfly, calendar and ratio spreads, straddles and strangles (contents)"]) },
        StrategyDef { book: "csscf", ..nt("csscf_seasonals", "Seasonal price tendencies of commodity futures", "Seasonal Charts for Futures Traders (1987)",
           "a sourcebook of futures seasonal charts and statistics; 'It should not be followed slavishly' (Preface) and it needs futures prices (read by preface and contents, scanned copy)",
           &["F|Seasonal moves with over 80% reliability for timing entries, exits and stops (Preface)"]) },
        nt("csmlt_combined", "Combined methods, risk and Kelly sizing", "Ch.7, Ch.10",
           "position sizing and combinations of the other methods (majority vote, tightest stop) need a multi-method portfolio; no separate rule", &[]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "csmlt_trend3" => ("csmlt_trend3", "Smith trend analysis", "three-bar swings"),
        "csmlt_cb55" => ("csmlt_cb55", "Smith channel breakout", "55/20 with enhancements"),
        "csmlt_conq1" => ("csmlt_conq", "Smith Conqueror", "I · 2 x ATR"),
        "csmlt_conq2" => ("csmlt_conq", "Smith Conqueror", "II · 12 x ATR"),
        "csmlt_stoch50" => ("csmlt_stoch50", "Smith stochastic 50 line", "always in"),
        "csmlt_revday" => ("csmlt_revday", "Smith reversal day", "with lemonade"),
        "csmlt_rrr" => ("csmlt_rrr", "Smith riding the rejection rule", "55-day"),
        _ if id.starts_with("csmlt_") || id.starts_with("csops_") || id.starts_with("csscf_") => ("csmlt_nt", "Smith (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "csmlt_trend3" => vec![p("bars", "Bars to the left that make a swing significant", 3., 1., 10., 1.),
            p("adx", "ADX rising filter (1 on, 0 off)", 0., 0., 1., 1.), p("bishop", "Bishop exit (1 on, 0 off)", 0., 0., 1., 1.)],
        "csmlt_cb55" | "csmlt_rrr" => vec![p("entry", "Entry channel (bars)", 55., 10., 100., 1.), p("exit", "Exit channel (bars)", 20., 5., 60., 1.),
            p("rejection", "Rejection rule (1 on, 0 off)", 1., 0., 1., 1.), p("lastbar", "Last-bar stop (1 on, 0 off)", 1., 0., 1., 1.),
            p("bishop", "Bishop exit (1 on, 0 off)", 1., 0., 1., 1.), p("adx", "ADX rising filter (1 on, 0 off)", 1., 0., 1., 1.)],
        "csmlt_conq1" => vec![p("k", "ATR multiple", 2., 0.5, 20., 0.5)],
        "csmlt_conq2" => vec![p("k", "ATR multiple", 12., 0.5, 20., 0.5)],
        "csmlt_stoch50" => vec![p("len", "Stochastic length", 14., 5., 30., 1.)],
        "csmlt_revday" => vec![p("lemonade", "Reverse when stopped (1 on, 0 off)", 1., 0., 1., 1.)],
        _ if id.starts_with("csmlt_") || id.starts_with("csops_") || id.starts_with("csscf_") => vec![],
        _ => return None,
    })
}

fn tr(dir: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn close(out: &mut Vec<Trade>, mut t: Trade, i: usize, px: f64, sig: usize, why: &str) {
    t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); out.push(t);
}
fn finish(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
}
/// Stop fill for a position of direction `dir` with stop `s` on bar i: Some(price) if hit.
fn stop_hit(x: &Ctx, i: usize, dir: f64, s: f64) -> Option<f64> {
    if !s.is_finite() { return None; }
    if dir > 0. { if x.o[i] < s { Some(x.o[i]) } else if x.l[i] < s { Some(s) } else { None } }
    else if x.o[i] > s { Some(x.o[i]) } else if x.h[i] > s { Some(s) } else { None }
}
/// Entry-stop fill: buy above `lv` (dir>0) / sell below `lv` (dir<0) on bar i.
fn entry_fill(x: &Ctx, i: usize, dir: f64, lv: f64) -> Option<f64> {
    if !lv.is_finite() { return None; }
    if dir > 0. { if x.o[i] > lv { Some(x.o[i]) } else if x.h[i] > lv { Some(lv) } else { None } }
    else if x.o[i] < lv { Some(x.o[i]) } else if x.l[i] < lv { Some(lv) } else { None }
}
/// Both-sides entry orders: returns the side filled, None if neither or if both were hit on a bar opening between them.
fn two_sided(x: &Ctx, i: usize, hi: f64, lo: f64, allow_long: bool, allow_short: bool) -> Option<(f64, f64)> {
    let l = if allow_long { entry_fill(x, i, 1., hi) } else { None };
    let s = if allow_short { entry_fill(x, i, -1., lo) } else { None };
    match (l, s) {
        (Some(p), None) => Some((1., p)),
        (None, Some(p)) => Some((-1., p)),
        (Some(p), Some(q)) => if hi.is_finite() && x.o[i] > hi { Some((1., p)) } else if lo.is_finite() && x.o[i] < lo { Some((-1., q)) } else { None },
        _ => None,
    }
}
fn adx14(x: &Ctx) -> Vec<f64> { dmi(x.r, 14).1 }
fn adx_rising(a: &[f64], i: usize) -> bool { i >= 2 && a[i - 1].is_finite() && a[i - 2].is_finite() && a[i - 1] > a[i - 2] }
fn bishop(a: &[f64], i: usize) -> bool { i >= 1 && a[i].is_finite() && a[i - 1].is_finite() && a[i - 1] > 40. && a[i] < a[i - 1] }

/// Significant swing points (bar index, value) confirmed at bar j+1. hi=true for highs.
fn swings(v: &[f64], hi: bool, bars: usize) -> Vec<(usize, f64)> {
    let better = |a: f64, b: f64| if hi { a > b } else { a < b };
    let mut out = vec![];
    for j in bars..v.len().saturating_sub(1) {
        if !better(v[j], v[j + 1]) { continue; }
        if (1..=bars).all(|k| better(v[j], v[j - k])) { out.push((j, v[j])); }
    }
    out
}

fn run_trend(x: &Ctx, from: usize, to: usize, bars: usize, adxf: bool, bish: bool) -> Vec<Trade> {
    let (sh, sl) = (swings(&x.h, true, bars), swings(&x.l, false, bars));
    let a = if adxf || bish { adx14(x) } else { vec![] };
    // last two swings known by the close of bar t (confirmed at j+1 <= t)
    let known = |s: &[(usize, f64)], t: usize| -> (f64, f64, usize) {
        let k = s.partition_point(|p| p.0 + 1 <= t);
        let last = if k >= 1 { s[k - 1].1 } else { f64::NAN };
        let prev = if k >= 2 { s[k - 2].1 } else { f64::NAN };
        (prev, last, k)
    };
    let (mut out, mut cur, mut pending_exit) = (vec![], None::<Trade>, false);
    for i in from.max(1)..=to {
        let (h1, h2, kh) = known(&sh, i - 1);
        let (l1, l2, kl) = known(&sl, i - 1);
        let mut stopped = false;
        if let Some(t) = cur.take() {
            if pending_exit {
                close(&mut out, t, i, x.o[i], i - 1, "neutral market: exit at the open");
                pending_exit = false;
            } else if t.legs[0].i < i {
                let s = if t.dir > 0. { l2 } else { h2 };
                if let Some(px) = stop_hit(x, i, t.dir, s) {
                    let why = if t.dir > 0. { "stop: latest swing low" } else { "stop: latest swing high" };
                    close(&mut out, t, i, px, i, why);
                    stopped = true;
                } else { cur = Some(t); }
            } else { cur = Some(t); }
        }
        if cur.is_none() && !stopped && i >= from && (!adxf || adx_rising(&a, i)) {
            let up = l2.is_finite() && l1.is_finite() && l2 > l1;
            let dn = h2.is_finite() && h1.is_finite() && h2 < h1;
            if let Some((d, px)) = two_sided(x, i, h2, l2, up, dn) {
                cur = Some(tr(d, i, px, "stop", i));
            }
        }
        // at this close: Bishop, and newly known swings against the position
        if let Some(t) = cur.take() {
            if bish && bishop(&a, i) {
                close(&mut out, t, i, x.c[i], i, "Bishop: ADX above 40 turned down");
            } else {
                let (nh1, nh2, nkh) = known(&sh, i);
                let (nl1, nl2, nkl) = known(&sl, i);
                let against = (nkh > kh && nh1.is_finite() && (if t.dir > 0. { nh2 < nh1 } else { nh2 > nh1 }))
                    || (nkl > kl && nl1.is_finite() && (if t.dir > 0. { nl2 < nl1 } else { nl2 > nl1 }));
                if against && i < to { pending_exit = true; }
                cur = Some(t);
            }
        }
    }
    finish(&mut out, cur, x, to);
    out
}

fn chan(v: &[f64], n: usize, hi: bool) -> Vec<f64> {
    // level on bar i = extreme of the previous n bars (i-n .. i-1)
    (0..v.len()).map(|i| if i >= n { let w = &v[i - n..i]; if hi { w.iter().cloned().fold(f64::MIN, f64::max) } else { w.iter().cloned().fold(f64::MAX, f64::min) } } else { f64::NAN }).collect()
}

/// Channel breakout simulation; also returns the rejection exits (bar, dir of the rejected trade) for RRR.
fn sim_cb(x: &Ctx, from: usize, to: usize, ne: usize, nx: usize, rej: bool, lastbar: bool, bish: bool, adxf: bool) -> (Vec<Trade>, Vec<(usize, f64)>) {
    let (eh, el, xh, xl) = (chan(&x.h, ne, true), chan(&x.l, ne, false), chan(&x.h, nx, true), chan(&x.l, nx, false));
    let a = if bish || adxf { adx14(x) } else { vec![] };
    let five = |i: usize, d: f64| -> bool {
        i >= 5 && (0..5).all(|k| { let (p, q) = if d > 0. { (eh[i - k], eh[i - k - 1]) } else { (el[i - k], el[i - k - 1]) };
            p.is_finite() && q.is_finite() && (if d > 0. { p <= q } else { p >= q }) })
    };
    let (mut out, mut rejs, mut cur) = (vec![], vec![], None::<Trade>);
    let (mut level, mut lbx, mut rule) = (f64::NAN, f64::NAN, false);
    for i in from.max(1)..=to {
        let mut stopped = false;
        if let Some(t) = cur.take() {
            let e = t.legs[0].i;
            let day = i - e + 1; // entry bar = day 1
            let mut s = if t.dir > 0. { xl[i] } else { xh[i] };
            if lastbar && (if rule { day >= 3 } else { day >= 2 }) {
                s = if t.dir > 0. { s.max(lbx) } else { s.min(lbx) };
            }
            if let Some(px) = stop_hit(x, i, t.dir, s) {
                let why = if lastbar && s == lbx && (if rule { day >= 3 } else { day >= 2 }) { "last-bar stop" } else { "20-day channel stop" };
                close(&mut out, t, i, px, i, why); stopped = true;
            } else { cur = Some(t); }
        }
        if cur.is_none() && !stopped && (!adxf || adx_rising(&a, i)) {
            if let Some((d, px)) = two_sided(x, i, eh[i], el[i], true, true) {
                level = if d > 0. { eh[i] } else { el[i] };
                lbx = if d > 0. { x.l[i] } else { x.h[i] };
                rule = rej && five(i, d);
                cur = Some(tr(d, i, px, "stop", i));
            }
        }
        if let Some(t) = cur.take() {
            let day = i - t.legs[0].i + 1;
            if rule && day <= 2 && (if t.dir > 0. { x.c[i] < level } else { x.c[i] > level }) {
                let d = t.dir;
                close(&mut out, t, i, x.c[i], i, "rejection rule: close back inside the channel");
                rejs.push((i, d));
            } else if bish && bishop(&a, i) {
                close(&mut out, t, i, x.c[i], i, "Bishop: ADX above 40 turned down");
            } else { cur = Some(t); }
        }
    }
    finish(&mut out, cur, x, to);
    (out, rejs)
}

fn run_rrr(x: &Ctx, from: usize, to: usize, rejs: &[(usize, f64)]) -> Vec<Trade> {
    let mut out: Vec<Trade> = vec![];
    for &(i, d0) in rejs {
        if i >= to { break; }
        if out.last().map_or(false, |t| t.exit_i > i) { continue; }
        let d = -d0;
        let t = tr(d, i, x.c[i], "close", i);
        let s = if d > 0. { x.l[i] } else { x.h[i] };
        let j = i + 1;
        if let Some(px) = stop_hit(x, j, d, s) { close(&mut out, t, j, px, j, "stop: other end of the entry bar"); }
        else { close(&mut out, t, j, x.c[j], j, "next day's close"); }
    }
    let _ = from;
    out
}

fn sma_exact(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if n > 0 && i + 1 >= n { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect()
}

fn run_conq(x: &Ctx, from: usize, to: usize, k: f64) -> Vec<Trade> {
    let c = &x.c;
    let m10 = sma_exact(c, 10);
    let trv: Vec<f64> = (0..c.len()).map(|i| if i == 0 { x.h[0] - x.l[0] } else { x.h[i].max(c[i - 1]) - x.l[i].min(c[i - 1]) }).collect();
    let atr = sma_exact(&trv, 40);
    let sig = |i: usize| -> Option<[f64; 3]> {
        if i < 40 || !m10[i].is_finite() || !m10[i - 10].is_finite() { return None; }
        Some([c[i] - m10[i], m10[i] - m10[i - 10], c[i] - c[i - 40]])
    };
    let aligned = |i: usize, d: f64| sig(i).map_or(false, |s| s.iter().all(|&v| v * d > 0.));
    let (mut out, mut cur) = (vec![], None::<Trade>);
    let (mut best, mut stop) = (f64::NAN, f64::NAN);
    for i in from.max(41)..=to {
        if let Some(t) = cur.take() {
            if let Some(px) = stop_hit(x, i, t.dir, stop) { close(&mut out, t, i, px, i, "adaptive ATR stop"); } else { cur = Some(t); }
        }
        if cur.is_none() {
            for d in [1., -1.] {
                if aligned(i, d) && !aligned(i - 1, d) { cur = Some(tr(d, i, c[i], "close", i)); best = c[i]; break; }
            }
        }
        if let Some(t) = cur.as_ref() {
            best = if t.dir > 0. { best.max(c[i]) } else { best.min(c[i]) };
            let agree = sig(i).map_or(0, |s| s.iter().filter(|&&v| v * t.dir > 0.).count()) as f64;
            stop = best - t.dir * k * atr[i] * agree / 3.;
        }
    }
    finish(&mut out, cur, x, to);
    out
}

fn run_stoch(x: &Ctx, from: usize, to: usize, n: usize) -> Vec<Trade> {
    let raw: Vec<f64> = (0..x.c.len()).map(|i| {
        if i + 1 < n { return f64::NAN; }
        let hh = x.h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max);
        let ll = x.l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min);
        if hh > ll { (x.c[i] - ll) / (hh - ll) * 100. } else { f64::NAN }
    }).collect();
    let k = sma_exact(&raw, 3);
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for i in from.max(1)..=to {
        if !k[i].is_finite() || !k[i - 1].is_finite() { continue; }
        let d = if k[i - 1] <= 50. && k[i] > 50. { 1. } else if k[i - 1] >= 50. && k[i] < 50. { -1. } else { 0. };
        if d == 0. { continue; }
        if let Some(t) = cur.take() {
            if t.dir == d { cur = Some(t); continue; }
            close(&mut out, t, i, x.c[i], i, if d > 0. { "%K crossed above 50" } else { "%K crossed below 50" });
        }
        cur = Some(tr(d, i, x.c[i], "close", i));
    }
    finish(&mut out, cur, x, to);
    out
}

fn run_rev(x: &Ctx, from: usize, to: usize, lemon: bool) -> Vec<Trade> {
    let mut out: Vec<Trade> = vec![];
    for i in from.max(1)..to {
        if out.last().map_or(false, |t| t.exit_i > i) { continue; }
        let d = if x.l[i] < x.l[i - 1] && x.c[i] > x.c[i - 1] { 1. } else if x.h[i] > x.h[i - 1] && x.c[i] < x.c[i - 1] { -1. } else { continue };
        let t = tr(d, i, x.c[i], "close", i);
        let j = i + 1;
        let s = if d > 0. { x.l[i] } else { x.h[i] };
        if let Some(px) = stop_hit(x, j, d, s) {
            close(&mut out, t, j, px, j, "stop beyond the reversal day");
            if lemon { let l = tr(-d, j, px, "stop", j); close(&mut out, l, j, x.c[j], j, "lemonade trade: close of the day"); }
        } else { close(&mut out, t, j, x.c[j], j, "next day's close"); }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let on = |k: &str| g(k) >= 0.5;
    let n = |k: &str| g(k).round().max(1.) as usize;
    Some(match id {
        "csmlt_trend3" => run_trend(x, from, to, n("bars"), on("adx"), on("bishop")),
        "csmlt_cb55" => sim_cb(x, from, to, n("entry"), n("exit"), on("rejection"), on("lastbar"), on("bishop"), on("adx")).0,
        "csmlt_rrr" => { let (_, r) = sim_cb(x, from, to, n("entry"), n("exit"), true, on("lastbar"), on("bishop"), on("adx")); run_rrr(x, from, to, &r) }
        "csmlt_conq1" | "csmlt_conq2" => run_conq(x, from, to, g("k")),
        "csmlt_stoch50" => run_stoch(x, from, to, n("len").max(2)),
        "csmlt_revday" => run_rev(x, from, to, on("lemonade")),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn reversal_day_stops_and_lemonades() {
        // bar1: lower low, higher close -> long at 10.5; bar2 trades below 9 -> stop at 9, short at 9, out at 8.5
        let r = rows(&[(10., 11., 9.5, 10.), (10., 10.6, 9., 10.5), (10., 10.2, 8., 8.5), (8.5, 9., 8.4, 8.8)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run_rev(&x, 0, 3, true);
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1., 10.5, 2, 9.));
        assert_eq!((t[1].dir, t[1].legs[0].px, t[1].exit_px), (-1., 9., 8.5));
    }
    #[test]
    fn swing_needs_three_lower_bars_left_and_one_right() {
        let h = [1., 2., 3., 4., 3.5, 5., 4.];
        assert_eq!(swings(&h, true, 3), vec![(3, 4.), (5, 5.)]);
        assert_eq!(swings(&[1., 3., 2., 3., 1.], true, 3), vec![]); // equal high stops the count
    }
}

// Instrument admission: CSmith 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/csmith/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"csmlt") && (crate::books::csmlt::reviewed(def.id)),
    label: "Courtney Smith wrote these methods for currency pairs; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.",
    basis: "Courtney D. Smith, How to Make a Living Trading Foreign Exchange (Wiley 2010): Ch.2 trend analysis (p.13-30), Ch.3 channel breakouts (p.31-53), Ch.4 the Conqueror (p.55-65), Ch.5 stochastics (p.81), Ch.6 reversal days and RRR (p.97-103). Reviewed in authors/csmith/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.",
};
