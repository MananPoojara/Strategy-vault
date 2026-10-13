//! Ryan Jones, *The Secret to Making $1 Million in 5 Years or Less* - Advanced Course workbook (Futures Learning Center,
//! 1998; 75 scanned pages). Sessions 5-7 are money management (Fixed Ratio - not strategies). Session 8 "Systems & Rules"
//! (p.53-72) gives three mechanical daily systems with their Omega SuperCharts code and futures results. The code is the
//! exact statement of each rule and wins over the prose where they differ (noted per rule). The Basic Course workbook
//! (55 pages) is money management only. Source + audit: library/rjmil/.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rjmil";

const NOTE_MA: &str = "Book market: futures (SuperCharts results p.56-59: Crude Oil, T-Bonds, Wheat, Soybeans, 1989/90-1997, one contract, no stops). \
Code (p.55) wins over prose (p.53): the short average is t2=10 days in the code that produced the results, the prose says 'the last 5 days' (param t2=5 for the prose). \
SuperCharts 'Buy/Sell at Market' on a signal bar = next bar's open. No exit order exists: each opposite entry reverses (always in once started); a same-direction signal while in that direction is ignored. Runs on the indices (instrument swap).";
const NOTE_FV: &str = "Book market: futures (results p.61-64: Crude Oil, Wheat, 10-yr Notes, T-Bonds, 1990-1997). Code (p.60) wins over prose (p.53): exit 'barssinceentry>x1' with x1=5 (the prose says 'Exit 10 days later'; param hold=10). \
'open tomorrow < average(low,4)' is the next bar's open compared with the signal bar's 4-day average low; then a buy stop at that average fills at the level if the bar's high reaches it (the open is below it). Exit 'at Market' when bars since entry > x1 = next open (entry bar = 0). \
An opposite entry while in a trade reverses at its stop price. Runs on the indices (instrument swap).";
const NOTE_BT: &str = "Book market: futures (results p.67-72, 1989/90-1997). Code (p.66): 'Buy next bar at open tomorrow - XP points Stop' and 'Sell next bar at open tomorrow - xp points Stop', XP=12, xdays=10. \
Literal TradeStation behaviour: a buy stop BELOW the open is already through, so the long fills at the open; the short needs the low to reach open - 12 points. The prose (p.54) is self-contradictory ('open of the next bar plus X number of points' / 'open tomorrow - X points'): param long_plus=1 puts the buy stop at open + XP (labelled, not default). \
TradeStation 'points' = the price's smallest decimal (1/PriceScale): 0.01 on SPX and NIFTY, so 12 points = 0.12 (param point). Exit when bars since entry > xdays at the next open; an opposite entry reverses. Runs on the indices (instrument swap).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("rjmil_mama_div", "MA-MA Divergence (40-day average up, 10-day average down; stop and reverse)", "Session 8 · Systems & Rules", "Long/Short", Status::Live, &[
            "E|Code (p.55): \"If average(close,t1) > average(close,t1)[x1] and average(close,t2) < average(close,t2)[x1] Then Buy at Market\", t1=40, x1=5, t2=10.",
            "E|Code (p.55): \"If average(close,t1) < average(close,t1)[x1] and average(close,t2) > average(close,t2)[x1] Then Sell at Market\".",
            "E|Prose (p.53): \"If the average close of the last 40 days is greater than the average close of the last forty days 5 days ago and the average close of the last 5 days is less than the average close of the last 5 days, 5 days ago THEN BUY NEXT DAY ON THE OPEN.\" (short average 5 in prose, 10 in code)",
            "X|No exit or stop in the code (all stops 'No', p.55): \"SuperCharts results are given on the following pages with no stops\" (p.53) - the opposite entry reverses.",
        ], NOTE_MA),
        d("rjmil_futurevideo", "FutureVideo (pullback below a 4-day low above the 38-day average; buy stop at the 4-day average low)", "Session 8 · Systems & Rules", "Long/Short", Status::Live, &[
            "E|Code (p.60): \"If Low < low[t1] and low > average(close,t2) and open tomorrow < average(low,t1) Then Buy next bar at average(low,t1) Stop\", t1=4, t2=38.",
            "E|Code (p.60): \"If high > high[t1] and high < average(close,t2) and open tomorrow > average(high,t1) Then Sell next bar at average(high,t1) Stop\".",
            "X|Code (p.60): \"If barssinceentry>x1 Then ExitLong at Market\" (ExitShort the same), x1=5. Prose (p.53): \"Exit 10 days later\"; \"exits are simply 10 days later with no stops\".",
        ], NOTE_FV),
        d("rjmil_bigtrend", "Big Trend (3-day low above the 30-day average, low below the low 5 days ago; time exit)", "Session 8 · Systems & Rules", "Long/Short", Status::Live, &[
            "E|Code (p.66): \"If lowest(low,T1) > average(close,X1) and low < low[t2] Then Buy next bar at open tomorrow - XP points Stop\", t1=3, x1=30, t2=5, XP=12.",
            "E|Code (p.66): \"If highest(high,t1) < average(close,x1) and high > high[t2] Then Sell next bar at open tomorrow - xp points Stop\".",
            "X|Code (p.66): \"If barssinceentry>xdays Then ExitLong at Market\" (ExitShort the same), xdays=10. Prose (p.54): \"Exit X days later or use as a reversal.\"",
        ], NOTE_BT),
        d("rjmil_money_management", "Fixed Ratio / Fixed Fractional sizing (Basic and Advanced workbooks)", "Sessions 1-7 · Money management", "Long/Short", Status::NotTestable(
            "position sizing, not an entry/exit rule (Fixed Ratio delta, rate of decrease, Fixed Fractional; Basic p.1-55, Advanced p.1-52)"), &[
            "F|\"Fixed Ratio requires adds contracts only as profits are achieved\" (Advanced p.2)"], "Noted only; the vault backtests one unit per trade."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "rjmil_mama_div" => Some(("rjmil_systems", "Million-Dollar workbook systems", "MA-MA Divergence")),
        "rjmil_futurevideo" => Some(("rjmil_systems", "Million-Dollar workbook systems", "FutureVideo")),
        "rjmil_bigtrend" => Some(("rjmil_systems", "Million-Dollar workbook systems", "Big Trend")),
        "rjmil_money_management" => Some(("rjmil_systems", "Million-Dollar workbook systems", "Money management")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "rjmil_mama_div" => Some(vec![
            p("t1", "Long average (days)", 40.0, 10.0, 200.0, 1.0),
            p("t2", "Short average (days; code 10, prose 5)", 10.0, 2.0, 50.0, 1.0),
            p("x1", "Slope lookback (days)", 5.0, 1.0, 20.0, 1.0),
        ]),
        "rjmil_futurevideo" => Some(vec![
            p("t1", "Short lookback / average low-high (days)", 4.0, 2.0, 20.0, 1.0),
            p("t2", "Average close (days)", 38.0, 10.0, 200.0, 1.0),
            p("hold", "Exit when bars since entry > this (code 5, prose 10)", 5.0, 1.0, 30.0, 1.0),
        ]),
        "rjmil_bigtrend" => Some(vec![
            p("t1", "Lowest low / highest high (days)", 3.0, 1.0, 10.0, 1.0),
            p("x1", "Average close (days)", 30.0, 10.0, 200.0, 1.0),
            p("t2", "Low below the low N days ago", 5.0, 1.0, 20.0, 1.0),
            p("xp", "Stop offset XP (points)", 12.0, 0.0, 500.0, 1.0),
            p("point", "Price of one point (TradeStation: 1/PriceScale = 0.01)", 0.01, 0.01, 1.0, 0.01),
            p("xdays", "Exit when bars since entry > xdays", 10.0, 1.0, 40.0, 1.0),
            p("long_plus", "Buy stop at open + XP (prose reading, 1) / open - XP (code, 0)", 0.0, 0.0, 1.0, 1.0),
        ]),
        _ => None,
    }
}

/// Sign of average(v, n)[t] - average(v, n)[t-lag] (1, -1, or 0 for an exact tie), from fresh window sums with a
/// relative tolerance so that a tie in the printed prices is a tie here too (book: strictly greater / less).
pub fn slope_sign(v: &[f64], t: usize, n: usize, lag: usize) -> Option<i8> {
    if t + 1 < n + lag { return None; }
    let a: f64 = v[t + 1 - n..=t].iter().sum();
    let b: f64 = v[t + 1 - n - lag..=t - lag].iter().sum();
    if a.is_nan() || b.is_nan() { return None; }
    let tol = 1e-9 * (a.abs() + b.abs());
    Some(if a - b > tol { 1 } else if b - a > tol { -1 } else { 0 })
}

fn tr(dir: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}

fn close_out(cur: &mut Option<Trade>, out: &mut Vec<Trade>, i: usize, px: f64, sig: usize, why: &str) {
    if let Some(mut o) = cur.take() { o.exit_i = i; o.exit_px = px; o.signal_i = sig; o.reason = why.into(); out.push(o); }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !matches!(id, "rjmil_mama_div" | "rjmil_futurevideo" | "rjmil_bigtrend") { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let n = x.c.len();
    if n == 0 || to >= n || from > to { return Some(vec![]); }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    if id == "rjmil_mama_div" {
        let (t1, t2, x1) = ((g("t1") as usize).max(1), (g("t2") as usize).max(1), (g("x1") as usize).max(1));
        let mut pending: Option<(f64, usize)> = None;
        for t in 0..=to {
            if let Some((d, s)) = pending.take() {
                close_out(&mut cur, &mut out, t, o[t], s, "Opposite signal: reverse at the open");
                cur = Some(tr(d, t, o[t], "open", s));
            }
            if t == to || t < x1 { continue; }
            let (Some(su), Some(sv)) = (slope_sign(c, t, t1, x1), slope_sign(c, t, t2, x1)) else { continue };
            let d = if su > 0 && sv < 0 { 1.0 } else if su < 0 && sv > 0 { -1.0 } else { continue };
            if cur.as_ref().map_or(true, |q| q.dir != d) { pending = Some((d, t)); }
        }
    } else {
        // stop entries placed for the next bar + time exit at the next open after `hold` bars
        let fv = id == "rjmil_futurevideo";
        let (t1, t2) = ((g("t1") as usize).max(1), (g("t2") as usize).max(1));
        let hold = if fv { g("hold") } else { g("xdays") }.max(0.0) as usize;
        let ac = if fv { sma(c, t2) } else { sma(c, (g("x1") as usize).max(1)) };
        let (al, ah) = (sma(l, t1), sma(h, t1));
        let off = if fv { 0.0 } else { g("xp") * g("point") };
        let plus = !fv && g("long_plus") >= 0.5;
        let mut exit_next: Option<usize> = None; // signal bar of a pending time exit
        let mut order: Option<(bool, bool, usize)> = None; // (buy setup, sell setup, signal bar) for the next bar
        for t in 0..=to {
            if let Some(s) = exit_next.take() { close_out(&mut cur, &mut out, t, o[t], s, "Time exit: bars since entry > limit, at the open"); }
            if let Some((b, sl, s)) = order.take() {
                // levels known at the open of t
                let (bl, sl_lvl) = if fv { (al[s], ah[s]) } else { (if plus { o[t] + off } else { o[t] - off }, o[t] - off) };
                let buy_ok = b && (!fv || o[t] < bl) && h[t] >= bl;
                let sell_ok = sl && (!fv || o[t] > sl_lvl) && l[t] <= sl_lvl;
                let cd = cur.as_ref().map(|q| q.dir);
                let try_go = |d: f64, px: f64, cur: &mut Option<Trade>, out: &mut Vec<Trade>| {
                    if cur.as_ref().map(|q| q.dir) == Some(d) { return; }
                    close_out(cur, out, t, px, t, "Opposite entry stop: reverse");
                    *cur = Some(tr(d, t, px, "stop", s));
                };
                if buy_ok && !sell_ok && cd != Some(1.0) { try_go(1.0, o[t].max(bl), &mut cur, &mut out); }
                else if sell_ok && !buy_ok && cd != Some(-1.0) { try_go(-1.0, o[t].min(sl_lvl), &mut cur, &mut out); }
            }
            if t == to { break; }
            if let Some(q) = cur.as_ref() { if t - q.legs[0].i > hold { exit_next = Some(t); } }
            if t < t1.max(t2) { continue; }
            let (b, sl) = if fv {
                (l[t] < l[t - t1] && !ac[t].is_nan() && l[t] > ac[t], h[t] > h[t - t1] && !ac[t].is_nan() && h[t] < ac[t])
            } else {
                let lo = l[t + 1 - t1..=t].iter().copied().fold(f64::INFINITY, f64::min);
                let hi = h[t + 1 - t1..=t].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                (!ac[t].is_nan() && lo > ac[t] && l[t] < l[t - t2], !ac[t].is_nan() && hi < ac[t] && h[t] > h[t - t2])
            };
            if b || sl { order = Some((b, sl, t)); }
        }
    }
    if let Some(mut q) = cur { q.exit_i = to; q.exit_px = c[to]; q.signal_i = to; q.reason = "open".into(); q.open = true; out.push(q); }
    out.retain(|t| t.legs[0].i >= from);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bars(n: usize) -> Vec<Row> {
        (0..n).map(|i| { let v = 100.0 + i as f64; Row { ticker: "T".into(), date: format!("2000-01-{:02}", 1 + i % 28), open: v, high: v + 1.0, low: v - 1.0, close: v, volume: None } }).collect()
    }
    #[test]
    fn bigtrend_long_fills_at_open_and_exits_after_ten_bars() {
        let mut r = bars(80);
        // bar 50: lowest low of 3 days above SMA30 (~135) and low < low 5 days ago
        r[50].low = 143.0; // low[45] = 144
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("rjmil_bigtrend", &x, 0, 79, &Params::new()).unwrap();
        let t = ts.iter().find(|t| t.legs[0].i == 51).expect("long on bar 51");
        // code literal: buy stop at open - 0.12 is below the open -> fills at the open; exit when bars since entry > 10 -> open of bar 51+12
        assert_eq!((t.dir, t.legs[0].px, t.exit_i, t.exit_px), (1.0, 151.0, 63, 163.0));
    }
}
