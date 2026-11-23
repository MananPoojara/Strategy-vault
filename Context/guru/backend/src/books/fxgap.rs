//! Forex Impact (Sharptrade Partners), "Forex Gap Play" (research share Trading Knowledge I/Jason Fielder - Forex Profit Formula System (forexprofitformula.com)/Gap Trading the Forex/
//! Gap Trading the Forex.pdf, 33 pp., text layer for pp.3-19, read in full). Book id `fxgap`. Weekly gap fade: when the new week opens 15 pips or more BELOW the previous week's close, buy
//! 2 lots at the open (15 pips or more above: sell 2 lots); stop 150 pips from the entry; once the price retraces to the previous week's close a breakeven stop goes at the entry; lot 1 is
//! exited at the previous week's close, lot 2 stays open until the end of the week. Lane K1 (23 Nov 2026, queue Q381 row 409). Independent check authors/conv-1123/K1/fxgap_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fxgap";
pub const REVIEWED: &[&str] = &["fxgap_weekly"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340/#380] The guide trades the EUR/USD weekend gap; this card runs the rule on the weekly gap of the NSE indices NIFTY 50 and NIFTY BANK (and the US indices) on daily bars. The printed pages 20-33 (worked examples, automation) have no text layer and were not read; the guide's claim is an accuracy 'almost 90%' of the time, not a trade list, so nothing is compared, UNVERIFIED. [Declared readings, each an unstated detail] (1) Pips are scaled by ONE rule, 1 pip = 0.01 percent of the price (the guide prints no price): the 15-pip gap is 0.15 percent of the previous close and the 150-pip stop 1.5 percent of the entry. (2) The week is the calendar week (Monday to Sunday); the first bar of a week is its open, the previous bar's close is the previous week's close; the last bar of the week closes lot 2. (3) Inside a daily bar the stop is tested before the retrace to the previous close (conservative), and the breakeven stop on lot 2 starts the bar AFTER the retrace; a stop fills at the stop price, or at the open on a gap through it. (4) The two lots are one position with two equal scale-out pieces; the trade's exit price is their average and its reason is the later piece's.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "fxgap_weekly", name: "Forex Gap Play: fade a weekly open gap of 15 pips or more, 2 lots, stop 150 pips, lot 1 at the previous close, lot 2 at the week's end (indices, daily bars)",
        group: "Forex Impact gap play", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "fxgap",
        rules: &[
            "E|\"Long Setup: Pair gaps down 15 pips or more ... we are going to buy 2 lots/2 mini-lots\" at the open; \"Short Setup: if the pair gaps up 15 pips or more ... sell 2 lots\" (pp.8-11)",
            "X|\"Place the stop loss 150 pips below the entry\" (short: above); a breakeven stop at the entry price once the pair re-traces to the previous week's close (pp.13-17)",
            "X|\"The first lot will be exited at the previous week's close and the second lot will remain open until the end of the week\" (p.19)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "fxgap_weekly" => Some(("fxgap", "Forex Gap Play (weekly gap fade)", "gap 0.15 percent, stop 1.5 percent")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "fxgap_weekly" => Some(vec![p("gap", "Minimum weekly gap, percent of the previous close (15 pips scaled at 1 pip = 0.01 percent)", 0.15, 0.0, 5.0, 0.05),
            p("stop", "Stop, percent of the entry (150 pips scaled at 1 pip = 0.01 percent)", 1.5, 0.1, 10.0, 0.1)]),
        _ => None,
    }
}

fn days(d: &str) -> Option<i64> {
    let v: Vec<i64> = d.split('-').map(|s| s.parse().ok()).collect::<Option<_>>()?;
    if v.len() != 3 { return None; }
    let (mut y, m, dd) = (v[0], v[1], v[2]);
    if m <= 2 { y -= 1; }
    let era = y.div_euclid(400); let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + dd - 1;
    Some(era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (gap, stop) = (g("gap") / 100.0, g("stop") / 100.0);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let wk: Vec<i64> = x.r.iter().map(|r| days(&r.date).map_or(-1, |d| (d + 3).div_euclid(7))).collect();
    let mut out: Vec<Trade> = vec![];
    let mut a = 1usize;
    while a <= to {
        if wk[a] == wk[a - 1] || wk[a] < 0 { a += 1; continue; }
        let mut e = a; while e < to && wk[e + 1] == wk[a] { e += 1; }
        let last_of_week = e;
        let pc = c[a - 1];
        let gp = (o[a] - pc) / pc;
        let dir = if gp <= -gap { 1.0 } else if gp >= gap { -1.0 } else { 0.0 };
        if dir != 0.0 && a >= from {
            let ep = o[a];
            let st = ep * (1.0 - dir * stop);
            let mut ex: [Option<(usize, f64, &'static str)>; 2] = [None, None]; // lot 1, lot 2
            let mut be_from = usize::MAX;
            for d in a..=e {
                let stop_hit = if dir > 0.0 { l[d] <= st } else { h[d] >= st };
                let sp = if dir > 0.0 { o[d].min(st) } else { o[d].max(st) };
                let touch = if dir > 0.0 { h[d] >= pc } else { l[d] <= pc };
                if ex[0].is_none() {
                    if stop_hit { ex[0] = Some((d, sp, "150-pip stop")); }
                    else if touch { ex[0] = Some((d, pc, "previous week's close")); }
                }
                if ex[1].is_none() {
                    let be_hit = d >= be_from && (if dir > 0.0 { l[d] <= ep } else { h[d] >= ep });
                    if be_hit { ex[1] = Some((d, if dir > 0.0 { o[d].min(ep) } else { o[d].max(ep) }, "breakeven stop")); }
                    else if stop_hit { ex[1] = Some((d, sp, "150-pip stop")); }
                    else if touch && be_from == usize::MAX { be_from = d + 1; }
                }
                if ex[0].is_some() && ex[1].is_some() { break; }
            }
            let f = |z: Option<(usize, f64, &'static str)>| z.unwrap_or((last_of_week, c[last_of_week], "end of the week"));
            let (l1, l2) = (f(ex[0]), f(ex[1]));
            let mut pieces = vec![(l1.0, l1.1, 0.5), (l2.0, l2.1, 0.5)];
            pieces.sort_by_key(|z| z.0);
            let fin = if l2.0 >= l1.0 { l2 } else { l1 };
            out.push(Trade { dir, legs: vec![Leg { i: a, px: ep, w: 1. }], exit_i: fin.0, exit_px: 0.5 * l1.1 + 0.5 * l2.1, reason: fin.2.into(), entry_kind: "open", signal_i: a.saturating_sub(1), open: false, exits: pieces });
        }
        a = e + 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn gap_down_week_buys_two_lots_and_rejects_other_ids() {
        let mk = |d: &str, o: f64, h: f64, l: f64, c: f64| Row { ticker: "NIFTY 50".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None };
        // Fri 2021-01-01 close 100; Mon 2021-01-04 gaps down to 99 (-1%), rallies to 100.5 (previous close) on Tuesday, week ends Friday at 100
        let r = vec![mk("2020-12-31", 99., 101., 99., 100.), mk("2021-01-01", 100., 101., 99.5, 100.), mk("2021-01-04", 99., 99.5, 98.8, 99.2), mk("2021-01-05", 99.2, 100.5, 99., 100.2), mk("2021-01-06", 100.2, 100.4, 99.9, 100.1), mk("2021-01-07", 100.1, 100.3, 99.8, 100.), mk("2021-01-08", 100., 100.2, 99.7, 100.)];
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("fxgap_x", &x, 0, 6, &Params::default()).is_none());
        let t = run("fxgap_weekly", &x, 0, 6, &Params::default()).unwrap();
        assert_eq!(t.len(), 1);
        assert!(t[0].dir == 1.0 && t[0].legs[0].i == 2 && t[0].exits.len() == 2 && t[0].exit_i == 6);
        assert_eq!((t[0].exits[0].0, t[0].exits[0].1), (3, 100.0));
    }
}

// Instrument admission: indices only (never single stocks or ETFs), declared adaptation.
pub fn index_only() -> &'static str { "Forex Gap Play (forex weekly gap rule): run on actual indices only (NIFTY 50 / NIFTY BANK and the US indices) as a declared adaptation (R4 #340/#380); UNVERIFIED" }
