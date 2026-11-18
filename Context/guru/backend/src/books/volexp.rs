//! Anand Rathi, "Volume Expansion: An edge for a low risk entry" (VOLEXPO.pptx, 44 slides; research share 'Prasad/VOLEXPO.pptx'). Book id
//! `volexp`. Mechanical content = slides 8-14: a high-volume candle (HVC); "Entry: candle close above or below high or low of HVC candle.
//! SL: low or high of HVC. Target: minimum target = range of HVC" (slide 14). The three HVC types (PRAKAT, GUPT, KAPAT, slides 10-12) are
//! told apart by candle colour against "the recent trend", which the deck does not measure, so one HVC rule covers all three (declared).
//! Slides 16-38 are chart examples on Indian stocks and Nifty; slides 40-43 are money-management and psychology text.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "volexp";
pub const REVIEWED: &[&str] = &["volexp_hvc_break"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings, each one an unstated detail of the deck, so the card is UNVERIFIED] (1) 'higher volumes than the recent sessions / recent full leg of move' (slides 8, 10): an HVC is a bar whose volume is at least the highest volume of the last N = 20 bars including itself. (2) The deck gives no measure of 'recent trend' or colour for the three types (slides 10-12): they are not applied, every HVC counts. (3) The deck does not say how long an HVC stays valid: the first close beyond its high (long) or low (short) within K = 5 bars after it, with the most recent HVC taking priority, opens the trade and uses that HVC up. (4) 'Minimum target - range of HVC' (slide 14) is the only exit target coded, entry price + (HVC high - HVC low) for a long, mirrored for a short; the 'probable target' (swing or supply/demand zone) is a judgement and not coded. Fill convention of the vault: entry at the breakout bar's close; stop and target from the next bar, a stop through a gap fills at the open, a target through a gap fills at the open, both inside one daily bar counts as the stop first (counted as order-dependent).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "volexp_hvc_break", name: "Volume Expansion: close beyond the high-volume candle, stop at its other end, target its range", group: "Anand Rathi Volume Expansion", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: "volexp",
        rules: &[
            "F|\"Whenever higher volumes are observed than in the recent sessions with or without major change in price behaviour we consider it as Volume Expansion\" (slide 8)",
            "E|\"Entry: Candle close above or below high or low of HVC candle\" (slide 14)",
            "X|\"SL: Low or high of HVC. Target: Minimum target - Range of HVC\" (slide 14)"], notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "volexp_hvc_break" => Some(("volexp_hvc_break", "Volume Expansion HVC break", "N 20, K 5, target 1 x HVC range")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "volexp_hvc_break" => Some(vec![p("n", "N, bars in the volume comparison (declared 20)", 20., 5., 100., 1.),
            p("k", "K, bars an HVC stays valid (declared 5)", 5., 1., 30., 1.),
            p("tgt", "Target as a multiple of the HVC range (deck: 1, the minimum target)", 1., 0.5, 5., 0.5)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (n, k, tm) = (g("n").round().max(2.) as usize, g("k").round().max(1.) as usize, g("tgt"));
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    // hvc[j]: bar j has the highest volume of the n bars ending at j (all n inside the run window: warm-up from the window start)
    let hvc: Vec<bool> = (0..x.r.len()).map(|j| j + 1 >= n + from && vol[j] > 0. && (j + 1 - n..=j).all(|q| vol[q] == vol[q] && vol[q] <= vol[j])).collect();
    let (mut out, mut used) = (vec![], vec![false; x.r.len()]);
    let mut i = from;
    while i <= to {
        // reference HVC: the latest one in [i-k, i-1] that is inside the run window and unused
        let r = (i.saturating_sub(k)..i).rev().find(|&j| hvc[j] && !used[j]);
        let Some(j) = r else { i += 1; continue };
        let d = if x.c[i] > x.h[j] { 1. } else if x.c[i] < x.l[j] { -1. } else { 0. };
        if d == 0. { i += 1; continue; }
        used[j] = true;
        let (e, rng) = (x.c[i], x.h[j] - x.l[j]);
        let (stp, tgt) = if d > 0. { (x.l[j], e + tm * rng) } else { (x.h[j], e - tm * rng) };
        let mut exit = None;
        for q in i + 1..=to {
            let sh = if d > 0. { x.o[q] <= stp || x.l[q] <= stp } else { x.o[q] >= stp || x.h[q] >= stp };
            let th = if d > 0. { x.o[q] >= tgt || x.h[q] >= tgt } else { x.o[q] <= tgt || x.l[q] <= tgt };
            if sh {
                let px = if d > 0. { x.o[q].min(stp) } else { x.o[q].max(stp) };
                exit = Some((q, px, "stop")); break;
            }
            if th {
                let px = if d > 0. { x.o[q].max(tgt) } else { x.o[q].min(tgt) };
                exit = Some((q, px, "target")); break;
            }
        }
        let (xi, xp, why, open) = match exit { Some((q, px, w)) => (q, px, w, false), None => (to, x.c[to], "open", true) };
        out.push(Trade { dir: d, legs: vec![Leg { i, px: e, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] });
        i = xi + 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bars(b: &[(f64, f64, f64, f64, f64)]) -> Vec<Row> {
        b.iter().enumerate().map(|(i, &(o, h, l, c, v))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: Some(v) }).collect()
    }
    #[test]
    fn close_above_hvc_buys_and_the_range_target_exits() {
        // n = 3: bar 2 has the top volume (HVC high 11, low 9, range 2); bar 3 closes 11.5 > 11 -> long at 11.5, stop 9, target 13.5
        let r = bars(&[(10., 10.5, 9.5, 10., 100.), (10., 10.5, 9.5, 10., 100.), (10., 11., 9., 10., 300.), (10.5, 11.6, 10.4, 11.5, 100.),
            (11.5, 12., 11., 12., 100.), (12., 14., 11.8, 13., 100.)]);
        let x = Ctx::new(&r, &r, &[]);
        let p: Params = [("n".to_string(), 3.), ("k".to_string(), 5.)].into_iter().collect();
        let t = run("volexp_hvc_break", &x, 0, 5, &p).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (1., 3, 11.5, 5, 13.5, "target"));
        assert!(run("volexp_x", &x, 0, 5, &p).is_none());
    }
}

// Instrument admission: Anand Rathi (Volume Expansion deck) 2026-11-18 (lane CV1). Admission only for the module REVIEWED lists.
// Runs on actual indices, ETFs and stocks that carry daily volume, US and India (declared application); the deck's examples are Indian stocks and Nifty.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "volexp" && crate::books::volexp::reviewed(def.id),
    label: "Anand Rathi shows chart examples on Indian stocks and Nifty and gives no test results; runs on indices, ETFs and US/India stocks with real daily volume are a declared application, not the author's test.",
    basis: "Anand Rathi, Volume Expansion: An edge for a low risk entry (VOLEXPO.pptx), slides 8-14",
};
