//! Dennis Meyers, "IBM Cubed" (Meyers Analytics, 1999, 22 pp.; research share Trading Knowledge III/Dennis Meyers Publications/
//! Dennis Meyers - Ibm Cubed.pdf; text authors/titles-1030/K14w/meyers.txt). Book id `dmibm`. The least-squares cubic forecast
//! system: rules p.3, parameters Figure 1 (p.7), every IBM trade 1993-98 in Figure 7 (p.13). Lane K14 (titles loop, 2 Nov 2026),
//! to-code row 211, lead decision 86; independent check authors/titles-1030/K14w/k14_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

/// Admitted ids (authors/titles-1030/K14w/k14_ref.py (header)).
pub const REVIEWED: &[&str] = &["dmibm_cubed"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

/// Figure 1 (p.7): (pctup, pctdn) of the three walk-forward windows; each is applied to the year after its window (p.3-5).
const SETS: [(f64, f64); 3] = [(10.5, 10.5), (10.0, 10.5), (6.5, 5.0)];
const T: usize = 34;

const NOTE: &str = "[Declared readings] Average price AP = (High + Low)/2. Each day a least-squares cubic a0 + b0 t + c0 t^2 + d0 t^3 is fitted to the last T = 34 average prices (t = 1 .. 34, today = 34) and Pf = its value at t = 35, the forecast for tomorrow (p.2-3; Figure 1: power 3 and T 34 in every window). Buy rule (p.3): Pf has risen by more than pctup % from the lowest Pf recorded while short AND today's average price > yesterday's Pf: cover and buy at tomorrow's open. Sell rule: Pf has fallen by more than pctdn % from the highest Pf recorded while long AND today's AP < yesterday's Pf: sell and go short at tomorrow's open. Always in the market after the first signal. Readings: the extreme is tracked over the closes from the fill day on (today's Pf included); before the first trade of a run both extremes are tracked from the run's first day, and either signal can open it. Parameters by the year of the signal day, as the paper applied them (lead decision 86): Figure 1 window 1 (10.5% / 10.5%) up to 1996 (its in-sample years 1993-95 and its out-of-sample year 1996), window 2 (10.0% / 10.5%) in 1997, window 3 (6.5% / 5.0%) from 1998 on; p.set = 1, 2 or 3 fixes one set for every year. Meyers re-optimised every year and warns that the parameters fit IBM 1993-98 only; runs on other stocks, indices and ETFs are a declared application. The paper's IBM prices are split-adjusted, not dividend-adjusted, like the vault's. Check: the paper's Figure 7 lists all 26 IBM trades 1993-98; the vault's IBM run over the same years is compared trade by trade in the gate note. Next-open fills; the vault's standard costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dmibm_cubed", name: "IBM Cubed: always in, reverse when the 34-day least-squares cubic forecast of (H+L)/2 turns by a set percentage from its extreme", group: "IBM Cubed (Meyers 1999)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "dmibm", rules: &[
            "F|\"The least squares forecast is constructed by solving for the least squares coefficients each day using the last T days of average prices. The average daily price is constructed by taking the average of the daily high and low.\" Pf = a0 + b0*(T+1) + c0*(T+1)^2 + d0*(T+1)^3 (p.2-3); Figure 1: Powr 3, T 34",
            "E|\"IF Pf has moved up by more than the percentage amount of pctup from the lowest low recorded in Pf while short and today's average price is greater than yesterday's Pf, then buy IBM at tomorrows open.\" (p.3)",
            "E|\"IF Pf has moved down by more than the percentage amount pctdn from the highest high recorded in Pf while long and today's average price is less than yesterday's Pf, then sell IBM at tomorrows open.\" (p.3)",
            "X|Always in: each signal reverses the position. Figure 1 pctup/pctdn: 10.5%/10.5% (1/1/93-12/31/95, applied 1996), 10.0%/10.5% (1/1/94-12/31/96, applied 1997), 6.5%/5.0% (1/1/95-12/31/97, applied 1998) (p.7)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "dmibm_cubed").then_some(("dmibm_cubed", "Meyers least-squares cubic forecast", "IBM Cubed, T 34"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "dmibm_cubed").then(|| vec![ParamDef { key: "set", label: "Figure 1 parameters: 0 = by year as the paper applied them; 1, 2, 3 = that window's set in every year", def: 0., min: 0., max: 3., step: 1. }])
}

/// Weights w[0..T] with Pf = sum w[j] * p[j] for the prices p of t = 1..T: the least-squares cubic's value at t = T + 1.
/// Normal equations (p.2) solved once by Gaussian elimination with partial pivoting; the forecast is linear in the prices.
/// The time axis is centred and scaled (u = (t - 17.5) / 17.5) for conditioning: the same cubics, so the same fit and forecast.
fn weights() -> Vec<f64> {
    let (c0, s0) = ((T as f64 + 1.) / 2., (T as f64 + 1.) / 2.);
    let tt: Vec<f64> = (1..=T).map(|t| (t as f64 - c0) / s0).collect();
    let mut m = [[0f64; 4]; 4];
    for (r, row) in m.iter_mut().enumerate() { for (c, v) in row.iter_mut().enumerate() { *v = tt.iter().map(|t| t.powi((r + c) as i32)).sum(); } }
    // Solve M a = X'p for each unit price vector: a = M^-1 X' e_j; Pf weight = [1, T+1, (T+1)^2, (T+1)^3] . a
    let mut inv = [[0f64; 4]; 4];
    for (i, row) in inv.iter_mut().enumerate() { row[i] = 1.; }
    let mut a = m;
    for col in 0..4 {
        let piv = (col..4).max_by(|&x, &y| a[x][col].abs().total_cmp(&a[y][col].abs())).unwrap();
        a.swap(col, piv); inv.swap(col, piv);
        for r in 0..4 { if r != col {
            let f = a[r][col] / a[col][col];
            for k in 0..4 { a[r][k] -= f * a[col][k]; inv[r][k] -= f * inv[col][k]; }
        } }
    }
    for r in 0..4 { let d = a[r][r]; for k in 0..4 { inv[r][k] /= d; } }
    let f = ((T + 1) as f64 - c0) / s0;
    let g: Vec<f64> = (0..4).map(|k| (0..4).map(|r| f.powi(r as i32) * inv[r][k]).sum()).collect();
    tt.iter().map(|t| (0..4).map(|k| g[k] * t.powi(k as i32)).sum()).collect()
}

/// Pf for every bar (NaN before T bars).
pub fn forecast(x: &Ctx) -> Vec<f64> {
    let w = weights();
    let ap: Vec<f64> = x.h.iter().zip(&x.l).map(|(h, l)| (h + l) / 2.).collect();
    (0..ap.len()).map(|i| if i + 1 < T { f64::NAN } else { w.iter().zip(&ap[i + 1 - T..=i]).map(|(a, b)| a * b).sum() }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let set = p.get("set").copied().unwrap_or(0.).round() as usize;
    let pf = forecast(x);
    let mut pos = Book::new();
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut pending = 0.;
    for t in from..=to {
        if pending != 0. {
            pos.enter(pending, t, x.o[t], if pending > 0. { "buy at the open" } else { "sell short at the open" });
            if pending > 0. { hi = f64::NEG_INFINITY; } else { lo = f64::INFINITY; }
            pending = 0.;
        }
        if t == to || t < T { continue; }
        let d = pos.side();
        if d <= 0. { lo = lo.min(pf[t]); }
        if d >= 0. { hi = hi.max(pf[t]); }
        let (up, dn) = match set {
            1..=3 => SETS[set - 1],
            _ => { let y: i32 = x.r[t].date.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(1998); SETS[if y <= 1996 { 0 } else if y == 1997 { 1 } else { 2 }] }
        };
        let ap = (x.h[t] + x.l[t]) / 2.;
        if d <= 0. && pf[t] > lo * (1. + up / 100.) && ap > pf[t - 1] { pending = 1.; }
        else if d >= 0. && pf[t] < hi * (1. - dn / 100.) && ap < pf[t - 1] { pending = -1.; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn cubic_forecast_and_reversals() {
        // the weights reproduce a cubic exactly: p(t) = 2 - t + 0.5 t^2 + 0.01 t^3 -> Pf = p(35)
        let w = weights();
        let p = |t: f64| 2. - t + 0.5 * t * t + 0.01 * t * t * t;
        let pf: f64 = w.iter().enumerate().map(|(j, w)| w * p((j + 1) as f64)).sum();
        assert!((pf - p(35.)).abs() < 1e-9 * p(35.), "{pf}");
        let r: Vec<Row> = (0..1500).map(|i| { let f = i as f64; let c = 100. + 25. * (f / 60.).sin() + 0.02 * f;
            Row { ticker: "T".into(), date: format!("{}-01-01", 1990 + i / 250), open: c, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("dmibm_cubed", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 6 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert_eq!(w[1].legs[0].i, w[0].exit_i); assert_eq!(w[1].dir, -w[0].dir); }
        assert!(family_of("dmibm_cubed").is_some() && params("dmibm_cubed").is_some() && run("dmibm_x", &x, 0, 9, &Params::new()).is_none());
    }
}

// Instrument admission: IBM Cubed 2026-11-02 (titles lane K14).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dmibm" && reviewed(def.id),
    label: "Meyers tested the system on IBM 1993-98 with walk-forward parameters. Runs on other stocks, indices and ETFs, US and India, are a declared application.",
    basis: "Dennis Meyers, IBM Cubed (1999), rules p.3, Figure 1 p.7, Figure 7 p.13; independent check authors/titles-1030/K14w/k14_ref.py",
};
