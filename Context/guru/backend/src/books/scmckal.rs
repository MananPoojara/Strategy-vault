//! Curtis McKallip Jr., "The Damping Index", Technical Analysis of Stocks & Commodities V.10:7 (July 1992) pp.296-299 (research
//! share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V10/C07/DAMPING.PDF; Figure 7 =
//! the TradeStation code, an image page, read with authors/ocr2). Book id `scmckal`. Lane K40 (4 Nov 2026, decision #203);
//! independent check authors/titles-1030/K40w/k40_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo, Pos};

pub const REVIEWED: &[&str] = &["scmckal_damping"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The article's own test rules are its Figure 7 TradeStation code (\"The actual trading rules used are listed in Figure 7\"); where the text and the code differ the code is the default: lenday = 4 (code) against the text's 'bar-1 to bar-5 / bar-6 to bar-10' (option n = 5 gives the text's windows), and 'value1 <= thresh' (the text says 'less than 1'). Damping index = [avg(H, n)[1] - avg(L, n)[1]] / [avg(H, n)[6] - avg(L, n)[6]] ([k] = the average ending k bars ago); a zero denominator gives no signal (the text: 'If the denominator is not zero'). Buy: close > avg(H, 2)[1] and DI <= 1 and avg(L, 3)[1] > avg(H, 4)[10]; sell short: close < avg(L, 2)[1] and DI <= 1 and avg(H, 3)[1] < avg(L, 4)[10]. 'at market' = the next bar's open; an opposite signal reverses there, a same-side signal only resets the stop level. Long stop = lowest(L, 4)[1] of the latest buy-signal bar, short stop = highest(H, 4)[1] of the latest sell-signal bar, placed from the bar after the entry (the code places them only while a position is held) and filled at the level or the open beyond it. Daily bars; the article's test is weekly Japanese yen futures 1978-1992 (Figure 6: 14 trades, 57% profitable, net $85,225), not held, so no book comparison. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scmckal_damping", name: "McKallip Damping Index: narrowing highs and lows (index <= 1) with a trend filter, buy above / sell below the last two bars, stop at the 4-bar extreme", group: "S&C: The Damping Index (McKallip, V.10:7)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scmckal", rules: &[
            "F|\"value1 = (@average(high,lenday)[1] - @average(low,lenday)[1]); value2 = (@average(high,lenday)[6] - @average(low,lenday)[6]); if value2 > 0 then value1 = value1 / value2;\" inputs lenday(4), thresh(1), bout(2) (Figure 7, PDF p.6)",
            "E|\"if close > @average(high,bout)[1] and value1 <= thresh and @average(low,3)[1] > @average(high,4)[10] then begin ... buy(\\\"newlg\\\") 1 contract at market; value4 = @lowest(low,4)[1];\" and the mirrored sell (Figure 7); text: \"the latter condition ensures that the market is in an uptrend\" (PDF p.7)",
            "X|\"if @marketposition = 1 then exitlong('protsell') ... at value4 stop;\" / \"exitshort('protbuy') ... at value5 stop\" (Figure 7); \"Long trades are exited when the price moves below the lowest low of the last four bars.\" (PDF p.7)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scmckal_damping", "McKallip Damping Index (S&C)", "lenday 4, threshold 1")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "lenday: bars in each average (code 4; the text's bar-1..bar-5 reading = 5)", 4., 2., 10., 1.),
        p("thresh", "Damping index at or below (code 1)", 1., 0.2, 3., 0.05),
        p("bout", "Breakout average of the last bars (code 2)", 2., 1., 10., 1.)])
}

fn avg(v: &[f64], n: usize, back: usize, t: usize) -> f64 { if t < back + n - 1 { f64::NAN } else { v[t - back - n + 1..=t - back].iter().sum::<f64>() / n as f64 } }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (n, thresh, bout) = (g("n").round().max(1.) as usize, g("thresh"), g("bout").round().max(1.) as usize);
    let (h, l, c) = (&x.h, &x.l, &x.c);
    // the signal at bar t: +1 buy, -1 sell, 0 none
    let sig = |t: usize| -> i8 {
        if t < (5 + n).max(13) { return 0; }
        let den = avg(h, n, 6, t) - avg(l, n, 6, t);
        if !(den > 0.) { return 0; }
        let di = (avg(h, n, 1, t) - avg(l, n, 1, t)) / den;
        if !(di <= thresh) { return 0; }
        if c[t] > avg(h, bout, 1, t) && avg(l, 3, 1, t) > avg(h, 4, 10, t) { 1 }
        else if c[t] < avg(l, bout, 1, t) && avg(h, 3, 1, t) < avg(l, 4, 10, t) { -1 } else { 0 }
    };
    let mut pos = Pos::new();
    let (mut v4, mut v5) = (f64::NAN, f64::NAN);
    let mut pend = 0i8; // market order for this bar's open
    let mut stop = f64::NAN; // protective stop for this bar (placed at the previous close while a position was held)
    for t in from..=to {
        if pend != 0 { pos.enter(pend as f64, t, x.o[t], "open"); }
        else if stop.is_finite() {
            if pos.side() > 0. && l[t] <= stop { pos.exit(t, x.o[t].min(stop), "lowest low of 4 bars"); }
            else if pos.side() < 0. && h[t] >= stop { pos.exit(t, x.o[t].max(stop), "highest high of 4 bars"); }
        }
        pend = 0;
        let s = sig(t);
        if s == 1 { v4 = lo(l, t - 4, t - 1); } else if s == -1 { v5 = hi(h, t - 4, t - 1); }
        if t < to && s as f64 != pos.side() && s != 0 { pend = s; }
        stop = if pos.side() > 0. { v4 } else if pos.side() < 0. { v5 } else { f64::NAN };
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn damping_entries_at_next_open_and_stops_from_the_signal_bar() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scmckal_damping", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in &t { let i = z.legs[0].i; assert_eq!(z.legs[0].px, x.o[i]); assert!(z.exit_i > i || z.open); }
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
        assert!(run("scmckal_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}

// Instrument admission: S&C McKallip Damping Index 2026-11-04 (titles lane K40).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scmckal" && reviewed(def.id),
    label: "Curtis McKallip tested the Damping Index system on weekly Japanese yen futures (TradeStation, 1978-1992). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.10:7 pp.296-299 (The Damping Index), Figure 7 TradeStation code; independent check authors/titles-1030/K40w/k40_ref.py",
};
