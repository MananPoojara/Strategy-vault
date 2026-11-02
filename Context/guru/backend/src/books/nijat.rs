//! Nija Turtle (Glenn Schultz, 10 Sep 1997), TradeStation code. Research share USB HDD 1/TradeStation Systems & Indicators/
//! _Misc/Nija Turtle System/code.txt (copy authors/batch-j/J1w/nija.txt). Book id `nijat`. Lane K17 (titles loop, 2 Nov 2026),
//! to-code row 305; independent check authors/titles-1030/K17w/k17_ref.py. Orders run in `rrsim` (TradeStation conventions).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{g, o, pd, run_ts, sdef, sma, A, P};

pub const REVIEWED: &[&str] = &["nijat_bands"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed code, TradeStation 1997 syntax (Buy / Sell = long / short entries, ExitLong / ExitShort = exits): Value1 = Average(Close,35), Value2 = StdDev(Close,35), bands Value1 +/- 2 x Value2. At each close: Close above the upper band -> buy at the next open; below the lower band -> sell short at the next open (reversing a position); while long, a close under the 35-day average -> exit at the next open; while short, a close over it -> exit at the next open. StdDev = TradeStation's population standard deviation (p.sd = 1: the sample one). 'Vars: Vollen(20)' is declared and never used. No stop or target. One unit. Daily bars on indices, ETFs and US/India stocks are a declared application (the code names no market); the vault's standard costs. Bollinger-type band breakout like Aberration (kfaber, 80 days); this is Schultz's own code with 35 days (lead decision 48).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("nijat", "nijat_bands", "Nija Turtle (Schultz): close outside the 35-day 2-SD band, enter next open; out on a close back through the average",
        "Nija Turtle (TradeStation code, 1997)", "Long/Short", Status::Live, &[
            "F|\"Value1 = Average(Close,35); Value2 = StdDev(Close,35); Value3 = Value1 + (2 * Value2); Value4 = Value1 - (2 * Value2);\"",
            "E|\"If Close > Value3 Then Buy Next Bar at Open; If Close < Value4 Then Sell Next Bar at Open;\"",
            "X|\"If marketposition(0) = 1 then begin If Close < Average(Close,35) then ExitLong Next Bar at Open; ... If marketposition(0) = -1 then begin If Close > Average(Close,35) then ExitShort Next Bar at Open;\""],
        NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("nijat_bands", "Nija Turtle band breakout", "35 days, 2 SD"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![pd("sd", "StdDev: 0 = population (TradeStation); 1 = sample", 0., 0., 1., 1.)])
}

/// Standard deviation of the last n values (ddof 0 = population, 1 = sample); NaN before n.
pub fn sdev(v: &[f64], n: usize, ddof: f64) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let w = &v[i + 1 - n..=i]; let m = w.iter().sum::<f64>() / n as f64;
        (w.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (n as f64 - ddof)).sqrt()
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (m, s) = (sma(&x.c, 35), sdev(&x.c, 35, if g(p, "sd", 0.) >= 0.5 { 1. } else { 0. }));
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if !(m[i].is_finite() && s[i].is_finite()) { return; }
        let c = x.c[i];
        if c > m[i] + 2. * s[i] { next.push(o(A::Buy, P::Mkt, "LE")); }
        if c < m[i] - 2. * s[i] { next.push(o(A::Sell, P::Mkt, "SE")); }
        if st.pos == 1 && c < m[i] { next.push(o(A::Xl, P::Mkt, "LX")); }
        if st.pos == -1 && c > m[i] { next.push(o(A::Xs, P::Mkt, "SX")); }
    }))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::Row;
    /// Synthetic daily bars with trends, swings and volume (shared by the K17 modules' tests).
    pub fn bars(n: usize) -> Vec<Row> {
        let mut seed = 0x2545F4914F6CDD1Du64;
        let mut rnd = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; (seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5 };
        let mut c = 100.0f64;
        (0..n).map(|i| {
            let f = i as f64;
            let prev = c;
            let drift = 0.004 * (f / 150.).sin();
            c = (c * (1. + drift + 0.03 * rnd())).max(5.);
            let op = prev * (1. + 0.01 * rnd());
            let (h, l) = (c.max(op) * (1. + 0.012 * (rnd() + 0.5)), c.min(op) * (1. - 0.012 * (rnd() + 0.5)));
            Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 336, 1 + i / 28 % 12, 1 + i % 28), open: op, high: h, low: l, close: c,
                volume: Some(1.0e6 * (1. + rnd())) }
        }).collect()
    }
    /// Every reviewed id of a module trades on the synthetic bars, with sane fills (inside the bar) and no overlap.
    pub fn sane(run: fn(&str, &Ctx, usize, usize, &Params) -> Option<Vec<Trade>>, ids: &[&str]) {
        let r = bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        for id in ids {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() >= 3, "{id}: {} trades", t.len());
            for z in &t {
                let (i, px) = (z.legs[0].i, z.legs[0].px);
                assert!(px >= r[i].low - 1e-9 && px <= r[i].high + 1e-9, "{id} entry {i} {px}");
                assert!(z.exit_px >= r[z.exit_i].low - 1e-9 && z.exit_px <= r[z.exit_i].high + 1e-9, "{id} exit");
            }
            for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i, "{id}"); }
            assert!(run(id, &x, 5, 5, &Params::new()).is_none());
        }
    }
    #[test]
    fn nija_bands() {
        sane(run, REVIEWED);
        let r = bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("nijat_bands", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().all(|z| z.legs[0].px == r[z.legs[0].i].open) && t.iter().any(|z| z.dir < 0.));
        assert!((sdev(&[1., 2., 3., 4.], 4, 0.)[3] - 1.25f64.sqrt()).abs() < 1e-12);
    }
}

// Instrument admission: Nija Turtle 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "nijat" && reviewed(def.id),
    label: "TradeStation code with no market or test named; runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Nija Turtle code.txt (Glenn Schultz, 1997), USB HDD 1/TradeStation Systems & Indicators/_Misc/Nija Turtle System; independent check authors/titles-1030/K17w/k17_ref.py",
};
