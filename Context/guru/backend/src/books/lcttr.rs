//! Charles L. Lindsay, "Trident: A Trading Strategy" (scanned book; research share USB HDD 2/tRADING KNO-2/Charles L.Lindsay -
//! Trident. A Trading Strategy/Lindsay, Charles - Trident A Trading Strategy.pdf; ocr2 text authors/batch-e/E6w/lindsay_trident.txt;
//! model printed p.40-63, trade criteria p.105-131). Book id `lcttr`. Minor-swing subset. Lane K10 (titles loop, 1 Nov 2026),
//! to-code row 178; independent check authors/titles-1030/K10w/k10_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K10w/k10_ref.py (header)).
pub const REVIEWED: &[&str] = &["lcttr_minor"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Subset] Minor swings only: the Intermediate and Major swing levels, the Subordinate Action Swing Rule (P2 = I3), inverted windows and inter-level target conflicts (p.114-130) are held. [Declared readings] A peak is a high above the previous and the next day's high, a valley a low below the previous and the next day's low (p.108), known at the next day's close; between two valleys with no peak the highest high between them is a phantom peak, and the mirror for valleys (p.110); a day that is both is taken in the order that keeps peaks and valleys alternating. Long Trident: P1 a valley, P2 the following peak, P3 the lowest low since P2 so far, which must be above P1 ('if buying, P3 must be greater than P1', p.126) and inside the P3 window between the Determinate Price C + .625 (B - A) and the Trend Reversal Price C + .625 (B - C), where A, B, C are the peak before P1, P1 and P2 (p.114-117: A 70.45, B 68.35, C 71.15 give 69.83 and 69.40). Entry: a buy stop at P3 + (P2 - P1)/4 (p.49), taken on a day that does not trade below P3 (P3 is assumed valid once the entry is reached, p.108); filled at the stop or at a higher open. Target T = P2 + P3 - P1 (p.42), exit there (or at a better open). Protective stop one tick below P3 (p.56); once the Critical Price (P3 + T)/2 is reached the stop moves to the entry price (p.58-59); once the price is one tick beyond P2 (the Window) the stop is replaced by a trailing stop |P3 - P2|/4 below the highest high since entry (p.59-61). Shorts are the mirror (P3 the highest high since the valley P2, below the peak P1). Levels reached on the entry day are not counted (the fill time inside the day is unknown); stops and target work from the next day; when one day touches the stop and the target, the stop is taken (conservative); stop fills at the stop or a worse open. Tick = the market's price step (0.01 US, 0.05 India). One position; no entry on an exit day. The book traded commodity futures and prints no test table (its 90% figure for well-qualified P3s is not shown as a claim); indices, ETFs and US/India stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "lcttr_minor", name: "Trident (Lindsay), minor swings: buy 25% up from a qualified P3, target P2 + P3 - P1, breakeven and trailing stops", group: "Trident: A Trading Strategy",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "lcttr", rules: &[
            "F|Peak-valley test (p.107-109) and phantom peaks/valleys (p.110-113); P3 window: \"DETERMINATE PRICE .625 (P2 - P1) + P3 ... TREND REVERSAL PRICE = .625 (P2 - P3) + P3 ... The new P3 can be expected to form within the price region lower than (or equal to) 69.83 (Determinate Price) and higher than (or equal to) 69.40 (Trend Reversal Price)\" (p.116-117)",
            "E|entry 25% of the way from P3 to the target (p.48-49); \"if buying, P3 must be greater than P1\" (p.126)",
            "X|target T = P2 + P3 - P1 (p.42); \"A TRIDENT TRADE ONCE EXECUTED MUST BE ABANDONED WHEN, AND IF, THE PRICE SWINGS ONE TICK (buying) BEYOND THE ESTABLISHED P3.\" (p.56); at the Critical Price \"Protective Stops should be moved to Entry Price\" (p.59); in the Window the STOP DIFFERENTIAL (P3 - P2)/4 \"IS APPLIED AS A TRAILING STOP, AND THE TARGET IS REACHED BEFORE BEING STOPPED OUT (with profit), EXIT (offset trade) AT THE TRIDENT TARGET\" (p.60-61)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "lcttr_minor").then_some(("lcttr_minor", "Trident (Lindsay)", "minor swings subset"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

/// Alternating swing list: (+1 peak / -1 valley, bar, price); phantoms fill two equal kinds in a row.
fn add(p: &mut Vec<(i8, usize, f64)>, k: i8, j: usize, px: f64, h: &[f64], l: &[f64]) {
    if let Some(&(lk, lj, _)) = p.last() {
        if lk == k && j > lj + 1 {
            let mut b = lj + 1;
            for q in lj + 1..j { if (k < 0 && h[q] > h[b]) || (k > 0 && l[q] < l[b]) { b = q; } }
            p.push((-k, b, if k < 0 { h[b] } else { l[b] }));
        }
    }
    p.push((k, j, px));
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let tk = x.market_tick();
    let mut pts: Vec<(i8, usize, f64)> = vec![];
    let mut out = vec![];
    // open trade: dir, entry bar, entry, P2, P3, target, critical price, stop differential, cp reached, window reached, best price
    let mut cur: Option<(f64, usize, f64, f64, f64, f64, f64, f64, bool, bool, f64)> = None;
    for t in 2..=to {
        let mut exited = false;
        if let Some((d, e, ep, p2, p3, tg, cp, sd, hit_cp, win, best)) = cur.as_mut() {
            if t > *e {
                let stop = if *win { *best - *d * *sd } else if *hit_cp { *ep } else { *p3 - *d * tk };
                let (hi, lo) = if *d > 0. { (h[t], l[t]) } else { (-l[t], -h[t]) };
                if lo <= *d * stop { out.push(mk(*d, *e, *ep, "stop", t, if *d > 0. { o[t].min(stop) } else { o[t].max(stop) }, "Trident stop", false, t)); exited = true; }
                else if hi >= *d * *tg { out.push(mk(*d, *e, *ep, "stop", t, if *d > 0. { o[t].max(*tg) } else { o[t].min(*tg) }, "Trident target", false, t)); exited = true; }
                else {
                    if hi >= *d * *cp { *hit_cp = true; }
                    if hi >= *d * (*p2 + *d * tk) { *win = true; }
                    *best = if *d > 0. { best.max(h[t]) } else { best.min(l[t]) };
                }
            }
        } else if t >= from {
            // setups from the swings known at yesterday's close
            let n = pts.len();
            for d in [1i8, -1] {
                if cur.is_some() || n < 4 { break; }
                // P2 = the latest point of kind d (peak for longs), P1 and A before it
                let k2 = if pts[n - 1].0 == d { n - 1 } else { n - 2 };
                if k2 < 2 { continue; }
                let (p2, p1, a) = (pts[k2], pts[k2 - 1], pts[k2 - 2]);
                if p2.1 + 1 > t - 1 { continue; }
                let df = d as f64;
                let p3 = if d > 0 { l[p2.1 + 1..t].iter().copied().fold(f64::INFINITY, f64::min) } else { h[p2.1 + 1..t].iter().copied().fold(f64::NEG_INFINITY, f64::max) };
                let (dp, trp) = (p2.2 + 0.625 * (p1.2 - a.2), p2.2 + 0.625 * (p1.2 - p2.2));
                let inside = p3 >= dp.min(trp) && p3 <= dp.max(trp);
                let untouched = if d > 0 { l[t] >= p3 } else { h[t] <= p3 };
                let entry = p3 + (p2.2 - p1.2) / 4.;
                let reach = if d > 0 { h[t] >= entry } else { l[t] <= entry };
                if df * (p3 - p1.2) > 0. && inside && untouched && reach {
                    let ep = if d > 0 { o[t].max(entry) } else { o[t].min(entry) };
                    let tg = p2.2 + p3 - p1.2;
                    cur = Some((df, t, ep, p2.2, p3, tg, (p3 + tg) / 2., (p3 - p2.2).abs() / 4., false, false, if d > 0 { f64::NEG_INFINITY } else { f64::INFINITY }));
                }
            }
        }
        if exited { cur = None; }
        // the close of t confirms bar t-1
        let j = t - 1;
        let pk = h[j] > h[j - 1] && h[j] > h[t];
        let vl = l[j] < l[j - 1] && l[j] < l[t];
        let peak_first = pts.last().map_or(true, |p| p.0 < 0);
        if pk && (peak_first || !vl) { add(&mut pts, 1, j, h[j], h, l); }
        if vl { add(&mut pts, -1, j, l[j], h, l); }
        if pk && !peak_first && vl { add(&mut pts, 1, j, h[j], h, l); }
    }
    if let Some((d, e, ep, ..)) = cur { out.push(mk(d, e, ep, "stop", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn swings_window_and_exits() {
        let r: Vec<Row> = (0..1500).map(|i| { let f = i as f64; let c = 100. + 0.03 * f + 6. * (f / 9.).sin() + 2.5 * (f / 3.3).cos();
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.3, high: c + 0.8, low: c - 0.8, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lcttr_minor", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3);
        for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i); }
        for z in &t { assert!(z.open || z.exit_i > z.legs[0].i); }
        let mut p = vec![];
        let (h, l) = (vec![1., 5., 2., 3., 1.5, 4.], vec![0., 3., 1., 2., 0.5, 3.]);
        add(&mut p, -1, 2, 1., &h, &l); add(&mut p, -1, 4, 0.5, &h, &l);
        assert_eq!(p, vec![(-1, 2, 1.), (1, 3, 3.), (-1, 4, 0.5)]);
        assert!(run("lcttr_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Charles L. Lindsay, Trident: A Trading Strategy 2026-11-01 (titles lane K10).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "lcttr" && reviewed(def.id),
    label: "Charles Lindsay traded the Trident on commodity futures and prints no test table; only the minor-swing trade is coded. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Charles L. Lindsay, Trident: A Trading Strategy, printed p.40-63 and 105-131 (scan, ocr2 text); independent check authors/titles-1030/K10w/k10_ref.py",
};
