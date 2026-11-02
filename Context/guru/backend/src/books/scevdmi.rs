//! Stuart Evens, "Directional Movement", Technical Analysis of Stocks & Commodities V.17:2 (February 1999) pp.87-94 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C02/014DIR.pdf). Book id `scevdmi`.
//! Lane K13 (titles loop, 2 Nov 2026), to-code row 203; independent check authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{dmi, next_open};

pub const REVIEWED: &[&str] = &["scevdmi_di"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Wilder's +DI(14), -DI(14), ADX(14) (Wilder smoothing, without the sidebar spreadsheet's rounding); 'ADX rising' = ADX above the day before; ADXR(14) = (ADX + ADX 14 days ago) / 2. Signals at the close, orders at the next open (the article gives no fill). Long and short as the article's examples trade both sides ('three short trades and three long trades'); long-only is an option. When an exit day also meets the other side's entry, the position is reversed ('not a stop and reverse point because the ADX(14) was falling'). Protective and trailing stops are recommended but not specified, so none is applied. The article shows chart examples only (IBP, Autodesk, International Rectifier), no test table. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scevdmi_di", name: "Evens directional movement: with ADX rising and ADXR over 25, long when +DI leads, short when -DI leads; out on the DI cross back", group: "S&C: Directional Movement (Evens, V.17:2)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scevdmi", rules: &[
            "E|\"Long: 1) +DI above -DI 2) ADX is rising 3) ADXR above 25 ... Short: 1) -DI above +DI 2) ADX rising 3) ADXR above 25\" (V.17:2 p.91)",
            "X|\"Exit: 1) When +DI crosses below -DI\" (long); \"Exit: 1) When -DI crosses below +DI\" (short) (p.91)",
            "F|\"ADXR(14) ... is simply today's ADX(14) plus the ADX(14) as of 14 days ago, divided by two.\" (p.91)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Evens directional movement (S&C)", "DI lead with ADX rising, ADXR > 25"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "side", label: "Sides (0 = long and short as printed; 1 = long only)", def: 0., min: 0., max: 1., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (pd, md, a) = dmi(x, 14);
    let both = p.get("side").copied().unwrap_or(0.0) < 0.5;
    Some(next_open(x, from, to, "DI cross back", |t, pos| {
        if t < 41 || !a[t - 14].is_finite() { return pos; }
        let mut d = pos;
        if pos > 0.0 && pd[t] < md[t] && pd[t - 1] >= md[t - 1] { d = 0.0; }
        if pos < 0.0 && md[t] < pd[t] && md[t - 1] >= pd[t - 1] { d = 0.0; }
        if d == 0.0 {
            let ok = a[t] > a[t - 1] && (a[t] + a[t - 14]) / 2.0 > 25.0;
            if pd[t] > md[t] && ok { d = 1.0; } else if md[t] > pd[t] && ok && both { d = -1.0; }
        }
        d
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn di_system_trades() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scevdmi_di", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty() && t.iter().all(|z| z.legs[0].px == x.o[z.legs[0].i]));
        let p: Params = [("side".to_string(), 1.0)].into_iter().collect();
        assert!(run("scevdmi_di", &x, 0, r.len() - 1, &p).unwrap().iter().all(|z| z.dir > 0.0));
    }
}

// Instrument admission: S&C Evens directional movement 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scevdmi" && reviewed(def.id),
    label: "Stuart Evens showed the rules on stock charts (no test table). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.17:2 pp.87-94 (Directional Movement); independent check authors/titles-1030/K13w/k13_ref.py",
};
