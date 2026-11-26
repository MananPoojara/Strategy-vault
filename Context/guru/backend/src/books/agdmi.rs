//! Kenneth Agostino and Brian Dolan (GAIN Capital), "Make the Trend Your Friend in Forex" (Technical Analysis of Stocks & Commodities 2004, Forex Markets; research share Trading Knowledge III/
//! !PDFs/Make the Trend Your Friend in Forex.pdf, text layer, read in full). Book id `agdmi`. The DMI system: a buy signal when DI+ crosses up through DI-, confirmed by Wilder's extreme-point rule
//! (the high of the crossover period must be exceeded in the subsequent period); a sell mirrored; once the ADX has registered above 25, the first lower ADX close "signals it is time to exit the
//! current position and wait for a fresh signal from the DI+/DI- crossover". Lane K1 (24 Nov 2026, queue Q381 row 113). Independent check authors/conv-1123/K1/agdmi_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "agdmi";
pub const REVIEWED: &[&str] = &["agdmi_daily"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340] The article trades forex (GBP/USD, AUD/USD) on daily and hourly charts; this card runs the DMI rules on the NSE indices NIFTY 50 and NIFTY BANK (and the US indices) on DAILY bars (the article's first two examples are daily). It prints no trade list or results; nothing is compared, UNVERIFIED. The trendline, parabolic-indicator and chart-pattern confirmations the article also uses are discretionary or alternative exits and are not coded. [Declared readings, each an unstated detail] (1) DMI/ADX length 14 (Wilder's; the article states none), Wilder smoothing, ADX = Wilder average of DX. (2) Extreme-point confirmation: the signal lives for the one bar after the crossover bar; the trade enters on a stop at the crossover bar's high (short: low) if the next bar trades through it, at the level or at the open on a gap; the opposite entry reverses a held position at its own price. (3) The ADX exit: after the ADX has closed above 25 on any bar since entry, the first close with the ADX below the previous close's ADX exits at that close. (4) One position at a time.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "agdmi_daily", name: "DMI with Wilder's extreme-point rule: DI crossover confirmed by the next bar, exit at the first lower ADX close after ADX > 25 (daily, indices)",
        group: "Agostino-Dolan DMI trend system", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "agdmi",
        rules: &[
            "E|\"A buy signal is generated when the DI+ line crosses up through the DI- line; a sell signal is generated when the DI- line crosses up through the DI+ line. ... note the extreme point for that period in the direction of the crossover (the high if DI+ crosses up over DI-; the low if DI- crosses up over DI+). Only if that extreme point is breached in the subsequent period is a trade signal confirmed.\"",
            "X|\"When the ADX begins to move lower from its highest level, the trend is either pausing or ending. This signals it is time to exit the current position and wait for a fresh signal from the DI+/DI- crossover\" (a reading above 25 indicates a trend)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "agdmi_daily" => Some(("agdmi", "DMI + extreme-point rule + ADX exit", "length 14, ADX 25")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "agdmi_daily" => Some(vec![p("len", "DMI/ADX length (Wilder's 14; the article states none)", 14., 5., 40., 1.), p("trend", "ADX level that counts as a trend (article 25)", 25., 10., 50., 1.)]),
        _ => None,
    }
}

/// Wilder DMI: returns (DI+, DI-, ADX), NaN until defined.
fn dmi(h: &[f64], l: &[f64], c: &[f64], n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let m = c.len();
    let (mut pdi, mut mdi, mut adx) = (vec![f64::NAN; m], vec![f64::NAN; m], vec![f64::NAN; m]);
    if m <= 2 * n { return (pdi, mdi, adx); }
    let (mut tr, mut pd, mut md) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
    for i in 1..m {
        tr[i] = (h[i] - l[i]).max((h[i] - c[i - 1]).abs()).max((l[i] - c[i - 1]).abs());
        let (up, dn) = (h[i] - h[i - 1], l[i - 1] - l[i]);
        pd[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        md[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
    }
    let nf = n as f64;
    let (mut st, mut sp, mut sm) = (tr[1..=n].iter().sum::<f64>(), pd[1..=n].iter().sum::<f64>(), md[1..=n].iter().sum::<f64>());
    let mut dx = vec![f64::NAN; m];
    for i in n..m {
        if i > n { st = st - st / nf + tr[i]; sp = sp - sp / nf + pd[i]; sm = sm - sm / nf + md[i]; }
        pdi[i] = 100.0 * sp / st; mdi[i] = 100.0 * sm / st;
        let s = pdi[i] + mdi[i];
        dx[i] = if s == 0.0 { 0.0 } else { 100.0 * (pdi[i] - mdi[i]).abs() / s };
    }
    let first = 2 * n - 1;
    adx[first] = dx[n..=first].iter().sum::<f64>() / nf;
    for i in first + 1..m { adx[i] = (adx[i - 1] * (nf - 1.0) + dx[i]) / nf; }
    (pdi, mdi, adx)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (n, lvl) = (g("len").round() as usize, g("trend"));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (pdi, mdi, adx) = dmi(h, l, c, n);
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64)> = None; // dir, entry bar, entry price
    let mut peak_seen = false;
    let mut pend: Option<(f64, f64)> = None; // direction, extreme level from the crossover bar
    let kind = "stop";
    let mk = |d: f64, e: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: e.saturating_sub(1), open, exits: vec![] };
    for t in 1..=to {
        if t >= from {
            if let Some((d, ext)) = pend.take() {
                let hit = if d > 0.0 { h[t] > ext } else { l[t] < ext };
                if hit {
                    let px = if d > 0.0 { o[t].max(ext) } else { o[t].min(ext) };
                    match pos { Some((pd_, e, ep)) if pd_ != d => { out.push(mk(pd_, e, ep, t, px, "opposite signal", false)); pos = Some((d, t, px)); peak_seen = false; }
                                None => { pos = Some((d, t, px)); peak_seen = false; }
                                _ => {} }
                }
            }
            if let Some((d, e, ep)) = pos {
                // ADX exit at this close: ADX has closed above the level on a bar since entry (before this one) and now closes lower
                if peak_seen && adx[t] < adx[t - 1] { out.push(mk(d, e, ep, t, c[t], "ADX turned down after trending", false)); pos = None; peak_seen = false; }
                else if adx[t].is_finite() && adx[t] > lvl { peak_seen = true; }
            }
            if t < to && !pdi[t - 1].is_nan() && !mdi[t - 1].is_nan() {
                if pdi[t - 1] <= mdi[t - 1] && pdi[t] > mdi[t] { pend = Some((1.0, h[t])); }
                else if mdi[t - 1] <= pdi[t - 1] && mdi[t] > pdi[t] { pend = Some((-1.0, l[t])); }
            }
        }
    }
    if let Some((d, e, ep)) = pos { out.push(mk(d, e, ep, to, c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn trades_and_rejects_other_ids() {
        let r: Vec<Row> = (0..300).map(|i| { let c = 100. + (i as f64 * 0.12).sin() * 10. + i as f64 * 0.1; Row { ticker: "NIFTY 50".into(), date: format!("2010-{:03}", i), open: c - 0.2, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("agdmi_x", &x, 0, 299, &Params::default()).is_none());
        let t = run("agdmi_daily", &x, 0, 299, &Params::default()).unwrap();
        for z in &t { assert!(z.exit_i >= z.legs[0].i); }
    }
}

// Instrument admission: indices only (never single stocks or ETFs), declared adaptation.
pub fn index_only() -> &'static str { "DMI system with the extreme-point rule (forex article): run on actual indices only (NIFTY 50 / NIFTY BANK and the US indices) as a declared adaptation (R4 #340); UNVERIFIED" }
