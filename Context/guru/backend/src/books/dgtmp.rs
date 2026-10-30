//! Dave Gagné, "Trading Master Plan" (Deluxe online edition, 2005, tradingmasterplan.com; book id `dgtmp`; printed
//! pages = PDF - 5). Reading: lane B 29 Oct (laneb/b3/triage-301-360.json) and day lane A1 30 Oct (text layer
//! authors/gagne/text/tmp.txt; admission/spring_ref.py). Live: the spring buy / sell setup (p.112-116) with the book's
//! entry rule (p.165), initial stop (p.115-116) and management rules (p.165-166: 20 MA - 20 ATR - $0.05 trailing stop,
//! half out at the 5Xma extreme 20 MA + 5 ATR). Not coded: the 30-minute high entry (intraday), the sentiment switch,
//! NRB, hammer and points scoring (refinements), the time stop (no value), the gap rules (5- and 30-minute lows).
//! Book: US stocks; dollar amounts -> US indices, ETFs and stocks only.
use crate::connors::{atr, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/gagne/admission/spring_ref.py (header)).
pub const REVIEWED: &[&str] = &["dgtmp_spring"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Setup on the daily close (p.114-115): the last three bars each have a lower high and a lower low than the bar before and closed below their open (red); sell setup: three higher lows, higher highs and green bars. The book wants this pullback inside a 'phase two' uptrend (p.113, 115), a chart judgement; declared convention [not from the book]: the 20-day and the 40-day simple averages (the book's core averages, p.84) both rose on the setup day (both fell for a sell setup); option 1 = no trend filter. Entry next day on a buy stop one cent above the setup day's high ('trades above the prior day's high'), filled at the stop or at the open if it gaps above; no trade when the fill is more than $0.25 above the stop (p.165 'never buy a stock that has moved more than $0.25 above the ideal entry price'). Initial stop $0.05 below the lower of the entry day's and the setup day's low (p.115), live from the day after entry. Each close sets the next day's levels: trailing stop 20-day SMA - 20-day ATR (Wilder) - $0.05, used once above the stop, never lowered (p.165, rule 3 p.164); 5Xma extreme = 20-day SMA + 5 x ATR: the first day the high reaches it, half the position is sold there (or at the open if above), the other half on a later day that reaches the level again, or on the stop (p.166). A day that touches both the stop and the 5Xma level is read stop first [not from the book]. A gap through the stop exits at the open. Shorts mirror every level. Option: the '$1 rule' refinement (stop to break-even once the trade is $1 in profit, p.165-166). The $12 / $0.03 substitutions are not applied. One position at a time; trades open inside the test window; a position still open at the end is marked at the last close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dgtmp_spring", name: "Spring buy / sell setup: three lower highs, lower lows and red bars in an uptrend, buy above the prior high; 20 MA - 20 ATR trail, half out at 20 MA + 5 ATR",
        group: "Gagné Trading Master Plan", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "dgtmp",
        rules: &[
            "E|\"Spring buy setup entry criteria: Setup - Three or more consecutive lower highs with emphasis on the highs. Three or more consecutive lower lows. Three or more consecutive dark or red bars.\" (p.114); \"Action - Buy when the stock trades above the prior day's high\" (p.115)",
            "F|\"Don't forget you're looking for this pattern during an overall phase two\" (p.115); \"Uptrend: Defined by a series of higher highs and higher lows (phase two of the cycle)\" (p.35); \"Rising major moving averages represent positive market action or strength\" (p.84) - convention: rising 20- and 40-day averages",
            "E|\"Never buy a stock that has moved more than $0.25 above the ideal entry price\" (p.165)",
            "X|\"Place your initial stop $0.05 below the entry day's low or the prior day's low, whichever is lower\" (p.115); \"Trailing stop is used once the formula is greater then the initial stop. The formula is 20 MA - 20 ATR - $0.05.\" (p.165); \"The stop loss can never be lowered - only raised.\" (p.164)",
            "X|\"Use the 5Xma extreme profit rule ... The formula for taking profits is 5 times the ATR value + 20 MA.\" \"I always sell half of my position the first time it hits my extreme exit point ... and the back half again several days later using the 5Xma rule\" (p.166)",
            "E|Spring sell setup: \"Three or more consecutive higher lows ... higher highs ... green bars. Short when the stock trades below the prior day's low ... initial stop $0.05 below [above] the entry day's high or the prior day's high, whichever is higher\" (p.116)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "dgtmp_spring").then_some(("dgtmp_spring", "Gagné spring setup", "3 lower highs/lows/red, 20MA-20ATR trail"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "dgtmp_spring").then(|| vec![
        p("trend", "Phase-two filter: 0 = 20- and 40-day averages both rising (declared convention), 1 = none", 0., 0., 1., 1.),
        p("be1", "$1 rule: 0 = off (book: optional refinement), 1 = stop to break-even after a $1 move", 0., 0., 1., 1.),
        p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let (trend, be1, side) = (g("trend").round() < 1., g("be1").round() >= 1., g("side").round() as i64);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (m20, m40, a20) = (sma(c, 20), sma(c, 40), atr(x.r, 20));
    let setup = |t: usize, d: f64| -> bool {
        if t < 3 { return false; }
        let pat = (0..3).all(|j| { let k = t - j; d * (h[k - 1] - h[k]) > 0. && d * (l[k - 1] - l[k]) > 0. && d * (o[k] - c[k]) > 0. });
        pat && (!trend || (d * (m20[t] - m20[t - 1]) > 0. && d * (m40[t] - m40[t - 1]) > 0.))
    };
    let mut out = vec![];
    // (dir, entry bar, entry px, kind, stop, half sold: (bar, px))
    let mut pos: Option<(f64, usize, f64, &'static str, f64, Option<(usize, f64)>)> = None;
    let close = |ps: &(f64, usize, f64, &'static str, f64, Option<(usize, f64)>), i: usize, px: f64, why: &str, open: bool| {
        let (exits, xp) = match ps.5 { Some((hi, hp)) => (vec![(hi, hp, 0.5), (i, px, 0.5)], (hp + px) / 2.), None => (vec![], px) };
        Trade { dir: ps.0, legs: vec![Leg { i: ps.1, px: ps.2, w: 1. }], exit_i: i, exit_px: xp, reason: why.into(), entry_kind: ps.3, signal_i: i, open, exits }
    };
    for t in 1..=to {
        if let Some(ps) = pos.as_mut() {
            let d = ps.0;
            if t > ps.1 {
                let tg = m20[t - 1] + d * 5. * a20[t - 1];
                let (fx, ax) = if d > 0. { (h[t], l[t]) } else { (l[t], h[t]) };
                let hit = if d * (o[t] - ps.4) <= 0. { Some(o[t]) } else if d * (ax - ps.4) <= 0. { Some(ps.4) } else { None };
                if let Some(px) = hit { let tr = close(ps, t, px, "Stop", false); out.push(tr); pos = None; }
                else if tg.is_finite() && d * (fx - tg) >= 0. {
                    let px = if d * (o[t] - tg) >= 0. { o[t] } else { tg };
                    match ps.5 { None => ps.5 = Some((t, px)), Some(_) => { let tr = close(ps, t, px, "5Xma extreme", false); out.push(tr); pos = None; } }
                }
            }
            // next day's stop from this close
            if let Some(ps) = pos.as_mut() {
                let d = ps.0;
                let tr = m20[t] - d * (a20[t] + 0.05);
                if tr.is_finite() && d * (tr - ps.4) > 0. { ps.4 = tr; }
                if be1 && d * ((if d > 0. { h[t] } else { l[t] }) - ps.2) >= 1. && d * (ps.2 - ps.4) > 0. { ps.4 = ps.2; }
            }
            continue;
        }
        if t < from || t >= to { continue; }
        for d in [1., -1.] {
            if (d > 0. && side == 2) || (d < 0. && side == 1) || !setup(t - 1, d) { continue; }
            let stp = if d > 0. { h[t - 1] + 0.01 } else { l[t - 1] - 0.01 };
            if d * ((if d > 0. { h[t] } else { l[t] }) - stp) < 0. { continue; }
            let gap = d * (o[t] - stp) > 0.;
            let fill = if gap { o[t] } else { stp };
            if d * (fill - stp) > 0.25 + 1e-9 { continue; }
            let st = if d > 0. { l[t].min(l[t - 1]) - 0.05 } else { h[t].max(h[t - 1]) + 0.05 };
            let mut ps = (d, t, fill, if gap { "open" } else { "stop" }, st, None);
            let tr = m20[t] - d * (a20[t] + 0.05);
            if tr.is_finite() && d * (tr - ps.4) > 0. { ps.4 = tr; }
            if be1 && d * ((if d > 0. { h[t] } else { l[t] }) - fill) >= 1. && d * (fill - ps.4) > 0. { ps.4 = fill; }
            pos = Some(ps);
            break;
        }
    }
    if let Some(ps) = pos { out.push(close(&ps, to, c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_and_params() {
        assert!(reviewed("dgtmp_spring") && !reviewed("dgtmp_x") && params("dgtmp_spring").unwrap().len() == 3);
    }
}
