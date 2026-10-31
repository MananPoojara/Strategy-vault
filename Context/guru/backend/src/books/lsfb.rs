//! Lank Stephens, "Trading the 4 Bar Fact & Calculating THE CLINCHER" (2nd edition, August 2007; book id `lsfb`; PDF
//! pages). Reading: claude-continuation-20261020/authors/lstephens (admission/fb_ref.py; text layer
//! authors/overnight-1029/D1w/txt/4bf.txt; CLINCHER grid formulas from the PDF form-field scripts). Live:
//! the 4 Bar Fact reversal (PDF 16-19, Reversal Exit PDF 25) - close beyond bar 4 and bar 2, only a reversal of the
//! previous signal counts, stop and reverse at the signal close; THE CLINCHER (PDF 47-53) - G = square root of
//! (high x low), a change of sign of G's daily change gives tomorrow's one-way trade from the open, out before the
//! session end. Not coded: the iceberg stop, the limit targets and time exits (no fixed values, PDF 25-26), the
//! aggressive stop entry (intraday). Book: T-bond, wheat, yen, gold, Swiss franc and eMini futures; indices, ETFs
//! and US/India stocks here are a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/lstephens/admission/fb_ref.py (header)).
pub const REVIEWED: &[&str] = &["lsfb_4bar_fact", "lsfb_clincher"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_F: &str = "[Declared readings] Bar 1 = today, bar 2 = yesterday, bar 4 = three bars ago (PDF 19). Sell signal: today's close below the low of bar 4 AND below the low of bar 2; buy signal: close above the high of bar 4 AND above the high of bar 2. A signal in the same direction as the previous signal is 'redundant' (R) and ignored (PDF 18); signals are counted from the start of the test window. Reversal Exit (PDF 25) with the book's Market-On-Close orders (PDF 17): each new signal closes the trade and opens the other side at that day's close, so the trader is always in. Side 1 = long only (a sell signal just closes the long), side 2 = short only. The book's 'use a stop' (iceberg) has no distance and its targets or time exits have no fixed values, so none is applied. A position still open on the last bar is marked at its close.";
const NOTE_C: &str = "[Declared readings] For each day G = square root of (high x low) and D = G today - G yesterday (the CLINCHER grid's 'H x L' and 'square root' fields, PDF 50). A buy signal is a positive D after a negative one, a sell signal a negative D after a positive one (red / black fonts, PDF 52); D = 0 keeps the previous sign. The signal is read at the close and the trade is tomorrow, in that direction only: in at tomorrow's open ('buying or selling on the Open', PDF 51) and out at that day's close ('exits are always before the session end', PDF 47). Checked on the book's Swiss franc prices (PDF 49): signals Buy 1/31, Sell 2/5, Buy 2/7, Sell 2/9 as printed (PDF 51). The limit targets (e.g. 15 or 35 ticks, PDF 52-53) are market-specific and not applied. Side 1 = long signals only, side 2 = short only. A signal on the last bar is not traded.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str| StrategyDef { id, name, group: "Stephens 4 Bar Fact & CLINCHER", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "lsfb", rules, notes, claim: None };
    vec![
        d("lsfb_4bar_fact", "4 Bar Fact: close beyond bar 4 and bar 2, reverse at the close", &[
            "E|\"To diagnose a SELL SIGNAL ... Condition A: Bar 1 (the current bar) must Close below the Low of Bar 4 (counting right to left, from most recent to previous). Condition B: Bar 1 must Close below the Low of Bar 2.\" Buy signal: \"Close above the High of Bar 4\" and \"above the High of Bar 2\" (PDF 19)",
            "F|\"A 4BF is labelled 'R' for redundant when the previous 4BF points in the same direction; ... a 4BF does not become a signal for entry unless it indicates a reversal in trend.\" (PDF 18)",
            "X|\"REVERSAL EXIT: Exit only upon 4BF reversal (new signal). Exit is simultaneous with next trade entry.\" (PDF 25); \"this is using a Market-On-Close order for each exit/new entry\" (PDF 17)"], NOTE_F),
        d("lsfb_clincher", "THE CLINCHER: sign change of the square root of high x low, one-way trade tomorrow from the open", &[
            "F|Grid fields: \"H x L\" = High x Low (AFSimple_Calculate PRD); \"square root\" = Math.sqrt(H x L); the daily figure = square root today - square root yesterday (PDF 50, form scripts)",
            "E|\"A valid signal must be preceded by a contrary direction; that is, if the previous number was a red font, the new number must be black in order to indicate a Buy signal\" (PDF 52); \"buying or selling on the Open\" (PDF 51); \"take only Long signals intra-day or Short signals intra-day - but never both\" (PDF 48)",
            "X|\"when you are signalled to jump in, exits are always before the session end\" (PDF 47)"], NOTE_C),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "lsfb_4bar_fact" => Some(("lsfb_4bar_fact", "Stephens 4 Bar Fact reversal", "close vs bars 2 and 4")),
        "lsfb_clincher" => Some(("lsfb_clincher", "Stephens CLINCHER one-way day", "sqrt(H x L) sign change")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "lsfb_4bar_fact" | "lsfb_clincher" => Some(vec![p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.)]),
        _ => None,
    }
}

fn mk(d: f64, ei: usize, ep: f64, kind: &'static str, xi: usize, xp: f64, why: &str, open: bool) -> Trade {
    Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: xi, open, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let side = p.get("side").copied().unwrap_or(schema[0].def).round() as i64;
    let ok = |d: f64| side == 0 || (side == 1 && d > 0.) || (side == 2 && d < 0.);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    if id == "lsfb_4bar_fact" {
        let (mut last, mut cur) = (0., None::<(f64, usize)>);
        for t in from.max(3)..=to {
            let s = if c[t] < l[t - 3] && c[t] < l[t - 1] { -1. } else if c[t] > h[t - 3] && c[t] > h[t - 1] { 1. } else { 0. };
            if s == 0. || s == last { continue; }
            last = s;
            if let Some((d, e)) = cur.take() { out.push(mk(d, e, c[e], "close", t, c[t], "4BF reversal", false)); }
            if ok(s) && t < to { cur = Some((s, t)); }
        }
        if let Some((d, e)) = cur { out.push(mk(d, e, c[e], "close", to, c[to], "open", true)); }
    } else {
        let g: Vec<f64> = (0..c.len()).map(|i| (h[i] * l[i]).sqrt()).collect();
        let mut sign = 0.;
        for t in 1..to {
            let dd = g[t] - g[t - 1];
            let s = if dd > 0. { 1. } else if dd < 0. { -1. } else { sign };
            if s != sign && sign != 0. && t >= from && ok(s) { out.push(mk(s, t + 1, o[t + 1], "open", t + 1, c[t + 1], "session close", false)); }
            sign = s;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(i: usize, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: format!("2007-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn clincher_matches_the_book_swiss_franc_signals() {
        // PDF 49: S6H7 1/24-2/12/2007; signals Buy 1/31, Sell 2/5, Buy 2/7, Sell 2/9 (PDF 51)
        let v = [(8085., 8094., 8039., 8047.), (8048., 8086., 8037., 8046.), (8042., 8050., 7979., 8013.), (8016., 8028., 7995., 8015.),
            (8017., 8035., 8006., 8021.), (8024., 8080., 7987., 8073.), (8067., 8105., 8057., 8076.), (8072., 8123., 8032., 8047.),
            (8037., 8051., 8018., 8039.), (8037., 8102., 8031., 8093.), (8094., 8106., 8077., 8081.), (8083., 8089., 8019., 8052.),
            (8053., 8053., 8011., 8039.), (8045., 8051., 8034., 8047.)];
        let r: Vec<Row> = v.iter().enumerate().map(|(i, &(o, h, l, c))| bar(i, o, h, l, c)).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lsfb_clincher", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|z| (z.dir, z.legs[0].i, z.legs[0].px, z.exit_px)).collect();
        // bar 5 = 1/31, 8 = 2/5, 10 = 2/7, 12 = 2/9
        assert_eq!(got, vec![(1., 5, 8024., 8073.), (-1., 8, 8037., 8039.), (1., 10, 8094., 8081.), (-1., 12, 8053., 8039.)]);
        assert!(run("lsfb_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
    #[test]
    fn four_bar_fact_reverses_and_skips_redundant_signals() {
        let r = vec![bar(0, 10., 11., 9., 10.), bar(1, 10., 11., 9., 10.), bar(2, 10., 11., 9., 10.),
            bar(3, 10., 12., 10., 11.5), // close 11.5 > high of bar 4 (11) and bar 2 (11): buy
            bar(4, 11.5, 13., 11., 12.5), // buy again: redundant
            bar(5, 12.5, 12.6, 10., 10.5), // close 10.5 < low bar 2 (11) but not < low of bar 4 (10): nothing
            bar(6, 10.5, 10.6, 9., 9.5)]; // close 9.5 < low of bar 4 (10) and of bar 2 (10): sell
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lsfb_4bar_fact", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|z| (z.dir, z.legs[0].i, z.exit_i, z.exit_px, z.open)).collect();
        assert_eq!(got, vec![(1., 3, 6, 9.5, false)]); // the short on the last bar is not opened
    }
}

// Instrument admission: Lank Stephens 2026-10-29 (lane K1) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lstephens/admission/fb_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="lsfb" && crate::books::lsfb::reviewed(def.id),
    label: "Lank Stephens trades the 4 Bar Fact and THE CLINCHER on T-bond, grain, currency, gold and eMini futures (some stocks); runs on indices, ETFs and US/India stocks are a declared application, not the author's test.",
    basis: "Lank Stephens, Trading the 4 Bar Fact & Calculating THE CLINCHER (2nd ed., 2007), PDF 16-19, 25, 47-53",
};
