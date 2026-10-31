//! Yuri Shramenko (techedgecycles.com), 'Trade Execution' course text, 'Entry and Exit Examples' technique 3, Hi-Lo Bands
//! (research share Trading Courses,Seminars,Videos/Yuri Shramenko - Trade Execution (techedgecycles.com)/Yuri Shramenko -
//! Trade Execution.pdf, PDF p.19; the same text in his Trading Forecasts Manual, PDF p.13). Book id `ysthl`. Lane K9
//! (titles loop, 1 Nov 2026), to-code row 142; rule text authors/batch-c/C3w/shramenko.txt; independent check
//! authors/titles-1030/K9w/k9_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K9w/k9_ref.py (header)).
pub const REVIEWED: &[&str] = &["ysthl_hilo"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Subset] Shramenko uses these bands inside forecast time-zones from his AIR Market Trader software; that proprietary forecast-window filter is not available, so the bands rule runs on every day (lead decision 28). [Declared readings] High band = 8-day simple average of the highs, low band = 8-day simple average of the lows ('You never optimize the period of 8', so no period option). Long: a buy stop at yesterday's high-band value, placed only when yesterday's close was at or below it (a fresh penetration), filled at that level or at a higher open. The stop starts at yesterday's low-band value and trails it each day, never lowered. On the first day whose whole bar (its low) is above that day's high band, the trail switches to the high band from the next day, and from then on a close below the high band also exits at the close. Stops are tested from the day after entry, hit inside the day at the stop or a worse open. Shorts are the mirror ('using going long as an example'). One position at a time; no new entry on an exit day; a day that reaches both entry levels while flat is skipped (order inside the day unknown). Daily bars; indices, ETFs and US/India stocks are a declared application; no results printed. The other techniques in the section are not coded (trendlines and the Swing Line are drawn or given only in a chart image; the 3x3 DMA and short-term breakout start from a forecast pivot; the lower time frame is 30-minute bars).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "ysthl_hilo", name: "Hi-Lo Bands (Shramenko): enter on a penetration of the 8-day high/low band, trail the opposite band, then the near band",
        group: "Shramenko Trade Execution", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "ysthl", rules: &[
            "F|\"These are two simple moving averages, the first an 8-period of the High, the second an 8-period of the Low. You never optimize the period of 8.\"",
            "E|\"Using going long as an example for this entire paragraph, you enter when the market penetrates the High moving average\"",
            "X|\"the Low moving average immediately becomes your stop-loss. It also becomes your trailing-stop. To help lock in more profit, the first day that you have an entire bar above the High moving-average then you switch to using the High moving average as a trailing-stop. You exit the market when you have a Close below the High moving-average.\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "ysthl_hilo").then_some(("ysthl_hilo", "Shramenko Hi-Lo Bands", "8-day bands, no forecast filter"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (hb, lb) = (sma(h, 8), sma(l, 8));
    let mut out = vec![];
    // (dir, entry bar, entry price, stop, switched to the near band)
    let mut pos: Option<(f64, usize, f64, f64, bool)> = None;
    for i in from.max(1)..=to {
        if let Some((d, e, ep, mut st, mut sw)) = pos {
            if i > e {
                let band = if (d > 0.) != sw { lb[i - 1] } else { hb[i - 1] }; // long: low band, then high band; short mirrored
                if band.is_finite() { st = if d > 0. { st.max(band) } else { st.min(band) }; }
                if (d > 0. && l[i] <= st) || (d < 0. && h[i] >= st) {
                    out.push(mk(d, e, ep, "stop", i, if d > 0. { o[i].min(st) } else { o[i].max(st) }, "trailing stop", false, i)); pos = None; continue;
                }
                if sw && ((d > 0. && c[i] < hb[i]) || (d < 0. && c[i] > lb[i])) {
                    out.push(mk(d, e, ep, "stop", i, c[i], "close beyond the near band", false, i)); pos = None; continue;
                }
            }
            if !sw && ((d > 0. && l[i] > hb[i]) || (d < 0. && h[i] < lb[i])) { sw = true; }
            pos = Some((d, e, ep, st, sw));
            continue;
        }
        let (u, v) = (hb[i - 1], lb[i - 1]);
        if !(u.is_finite() && v.is_finite()) { continue; }
        let long = c[i - 1] <= u && h[i] >= u;
        let short = c[i - 1] >= v && l[i] <= v;
        if long == short { continue; }
        let d = if long { 1. } else { -1. };
        let ep = if long { o[i].max(u) } else { o[i].min(v) };
        let sw = (long && l[i] > hb[i]) || (short && h[i] < lb[i]);
        pos = Some((d, i, ep, if long { v } else { u }, sw));
    }
    if let Some((d, e, ep, _, _)) = pos { out.push(mk(d, e, ep, "stop", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn band_entries_and_trailing_exits() {
        let r: Vec<Row> = (0..600).map(|i| { let f = i as f64; let c = 100. + 10. * (f / 17.).sin() + 0.02 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.3, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ysthl_hilo", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        let (hb, lb) = (sma(&x.h, 8), sma(&x.l, 8));
        for z in &t {
            let e = z.legs[0].i;
            let lvl = if z.dir > 0. { hb[e - 1] } else { lb[e - 1] };
            assert_eq!(z.legs[0].px, if z.dir > 0. { x.o[e].max(lvl) } else { x.o[e].min(lvl) });
        }
        for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i); }
        assert!(run("ysthl_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Yuri Shramenko (Trade Execution) 2026-11-01 (titles lane K9).
// Admission only for the module REVIEWED lists (authors/titles-1030/K9w/k9_ref.py (header)).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "ysthl" && reviewed(def.id),
    label: "Yuri Shramenko trades the Hi-Lo Bands inside his software's forecast time-zones, without a published test; the bands rule without that filter on indices, ETFs and stocks, US and India, is a declared application.",
    basis: "Yuri Shramenko, Trade Execution (techedgecycles.com), PDF p.19 'Entry and Exit Examples', technique 3 Hi-Lo Bands",
};
