//! David Penn, "The Three Secrets to Trading Momentum Indicators" (Marketplace Books, 2009; book id `dpstmi`; e-book
//! text layer authors/penn/text/stmi.txt, chapter 6). Reading: lane B 29 Oct and day lane A1 30 Oct (admission/boso_ref.py).
//! Live: BOSO, 'buying the overbought and selling the oversold' (Ch.6): the stochastic entering the Greed / Fear zone,
//! confirmed by a follow-through close beyond that day's extreme, held until it slips out of the zone and follows
//! through out of it on a closing basis. Not coded: the 10/20/50 EMA realignment (Ch.9, no exit given), crossovers,
//! divergences and hooks (examples only), the candlestick exits Penn prefers (judgement). Book: S&P 500, currency pairs,
//! futures daily charts; indices, ETFs and US/India stocks here are a declared application.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/penn/admission/boso_ref.py (header)).
pub const REVIEWED: &[&str] = &["dpstmi_boso"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Stochastic settings 20 and 20 (%K and %D, Penn's preference for 'one of the other stochastic strategies I will introduce later', the BOSO); %K = 100 x (close - lowest low of 20 days) / (highest high - lowest low), %D = 20-day average of %K. The zone line is %K [declared; option: %D]. Greed Zone above 80, Fear Zone below 20 (the levels the chapter uses for overbought / oversold). Entry: the day %K moves above 80 from at or below it marks the setup; the first later close above that day's high while %K is still above 80 is the follow-through confirmation (p. Ch.6, S&P 500 Nov 2005: 'Confirmation in the form of a follow-through close above the high of the day did not arrive for five days'), and the trade is taken at that close; if %K drops back to 80 or less first, the setup lapses. Exit: the day %K falls back to 80 or less marks the slip; the first later close below that day's low is the follow-through out of the zone, and the trade closes at that close; if %K goes back above 80 first, the exit setup lapses and the next slip counts. Shorts mirror at 20 (oversold, close below the low, exit on a close above the high of the day it rises back to 20 or more). One position at a time; a confirmation on the day of an exit can open the new trade at the same close; trades open inside the test window; a position still open at the end is marked at the last close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dpstmi_boso", name: "BOSO: buy the overbought / sell the oversold - stochastic 20/20 into the 80/20 zone, follow-through close in and out",
        group: "Penn momentum indicators", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "dpstmi",
        rules: &[
            "F|\"for one of the other stochastic strategies I will introduce later, the settings of 20 and 20 are my preference\" (Ch.6); \"require that the cross take place in oversold territory (below 20) ... or in overbought territory (above 80)\" (Ch.6)",
            "E|\"the BOSO approach (shorthand for 'buying the overbought and selling the oversold') takes advantage of bullish and bearish momentum by waiting until the moment that momentum reaches a certain threshold, specifically, the overbought or oversold threshold\" (Ch.6)",
            "E|\"Confirmation in the form of a follow-through close above the high of the day did not arrive for five days.\" (Ch.6, Fig 6.7); \"if the market becomes oversold and shows follow-through to the downside, then the proper play is to continue (or begin) betting against the market\" (Ch.6)",
            "X|\"It is only after a market slips from the Greed or Fear threshold - and shows follow-through out of those zones on a closing basis in subsequent sessions - that the trader must abandon the trade.\" (Ch.6)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "dpstmi_boso").then_some(("dpstmi_boso", "Penn BOSO stochastic", "20/20, 80/20, follow-through closes"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "dpstmi_boso").then(|| vec![
        p("k", "%K length (book 20)", 20., 2., 100., 1.), p("d", "%D length (book 20)", 20., 1., 100., 1.),
        p("line", "Zone line: 0 = %K (declared), 1 = %D", 0., 0., 1., 1.),
        p("ob", "Greed Zone level (80)", 80., 50., 99., 1.), p("os", "Fear Zone level (20)", 20., 1., 50., 1.),
        p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let (kn, dn) = (g("k").round().max(2.) as usize, g("d").round().max(1.) as usize);
    let (ob, os, side) = (g("ob"), g("os"), g("side").round() as i64);
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    let mut k = vec![f64::NAN; n];
    for t in kn - 1..n {
        let (hh, ll) = (h[t + 1 - kn..=t].iter().cloned().fold(f64::MIN, f64::max), l[t + 1 - kn..=t].iter().cloned().fold(f64::MAX, f64::min));
        if hh > ll { k[t] = 100. * (c[t] - ll) / (hh - ll); }
    }
    let s = if g("line").round() >= 1. { sma(&k, dn) } else { k };
    // in-zone test for direction d: long = above ob, short = below os (NaN = not in the zone)
    let inz = |t: usize, d: f64| if d > 0. { s[t] > ob } else { s[t] < os };
    let (mut setup, mut slip) = ([None::<f64>; 2], None::<f64>); // setup[0] long ref high, [1] short ref low; slip ref
    let (mut out, mut pos) = (vec![], None::<(f64, usize)>);
    for t in 1..=to {
        // exit first
        if let Some((d, e)) = pos {
            if t > e {
                if let Some(rf) = slip { if d * (c[t] - rf) < 0. && !inz(t, d) {
                    out.push(Trade { dir: d, legs: vec![Leg { i: e, px: c[e], w: 1. }], exit_i: t, exit_px: c[t], reason: "Follow-through out of the zone".into(),
                        entry_kind: "close", signal_i: t, open: false, exits: vec![] });
                    pos = None; slip = None;
                } }
                if let Some((d, _)) = pos {
                    if inz(t, d) { slip = None; } else if inz(t - 1, d) { slip = Some(if d > 0. { l[t] } else { h[t] }); }
                }
            }
        }
        // setups and confirmations
        for (j, d) in [(0usize, 1.), (1, -1.)] {
            if !inz(t, d) { setup[j] = None; continue; }
            if !inz(t - 1, d) { setup[j] = Some(if d > 0. { h[t] } else { l[t] }); continue; }
            if let Some(rf) = setup[j] {
                if d * (c[t] - rf) > 0. {
                    setup[j] = None;
                    let ok = (d > 0. && side != 2) || (d < 0. && side != 1);
                    if pos.is_none() && ok && t >= from && t < to { pos = Some((d, t)); slip = None; }
                }
            }
        }
    }
    if let Some((d, e)) = pos {
        out.push(Trade { dir: d, legs: vec![Leg { i: e, px: c[e], w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_and_params() {
        assert!(reviewed("dpstmi_boso") && !reviewed("dpstmi_trio"));
        assert_eq!(params("dpstmi_boso").unwrap()[3].def, 80.);
    }
}
