//! LaSalle Street Trading Group, RemaBands strategy (TradeStation/MultiCharts; research share USB HDD 1/TradeStation
//! Systems & Indicators/LaSalle Street Trading Group System Pack/LaSalle_RemaBands_Strategy.txt, plain-text EasyLanguage,
//! + Readme_Remabands.docx). Book id `lsrema`. One rule: regularized-EMA bands, stop-and-reverse on the close.
//! Lane K6 (titles loop, 31 Oct 2026); independent check authors/titles-1030/K6w/k6_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

/// Admitted ids (authors/titles-1030/K6w/k6_ref.py (header)).
pub const REVIEWED: &[&str] = &["lsrema_bands"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Code read literally: REMA = (REMA[1] x (1 + 2 x Ceres) + alpha x (C - REMA[1]) - Ceres x REMA[2])/(1 + Ceres), alpha = 2/(EmaLength + 1), once CurrentBar > 3, else REMA = C; CurrentBar counts from the first bar of the data. Lower = REMA x (1 - PctBelow%), Upper = REMA x (1 + PctAbove%). 'LOW 1 BAR AGO CROSS OVER Var3' = yesterday's low above today's lower band after the low two bars ago was at or below yesterday's lower band (EasyLanguage crosses over); the short mirrors it with the high and the upper band. Both orders are 'this bar on close', so the position reverses at the signal bar's close and is always in after the first signal ('This is a reversal system and is in the market over 95% of the time', readme). If both signals fire on one bar the later order in the code (sell short) decides. Defaults = the code's inputs (9, 9, 1, 1); the readme says to optimise them. Readme example: ES intraday, 1 lot; daily bars on indices, ETFs and US/India stocks are a declared application. The readme's '$12,672 in 25 days, no losing trades' is a vendor claim and is not shown.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "lsrema_bands", name: "RemaBands: regularized EMA bands, reverse on the close when yesterday's low or high crosses back over a band", group: "LaSalle Street Trading Group RemaBands",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "lsrema", rules: &[
            "F|\"If CurrentBar > 3 then Var1 = (Var1[1]*(1+2*Ceres)+Var2*(CLOSE-Var1[1])-Ceres*Var1[2])/(1+Ceres) ELSE Var1 = CLOSE\"; Var2 = 2/(EmaLength+1); Var3 = Var1*(1-PctBelow*0.01); Var4 = Var1*(1+PctAbove*0.01)",
            "E|\"Var7 = LOW 1 BAR AGO CROSS OVER Var3; if Var7 then BUY (\"Buy\") THIS BAR CLOSE\"",
            "E|\"Var8 = HIGH 1 BAR AGO CROSS UNDER Var4; if Var8 then SELLSHORT (\"Sell\") THIS BAR CLOSE\"",
            "X|\"This is a reversal system\" (readme): each entry closes the opposite position"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "lsrema_bands").then_some(("lsrema_bands", "LaSalle RemaBands", "REMA 9, Ceres 9, bands 1%"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "lsrema_bands").then(|| vec![p("len", "EmaLength (code 9)", 9., 2., 50., 1.), p("ceres", "Ceres, the regularization weight (code 9)", 9., 0., 50., 1.),
        p("above", "PctAbove, upper band % (code 1)", 1., 0.1, 10., 0.1), p("below", "PctBelow, lower band % (code 1)", 1., 0.1, 10., 0.1)])
}

/// Regularized EMA as the strategy code computes it (REMA = C on the first three bars).
pub fn rema(c: &[f64], len: f64, lam: f64) -> Vec<f64> {
    let a = 2. / (len + 1.);
    let mut v = c.to_vec();
    for i in 3..c.len() { v[i] = (v[i - 1] * (1. + 2. * lam) + a * (c[i] - v[i - 1]) - lam * v[i - 2]) / (1. + lam); }
    v
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let v = rema(&x.c, g("len").max(1.), g("ceres").max(0.));
    let lo: Vec<f64> = v.iter().map(|r| r * (1. - g("below") * 0.01)).collect();
    let hi: Vec<f64> = v.iter().map(|r| r * (1. + g("above") * 0.01)).collect();
    let mut tgt = vec![0i8; x.c.len()];
    for i in 2..x.c.len() {
        let buy = x.l[i - 1] > lo[i] && x.l[i - 2] <= lo[i - 1];
        let sell = x.h[i - 1] < hi[i] && x.h[i - 2] >= hi[i - 1];
        tgt[i] = if sell { -1 } else if buy { 1 } else { tgt[i - 1] };
    }
    Some(target_trades(x, &tgt, from, to, false, "reversal"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn rema_and_reversals_on_the_close() {
        let v = rema(&[10., 11., 12., 13., 14.], 9., 9.);
        // bar 3: (12 x 19 + 0.2 x (13 - 12) - 9 x 11)/10 = 12.92
        assert!((v[3] - 12.92).abs() < 1e-12 && v[2] == 12.);
        // flat closes at 100 -> bands 99 / 101; a low below 99 then back above -> buy; a high above 101 then back -> short
        let b = [(100., 100.), (100.5, 99.5), (100.5, 98.), (100.5, 99.5), (100.5, 99.5), (102., 99.5), (100.5, 99.5), (100.5, 99.5)];
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(h, l))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: 100., high: h, low: l, close: 100., volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lsrema_bands", &x, 0, r.len() - 1, &Params::new()).unwrap();
        // bar 4: low[3] 99.5 > 99 and low[2] 98 <= 99 -> buy; bar 7: high[6] 100.5 < 101 and high[5] 102 >= 101 -> short
        assert_eq!(t.iter().map(|z| (z.dir, z.legs[0].i, z.exit_i, z.open)).collect::<Vec<_>>(), vec![(1., 4, 7, false), (-1., 7, 7, true)]);
        assert!(run("lsrema_x", &x, 0, 7, &Params::new()).is_none());
    }
}

// Instrument admission: LaSalle Street Trading Group (RemaBands) 2026-10-31 (titles lane K6).
// Admission only for the module REVIEWED lists (authors/titles-1030/K6w/k6_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "lsrema" && crate::books::lsrema::reviewed(def.id),
    label: "A vendor TradeStation strategy shown on ES intraday bars; daily runs on indices, ETFs and US/India stocks are a declared application, not the vendor's test.",
    basis: "LaSalle Street Trading Group System Pack, LaSalle_RemaBands_Strategy.txt (EasyLanguage) and Readme_Remabands.docx",
};
