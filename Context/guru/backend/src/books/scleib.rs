//! Lee Leibfarth, "Trading With An Adaptive Price Zone", Technical Analysis of Stocks & Commodities V.24:9 (September 2006)
//! pp.28-31, EasyLanguage sidebar (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008)
//! (traders.com)/VOLUMES/V24/C09/170LEIB.pdf). Book id `scleib`. Lane K19 (titles loop, 2 Nov 2026), to-code row 275;
//! independent check authors/titles-1030/K19w/k19_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{dmi, next_open};

pub const REVIEWED: &[&str] = &["scleib_apz"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The sidebar's 'Adaptive Price Zone Strategy' inputs Period 20, BandPct 1.4, ADXThreshold 30 are the default (decision 73; the chart examples use 30/1.8, 20/1.2 and 50/2.2). Double_Smooth_EMA = XAverage(XAverage(price, sqrt(Length)), sqrt(Length)), TradeStation's XAverage with the fractional period (factor 2 / (period + 1), started at the first value). ADX(14) is Wilder's (decision 56). At each close: ADX <= 30 and low <= LowerBand -> buy next bar at market; ADX <= 30 and high >= UpperBand -> sell short next bar at market (when both bands are touched on one bar the later printed order, the short, stands); ADX > 30 -> sell and buy to cover next bar at market. Fills at the next open; no stop or target (the article recommends money management but gives no values). Article: chart examples on e-mini S&P, QQQQ and Dow minis, no results. Bandy's hbqts_apz is a different, long-only AmiBroker version (50 / 2, close fills). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scleib_apz", name: "Leibfarth adaptive price zone: fade the 1.4 x double-smoothed range bands while ADX(14) is 30 or less, flat above 30", group: "S&C: Trading With An Adaptive Price Zone (Leibfarth, V.24:9)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scleib", rules: &[
            "F|\"Period = iff(Length < 0, 1, squareroot(Length)); X = Xaverage( Xaverage(Price, Period), Period)\"; \"UpperBand = BandPct * RangeDsEMA + DsEMA; LowerBand = DsEMA - BandPct * RangeDsEMA\" with RangeDsEMA = Double_Smooth_Ema((H - L), period) (sidebar, p.29)",
            "E|\"if AdxValue <= ADXTheshold then begin if l <= LowerBand then buy next bar at market; if h >= UpperBand then sell short next bar at market\" (Period 20, BandPct 1.4, ADXThreshold 30, ADX(14))",
            "X|\"if AdxValue > ADXTheshold then begin sell next bar at market; buy to cover next bar at market\" (sidebar)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scleib_apz", "Leibfarth adaptive price zone (S&C)", "20 / 1.4 / ADX 30"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("period", "Period (20)", 20., 1., 100., 1.), p("band", "BandPct (1.4)", 1.4, 0.1, 4., 0.1), p("adx", "ADX threshold (30)", 30., 5., 60., 1.)])
}

/// TradeStation XAverage with a (possibly fractional) period: started at the first value.
fn xaverage(v: &[f64], period: f64) -> Vec<f64> {
    let a = 2.0 / (period + 1.0);
    let mut out = Vec::with_capacity(v.len());
    for (i, &z) in v.iter().enumerate() { out.push(if i == 0 { z } else { out[i - 1] + a * (z - out[i - 1]) }); }
    out
}

pub fn double_smooth_ema(v: &[f64], len: f64) -> Vec<f64> {
    let period = if len < 0.0 { 1.0 } else { len.sqrt() };
    xaverage(&xaverage(v, period), period)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (len, bp, th) = (g("period", 20.), g("band", 1.4), g("adx", 30.));
    let ds = double_smooth_ema(&x.c, len);
    let rng: Vec<f64> = (0..x.r.len()).map(|i| x.h[i] - x.l[i]).collect();
    let rd = double_smooth_ema(&rng, len);
    let adx = dmi(x, 14).2;
    Some(next_open(x, from, to, "signal", |t, side| {
        if !adx[t].is_finite() { return side; }
        if adx[t] > th { return 0.0; }
        if x.h[t] >= ds[t] + bp * rd[t] { -1.0 } else if x.l[t] <= ds[t] - bp * rd[t] { 1.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xaverage_fractional_and_fades() {
        let e = xaverage(&[10., 20., 20.], 3.0_f64.sqrt());
        let a = 2.0 / (3.0_f64.sqrt() + 1.0);
        assert!((e[1] - (10. + a * 10.)).abs() < 1e-12 && e[0] == 10.);
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scleib_apz", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3);
    }
}

// Instrument admission: S&C Leibfarth adaptive price zone 2026-11-02 (titles lane K19).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scleib" && reviewed(def.id),
    label: "Leibfarth showed the adaptive price zone on e-mini S&P, QQQQ and Dow mini charts, with no test results. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.24:9 pp.28-31 (Trading With An Adaptive Price Zone, EasyLanguage sidebar); independent check authors/titles-1030/K19w/k19_ref.py",
};
