//! Jeanette Schwarz Young, "The Commodity Channel Index Revisited", Technical Analysis of Stocks & Commodities V.13:12 (December
//! 1995) pp.510-512 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/
//! C12/THECOMM.pdf). Book id `scyodcci`: the dual CCI. Lane K16 (titles loop, 2 Nov 2026), to-code row 293; independent check
//! authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::atm03::cci_of;
use super::sctraub::{ema0, next_open};

pub const REVIEWED: &[&str] = &["scyodcci_20_5"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] DCCI = 5-period EMA of the 20-period CCI (Lambert's CCI: typical price, its 20-day SMA, mean deviation, constant 0.015; the EMA starts at the first CCI value). Buy when the DCCI crosses above zero, sell short when it crosses below zero; stop and reverse, filled at the next open. The article (daily OEX example, 1995) shows signals on a chart and prints no results. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scyodcci_20_5", name: "Young dual CCI: 5-period EMA of CCI(20) crosses zero, stop and reverse at the next open", group: "S&C: The Commodity Channel Index Revisited (Young, V.13:12)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scyodcci", rules: &[
            "F|\"For my own OEX trading, I recommend using a 20-day lookback period. Then I smooth the daily values of the CCI with a five-day EMA.\" (V.13:12, PDF p.2)",
            "E|\"It is the crossover of the zero line by the five-day EMA that is my trading alert. This will lead to my initiating a position or closing out a position.\" (PDF p.2); short signal when \"the five-period EMA crossed below zero\" (PDF p.2)", "X|Stop and reverse on the opposite zero crossing."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scyodcci_20_5", "Young dual CCI (S&C)", "CCI 20, EMA 5")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let d = ema0(&cci_of(&x.h, &x.l, &x.c, 20), 5);
    Some(next_open(x, from, to, "reversed", |t, side| {
        if t == 0 || !d[t - 1].is_finite() { return side; }
        if d[t] > 0.0 && d[t - 1] <= 0.0 { 1.0 } else if d[t] < 0.0 && d[t - 1] >= 0.0 { -1.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dcci_reverses() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("scyodcci_20_5", &x, 0, r.len() - 1, &Params::new()).unwrap().len() > 5);
    }
}

// Instrument admission: S&C Young dual CCI 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scyodcci" && reviewed(def.id),
    label: "Jeanette Schwarz Young showed the dual CCI on the OEX (1995, chart only). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.13:12 pp.510-512 (The Commodity Channel Index Revisited); independent check authors/titles-1030/K16w/k16_ref.py",
};
