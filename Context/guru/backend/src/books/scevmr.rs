//! Stuart Evens, "Momentum And Relative Strength Index", Technical Analysis of Stocks & Commodities V.17:8 (August 1999)
//! pp.367-372 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C08/
//! 062MOME.pdf). Book id `scevmr`: the momentum, RSI and combined plans. Lane K16 (titles loop, 2 Nov 2026), to-code row 253;
//! independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::scevbb::ms_book;

pub const REVIEWED: &[&str] = &["scevmr_mom", "scevmr_rsi", "scevmr_both"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] 14-day lookbacks: momentum = close - close 14 days ago (the article's 'difference method' custom indicator), RSI(14) as MetaStock (Wilder smoothing; the sidebar's spreadsheet uses plain averages for the first value). The combined plan's MetaStock rules are printed (Figure 5) as level tests: enter long Mom > 0 AND RSI > 50 AND RSI < 70; close long Mom < 0 OR RSI < 50 OR RSI > 70; enter short Mom < 0 AND RSI < 50 AND RSI > 30; close short Mom > 0 OR RSI > 50 OR RSI < 30. Figures 3 and 4 (momentum and RSI plans) did not survive in the text layer; they are read the same way from the plan summaries: momentum long above 0 / short below 0 (stop and reverse); RSI enter long above 50 and below 70, close long below 50 or above 70, enter short below 50 and above 30, close short above 50 or below 30. 'The trades in our examples will be executed at the opening price on the day following the signal'; both long and short, no stops. Article: KO, CPQ, MSFT, INTC and K, 9 December 1994 - 18 May 1999 (Figure 6 table, points). The author tested stocks; runs on indices and ETFs, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "S&C: Momentum And Relative Strength Index (Evens, V.17:8)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scevmr", rules, notes: NOTE, claim: None };
    vec![
        d("scevmr_mom", "Evens momentum plan: long while 14-day momentum is above 0, short below 0, next open", &[
            "E|\"The interpretation of the momentum indicator we will use is to go long when its value rises above the zero line and to go short when dropping below the zero line.\" (V.17:8, PDF p.2)", "X|Stop and reverse: a long closes when momentum drops below zero, a short when it rises above zero (PDF p.2)."]),
        d("scevmr_rsi", "Evens RSI plan: RSI(14) above 50 (below 70) long, below 50 (above 30) short; out at 50 or at 70/30", &[
            "E|\"The 70/30 thresholds are used only to exit long and short positions while the 50 threshold is used to enter and exit.\" (PDF p.3)",
            "X|Close long: RSI falls below 50 or rises to 70 or above; close short: RSI rises above 50 or falls to 30 or less (PDF p.3)"]),
        d("scevmr_both", "Evens combined plan: momentum and RSI(14) must agree to enter, either one exits", &[
            "E|\"ENTER LONG Fml(\\\"Momentum (diff method) (14)\\\") > 0 AND RSI(14) > 50 AND RSI(14) < 70 ... ENTER SHORT Fml(...) < 0 AND RSI(14) < 50 AND RSI(14) > 30\" (PDF p.5, Figure 5)",
            "X|\"CLOSE LONG Fml(...) < 0 OR RSI(14) < 50 OR RSI(14) > 70 ... CLOSE SHORT Fml(...) > 0 OR RSI(14) > 50 OR RSI(14) < 30\" (PDF p.5)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "scevmr_mom" => "Momentum", "scevmr_rsi" => "RSI", "scevmr_both" => "Combined", _ => return None };
    Some(("scevmr", "Evens momentum and RSI plans (S&C)", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let r = rsi(&x.c, 14);
    let m: Vec<f64> = (0..x.c.len()).map(|i| if i >= 14 { x.c[i] - x.c[i - 14] } else { f64::NAN }).collect();
    let k = id.as_bytes()[7];
    Some(ms_book(x, from, to, true, |t| {
        let (q, v) = (r[t], m[t]);
        let (rl, rcl, rs, rcs) = (q > 50.0 && q < 70.0, q < 50.0 || q > 70.0, q < 50.0 && q > 30.0, q > 50.0 || q < 30.0);
        match k {
            b'm' => (v > 0.0, v < 0.0, v < 0.0, v > 0.0),
            b'r' => (rl, rcl, rs, rcs),
            _ => (v > 0.0 && rl, v < 0.0 || rcl, v < 0.0 && rs, v > 0.0 || rcs),
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn three_plans() {
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 5 && t.iter().any(|z| z.dir < 0.0), "{id}");
        }
    }
}

// Instrument admission: S&C Evens momentum and RSI 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scevmr" && reviewed(def.id),
    label: "Stuart Evens tested the three plans on five US stocks (1994-1999). Runs on indices, ETFs and other stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.17:8 pp.367-372 (Momentum And Relative Strength Index), Figure 5 MetaStock rules; independent check authors/titles-1030/K16w/k16_ref.py",
};
