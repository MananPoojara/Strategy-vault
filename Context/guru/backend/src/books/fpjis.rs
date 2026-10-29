//! Franklin Paul Jackson, "Studies in Market Action and Speculative Technique: The I-S Method" (Spice Market Institute, 1980s,
//! scanned, charts to 1982; book id `fpjis`; PDF pages, printed page = PDF + 118). Reading:
//! claude-continuation-20261020/authors/fpjackson (admission/is_ref.py; OCR authors/overnight-1029/D1w/txt/jackson.txt).
//! Live: Study II's Key Pre MA-1 / Key MA-1 (PDF 88-95): the 10-week moving average plotted three weeks ahead, a
//! weekly stop one-fourth of its square root beyond it ('Pre': get out), and the reversal completed when the moving
//! average itself turns the same way; trading the Pre signals alone (PDF 91) as an option. Not coded: Key MA-1.2
//! (double bottom left to judgement, PDF 91), the SR, Thrust, Repeat and cancellation keys, Donchian's widened
//! penetration (PDF 97). Book: individual US stocks; indices, ETFs and India stocks here are a declared application.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use super::swsp::week_ends;
/// Admitted ids (authors/fpjackson/admission/is_ref.py (header)).
pub const REVIEWED: &[&str] = &["fpjis_key_ma1"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weeks are calendar weeks; a week's close is its last trading day. MA = simple 10-week average of weekly closes; the figure calculated at the end of week j is plotted three weeks later, so the level used during week w is MA(w-3) = L, with penetration P = square root of L / 4 (PDF 89). Each weekend a stop good for the next week only (PDF 89, 95): long, or flat after an up signal, sell stop at L - P; short, or flat after a down signal, buy stop at L + P; before the first signal both. Stops fill at the level, or at the open of the day that gaps through it (daily bars inside the week); one Pre per week, and a week whose day touches both levels before any signal gives none. A Pre always closes the open position (Get Out, PDF 89). Key MA-1 (book default): the new side is taken when the moving average turns the same way - 'the MA calculated this week drops below last week's calculation' (PDF 90, 93): if last week's figure already turned, the Pre stop itself reverses the position; otherwise the trade is entered at the close of the first later week whose figure turns that way while that Pre is still the latest one. The midweek inference of the turn (PDF 94) is not used. Option mode 1: buy and sell on the Pre signals alone (PDF 91), stop and reverse. The book's other keys (SR, Thrust, Repeat, double bottom) are not applied. Signals are tracked from the first week the vault holds; trades open only inside the test window; a position still open on the last bar is marked at its close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "fpjis_key_ma1", name: "I-S Key MA-1: 10-week average 3 weeks ahead, 1/4 square-root penetration, confirmed by the average's turn", group: "Jackson I-S Method",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "fpjis",
        rules: &["F|\"On a weekly chart showing each week's High-Low-Close, plot the ten-week moving average figure in the space three weeks later than the last price used in the calculation\" (PDF 88)",
            "X|\"In Uptrend, when Price drops this week below the MA line you plotted three weeks ago ... This penetration distance is one-fourth of the Square Root of the moving average figure ... you should exit - 'Get Out'\"; \"each Friday you should give your broker the Stop Order ... good for the week only\" (PDF 89)",
            "E|\"In an Uptrend we have a confirmed Key MA-1 Trend Reversal Signal when A Pre Key 1 is present and The Moving Average declines below last week's figure ... we may sell short\"; the mirror for a Downtrend (PDF 90)",
            "F|\"it is the moving average calculated this week which drops below last week's calculation that completes the signal\" (PDF 93)",
            "F|\"in most stocks over a sufficient period of time you will make money by selling and buying on the Pre signals alone\" (PDF 91): mode 1"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "fpjis_key_ma1" => Some(("fpjis_key_ma1", "Jackson I-S Key MA-1", "10-week MA +3, sqrt/4")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "fpjis_key_ma1" => Some(vec![p("mode", "Signals: 0 = Key MA-1, Pre confirmed by the average's turn (book), 1 = Pre signals alone", 0., 0., 1., 1.),
            p("weeks", "Moving average, weeks (book 10)", 10., 2., 52., 1.),
            p("ahead", "Average plotted this many weeks ahead (book 3)", 3., 0., 10., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (pre_only, n, ahead) = (g("mode").round() >= 1., g("weeks").round().max(1.) as usize, g("ahead").round().max(0.) as usize);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let we = week_ends(x);
    if we.is_empty() { return Some(vec![]); }
    let wc: Vec<f64> = we.iter().map(|&i| c[i]).collect();
    let ma = sma(&wc, n);
    let turn = |k: usize| -> f64 { if k == 0 { return 0.; } let d = ma[k] - ma[k - 1]; if d > 0. { 1. } else if d < 0. { -1. } else { 0. } };
    let mk = |d: f64, ei: usize, ep: f64, kind: &'static str, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }],
        exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur, mut pre) = (vec![], None::<(f64, usize, f64, &'static str)>, 0.);
    for w in 1..=we.len() {
        let start = we[w - 1] + 1;
        if start > to { break; }
        let end = if w < we.len() { we[w].min(to) } else { to };
        let lv = if w >= ahead { ma[w - ahead] } else { f64::NAN };
        if lv.is_finite() && lv > 0. {
            let pen = lv.sqrt() / 4.;
            let (up, dn) = (lv + pen, lv - pen);
            for i in start..=end {
                let (hs, hb) = (pre >= 0. && l[i] <= dn, pre <= 0. && h[i] >= up);
                if !(hs || hb) { continue; }
                if hs && hb { break; } // no signal yet and both levels touched in one day: order unknown
                let (d, f) = if hs { (-1., o[i].min(dn)) } else { (1., o[i].max(up)) };
                if let Some((cd, e, ep, k)) = cur.take() { out.push(mk(cd, e, ep, k, i, f, "Pre stop", false)); }
                pre = d;
                if (pre_only || turn(w - 1) == d) && i >= from && i < to { cur = Some((d, i, f, "stop")); }
                break;
            }
        }
        if w < we.len() && we[w] <= to && !pre_only && cur.is_none() && pre != 0. && turn(w) == pre {
            let i = we[w];
            if i >= from && i < to { cur = Some((pre, i, c[i], "close")); }
        }
    }
    if let Some((d, e, ep, k)) = cur { out.push(mk(d, e, ep, k, to, c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn civil(z: i64) -> (i64, i64, i64) { // days since 1970-01-01 -> (y, m, d)
        let z = z + 719468; let era = z.div_euclid(146097); let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153; let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
        (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
    }
    #[test]
    fn hand_checked_key_ma1() {
        // weekly flat bars (one bar per week, each a Friday): 12 weeks at 100, then 121 (week 12), 121 ...; MA(10)
        // first exists at week 9 = 100. Week 12: L = MA(9) = 100, P = 2.5: buy stop 102.5 hit -> Pre buy. MA(11) = 100
        // did not rise, so no entry on the stop; MA(12) = 102.1 rises -> long at week 12's close 121.
        let mut cl = vec![100.; 12];
        cl.extend([121., 121., 121., 121., 121., 121., 121., 121., 121., 121., 121., 121., 121., 60.]);
        let base = super::super::rgcts::days("2001-01-05"); // a Friday
        let r: Vec<Row> = cl.iter().enumerate().map(|(k, &c)| {
            let (y, m, d) = civil(base + 7 * k as i64);
            Row { ticker: "T".into(), date: format!("{y:04}-{m:02}-{d:02}"), open: c, high: c, low: c, close: c, volume: None }
        }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("fpjis_key_ma1", &x, 0, r.len() - 1, &Params::new()).unwrap();
        // week 25 (close 60): L = MA(22) = 121, P = 2.75, sell stop 118.25 gapped -> out at the open 60
        let got: Vec<_> = t.iter().map(|z| (z.dir, z.legs[0].i, z.legs[0].px, z.entry_kind, z.exit_i, z.exit_px)).collect();
        assert_eq!(got, vec![(1., 12, 121., "close", 25, 60.)]);
        assert!(run("fpjis_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}
