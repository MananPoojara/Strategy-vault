//! R4 #386 (Q390 family audit, lane K4, 24 Nov 2026): families that group DISTINCT printed systems are split so each system
//! is its own method (variants of ONE system stay together). Display grouping only: no id, rule or result changes.
//! Every family listed here was source-checked (authors/conv-1123/Q390-families.tsv, column 'applied'): its members are
//! separately named / separately paged systems of one book or pack.
//! `Each`: every id is its own method. `Stem`: ids whose names share a system stem (name before '(' / ' · ' / ' w/', digits
//! and punctuation removed) stay together.
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq)]
enum Mode { Each, Stem }
const SPLIT: &[(&str, Mode)] = &[
    // R4 #387 (24 Nov 2026): re-checked after the independent audit. A family is listed only if EVERY member is a separately named /
    // separately printed system; filter, fill, stop, parameter, version and table-row variants of one system stay ONE method.
    // Reverted after the audit (grids / variants / table rows): msws_gm3, msws_gm2, msws_gm4, acbff_intraday, lwnl_commodity, lwnl_seasonal, jsats, msadd_*, r3r4, ...
    // Stem (variants of one system grouped by name stem):
    ("mstt_wow", Mode::Stem), ("kets_ma", Mode::Stem), ("kdta_breadth", Mode::Stem), ("mbsot_appel", Mode::Stem),
    // Each (every member its own system):
    ("mssta_systems", Mode::Each), ("jchr2", Mode::Each), ("msws_gm8", Mode::Each), ("mstt_equis", Mode::Each), ("khtwg", Mode::Each),
    ("apwms_other", Mode::Each), ("mbhfe_macro", Mode::Each), ("kdta_other", Mode::Each), ("capttm_other", Mode::Each), ("cktwo", Mode::Each),
    ("tdperl_other", Mode::Each), ("apsmts_other", Mode::Each), ("apoi_other", Mode::Each), ("lbi12_twelve", Mode::Each),
    ("lebcafm_daytrading", Mode::Each), ("mbsot_macro", Mode::Each), ("tkttl_setups", Mode::Each), ("jchr1_other", Mode::Each),
    ("hbqts_filters", Mode::Each), ("lebfut_futures", Mode::Each), ("jhbct_charting", Mode::Each), ("tdwb_other", Mode::Each),
    ("vsbrw_other", Mode::Each), ("mslib_misc", Mode::Each), ("mslib_hr", Mode::Each), ("cov_tfm", Mode::Each), ("lwdg2_daytrade", Mode::Each),
    ("putg_studies", Mode::Each), ("jbcdt_intraday", Mode::Each), ("hsta_stats", Mode::Each), ("tdnmtt_other", Mode::Each),
    ("maetf_alloc", Mode::Each), ("cbpst_acme", Mode::Each), ("ramtm_models", Mode::Each), ("acbff_other", Mode::Each), ("mzwws_models", Mode::Each),
    ("mpmm_interp", Mode::Each), ("cgift_nt", Mode::Each), ("dgnl_spi", Mode::Each), ("atm01_tsl", Mode::Each), ("kllien_nt", Mode::Each),
    ("pbwts_examples", Mode::Each), ("ehmd_indicators", Mode::Each), ("tdnsta_other", Mode::Each), ("mzwws_seasonal", Mode::Each),
    ("wbmdd_other", Mode::Each), ("jbcaf_nt", Mode::Each), ("dgnl_methods", Mode::Each), ("atm00_tsl", Mode::Each), ("gg_nt", Mode::Each),
    ("bstacm_nt", Mode::Each), ("mmft_boucher", Mode::Each), ("lwdg2_tools", Mode::Each),
    // Q390 re-check batch 1 (R4 #387, lane K4; authors/code-1123/Q390-recheck-1.tsv)
    ("atm04_counter", Mode::Each), ("atm10", Mode::Each), ("fotol", Mode::Each), ("jb30d_rev", Mode::Each),
    ("hsta_holiday", Mode::Each), ("lwrsr_other", Mode::Each), ("kfst_other", Mode::Each), ("mkits_fx", Mode::Each),
    ("mkits_other", Mode::Each), ("rwmts_bands", Mode::Each), ("dmst2_breakout", Mode::Each),
    // Q390 re-check batch 2 (R4 #387, lane K4; authors/code-1123/Q390-recheck-2.tsv)
    ("lwppt", Mode::Each), ("lwmil", Mode::Each), ("lwcot_cot", Mode::Each), ("rwtlc_disc", Mode::Each),
    ("mahrs_income", Mode::Each), ("lkcpt_barnes", Mode::Each), ("ndbrm_tape", Mode::Each), ("lwad", Mode::Each),
    ("dvetf_style", Mode::Each), ("jbeft_intraday", Mode::Each), ("jbeft_other", Mode::Each), ("kfgei_weekly", Mode::Each),
    ("prpr", Mode::Each), ("hkcgdt_nt", Mode::Each), ("atm03_counter", Mode::Each), ("atm04_trend", Mode::Each),
    ("ftm_km", Mode::Each), ("atmrc", Mode::Each), ("lwnl_patterns", Mode::Each),
    // Q390 re-check batch 3 (R4 #387, lane K4; authors/code-1123/Q390-recheck-3.tsv)
    ("lwnl_od", Mode::Each), ("wmny_timing", Mode::Each), ("ftm2_nd", Mode::Each), ("ftm2_nt", Mode::Each),
    ("mbbb2_bigblue2", Mode::Each), ("mslib_carayo", Mode::Each), ("csmlt_nt", Mode::Each), ("djhta_nt", Mode::Each),
    ("pvelez_nt", Mode::Each), ("epfpp_nt", Mode::Each), ("jfeot_tests", Mode::Each), ("otvt_hv", Mode::Each),
    ("cov_turtle", Mode::Each), ("hpetf_mdd", Mode::Each), ("tharp_common", Mode::Each), ("lwlts_tdw", Mode::Each),
    ("tcdt_other", Mode::Each),
    // Q390 re-check batch 4 (R4 #387, lane K4; authors/code-1123/Q390-recheck-4.tsv)
    ("jt_bollinger", Mode::Each), ("lebcafm_adx", Mode::Each), ("ehrs_indicators", Mode::Each), ("ehmtc", Mode::Each),
    ("hsta_forecast", Mode::Each), ("rgs_toolbox", Mode::Each), ("kfbpm_2sys", Mode::Each), ("dlost_pullbacks", Mode::Each),
    ("mbhfe_breadth", Mode::Each), ("mbhfe_stocks", Mode::Each), ("mbsot_timing_nt", Mode::Each), ("mkits_dax", Mode::Each),
    ("rwmts_ma", Mode::Each), ("hbqts_osc", Mode::Each),
    // Q390 re-check batch 5 (R4 #387, lane K4; authors/code-1123/Q390-recheck-5.tsv)
    ("hbiab_misc", Mode::Each), ("dmast_poly", Mode::Each), ("eltfl_tools", Mode::Each), ("lwsss", Mode::Each),
    ("gb10_or", Mode::Each), ("rwtlc_rsix", Mode::Each), ("lkcpt_bernstein", Mode::Each), ("lkcpt_jensen", Mode::Each),
    ("jsdow_composite", Mode::Each), ("mzwws_thrust", Mode::Each), ("rjmil_systems", Mode::Each), ("dvetf_sector", Mode::Each),
    ("dl10b_resumption", Mode::Each), ("kfgei_alloc", Mode::Each), ("dgdwb_darvas", Mode::Each), ("sglr_cal", Mode::Each),
    // Q390 re-check batch 6 (R4 #387, lane K4; authors/code-1123/Q390-recheck-6.tsv)
    ("cgtdd_judgement", Mode::Each), ("palets_dv", Mode::Each), ("zsms", Mode::Each), ("sbcft_filters", Mode::Each),
    ("ckbtm_lists", Mode::Each), ("phbt_surveys", Mode::Each), ("mpasi_nt", Mode::Each), ("mjzz", Mode::Each),
    ("mlhpt_bo", Mode::Each), ("cpgap_gap_fill", Mode::Each), ("atm03_trend", Mode::Each), ("atmmisc", Mode::Each),
    ("lwicw_holidays", Mode::Each), ("ltsfut", Mode::Each), ("ccl08_options", Mode::Each), ("bfmm_mm1", Mode::Each),
    ("rinadz_osc", Mode::Each), ("mslib_ma", Mode::Each), ("rrlft", Mode::Each), ("mptae_ma", Mode::Each),
    // Q390 re-check batch 7 (R4 #387, lane K4; authors/code-1123/Q390-recheck-7.tsv)
    ("fzhci_nt", Mode::Each), ("mm_nt", Mode::Each), ("onc_nt", Mode::Each), ("jjm_nt", Mode::Each),
    ("rwarms_nt", Mode::Each), ("jkennedy_nt", Mode::Each), ("gmaclean_nt", Mode::Each), ("dmpub", Mode::Each),
    ("scevkc", Mode::Each), ("mscvl_systems", Mode::Each), ("msadd_candle", Mode::Each), ("acbff_eitheror", Mode::Each),
];

/// R4 #387 variant check on the 'Each' families: ids that differ only by a number, parameter or version suffix are ONE method.
/// (ids, shown name); the method key is the first id.
const MERGE: &[(&[&str], &str)] = &[
    (&["mssta_regress_slope_1", "mssta_regress_slope_2", "mssta_regress_slope_3", "mssta_regress_slope_4"], "Regress Slope"),
    (&["mssta_ma_100", "mssta_ma_50"], "Moving average (100% / 50%)"),
    (&["mstt_tezel_maavpb", "mstt_tezel_regavpb"], "Asymmetric Volatility Price Bands (Tezel)"),
    (&["tdperl_tdma1", "tdperl_tdma2"], "TD Moving Average"),
    (&["tdperl_demarker1", "tdperl_demarker2"], "TD DeMarker"),
    // Q390 re-check batch 1
    (&["atm04_vomund", "atm04_fut_vomund"], "Volume-spike screen"),
    (&["atm10_one_pct", "atm10_fut_one_pct"], "1-percent strategy"),
    (&["jb30d_rev_buy_fpo", "jb30d_rev_sell_fpo", "jb30d_rev_buy_stop", "jb30d_rev_buy_3pc"], "Reversal-up with trigger"),
    (&["mkits_euro_ma", "mkits_euro_ma_im"], "Euro classic MA system"),
    (&["mkits_yen_vol", "mkits_yen_fut_vol"], "Yen"),
    (&["rwmts_bb_fade", "rwmts_bb200", "rwmts_bb_adx"], "Bollinger band fade with 20-day MA exit"),
    (&["dmst2_noisechannel", "dmst2_noisechannel2"], "Noise Channel Breakout"),
    // Q390 re-check batch 2
    (&["mahrs_hy_stops", "mahrs_muni_stop"], "High-yield bond fund 3% monthly buy and sell stops"),
    (&["lkcpt_box10", "lkcpt_box40"], "Box size breakout contra method"),
    (&["lkcpt_osc", "lkcpt_osc_adx"], "Oscillator method"),
    (&["ndbrm_trin40", "ndbrm_trin40_nse"], "40-day TRIN"),
    (&["ndbrm_nasdaq_ad", "ndbrm_ad_line_nse"], "NASDAQ A/D line vs its 25-day moving average"),
    (&["dvetf_style_index", "dvetf_style_weekly", "dvetf_style_bond", "dvetf_style_intl"], "Style Index rotation"),
    (&["jbeft_mom_ma", "jbeft_mom_rise_fall"], "MOM/MA system"),
    (&["kfgei_sys_a", "kfgei_sys_b", "kfgei_world_b"], "System A"),
    (&["hkcgdt_exits", "hkcgdt_300_200"], "Stop-loss, profit-target, trailing and time-stop menu"),
    (&["atm03_pullback6", "atm03_pullback_fut"], "Basic pullback buyer"),
    (&["atm04_monthly_bo", "atm04_55_trail", "atm04_55_simple"], "Monthly breakout"),
    (&["ftm_km_co_open", "ftm_km_co_limit", "ftm_km_hh_open", "ftm_km_hh_limit"], "Close-only breakout"),
    (&["atmrc_swing_stop", "atmrc_swing_trail"], "Stridsman swing pattern"),
    // Q390 re-check batch 3
    (&["lwnl_od_buy", "lwnl_od_buy3", "lwnl_od_sell"], "Down-close outside day: buy at the high of the day before, exit next day"),
    (&["lwnl_od_fade_buy", "lwnl_od_fade_sell"], "Open below a down-close outside day: buy at its low, exit on the close"),
    (&["mslib_manda_carayo", "mslib_carayooo"], "Estrategia manda carayo"),
    (&["otvt_hv_ratio_under_50", "otvt_hv_ratio_combined", "otvt_hv_ratio_combined_40min"], "HV 6/100 or 10/100 under 50%"),
    (&["cov_turtle_s2", "cov_turtle_whipsaw"], "Turtle System 2"),
    (&["hpetf_mdd_long", "hpetf_mdd_long_aggr"], "Multiple Days Down"),
    (&["hpetf_mdu_short", "hpetf_mdu_short_aggr"], "Multiple Days Up"),
    (&["lwlts_mon_oc", "lwlts_mon_3day"], "Monday open-to-close"),
    (&["lwlts_mon_vb", "lwlts_tdw_bonds"], "Monday open + 5% of Friday's range, exit close"),
    (&["tcdt_orb_stretch", "tcdt_orbp"], "ORB I"),
    // Q390 re-check batch 4
    (&["jt_bb_futures", "jt_bb_stocks", "jt_bb_stocks_portfolio"], "Bollinger Band breakout, 60 days / 2 SD, MA stop exit"),
    (&["kfbpm_2sys", "kfbpm_2sys_overview"], "2-Systems combined trend"),
    (&["mbhfe_upvol77", "mbhfe_upvol77_nse"], "Up volume > 77% of total volume"),
    (&["mbhfe_ad11", "mbhfe_ad11_nse"], "11-day advance/decline ratio > 1.9"),
    (&["mkits_dax_ma", "mkits_dax_ma_im"], "DAX MA Crossover System"),
    (&["mkits_dax_disparity", "mkits_dax_stocks"], "DAX Daily Disparity System"),
    (&["rwmts_sma200_sar", "rwmts_sma26_sar", "rwmts_sma26_wait3"], "200-day moving average stop-and-reverse"),
    // Q390 re-check batch 5
    (&["dmast_fixmempoly", "dmast_fixmempoly2_ibm5m"], "Fixed Memory Polynomial"),
    (&["rwtlc_rsix", "rwtlc_rsix_vol", "rwtlc_rsix_fx"], "RSI Extremes with 200-day MA, $7,500 stop"),
    (&["lkcpt_spop", "lkcpt_spop_rev"], "Stochastic pop"),
    (&["lkcpt_kurt03", "lkcpt_kurt10"], "Kurtosis"),
    (&["mzwws_ad_2to1", "mzwws_ad_2to1_nse"], "Ten-day Advance/Decline ratio >= 2-to-1"),
    (&["mzwws_double_91", "mzwws_double_91_nse"], "Double 9-to-1 up-volume days"),
    (&["dvetf_fidelity_sectors", "dvetf_ishares_sectors_rs"], "Style Index model on 41 Fidelity sector funds"),
    (&["dvetf_lookback_research", "dvetf_sector_1y"], "Look-back study: rotate to best 2 every 22 days"),
    (&["dgdwb_classic", "dgdwb_modern", "dgdwb_entry_tactics"], "Classic Darvas box"),
    // Q390 re-check batch 6
    (&["palets_dvpullback", "palets_dvpullback_full", "palets_dvpullback_5x20"], "DVpullback"),
    (&["zsms_zzf_zoomers", "zsms_zzf_risers"], "ZZF Zoomers"),
    (&["sbcft_tt2a", "sbcft_tt2b"], "Trading Technique 2 Plan A: z% filter with the market, trailing z% stop"),
    (&["phbt_ii_bb", "phbt_consensus_bb", "phbt_surveys_other"], "Investors Intelligence bulls, 21-week Bollinger"),
    (&["mpasi_six_stages_ma12", "mpasi_conservative_aggressive"], "Six-stage allocation from 12-month MA crossovers of bonds, stocks, commodities"),
    (&["cpgap_gap_fill_1", "cpgap_gap_fill_rank"], "Gap Fill I"),
    (&["bfmm_s_es", "bfmm_s_nq"], "MiniMax S short"),
    // Q390 re-check batch 7
    (&["dmpub_epfft5", "dmpub_epfft1"], "End Point FFT System"),
    (&["scevkc_a", "scevkc_c"], "Keltner method A: long on a close above the upper channel, short on a close below the lower"),
    (&["scevkc_b", "scevkc_d"], "Keltner method B: long on a close below the lower channel, short on a close above the upper"),
    (&["mscvl_gap1", "mscvl_gap2", "mscvl_gap3", "mscvl_gapup_5d"], "Gap1 System"),
    (&["mscvl_fosc", "mscvl_fosc_alt"], "Forecast Oscillator System"),
    (&["mscvl_tezel_ma", "mscvl_tezel_reg"], "MovAvg Asymmetric Volatility Price Bands"),
    (&["mscvl_greening_r2_1", "mscvl_greening_r2_2", "mscvl_greening_r2_3", "mscvl_greening_r2_4"], "01_R2/Regress Slope/MFI/TSF"),
    (&["msadd_n_can_9p", "msadd_o_can_9p"], "N-CAN-9P  CANDLE PATTERNS"),
    (&["msadd_n_can_bh", "msadd_o_can_bh"], "N-CAN-BH  CANDLE BELT HOLD PATTERN"),
    (&["msadd_n_can_ca", "msadd_o_can_ca"], "N-CAN-CA  CANDLE COUNTER-ATTACK PATTERN"),
    (&["msadd_n_can_ds", "msadd_o_can_ds"], "N-CAN-DS  CANDLE DOJI STAR PATTERN"),
    (&["msadd_n_can_el", "msadd_o_can_el"], "N-CAN-EL  CANDLE ENGULFING PATTERN"),
    (&["msadd_n_can_ha", "msadd_o_can_ha"], "N-CAN-HA  CANDLE HAMMER PATTERN"),
    (&["msadd_n_can_hi", "msadd_o_can_hi"], "N-CAN-HI  CANDLE HARAMI PATTERN"),
    (&["msadd_n_can_pl", "msadd_o_can_pl"], "N-CAN-PL  CANDLE PIERCING LINE PATTERN"),
    (&["msadd_n_can_ss", "msadd_o_can_ss"], "N-CAN-SS  CANDLE SHOOTING STAR PATTERN"),
    (&["msadd_n_can_st", "msadd_o_can_st"], "N-CAN-ST  CANDLE MORN/EVEN STAR PATTERN"),
    (&["acbff_three", "acbff_five", "acbff_five_4of5", "acbff_c3_124", "acbff_c3_125", "acbff_c3_134", "acbff_c3_135", "acbff_c3_145", "acbff_c3_234", "acbff_c3_235", "acbff_c3_245", "acbff_c3_345"], "First three either-or indicators combined"),
];
fn merged(id: &str) -> Option<(&'static str, &'static str)> {
    MERGE.iter().find(|(ids, _)| ids.contains(&id)).map(|(ids, name)| (ids[0], *name))
}

/// System stem of a strategy name (same normalisation as the audit: authors/conv-1123/Q390-families.tsv).
pub fn stem(name: &str) -> String {
    let n = name.to_lowercase();
    let n = n.split('(').next().unwrap_or("").split('·').next().unwrap_or("").split(" w/").next().unwrap_or("").to_string();
    let n: String = n.chars().map(|c| if c.is_ascii_digit() || ".%,$/-".contains(c) { ' ' } else { c }).collect();
    n.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn names() -> &'static HashMap<String, &'static str> {
    static M: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    M.get_or_init(|| crate::books::catalogue().into_iter().map(|d| (d.id.to_string(), d.name)).collect())
}
/// Group name of a stem group = its shortest member name (ties: alphabetical), so every member shows the same title.
fn groups() -> &'static HashMap<String, &'static str> {
    static G: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    G.get_or_init(|| {
        let mut g: HashMap<String, &'static str> = HashMap::new();
        for d in crate::books::catalogue() {
            let Some(f) = crate::books::family_of(d.id) else { continue };
            if !SPLIT.iter().any(|(fam, m)| *fam == f.0 && *m == Mode::Stem) { continue; }
            let key = format!("{}~{}", f.0, stem(d.name));
            let better = |cur: &str| (d.name.len(), d.name) < (cur.len(), cur);
            if g.get(&key).map_or(true, |cur| better(cur)) { g.insert(key, d.name); }
        }
        g
    })
}
fn leak(s: String) -> &'static str { Box::leak(s.into_boxed_str()) }

/// Apply the split to a module's (family id, family name, variant) answer for `id`.
pub fn apply(id: &str, f: (&'static str, &'static str, &'static str)) -> (&'static str, &'static str, &'static str) {
    let Some((_, mode)) = SPLIT.iter().find(|(fam, _)| *fam == f.0) else { return f };
    let Some(name) = names().get(id).copied() else { return f };
    static MEMO: std::sync::Mutex<Option<HashMap<String, (&'static str, &'static str, &'static str)>>> = std::sync::Mutex::new(None);
    if let Some(hit) = MEMO.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|m| m.get(id)) { return *hit; }
    let out = match mode {
        Mode::Each => match merged(id) { Some((key, shown)) => (key, shown, name), None => (leak(id.to_string()), name, "") },
        Mode::Stem => {
            let s = stem(name);
            let k = format!("{}~{}", f.0, if s.is_empty() { id.to_string() } else { s });
            let shown = groups().get(&k).copied().unwrap_or(name);
            (leak(k), shown, if f.2.is_empty() { name } else { f.2 })
        }
    };
    MEMO.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(Default::default).insert(id.to_string(), out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    /// (family, ids in the family, methods after the split) at 24 Nov 2026: update when ids are added to these families.
    const PINS: &[(&str, usize, usize)] = &[("mssta_systems", 38, 34), ("jchr2", 15, 15), ("msws_gm8", 15, 15), ("mstt_equis", 17, 16), ("khtwg", 18, 18), ("apwms_other", 24, 24), ("mbhfe_macro", 20, 20), ("kdta_other", 18, 18), ("capttm_other", 17, 17), ("cktwo", 13, 13), ("tdperl_other", 12, 10), ("apsmts_other", 11, 11), ("apoi_other", 12, 12), ("lbi12_twelve", 12, 12), ("lebcafm_daytrading", 11, 11), ("mbsot_macro", 11, 11), ("tkttl_setups", 11, 11), ("jchr1_other", 10, 10), ("hbqts_filters", 10, 10), ("lebfut_futures", 10, 10), ("jhbct_charting", 9, 9), ("tdwb_other", 9, 9), ("vsbrw_other", 9, 9), ("mslib_misc", 9, 9), ("mslib_hr", 9, 9), ("cov_tfm", 8, 8), ("lwdg2_daytrade", 8, 8), ("putg_studies", 8, 8), ("jbcdt_intraday", 8, 8), ("hsta_stats", 8, 8), ("tdnmtt_other", 8, 8), ("maetf_alloc", 8, 8), ("cbpst_acme", 8, 8), ("ramtm_models", 8, 8), ("acbff_other", 7, 7), ("mzwws_models", 7, 7), ("mpmm_interp", 7, 7), ("cgift_nt", 7, 7), ("dgnl_spi", 7, 7), ("atm01_tsl", 7, 7), ("kllien_nt", 12, 12), ("pbwts_examples", 6, 6), ("ehmd_indicators", 6, 6), ("tdnsta_other", 6, 6), ("mzwws_seasonal", 6, 6), ("wbmdd_other", 6, 6), ("jbcaf_nt", 6, 6), ("dgnl_methods", 6, 6), ("atm00_tsl", 6, 6), ("gg_nt", 9, 9), ("bstacm_nt", 6, 6), ("mmft_boucher", 5, 5), ("lwdg2_tools", 5, 5), ("mstt_wow", 147, 107), ("kets_ma", 52, 20), ("kdta_breadth", 16, 8), ("mbsot_appel", 8, 4), ("atm04_counter", 6, 5), ("atm10", 6, 5), ("fotol", 6, 6), ("jb30d_rev", 5, 2), ("hsta_holiday", 5, 5), ("lwrsr_other", 5, 5), ("kfst_other", 5, 5), ("mkits_fx", 5, 3), ("mkits_other", 5, 5), ("rwmts_bands", 5, 3), ("dmst2_breakout", 5, 4), ("lwppt", 5, 5), ("lwmil", 5, 5), ("lwcot_cot", 5, 5), ("rwtlc_disc", 5, 5), ("mahrs_income", 5, 4), ("lkcpt_barnes", 5, 3), ("ndbrm_tape", 5, 3), ("lwad", 5, 5), ("dvetf_style", 5, 2), ("jbeft_intraday", 5, 4), ("jbeft_other", 5, 5), ("kfgei_weekly", 5, 3), ("prpr", 5, 5), ("hkcgdt_nt", 5, 4), ("atm03_counter", 5, 4), ("atm04_trend", 5, 3), ("ftm_km", 5, 2), ("atmrc", 5, 4), ("lwnl_patterns", 5, 5), ("lwnl_od", 5, 2), ("wmny_timing", 5, 5), ("ftm2_nd", 5, 5), ("ftm2_nt", 5, 5), ("mbbb2_bigblue2", 5, 5), ("mslib_carayo", 5, 4), ("csmlt_nt", 7, 7), ("djhta_nt", 5, 5), ("pvelez_nt", 5, 5), ("epfpp_nt", 5, 5), ("jfeot_tests", 5, 5), ("otvt_hv", 4, 2), ("cov_turtle", 4, 3), ("hpetf_mdd", 4, 2), ("tharp_common", 5, 5), ("lwlts_tdw", 4, 2), ("tcdt_other", 4, 3), ("jt_bollinger", 4, 2), ("lebcafm_adx", 4, 4), ("ehrs_indicators", 4, 4), ("ehmtc", 4, 4), ("hsta_forecast", 4, 4), ("rgs_toolbox", 4, 4), ("kfbpm_2sys", 4, 3), ("dlost_pullbacks", 4, 4), ("mbhfe_breadth", 4, 2), ("mbhfe_stocks", 4, 4), ("mbsot_timing_nt", 4, 4), ("mkits_dax", 4, 2), ("rwmts_ma", 4, 2), ("hbqts_osc", 4, 4), ("hbiab_misc", 4, 4), ("dmast_poly", 4, 3), ("eltfl_tools", 4, 4), ("lwsss", 4, 4), ("gb10_or", 4, 4), ("rwtlc_rsix", 4, 2), ("lkcpt_bernstein", 4, 3), ("lkcpt_jensen", 4, 3), ("jsdow_composite", 4, 4), ("mzwws_thrust", 4, 2), ("rjmil_systems", 4, 4), ("dvetf_sector", 4, 2), ("dl10b_resumption", 4, 4), ("kfgei_alloc", 4, 4), ("dgdwb_darvas", 4, 2), ("sglr_cal", 4, 4), ("cgtdd_judgement", 4, 4), ("palets_dv", 4, 2), ("zsms", 4, 3), ("sbcft_filters", 4, 3), ("ckbtm_lists", 4, 4), ("phbt_surveys", 4, 2), ("mpasi_nt", 4, 3), ("mjzz", 4, 4), ("mlhpt_bo", 4, 4), ("cpgap_gap_fill", 4, 3), ("atm03_trend", 4, 4), ("atmmisc", 4, 4), ("lwicw_holidays", 7, 7), ("ltsfut", 4, 4), ("ccl08_options", 4, 4), ("bfmm_mm1", 4, 3), ("rinadz_osc", 4, 4), ("mslib_ma", 4, 4), ("rrlft", 4, 4), ("mptae_ma", 4, 4), ("fzhci_nt", 4, 4), ("mm_nt", 6, 6), ("onc_nt", 4, 4), ("jjm_nt", 6, 6), ("rwarms_nt", 7, 7), ("jkennedy_nt", 6, 6), ("gmaclean_nt", 4, 4), ("dmpub", 4, 3), ("scevkc", 4, 2), ("mscvl_systems", 29, 21), ("msadd_candle", 20, 10), ("acbff_eitheror", 18, 7)];
    #[test]
    fn stems_group_variants() {
        assert_eq!(stem("200 dma violated by 100%"), stem("200 dma violated by 50%"));
        assert_eq!(stem("Dual SMA crossover · next open (Test 1)"), stem("Dual SMA crossover · limit entry (Test 2)"));
        assert_ne!(stem("Gap1 System (Ton Maas)"), stem("Hammer CCI"));
    }
    /// The split is display grouping only: it never touches an id, a rule or a result. Every catalogue id still resolves a family,
    /// ids outside the split families keep exactly their module's family, ids inside get a key derived from the original family.
    #[test]
    fn split_is_grouping_only() {
        let cat = crate::books::catalogue();
        let mut seen = std::collections::HashSet::new();
        for d in &cat { assert!(seen.insert(d.id), "duplicate id {}", d.id); }
        for d in &cat {
            let Some(orig) = crate::books::family_of(d.id) else { continue };
            let now = crate::connors::family_of(d.id);
            assert_ne!(now.0, "other", "{} lost its family", d.id);
            match SPLIT.iter().find(|(f, _)| *f == orig.0) {
                None => assert_eq!(now.0, orig.0, "{} must keep its family", d.id),
                Some((f, Mode::Each)) => assert_eq!(now.0, merged(d.id).map_or(d.id, |m| m.0), "{} (family {f}) must be its own method (or its merged variant group)", d.id),
                Some((f, Mode::Stem)) => assert!(now.0.starts_with(&format!("{f}~")), "{} key {}", d.id, now.0),
            }
        }
    }
    /// Per-family method counts, so a split cannot drift silently (R4 #387).
    #[test]
    fn per_family_method_counts() {
        let cat = crate::books::catalogue();
        let mut bad = vec![];
        for (fam, ids, methods) in PINS {
            let keys: std::collections::HashSet<&str> = cat.iter().filter(|d| crate::books::family_of(d.id).map_or(false, |f| f.0 == *fam)).map(|d| crate::connors::family_of(d.id).0).collect();
            let n = cat.iter().filter(|d| crate::books::family_of(d.id).map_or(false, |f| f.0 == *fam)).count();
            if (n, keys.len()) != (*ids, *methods) { bad.push(format!("{fam}: pinned ({ids}, {methods}), found ({n}, {})", keys.len())); }
        }
        assert!(bad.is_empty(), "family counts drifted: {bad:?}");
    }
    /// Variants that differ only by a number / parameter / version suffix are ONE method (R4 #387 audit: Regress Slope 1-4 etc.).
    #[test]
    fn numbered_variants_are_one_method() {
        for (ids, _) in MERGE {
            let keys: std::collections::HashSet<&str> = ids.iter().map(|id| crate::connors::family_of(id).0).collect();
            assert_eq!(keys.len(), 1, "{ids:?} must share one method key");
            assert_eq!(*keys.iter().next().unwrap(), ids[0]);
        }
        assert_ne!(crate::connors::family_of("mssta_ma_asimetric").0, crate::connors::family_of("mssta_ma_100").0);
    }
    /// Reverted families stay ONE method (R4 #387: pattern x filter grids, fill / stop variants, table rows).
    #[test]
    fn grids_and_variants_stay_one_method() {
        let cat = crate::books::catalogue();
        for fam in ["msws_gm3", "msws_gm2", "msws_gm4", "acbff_intraday", "lwnl_commodity", "jsats", "lwnl_seasonal", "msadd_cci", "r3r4"] {
            let keys: std::collections::HashSet<&str> = cat.iter().filter(|d| crate::books::family_of(d.id).map_or(false, |f| f.0 == fam)).map(|d| crate::connors::family_of(d.id).0).collect();
            assert!(keys.len() <= 1, "family {fam} must stay one method, found {}", keys.len());
        }
    }
}
