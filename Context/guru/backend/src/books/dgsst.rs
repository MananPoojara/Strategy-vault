//! David Graeme-Smith ("David Smith"), "Short Swing Trading" v6.0 (e-book, 2004; book id `dgsst`; the e-book's own page
//! numbers "Page N of 121" and PDF pages cited). Reading: claude-continuation-20261020/authors/dsmith (admission/sst_ref.py).
//! Live: the six-rule SST strategy (Ch.7-9, PDF 46-70): trade with the 50-day simple average, +DI/-DI cross of an 8-period
//! Directional Movement indicator, buy above the last significant (non-inside) day's high (short: below its low), stop 0.5%
//! under that day's low, trailed under each new significant day's low (50% of a 5% "big move day"). Rule 1 (risk at most 5%
//! of capital per trade) is position sizing and is not applied. Book: UK shares; indices, ETFs and US/India stocks are a
//! declared application (the author warns that indices and other shares may need more stop "space").
use crate::connors::{true_range, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/dsmith/admission/sst_ref.py (header)).
pub const REVIEWED: &[&str] = &["dgsst_sst"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] 50 ma = simple 50-day average of closes; Directional Movement = Wilder's +DI/-DI over 8 days (Wilder smoothing). A 'cross' = +DI at or above -DI after being below it the day before (a touch counts as a cross; short: -DI at or above +DI after being below it). A cross arms one trade in its direction; it stays armed while that line stays at or above the other and is used up by the entry. Inside day = high not above and low not below the previous day's; the 'last significant day' = the latest day that is not an inside day. Entry: a stop order for the next day at the last significant day's high (long; short: its low), filled when the price goes above it, at that high or at a higher open; the day before must close above the 50 ma (short: below) with the setup armed. Space = 0.5% of the reference price: stop = low x 0.995 (short: high x 1.005). The initial stop (last significant day before entry) is live from the day after entry. After each day in the trade (the entry day counts) that is not an inside day the stop moves to that day's low x 0.995 if higher (short: high x 1.005 if lower); a 'big move day' (high - low at least 5% of the previous close) uses its midpoint instead. Stops fill at the level or a worse open. One position at a time; a new entry can come from the day after an exit. Not applied: the risk-per-trade size (Rule 1), the 200-day average caution, share 'personality' selection, intraday entries on the day of the cross.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dgsst_sst", name: "Short Swing Trading: 50-day trend + 8-day DMI cross, buy above the last significant day, trailing stop", group: "The SST Strategy (Ch.7-9)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "dgsst",
        rules: &["F|\"RULE 2 - Always trade with the trend (determined by price above or below the 50 ma).\" \"The SST Strategy uses the most straightforward of these variations called the 'SIMPLE' moving average.\" (p.43-44, PDF 49-50)",
            "E|\"RULE 3 - Look for +DI crossing or touching -DI on the Directional Movement indicator\"; \"Although the default setting for Directional Movement is usually 14 period (14 day) we adjust this to 8 period (8 day)\" (p.45-49, PDF 51-55)",
            "E|\"RULE 4 - Enter after price confirmation (price must go above the last significant high for a long position or below the last significant low for a short position).\" \"Ignore inside days - they are insignificant. An inside day is where neither the price high nor the price low exceeds the previous day.\" (p.53-57, PDF 59-63)",
            "X|\"RULE 5 - Initial stop - Place the initial stop loss just below (if long) or just above (if short) the previous significant low (if long) or high (if short).\" \"Allow additional 'space' of 0.5% of the share price.\" (p.58-61, PDF 64-67)",
            "X|\"RULE 6 - Trailing stop & exit - When a new significant day is made, move the stop to just below that day's low if long or just above that day's high if short. Use 50% of a 'big move (5%) day'.\" \"The trailing stop will do it for us\" (p.62-64, PDF 68-70)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "dgsst_sst" => Some(("dgsst_sst", "Graeme-Smith Short Swing Trading", "SST six rules")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "dgsst_sst" => Some(vec![p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.),
            p("space", "Stop space, % of price (book 0.5)", 0.5, 0., 10., 0.1),
            p("dmi", "Directional Movement period (book 8)", 8., 2., 50., 1.)]),
        _ => None,
    }
}

/// +DI and -DI (Wilder, n days).
fn di(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>) {
    let len = x.r.len();
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    let trs = true_range(x.r);
    for i in 1..len {
        let (up, dn) = (x.h[i] - x.h[i - 1], x.l[i - 1] - x.l[i]);
        pdm[i] = if up > dn && up > 0. { up } else { 0. };
        mdm[i] = if dn > up && dn > 0. { dn } else { 0. };
        tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&tr, n));
    let mut p = vec![f64::NAN; len];
    let mut m = vec![f64::NAN; len];
    for i in 0..len { if st[i] > 0. { p[i] = 100. * sp[i] / st[i]; m[i] = 100. * sm[i] / st[i]; } }
    (p, m)
}

fn run_sst(x: &Ctx, from: usize, to: usize, side: u8, space: f64, n: usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    // plain 50-day sum each day (no running sum), so a close exactly on the average reads as 'not above'
    let ma: Vec<f64> = (0..c.len()).map(|i| if i >= 49 { c[i - 49..=i].iter().sum::<f64>() / 50. } else { f64::NAN }).collect();
    let (pdi, mdi) = di(x, n);
    let inside = |i: usize| h[i] <= h[i - 1] && l[i] >= l[i - 1];
    let (lo_k, hi_k) = (1. - space / 100., 1. + space / 100.);
    // stop level from a significant day: the low (short: high), or the midpoint of a big move day
    let level = |i: usize, d: f64| if h[i] - l[i] >= 0.05 * c[i - 1] { (h[i] + l[i]) / 2. } else if d > 0. { l[i] } else { h[i] };
    let mk = |d: f64, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "stop", signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64, f64)>); // dir, entry bar, entry price, stop
    let (mut arm_l, mut arm_s) = (false, false);
    let mut sig = 0usize; // last significant day up to the previous bar
    let start = from.max(51);
    for j in (1..start).rev() { if !inside(j) { sig = j; break; } }
    for i in start..=to {
        let (ok_l, ok_s) = (side != 2, side != 1);
        let mut entered = false;
        if cur.is_none() && i > start && sig > 0 {
            let p = i - 1;
            if ok_l && arm_l && c[p] > ma[p] && (o[i] > h[sig] || h[i] > h[sig]) {
                let px = if o[i] > h[sig] { o[i] } else { h[sig] };
                cur = Some((1., i, px, l[sig] * lo_k)); arm_l = false; entered = true;
            } else if ok_s && arm_s && c[p] < ma[p] && (o[i] < l[sig] || l[i] < l[sig]) {
                let px = if o[i] < l[sig] { o[i] } else { l[sig] };
                cur = Some((-1., i, px, h[sig] * hi_k)); arm_s = false; entered = true;
            }
        }
        if let Some((d, ei, ep, mut s)) = cur {
            if !entered {
                if (d > 0. && o[i] <= s) || (d < 0. && o[i] >= s) { out.push(mk(d, ei, ep, i, o[i], "stop", false)); cur = None; }
                else if (d > 0. && l[i] <= s) || (d < 0. && h[i] >= s) { out.push(mk(d, ei, ep, i, s, "stop", false)); cur = None; }
            }
            if cur.is_some() && !inside(i) {
                s = if d > 0. { s.max(level(i, d) * lo_k) } else { s.min(level(i, d) * hi_k) };
                cur = Some((d, ei, ep, s));
            }
        }
        // end of day i: DI arming and the last significant day
        if pdi[i] >= mdi[i] && pdi[i - 1] < mdi[i - 1] { arm_l = true; }
        if pdi[i] < mdi[i] { arm_l = false; }
        if mdi[i] >= pdi[i] && mdi[i - 1] < pdi[i - 1] { arm_s = true; }
        if mdi[i] < pdi[i] { arm_s = false; }
        if !inside(i) { sig = i; }
    }
    if let Some((d, ei, ep, _)) = cur { out.push(mk(d, ei, ep, to, c[to], "open", true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    Some(match id {
        "dgsst_sst" => run_sst(x, from, to, g("side").clamp(0., 2.) as u8, g("space").max(0.), g("dmi").max(2.) as usize),
        _ => return None,
    })
}

// Instrument admission: David Graeme-Smith 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/dsmith/admission/sst_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="dgsst" && crate::books::dgsst::reviewed(def.id),
    label: "David Graeme-Smith wrote Short Swing Trading for UK shares (FTSE 250 type); runs on indices, ETFs and US/India stocks are a declared application, not the author's test, and he warns indices may need more stop space. Cash-index shorts are hypothetical.",
    basis: "David Graeme-Smith, Short Swing Trading v6.0 (e-book, 2004), Ch.7-9 the six SST rules (PDF 46-70)",
};
