//! Scott Castleman, "Forecasting Trends With Indexes: Using Implied Volatility And Volume", Technical Analysis of Stocks &
//! Commodities V.21:6 (June 2003) pp.32-37, TradeStation code Figure 4 (research share Trading Knowledge I/Stocks & Commodities
//! DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V21/C06/117CAST.pdf). Book id `sccast`. Lane K19 (titles loop, 2 Nov 2026),
//! to-code row 271; independent check authors/titles-1030/K19w/k19_ref.py.
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::scpass::vix_rows;

pub const REVIEWED: &[&str] = &["sccast_ivvol"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Data set 2 ('the appropriate volatility index') is the engine's volatility index for the instrument's market (US: CBOE VIX; India: India VIX), taken on this instrument's trading days; a day without a volatility close uses the latest earlier one (TradeStation's data 2). Volume is the instrument's daily volume; a 10- or 50-day window with a missing volume day does not update the look-backs. dayLengthLong and dayLengthShort are only set once both ranges exist (printed: 'If (highestIV - lowestIV) <> 0 and (highestVolume - lowestVolume) <> 0'), and keep their last value otherwise. highest(high[1], n) = the highest high of the n bars before the current one. Both orders are next-bar market orders at the open; when both breakouts happen on one bar, the later printed order (sell short) stands. Always in once the first order fills. Printed inputs 29 / 25 / 252 / 50. Article: Russell 2000 futures 1/10/1996-12/31/2002, 64 trades, 46.88% profitable, +$199,750 (a volatility index for the Russell is not named; VIX is used here for every US instrument). Runs on indices, ETFs and stocks with volume, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sccast_ivvol", name: "Castleman implied volatility and volume Donchian: look-backs shrink with high VIX and volume, stop and reverse at the next open", group: "S&C: Using Implied Volatility And Volume (Castleman, V.21:6)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: true, needs_vix: true, needs_series: &[], book: "sccast", rules: &[
            "F|\"dayLengthLong = intportion(refLength1 - 0.5*refLength2*((currentIV - lowestIV)/(highestIV - lowestIV) + (avgVolume - lowestVolume)/(highestVolume-lowestVolume)))\"; dayLengthShort likewise with (highestIV - currentIV) and (highestVolume - avgVolume); IV = close of data 2 over 252 days, avgVolume = average(volume,10), volume range over 50 days (Figure 4, p.36)",
            "E|\"If high>highest(high[1],dayLengthLong) then buy next bar on open\" (Figure 4)",
            "X|\"If low<lowest(low[1],dayLengthShort) then sell Short next bar on open\" (Figure 4); a Donchian stop-and-reverse system (p.36)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("sccast_ivvol", "Castleman IV and volume Donchian (S&C)", "29 / 25 / 252 / 50"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("ref1", "refLength1 (29)", 29., 5., 60., 1.), p("ref2", "refLength2 (25)", 25., 1., 50., 1.),
        p("lbiv", "lookBackIV, days (252)", 252., 20., 500., 1.), p("lbvol", "lookBackVolume, days (50)", 50., 10., 200., 1.)])
}

/// Volatility closes on this instrument's days, carried forward over days without one (NaN before the first).
fn vix_on_bars(x: &Ctx, vix: &[(String, f64)]) -> Vec<f64> {
    let m: std::collections::HashMap<&str, f64> = vix.iter().map(|(d, c)| (d.as_str(), *c)).collect();
    let mut last = f64::NAN;
    x.r.iter().map(|r| { if let Some(&c) = m.get(r.date.as_str()) { if c.is_finite() { last = c; } } last }).collect()
}

/// (dayLengthLong, dayLengthShort) per bar, 0 until first set.
pub fn day_lengths(x: &Ctx, v: &[f64], p: &Params) -> (Vec<usize>, Vec<usize>) {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (r1, r2) = (g("ref1", 29.), g("ref2", 25.));
    let (liv, lvol) = (g("lbiv", 252.).round().max(1.) as usize, g("lbvol", 50.).round().max(1.) as usize);
    let n = x.r.len();
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|v| v.is_finite() && *v > 0.0).unwrap_or(f64::NAN)).collect();
    let av = sma(&vol, 10);
    let (mut dl, mut ds) = (vec![0usize; n], vec![0usize; n]);
    let (mut l, mut s) = (0usize, 0usize);
    for i in 0..n {
        if i + 1 >= liv && i + 1 >= lvol && av[i].is_finite() && v[i + 1 - liv..=i].iter().all(|z| z.is_finite()) && vol[i + 1 - lvol..=i].iter().all(|z| z.is_finite()) {
            let (loiv, hiiv) = (lo(v, i + 1 - liv, i), hi(v, i + 1 - liv, i));
            let (lov, hiv) = (lo(&vol, i + 1 - lvol, i), hi(&vol, i + 1 - lvol, i));
            if hiiv - loiv != 0.0 && hiv - lov != 0.0 {
                l = (r1 - 0.5 * r2 * ((v[i] - loiv) / (hiiv - loiv) + (av[i] - lov) / (hiv - lov))).trunc().max(0.0) as usize;
                s = (r1 - 0.5 * r2 * ((hiiv - v[i]) / (hiiv - loiv) + (hiv - av[i]) / (hiv - lov))).trunc().max(0.0) as usize;
            }
        }
        dl[i] = l;
        ds[i] = s;
    }
    (dl, ds)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let v = vix_on_bars(x, vix_rows());
    let (dl, ds) = day_lengths(x, &v, p);
    Some(super::sctraub::next_open(x, from, to, "reversed", |t, side| {
        let short = ds[t] >= 1 && t >= ds[t] && x.l[t] < lo(&x.l, t - ds[t], t - 1);
        let long = dl[t] >= 1 && t >= dl[t] && x.h[t] > hi(&x.h, t - dl[t], t - 1);
        if short { -1.0 } else if long { 1.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lengths_in_printed_range_and_reversals() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let v: Vec<f64> = (0..r.len()).map(|i| 20. + 8. * (i as f64 / 23.).sin()).collect();
        let (dl, ds) = day_lengths(&x, &v, &Params::new());
        assert!(dl[300..].iter().chain(&ds[300..]).all(|&k| (4..=29).contains(&k)));
        assert_eq!(dl[100], 0);
        assert!(family_of("sccast_ivvol").is_some() && params("sccast_ivvol").is_some());
    }
}

// Instrument admission: S&C Castleman implied volatility and volume 2026-11-02 (titles lane K19).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sccast" && reviewed(def.id),
    label: "Castleman tested the system on Russell 2000 futures (1996-2002) with a volatility index as data 2. Runs on indices, ETFs and stocks with volume, US and India, with their market's VIX, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.21:6 pp.32-37 (Using Implied Volatility And Volume, TradeStation code Figure 4); independent check authors/titles-1030/K19w/k19_ref.py",
};
