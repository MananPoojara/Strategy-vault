//! David Bowden, "The Number One Trading Plan" (Safety in the Market, sitm.com.au; scanned course manual, book id `dbnotp`;
//! printed pages, PDF = printed + 6/7). Reading: lane B 29 Oct (authors/bowden/agents/notp) and day lane A1 30 Oct (OCR
//! authors/bowden/text/notp.txt; admission/abc_ref.py). Live: the ABC swing trade in its three exit styles (Stock,
//! Currency, Stock Index; p.46-49) and the long-only trailing swing stop (p.73) on the 1-day swing chart (p.62-65).
//! Not coded: adding on the break of B, the weekly 'in sympathy' filter (not a condition in the book's exercise, p.79),
//! the advanced filters/entries (judgement or intraday, p.93-112), 2- and 3-day swing charts (p.76-80, option later).
//! Book: stocks (MSFT, YHOO, BHP), stock index futures and currencies; indices, ETFs and US/India stocks run here.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/bowden/admission/abc_ref.py (header)).
pub const REVIEWED: &[&str] = &["dbnotp_abc_stock", "dbnotp_abc_currency", "dbnotp_abc_index", "dbnotp_abc_trail"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Swing chart: the book defers the 1-day chart to its Smarter Starter Pack (not in the share); here the multi-day rules of p.65 with one day: each bar is compared with the bar before it; a higher high is a day up, a lower low a day down, an inside day counts as neither; an outside day counts twice, up first when it opened above its close, otherwise down first (p.64). One day against the swing turns it; the swing's extreme becomes a swing top or bottom. Long setup (p.46): the last swing top B is above the swing top before it, and the current or just-confirmed swing bottom C is above the swing bottom A that started the A-B move (higher top and higher bottom = uptrend). Entry: buy stop one tick above the high of the bar that made C (p.76), filled at the stop or the open if it gaps above, and only at or below C + 25% of the A-B range (33% for the Currency style and the trailing swing stop); a fill above that limit cancels the setup, and each C is traded at most once. One tick = 0.01 on US prices, 0.05 on NSE prices; commission cover = 3 ticks ('3 cents, or points', p.46). Initial stop one tick below C (p.46). Stock style: at 50% of A-B from C the stop moves to entry + commission; profit at 75% (p.46-47). Currency style: at 50% entry + commission, at 75% the stop goes 30 ticks below the 50% level (text p.48; the matrix's 1/5 average daily range is an option), profit at 100%. Stock Index style: at 50% and each later 25% milestone the stop goes 50 ticks below that milestone (text p.49; the matrix's 1/3 average daily range is an option), no target. Trailing swing stop (long only, p.73): at 50% entry + commission, then each new swing bottom confirmed after entry raises the stop to one tick below it from the next bar. Stops never move down. Milestones, stops and targets are watched intraday: on a bar that touches both sides the path is open-high-low-close when it opened above its close, else open-low-high-close (the book's outside-day order); a long entry bar that first trades below C is no entry (C moves). Shorts mirror everything (p.47). Average daily range for the matrix options = mean high-low of the 10 days before the bar [not from the book]. One position at a time; trades open inside the test window; a position still open at the end is marked at the last close.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str]| StrategyDef { id, name, group: "Bowden ABC swing trades", session: 0,
        dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "dbnotp", rules, notes: NOTE, claim: None };
    const SETUP: &str = "E|\"Point A is the start of the immediately preceding up move. Point B is the termination of the first up move and is identified by a swing down. Point C is the termination of the counter-trend, corrective move and is identified by a swing up. Point B must be above the previous swing top.\" (p.46); \"For a 1-day swing chart this is the high of Point C plus 1 in a long trade, and the low of C minus 1 in a short trade.\" (p.76)";
    const SWING: &str = "F|\"Ignore inside days ... Compare each bar to the previous bar / Outside days count twice. The order is based on the open and the close of the day. The rules for a two-day (and all other multi-day) swing charts are the same\" (p.65); \"Counting the day up first and down second implies that the market opened higher than it closed.\" (p.64)";
    vec![
        d("dbnotp_abc_stock", "ABC Stock Style: enter within 25% of A-B above C, stop to entry at 50%, profit at 75%", "Long/Short", &[SWING, SETUP,
            "E|\"the long trade must be entered at, or before, the market has traveled up a distance equal to 25% of the A-B range, from Point C\" (p.46)",
            "X|\"Upon entering an initial trade, a stop-loss is placed one point from Point C ... Upon the market reaching 50% of the A-B range, a stop is placed at the point of entry, plus commission ... Use 3 cents, or points, as the basis for covering commission.\" (p.46); \"Profits are taken at 75% of the A-B range, or the first time a progressive stop is triggered.\" (p.47)"]),
        d("dbnotp_abc_currency", "ABC Currency Style: 33% entry limit, stop to entry at 50%, 30 ticks under the 50% level at 75%, profit at 100%", "Long/Short", &[SWING, SETUP,
            "E|\"the long trade must be entered at, or before, the market has traveled up a distance equal to 33% of the A-B range, from Point C\" (p.47)",
            "X|\"when the market reaches the 75% milestone you will raise the stops ... to 30 points (ticks) below the 50% level. Profits are taken at 100% of the A-B range, or the first time the progressive stop is triggered.\" (p.48); matrix: \"At 75%: 1/5 of Average Daily Range below 50%\" (p.49, option)"]),
        d("dbnotp_abc_index", "ABC Stock Index Style: stop 50 ticks under each 25% milestone from 50%, out only on the stop", "Long/Short", &[SWING, SETUP,
            "E|\"the long trade must be entered at, or before, the market has traveled up a distance equal to 25% of the A-B range, from Point C\" (p.48)",
            "X|\"Upon the market reaching 50% of the A-B range, a stop is placed 50 points below the 50% level ... each time it passes through the next 25% barrier, the stop is raised to 50 points (or ticks) below that barrier ... Profits are taken the first time the progressive stop is triggered.\" (p.49); matrix: \"1/3 of Average Daily Range below\" each milestone (option)"]),
        d("dbnotp_abc_trail", "ABC with a trailing swing stop (long only): 33% entry limit, entry at 50%, then one tick under each new swing bottom", "Long", &[SWING, SETUP,
            "E|\"Only trade long in this exercise ... the long trade must be entered at, or before, the market has traveled up a distance equal to 33% of the A-B range, from Point C.\" (p.73)",
            "X|\"Once the market trades to the 50% milestone, move your stop to entry plus commission. Upon the market making its first higher swing bottom, move your stop $0.01 below the swing bottom. Continue this process with each new swing bottom until you are stopped out. Do not enter any additional positions or ABC trades while in an existing trade.\" (p.73)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("dbnotp_abc", "Bowden ABC swing trade", match id {
        "dbnotp_abc_stock" => "Stock Style", "dbnotp_abc_currency" => "Currency Style", "dbnotp_abc_index" => "Stock Index Style", _ => "Trailing swing stop" }))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let side = p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.);
    let adr = p("adr", "Trailing distance: 0 = ticks from the text (book), 1 = the matrix's share of the 10-day average daily range", 0., 0., 1., 1.);
    match id {
        "dbnotp_abc_stock" => Some(vec![side]),
        "dbnotp_abc_currency" | "dbnotp_abc_index" => Some(vec![side, adr]),
        "dbnotp_abc_trail" => Some(vec![]),
        _ => None,
    }
}

/// One confirmed swing point: (+1 top / -1 bottom, price, bar that made it).
type Pt = (i8, f64, usize);
struct Pos { d: f64, ei: usize, ep: f64, kind: &'static str, c: f64, r: f64, stop: f64, tgt: f64, nxt: f64 }

/// Favourable excursion fx: milestone stop moves (Stock/Trail: 50%; Currency: 50%, 75%; Index: every 25% from 50%),
/// then the target. off = the trailing distance for the Currency / Index milestones.
fn fav(ps: &mut Pos, fx: f64, style: u8, comm: f64, off: f64) -> Option<(f64, &'static str)> {
    let (d, cc, r) = (ps.d, ps.c, ps.r);
    let lvl = |m: f64| cc + d * m * r;
    while d * (fx - lvl(ps.nxt)) >= 0. {
        let m = ps.nxt;
        let ns = if style == 2 { lvl(m) - d * off } else if m == 0.5 { ps.ep + d * comm } else { lvl(0.5) - d * off };
        ps.stop = if d > 0. { ps.stop.max(ns) } else { ps.stop.min(ns) };
        ps.nxt = if style == 2 || (style == 1 && m == 0.5) { m + 0.25 } else { f64::INFINITY };
    }
    (ps.tgt.is_finite() && d * (fx - ps.tgt) >= 0.).then_some((ps.tgt, "Target"))
}

/// One bar for an open position (no open-gap checks): open-high-low-close if the bar opened above its close, else
/// open-low-high-close; skip_first drops an adverse side that came before a stop entry.
fn bar(ps: &mut Pos, h: f64, l: f64, hf: bool, skip_first: bool, style: u8, comm: f64, off: f64) -> Option<(f64, &'static str)> {
    let d = ps.d;
    let (fx, ax) = if d > 0. { (h, l) } else { (l, h) };
    let fav_first = if d > 0. { hf } else { !hf };
    if !fav_first && !skip_first && d * (ax - ps.stop) <= 0. { return Some((ps.stop, "Stop")); }
    if let Some(x) = fav(ps, fx, style, comm, off) { return Some(x); }
    if fav_first && d * (ax - ps.stop) <= 0. { return Some((ps.stop, "Stop")); }
    None
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let style: u8 = match id { "dbnotp_abc_stock" => 0, "dbnotp_abc_currency" => 1, "dbnotp_abc_index" => 2, _ => 3 };
    let side = if style == 3 { 1 } else { g("side").round() as i64 };
    let use_adr = g("adr").round() >= 1.;
    let lim = if style == 1 || style == 3 { 0.33 } else { 0.25 };
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let tick = x.market_tick();
    let comm = 3. * tick;
    let off = |t: usize| -> f64 {
        if !use_adr { return if style == 1 { 30. * tick } else { 50. * tick }; }
        if t < 10 { return f64::NAN; }
        (t - 10..t).map(|k| h[k] - l[k]).sum::<f64>() / 10. / if style == 1 { 5. } else { 3. }
    };
    let (mut dir, mut ext, mut extb) = (0i8, f64::NAN, 0usize);
    let mut pts: Vec<Pt> = vec![];
    let mut used: Vec<usize> = vec![]; // C bars already touched (traded or cancelled)
    let mut pos: Option<Pos> = None;
    let mut out = vec![];
    let fin = |ps: &Pos, i: usize, px: f64, why: &str, open: bool| Trade { dir: ps.d, legs: vec![Leg { i: ps.ei, px: ps.ep, w: 1. }], exit_i: i, exit_px: px,
        reason: why.into(), entry_kind: ps.kind, signal_i: i, open, exits: vec![] };
    for t in 1..=to {
        let hf = o[t] > c[t];
        let flat = pos.is_none();
        // open position from an earlier bar: gaps, then the bar's path
        if let Some(ps) = pos.as_mut() {
            let d = ps.d;
            let done = if d * (o[t] - ps.stop) <= 0. { Some((o[t], "Stop")) }
                else if ps.tgt.is_finite() && d * (o[t] - ps.tgt) >= 0. { Some((o[t], "Target")) }
                else { bar(ps, h[t], l[t], hf, false, style, comm, off(t)) };
            if let Some((px, why)) = done { out.push(fin(ps, t, px, why, false)); pos = None; }
        }
        // setups as of the close of t-1 (long first): B above the top before A, C beyond A, C bar not used
        let n = pts.len();
        for d in [1i8, -1] {
            let (cc, cb, k) = if dir == -d { (ext, extb, n) } else if dir == d && n >= 1 && pts[n - 1].0 == -d { (pts[n - 1].1, pts[n - 1].2, n - 1) } else { continue };
            if k < 3 || used.contains(&cb) { continue; }
            let (b, a, pb) = (pts[k - 1], pts[k - 2], pts[k - 3]);
            let df = d as f64;
            if !(b.0 == d && a.0 == -d && pb.0 == d && df * (b.1 - pb.1) > 0. && df * (cc - a.1) > 0.) { continue; }
            let r = (b.1 - a.1).abs();
            if r <= 0. { continue; }
            let stp = if d > 0 { h[cb] + tick } else { l[cb] - tick };
            if df * ((if d > 0 { h[t] } else { l[t] }) - stp) < 0. { continue; }
            used.push(cb);
            let gap = df * (o[t] - stp) > 0.;
            let fill = if gap { o[t] } else { stp };
            if !flat || pos.is_some() || t < from || t >= to || df * (fill - (cc + df * lim * r)) > 0. || (d > 0 && side == 2) || (d < 0 && side == 1) { continue; }
            let fav_first = if d > 0 { hf } else { !hf };
            if !gap && !fav_first && df * ((if d > 0 { l[t] } else { h[t] }) - cc) < 0. { continue; } // C broken before the entry: C moves
            let tgt = match style { 0 => cc + df * 0.75 * r, 1 => cc + df * r, _ => f64::NAN };
            let mut ps = Pos { d: df, ei: t, ep: fill, kind: if gap { "open" } else { "stop" }, c: cc, r, stop: cc - df * tick, tgt, nxt: 0.5 };
            match bar(&mut ps, h[t], l[t], hf, !gap && !fav_first, style, comm, off(t)) {
                Some((px, why)) => out.push(fin(&ps, t, px, why, false)),
                None => pos = Some(ps),
            }
        }
        // swing chart update with bar t
        let (up, dn) = (h[t] > h[t - 1], l[t] < l[t - 1]);
        let ev: &[i8] = match (up, dn) { (true, true) => if hf { &[1, -1] } else { &[-1, 1] }, (true, false) => &[1], (false, true) => &[-1], _ => &[] };
        for &e in ev {
            let px = if e > 0 { h[t] } else { l[t] };
            if dir == e { if (e as f64) * (px - ext) > 0. { ext = px; extb = t; } continue; }
            if dir != 0 {
                pts.push((dir, ext, extb));
                if let Some(ps) = pos.as_mut().filter(|ps| style == 3 && dir < 0 && extb > ps.ei) { ps.stop = ps.stop.max(ext - tick); }
            }
            dir = e; ext = px; extb = t;
        }
    }
    if let Some(ps) = pos { out.push(fin(&ps, to, c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trail_reproduces_the_book_yhoo_trades() {
        // YHOO 1 Mar - 8 Jul 2004 (Appendix A, p.85-87); book p.75: in 22.49 (25 Mar) out 26.66 (21 Apr), in 28.34 (21 May) out 31.94 (21 Jun)
        let v: [(f64, f64, f64, f64); 90] = [
            (22.26,22.44,21.95,22.04), (22.06,22.30,21.49,21.50), (21.42,21.78,21.09,21.68), (21.73,22.10,21.67,22.07), (21.80,22.39,21.74,22.20), (22.17,22.72,21.85,21.92),
            (21.90,22.14,21.45,21.67), (21.68,21.88,20.75,20.85), (20.65,21.28,20.57,20.83), (20.95,21.63,20.86,21.51), (21.58,21.68,20.81,20.88), (21.05,21.49,21.00,21.28),
            (22.09,22.52,22.02,22.42), (22.40,22.65,22.18,22.52), (22.44,23.34,22.42,22.88), (22.47,22.58,21.88,22.24), (22.38,22.42,21.92,22.04), (22.14,22.28,21.75,22.25),
            (22.49,23.50,22.48,23.47), (23.42,23.76,23.36,23.57), (23.59,23.95,23.56,23.84), (23.80,24.43,23.77,24.40), (24.39,24.65,24.17,24.24), (24.38,24.85,24.21,24.73),
            (25.27,25.32,24.63,25.08), (25.02,25.50,24.65,25.00), (24.56,24.65,24.11,24.39), (24.23,24.63,23.94,24.17), (27.90,28.12,27.36,28.10), (27.88,27.99,27.43,27.57),
            (27.48,27.54,26.92,27.07), (26.84,27.48,26.72,27.34), (27.41,27.43,26.88,26.95), (26.84,27.39,26.67,27.07), (26.94,27.92,26.88,27.84), (27.93,27.98,26.75,26.77),
            (27.09,27.38,26.55,27.29), (27.25,28.99,27.25,28.80), (28.70,28.75,27.95,28.38), (28.29,29.17,27.98,28.50), (28.49,29.13,28.28,28.77), (28.86,29.16,27.88,27.92),
            (27.90,28.20,26.77,27.35), (27.20,27.30,25.01,25.26), (25.27,26.20,25.25,26.15), (26.17,26.93,26.13,26.42), (26.49,26.96,26.45,26.58), (26.56,26.60,25.82,26.18),
            (26.24,26.87,26.14,26.40), (26.13,26.20,25.33,25.67), (26.17,27.00,26.09,26.76), (26.66,27.18,25.76,27.08), (27.18,28.10,26.78,27.10), (27.54,27.67,26.75,26.97),
            (26.35,27.66,26.21,27.02), (27.63,27.98,27.31,27.77), (28.61,28.90,27.82,27.95), (28.14,28.30,27.51,28.03), (28.34,28.85,28.15,28.55), (28.87,29.76,28.84,29.43),
            (28.95,30.50,28.88,30.28), (29.82,30.40,29.77,30.11), (30.36,30.80,30.00,30.56), (30.49,31.16,30.30,30.66), (30.56,32.82,30.43,32.48), (32.50,32.84,31.49,31.55),
            (31.70,31.81,31.06,31.19), (31.86,32.20,31.45,31.87), (32.39,32.52,32.03,32.51), (32.42,33.00,32.28,32.99), (33.01,33.01,32.08,32.32), (32.50,32.74,31.86,32.40),
            (32.09,32.24,31.44,31.65), (32.10,32.58,31.97,32.10), (32.41,32.77,32.07,32.47), (32.39,32.53,31.96,32.38), (32.06,32.87,31.95,32.07), (32.16,32.38,31.56,31.67),
            (31.95,32.55,31.77,32.54), (32.54,34.19,32.52,33.97), (33.80,34.38,33.64,34.11), (34.25,35.36,33.50,34.91), (35.24,36.27,34.98,35.48), (35.39,35.77,35.08,35.35),
            (35.99,36.51,35.70,36.40), (35.15,35.34,34.11,34.30), (34.50,34.54,33.57,33.94), (33.98,34.00,32.30,33.22), (33.14,33.14,32.30,32.60), (29.29,31.24,28.99,30.08)];
        let r: Vec<crate::Row> = v.iter().enumerate().map(|(i, &(o, h, l, c))| crate::Row { ticker: "YHOO".into(), date: format!("2004-{:03}", i), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("dbnotp_abc_trail", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|z| (z.legs[0].i, (z.legs[0].px * 100.).round(), z.exit_i, (z.exit_px * 100.).round())).collect();
        assert_eq!(got, vec![(18, 2249., 36, 2666.), (58, 2834., 77, 3194.)]);
    }
    #[test]
    fn milestones_follow_each_style() {
        let mk = |d: f64| Pos { d, ei: 0, ep: 10.5, kind: "stop", c: 10., r: 4., stop: 9.99, tgt: f64::NAN, nxt: 0.5 };
        let mut s = mk(1.); s.tgt = 13.; // Stock: 50% = 12 -> entry + 0.03, target 75% = 13
        assert_eq!(fav(&mut s, 12.5, 0, 0.03, 0.), None); assert!((s.stop - 10.53).abs() < 1e-12);
        assert_eq!(fav(&mut s, 13.2, 0, 0.03, 0.), Some((13., "Target")));
        let mut cu = mk(1.); cu.tgt = 14.; // Currency: 75% = 13 -> 50% level 12 - 0.30
        fav(&mut cu, 13.1, 1, 0.03, 0.30); assert!((cu.stop - 11.70).abs() < 1e-12 && cu.nxt.is_infinite());
        let mut ix = mk(1.); // Index: 50%, 75%, 100% passed -> 14 - 0.5
        fav(&mut ix, 14.2, 2, 0.03, 0.5); assert!((ix.stop - 13.5).abs() < 1e-12 && (ix.nxt - 1.25).abs() < 1e-12);
        let mut sh = mk(-1.); sh.stop = 10.01; sh.ep = 9.5; // short Stock: 50% = 8 -> entry - 0.03
        fav(&mut sh, 7.9, 0, 0.03, 0.); assert!((sh.stop - 9.47).abs() < 1e-12);
    }
}
