//! Mark McRae, "Trading For Beginners" (Traders Secret Code course, traderssecretcode.com; research share Trading Knowledge III/
//! Mark McRae - Traders Secret Code Complete Course (traderssecretcode.com)/36- TFBV1.pdf, p.92). Book id `mcrtf`. One rule: The
//! Trend Following Method (9/18-period simple averages, pullback to the 9, close back beyond it). Lane K8 (titles loop, 1 Nov 2026);
//! independent check authors/titles-1030/K8w/k8_ref.py (header). The Breakout and Reversal methods on the same pages are held
//! (hand-drawn levels).
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["mcrtf_trend"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// Price-vs-average and average-vs-average tests use a 1e-9 allowance: on cent prices a 9-bar average can equal a high or a
/// close exactly, and rounding in the average must not decide a touch, a close beyond it or a crossing (equal = not beyond).
const E: f64 = 1e-9;

const NOTE: &str = "[Declared readings] Daily bars (the author: a period 'can be anything that you want it to be'). The set-up is the crossing of the 9- and 18-period simple averages of the close; an up-cross sets up a long, a down-cross a short, and a new crossing replaces the set-up (one trade per crossing). 'Return back to the 9' = a later bar whose low (for a short: high) reaches the 9-period average; from that bar on, the first close back above the 9 (short: below) is the signal, filled at that close. Stop 'below the most recent low' = the lowest low from the crossing bar to the entry bar (short: highest high), hit inside the day (fill at the stop, or the open on a gap), tested from the day after entry. Exit at the close when the close goes back below the 18-period average (short: above) ('The exit could be when we have a closing above the 18 period moving average'). Long and short. A course example with no printed results; ETFs, US/India indices and stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mcrtf_trend", name: "Trend following 9/18 SMA: after the crossing, a pullback to the 9 and a close back beyond it; out on a close beyond the 18", group: "McRae Trading For Beginners",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "mcrtf", rules: &[
            "F|\"When the 9 period moving average crosses the 18 period moving average this is the set up\" (p.92)",
            "E|\"I like to see the price first return back to the 9 period moving average ... If the price then closes back below the 9 period moving average then a sell signal is generated. Obviously it would need to close back above the 9 period moving average in an up trend\"",
            "X|\"My stop loss would be placed above the most recent high in a down trend and below the most recent low in an up trend. The exit could be when we have a closing above the 18 period moving average\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "mcrtf_trend").then_some(("mcrtf_trend", "McRae 9/18 trend following", "9/18 SMA"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "mcrtf_trend").then(|| vec![p("fast", "Fast average (9)", 9., 2., 50., 1.), p("slow", "Slow average (18)", 18., 3., 100., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def)).round().max(1.) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (s9, s18) = (sma(c, g("fast")), sma(c, g("slow")));
    let (mut out, mut pos) = (vec![], None::<(f64, usize, f64, f64)>);
    // set-up: (dir, crossing bar, touched, traded)
    let mut setup: Option<(f64, usize, bool, bool)> = None;
    for i in from..=to {
        if let Some((d, e, ep, st)) = pos {
            if i > e {
                let px = if d > 0. && l[i] < st { Some(o[i].min(st)) } else if d < 0. && h[i] > st { Some(o[i].max(st)) }
                    else if (d > 0. && c[i] < s18[i] - E) || (d < 0. && c[i] > s18[i] + E) { Some(c[i]) } else { None };
                if let Some(px) = px { out.push(mk(d, e, ep, "close", i, px, "stop or close beyond the slow average", false, i)); pos = None; }
            }
        }
        if i >= 1 && s9[i] > s18[i] + E && s9[i - 1] <= s18[i - 1] + E { setup = Some((1., i, false, false)); }
        else if i >= 1 && s9[i] < s18[i] - E && s9[i - 1] >= s18[i - 1] - E { setup = Some((-1., i, false, false)); }
        else if let (None, Some((d, c0, mut touched, false))) = (pos, setup) {
            if i < to {
                if !touched { touched = if d > 0. { l[i] <= s9[i] + E } else { h[i] >= s9[i] - E }; }
                let go = touched && ((d > 0. && c[i] > s9[i] + E) || (d < 0. && c[i] < s9[i] - E));
                if go {
                    let st = if d > 0. { l[c0..=i].iter().copied().fold(f64::INFINITY, f64::min) } else { h[c0..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) };
                    pos = Some((d, i, c[i], st));
                }
                setup = Some((d, c0, touched, go));
            }
        }
    }
    if let Some((d, e, ep, _)) = pos { out.push(mk(d, e, ep, "close", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn pullback_entry_after_the_cross() {
        let mut c: Vec<f64> = vec![100.; 20];
        c.extend([101., 102., 103., 104., 105., 104.5, 106., 107., 90.]);
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: v, high: v + 0.5, low: v - 0.5, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p: Params = [("fast".to_string(), 3.), ("slow".to_string(), 6.)].into_iter().collect();
        let t = run("mcrtf_trend", &x, 0, r.len() - 1, &p).unwrap();
        assert!(t.iter().any(|z| z.dir == 1.));
    }
}

// Instrument admission: Mark McRae (Trading For Beginners) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "mcrtf" && crate::books::mcrtf::reviewed(def.id),
    label: "A course example for any market and period with no results; ETFs, US/India indices and stocks are a declared application.",
    basis: "Mark McRae, Trading For Beginners (Traders Secret Code course) p.92 'The Trend Following Method'; independent check authors/titles-1030/K8w/k8_ref.py",
};
