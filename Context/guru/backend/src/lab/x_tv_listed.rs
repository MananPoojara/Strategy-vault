//! TradingView built-in indicators the lab did not have (2026-10-30 gap check against TradingView's built-in list).
//! Listed-only: each needs data the lab does not hold. The other TradingView fundamentals, derivatives and on-chain titles
//! were already listed in listed.rs (e.g. dividend_yield, open_interest, onchain_power_law_model); only Supply in Profit was missing.
#![allow(dead_code)]
use super::catalogue::{IndicatorDef, Needs, Placement};

/// Listed in the picker, not computed: the data it needs is not admitted (the app shows the note as "data not available").
const fn listed(id: &'static str, name: &'static str, category: &'static str, needs: Needs, definition: &'static str, note: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category, params: &[], outputs: &["value"], placement: Placement::Pane, needs, definition,
                   first: &[usize::MAX], sparse: false, compute: None, note }
}

const OCH: &str = "On-chain (crypto)";
const OC: Needs = Needs { onchain: true, ..Needs::NONE };
const OC_NOTE: &str = "not available yet — needs blockchain on-chain data (crypto only)";

pub const DEFS: &[IndicatorDef] = &[
    listed("onchain_supply_in_profit", "Supply in Profit", OCH, OC, "TradingView Supply in Profit: the number of coins currently in profit, i.e. at each day's end the sum of all unspent outputs (UTXOs) whose creation price (USD price on the day the UTXO was created) is below the current price; UTXO-model blockchains only", OC_NOTE),
];
/// (id, source, licence/notes), one per entry.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("onchain_supply_in_profit", "TradingView: Supply in Profit, https://www.tradingview.com/support/solutions/43000796657-supply-in-profit/", ""),
];

#[cfg(test)]
mod tests {
    use super::{DEFS, SOURCES};
    use crate::lab::catalogue::get;

    #[test] fn every_entry_is_listed_with_a_note_and_a_source() {
        assert_eq!(DEFS.len(), 1);
        for d in DEFS {
            assert!(d.compute.is_none(), "{}", d.id);
            assert!(d.note.starts_with("not available yet — needs "), "{}: {}", d.id, d.note);
            assert!(d.needs.any_data(), "{}", d.id);
            assert_eq!(SOURCES.iter().filter(|s| s.0 == d.id).count(), 1, "{}", d.id);
            assert_eq!(get(d.id).map(|g| g.id), Some(d.id));
        }
        assert_eq!(SOURCES.len(), DEFS.len());
    }
}
