//! John F. Ehlers, "The MACD Indicator Revisited", Technical Analysis of Stocks & Commodities V.9:10 (October 1991) pp.391-395
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V09/C10/MACDREV.PDF).
//! Book id `scehmacd`. Lane K16 (titles loop, 2 Nov 2026), to-code row 235, lead decision 73; independent check
//! authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{ema0, next_open};

pub const REVIEWED: &[&str] = &["scehmacd_412"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The article's 'inverse' parameter set, read off its S&P 500 profit surface for the 12-day cycle of early 1991, is the default (decision 73): first EMA 4 days, second EMA 12 days, signal = EMA of the MACD line with the second EMA's length ('the signal line uses the same parameters as the second EMA'). The MACD line is formed as printed, 'by subtracting the long (first) moving average from the short (second) moving average', so with the inverse set it is EMA(12) - EMA(4); the article confirms it 'is somewhat out of phase' with price ('When the price reaches a peak, the MACD is at a valley'). Option sign = 1 uses EMA(4) - EMA(12). Buy when the MACD line crosses above its signal, sell and reverse when it crosses below; 'we can't actually buy or sell until the day after we get the signal', so fills are at the next open. EMAs use alpha 2/(n+1), seeded with the first close. The author warns that 'this particular set of parameters is only valid in the near past'. The article prints no trade results. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scehmacd_412", name: "Ehlers inverse MACD 4/12: MACD = EMA12 - EMA4 crosses its 12-day EMA signal, stop and reverse at the next open", group: "S&C: The MACD Indicator Revisited (Ehlers, V.9:10)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scehmacd", rules: &[
            "F|\"The MACD line is formed by subtracting the long (first) moving average from the short (second) moving average. A signal line is formed by smoothing the MACD line with a third EMA ... I use the third EMA to be equal in length to the second EMA.\" (V.9:10, PDF p.1)",
            "F|\"the maximum profit occurs in the 'inverse' domain where the first EMA is smaller than the second EMA ... this 4/12-day EMA parameter set\" (PDF p.5); \"When we invert the first (four-day) EMA and perform the vector addition ... the resulting MACD lags the price function by 203 degrees\" (PDF p.10)",
            "E|\"buy/sell signals are formed when the MACD crosses its signal\" (PDF p.1); \"Since we can't actually buy or sell until the day after we get the signal\" (PDF p.5): next open",
            "X|Stop and reverse on the opposite crossing (always in)."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scehmacd_412", "Ehlers inverse MACD (S&C)", "EMA 4 / 12, signal 12"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("first", "First EMA, days (article 4)", 4., 2., 40., 1.), p("second", "Second EMA and signal, days (article 12)", 12., 2., 40., 1.),
        p("sign", "0 = MACD as printed (second - first); 1 = first - second", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (f, s) = (g("first", 4.).round().max(1.) as usize, g("second", 12.).round().max(1.) as usize);
    let (e1, e2) = (ema0(&x.c, f), ema0(&x.c, s));
    let m: Vec<f64> = (0..x.c.len()).map(|i| if g("sign", 0.) > 0.5 { e1[i] - e2[i] } else { e2[i] - e1[i] }).collect();
    let sg = ema0(&m, s);
    Some(next_open(x, from, to, "reversed", |t, side| {
        if t == 0 { return side; }
        if m[t] > sg[t] && m[t - 1] <= sg[t - 1] { 1.0 } else if m[t] < sg[t] && m[t - 1] >= sg[t - 1] { -1.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inverse_macd_reverses() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scehmacd_412", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().all(|z| z.legs[0].px == x.o[z.legs[0].i]));
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert_ne!(w[0].dir, w[1].dir); }
        assert!(run("scehmacd_x", &x, 0, 9, &Params::new()).is_none());
    }
}

// Instrument admission: S&C Ehlers MACD revisited 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scehmacd" && reviewed(def.id),
    label: "John Ehlers read the 4/12 set off an S&P 500 profit surface (early 1991, no printed results). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.9:10 pp.391-395 (The MACD Indicator Revisited); independent check authors/titles-1030/K16w/k16_ref.py",
};
