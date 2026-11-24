//! David Penn, "The Three Secrets to Trading Momentum Indicators" (Marketplace Books, 2009; book id `dpstmi`; e-book
//! text layer authors/penn/text/stmi.txt, chapter 6). Reading: lane B 29 Oct and day lane A1 30 Oct (admission/boso_ref.py).
//! Live: BOSO, 'buying the overbought and selling the oversold' (Ch.6): the stochastic entering the Greed / Fear zone,
//! confirmed by a follow-through close beyond that day's extreme, held until it slips out of the zone and follows
//! through out of it on a closing basis. Not coded: the 10/20/50 EMA realignment (Ch.9, no exit given), crossovers,
//! divergences and hooks (examples only), the candlestick exits Penn prefers (judgement). Book: S&P 500, currency pairs,
//! futures daily charts; indices, ETFs and US/India stocks here are a declared application.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/penn/admission/boso_ref.py (header)).
pub const REVIEWED: &[&str] = &["dpstmi_boso", "dpstmi_kleinman_trio"];
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
        notes: NOTE, claim: None },
    StrategyDef { id: "dpstmi_kleinman_trio", name: "Kleinman 2/9/30 EMA trio: EMA(2) crosses EMA(9) with both on the trend side of EMA(30)",
        group: "Penn momentum indicators", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "dpstmi",
        rules: &[
            "F|\"Kleinman uses a 2-, 9-, and 30-period trio of exponential moving averages ... the longest EMA, the 30-period, exclusively as a trend-tracking indicator. He will only buy markets trading above the 30-period EMA, and will only short markets that are trading below the 30-period EMA.\" (Ch.9, PDF p.55)",
            "E|\"When the 2-period EMA crosses above the 9-period EMA and both moving averages are above the 30-period EMA, then a buy signal has been generated. When the 2-period EMA crosses below the 9-period EMA and both moving averages are below the 30-period EMA, then a short signal has been generated.\" (Ch.9, PDF p.55)",
            "X|[Declared reading, lane K2] The book states no exit: a position is held until the opposite signal (a short signal closes a long and reverses it, and the other way round); no stop or target is invented.",
            "A|[Declared readings, lane K2, 2] (1) the opposite signal is the exit (above); (2) the signals are known at the close, so the order fills at the NEXT bar's close (R4 #374). EMAs are the usual 2/(n+1) smoothing started at the first close [book silent]. Kleinman's trio is quoted in Penn's Ch.9 (Mastering Commodity Futures and Options); the book prints no results. Run on the NSE indices and stocks as a declared application."],
        notes: "Penn Ch.9 quoting Kleinman; the entry is printed, the exit is not (reading above). Penn's own 10/20/50 realignment in the same chapter stays uncoded (no exit given).", claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "dpstmi_boso" => Some(("dpstmi_boso", "Penn BOSO stochastic", "20/20, 80/20, follow-through closes")), "dpstmi_kleinman_trio" => Some(("dpstmi_kleinman", "Kleinman EMA trio (Penn Ch.9)", "2/9/30")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if id == "dpstmi_kleinman_trio" { return Some(vec![p("fast", "Fast EMA (2)", 2., 1., 20., 1.), p("mid", "Middle EMA (9)", 9., 2., 60., 1.), p("slow", "Trend EMA (30)", 30., 5., 200., 1.)]); }
    (id == "dpstmi_boso").then(|| vec![
        p("k", "%K length (book 20)", 20., 2., 100., 1.), p("d", "%D length (book 20)", 20., 1., 100., 1.),
        p("line", "Zone line: 0 = %K (declared), 1 = %D", 0., 0., 1., 1.),
        p("ob", "Greed Zone level (80)", 80., 50., 99., 1.), p("os", "Fear Zone level (20)", 20., 1., 50., 1.),
        p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    if id == "dpstmi_kleinman_trio" { return Some(kleinman(x, from, to, p)); }
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

/// EMA with 2/(n+1) smoothing started at the first close.
fn ema_series(c: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut o = Vec::with_capacity(c.len());
    for (i, v) in c.iter().enumerate() { o.push(if i == 0 { *v } else { a * v + (1.0 - a) * o[i - 1] }); }
    o
}

fn kleinman(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let (e2, e9, e30) = (ema_series(&x.c, g("fast", 2.)), ema_series(&x.c, g("mid", 9.)), ema_series(&x.c, g("slow", 30.)));
    let mut pos = super::tselhs::Book::new();
    let mut pend = 0.0;
    for t in from..=to {
        if pend != 0.0 { pos.exit(t, x.c[t], "reversed"); pos.enter(pend, t, x.c[t], "close"); pend = 0.0; }
        if t > from && t < to {
            let d = if e2[t] > e9[t] && e2[t - 1] <= e9[t - 1] && e2[t] > e30[t] && e9[t] > e30[t] { 1.0 }
                else if e2[t] < e9[t] && e2[t - 1] >= e9[t - 1] && e2[t] < e30[t] && e9[t] < e30[t] { -1.0 } else { 0.0 };
            if d != 0.0 && d != pos.side() { pend = d; }
        }
    }
    pos.done(to, x.c[to])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_and_params() {
        assert!(reviewed("dpstmi_boso") && reviewed("dpstmi_kleinman_trio") && !reviewed("dpstmi_trio"));
        assert_eq!(params("dpstmi_boso").unwrap()[3].def, 80.);
    }
    #[test]
    fn kleinman_reverses_at_the_next_close() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("dpstmi_kleinman_trio", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5);
        for z in &t { assert_eq!(z.legs[0].px, r[z.legs[0].i].close); }
    }
    /// hand-run dump for the pandas reference: DPK_CSV=<idx_data.csv> DPK_OUT=<file> cargo test --release dpk_dump -- --ignored
    #[test]
    #[ignore]
    fn dpk_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("DPK_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("DPK_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for z in run("dpstmi_kleinman_trio", &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "dpstmi_kleinman_trio,{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

// Instrument admission: David Penn 2026-10-30 (day lane A1) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/penn/admission/boso_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application); dpstmi_kleinman_trio is gated to the five NSE indices.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="dpstmi" && crate::books::dpstmi::reviewed(def.id),
    label: "David Penn shows BOSO on S&P 500, currency pair and futures daily charts; dpstmi_boso: runs on indices, ETFs and US/India stocks are a declared application, not the author's test. dpstmi_kleinman_trio (Ch.9, quoting Kleinman): the five NSE index series its independent reference covers only (the gate row allows only those, R4 #378).",
    basis: "David Penn, The Three Secrets to Trading Momentum Indicators (2009), Ch.6 Stochastics: BOSO; Ch.9 Moving Average Trios: Kleinman 2/9/30",
};
