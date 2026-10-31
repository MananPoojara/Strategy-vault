//! Dean Malone (compassfx), "Traders Dynamic Index" write-up and MQL4 indicator (research share USB HDD 1/META TRADER/MetaTrader/
//! Dean Malone's Easy Method Advanced Indicators (compassfx.xom)/Dean Malone's EASY Strategy/). Book id `dmtdi`. One rule: the
//! write-up's entry and exit conditions on the TDI lines. Lane K8 (titles loop, 1 Nov 2026); independent check
//! authors/titles-1030/K8w/k8_ref.py (header).
use crate::connors::{rsi, sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["dmtdi_tdi"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Lines from the MQL4 code: RSI(13) of closes (MetaTrader's iRSI, Wilder smoothing), Green 'RSI Price Line' = 2-bar simple average of the RSI, Red 'Trade Signal Line' = 7-bar simple average, Yellow 'Market Base Line' = the middle of the 34-bar volatility band, i.e. the 34-bar average of the RSI. Default tier = 'Active' (the middle of the three printed): long when Green is over Red and Yellow, short when Red is over Green and Green is below Yellow (read like the Moderate wording 'Green below Yellow'); 'Scalping' (Green vs Red only) and 'Moderate' (also the 50 line) are options. A trade is taken on the bar where the tier's condition first becomes true, filled at the next open; long exits when Green crosses below Red, short when Green crosses above Red, at the next open; flat in between. 'If Green crosses either Blue lines, consider exiting' is advice, not coded; no stop in the write-up. Written for forex in MetaTrader on any time frame; daily bars on ETFs, US/India indices and stocks are a declared application. No printed results.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dmtdi_tdi", name: "Traders Dynamic Index: Green over Red and the Yellow base line to enter, Green/Red cross to exit (Active tier), next open", group: "Malone Traders Dynamic Index",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "dmtdi", rules: &[
            "F|RSI_Period 13; RSI_Price_Line 2; Trade_Signal_Line 7; Volatility_Band 34 (MA of the RSI +/- 1.6185 x StDev); MdZone = (UpZone + DnZone)/2 (MQL4 code)",
            "E|\"Active - Long = Green over Red & Yellow lines; Short = Red over Green & Yellow lines\" (write-up header)",
            "X|\"Exit conditions: Long = Green crosses below Red; Short = Green crosses above Red\" (write-up header)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "dmtdi_tdi").then_some(("dmtdi_tdi", "Malone Traders Dynamic Index", "Active tier, 13/2/7/34"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "dmtdi_tdi").then(|| vec![p("tier", "0 = Scalping, 1 = Active (default), 2 = Moderate (also the 50 line)", 1., 0., 2., 1.),
        p("rsi", "RSI period (13; the code allows 8-25)", 13., 8., 25., 1.), p("band", "Volatility band / base line bars (34)", 34., 20., 40., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let r = rsi(&x.c, g("rsi").round().max(1.) as usize);
    let (gr, rd, ye) = (sma(&r, 2), sma(&r, 7), sma(&r, g("band").round().max(1.) as usize));
    let tier = g("tier").round() as i64;
    let long = |i: usize| gr[i] > rd[i] && (tier < 1 || gr[i] > ye[i]) && (tier < 2 || gr[i] > 50.);
    let short = |i: usize| rd[i] > gr[i] && (tier < 1 || gr[i] < ye[i]) && (tier < 2 || gr[i] < 50.);
    let mut tgt = vec![0i8; x.c.len()];
    for i in 1..x.c.len() {
        let mut pos = tgt[i - 1];
        if (pos == 1 && gr[i] < rd[i] && gr[i - 1] >= rd[i - 1]) || (pos == -1 && gr[i] > rd[i] && gr[i - 1] <= rd[i - 1]) { pos = 0; }
        if pos == 0 { if long(i) && !long(i - 1) { pos = 1; } else if short(i) && !short(i - 1) { pos = -1; } }
        tgt[i] = pos;
    }
    Some(target_trades(x, &tgt, from, to, true, "Green crossed Red"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn enters_on_a_fresh_condition_and_exits_on_the_cross() {
        let c: Vec<f64> = (0..120).map(|i| 100. + 10. * (i as f64 / 9.).sin()).collect();
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: v, high: v + 0.5, low: v - 0.5, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("dmtdi_tdi", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let rr = rsi(&x.c, 13);
        let (gr, rd) = (sma(&rr, 2), sma(&rr, 7));
        assert!(t.len() >= 2 && t.iter().any(|z| z.dir == -1.));
        for z in t.iter().filter(|z| !z.open) {
            let s = z.exit_i - 1; // exit signalled at the previous close: Green crossed Red against the trade (or a reversal)
            assert!((z.dir > 0. && gr[s] < rd[s]) || (z.dir < 0. && gr[s] > rd[s]));
        }
    }
}

// Instrument admission: Dean Malone (Traders Dynamic Index) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dmtdi" && crate::books::dmtdi::reviewed(def.id),
    label: "A MetaTrader forex indicator with written entry/exit conditions and no results; daily ETFs, US/India indices and stocks are a declared application.",
    basis: "Dean Malone, Traders Dynamic Index write-up and Traders_Dynamic_Index.mq4 (compassfx); independent check authors/titles-1030/K8w/k8_ref.py",
};
