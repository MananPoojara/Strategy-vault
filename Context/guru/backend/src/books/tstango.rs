//! Tango System (dancewiththemarkets.com), TradeStation code "Open_Code.txt" (research share USB HDD 1/TradeStation Systems &
//! Indicators/Tango System for TS (dancewiththemarkets.com)/Open_Code.rar -> Open_Code.txt). Book id `tstango`. Lane K18 (titles
//! loop, 2 Nov 2026), to-code row 338; independent check authors/titles-1030/K18w/k18_ref.py.
//!
//! The code reads PositionProfit(1) (the last closed trade's profit), which the shared rrsim engine does not expose, so this
//! module carries `run_ts_pp`: the rrsim daily-bar order conventions (rrsim.rs header) with the last closed trade's profit
//! passed to the strategy code. rrsim itself is unchanged.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, xavg, A, O, P};
use super::sctraub::{dmi, ticks};

pub const REVIEWED: &[&str] = &["tstango_adx"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation code as printed (ADXLEN 14, CUTOFF 30, XMALEN 20, ENTRYADD and EXITADD 1 point = 1 tick of the instrument, decision 57; option tick 0 = no offsets). ADX(14) = Wilder's ADX; XMA = XAverage(Close, 20) (seeded with the first close). Per bar, in code order: while not long the long exit stop is the lowest low of 3 bars - 1 tick, while not short the short exit stop the highest high of 3 bars + 1 tick; a bar that reaches the previous bar's pullback buy (sell) stop tightens that exit stop to its own low (high) and arms the re-entry. Trend set-up when ADX > 30 and rising: a low above the XMA arms buys, a high below it arms sells; a low above the XMA clears the sell set-up and the re-sell, a high below it the buy set-up and the re-buy; a position clears its own set-up. Flat: after a profitable last trade, or a new 3-bar high above the XMA (low below it), the re-sell (re-buy) is cancelled; otherwise the re-entry stop at the last pullback entry is re-issued. Pullback entries: with the set-up armed and the bar touching the XMA, buy next bar on a stop 1 tick over the high (sell short 1 tick under the low), and that level becomes the re-entry level. Opposite entries reverse. MarketPosition, MP[1] and PositionProfit(1) are read as TradeStation does: the position at the bar's close before its own orders, the previous bar's, and the last closed trade's profit (a trade with no predecessor counts 0). Fills on the vault's TradeStation daily-bar engine conventions (rrsim). No results are printed. Runs on daily indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tstango_adx", name: "Tango: ADX(14) above 30 and rising arms pullbacks to the 20 XMA, stop entry over the pullback bar, 3-bar extreme exit stop and re-entry", group: "TradeStation code: Tango System (dancewiththemarkets.com)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tstango", rules: &[
            "F|IF ADX14>CUTOFF AND ADX14>ADX14[1] THEN BEGIN IF L>XMA20 THEN SETBUY=TRUE; IF H<XMA20 THEN SETSELL=TRUE; END; (CUTOFF 30)",
            "E|IF SETBUY AND L<=XMA20 THEN BBUY=H+ENTRYADD; ... Buy(\"Buy-1\") Next Bar BBUY STOP; REBUY=BBUY; re-entry Buy(\"Re-Buy\") Next Bar REBUY STOP while flat (short side mirrored)",
            "X|IF MP<1 THEN LXSTOP=MINLIST(L,L[1],L[2])-EXITADD; ... Sell(\"LX\") Next Bar LXSTOP STOP; Buy to Cover(\"SX\") Next Bar SXSTOP STOP (1 point = 1 tick)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tstango_adx", "Tango System (TradeStation)", "ADX 14 / 30, XMA 20")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("cutoff", "ADX cutoff (30)", 30., 5., 60., 1.), p("xma", "XAverage length (20)", 20., 5., 100., 1.), p("tick", "ENTRYADD / EXITADD in ticks (1 point)", 1., 0., 10., 1.)])
}

/// Position state at a close, plus the last closed trade's profit (price points, 0 before any trade) and the number of entries
/// filled on this bar (TradeStation TradesToday(Date) on daily bars, as the vault reads it).
#[derive(Clone, Copy, Default)]
pub struct StPp { pub pos: i8, pub entry_px: f64, pub entry_i: usize, pub last_profit: f64, pub entries_today: usize }

/// rrsim's `run_ts` with PositionProfit(1): next-bar orders only.
pub fn run_ts_pp(x: &Ctx, from: usize, to: usize, mut f: impl FnMut(usize, &StPp, &mut Vec<O>)) -> Vec<Trade> {
    let (mut pos, mut cur, mut out, mut last): (i8, Option<Trade>, Vec<Trade>, f64) = (0, None, vec![], 0.);
    let mut close = |pos: &mut i8, cur: &mut Option<Trade>, out: &mut Vec<Trade>, last: &mut f64, i: usize, px: f64, why: &str| {
        if let Some(mut t) = cur.take() { *last = t.dir * (px - t.legs[0].px); t.exit_i = i; t.exit_px = px; t.reason = why.into(); t.signal_i = i - 1; out.push(t); }
        *pos = 0;
    };
    let valid = |pos: i8, od: &O| match od.a { A::Buy => pos != 1, A::Sell => pos != -1, A::Xl => pos == 1, A::Xs => pos == -1 };
    let rising = |a: A| matches!(a, A::Buy | A::Xs);
    let mut pending: Vec<O> = vec![];
    for i in 0..=to {
        let mut entries = 0usize;
        if i > from && !pending.is_empty() {
            let (op, hi, lo, cl) = (x.o[i], x.h[i], x.l[i], x.c[i]);
            let mut done = vec![false; pending.len()];
            let mut exec = |k: usize, px: f64, kind: &'static str, pos: &mut i8, cur: &mut Option<Trade>, out: &mut Vec<Trade>, last: &mut f64, done: &mut Vec<bool>, n_ent: &mut usize| {
                done[k] = true;
                let od = pending[k];
                match od.a {
                    A::Buy | A::Sell => {
                        let d: i8 = if od.a == A::Buy { 1 } else { -1 };
                        if *pos == -d { close(pos, cur, out, last, i, px, "reversed"); }
                        *pos = d;
                        *n_ent += 1;
                        *cur = Some(Trade { dir: d as f64, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i - 1, open: false, exits: vec![] });
                    }
                    A::Xl | A::Xs => close(pos, cur, out, last, i, px, od.tag),
                }
            };
            for pass in 0..2 {
                for k in 0..pending.len() {
                    let od = pending[k];
                    let is_exit = matches!(od.a, A::Xl | A::Xs);
                    if matches!(od.p, P::Mkt) && (pass == 0) == is_exit && !done[k] && valid(pos, &od) { exec(k, op, "open", &mut pos, &mut cur, &mut out, &mut last, &mut done, &mut entries); }
                }
            }
            loop {
                let hit = (0..pending.len()).find(|&k| { let od = pending[k]; !done[k] && valid(pos, &od) && match od.p {
                    P::Stop(s) => if rising(od.a) { op >= s } else { op <= s }, P::Lim(s) => if rising(od.a) { op <= s } else { op >= s }, P::Mkt => false } });
                match hit { Some(k) => { let kind = if matches!(pending[k].p, P::Lim(_)) { "limit" } else { "stop" }; exec(k, op, kind, &mut pos, &mut cur, &mut out, &mut last, &mut done, &mut entries); } None => break }
            }
            let path = if hi - op < op - lo { [hi, lo, cl] } else { [lo, hi, cl] };
            let (mut c0, mut seg) = (op, 0);
            while seg < 3 {
                let t = path[seg];
                let up = t > c0;
                let mut best: Option<(usize, f64)> = None;
                for k in 0..pending.len() {
                    let od = pending[k];
                    if done[k] || !valid(pos, &od) { continue; }
                    let lv = match od.p { P::Stop(s) if rising(od.a) == up => s, P::Lim(s) if rising(od.a) != up => s, _ => continue };
                    let inside = if up { lv > c0 && lv <= t } else { lv < c0 && lv >= t };
                    if inside && best.map_or(true, |(_, b)| (lv - c0).abs() < (b - c0).abs()) { best = Some((k, lv)); }
                }
                match best {
                    Some((k, lv)) => { let kind = if matches!(pending[k].p, P::Lim(_)) { "limit" } else { "stop" }; exec(k, lv, kind, &mut pos, &mut cur, &mut out, &mut last, &mut done, &mut entries); c0 = lv; }
                    None => { c0 = t; seg += 1; }
                }
            }
        }
        pending.clear();
        let st = StPp { pos, entry_px: cur.as_ref().map_or(0., |t| t.legs[0].px), entry_i: cur.as_ref().map_or(0, |t| t.legs[0].i), last_profit: last, entries_today: entries };
        let mut next = vec![];
        f(i, &st, &mut next);
        if i >= from && i < to { pending = next; }
    }
    if let Some(mut t) = cur.take() { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let cutoff = p.get("cutoff").copied().unwrap_or(30.);
    let xn = p.get("xma").copied().unwrap_or(20.).round().max(1.) as usize;
    let add = ticks(x, p);
    let (h, l) = (&x.h, &x.l);
    let (_, _, adx) = dmi(x, 14);
    let xma = xavg(&x.c, xn);
    let (mut bbuy, mut ssell, mut rebuy, mut resell) = (99999., 0., 99999., 0.);
    let (mut setbuy, mut setsell, mut gorebuy, mut goresell) = (false, false, false, false);
    let (mut lxstop, mut sxstop) = (0., 99999.);
    let (mut bbuy1, mut ssell1, mut rebuy1, mut resell1, mut gorebuy1, mut goresell1, mut mp1) = (99999., 0., 99999., 0., false, false, 0i8);
    Some(run_ts_pp(x, from, to, |i, st, next| {
        let mp = st.pos;
        if i >= 2 {
            if mp < 1 { lxstop = l[i].min(l[i - 1]).min(l[i - 2]) - add; }
            if mp > -1 { sxstop = h[i].max(h[i - 1]).max(h[i - 2]) + add; }
        }
        if bbuy1 < 99999. && h[i] >= bbuy1 { lxstop = f64::min(l[i] - add, lxstop); gorebuy = true; }
        if ssell1 > 0. && l[i] <= ssell1 { sxstop = f64::max(h[i] + add, sxstop); goresell = true; }
        next.push(o(A::Xl, P::Stop(lxstop), "LX"));
        next.push(o(A::Xs, P::Stop(sxstop), "SX"));
        if i > 0 && adx[i] > cutoff && adx[i] > adx[i - 1] {
            if l[i] > xma[i] { setbuy = true; }
            if h[i] < xma[i] { setsell = true; }
        }
        if l[i] > xma[i] { setsell = false; resell = 0.; }
        if h[i] < xma[i] { setbuy = false; rebuy = 99999.; }
        if mp == 1 { setbuy = false; }
        if mp == -1 { setsell = false; }
        if !setsell { ssell = 0.; }
        if !setbuy { bbuy = 99999.; }
        if mp1 == 0 {
            if l[i] <= resell1 && goresell1 { resell = 0.; }
            if h[i] >= rebuy1 && gorebuy1 { rebuy = 99999.; }
        }
        if mp == 0 {
            if st.last_profit > 0. || (i >= 2 && h[i] > h[i - 2] && h[i] > h[i - 1] && l[i] > xma[i]) { resell = 0.; goresell = false; }
            if resell > 0. { next.push(o(A::Sell, P::Stop(resell), "Re-Sell")); }
            if st.last_profit > 0. || (i >= 2 && l[i] < l[i - 2] && l[i] < l[i - 1] && h[i] < xma[i]) { rebuy = 99999.; gorebuy = false; }
            if rebuy < 99999. { next.push(o(A::Buy, P::Stop(rebuy), "Re-Buy")); }
        }
        if setbuy && l[i] <= xma[i] { bbuy = h[i] + add; }
        if setsell && h[i] >= xma[i] { ssell = l[i] - add; }
        if bbuy < 99999. { next.push(o(A::Buy, P::Stop(bbuy), "Buy-1")); rebuy = bbuy; }
        if ssell > 0. { next.push(o(A::Sell, P::Stop(ssell), "Sell-2")); resell = ssell; }
        bbuy1 = bbuy; ssell1 = ssell; rebuy1 = rebuy; resell1 = resell; gorebuy1 = gorebuy; goresell1 = goresell; mp1 = mp;
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tango_trades() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tstango_adx", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3);
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
        assert!(run("tstango_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Tango System 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tstango" && reviewed(def.id),
    label: "TradeStation trend-pullback system with no named market and no published results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Tango System Open_Code.txt (dancewiththemarkets.com, printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
