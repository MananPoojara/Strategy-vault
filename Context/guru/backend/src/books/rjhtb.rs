//! Robert J. Hall Jr., "Trading The Bands" (bandtraders.com; research share USB HDD 1/TradeStation Systems & Indicators/
//! Trading The Bands System + Manual For TS (bandtraders.com)/Trading The Bands.doc, section 'TRADING THE PAINT BARS', and
//! Instructions.pdf p.2). Book id `rjhtb`. One rule: the GTrade Paint Bars (RSI(21) colours, stop-and-reverse). The band
//! trades are not coded: the G Channel formula exists only in the compiled ELA. Excerpt authors/titles-1030/R3w/bands_paintbars.txt.
//! Lane K5 (titles loop, 31 Oct 2026); independent check authors/rjhall/admission/k5_ref.py (header).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/rjhall/admission/k5_ref.py (header)).
pub const REVIEWED: &[&str] = &["rjhtb_paint"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Bar colour on the close: blue/green when RSI(21) (the vault's Wilder RSI) closes above 51, red when it closes below 49; between 49 and 51 the colour, and the position, stay as they were. 'BUY blue - SELL red, reverse if necessary, but stay with the colors': always in after the first coloured bar, long on blue and short on red. 'Buy at the mkt' = at the next open after the close that changes the colour; a reversal closes the old side and opens the new one at that open. The author trades a 3-minute S&P chart but writes 'or whatever time frame you prefer': daily bars are a declared application. 'Place a stop' gives no size, so no stop is applied (the 2-point stop belongs to the band trades). A position still open at the window end is marked at the last close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "rjhtb_paint", name: "Trading The Bands Paint Bars: long when RSI(21) closes above 51, short when it closes below 49, stop-and-reverse at the next open", group: "Hall Trading The Bands",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "rjhtb", rules: &[
            "F|\"the bars will turn blue when the 21 bar RSI closes above 51 and turns red when the 21 bar RSI closes below 49\" (.doc, TRADING THE PAINT BARS); \"If the RSI closes over 51 the bars will be green, closes under 49, bars will be red.\" (Instructions.pdf p.2)",
            "E|\"BUY blue------SELL red, reverse if necessary, but stay with the colors.\" (.doc)",
            "E|\"If the BARS turn GREEN, buy at the mkt, Red, sell at the mkt.\" (Instructions.pdf p.2) [reading] at the next open",
            "X|The opposite colour reverses the position (no stop size is given)"],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "rjhtb_paint").then_some(("rjhtb_paint", "Hall Paint Bars", "RSI(21) 51/49"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max| ParamDef { key, label, def, min, max, step: 1. };
    (id == "rjhtb_paint").then(|| vec![p("n", "RSI period (book 21)", 21., 2., 100.), p("up", "Blue above (book 51)", 51., 50., 90.), p("dn", "Red below (book 49)", 49., 10., 50.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (rs, up, dn) = (x.rsi_n(g("n").round().max(1.) as usize), g("up"), g("dn"));
    let (o, c) = (&x.o, &x.c);
    let mk = |d: f64, e: usize, xi: usize, xp: f64, sig: usize, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: e, px: o[e], w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "open", signal_i: sig, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<(f64, usize)>);
    for i in from..to {
        let col = if rs[i] > up { 1. } else if rs[i] < dn { -1. } else { 0. };
        if col == 0. || cur.is_some_and(|(d, _)| d == col) { continue; }
        if let Some((d, e)) = cur { out.push(mk(d, e, i + 1, o[i + 1], i, if col > 0. { "blue bar, reversed next open" } else { "red bar, reversed next open" }, false)); }
        cur = Some((col, i + 1));
    }
    if let Some((d, e)) = cur { out.push(mk(d, e, to, c[to], to, "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn paint_bars_reverse_on_colour_change_at_next_open() {
        let cl: Vec<f64> = (0..300).map(|i| 100. + 15. * ((i as f64) / 20.).sin()).collect();
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i % 28 + 1), open: c + 0.3, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("rjhtb_paint", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let rs = x.rsi_n(21);
        assert!(t.len() > 3);
        for w in t.windows(2) { assert!(w[0].exit_i == w[1].legs[0].i && w[0].dir == -w[1].dir && w[0].exit_px == x.o[w[0].exit_i]); }
        for t in &t { let s = t.legs[0].i - 1; assert!(if t.dir > 0. { rs[s] > 51. } else { rs[s] < 49. }); }
        assert!(run("rjhtb_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "rjhtb" && crate::books::rjhtb::reviewed(def.id),
    label: "The author trades the Paint Bars on a 3-minute S&P futures chart ('or whatever time frame you prefer'); daily bars on indices, ETFs and stocks, US and India, are a declared application, not the author's test.",
    basis: "Robert J. Hall Jr., Trading The Bands (bandtraders.com): Trading The Bands.doc 'TRADING THE PAINT BARS' and Instructions.pdf p.2",
};
