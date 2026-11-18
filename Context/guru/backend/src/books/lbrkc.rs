//! Linda Bradford Raschke, "A Little Keltner, a Little Wyckoff, and a Lot of Street Smarts" (TAG XVIII, New Orleans 1996, INO
//! workbook, 25 pp scan; research share Trading Knowledge II/@INO (ino.com)/Linda Raschke - A Litle Keltner, a Litle Wycoff and of
//! lot of Street Smarts/RASCHKE1.PDF). Book id `lbrkc`. Lane CV2 (18 Nov 2026). Page = PDF page of RASCHKE1.PDF.
//! Live: the "second set of mechanical rules" of p.5: enter on the close of the first bar to close outside the Keltner Channels
//! (2.5 x 20-day average daily range around the 20-period EMA, p.3), initial stop at the entry bar's low (high), the 3 x 3 offset
//! SMA as trailing stop. Everything else in the workbook (Baby Grail, ADX filters, Wyckoff, tape reading) is a filter or judgement.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["lbrkc_close_outside"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Complete rule, four labelled readings; UNVERIFIED (R4 #339/#343); the workbook prints no results for it] Source p.3 and p.5. Printed: Keltner Channels 'set at 2.5 times the 20-day average daily range centered around the 20-period EMA'; 'enter on the close of the first bar to close outside the Channels'; 'the low (or high) of this initial entry bar should be used as the initial stop'; 'the 3 x 3 offset SMA functions as an excellent trailing stop'. Readings: (1) average daily range = 20-bar simple average of (high - low), the EMA is the exponential average seeded with the first close; (2) 'first bar to close outside' = a close beyond the band while the previous bar's close was not beyond it (bands as of that bar); (3) the 3 x 3 offset SMA = the 3-bar simple average of closes displaced 3 bars forward (its value on bar j is the average of the closes of bars j-5..j-3, known before bar j opens); (4) the stop on each bar after entry is the tighter of the entry-bar low and the 3 x 3 line (long; mirrored short), a stop order that fills at its level, or at the open if the bar opens beyond it; the first stop check is the bar after entry. Both sides are traded (the p.5 text gives 'low (or high)'); one position at a time, a new signal is taken only when flat.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "lbrkc_close_outside", name: "Raschke: enter on the first close outside the Keltner Channel, stop at the entry bar's extreme, 3x3 offset SMA trail",
        group: "A Little Keltner, a Little Wyckoff (Raschke, TAG 1996) p.3-5", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "lbrkc",
        rules: &["F|\"Keltner Channels set at 2.5 times the 20-day average daily range centered around the 20-period EMA\" (p.3)",
            "E|\"enter on the close of the first bar to close outside the Channels. The low (or high) of this initial entry bar should be used as the initial stop\" (p.5)",
            "X|\"The 3 x 3 offset SMA functions as an excellent trailing stop for this trade\" (p.5); the initial stop (entry-bar low/high) is the other exit"],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("lbrkc_close_outside", "Raschke Keltner close-outside entry", "2.5 x ADR(20), EMA 20, 3x3"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "EMA / average daily range length (book 20)", 20., 5., 100., 1.), p("k", "Band width in daily ranges (book 2.5)", 2.5, 0.5, 6., 0.25),
        p("shorts", "1 = trade both sides (p.5 'low (or high)'), 0 = long only", 1., 0., 1., 1.)])
}

fn ema0(v: &[f64], n: usize) -> Vec<f64> {
    let k = 2. / (n as f64 + 1.);
    let mut out = Vec::with_capacity(v.len());
    for (i, &x) in v.iter().enumerate() { out.push(if i == 0 { x } else { out[i - 1] + k * (x - out[i - 1]) }); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (n, k, shorts) = (g("n", 20.).round().max(2.) as usize, g("k", 2.5), g("shorts", 1.) >= 0.5);
    let len = x.c.len();
    let ema = ema0(&x.c, n);
    let rng: Vec<f64> = (0..len).map(|i| x.h[i] - x.l[i]).collect();
    let adr = sma(&rng, n);
    let s3 = sma(&x.c, 3);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out: Vec<Trade> = vec![];
    // open position: (direction, entry bar, entry-bar extreme)
    let mut cur: Option<(f64, usize, f64)> = None;
    for i in from.max(1)..=to {
        if let Some((d, e, ext)) = cur {
            let line = if i >= 6 { s3[i - 3] } else { f64::NAN };
            let stop = if line.is_finite() { if d > 0. { ext.max(line) } else { ext.min(line) } } else { ext };
            let hit = if d > 0. { l[i] <= stop || o[i] <= stop } else { h[i] >= stop || o[i] >= stop };
            if hit {
                let px = if d > 0. { o[i].min(stop) } else { o[i].max(stop) };
                out.push(Trade { dir: d, legs: vec![Leg { i: e, px: c[e], w: 1. }], exit_i: i, exit_px: px, reason: "stop (entry-bar extreme / 3x3 offset SMA)".into(),
                    entry_kind: "close", signal_i: i, open: false, exits: vec![] });
                cur = None;
            } else { continue; }
        }
        if !(adr[i].is_finite() && adr[i - 1].is_finite()) { continue; }
        let (up, dn) = (ema[i] + k * adr[i], ema[i] - k * adr[i]);
        let (up1, dn1) = (ema[i - 1] + k * adr[i - 1], ema[i - 1] - k * adr[i - 1]);
        if c[i] > up && c[i - 1] <= up1 { cur = Some((1., i, l[i])); }
        else if shorts && c[i] < dn && c[i - 1] >= dn1 { cur = Some((-1., i, h[i])); }
    }
    if let Some((d, e, _)) = cur {
        out.push(Trade { dir: d, legs: vec![Leg { i: e, px: c[e], w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn close_outside() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lbrkc_close_outside", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 20 && t.iter().any(|z| z.dir < 0.) && t.iter().all(|z| z.exit_i > z.legs[0].i || z.open));
        let mut p = Params::new(); p.insert("shorts".into(), 0.);
        assert!(run("lbrkc_close_outside", &x, 0, r.len() - 1, &p).unwrap().iter().all(|z| z.dir > 0.));
    }
}

// Instrument admission: Raschke Keltner workshop (TAG 1996) 2026-11-18 (lane CV2).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "lbrkc" && reviewed(def.id),
    label: "Raschke illustrates the channels on futures (coffee, S&P 500, crude oil); indices, ETFs and stocks, US and India, are a declared application (instrument swap).",
    basis: "Raschke, A Little Keltner, a Little Wyckoff, and a Lot of Street Smarts (TAG XVIII 1996), p.3-5; independent check authors/convert-1118/CV2/ref/lbrkc_ref.py",
};
