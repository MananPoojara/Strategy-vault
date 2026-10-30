//! Larry Connors / TradingMarkets, "Chairman's Club December 2007" (Las Vegas, Dec 1-2 2007; research share Trading
//! Knowledge II/@TradingMarkets.com & TradersLibrary.com/Chairman's Club December-2007 (tradingmarkets.com)/CC 2007 (Dec
//! 1,2) - Las Vegas Manual.pdf, image slides; refs sN = printed slide number = PDF page). Book id `ccl07`.
//! Live: RSI 10 Scale-In (s78-80) and 10 Worst Performers (s214-216). Not coded: the Short Selling Methodology (s197-201)
//! is the live STC IX rule `h10` (same LF > 20, RSI(2) > 90, HV(100) > 40, ADX(10) > 40, close > 200-day MA, 10-day
//! high, limit close + 0.4 x ATR(10), cover next open after RSI(2) < 40) - a duplicate, so no copy here.
//! Lane K3 (titles loop, 30 Oct 2026); independent check authors/ccl07/admission/k3_ref.py (header).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/ccl07/admission/k3_ref.py (header)).
pub const REVIEWED: &[&str] = &["ccl07_rsi10_scale", "ccl07_worst10"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_R: &str = "[Declared readings] The slide trades S&P e-minis with SPY as the filter; here the traded series is its own filter (SPY, the S&P 500 and other indices, ETFs and stocks are a declared application). RSI(2) is the vault's Connors RSI (Wilder), the 200-day MA a simple average of closes. Units 2/3/5 = 20%/30%/50% of a full position. [Reading, open question] The slide does not say when the adds are bought: here at the close that is lower than the earlier entry price (the entry day's own close counts); on a close where the exit rule fires, nothing is added. One position at a time; a position still open at the window end is marked at the last close.";
const NOTE_W: &str = "[Declared readings] A stock method: the slide's results (s216) are a portfolio of US stocks and the rules do not explain the '10' in the title (open question: ten slots, or a pick of the ten worst). Here it runs per instrument on actual US and NSE stocks; indices and ETFs are a declared application. 'LF 3' is printed without an operator and is read as LF > 3 like the other Chairman's Club decks; LF = the vault Liquidity Filter (TradingMarkets 1/19/10: 1,000,000 x SMA20(close x volume) / SMA65(benchmark close x volume), benchmark S&P 500 / NIFTY 50). 'Trading for 1 year' and 'worst performance in the past year' = today's 5-day % change (close vs the close 5 days earlier) is the lowest (ties count) of the last 252 daily 5-day changes, today included, all of which must exist. The exit is checked from the entry day's close. One position at a time.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, vol: bool, rules: &'static [&'static str], notes: &'static str| StrategyDef { id, name, group: "Connors Chairman's Club Dec 2007",
        session: 0, dir: "Long", status: Status::Live, needs_volume: vol, needs_vix: false, needs_series: &[], book: "ccl07", rules, notes, claim: None };
    vec![
        d("ccl07_rsi10_scale", "RSI 10 Scale-In: above the 200-day, buy 2-3-5 units after RSI(2) closes under 10, out next open after RSI(2) > 75", false, &[
            "F|\"1. SPY is above the 200-day MA.\" (s79)",
            "E|\"2. Buy 2 e-minis on the next days open if the 2-period RSI closes under 10.\" (s79)",
            "A|\"3. Buy 3 more e-minis if prices closes lower than entry price anytime you're in the initial position.\" (s79) [reading] bought at that close",
            "A|\"4. Buy 5 more e-mini's if prices closes lower than second entry price anytime you're in the second position.\" (s79) [reading] bought at that close",
            "X|\"5. Exit RSI 75 on the next days open.\" (s79) = after RSI(2) closes above 75, sell everything at the next open"], NOTE_R),
        d("ccl07_worst10", "10 Worst Performers: above the 200-day, 5-day change the worst of the past year, buy next open, out next open after a close above the 5-day MA", true, &[
            "F|\"1. Close is above the 200MA\" (s215)",
            "F|\"2. LF 3\" (s215) [reading] LF > 3",
            "F|\"3. Stock has to have been trading for 1 Year\" (s215)",
            "E|\"4. The 5 day performance of the stock is its worst performance in the past year.\" (s215)",
            "E|\"5. Buy at next open\" (s215)",
            "X|\"6. Sell when the close is above the 5 ma (next days open).\" (s215)"], NOTE_W),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "ccl07_rsi10_scale" => Some(("ccl07_rsi10_scale", "Chairman's Club 2007 RSI 10 Scale-In", "2-3-5 units")),
        "ccl07_worst10" => Some(("ccl07_worst10", "Chairman's Club 2007 10 Worst Performers", "5-day change, 1 year")),
        _ => None,
    }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "ccl07_rsi10_scale" => Some(vec![p("trend", "Trend average, days (slide 200)", 200., 10., 300., 10.), p("rsi_lt", "Entry: RSI(2) closes under (slide 10)", 10., 1., 50., 1.),
            p("exit_rsi", "Exit: RSI(2) closes above (slide 75)", 75., 50., 95., 5.)]),
        "ccl07_worst10" => Some(vec![p("trend", "Trend average, days (slide 200)", 200., 10., 300., 10.), p("lf", "Liquidity Filter above (slide 3)", 3., 0., 40., 1.),
            p("perf", "Performance period, days (slide 5)", 5., 1., 20., 1.), p("year", "Past year, trading days (reading 252)", 252., 20., 504., 1.), p("ma", "Exit average, days (slide 5)", 5., 2., 20., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let (o, c) = (&x.o, &x.c);
    let tr = x.sma_n(u("trend"));
    let n = c.len();
    // entry signal on bar i's close (bought at the next open); exit signal on a close (sold at the next open)
    let (entry, exit, w, why): (Vec<bool>, Vec<bool>, &[f64], String) = if id == "ccl07_rsi10_scale" {
        let (rs, lt, ex) = (x.rsi_n(2), g("rsi_lt"), g("exit_rsi"));
        ((0..n).map(|i| c[i] > tr[i] && rs[i] < lt).collect(), (0..n).map(|i| rs[i] > ex).collect(), &[0.2, 0.3, 0.5], format!("RSI(2) > {ex}, sold next open"))
    } else {
        let (lf, k, yr, ma) = (g("lf"), u("perf"), u("year"), x.sma_n(u("ma")));
        let r: Vec<f64> = (0..n).map(|i| if i >= k { c[i] / c[i - k] - 1. } else { f64::NAN }).collect();
        let worst = |i: usize| i + 1 >= yr + k && r[i + 1 - yr..=i].iter().all(|&v| r[i] <= v);
        ((0..n).map(|i| c[i] > tr[i] && x.lf[i] > lf && worst(i)).collect(), (0..n).map(|i| c[i] > ma[i]).collect(), &[1.0], format!("Close > {}-day MA, sold next open", u("ma")))
    };
    let (mut out, mut cur, mut pend_in) = (vec![], None::<Trade>, false);
    for i in from..=to {
        if pend_in {
            pend_in = false;
            cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: o[i], w: w[0] }], exit_i: i, exit_px: o[i], reason: String::new(), entry_kind: "open", signal_i: i, open: false, exits: vec![] });
        }
        if let Some(t) = cur.as_mut() {
            if exit[i] {
                if i < to { t.exit_i = i + 1; t.exit_px = o[i + 1]; t.reason = why.clone(); t.signal_i = i; out.push(cur.take().unwrap()); }
            } else if t.legs.len() < w.len() && c[i] < t.legs.last().unwrap().px {
                let k = t.legs.len();
                t.legs.push(Leg { i, px: c[i], w: w[k] });
            }
            continue;
        }
        if i < to && entry[i] { pend_in = true; }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.reason = "open".into(); t.signal_i = to; t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn rsi10_scale_adds_on_lower_closes_and_exits_next_open() {
        // 220 rising bars (above the 200-day), then a slide (RSI(2) < 10), more lower closes, then a rally (RSI(2) > 75)
        let mut cl: Vec<f64> = (0..220).map(|i| 100. + i as f64).collect();
        cl.extend([310., 300., 299., 298., 330., 340.]);
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i % 28 + 1), open: c + 0.5, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ccl07_rsi10_scale", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        let z = &t[0];
        // signal at 221 (close 300), bought at 222's open 299.5; 222 closes 299 < 299.5 -> add; 223 closes 298 < 299 -> add
        assert_eq!(z.legs.iter().map(|l| (l.i, l.px, l.w)).collect::<Vec<_>>(), vec![(222, 299.5, 0.2), (222, 299., 0.3), (223, 298., 0.5)]);
        assert_eq!((z.exit_i, z.exit_px, z.open), (225, 340.5, false));
        assert!(run("ccl07_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}
