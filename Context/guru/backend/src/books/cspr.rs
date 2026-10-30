//! Chris Satchwell, "Pattern Recognition and Trading Decisions" (McGraw-Hill, 2005; book id `cspr`; printed pages).
//! Reading: lane B 29 Oct (authors/csatchwell/agents/prtd) and day lane A1 30 Oct (admission/t2n_ref.py). Live: the
//! Ch.17 trading system (p.293-294): the Trend2Noise market classifier (p.177-178) with thresholds +/-T, and the
//! quadratic filtered line with stop lines P -/+ z x (R + sigma) (p.187-188, equations 14.1/14.2 p.247-248) for
//! tactical entries and trailing stops; book-literal settings from Fig 15.5 / p.262 (21-week Trend2Noise, +/-0.1, 21-week
//! quadratic filter, z = 1.5). Not coded: the Trend2Noise + parabolic / moving-average illustrations (p.257-262, settings
//! not given). Book: U.S. Treasury bond futures (Ch.17 test) and the Dow weekly (Ch.15); indices and ETFs here are the
//! Dow's class, single stocks a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use super::swsp::week_ends;
/// Admitted ids (authors/csatchwell/admission/t2n_ref.py (header)).
pub const REVIEWED: &[&str] = &["cspr_t2n_filter"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly bars built from the daily bars (Monday-Sunday weeks; high = highest high, low = lowest low, close = last close; the data's last bar closes a week only on a Friday). Trend2Noise over the last 21 weeks: least-squares line through the weekly closes against the week number (m = slope per week); for each week the larger squared distance of its high or low from the line; RMS = square root of their mean; Trend2Noise = m / RMS (p.178, no regularisation). Filtered line: ordinary least-squares quadratic through the last 21 weekly closes, P = its value at the latest week (the book prints only the straight-line formula, Appendix 10.3); sigma = root-mean-square of the closes about that quadratic (divided by the window length); R = average weekly high-low range over the same 21 weeks. Stop lines for week w use weeks up to w-1 only (p.188): lower line P - z x (R + sigma), upper line P + z x (R + sigma), z = 1.5. A lower line can only rise, an upper line only fall; when a day of the week trades through the live line (low at or under the lower line, high at or over the upper line) that line ends and a fresh line starts on the other side next week (p.188). Tactical system long = the lower line is live, short = the upper line is live; before the first cut both lines are live and the first cut decides (a day touching both before any cut: no decision that week). Entry (p.294 rules 1-2): at the open of the first day of a week when, at the previous week's close, Trend2Noise > +0.1 and the tactical system is long (short: < -0.1 and tactical short). The position's stop is the live stop line (the trail stop of equation 14.1/14.2), checked on every day of the week including the entry day: filled at the line, or at the day's open if it opens through it (rule 3). No other exit. Lines and Trend2Noise are tracked from the first week of data; entries only inside the test window; a position still open at the end is marked at the last close. Book option (Fig 17.1, bond optimisation): Trend2Noise 16 weeks, filter 19, stop width 1, threshold 0.21.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "cspr_t2n_filter", name: "Trend2Noise +/-0.1 with a 21-week quadratic filtered-line trail stop (weekly)", group: "Satchwell Trend2Noise system",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "cspr",
        rules: &[
            "F|\"Θj² = max[{m(t - j) + c - High|j}², {m(t - j) + c - Low|j}²] (10.11) ... RMS = (Σ Θj² / w)^0.5 (10.12) ... Trend2Noise = m / RMS (10.13)\" (p.178)",
            "F|\"Plus or minus T might be used to define bull (Trend2Noise above +T), bear (Trend2Noise less than -T), and sideways (Trend2Noise between -T and +T) markets.\" (p.294)",
            "F|\"When an upper or lower stop line is cut, the line is ended and new one started on the opposite side of the filter line ... a stop line below the filter line cannot reduce in value and a stop line above the filter line cannot increase in value ... lines are calculated only from information available prior to the time when they are plotted.\" (p.188); \"Buy stop = P - z × (R + σ) (14.1); Sell stop = P + z × (R + σ) (14.2)\" (p.248)",
            "E|\"1. Enter long at the open of a bar following the Trend2Noise being greater than the threshold (+T) and the tactical entry system long. Place a stop order at the level indicated by the filtered line and equation 14.1. Treat it as a trail stop ... 2. Enter short at the open of a bar following the Trend2Noise being less than the threshold (-T) and the tactical entry system short.\" (p.294)",
            "X|\"3. Exit whenever stopped out, remembering that if stopped on open, the exit price to be assumed is the open price, not the stop price.\" (p.294)",
            "F|\"using the Trend2Noise thresholds of ±0.1 to define sideways markets, trading only with the primary trend, using a 21-week quadratic filtered line with stops (based on equations 14.1 and 14.2) and z value of 1.5\" (p.262, Fig 15.5: 21-week Trend2Noise)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "cspr_t2n_filter").then_some(("cspr_t2n_filter", "Satchwell Trend2Noise system", "T2N 21 +/-0.1, quadratic 21, z 1.5"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "cspr_t2n_filter").then(|| vec![
        p("t2n", "Trend2Noise length, weeks (book 21; Fig 17.1 bonds 16)", 21., 3., 104., 1.),
        p("filter", "Quadratic filter length, weeks (book 21; Fig 17.1 bonds 19)", 21., 4., 104., 1.),
        p("z", "Stop width z (book 1.5; Fig 17.1 bonds 1)", 1.5, 0.1, 5., 0.1),
        p("thr", "Trend2Noise threshold T (book 0.1; Fig 17.1 bonds 0.21)", 0.1, 0., 2., 0.01),
        p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.)])
}

/// Trend2Noise of the last n values ending at k (NaN before the window is full or with zero noise).
fn t2n(c: &[f64], h: &[f64], l: &[f64], k: usize, n: usize) -> f64 {
    if k + 1 < n { return f64::NAN; }
    let s = k + 1 - n;
    let nf = n as f64;
    let (mut si, mut sii, mut sy, mut siy) = (0., 0., 0., 0.);
    for j in 0..n { let (x, y) = (j as f64, c[s + j]); si += x; sii += x * x; sy += y; siy += x * y; }
    let m = (nf * siy - si * sy) / (nf * sii - si * si);
    let c0 = (sy - m * si) / nf;
    let th: f64 = (0..n).map(|j| { let v = m * j as f64 + c0; (v - h[s + j]).powi(2).max((v - l[s + j]).powi(2)) }).sum();
    let rms = (th / nf).sqrt();
    if rms > 0. { m / rms } else { f64::NAN }
}

/// Quadratic least-squares filter over the last n closes ending at k: (P at k, sigma, average range).
fn filt(c: &[f64], h: &[f64], l: &[f64], k: usize, n: usize) -> Option<(f64, f64, f64)> {
    if k + 1 < n { return None; }
    let s = k + 1 - n;
    let mid = (n as f64 - 1.) / 2.;
    let (mut s0, mut s2, mut s4, mut ty, mut tuy, mut tu2y) = (0., 0., 0., 0., 0., 0.);
    for j in 0..n {
        let (u, y) = (j as f64 - mid, c[s + j]);
        s0 += 1.; s2 += u * u; s4 += u.powi(4); ty += y; tuy += u * y; tu2y += u * u * y;
    }
    let det = s0 * s4 - s2 * s2;
    let (b, a, q) = (tuy / s2, (ty * s4 - s2 * tu2y) / det, (s0 * tu2y - s2 * ty) / det);
    let fit = |u: f64| a + b * u + q * u * u;
    let sig = ((0..n).map(|j| (c[s + j] - fit(j as f64 - mid)).powi(2)).sum::<f64>() / n as f64).sqrt();
    let rng = (0..n).map(|j| h[s + j] - l[s + j]).sum::<f64>() / n as f64;
    Some((fit(mid), sig, rng))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (tn, fl, z, thr) = (g("t2n").round().max(3.) as usize, g("filter").round().max(4.) as usize, g("z"), g("thr"));
    let side = g("side").round() as i64;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let we = week_ends(x);
    if we.is_empty() { return Some(vec![]); }
    let (mut wh, mut wl, mut wc) = (vec![], vec![], vec![]);
    for (k, &e) in we.iter().enumerate() {
        let s = if k == 0 { 0 } else { we[k - 1] + 1 };
        wh.push(h[s..=e].iter().cloned().fold(f64::MIN, f64::max));
        wl.push(l[s..=e].iter().cloned().fold(f64::MAX, f64::min));
        wc.push(c[e]);
    }
    let mk = |d: f64, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }],
        exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "open", signal_i: xi, open, exits: vec![] };
    // tactical state: 0 = none yet (both lines live), 1 = long (lower line), -1 = short (upper line); fresh = line restarts
    let (mut st, mut lo, mut hi, mut fresh) = (0i8, f64::NAN, f64::NAN, true);
    let (mut out, mut pos) = (vec![], None::<(f64, usize, f64)>);
    for w in 1..=we.len() {
        let start = we[w - 1] + 1;
        if start > to { break; }
        let end = if w < we.len() { we[w].min(to) } else { to };
        let k = w - 1;
        let (fv, tv) = (filt(&wc, &wh, &wl, k, fl), t2n(&wc, &wh, &wl, k, tn));
        // lines for week w from weeks up to w-1
        match fv {
            Some((pp, sg, rg)) => {
                let (rl, rs) = (pp - z * (rg + sg), pp + z * (rg + sg));
                if st >= 0 { lo = if fresh || lo.is_nan() { rl } else { lo.max(rl) }; }
                if st <= 0 { hi = if fresh || hi.is_nan() { rs } else { hi.min(rs) }; }
                fresh = false;
            }
            None => { lo = f64::NAN; hi = f64::NAN; }
        }
        // entry at the week's first open
        if pos.is_none() && tv.is_finite() && start >= from && start < to {
            if st == 1 && tv > thr && side != 2 && lo.is_finite() { pos = Some((1., start, o[start])); }
            if st == -1 && tv < -thr && side != 1 && hi.is_finite() { pos = Some((-1., start, o[start])); }
        }
        for i in start..=end {
            let cut_lo = st >= 0 && lo.is_finite() && l[i] <= lo;
            let cut_hi = st <= 0 && hi.is_finite() && h[i] >= hi;
            if !(cut_lo || cut_hi) { continue; }
            if cut_lo && cut_hi { break; } // st == 0 only: order inside the day unknown, no decision this week
            if let Some((d, e, ep)) = pos {
                if (d > 0.) == cut_lo {
                    let f = if cut_lo { o[i].min(lo) } else { o[i].max(hi) };
                    out.push(mk(d, e, ep, i, f, "Stop line hit", false));
                    pos = None;
                }
            }
            st = if cut_lo { -1 } else { 1 };
            fresh = true;
            break;
        }
    }
    if let Some((d, e, ep)) = pos { out.push(mk(d, e, ep, to, c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quadratic_filter_is_exact_on_a_parabola() {
        let c: Vec<f64> = (0..21).map(|i| { let x = i as f64; 100. + 2. * x - 0.1 * x * x }).collect();
        let (p, s, r) = filt(&c, &c, &c, 20, 21).unwrap();
        assert!((p - c[20]).abs() < 1e-9 && s < 1e-6 && r == 0.);
        let h: Vec<f64> = c.iter().map(|v| v + 1.).collect();
        let l: Vec<f64> = c.iter().map(|v| v - 1.).collect();
        let lin: Vec<f64> = (0..21).map(|i| 50. + i as f64).collect();
        let (hl, ll): (Vec<f64>, Vec<f64>) = (lin.iter().map(|v| v + 2.).collect(), lin.iter().map(|v| v - 2.).collect());
        assert!((t2n(&lin, &hl, &ll, 20, 21) - 0.5).abs() < 1e-12); // slope 1 / RMS 2
        assert!(t2n(&c, &h, &l, 19, 21).is_nan());
    }
}
