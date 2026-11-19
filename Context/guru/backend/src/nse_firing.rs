//! R4 #347 / BACKLOG Q20 (19 Nov 2026): every NSE-breadth twin card states how often its signal fires on NSE against NYSE.
//! Text only, appended to the card note; no rule, parameter or calculation changes. Rates are closed plus open trades per year
//! from private engine runs of the unchanged cards (NSE twin on NIFTY 50, the NYSE original on SPX, or SPY where SPX opens
//! start only in 2014) over the years where both series exist, else each series' own window (the line says which).
//! authors/audit-1119/R1/Q20.md holds the method and the raw counts.
use crate::connors::StrategyDef;
use std::{collections::BTreeMap, sync::OnceLock};

pub const NSE_FIRING: &[(&str, &str)] = &[
    ("agtsic_rsi8_breadth_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 2.8/yr on NSE (2002-06-21 to 2019-08-19) vs 2.7/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("apmacd_ad_stay_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.6/yr on NSE (2002-06-21 to 2019-08-19) vs 1.9/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("apsmts_ad_obos_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 3.0/yr on NSE (2002-06-21 to 2019-08-19) vs 5.7/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("apta_breadth_impulse_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 1.5/yr on NSE (2002-06-21 to 2019-08-19) vs 2.2/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("apta_breadth_macd_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 1.2/yr on NSE (2002-06-21 to 2019-08-19) vs 2.0/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("apta_nhnl_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.0/yr on NSE (2025-08-07 to 2026-11-17; the NSE new-high/new-low series holds only 1.3 years, no overlap with NYSE) vs 2.4/yr on NYSE (1991-06-10 to 2020-09-04, SPX)."),
    ("apwms_ad10_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 24.4/yr on NSE (1999-11-09 to 2020-02-10) vs 29.7/yr on NYSE (1999-11-09 to 2020-02-10, SPX)."),
    ("apwms_haurlan_int_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 23.2/yr on NSE (2002-06-21 to 2019-08-19) vs 28.9/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("apwms_haurlan_long_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 7.3/yr on NSE (2002-06-21 to 2007-12-14) vs 6.6/yr on NYSE (2002-06-21 to 2007-12-14, SPX)."),
    ("apwms_haurlan_short_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 34.8/yr on NSE (2002-06-21 to 2019-08-19) vs 34.2/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("apwms_trin5_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 32.3/yr on NSE (1999-11-09 to 2020-02-10) vs 42.4/yr on NYSE (1999-11-09 to 2020-02-10, SPX)."),
    ("apwms_updown_vol_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 22.7/yr on NSE (1999-11-09 to 2020-02-10) vs 33.3/yr on NYSE (1999-11-09 to 2020-02-10, SPX)."),
    ("ats_padi_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 43.3/yr on NSE (2002-06-21 to 2019-08-19) vs 45.5/yr on NYSE (2002-06-21 to 2019-08-19, SPY)."),
    ("ats_trin_thrust_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 148.6/yr on NSE (1999-11-09 to 2020-02-10) vs 126.2/yr on NYSE (1999-11-09 to 2020-02-10, SPY)."),
    ("ch_ntt_to_buy_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 1.1/yr on NSE (2002-06-21 to 2019-08-19) vs 0.5/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("hfm_chadtp_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 8.9/yr on NSE (2002-06-21 to 2019-08-19) vs 24.5/yr on NYSE (2002-06-21 to 2019-08-19, SPY)."),
    ("kdta_ad1018_nse", "R4 #347 firing rate (measured, private engine, trades per year): No firing rate: the card is held (no published result), so there is nothing to measure."),
    ("kdta_adline_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 101.0/yr on NSE (2002-06-21 to 2019-08-19) vs 120.1/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("kdta_adr191_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.2/yr on NSE (1999-11-09 to 2020-02-10) vs 0.2/yr on NYSE (1999-11-09 to 2020-02-10, SPX)."),
    ("kdta_arms10ma_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.5/yr on NSE (1999-11-09 to 2020-02-10) vs 0.8/yr on NYSE (1999-11-09 to 2020-02-10, SPX)."),
    ("kdta_arms265_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.6/yr on NSE (1999-11-09 to 2020-02-10) vs 0.7/yr on NYSE (1999-11-09 to 2020-02-10, SPX)."),
    ("kdta_arms_ext_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.0/yr on NSE (1999-11-09 to 2020-02-10) vs 0.0/yr on NYSE (1999-11-09 to 2020-02-10, SPX)."),
    ("kdta_haurlan200_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.0/yr on NSE (2002-06-21 to 2019-08-19) vs 0.0/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("kdta_haurlan20_nse", "R4 #347 firing rate (measured, private engine, trades per year): No firing rate: the card is held (no published result), so there is nothing to measure."),
    ("kdta_plurality_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 1.5/yr on NSE (2002-06-21 to 2019-08-19) vs 1.5/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("kdta_zweig_thrust_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.3/yr on NSE (2002-06-21 to 2019-08-19) vs 0.2/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("mbhfe_ad11_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.2/yr on NSE (2002-06-21 to 2019-08-19) vs 0.2/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("mbhfe_upvol77_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 1.7/yr on NSE (2002-06-21 to 2019-08-19) vs 0.2/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("mbsot_appel_combo_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.6/yr on NSE (2002-06-21 to 2019-08-19) vs 5.2/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("mbsot_appel_impulse_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.8/yr on NSE (2002-06-21 to 2019-08-19) vs 11.1/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("mbsot_appel_mcclellan_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.2/yr on NSE (2002-06-21 to 2019-08-19) vs 1.6/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("mbsot_mcclellan_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 12.1/yr on NSE (2002-06-21 to 2019-08-19) vs 14.7/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("mbsot_renshaw_thrust_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 1.8/yr on NSE (2002-06-21 to 2019-08-19) vs 1.3/yr on NYSE (2002-06-21 to 2019-08-19, SPY)."),
    ("mpmm_summation35_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 3.8/yr on NSE (2002-06-21 to 2019-08-19) vs 4.4/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("mzwws_ad_2to1_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.2/yr on NSE (2002-06-21 to 2019-08-19) vs 0.2/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("mzwws_double_91_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 2.3/yr on NSE (2002-06-21 to 2019-08-19) vs 1.3/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("ndbrm_ad_line_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 9.1/yr on NSE (1999-11-09 to 2026-10-06); this card has no NYSE counterpart, so no NYSE rate exists."),
    ("ndbrm_trin40_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 1.4/yr on NSE (1999-11-09 to 2020-02-10) vs 0.1/yr on NYSE (1999-11-09 to 2020-02-10, SPX)."),
    ("nfhp_quadram_ct_nse", "R4 #347 firing rate (measured, private engine, trades per year): No firing rate: the card is held (no published result), so there is nothing to measure."),
    ("nfhp_quadram_tf_nse", "R4 #347 firing rate (measured, private engine, trades per year): No NSE firing rate: the NSE run is not published (fixed point/dollar amounts of an instrument the vault does not hold); NYSE side fires 40.0/yr (2014-01-02 to 2019-08-19, SPX)."),
    ("ptjmt_padi_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 29.1/yr on NSE (2002-06-21 to 2019-08-19) vs 31.9/yr on NYSE (2002-06-21 to 2019-08-19, SPY)."),
    ("ptjmt_trin_reversals_20_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 18.0/yr on NSE (2002-06-21 to 2019-08-19) vs 18.9/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("ptjmt_trin_reversals_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 13.2/yr on NSE (2002-06-21 to 2019-08-19) vs 13.2/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("ptjmt_trin_thrusts_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 139.2/yr on NSE (1999-11-09 to 2020-02-10) vs 118.4/yr on NYSE (1999-11-09 to 2020-02-10, SPY)."),
    ("scachobs_200_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 16.0/yr on NSE (2002-06-21 to 2019-08-19) vs 38.3/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("scarms_265_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 0.4/yr on NSE (1999-11-09 to 2020-02-10) vs 0.4/yr on NYSE (1999-11-09 to 2020-02-10, SPX)."),
    ("scarmx_21_55_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 18.0/yr on NSE (1999-11-09 to 2020-02-10) vs 20.7/yr on NYSE (1999-11-09 to 2020-02-10, SPX)."),
    ("scwtrin_wtrin10_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 6.9/yr on NSE (2002-06-21 to 2019-08-19) vs 7.3/yr on NYSE (2002-06-21 to 2019-08-19, SPX)."),
    ("ss_ad_market_buster_nse", "R4 #347 firing rate (measured, private engine, trades per year): Fires 23.0/yr on NSE (2002-06-21 to 2019-08-19) vs 22.5/yr on NYSE (2002-06-21 to 2019-08-19, SPY)."),
    ("ss_arms_index_5ma_nse", "R4 #347 firing rate (measured, private engine, trades per year): No firing rate: the card is held (no published result), so there is nothing to measure."),
];

/// Appends each twin's firing-rate line to its note (leaked once per id, not per catalogue call).
pub fn apply(v: &mut [StrategyDef]) {
    static NOTES: OnceLock<BTreeMap<&'static str, &'static str>> = OnceLock::new();
    let map = NOTES.get_or_init(|| {
        let mut m = BTreeMap::new();
        for d in v.iter() {
            if let Some((_, line)) = NSE_FIRING.iter().find(|(id, _)| *id == d.id) {
                m.insert(d.id, &*Box::leak(format!("{} {}", d.notes, line).into_boxed_str()));
            }
        }
        m
    });
    for d in v.iter_mut() { if let Some(n) = map.get(d.id) { d.notes = n; } }
}

/// CV6's six NSE twins state their own rates in their own notes: no table entry, no appended line.
pub const OWN_RATE_NOTES: &[&str] = &["arebta_tt_br_nse", "arebta_ti_br_nse", "arebta_e_br_nse", "arebta_d_br_nse", "wmny_nhnl_ratio_nse", "wmny_nhnl_cci_nse"];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_nse_twin_states_its_firing_rate_once() {
        let cat = crate::connors::catalogue();
        let twins: Vec<&str> = cat.iter().filter(|d| d.id.ends_with("_nse")).map(|d| d.id).collect();
        assert!(!twins.is_empty());
        for id in &twins { if !OWN_RATE_NOTES.contains(id) { assert!(NSE_FIRING.iter().any(|(i, _)| i == id), "{id} has no firing-rate line"); } }
        for (id, _) in NSE_FIRING { assert!(twins.contains(id) && !OWN_RATE_NOTES.contains(id), "{id} is not a table twin"); }
        for d in cat.iter().filter(|d| d.id.ends_with("_nse")) {
            let n = d.notes.matches("R4 #347 firing rate").count();
            if OWN_RATE_NOTES.contains(&d.id) { assert_eq!(n, 0, "{} states its rate in its own note", d.id); assert!(d.notes.contains("a year"), "{} own note must state its rate", d.id); }
            else { assert_eq!(n, 1, "{}", d.id); }
        }
    }
}
