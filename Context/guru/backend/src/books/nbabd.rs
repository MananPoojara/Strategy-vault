//! Mark Soberman (NetPicks), 'NetPicks BreakAway or BreakDown Trading System', a chapter in Stephen Pierce (ed.), Amazing
//! Trading Plans (Impulsive Profits, 2004), PDF p.88-93; research share USB HDD 2/tRADING KNO-2/Stephen A.Pierce - Amazing
//! Trading Plans (536 MB) (amazingtradingplans.com)/CD5 - Amazing Trading Plans Manual/Amazing Manual.pdf. Book id `nbabd`.
//! Lane K7 (titles loop, 1 Nov 2026); rule text authors/batch-c/C2w/pierce_netpicks.txt; independent check
//! authors/titles-1030/K7w/k7_ref.py (header).
use crate::connors::{atr, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{hhv, llv};
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K7w/k7_ref.py (header)).
pub const REVIEWED: &[&str] = &["nbabd_breakaway"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Stochastic = fast %K over 14 days, 100 x (close - lowest low) / (highest high - lowest low) (option 20 days; 'either a 14 or 20 period', p.88); ATR = Wilder's average true range over 7 and 14 days, the larger one used. Long set-up: %K at or below 20 at the close; buy stop for the next day = that day's high + the larger ATR, kept until filled and replaced when a later set-up day gives a lower buy stop (its own protective stop and ATR then apply); an opposite set-up cancels it. Protective stop one tick below the set-up bar's low ('a break below the set-up bar', p.89; his nudge below round numbers is not applied); a tick is the instrument's own (0.01 US, 0.05 NSE). Target = entry + 2 x the larger ATR of the set-up day (the example's lower target, p.90); options 1.0x or 1.5x the ATR, or 1.5 x the risk (entry - protective stop). 'should a trade set up in the opposite direction, you would exit there': the opposite set-up's stop order, when filled, closes the trade (option: reverse into it). Exits are tested from the day after the entry; stops fill at the level or a worse open, the target at the level or a better open, and a stop is taken before the target when both are inside one day. Shorts mirrored (%K at or above 80, sell stop = low - the larger ATR, protective stop one tick above the set-up bar's high). Not coded: raising the stop to break-even and trailing (his options, no fixed rule) and the stock selection (momentum, volume). The chapter trades US stocks with one worked example (Sierra Wireless, May-Jul 2004, not in the vault's baskets); ETFs, indices and India are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "nbabd_breakaway", name: "BreakAway / BreakDown: fast stochastic at 20 or 80, stop entry one ATR beyond the set-up bar, 2 x ATR target",
        group: "NetPicks BreakAway or BreakDown", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "nbabd", rules: &[
            "F|\"Put the Average True Range (ATR) on the chart of a 7 period and 14 period duration\"; \"Put the stochastic (Fast) on your chart; we like to use either a 14 or 20 period\" (p.88)",
            "E|\"The Stochastic needs to read 20 or less ... Take the greater of the two Average True Range figures for the current trading day ... Add the Average True Range number to the high of the current trading day. This becomes your buy stop price for the next trading day\" (p.89); \"That buy stop remains valid until filled. You can adjust it the next trading day(s) if the next trading day(s) end up calculating to a lower buy stop price\" (p.90)",
            "X|\"Your protective stop will be a break below the set-up bar\"; \"take gains at a multiple of the Average True Range such as 1.0x, 1.50x, 2.0x. Or ... your risk and multiply that by 1.5x\"; \"should a trade set up in the opposite direction, you would exit there\" (p.89)",
            "E|Short: \"The Stochastic needs to read 80 or greater ... Subtract the Average True Range number from the low of the current trading day. This becomes your sell short stop price\"; \"Your protective stop will be a break above the set-up bar\" (p.91)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "nbabd_breakaway").then_some(("nbabd_breakaway", "NetPicks BreakAway or BreakDown", "stochastic 14, ATR 7/14, 2 x ATR target"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "nbabd_breakaway").then(|| vec![
        ParamDef { key: "k", label: "Fast stochastic period (14 or 20)", def: 14., min: 5., max: 30., step: 1. },
        ParamDef { key: "target", label: "Target: 0 = 2 x ATR (reading), 1 = 1 x ATR, 2 = 1.5 x ATR, 3 = 1.5 x risk", def: 0., min: 0., max: 3., step: 1. },
        ParamDef { key: "reverse", label: "Opposite entry: 0 = exit (reading), 1 = reverse into it", def: 0., min: 0., max: 1., step: 1. },
    ])
}

#[derive(Clone, Copy)]
struct Ord { d: f64, px: f64, stop: f64, atr: f64 }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let n = g("k", 14.).round().max(1.) as usize;
    let (tmode, rev) = (g("target", 0.).round() as i32, g("reverse", 0.) >= 0.5);
    let tick = x.market_tick();
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (hh, ll) = (hhv(h, n), llv(l, n));
    let k: Vec<f64> = (0..c.len()).map(|i| { let d = hh[i] - ll[i]; if d > 0. { 100. * (c[i] - ll[i]) / d } else { f64::NAN } }).collect();
    let (a7, a14) = (atr(x.r, 7), atr(x.r, 14));
    let target = |ord: &Ord, ep: f64| ep + ord.d * match tmode { 1 => ord.atr, 2 => 1.5 * ord.atr, 3 => 1.5 * (ep - ord.stop).abs(), _ => 2. * ord.atr };
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64, f64, f64)> = None; // (dir, entry bar, price, stop, target)
    let mut pend: Option<Ord> = None;
    for i in from..=to {
        let hit = |q: &Ord| if q.d > 0. { h[i] >= q.px } else { l[i] <= q.px };
        let fill = |q: &Ord| if q.d > 0. { o[i].max(q.px) } else { o[i].min(q.px) };
        if let Some((d, e, ep, st, tp)) = pos {
            if i > e {
                let opp = pend.filter(|q| q.d == -d && hit(q));
                let st_hit = if d > 0. { l[i] <= st } else { h[i] >= st };
                if st_hit || opp.is_some() {
                    // both are stop orders against the trade: the one nearer the price is reached first
                    let lv = match (st_hit, opp) { (true, Some(q)) => if d > 0. { st.max(q.px) } else { st.min(q.px) }, (true, None) => st, (_, Some(q)) => q.px, _ => unreachable!() };
                    let px = if d > 0. { o[i].min(lv) } else { o[i].max(lv) };
                    out.push(mk(d, e, ep, "stop", i, px, if opp.is_some() && lv == opp.unwrap().px { "opposite entry" } else { "protective stop" }, false, i));
                    pos = None;
                    if let (true, Some(q)) = (rev, opp) { let ep2 = fill(&q); pos = Some((q.d, i, ep2, q.stop, target(&q, ep2))); pend = None; }
                    else if opp.is_some() { pend = None; }
                } else if if d > 0. { h[i] >= tp } else { l[i] <= tp } {
                    out.push(mk(d, e, ep, "stop", i, if d > 0. { o[i].max(tp) } else { o[i].min(tp) }, "target", false, i)); pos = None;
                }
            }
        } else if let Some(q) = pend {
            if hit(&q) { let ep = fill(&q); pos = Some((q.d, i, ep, q.stop, target(&q, ep))); pend = None; }
        }
        if i >= to { break; }
        // set-ups at this close
        let su = if k[i] <= 20. { 1. } else if k[i] >= 80. { -1. } else { 0. };
        if su != 0. && a7[i].is_finite() && a14[i].is_finite() {
            let a = a7[i].max(a14[i]);
            let q = if su > 0. { Ord { d: 1., px: h[i] + a, stop: l[i] - tick, atr: a } } else { Ord { d: -1., px: l[i] - a, stop: h[i] + tick, atr: a } };
            let in_dir = pos.map_or(false, |z| z.0 == su);
            pend = match pend {
                _ if in_dir => None,                                                            // already in that direction
                Some(old) if old.d == su && (su > 0. && old.px <= q.px || su < 0. && old.px >= q.px) => Some(old),
                _ => Some(q),
            };
        }
    }
    if let Some((d, e, ep, ..)) = pos { out.push(mk(d, e, ep, "stop", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn entries_are_atr_stops_beyond_a_stochastic_setup_bar() {
        let c: Vec<f64> = (0..700).map(|i| 40. + 8. * (i as f64 / 11.).sin() + 2. * (i as f64 / 2.7).cos()).collect();
        let rows: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{}", i), open: v, high: v + 0.6, low: v - 0.6, close: v, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let t = run("nbabd_breakaway", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5);
        for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].legs[0].i && w[1].legs[0].i >= w[0].exit_i); }
        let mut p = Params::new(); p.insert("reverse".into(), 1.);
        assert!(run("nbabd_breakaway", &x, 0, rows.len() - 1, &p).unwrap().len() >= t.len());
        assert!(run("nbabd_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Mark Soberman (NetPicks BreakAway or BreakDown, in Amazing Trading Plans) 2026-11-01 (titles lane K7).
// Admission only for the module REVIEWED lists (authors/titles-1030/K7w/k7_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "nbabd" && reviewed(def.id),
    label: "Mark Soberman (NetPicks) wrote the BreakAway/BreakDown rules for momentum US stocks with one worked example; US and India stocks, ETFs and indices are a declared application.",
    basis: "Mark Soberman, NetPicks BreakAway or BreakDown Trading System, in Stephen Pierce (ed.), Amazing Trading Plans (2004), PDF p.88-93",
};
