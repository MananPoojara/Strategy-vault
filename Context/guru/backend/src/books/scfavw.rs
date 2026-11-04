//! Jerry Favors, "Gann Weekly Swing Chart", Technical Analysis of Stocks & Commodities V.10:9 (September 1992) pp.390-393 (research
//! share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V10/C09/Gannwk.pdf). Book id
//! `scfavw`. Lane K40 (4 Nov 2026, decision #203); independent check authors/titles-1030/K40w/k40_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{high_first, Pos};
use super::sctraub::week_id;

pub const REVIEWED: &[&str] = &["scfavw_swing"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Calendar weeks (Monday-based); 'the prior week' = the previous week with bars. The swing chart, checked on daily bars: while not up it turns UP on the first day whose high is above the prior week's high; while not down it turns DOWN on the first day whose low is below the prior week's low (the article's House of Fabrics walk-through, p.391: 'it is always the highest or lowest price seen in the prior week that turns the weekly chart's direction'; a turn and a turn back can happen in the same week). A day turns the chart at most once; before the first turn a day that could do both takes the extreme nearer the open. A turn down records a swing top = the highest high of the up leg including that day; a turn up records a swing bottom = the lowest low of the down leg including that day. Trading (p.392-393): flat, buy on a stop one tick above the latest swing top or sell short one tick below the latest swing bottom (the article's 'prior swing high of 25-1/8 ... exceeded', paying 25-1/4, and stops 'just below' a low: 22-7/8 -> 22-3/4, one 1/8 tick); a long's stop is one tick below the latest swing bottom when the position is opened, raised to each higher bottom; a short's stop one tick above the latest swing top, lowered to each lower top; a stop that is hit closes the position and reverses it at the same price ('the rules state to reverse position anytime we are stopped out'). Levels are those known at the previous close; fills at the level or at the open beyond it; tick = 0.01 US, 0.05 NSE (option tick = units). Book example: House of Fabrics 1991-92 (long 8/6/91 near 25-1/4, stopped and reversed 11/19/91 near 36-1/2); no test statistics are printed. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scfavw_swing", name: "Favors Gann weekly swing chart: stop and reverse at the last swing top / bottom, the stop trailed to each new swing point", group: "S&C: Gann Weekly Swing Chart (Favors, V.10:9)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scfavw", rules: &[
            "F|\"The weekly swing chart will turn up from any extreme low price when the stock in question rises above the highest price seen in the prior week. The chart will then continue higher to the highest price reached before the stock falls below the lowest price seen in any prior week, which then turns the chart down.\" (p.391)",
            "E|\"The first signal to take action on this chart was at point A, when the prior swing high of 25-1/8 on July 17 was exceeded on August 6, 1991. We would then go long with a stop place just below the prior low of 22-7/8.\" (p.392); \"The first time the stock falls below a prior swing chart bottom, you will sell long positions and go short, with a stop then placed just above the prior swing chart top.\" (p.392)",
            "X|\"you will remain long in the stock in question and keep raising your stop to each higher swing-chart bottom\"; \"If the trend continues down, you will keep lowering your stop to just above each successively lower top.\" (p.392); \"the rules state to reverse position anytime we are stopped out\" (p.393)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scfavw_swing", "Favors Gann weekly swing chart (S&C)", "stop and reverse, 1 tick")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("tick", "'Just above / below' a swing point, in instrument ticks (article: one 1/8 tick)", 1., 0., 10., 1.)])
}

/// v at or beyond the level (a float-noise margin so a price one tick away triggers).
pub fn ge(v: f64, lv: f64) -> bool { v >= lv - 1e-9 * lv.abs().max(1.) }
pub fn le(v: f64, lv: f64) -> bool { v <= lv + 1e-9 * lv.abs().max(1.) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let tk = p.get("tick").copied().unwrap_or(1.) * x.market_tick();
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let mut pos = Pos::new();
    let (mut wk, mut pw, mut cw) = (i64::MIN, None::<(f64, f64)>, (f64::NEG_INFINITY, f64::INFINITY));
    let (mut dir, mut legmax, mut legmin) = (0i8, f64::NAN, f64::NAN);
    let (mut top, mut bot, mut stop) = (f64::NAN, f64::NAN, f64::NAN);
    for t in 0..=to {
        let w = week_id(&x.r[t].date);
        if w != wk { pw = (wk != i64::MIN).then_some(cw); wk = w; cw = (f64::NEG_INFINITY, f64::INFINITY); }
        // 1. orders from the levels known at the previous close
        let mut entered = false;
        if t >= from {
            let s = pos.side();
            if s > 0. && le(l[t], stop) { let px = o[t].min(stop); pos.exit(t, px, "below the swing bottom"); pos.enter(-1., t, px, "stop"); entered = true; }
            else if s < 0. && ge(h[t], stop) { let px = o[t].max(stop); pos.exit(t, px, "above the swing top"); pos.enter(1., t, px, "stop"); entered = true; }
            else if s == 0. {
                let (bu, se) = (top + tk, bot - tk);
                let (hu, hd) = (top.is_finite() && ge(h[t], bu), bot.is_finite() && le(l[t], se));
                if hu && (!hd || high_first(o[t], h[t], l[t])) { pos.enter(1., t, o[t].max(bu), "stop"); entered = true; }
                else if hd { pos.enter(-1., t, o[t].min(se), "stop"); entered = true; }
            }
        }
        // 2. the swing chart with this day
        let (mut new_top, mut new_bot) = (false, false);
        if let Some((ph, pl)) = pw {
            let (up, dn) = (dir != 1 && h[t] > ph, dir != -1 && l[t] < pl);
            let turn = if up && dn { if high_first(o[t], h[t], l[t]) { 1 } else { -1 } } else if up { 1 } else if dn { -1 } else { 0 };
            if turn == -1 {
                if dir == 1 { top = legmax.max(h[t]); new_top = true; }
                dir = -1; legmin = l[t];
            } else if turn == 1 {
                if dir == -1 { bot = legmin.min(l[t]); new_bot = true; }
                dir = 1; legmax = h[t];
            } else if dir == 1 { legmax = legmax.max(h[t]); } else if dir == -1 { legmin = legmin.min(l[t]); }
        }
        cw = (cw.0.max(h[t]), cw.1.min(l[t]));
        // 3. the protective stop for the next day
        let s = pos.side();
        if s > 0. && (entered || (new_bot && !stop.is_finite())) { stop = bot - tk; }
        else if s > 0. && new_bot { stop = stop.max(bot - tk); }
        else if s < 0. && (entered || (new_top && !stop.is_finite())) { stop = top + tk; }
        else if s < 0. && new_top { stop = stop.min(top + tk); }
        else if s == 0. { stop = f64::NAN; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekly_swing_stop_and_reverse() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scfavw_swing", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        // after the first trade the position is always in: each exit is the next entry, at the same price
        for w in t.windows(2) { assert_eq!(w[1].legs[0].i, w[0].exit_i); assert_eq!(w[1].legs[0].px, w[0].exit_px); assert!(w[1].dir == -w[0].dir); }
        for z in &t { let i = z.legs[0].i; assert!(z.legs[0].px <= x.h[i] + 1e-6 && z.legs[0].px >= x.l[i] - 1e-6); }
    }
}

// Instrument admission: S&C Favors Gann weekly swing chart 2026-11-04 (titles lane K40).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scfavw" && reviewed(def.id),
    label: "Jerry Favors applied the Gann weekly swing chart to individual US stocks (House of Fabrics 1991-92 example). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.10:9 pp.390-393 (Gann Weekly Swing Chart); independent check authors/titles-1030/K40w/k40_ref.py",
};
