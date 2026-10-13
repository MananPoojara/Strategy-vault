//! Joseph T. Duffy, "Pulsar 2.0 Trading System for the Commodity Futures Markets" (11-page manual).
//! Source copy: Books/Joseph T. Duffy/Pulsar 2.0 Trading System.pdf. Notes: library/jtdpls/README.md.
//! Nothing is Live: the Impulse Pattern entry is mechanical, but its always-on stop ("300 points in the
//! S&P") and target ("30 points") use a unit the manual never defines and illustrates with no price
//! example; the Volatility module is named but its rules are not in the manual.
use crate::connors::{ParamDef, Params, Status, StrategyDef, Trade, Ctx};

const BOOK: &str = "jtdpls";
const UNIT: &str = "stop \"300 points in the S&P\" and target \"30 points\" (pdf p.9) use a unit the manual never defines and never shows with prices; the stop is always on, so the system cannot run without guessing it (300 index points vs the old 0.01 quote convention = 3.00 index points)";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, dir, status, rules: &'static [&'static str], notes| StrategyDef {
        id, name, group: "Pulsar 2.0 · Impulse Pattern", session: 0, dir, status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        make("jtdpls_impulse", "Pulsar Impulse Pattern (four-day low/high, up/down close, 2-day close stop entry)", "Long/Short",
            Status::NotTestable(UNIT),
            &["F|\"the intraday low of the latest trading day must be lower than each of the intraday lows of the previous days ... at least lower than the most recent three\" (four day low, pdf p.4, Impulse 2)",
              "F|\"an up close must be achieved. There may be more than one up close in a row. Also, the up close may be achieved on the same day as the four day low.\" (pdf p.5, Impulse 3)",
              "F|\"a down close which does not make an intraday low, lower than the original four day low. There may be more than one down close in a row.\" (pdf p.6, Impulse 4)",
              "E|\"entry is achieved by placing a buy stop at the highest close of the previous two trading days\" (pdf p.7, Impulse 5)",
              "E|\"Sell entry signals are the reverse of buy entry signals ... a four day high followed by a down close(s) followed by an up close(s) without a violation of the original four day high\" (pdf p.8, Impulse 6)",
              "X|\"Use a stop loss of 300 points in the S&P, and 20 ticks in the T-Bonds.\" (pdf p.9, Impulse 7) [unit undefined: NOT TESTABLE]",
              "X|\"Exit at the first profitable open. In other words exit the trade the first day that the opening is above the entry price.\" (pdf p.9)",
              "X|\"If the opening is below the entry price, then enter an order to exit at a target price ... entry price plus 30 points in the S&P, and the entry price plus 3 ticks in the bonds.\" Stop stays on; one cancels the other (pdf p.9)"],
            "Book markets: S&P and T-Bond futures (pdf p.3, p.9). No published results, trade lists or price examples (figures are unlabelled bar sketches). The '300 points' unit cannot be resolved from the manual's own text, so the system is not run; no substitute stop is used."),
        make("jtdpls_volatility", "Pulsar Volatility module", "Long/Short",
            Status::NotTestable("named on pdf p.3 (\"designed to trade probable impulse waves that the Impulse Pattern has not identified\") but no rules for it are in this 11-page manual"),
            &["F|\"The VOLATILITY module is designed to trade probable impulse waves that the Impulse Pattern has not identified.\" (pdf p.3) [no rules given]"],
            "Rules absent from the source; nothing to code."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "jtdpls_impulse" => Some(("jtdpls_pulsar", "Pulsar 2.0", "Impulse Pattern · stop unit undefined")),
        "jtdpls_volatility" => Some(("jtdpls_pulsar", "Pulsar 2.0", "Volatility module · rules missing")),
        _ => None,
    }
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nothing_is_live_and_ids_are_prefixed() {
        let c = catalogue();
        assert_eq!(c.len(), 2);
        assert!(c.iter().all(|d| matches!(d.status, Status::NotTestable(_)) && d.id.starts_with("jtdpls_")));
        assert!(c.iter().all(|d| family_of(d.id).is_some() && params(d.id).is_none()));
    }
}
