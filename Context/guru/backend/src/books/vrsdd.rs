//! Vladimir Ribakov, "The Secret of the Double Doji Forex Trading Strategy" (27 December 2009, 13-page e-book; book id
//! `vrsdd`; printed page numbers). Reading: claude-continuation-20261020/authors/ribakov (admission/dd_ref.py). Live:
//! two dojis in a row, a buy stop above their upper border and a sell stop below their lower border (one cancels the
//! other), stop at the opposite border, target = the stop distance (trade management method 1, p.8-9; methods 2 and 3,
//! p.10-11, as options). The book measures the doji body and the order offsets in forex pips; here the doji is a body of
//! at most a set share of the candle's range and the orders sit exactly at the borders (declared readings). Book: forex
//! pairs on 1-hour, 4-hour and daily charts; indices, ETFs and US/India stocks here are a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/ribakov/admission/dd_ref.py (header)).
pub const REVIEWED: &[&str] = &["vrsdd_double_doji"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Doji: the book defines it as a candle whose open and close are the same (p.5) and accepts 'a body of just a few pips, 2-4 pips' (p.6). Pips are a forex unit, so here a doji is a candle whose body (|close - open|) is at most 10% of its high-low range (option: any share; 0 = open exactly equal to close). Two dojis in a row; the borders are the higher of their two highs and the lower of their two lows. The book's '1 pip above / below' is read as any trade beyond the border, so the buy stop sits at the upper border and the sell stop at the lower border; the stop loss is the opposite border. The book does not say how long the pending orders last: they stay until the first day that trades beyond either border; that day fills one of them (at the border, or at the open when the day opens beyond it) and the other is cancelled. If that day reaches both borders without opening beyond either, the order of the moves is unknown on daily bars and no trade is taken. Method 1 (default): the target is the stop distance from the fill. Method 2: half at 1x the stop distance, half at 2x, stop to the entry price once the first half is out. Method 3: 80% at 0.5x, 20% at 2x. Stops and targets are checked from the day after the fill (stop first when both are touched), each filled at its level or a gapped open. One position at a time; dojis during a trade are ignored. The preferred 'clear trend before' (3 candles of one colour) is advice, not a condition, and is not applied. NIFTY opens before 2011 are close to the previous close, which makes many early NIFTY candles look like dojis.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "vrsdd_double_doji", name: "Double doji: stop orders beyond both borders, stop at the other border, target = the stop distance",
        group: "Ribakov Double Doji", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "vrsdd",
        rules: &["F|\"The meaning is that the opening and closing price of the candle are the same\" (p.5); \"a candle with a body of just a few pips, 2-4 pips, is considered a Doji\"; \"the first condition for this strategy is to identify 2 Dojis one after the other\" (p.6)",
            "E|\"Place 2 pending orders. One in case the market breaks the upper border of the Dojis (1 pip above it is enough) and one in case the market breaks the lower border (1 pip below)\" (p.8); \"When one of the orders executed, I'll cancel the other one\" (p.13)",
            "X|\"For a Buy (Long) trade, place the Stop Loss 1 pip below the Doji's lower border. For a Sell (Short) trade, place the Stop Loss 1 pip above the Doji's upper border\" (p.8)",
            "X|\"Trade management method 1: Take profit is equal to the Stop Loss\" (p.8); method 2: \"2 identical trades ... The first has a Stop Loss equal to the Take Profit. The second has a profit target which is double the Stop Loss ... move the Stop Loss to the opening price of the trade ('Break Even') once the first trade exited in profit\" (p.9); method 3: \"a profit target which is half the Stop Loss ... This is where I'll close 80% of the trade ... The second profit target, for the remaining 0.2 lots, would be x2 of the Stop Loss\" (p.10-11)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "vrsdd_double_doji").then_some(("vrsdd_double_doji", "Ribakov double doji breakout", "method 1"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "vrsdd_double_doji").then(|| vec![p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.),
        p("body", "Doji: body at most this share of the range (reading; 0 = open equals close, p.5)", 0.10, 0., 0.5, 0.01),
        p("method", "Trade management method (book 1, 2 or 3)", 1., 1., 3., 1.)])
}

/// Manage an open position from the bar after entry `e`: the stop first, then the target pieces (fraction, level) in
/// order, each filled at its level or a gapped open; with `be` the stop moves to the entry price (from the next bar)
/// once the first piece is out; at bar `last` (time exit, usize::MAX = none) the rest is sold at that close.
pub fn manage(x: &Ctx, d: f64, e: usize, ep: f64, kind: &'static str, stop0: f64, pieces: &[(f64, f64)], be: bool, last: usize, to: usize) -> Trade {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (mut stop, mut j, mut done, mut why, mut open) = (stop0, 0usize, Vec::<(usize, f64, f64)>::new(), "open", true);
    let end = last.min(to);
    let rest = |j: usize| pieces[j..].iter().map(|z| z.0).sum::<f64>();
    for k in e + 1..=end {
        if (d > 0. && l[k] <= stop) || (d < 0. && h[k] >= stop) {
            done.push((k, if d > 0. { o[k].min(stop) } else { o[k].max(stop) }, rest(j)));
            why = "stop"; open = false; j = pieces.len(); break;
        }
        while j < pieces.len() {
            let lv = pieces[j].1;
            if !((d > 0. && h[k] >= lv) || (d < 0. && l[k] <= lv)) { break; }
            done.push((k, if d > 0. { o[k].max(lv) } else { o[k].min(lv) }, pieces[j].0));
            j += 1;
        }
        if j == pieces.len() { why = "target"; open = false; break; }
        if be && j > 0 { stop = ep; }
    }
    if j < pieces.len() {
        if last <= to { why = "time exit"; open = false; }
        done.push((end, c[end], rest(j)));
    }
    let xi = done.last().map_or(end, |z| z.0);
    let xp: f64 = done.iter().map(|z| z.1 * z.2).sum();
    Trade { dir: d, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: xi, open,
        exits: if pieces.len() > 1 { done } else { vec![] } }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (side, body, method) = (g("side").round() as i64, g("body"), g("method").round() as i64);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let doji = |i: usize| (c[i] - o[i]).abs() <= body * (h[i] - l[i]);
    let mut out = vec![];
    let mut t = from.max(1); // second doji
    while t < to {
        if !(doji(t - 1) && doji(t)) { t += 1; continue; }
        let (up, dn) = (h[t].max(h[t - 1]), l[t].min(l[t - 1]));
        let mut e = t + 1;
        while e <= to && h[e] <= up && l[e] >= dn { e += 1; }
        if e > to { break; }
        let (d, ep, kind) = if o[e] > up { (1., o[e], "open") } else if o[e] < dn { (-1., o[e], "open") }
            else if h[e] > up && l[e] < dn { t = e + 1; continue; } else if h[e] > up { (1., up, "stop") } else { (-1., dn, "stop") };
        if (side == 1 && d < 0.) || (side == 2 && d > 0.) { t = e + 1; continue; }
        let stop = if d > 0. { dn } else { up };
        let r = (ep - stop).abs();
        let lv = |m: f64| ep + d * m * r;
        let (pieces, be) = match method { 2 => (vec![(0.5, lv(1.)), (0.5, lv(2.))], true), 3 => (vec![(0.8, lv(0.5)), (0.2, lv(2.))], false), _ => (vec![(1., lv(1.))], false) };
        let tr = manage(x, d, e, ep, kind, stop, &pieces, be, usize::MAX, to);
        let (xi, op) = (tr.exit_i, tr.open);
        out.push(tr);
        if op { break; }
        t = xi + 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(i: usize, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: format!("2020-05-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn hand_checked_double_doji_trades() {
        let r = vec![bar(0, 100., 101., 99., 100.5),
            bar(1, 100., 101., 99., 100.1), bar(2, 100.1, 100.8, 99.5, 100.), // two dojis: borders 101 / 99
            bar(3, 100., 100.9, 99.2, 100.5), // inside: orders keep working
            bar(4, 100.5, 101.5, 100.2, 101.4), // long at 101, stop 99, target 103
            bar(5, 101.4, 103.2, 101., 103.), // target
            bar(6, 103., 103.4, 102.6, 103.), bar(7, 103., 103.3, 102.7, 103.), // dojis: borders 103.4 / 102.6
            bar(8, 102.5, 102.6, 101.8, 102.), // opens below: short at 102.5, stop 103.4, target 101.6
            bar(9, 102., 103.6, 101.9, 103.5)]; // stop
        let x = Ctx::new(&r, &r, &[]);
        let t = run("vrsdd_double_doji", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|z| (z.dir, z.legs[0].i, (z.legs[0].px * 100.).round(), z.entry_kind, z.exit_i, (z.exit_px * 100.).round(), z.reason.clone())).collect();
        assert_eq!(got, vec![(1., 4, 10100., "stop", 5, 10300., "target".to_string()), (-1., 8, 10250., "open", 9, 10340., "stop".to_string())]);
        let mut m2 = Params::new(); m2.insert("method".into(), 2.);
        let t2 = run("vrsdd_double_doji", &x, 0, r.len() - 1, &m2).unwrap();
        // method 2: half at 103 on bar 5, stop to 101; the rest (target 105) is never hit and no low reaches 101: open at 103.5
        assert_eq!((t2[0].exit_i, t2[0].open, (t2[0].exit_px * 100.).round()), (9, true, 10325.));
        assert!(run("vrsdd_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}
