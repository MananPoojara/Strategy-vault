//! Jeremy J. Burns, "The One Trading Secret That Could Make You Rich: Inside Days" (TradeCashSystem.com, 2009; book id
//! `jbid`; PDF pages). Reading: claude-continuation-20261020/authors/jjburns (admission/id_ref.py). Live: the inside-day
//! (NIB) breakout, PDF 7-17: a day with a lower high and a higher low than the day before; on the next day a buy stop
//! one cent over its high and a sell stop one cent under its low (day orders); stop at the other side; conservative exit
//! at target 1 (entry +/- the inside day's range); otherwise out at the close of day 3 (day 4 as an option).
//! Not coded: the scale-out over targets 2-3, the previous-day stop variation, the 'fake out' re-entry, the earnings
//! filter. Book: US stocks (AXP); indices, ETFs and India stocks here are a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/jjburns/admission/id_ref.py (header)).
pub const REVIEWED: &[&str] = &["jbid_inside_day"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Inside day = high below the previous high and low above the previous low (any colour or body, PDF 7); the range limit ('50% ideal, 60% ok, even 70%', 'simply eyeing') is an option, off by default. Day 1 = the inside day; the stop orders work on day 2 only (DAY ORDERS): filled at the level, or at the open when day 2 opens beyond it. If day 2 reaches both levels without opening beyond either, the order is unknown on daily bars and no trade is taken. One cent = 0.01 in the instrument's price (option tick). From day 3: the stop at the other side of the inside day (one cent beyond) and target 1 = entry +/- the inside day's range, each filled at its level or a gapped open; if both are touched on one day the stop is taken. Not hit by the close of day 3 (option: day 4): sold at that close. Exits on day 2 itself are not checked (the order of the fill and the later moves inside the day is unknown). One position at a time; an inside day while in a trade is ignored.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "jbid_inside_day", name: "Inside-day breakout: stop entries one cent beyond, stop at the other side, target 1 or day-3 close", group: "Burns inside days",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "jbid",
        rules: &["F|\"the 'inside day' has a 'lower high' and a 'higher low' than the previous day\" (PDF 7)",
            "E|\"Use one penny above the high of the NIB to enter a LONG position and one penny below the low of the NIB to enter a SHORT position\"; \"Note: You want to make sure these are DAY ORDERS only.\" (PDF 11-12)",
            "X|\"If you get your LONG order filled at 44.58 then you would place your stop loss order at 43.70 which is the low of the NIB minus one penny.\" (PDF 12)",
            "X|\"take the NIB Range and subtract/add it to your entry prices for each target\"; \"Conservative players will exit 100% of their position on hitting Profit Target #1.\" (PDF 14-17)",
            "X|\"We count the Inside Day we are using in the setup as Day 1. Exit your position on Day 4 or the close of trading on Day 3\" (PDF 17)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "jbid_inside_day" => Some(("jbid_inside_day", "Burns inside-day breakout", "target 1, day 3")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "jbid_inside_day" => Some(vec![p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.),
            p("day", "Time exit at the close of day N (book 3, or 4)", 3., 3., 4., 1.),
            p("ratio", "Inside-day range at most this share of the previous range (1 = any inside day; book 'ideal' 0.5)", 1., 0.3, 1., 0.05),
            p("tick", "One cent (price units)", 0.01, 0., 1., 0.01)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (side, last, ratio, tk) = (g("side").round() as i64, g("day").round() as usize, g("ratio"), g("tick"));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mk = |d: f64, ei: usize, ep: f64, kind: &'static str, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }],
        exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: xi, open, exits: vec![] };
    let mut out = vec![];
    let mut t = from.max(1); // candidate inside day (day 1)
    while t + 1 < to {
        let inside = h[t] < h[t - 1] && l[t] > l[t - 1] && h[t] - l[t] <= ratio * (h[t - 1] - l[t - 1]);
        if !inside { t += 1; continue; }
        let (bu, se, rg, e) = (h[t] + tk, l[t] - tk, h[t] - l[t], t + 1);
        let long_ok = side != 2 && (o[e] >= bu || h[e] >= bu);
        let short_ok = side != 1 && (o[e] <= se || l[e] <= se);
        let (d, ep, kind) = if long_ok && o[e] >= bu { (1., o[e], "open") } else if short_ok && o[e] <= se { (-1., o[e], "open") }
            else if long_ok && !(l[e] <= se) { (1., bu, "stop") } else if short_ok && !(h[e] >= bu) { (-1., se, "stop") } else { t += 1; continue; };
        let (stop, tgt) = if d > 0. { (se, ep + rg) } else { (bu, ep - rg) };
        let end = (t + last - 1).min(to);
        let mut done = None;
        for k in e + 1..=end {
            let (hs, ht) = if d > 0. { (l[k] <= stop, h[k] >= tgt) } else { (h[k] >= stop, l[k] <= tgt) };
            if hs { done = Some((k, if d > 0. { o[k].min(stop) } else { o[k].max(stop) }, "stop", false)); break; }
            if ht { done = Some((k, if d > 0. { o[k].max(tgt) } else { o[k].min(tgt) }, "target 1", false)); break; }
        }
        let (xi, xp, why, open) = done.unwrap_or(if end == t + last - 1 { (end, c[end], "day close", false) } else { (end, c[end], "open", true) });
        out.push(mk(d, e, ep, kind, xi, xp, why, open));
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
    fn hand_checked_inside_day_trades() {
        let r = vec![bar(0, 50., 52., 48., 50.), bar(1, 50., 51., 49., 50.5), // inside day: buy stop 51.01, sell stop 48.99
            bar(2, 50.5, 51.5, 50., 51.2), // long at 51.01, target 51.01 + 2 = 53.01
            bar(3, 51.2, 53.5, 51., 53.), // target hit
            bar(4, 53., 54., 52., 53.5), bar(5, 53.5, 53.8, 52.5, 53.), // inside day
            bar(6, 53., 53.1, 52., 52.2), // short at 52.49, stop 53.81, target 51.19
            bar(7, 52.2, 52.6, 51.5, 52.)]; // day 3 of that setup: out at the close
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jbid_inside_day", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|z| (z.dir, z.legs[0].i, (z.legs[0].px * 100.).round(), z.exit_i, (z.exit_px * 100.).round(), z.reason.clone())).collect();
        assert_eq!(got, vec![(1., 2, 5101., 3, 5301., "target 1".to_string()), (-1., 6, 5249., 7, 5200., "day close".to_string())]);
        assert!(run("jbid_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}

// Instrument admission: Jeremy J. Burns 2026-10-29 (lane K1) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/jjburns/admission/id_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="jbid" && crate::books::jbid::reviewed(def.id),
    label: "Jeremy Burns trades inside days on US stocks; runs on indices, ETFs and India stocks are a declared application, not the author's test.",
    basis: "Jeremy J. Burns, Inside Days (TradeCashSystem.com, 2009), PDF 7-17",
};
