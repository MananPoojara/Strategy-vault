//! Dave Landry, "Dave Landry's 10 Best Swing Trading Patterns & Strategies" (TradingMarkets Publishing, 2003).
//! Source: Books/Dave Landry/Dave Landry - 10 Best Swing Trading.pdf (197 pages, scanned; printed page = PDF page - 14
//! up to p.54, PDF page - 15 after the inserted advert page). Extraction and verification: ~/.local/share/strategy-vault/library/dl10b/.
//!
//! Only two patterns are mechanical in this book: the Explosion Gap Pivot (Ch 7) and the Reversal Gap Strategy (Ch 10).
//! Every other pattern starts from a trend the author judges by eye ("I don't use ADX ... I 'eyeball' a chart", p.18).
//!
//! Conventions: setups are read on the close of bar t with data <= t. Entries are stop orders for later bars, filled at
//! max(open, level) for buys (min for sells). Exits = the book's money management (Primer p.14-17): the initial stop of the
//! pattern; "as soon as your profits are equal to or greater than your initial risk, you should lock in half of your profits
//! and move your protective stop on your remaining shares to breakeven" (p.15); the rest is trailed "below a two-to-three
//! bar low" (p.16). Bar order: an order already through the open fills at the open first; then the stop that was working
//! at the open is checked before the profit target (conservative); stops moved during a bar (breakeven, trail) work from
//! the next bar.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "dl10b";

const MM1: &str = "X|\"As soon as your profits (a) are equal to or greater than your initial risk (b), you should lock in half of your profits and move your protective stop on your remaining shares to breakeven (c)\" (Primer p.15; Ingersoll-Rand example p.157: half taken on the gap open)";
const MM2: &str = "X|Remaining half: \"placing a stop below a two-to-three bar low can often catch the majority of a strongly trending market\" (p.16): trailed `tick` below the lowest low of the last `trail_n` bars (book 2-3; 2 coded as default), only after the half is taken, never lowered";
macro_rules! note { () => { "Book patterns are for US stocks (prices in dollars); India = instrument swap (NIFTY 50 / indices / NSE basket stocks via basket:<list>:<SYMBOL>). Dollar amounts (the 10-cent entry offset, the .25 stop offset) are kept literally as price points. Not applied: the Primer's dollar stop table by stock price (p.15, a 'general guideline' for stocks), the Q&A's minimum gap of '1 to 2 points' (p.110), market/sector confirmation (tips pages), and the discretionary looser trail 'below each base/pullback' (p.17, p.159). The book publishes no test results." }; }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    let nt = |id, name, group, why: &'static str, rules| d(id, name, group, "Long/Short", NotTestable(why), rules, why);
    vec![
        d("dl10b_egp", "Explosion Gap Pivots", "Ch 7 · Explosion Gap Pivots", "Long/Short", Live,
          &["F|\"The stock should gap to at least a 20-day new high\" (p.73): the bar's low is above the prior bar's high and its high is the highest of the last 20 bars",
            "F|\"The stock should 'test' the gap by making a low into the area of the gap. Said another way, the low of the test bar should be lower than that of the breakout bar. However, it should not fill the gap\" (p.73)",
            "F|\"The stock should then make a higher low. This action forms a 'pivot low' (a low surrounded by two higher lows) into the area of the gap\" (p.74); \"A pivot low is simply a low (or in some cases two or more equal lows) 'surrounded' (the day before and the day after) by two higher lows\" (p.72)",
            "E|\"Go long on the first subsequent day the stock takes out the high of the right side of the pivot pattern\" (p.74); entry 10 cents above that high (\"giving an entry of 31.13, 10 cents above the high of the bar (31.03)\", p.77)",
            "E|A later pivot into the gap replaces the order (\"Sometimes the gap is tested two or even three times\", Alliant Techsystems p.78); a filled gap cancels the setup (\"The stock closes the gap. No trade.\", p.79); a new gap to a new high starts a new setup (back-to-back pivots, p.145)",
            "X|\"If filled, place a protective stop .25 to 1.00 below the pivot low of the test\" (p.75): stop_off, default .25 (the low end of the book's range)",
            "E|\"Short sales are reversed\" (p.73): gap down to a 20-day new low, test into the gap with a pivot high, sell 10 cents below the low of the right side, stop above the pivot high",
            MM1, MM2],
          concat!("Reading: one trade per gap setup; the order works from the bar after the pivot's right side until the first day it is taken out, even when a position is already open (then that setup is gone). One position at a time; if a long and a short order could both fill on one bar, the long is tried first. Equal-low pivots: a run of equal lows with a higher low on each side. ", note!())),
        d("dl10b_rgs", "Reversal Gap Strategy", "Ch 10 · Reversal Gap Strategy", "Long/Short", Live,
          &["F|Shorts: \"The stock must make at least a two-month high. Longer-term highs are even better\" (p.104): the high is the highest of the last `nh` bars (two months = 43 trading days, Landry's own definition in Dave Landry on Swing Trading p.37)",
            "F|\"The stock should then gap lower within the next day or so ... Ideally, this should occur the first day after hitting a new high. However, a variation of the pattern, where the stock gaps lower within 10 days of hitting a new high, can often work well\" (p.104): the bar's high is below the prior bar's low, `within` (default 1) bars after the new-high bar",
            "F|\"The stock must then make a higher high and a higher low, in other words, a one-day pullback. Ignore the setup if the gap is filled\" (p.105)",
            "E|\"Go short below the low of (3)\" (p.105); \"Go short tomorrow below today's low\" (eBay example p.106): a sell stop 10 cents below the pullback bar's low (Primer p.13), good for the next day; each new higher-high/higher-low bar sets a new order (Verizon example p.107)",
            "X|\"Place a protective stop above the high of (3), taking into consideration the initial stop placement guidelines outlined in Primer\" (p.105): stop_off (.10) above the pullback bar's high",
            "E|\"Buys are reversed\" (p.104): two-month low, gap higher, a lower low and lower high, buy 10 cents above the high (American Healthways example p.108)",
            MM1, MM2],
          concat!("Reading: one trade per gap setup; the setup ends when the gap fills or a new gap setup appears. 'Above the high of (3)' gives no amount: .10 (the book's entry offset and the Witch Hat stop offset, p.45) is the default. ", note!())),
        nt("dl10b_persistent_pb", "Persistent Pullbacks", "Ch 2 · Persistent Pullbacks",
           "not testable: persistency is a trend line drawn by hand 'through as many bars as possible', ideally 20 bars and 'at least 10 points (more or less depending on the volatility and price of the stock)' (p.24-25); entry is 'a pullback or pullback related pattern' (p.26)",
           &["F|~20 bars moving one way, trend line through the bars, >= 10 points (p.25)", "E|Enter on a pullback or a TKO as the trend resumes (p.26)"]),
        nt("dl10b_tko", "Trend Knockout", "Ch 3 · Trend Knockout",
           "not testable: 'The stock should be in a strong uptrend and ideally, a persistent uptrend' (p.33) is judged by eye in this book (p.18). The ADX version of the TKO from the author's first book is already in the vault as dlost_tko",
           &["F|Strong uptrend (by eye)", "E|\"The stock should take out (trade below) at least the two prior lows\"; \"Go long above the high of (2)\" (p.33-34)", "X|\"Place a protective stop below the low of (3), taking into consideration those concepts discussed under Primer\" (p.34)"]),
        nt("dl10b_witch_hat", "Witch Hats", "Ch 4 · Witch Hats",
           "not testable: 'strong downtrend, ideally an accelerating downtrend', 'the sharper the resumption, the better' and 'retrace sharply to near the level of the pivot high' (p.43-44) are undefined",
           &["F|Strong downtrend; pivot high (2); trend resumes (3); sharp retrace near (2) (4)", "E|\"Go short .10 below yesterday's low (4)\" (p.45)", "X|\".10 above the pivot point of (2) or the high of (4)\" (p.45)"]),
        nt("dl10b_tpp_second_entry", "False Rally Pullbacks: Trend Pivot and Second Entry Pullbacks", "Ch 5 · False Rally Pullbacks",
           "not testable: 'hitting a new high' has no lookback in this book (p.52) and the trend is judged by eye; the Trend Pivot Pullback with a two-month high is already in the vault as dlost_tpp",
           &["F|New high, pullback 2 to 5 bars (p.52)", "F|Trades above the prior high, then closes below it, or a pivot forms (p.53)", "E|\"Go long if the stock can trade above the high of the false rally\" (p.54)"]),
        nt("dl10b_accel_momentum", "Accelerating Momentum Strategy", "Ch 6 · Accelerating Momentum",
           "not testable: 'longer-term gradual uptrend', 'should accelerate out of its gradual uptrend' (p.64) are chart judgement",
           &["F|Gradual uptrend of two to three months, then acceleration", "E|\"Look to enter on the next pullback or Trend Knockout as the trend resumes\" (p.65)"]),
        nt("dl10b_first_thrust", "First Thrusts", "Ch 8 · First Thrusts",
           "not testable: 'major new low' and 'rally sharply' are undefined; 'It should be obvious. I take it on a case-by-case basis' (p.92)",
           &["F|Major new low, then a sharp rally (p.85)", "E|\"The stock must make a lower high and a lower low ... Go long above the high of (3)\" (p.85)"]),
        nt("dl10b_bow_tie", "Bow Ties", "Ch 9 · Bow Ties",
           "duplicate / not testable as written here: the same 10-SMA / 20-EMA / 30-EMA shift as dlost_bow_tie (already in the vault, from the author's first book with its TradeStation add-on); this book's version 'reevaluates' (not cancels) the order below the 20/30-EMA and sets the stop by the Primer guidelines (p.95)",
           &["F|Averages shift from 10-SMA < 20-EMA < 30-EMA to 10-SMA > 20-EMA > 30-EMA, 'ideally' over three to four days (p.94)", "E|Lower low and lower high; go long above the high, order worked each day until filled (p.95)"]),
        nt("dl10b_gatekeeper", "The Gatekeeper", "Ch 11 · The Gatekeeper",
           "not testable: the 'sharp sell-off' is 'obvious'; quantified only for stocks as 2-3 / 3-4 points, 'larger in higher priced/more volatile stocks' (p.121), and the retrace is '61.8% to 78.6%' with 78.6% 'ideal' (p.114)",
           &["F|Two-month calendar high (a), sharp sell-off (b), retrace of 61.8-78.6% within 10-11 days (c) (p.112-114)", "E|\"go short (d) below the bar that hit the retracement level\" (p.114)", "X|\".25 to 1.00 above the 78.6% retracement level\" (p.115)"]),
        nt("dl10b_variations", "Ch 12 variations (Double Top Knockout, First Gap / two-step Gatekeepers, intraday entries)", "Ch 12 · Variations and scenarios",
           "not testable: examples without rules; the Double Top Knockout is in the vault as dlost_dtko; intraday entries need intraday bars",
           &["E|Examples only (p.127-159)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "dl10b_egp" => ("dl10b_gaps", "Landry gap patterns", "Explosion Gap Pivot"),
        "dl10b_rgs" => ("dl10b_gaps", "Landry gap patterns", "Reversal Gap"),
        "dl10b_persistent_pb" | "dl10b_tko" | "dl10b_witch_hat" | "dl10b_tpp_second_entry" => ("dl10b_resumption", "Landry trend resumption", "Discretionary"),
        "dl10b_accel_momentum" => ("dl10b_acceleration", "Landry trend acceleration", "Accelerating Momentum"),
        "dl10b_first_thrust" | "dl10b_bow_tie" | "dl10b_gatekeeper" => ("dl10b_transition", "Landry trend transition", "Discretionary"),
        "dl10b_variations" => ("dl10b_variations", "Landry variations", "Examples"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let common = || vec![
        p("tick", "Entry offset beyond the trigger bar (book: 10 cents)", 0.10, 0.0, 1.0, 0.05),
        p("trail_n", "Trail the second half below the lowest low of N bars (book: 'two-to-three')", 2.0, 2.0, 3.0, 1.0),
        p("side", "Side: 0 = long and short (book), 1 = long only, 2 = short only", 0.0, 0.0, 2.0, 1.0),
    ];
    let mut v = match id {
        "dl10b_egp" => vec![p("nh", "Gap to at least an N-day new high", 20.0, 10.0, 60.0, 1.0),
            p("stop_off", "Stop below the pivot low (book: .25 to 1.00)", 0.25, 0.25, 1.0, 0.25)],
        "dl10b_rgs" => vec![p("nh", "New high = highest of N bars (two months = 43)", 43.0, 20.0, 65.0, 1.0),
            p("within", "Gap within N days of the new high (book: ideally 1; variation 10)", 1.0, 1.0, 10.0, 1.0),
            p("stop_off", "Stop beyond the pullback bar (book: 'above the high'; amount not given)", 0.10, 0.0, 1.0, 0.05)],
        _ => return None,
    };
    v.extend(common());
    Some(v)
}

/// A stop order working on one bar: enter at `level`, initial stop `stop`, setup bar `signal`.
#[derive(Clone, Copy)]
struct Order { dir: f64, level: f64, stop: f64, signal: usize }

/// Mirrored highs/lows: for shorts the price axis is flipped so one routine serves both sides.
fn mirror(x: &Ctx, d: f64) -> (Vec<f64>, Vec<f64>) {
    if d > 0.0 { (x.h.clone(), x.l.clone()) } else { (x.l.iter().map(|v| -v).collect(), x.h.iter().map(|v| -v).collect()) }
}

/// Explosion Gap Pivot orders for one side, on mirrored bars (long logic).
fn egp_orders(x: &Ctx, d: f64, start: usize, nh: usize, tick: f64, stop_off: f64, orders: &mut [Vec<Order>]) {
    let (hh, ll) = mirror(x, d);
    let n = hh.len();
    // setup: (gap bar g, gap bottom = high before the gap, breakout bar low)
    let mut setup: Option<(usize, f64, f64)> = None;
    let mut pending: Option<Order> = None;
    for t in start.max(1)..n {
        if let Some(o) = pending {
            orders[t].push(o);
            if hh[t] >= d * o.level { pending = None; setup = None; } // taken out today: one trade per setup
        }
        if let Some((_, pre, _)) = setup {
            if ll[t] <= pre { setup = None; pending = None; } // gap filled
        }
        let new_hi = t + 1 >= nh && (t + 1 - nh..=t).all(|k| hh[t] >= hh[k]);
        if ll[t] > hh[t - 1] && new_hi {
            setup = Some((t, hh[t - 1], ll[t]));
            pending = None;
            continue;
        }
        let Some((g, _, brk)) = setup else { continue };
        // pivot low completed today: today's low is higher than a run of equal lows that sits below the breakout low
        if t < g + 2 || !(ll[t] > ll[t - 1]) { continue; }
        let piv = ll[t - 1];
        let mut j = t - 1;
        while j > g + 1 && ll[j - 1] == piv { j -= 1; }
        if ll[j - 1] > piv && piv < brk {
            pending = Some(Order { dir: d, level: d * (hh[t] + tick), stop: d * (piv - stop_off), signal: t });
        }
    }
}

/// Reversal Gap orders for one side, on mirrored bars (long logic: new low, gap up, lower low and lower high, buy above).
fn rgs_orders(x: &Ctx, d: f64, start: usize, nh: usize, within: usize, tick: f64, stop_off: f64, orders: &mut [Vec<Order>]) {
    let (hh, ll) = mirror(x, d);
    let n = hh.len();
    let new_lo = |t: usize| t + 1 >= nh && (t + 1 - nh..=t).all(|k| ll[t] <= ll[k]);
    let mut setup: Option<(usize, f64)> = None; // (gap bar, gap bottom = high before the gap)
    let mut pending: Option<Order> = None;
    for t in start.max(1)..n {
        if let Some(o) = pending.take() {
            orders[t].push(o); // good for this day only
            if hh[t] >= d * o.level { setup = None; }
        }
        if let Some((_, pre)) = setup {
            if ll[t] <= pre { setup = None; } // "Ignore the setup if the gap is filled"
        }
        if ll[t] > hh[t - 1] && (t.saturating_sub(within)..t).any(|k| new_lo(k)) {
            setup = Some((t, hh[t - 1]));
            continue;
        }
        let Some((g, _)) = setup else { continue };
        if t > g && ll[t] < ll[t - 1] && hh[t] < hh[t - 1] {
            pending = Some(Order { dir: d, level: d * (hh[t] + tick), stop: d * (ll[t] - stop_off), signal: t });
        }
    }
}

/// The book's money management on a sequence of working orders, one position at a time.
fn manage(x: &Ctx, from: usize, to: usize, orders: &[Vec<Order>], tick: f64, trail_n: usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = Vec::new();
    let mut i = from;
    while i <= to {
        // an order the open already went through fills first (at the open); otherwise the first listed (long before short)
        let fills: Vec<(Order, f64)> = orders[i].iter().filter_map(|od| {
            let px = if od.dir > 0.0 { (h[i] >= od.level).then(|| o[i].max(od.level)) } else { (l[i] <= od.level).then(|| o[i].min(od.level)) };
            px.map(|px| (*od, px))
        }).collect();
        let fill = fills.iter().find(|f| f.1 == o[i]).or(fills.first()).copied();
        let Some((od, e)) = fill else { i += 1; continue };
        let d = od.dir;
        let risk = d * (e - od.stop);
        if !(risk > 0.0) { i += 1; continue; }
        let target = e + d * risk;
        let (mut stop, mut half) = (od.stop, false);
        let mut ex: Vec<(usize, f64, f64)> = Vec::new();
        let mut why = "open";
        let mut k = i;
        let closed = loop {
            let left = if half { 0.5 } else { 1.0 };
            if k > i {
                if d * (o[k] - stop) <= 0.0 { ex.push((k, o[k], left)); why = if half { "Trailing/breakeven stop (gap)" } else { "Initial stop (gap)" }; break true; }
                if !half && d * (o[k] - target) >= 0.0 { ex.push((k, o[k], 0.5)); half = true; }
            }
            let left = if half { 0.5 } else { 1.0 };
            if (d > 0.0 && l[k] <= stop) || (d < 0.0 && h[k] >= stop) {
                ex.push((k, stop, left));
                why = if !half { "Initial stop" } else if d * (stop - e) > 1e-12 { "Trailing stop" } else { "Breakeven stop" };
                break true;
            }
            if !half && ((d > 0.0 && h[k] >= target) || (d < 0.0 && l[k] <= target)) { ex.push((k, target, 0.5)); half = true; }
            if half {
                // breakeven, then the N-bar trail, effective next bar, ratchet only
                let mut s = if d > 0.0 { stop.max(e) } else { stop.min(e) };
                if k + 1 >= trail_n {
                    let w = k + 1 - trail_n..=k;
                    let tr = if d > 0.0 { w.map(|j| l[j]).fold(f64::INFINITY, f64::min) - tick } else { w.map(|j| h[j]).fold(f64::NEG_INFINITY, f64::max) + tick };
                    s = if d > 0.0 { s.max(tr) } else { s.min(tr) };
                }
                stop = s;
            }
            if k >= to { break false; }
            k += 1;
        };
        let mut t = Trade { dir: d, legs: vec![Leg { i, px: e, w: 1.0 }], exit_i: k, exit_px: c[k], reason: why.into(),
            entry_kind: "stop", signal_i: k, open: !closed, exits: Vec::new() };
        if !closed {
            let done: f64 = ex.iter().map(|e| e.2).sum();
            ex.push((k, c[k], 1.0 - done));
        }
        t.exit_px = ex.iter().map(|e| e.1 * e.2).sum();
        if ex.len() > 1 { t.exits = ex; }
        out.push(t);
        if !closed { break; }
        i = k + 1;
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(0.0) as usize;
    let side = u("side");
    let dirs: Vec<f64> = [(side != 2, 1.0), (side != 1, -1.0)].iter().filter(|a| a.0).map(|a| a.1).collect();
    let n = x.c.len();
    let mut orders: Vec<Vec<Order>> = vec![Vec::new(); n + 1];
    let (tick, nh) = (g("tick"), u("nh").max(1));
    // setups may begin before `from`; orders before `from` are never used. Data after `to` is never read for orders <= to.
    for &d in &dirs {
        match id {
            "dl10b_egp" => egp_orders(x, d, 0, nh, tick, g("stop_off"), &mut orders),
            "dl10b_rgs" => rgs_orders(x, d, 0, nh, u("within").max(1), tick, g("stop_off"), &mut orders),
            _ => return None,
        }
    }
    Some(manage(x, from, to, &orders, tick, u("trail_n").max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bars(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    /// Hand-checked Explosion Gap Pivot (nh = 3): gap bar 3 (low 12 > prior high 11), test bar 4 (low 11.5 < 12, > 11),
    /// right side bar 5 (low 11.8): buy 10 cents above 12.4 on bar 6; stop 11.5 - .25 = 11.25; risk 1.25; target 13.75 on bar 7.
    #[test]
    fn egp_hand_example() {
        let r = bars(&[(10.0, 10.5, 9.5, 10.0), (10.0, 10.8, 9.8, 10.5), (10.5, 11.0, 10.2, 10.8), (12.2, 13.0, 12.0, 12.8),
            (12.5, 12.6, 11.5, 12.0), (12.0, 12.4, 11.8, 12.3), (12.3, 12.9, 12.2, 12.8), (12.8, 14.0, 12.7, 13.9), (13.9, 14.2, 13.5, 13.6), (13.6, 13.7, 12.0, 12.2)]);
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new();
        p.insert("nh".into(), 3.0);
        p.insert("side".into(), 1.0);
        let t = run("dl10b_egp", &x, 0, 9, &p).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, 6);
        assert!((t[0].legs[0].px - 12.5).abs() < 1e-9);
        assert_eq!(t[0].exits[0], (7, 13.75, 0.5));
        // after bar 7: trail = min(low 6, low 7) - .1 = 12.1 (above breakeven 12.5? no: 12.1 < 12.5 -> 12.5); bar 8: min(12.7,13.5)-.1 = 12.6
        assert_eq!(t[0].exits[1].0, 9);
        assert!((t[0].exits[1].1 - 12.6).abs() < 1e-9);
        assert_eq!(t[0].reason, "Trailing stop");
    }
    /// Reversal gap short (nh = 3): new high bar 2, gap down bar 3, higher high/higher low bar 4, sell below on bar 5.
    #[test]
    fn rgs_short_hand_example() {
        let r = bars(&[(10.0, 10.5, 9.5, 10.0), (10.0, 10.8, 9.8, 10.5), (10.5, 11.0, 10.2, 10.8), (9.8, 10.0, 9.4, 9.6),
            (9.6, 10.1, 9.5, 10.0), (10.0, 10.05, 9.0, 9.1), (9.1, 9.2, 8.0, 8.1)]);
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new();
        p.insert("nh".into(), 3.0);
        let t = run("dl10b_rgs", &x, 0, 6, &p).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].dir, -1.0);
        assert_eq!(t[0].legs[0].i, 5);
        assert!((t[0].legs[0].px - 9.4).abs() < 1e-9); // 9.5 - .10
        // stop 10.1 + .1 = 10.2, risk 0.8, target 8.6 hit on bar 6; rest open at the close
        assert_eq!((t[0].exits[0].0, t[0].exits[0].2), (6, 0.5));
        assert!((t[0].exits[0].1 - 8.6).abs() < 1e-9);
        assert!(t[0].open);
    }
}
