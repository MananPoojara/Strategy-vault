//! John Sweeney, "No chumps and Mystery Charts", Technical Analysis of Stocks & Commodities V.8:14 (1990) pp.570-573 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/PR/V08/108chmp.pdf): the winning reader
//! entry by Erwin Goodman, four seven-day high/low reversal patterns traded on Teledyne. Book id `scgtdy`. Lane K41 (batch T7a,
//! 4 Nov 2026); independent check authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scgtdy_reversal"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Seven-day high = the bar's high is the highest high of the last 7 bars including it (ties count); seven-day low mirrored. Close position q = (close - low) / (high - low). Sell patterns (buys mirrored): (1) one-day reversal: a 7-day high closing below the previous close; (2) two-day reversal: a 7-day high with q >= 0.75, then a lower high with q <= 0.25; (3) half-day range: a 7-day high with 0.40 <= q <= 0.60 (on a 7-day low it is a buy); patterns 1-3 enter at the next open. (4) inside-range day: a 7-day high, then an inside day (lower high, higher low) with q > 0.25 (buy: after a 7-day low, q < 0.75): a stop order for the next day only, below the inside day's low (buy: above its high). The printed offset is $1 on Teledyne (about $300-$377 a share in 1989); a fixed dollar amount on split-adjusted US stocks is not the author's amount (decision 166), so the offset is 0.3% of price (the user's 7 Oct rule: offsets may be stated as a percentage; option off); a stop the open gaps through fills at the open. Always in: 'If we enter and reverse our position at each signal'; a signal for the side already held is ignored; a buy and a sell on the same bar cancel; a next-open signal takes the place of a stop order from the same bar. No stops ('we do not operate with stop losses'). Author example: Teledyne Feb-Aug 1989, 14 trades, 12 winners, +$91 a share (chart only, no dates). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scgtdy_reversal", name: "Seven-day high/low reversal patterns (one-day, two-day, half-range, inside day), always in", group: "S&C: Teledyne reversals (Goodman, Sweeney column V.8:14)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scgtdy", rules: &[
            "F|\"the first thing I looked for is a day whose high is the highest reached in at least seven days of trading. This is a sell signal. As a buy signal, I look for the lowest low over at least seven days.\" (V.8:14, PDF p.1)",
            "E|\"The one-day reversal, which consists of a minimum seven-day high (or low) with a close below (or above) the previous day's close. Entry is then made on the opening of the following day.\" (PDF p.2)",
            "E|\"The two-day reversal, consisting of a minimum seven-day high whose closing price is within 25% of the day's range from the high of the day. The next trading day's high is lower than the previous day's high and the closing price is within 25% of the day's range from the low of the day. A sale is then made at the next day's opening. Conditions for a buy would be just the reverse.\" (PDF p.2)",
            "E|\"The half-day range reversal, which consists of a minimum seven-day high (or low) whose closing price is within 40% to 60% of the day's range ... Purchase or sale is made at the next day's opening.\" (PDF p.2)",
            "E|\"The inside-range day reversal ... a minimum seven-day high (or low) followed by the next trading day whose high is lower than the previous day's high and whose low is higher than the previous day's low. The closing price is more than 25% from the bottom of the day's range in the case of a sell signal and more than 25% from the top of the day's range in the case of a buy signal. Entry is made the next day on a stop order $1 below the low of the inside-range day in the case of a sale, and $1 above the high\" (PDF p.2)",
            "X|\"If we enter and reverse our position at each signal, there will be 14 complete trades\"; \"we do not operate with stop losses\" (PDF p.2)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scgtdy_reversal", "Seven-day reversal patterns (S&C)", "Four patterns, always in")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "off", label: "Inside-day stop offset, % of price (the article's $1 on Teledyne at about $300-$377)", def: 0.3, min: 0., max: 2., step: 0.05 }])
}

/// Signal at the close of t: (next-open side, stop order (side, level) for t+1).
pub fn signal(x: &Ctx, t: usize, off: f64) -> (f64, Option<(f64, f64)>) {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    if t < 6 { return (0.0, None); }
    let hi7 = |i: usize| i >= 6 && (i - 6..=i).all(|j| h[i] >= h[j]);
    let lo7 = |i: usize| i >= 6 && (i - 6..=i).all(|j| l[i] <= l[j]);
    let q = |i: usize| if h[i] > l[i] { (c[i] - l[i]) / (h[i] - l[i]) } else { f64::NAN };
    let (mut sell, mut buy) = (false, false);
    if hi7(t) && c[t] < c[t - 1] { sell = true; }
    if lo7(t) && c[t] > c[t - 1] { buy = true; }
    if hi7(t - 1) && q(t - 1) >= 0.75 && h[t] < h[t - 1] && q(t) <= 0.25 { sell = true; }
    if lo7(t - 1) && q(t - 1) <= 0.25 && l[t] > l[t - 1] && q(t) >= 0.75 { buy = true; }
    if hi7(t) && q(t) >= 0.40 && q(t) <= 0.60 { sell = true; }
    if lo7(t) && q(t) >= 0.40 && q(t) <= 0.60 { buy = true; }
    if sell || buy { return (if sell && buy { 0.0 } else if sell { -1.0 } else { 1.0 }, None); }
    let inside = h[t] < h[t - 1] && l[t] > l[t - 1];
    let s4 = inside && hi7(t - 1) && q(t) > 0.25;
    let b4 = inside && lo7(t - 1) && q(t) < 0.75;
    match (s4, b4) { (true, false) => (0.0, Some((-1.0, l[t] * (1.0 - off)))), (false, true) => (0.0, Some((1.0, h[t] * (1.0 + off)))), _ => (0.0, None) }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let off = p.get("off").copied().unwrap_or(0.3).max(0.) / 100.0;
    let mut pos = Book::new();
    let (mut pend, mut stop): (f64, Option<(f64, f64)>) = (0.0, None);
    for t in from..=to {
        if pend != 0.0 && pend != pos.side() { pos.enter(pend, t, x.o[t], "open"); }
        if let Some((d, lv)) = stop {
            if d != pos.side() {
                if d > 0.0 && x.h[t] >= lv { pos.enter(d, t, x.o[t].max(lv), "stop"); }
                if d < 0.0 && x.l[t] <= lv { pos.enter(d, t, x.o[t].min(lv), "stop"); }
            }
        }
        pend = 0.0; stop = None;
        if t < to { let (n, s) = signal(x, t, off); pend = n; stop = s; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reversal_patterns_trade_both_sides() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scgtdy_reversal", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.0) && t.iter().any(|z| z.dir < 0.0));
        assert!(t.windows(2).all(|w| w[0].dir != w[1].dir && w[0].exit_i == w[1].legs[0].i));
    }
}
