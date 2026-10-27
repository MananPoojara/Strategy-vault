//! Book templates (Customize spec §4): declared adaptations of admitted book methods into lab rule specs.
//! A template is a starting point for a user variant; it never changes, replaces or verifies the book method.
//! Book code is read only: the catalogue for names, status and instrument policy; `connors::run` in the fidelity tests.
use super::rules::{self, Spec};
use crate::connors::{self, Status, StrategyDef};

pub struct Template { pub method: &'static str, pub version: u32, pub spec: &'static str }
pub const TEMPLATE_LABEL: &str = "Book template · declared adaptation into lab rules · not a verification";
/// Adaptations every template declares (method-specific ones are listed with the template).
pub const COMMON_NOTES: &[&str] = &[
    "Indicators are the lab's TradingView definitions, not the book engine's implementations; edge values can differ (e.g. the lab RSI is 100 when a window has no losses and no gains, the book's Wilder RSI is 50).",
    "Lab comparisons are exact; some book engines compare with a 1e-9 relative tie band.",
    "Orders fill as the template's execution says (signal close, next open or a limit on the next bar); costs come from the lab run settings.",
    "Book-reported figures, gate verdicts and readiness are not part of a template.",
];

/// Readable label for a variant run (spec §9).
pub fn variant_label(def: &StrategyDef) -> String { format!("Customized from {} · {} · user variant · simulated", def.book, def.name) }

pub fn method(id: &str) -> Option<StrategyDef> { connors::catalogue().into_iter().find(|d| d.id == id) }
pub fn template(method: &str) -> Option<&'static Template> { TEMPLATES.iter().find(|t| t.method == method) }
/// The catalogue status the guru pages show (same rule as main.rs catalogue_json, read-only).
pub fn book_status(d: &StrategyDef) -> (&'static str, &'static str) {
    let gated = crate::gate::get(d.id);
    match d.status {
        Status::Live if gated.is_some_and(|g| g.verdict == "withheld") && !crate::instrument_policy::reviewed_cash_rule(d) => ("not_testable", gated.map_or("", |g| g.reason)),
        Status::Live if crate::instrument_policy::policy(d).requirement == "source_review_required" => ("not_testable", "Source-specific rules and required instrument have not completed admission review; no backtest is published."),
        Status::Live => ("live", ""),
        Status::NeedsData(w) => ("needs_data", w),
        Status::NotTestable(w) => ("not_testable", w),
    }
}
/// Why a method has no template ("Customize not available — <reason>"); None when it has one.
pub fn unavailable(d: &StrategyDef) -> Option<String> {
    let (status, why) = book_status(d);
    if status != "live" { return Some(format!("the book method is not admitted for backtesting ({status}): {why}")); } // checked first: a demoted method never keeps its template
    if template(d.id).is_some() { return None; }
    let named = NO_TEMPLATE.iter().find(|(id, _)| *id == d.id).map(|(_, r)| *r).or(NEW_ADMISSIONS_OCT25.contains(&d.id).then_some(NEW_ADMISSION_OCT25));
    let two_sided = (d.dir != "Long" && d.dir != "Short").then_some(TWO_SIDED);
    Some(named.or(two_sided).unwrap_or(NOT_YET).to_string())
}
pub fn parsed(t: &Template) -> Spec { rules::parse_spec(t.spec).expect("templates are checked by tests") }

pub const NOT_YET: &str = "no template has been written for this method yet";
pub const SCALE_IN: &str = "the method adds units while in a trade (scale-in), which the lab spec does not simulate yet";
pub const TWO_SIDED: &str = "the method is long and short in one strategy (always in or reversing); a lab strategy has one direction";
pub const VIX: &str = "the rules read the VIX, a second instrument; lab rules read the traded instrument only";
pub const SERIES: &str = "the rules read an external series (e.g. TRIN) that lab rules cannot reference";
pub const FUTURES: &str = "the method needs futures contract prices, which the lab does not admit";
pub const CALENDAR: &str = "the rules use calendar dates (day of month, month end, named months); the lab has no calendar operand";
pub const SUM: &str = "the rules add an indicator over several days; lab conditions compare single values";
pub const RANGE: &str = "the rules place the close within the day's range (a ratio of bar prices); lab conditions compare single values";
pub const BOOK_FILTERS: &str = "the rules use book-specific series (Liquidity Filter against the benchmark, the book's HV and ADX definitions or HV-scaled stretches)";
pub const CUSTOM: &str = "the rules use an indicator the lab catalogue does not have (e.g. a smoothed oscillator of an oscillator, MAMA, CGO)";
pub const COUNT: &str = "the rules count days in a window (e.g. 4 of the last 5); lab conditions are all-of or any-of";
pub const EXIT_ALL: &str = "the exit needs several conditions together, but lab exits fire on any one condition";
pub const INTRADAY: &str = "the method trades within the day (today's open, stop entries above the open, buy the close and sell the next open)";
pub const PORTFOLIO: &str = "the rules include portfolio settings (position caps, ranking, cash in another ETF)";
pub const FILL_CONVENTION: &str = "the book fills a limit at the limit price even when the bar opens beyond it (AmiBroker BuyPrice); the lab fills a gapped limit at the open, so a template would not reproduce the book engine";
pub const SIGNAL_CLOCK: &str = "the time exit counts bars since the latest buy signal, which repeats while in the trade (AmiBroker BarsSince); lab bars_held counts from the fill";
pub const MULTI_DAY: &str = "the entry is a multi-day setup with a pending order window, which the lab does not model";
/// Methods Manan admitted to the book engine on 2026-10-21/22 (merged into Nandini 2026-10-22) whose lab template is still to be reviewed.
pub const NEW_ADMISSION: &str = "admitted to the book engine in the 2026-10-21/22 book work; no lab template has been written or compared with the book engine yet";
/// Methods that became live in the lab when Manan's 2026-10-22/25 book work was merged into Nandini (2026-10-26); their lab template is still to be reviewed.
pub const NEW_ADMISSION_OCT25: &str = "admitted to the book engine in the 2026-10-22/25 book work; no lab template has been written or compared with the book engine yet";
pub const NEW_ADMISSIONS_OCT25: &[&str] = &[
    "price", "oiss_short_put", "oiss_short_call", "oiss_plus_three_put", "oiss_plus_three_call", "otvt_cvr1_sell_puts", "spmt_eom_puts", "btp2_10pct_oops_long",
    "btp2_10pct_oops_short", "btp2_wide_range_exhaustion_gap_short", "jr_sne_l", "jr_sne_s", "jr_bb2_l", "jr_bb2_s", "jr_123_std_l", "jr_123_std_s", "jr_hook_std_l",
    "jr_hook_std_s", "jsats_stock1", "nmwbk_penny", "ststw_cum_rsi_35", "ststw_cum_rsi_50", "ststw_vix_stretch", "lwlts_mon_gap_t500", "lwlts_mon_gap_t1000", "lwlts_mon_gap_close",
    "lwlts_mon_gap_next_close", "lwlts_mon_gap_6close", "lwlts_mon_oc", "lwlts_mon_3day", "lwlts_mon_vb", "lwlts_tdm6_sp", "lwlts_tdm7_sp", "lwlts_tdm8_sp", "lwlts_tdm19_sp",
    "lwlts_tdm20_sp", "lwlts_tdm21_sp", "lwlts_tdm22_sp", "lwlts_study_all", "lwlts_study_down1", "lwlts_study_down3", "lwlts_study_trend", "lwlts_study_trend_mon",
    "lwlts_outside_down", "lwlts_outside_down_xthu", "lwlts_3up_tue_sell", "lwlts_oops_buy", "lwlts_oops_sell_thu", "lwlts_oops_buy_ma9", "lwlts_oops_sell_ma9", "lwlts_month_day1",
    "lwlts_hh1", "lwlts_hh5", "lwlts_hh10", "lwlts_hh15", "lwlts_hh20", "lwdg2_dc_lower_open", "lwdg2_hh_gap_sell", "tcdt_ctl_p80", "tcdt_ctl_p160", "tcdt_ctl_p300",
    "tcdt_ctl_p600", "tcdt_ctl_m80", "tcdt_ctl_m160", "tcdt_ctl_m300", "tcdt_ctl_m600", "tcdt_id_p160", "tcdt_id_p80", "tcdt_id_m80", "tcdt_id_m160", "tcdt_doji_p160",
    "tcdt_doji_p80", "tcdt_doji_m80", "tcdt_doji_m160", "tcdt_nr4_p160", "tcdt_nr4_p80", "tcdt_nr4_m80", "tcdt_nr4_m160", "tcdt_dojinr4_p160", "tcdt_dojinr4_p80",
    "tcdt_dojinr4_m80", "tcdt_dojinr4_m160", "tcdt_idnr4_p160", "tcdt_idnr4_p80", "tcdt_idnr4_m80", "tcdt_idnr4_m160", "tcdt_nr7_p160", "tcdt_nr7_p80", "tcdt_nr7_m80",
    "tcdt_nr7_m160", "tcdt_ws4_p160", "tcdt_ws4_p80", "tcdt_ws4_m80", "tcdt_ws4_m160", "tcdt_ws7_p160", "tcdt_ws7_p80", "tcdt_ws7_m80", "tcdt_ws7_m160", "tcdt_bearhook_p160",
    "tcdt_bearhook_p80", "tcdt_bearhook_m80", "tcdt_bearhook_m160", "tcdt_bullhook_p160", "tcdt_bullhook_p80", "tcdt_bullhook_m80", "tcdt_bullhook_m160", "tcdt_dr1_1",
    "tcdt_dr1_2", "tcdt_dr1_3", "tcdt_dr1_4", "tcdt_dr1_5", "tcdt_dr1_6", "tcdt_dr1_7", "tcdt_dr1_8", "tcdt_dr1_9", "tcdt_2bnr_p180_h0", "tcdt_2bnr_p80_h0", "tcdt_2bnr_m80_h0",
    "tcdt_2bnr_m160_h0", "tcdt_3bnr_p160_h0", "tcdt_3bnr_p80_h0", "tcdt_3bnr_m80_h0", "tcdt_3bnr_m160_h0", "jt_bom", "jt_bb_stocks", "mha_3lr", "mha_gap2h", "mhp_qqqq_1_20050218",
    "mhp_qqqq_11_20041012", "mhp_qqqq_7_20041101", "mhp_qqqq_7_20041001", "mhp_qqqq_5_20050207", "mhp_qqqq_6_20050120", "mhp_qqqq_2_20050218", "mhp_qqqq_4_20040430",
    "mhp_qqqq_12_20041111", "mhp_qqqq_4_20041203", "mhp_spy_2_20070807", "mhp_spy_2_20070703", "mhp_spy_10_20061101", "mhp_spy_8_20070511", "mhp_spy_9_20070808",
    "mhp_spy_2_20070724", "mhp_spy_2_20070327", "mhp_spy_1_20070703", "mhp_spy_1_20070621", "mhp_spy_1_20070424", "mhp_dax_11_20050121", "mhp_dax_10_20050331",
    "mhp_dax_6_20050223", "mhp_dax_4_20050202", "mhp_ftse_3_20040712", "mhp_ftse_10_20040730", "mhp_ftse_5_20040922", "mhp_ftse_5_20041019", "mhp_ftse_12_20040917",
    "mhp_ftse_3_20041007", "mhp_ftse_12_20040930", "mhp_ftse_12_20040728", "strmm_hybrid", "strmm_hybrid_sl", "strmm_hybrid_ts", "strmm_meander", "strmm_meander_sl",
    "strmm_meander_ts", "strmm_vwa_sl", "strmm_vwa_ts", "strmm_harris_orig", "strmm_harris_sl", "strmm_harris_ts", "strmm_expert_sl", "strmm_expert_ts", "strtw_meander1_long",
    "strtw_meander1_short", "apta_ma200_dji", "apta_ma100_dji", "apta_triple_mom", "apta_roc15", "apta_six_best", "apta_six_worst", "apta_breadth_impulse",
    "apta_breadth_impulse_nse", "apta_breadth_macd", "apta_breadth_macd_nse", "apta_vix35", "apta_macd", "apmacd_neutral", "apmacd_uptrend", "apmacd_downtrend", "apmacd_ad_stay",
    "lebtc_highbeta", "jb30d_rev_buy_fpo", "jb30d_rev_buy_stop", "jb30d_rev_buy_3pc", "jb30d_sp_seasonal", "jbsmsw_mom_ma", "jbssf_sp_jan", "hsta_best6", "hsta_best6_macd",
    "hsta_best8_macd", "hsta_best6_4yr", "hsta_santa", "hsta_stpat", "hsta_midyear", "hsta_first_day", "hsta_thanksgiving", "hsta_super8", "hsta_yk_passover", "hsta_election",
    "hcta_sp_jan_long", "hcta_sp_apr_long", "hcta_sp_jul_short", "hcta_sp_oct_long", "jkbwts_rainmaker", "jkbk_call_buyer", "rgc_cdow_mon_long", "rgc_tdom17_long",
    "rgc_tdom2_short", "rgc_elliott", "rgc_ga1723", "apsmts_confirm", "apsmts_shift", "apsmts_mmacd", "apsmts_chord", "apsmts_ad_obos", "apsmts_ad_obos_nse", "apoi_pres_s1",
    "apoi_pres_s2", "lwicw_sp_tdw_mon", "lwicw_sp_tdw_tue", "lwicw_sp_tdw_wed", "lwicw_sp_tdw_thu", "lwicw_sp_tdw_fri", "lwicw_sp_tdom", "lwicw_oops_tdom_buy",
    "lwicw_oops_tdom_sell", "lwicw_mon_gap_bail", "lwmdc_down_smaller", "lwmdc_down_larger", "lwmdc_up_larger", "lwmdc_up_smaller", "lwmdc_up_sell_smaller", "lwmdc_up_sell_larger",
    "lwmdc_2down", "lwmdc_zone", "lwmdc_od_up_oc", "lwmdc_od_down_oc", "lwmdc_nonod_up_oc", "lwmdc_nonod_down_oc", "lwmdc_od_oops_buy", "lwmdc_od_oops_sell", "lwmdc_od_high_buy",
    "lwmdc_od_prior_high_buy", "lwmdc_open_up_oc", "lwmdc_open_down_oc", "lwmdc_fdom_oc", "lwmdc_fdom_month", "lwmdc_thanksgiving", "lwpro_after_up", "lwpro_after_down",
    "lwpro_open_up", "lwpro_open_down", "tdnsta_seq_long", "tdnsta_seq_short", "tdnmtt_seq_long", "tdnmtt_seq_short", "tdnmtt_combo1_long", "tdnmtt_combo1_short",
    "tdnmtt_combo2_long", "tdnmtt_combo2_short", "tdnmtt_trap_long", "tdnmtt_trap_short", "mbhfe_upvol77", "mbhfe_upvol77_nse", "mbhfe_ad11", "mbhfe_ad11_nse", "mbsot_zweig4",
    "mbsot_renshaw_thrust", "mbsot_renshaw_thrust_nse", "mbsot_appel_impulse", "mbsot_appel_impulse_nse", "mbsot_appel_mcclellan", "mbsot_appel_mcclellan_nse", "mbsot_appel_combo",
    "mbsot_appel_combo_nse", "mkfve_div", "hbqts_ma3_20", "hbqts_stop_entry", "hbqts_scale_in", "hbqts_scale_out", "hbqts_parabolic_stop", "hbqts_chandelier", "hbqts_ma6_35",
    "hbqts_ma45_26", "hbqts_apz", "hbqts_tdom", "hbqts_pattern111", "hbqts_common_signal", "hbqts_ma8_7", "hbqts_weekly_pir", "hbqts_macd_weekly", "hbqts_noise0",
    "hbqts_ma8_7_atr_stop", "hbqts_equity_alloc", "hbqts_ma11_50", "hbiab_trend_following", "hbiab_mean_reversion", "eltfl_sroc", "lwic6_pctr_ma50_long", "lwcot_vol_p5up_v5up",
    "lwcot_vol_p5up_v5dn", "lwcot_vol_dn_vdn", "lwcot_vol_dn_vup", "lwcot_vol_t50dn_p8up_v8up", "lwcot_vol_t50dn_p8up_v8dn", "lwcot_vol_t50dn_p8dn_v8up",
    "lwcot_vol_t50dn_p8dn_v8dn", "lwcot_down_closes_daytrade", "lwcot_every_day_bail", "pstc_rs2", "pstc_r3_long", "pstc_r3_short", "pstc_rsi20_long", "pstc_rsi20_short",
    "pstc_r4_long", "pstc_r4_short", "pstc_4_10_60", "prap2_raptor2", "acatr_kuo_gma21", "kpss_first5", "kpss_last5", "kpss_janbar", "kpss_jbs", "kpss_jaynewary", "kpss_hol_d1",
    "kpss_hol_d2", "kpss_hol_3x3", "kpss_hol_top24", "kpss_uhts", "kpss_xmas_week", "kpss_td2", "kpss_td1_5", "kpss_mid4", "kpss_worst14", "kpss_nonworst", "kpss_six",
    "kpss_monthly10", "kpss_santa", "kpss_m10_santa", "kpss_umds", "kpss_uumds", "kpss_dec_b", "kpss_dec_c", "kpss_middecade", "kpss_latedecade", "kpss_20yr", "kpss_intradecade",
    "kpss_c212", "kpss_c40w", "kpss_c53d", "kpss_ctcm1", "kpss_ctcm2", "kpss_utcm", "kpss_yr_post", "kpss_yr_mid", "kpss_yr_pre", "kpss_yr_elec", "kpss_yr_post_mid",
    "kpss_yr_pre_elec", "kpss_stoken15", "kpss_fav14", "kpss_fav21", "kpss_midterm", "kpss_pre_mar_jul", "kpss_uecs", "kpss_nov_may", "kpss_ndx_nov_jun", "kpss_jja", "kpss_sept",
    "kpss_ex_sept", "kpss_oct7", "kpss_kti_long", "kpss_lopl", "mahrs_covered_calls", "lkqts_dom1", "lkqts_dom2", "lkqts_vixscore", "lkqts_gross_vix", "lkqts_m_sma12",
    "lkqts_m_sma6", "lkqts_m_sma3", "lkqts_m_mom12", "lkqts_m_mom6", "lkqts_m_mom3", "nfpk_ma20w", "nfpk_ma20w_f1", "nfpk_wma15w", "nfpk_wma15w_swing", "ndbrm_spx_reversals",
    "ndbrm_trin40", "ndbrm_trin40_nse", "ndbrm_ad_line_nse", "svmws_10w30w_long", "svmws_10w30w_short", "svtvc_ma150_t1", "svtvc_ma200_t1", "svtvc_ma250_t1", "svtvc_ma150_t2",
    "svtvc_ma200_t2", "svtvc_ma250_t2", "pstc08_r4", "pstc08_r5", "pstc08_meteors", "pstc08_meteors5", "pstc08_rsi25_20", "pstc08_rsi25_20_noma", "tdwb_tdline_fade_short",
    "tdwb_tdline_fade_long", "jbcdt2_dates_oc", "jbcdt2_preholiday_oc", "jbcdt2_mon_cc", "jbhsms_sp_0113", "jbhsms_sp_0226", "jbhsms_sp_0301", "jbhsms_sp_0414", "jbhsms_sp_0601",
    "jbhsms_sp_0710", "jbhsms_sp_0815", "jbhsms_sp_1220", "jbhsms_dj_0405", "jbhsms_dj_1117", "jbhsms_dj_1203", "jbhsms_dj_0218", "jbhsms_dj_0313", "jbhsms_dj_0407",
    "jbhsms_dj_0528", "jbhsms_stk_aa", "jbhsms_stk_ko", "jbhsms_stk_ge", "jbhsms_stk_msft", "jbhsms_stk_pg", "jbhsms_stk_txn", "jbtywr_dj_1215", "jbtywr_dj_0406",
    "jbtywr_crude_outside", "jbhpp_dj_1026", "kfgei_sys_a", "kfgei_sys_b", "kfgei_sys_e", "kfgei_uc", "lebws_atr_abc", "dgdwb_modern", "sglr_dow_ma200_band",
    "sglr_nasdaq_ma200_band", "lydetf_ema200", "mpts_ma2d_long", "mpts_ma10m_long", "mpmm_cm_rsi50_21w", "mpmm_cm_wma69w", "mpmm_cm_ema42w", "mpmm_cm_sma45w", "mpmm_gartley_25a3",
    "mpmm_cm_15_36w", "mpmm_kst_lt", "mpmm_summation35", "mpmm_summation35_nse", "mpmm_dev26_40w", "mpmm_dev26_40w_adv10", "mpmm_kst_wk", "mpmm_roc12_dir", "kdta_vix_ema",
    "kdta_rsi_wrong_long", "kdta_rsi_wrong_short", "kdta_apple_v2", "kdta_pullback6", "kdta_adr191", "kdta_zweig_thrust", "kdta_haurlan200", "kdta_arms265", "kdta_arms_ext",
    "kdta_arms10ma", "kdta_adr191_nse", "kdta_zweig_thrust_nse", "kdta_haurlan200_nse", "kdta_arms265_nse", "kdta_arms_ext_nse", "kdta_arms10ma_nse", "phbt_vix45_weekly",
    "phbt_accel_daily", "phbt_accel_weekly", "phbt_accel_monthly", "lydpes_ma200_8pct", "mpasi_sp_ma12", "capttm_ma14_50", "ddrvi_inertia50", "mlhpt_bo10", "lwnl_jan_rally",
    "lwnl_mar_short", "lwnl_oct_td9_buy", "lwnl_oct_td17_sell", "lwnl_nov_td17_db7", "lwnl_jan97_td16_buy", "lwnl_jan97_td2_sell", "lwnl_jul97_td4_buy", "lwnl_jul97_td17_buy",
    "lwnl_jul97_td16_buy_30d", "lwnl_jul97_td14_sell", "lwnl_aug97_td12_buy_bail", "lwnl_aug97_td18_buy_bail", "lwnl_aug97_td7_sell_bail", "lwnl_aug97_td18_buy_7d",
    "lwnl_aug97_td2_sell_7d", "lwnl_nov97_td17_buy_7d", "lwnl_nov97_td18_buy_7d", "lwnl_nov97_td6_buy_30d", "lwnl_nov97_td15_buy_30d", "lwnl_dec97_td2_buy_1d",
    "lwnl_dec97_td14_buy_1d", "lwnl_dec97_td9_sell_1d", "lwnl_yearend_sell", "lwnl_tg_close", "lwnl_tg_open8", "lwnl_tg_oc", "lwnl_tg_open3_bail", "lwnl_xmas_bail",
    "lwnl_xmas_bail9", "lwnl_memorial_bail", "lwnl_memorial_bail6", "lwnl_july4_buy", "lwnl_pullback", "lwnl_krut_buy", "lwnl_krut_sell", "lwnl_od_buy", "lwnl_od_buy3",
    "lwnl_od_sell", "lwnl_od_fade_buy", "lwnl_od_fade_sell", "lwnl_engulf_bull_1", "lwnl_engulf_bull_5", "lwnl_engulf_bull_10", "lwnl_engulf_bear_1", "lwnl_engulf_bear_5",
    "lwnl_engulf_bear_10", "lwnl_keyrev_buy", "lwnl_proj_limit", "lwnl_truth1_closehigh", "lwnl_tdw_oc_tue", "lwnl_tdw_oc_wed", "lwnl_tdw_oc_thu", "lwnl_tdw_oc_fri",
    "msws_positive_vol_crossover", "lwinb_inside_bar", "lwfc10_tdlm", "lwfc10_tdoy", "lwfc10_midterm", "ccl08_weekly_rsi_options", "bfmm_esl", "bfmm_nql", "bfmm_s_es",
    "rinadz_rsi14s", "rinadz_pctr", "rinadz_xmacd", "phbttk_pctr_long", "phbttk_pctr_short", "mptae_ma40w", "mptae_1030w", "mptae_russell", "mptae_tw9m", "mptae_coppock",
    "mpsel_kst_lt", "ggth_dmaco", "ggth_dmaco_es", "fgitc_rocadx", "fgitc_qqqvol", "fgitc_ema1020", "fgsmc_ma50_200", "fgtofd_ema20_50", "apitc_ma50", "cbsiws_5050",
];
pub const NO_TEMPLATE: &[(&str, &str)] = &[
    ("spmt_vix200", VIX), ("spmt_vix2", VIX), ("spmt_vix4", VIX), ("spmt_vix7", VIX), ("spmt_vix8", VIX), ("spmt_short_vix53", VIX), ("spmt_short_vix101", VIX), ("ststw_vix_rsi", VIX),
    ("spmt_trin_1", SERIES), ("spmt_trin_2", SERIES), ("spmt_trin_3", SERIES), ("spmt_trin_4", SERIES), ("spmt_trin_5", SERIES), ("spmt_nyse_adv", SERIES),
    ("spmt_eom_und", CALENDAR),
    ("ccl08_tps_long", SCALE_IN), ("ccl08_tps_short", SCALE_IN), ("ccl08_tps_research", SCALE_IN), ("hpetf_sem_rsi14_tps", SCALE_IN), ("r3r4_lowprice_scale", SCALE_IN),
    ("ss_long_2nd", NEW_ADMISSION), ("ss_short_2nd", NEW_ADMISSION), ("ss_study_close_upper_90", NEW_ADMISSION), ("ss_study_close_upper_80", NEW_ADMISSION), ("ss_study_close_lower_10", NEW_ADMISSION), ("ss_study_close_lower_20", NEW_ADMISSION),
    ("ss_study_wr7_high_close", NEW_ADMISSION), ("ss_study_wr7_low_close", NEW_ADMISSION), ("ss_study_rocrsi_buy", NEW_ADMISSION), ("ss_study_rocrsi_sell", NEW_ADMISSION), ("ss_study_fresh_roc_buy", NEW_ADMISSION), ("ss_study_fresh_roc_sell", NEW_ADMISSION),
    ("ss_study_oops_buy", NEW_ADMISSION), ("ss_study_oops_sell", NEW_ADMISSION), ("ss_study_oops_adx_gapper_buy", NEW_ADMISSION), ("ss_study_oops_adx_gapper_sell", NEW_ADMISSION), ("ss_study_gap_failure_buy", NEW_ADMISSION), ("ss_study_gap_failure_sell", NEW_ADMISSION),
    ("otvt_cvr1_long", NEW_ADMISSION), ("otvt_cvr1_short", NEW_ADMISSION), ("btp_dvmtm_short", NEW_ADMISSION), ("btp2_smtp_long", NEW_ADMISSION), ("btp2_smtp_short", NEW_ADMISSION), ("btp2_1234_long", NEW_ADMISSION),
    ("btp2_1234_short", NEW_ADMISSION), ("btp2_8day_hl_reversal_long", NEW_ADMISSION), ("btp2_8day_hl_reversal_short", NEW_ADMISSION), ("btp2_momentum_ma_long", NEW_ADMISSION), ("btp2_momentum_ma_short", NEW_ADMISSION), ("btp2_gipsons_long", NEW_ADMISSION),
    ("btp2_gipsons_short", NEW_ADMISSION), ("btp2_wide_range_exhaustion_gap_long", NEW_ADMISSION), ("alpha_trend_6m", NEW_ADMISSION), ("afactor_spx_down", NEW_ADMISSION), ("r3r4_lowprice", NEW_ADMISSION), ("hbqts_dema16_8", NEW_ADMISSION),
    ("hbiab_ma5_20", NEW_ADMISSION), ("hbiab_ma7_4", NEW_ADMISSION), ("hbiab_macd_example", NEW_ADMISSION), ("hbiab_simple_ma25", NEW_ADMISSION), ("ccl08_crdt1_long", NEW_ADMISSION), ("ccl08_crdt1_short", NEW_ADMISSION),
    ("ccl08_crdt2_long", NEW_ADMISSION), ("ccl08_crdt3_long", NEW_ADMISSION), ("ccl08_crdt4_long", NEW_ADMISSION), ("ccl08_crdt5_long", NEW_ADMISSION), ("ccl08_crdt2_short", NEW_ADMISSION), ("ccl08_crdt4_short", NEW_ADMISSION),
    ("ccl08_crdt5_short", NEW_ADMISSION),
    ("radge_oc_long", "the admitted Radge stop-free open-to-close cash-index research rule has no source-specific lab template or template-fidelity comparison yet"),
    ("sbma_sma50_long", "the admitted Burns 50-day SMA long/cash stock rule has no source-specific lab template or template-fidelity comparison yet; strict above/below ties and reviewed fill conventions must be preserved"),
    ("sbma_x_5s20s", "the admitted Burns 5/20-day SMA long/cash stock crossover has no source-specific lab template or template-fidelity comparison yet; strict crossover ties and reviewed fill conventions must be preserved"),
    ("sbma_x_5e50s", "the admitted Burns 5-day EMA / 50-day SMA long/cash stock crossover has no source-specific lab template or template-fidelity comparison yet; EMA seeding, crossover ties and reviewed fill conventions must be preserved"),
    ("sbma_x_10e20e", "the admitted Burns 10/20-day EMA long/cash stock crossover has no source-specific lab template or template-fidelity comparison yet; EMA seeding, crossover ties and reviewed fill conventions must be preserved"),
    ("sbma_x_10e50e", "the admitted Burns 10/50-day EMA long/cash stock crossover has no source-specific lab template or template-fidelity comparison yet; EMA seeding, crossover ties and reviewed fill conventions must be preserved"),
    ("sbma_x_15e30e", "the admitted Burns 15/30-day EMA long/cash stock crossover has no source-specific lab template or template-fidelity comparison yet; EMA seeding, crossover ties and reviewed fill conventions must be preserved"),
    ("sbma_x_15e150e", "the admitted Burns 15/150-day EMA long/cash stock crossover has no source-specific lab template or template-fidelity comparison yet; EMA seeding, crossover ties and reviewed fill conventions must be preserved"),
    ("sbma_x_10s200s", "the admitted Burns 10/200-day SMA long/cash stock crossover has no source-specific lab template or template-fidelity comparison yet; strict crossover ties and reviewed fill conventions must be preserved"),
    ("sbma_x_50s200s", "the admitted Burns 50/200-day SMA long/cash stock crossover has no source-specific lab template or template-fidelity comparison yet; strict crossover ties and reviewed fill conventions must be preserved"),
    ("r3", SCALE_IN), ("tps_stocks", SCALE_IN), ("tps_long", SCALE_IN), ("tps_short", SCALE_IN), ("s12345", SCALE_IN),
    ("x555", BOOK_FILTERS), ("h10", BOOK_FILTERS),
    ("eom_co", CALENDAR), ("eom_rsi", CALENDAR), ("eom_cc", CALENDAR), ("spmt_eom_co", CALENDAR), ("spmt_eom_rsi", CALENDAR), ("spmt_eom_cc", CALENDAR),
    ("rsi_1", SUM), ("rsi_2", SUM), ("rsi_3", SUM), ("spmt_rsi_1", SUM), ("spmt_rsi_2", SUM), ("spmt_rsi_3", SUM),
    ("pctb_2", RANGE), ("spmt_pctb_2", RANGE), ("spmt_price", RANGE),
    ("vix200", VIX), ("vix2", VIX), ("vix4", VIX), ("vix7", VIX), ("vix8", VIX), ("vix53_short", VIX), ("vix101_short", VIX),
    ("r25_rsi25_vix", VIX), ("r25_rsi25_15", SCALE_IN), ("r25_rsi25_15_vix", VIX),
    ("ss_long", SCALE_IN), ("ss_short", SCALE_IN), ("ss_both", TWO_SIDED),
    ("trin4", SERIES), ("trin4_12", SERIES), ("trin3_10", SERIES), ("trin3_11", SERIES), ("trin2_11", SERIES),
    ("ats_dvmtm", MULTI_DAY), ("pbs_percent_b_swing", TWO_SIDED), ("pbs_percent_b_plus_one", TWO_SIDED),
    ("alpha_spy_rsi4_mr", SCALE_IN), ("alpha_etf_avalanches_single", PORTFOLIO), ("radge_ma220", FUTURES),
    ("ststw_overnight", INTRADAY), ("ststw_double7", EXIT_ALL),
    ("hpetf_3dhl_long_aggr", SCALE_IN), ("hpetf_rsi25_long_aggr", SCALE_IN), ("hpetf_r3_long_aggr", SCALE_IN), ("hpetf_pctb_long_aggr", SCALE_IN),
    ("hpetf_pctb_short_aggr", SCALE_IN), ("hpetf_mdd_long", COUNT), ("hpetf_mdd_long_aggr", SCALE_IN), ("hpetf_mdu_short", COUNT),
    ("hpetf_mdu_short_aggr", SCALE_IN), ("hpetf_rsi10_6_long", SCALE_IN), ("hpetf_rsi90_94_short", SCALE_IN), ("hpetf_tps_long", SCALE_IN), ("hpetf_tps_short", SCALE_IN),
    ("afactor_baseline", INTRADAY), ("r3r4_base", BOOK_FILTERS), ("r3r4_base_scale", SCALE_IN), ("r3r4_bigcap", BOOK_FILTERS), ("r3r4_bigcap_scale", SCALE_IN),
    ("r3r4_bigcap_deck", BOOK_FILTERS), ("r3r4_bigcap_deck_scale", SCALE_IN), ("r3r4_midcap", BOOK_FILTERS), ("r3r4_midcap_scale", SCALE_IN),
    ("mmft_nasdaq79", TWO_SIDED), ("lwdg2_vol_up_up", FUTURES), ("lwdg2_vol_dn_up", FUTURES), ("lwdg2_vol_up_dn", FUTURES), ("lwdg2_vol_dn_dn", FUTURES), ("lwdg2_weekly_entry", FUTURES),
    ("kftsm_month_end", CALENDAR), ("kftsm_hirsch", CALENDAR), ("lwrsr_oct_apr", CALENDAR), ("lwrsr_apr_oct", CALENDAR),
    ("ehmd_mama", CUSTOM), ("ch_cbpb_time", CUSTOM),
    ("hbqts_fixed_interval", CUSTOM), ("hbqts_bb_breakout", TWO_SIDED), ("hbqts_cgo", CUSTOM), ("hbqts_stoch_rsi", CUSTOM), ("hbqts_dpo", CUSTOM),
    ("hbqts_dom", TWO_SIDED), ("hbqts_limit_entry", FILL_CONVENTION), ("hbqts_entry_id1", SIGNAL_CLOCK), ("hbqts_macd_daily", CUSTOM), ("hbqts_cmo", CUSTOM),
    ("kfsc_two_ma", "the admitted Kaufman two-moving-average long/cash stock rule (selected 80/10 SMA example, exit when either SMA turns down, equality holds) has no source-specific lab template or template-fidelity comparison yet; SMA slope ties and reviewed fill conventions must be preserved"),
];

pub const TEMPLATES: &[Template] = &[
    Template { method: "f1a", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"f1a","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"fill":"close"}}"# },
    Template { method: "f3a", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"f3a","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":4}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":5}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":6}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":7}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":8}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":9}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"fill":"close"}}"# },
    Template { method: "f4a", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"f4a","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"low"},"op":"<=","rhs":{"s":"lowest","p":{"length":10}}}],"limit":{"pct":3}},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"exit_fill":"close"}}"# },
    Template { method: "f4c", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"f4c","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"low"},"op":"<=","rhs":{"s":"lowest","p":{"length":10}}}],"limit":{"pct":7}},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"exit_fill":"close"}}"# },
    Template { method: "f4d", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"f4d","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"low"},"op":"<=","rhs":{"s":"lowest","p":{"length":10}}}],"limit":{"pct":7}},"exit":{"any":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"exit_fill":"close"}}"# },
    Template { method: "rsi2_5d", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"rsi2_5d","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":2}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "rsi2_4", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"rsi2_4","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":4}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "rsi4_25", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"rsi4_25","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":4}},"op":"<","rhs":{"value":25}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "rsi_div8", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"rsi_div8","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":"<","rhs":{"s":"close","offset":8}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"s":"rsi","p":{"length":2},"offset":8}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":15}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "rsi_div2", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"rsi_div2","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":"<","rhs":{"s":"close","offset":2}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"s":"rsi","p":{"length":2},"offset":2}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":30}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "pctb_1", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"pctb_1","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"bbpct","p":{"length":5,"mult":1}},"op":"<","rhs":{"value":0}},{"lhs":{"s":"bbpct","p":{"length":5,"mult":1},"offset":1},"op":"<","rhs":{"value":0}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "dmi_1", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"dmi_1","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":3},"out":"plus_di"},"op":"<","rhs":{"value":5}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "dmi_2", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"dmi_2","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":3}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "dmi_3", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"dmi_3","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":10}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":1},"op":"<","rhs":{"value":10}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "dmi_4", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"dmi_4","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":10}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":1},"op":"<","rhs":{"value":10}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":2},"op":"<","rhs":{"value":10}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "dmi_5", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"stc9","method":"dmi_5","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":15}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":1},"op":"<","rhs":{"value":15}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":2},"op":"<","rhs":{"value":15}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":3},"op":"<","rhs":{"value":15}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "short_4up", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"stc9","method":"short_4up","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close","offset":1},"op":">","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close","offset":2},"op":">","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close","offset":3},"op":">","rhs":{"s":"close","offset":4}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "short_strong", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"stc9","method":"short_strong","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"roc","p":{"length":2}},"op":">","rhs":{"value":3}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":80}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_rsi2_4", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_rsi2_4","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":4}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_rsi4_25", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_rsi4_25","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":4}},"op":"<","rhs":{"value":25}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_rsi_div8", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_rsi_div8","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":"<","rhs":{"s":"close","offset":8}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"s":"rsi","p":{"length":2},"offset":8}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":15}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_rsi_div2", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_rsi_div2","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":"<","rhs":{"s":"close","offset":2}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"s":"rsi","p":{"length":2},"offset":2}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":30}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_pctb_1", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_pctb_1","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"bbpct","p":{"length":5,"mult":1}},"op":"<","rhs":{"value":0}},{"lhs":{"s":"bbpct","p":{"length":5,"mult":1},"offset":1},"op":"<","rhs":{"value":0}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_dmi_1", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_dmi_1","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":3},"out":"plus_di"},"op":"<","rhs":{"value":5}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_dmi_2", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_dmi_2","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":3}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_dmi_3", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_dmi_3","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":10}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":1},"op":"<","rhs":{"value":10}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_dmi_4", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_dmi_4","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":10}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":1},"op":"<","rhs":{"value":10}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":2},"op":"<","rhs":{"value":10}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_dmi_5", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"spmt","method":"spmt_dmi_5","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di"},"op":"<","rhs":{"value":15}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":1},"op":"<","rhs":{"value":15}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":2},"op":"<","rhs":{"value":15}},{"lhs":{"s":"dmi","p":{"di_length":2},"out":"plus_di","offset":3},"op":"<","rhs":{"value":15}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<=","rhs":{"value":65}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":65}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_short_4up", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"spmt","method":"spmt_short_4up","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close","offset":1},"op":">","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close","offset":2},"op":">","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close","offset":3},"op":">","rhs":{"s":"close","offset":4}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "spmt_short_strong", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"spmt","method":"spmt_short_strong","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"roc","p":{"length":2}},"op":">","rhs":{"value":3}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":80}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "r25_rsi25_basic_no_filter", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"rsi25","method":"r25_rsi25_basic_no_filter","template_version":1},"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":4}},"op":"<","rhs":{"value":25}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":4}},"op":">","rhs":{"value":55}}]},"execution":{"fill":"close"}}"# },
    Template { method: "r25_rsi25_official_sma200", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"rsi25","method":"r25_rsi25_official_sma200","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":4}},"op":"<","rhs":{"value":25}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":4}},"op":">","rhs":{"value":55}}]},"execution":{"fill":"close"}}"# },
    Template { method: "ststw_10d_high_buy", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"ststw","method":"ststw_10d_high_buy","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":4}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":5}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":6}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":7}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":8}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":9}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":10}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "ststw_10d_high_short", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"ststw","method":"ststw_10d_high_short","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":4}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":5}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":6}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":7}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":8}},{"lhs":{"s":"close"},"op":">=","rhs":{"s":"close","offset":9}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":10}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "ststw_10d_low_buy", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"ststw","method":"ststw_10d_low_buy","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":4}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":5}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":6}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":7}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":8}},{"lhs":{"s":"close"},"op":"<=","rhs":{"s":"close","offset":9}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":10}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "ststw_rsi2_under5", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"ststw","method":"ststw_rsi2_under5","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":5}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "ststw_sp_short", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"ststw","method":"ststw_sp_short","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","offset":1}},{"lhs":{"s":"close","offset":1},"op":">","rhs":{"s":"close","offset":2}},{"lhs":{"s":"close","offset":2},"op":">","rhs":{"s":"close","offset":3}},{"lhs":{"s":"close","offset":3},"op":">","rhs":{"s":"close","offset":4}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "hpetf_3dhl_long", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"hpetf","method":"hpetf_3dhl_long","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":5}}},{"lhs":{"s":"high"},"op":"<","rhs":{"s":"high","offset":1}},{"lhs":{"s":"low"},"op":"<","rhs":{"s":"low","offset":1}},{"lhs":{"s":"high","offset":1},"op":"<","rhs":{"s":"high","offset":2}},{"lhs":{"s":"low","offset":1},"op":"<","rhs":{"s":"low","offset":2}},{"lhs":{"s":"high","offset":2},"op":"<","rhs":{"s":"high","offset":3}},{"lhs":{"s":"low","offset":2},"op":"<","rhs":{"s":"low","offset":3}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "hpetf_rsi75_short", version: 1, spec: r#"{"v":1,"direction":"short","origin":{"book":"hpetf","method":"hpetf_rsi75_short","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":4}},"op":">","rhs":{"value":75}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":4}},"op":"<","rhs":{"value":45}}]},"execution":{"fill":"close"}}"# },
    Template { method: "hpetf_r3_long", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"hpetf","method":"hpetf_r3_long","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"s":"rsi","p":{"length":2},"offset":1}},{"lhs":{"s":"rsi","p":{"length":2},"offset":1},"op":"<","rhs":{"s":"rsi","p":{"length":2},"offset":2}},{"lhs":{"s":"rsi","p":{"length":2},"offset":2},"op":"<","rhs":{"s":"rsi","p":{"length":2},"offset":3}},{"lhs":{"s":"rsi","p":{"length":2},"offset":2},"op":"<","rhs":{"value":60}},{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":70}}]},"execution":{"fill":"close"}}"# },
    Template { method: "hbqts_ma5_25", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"hbqts","method":"hbqts_ma5_25","template_version":1},"entry":{"all":[{"lhs":{"s":"sma","p":{"length":5}},"op":"crosses_above","rhs":{"s":"sma","p":{"length":25}}}]},"exit":{"any":[{"lhs":{"s":"sma","p":{"length":5}},"op":"crosses_below","rhs":{"s":"sma","p":{"length":25}}}]},"execution":{"fill":"close"}}"# },
    Template { method: "hbqts_profit_target", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"hbqts","method":"hbqts_profit_target","template_version":1},"entry":{"all":[{"lhs":{"s":"sma","p":{"length":5}},"op":"crosses_above","rhs":{"s":"sma","p":{"length":25}}}]},"exit":{"any":[],"target":{"pct":10}},"execution":{"fill":"close"}}"# },
    Template { method: "hbqts_ma1_36", version: 1, spec: r#"{"v":1,"direction":"long","origin":{"book":"hbqts","method":"hbqts_ma1_36","template_version":1},"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":36}}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":36}}}]},"execution":{"fill":"close"}}"# },
];
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::bars::Bars;
    use crate::lab::fidelity::compare;
    use crate::lab::series::{compile, Cache, Frames};
    use crate::lab::simulate::simulate;
    use crate::{connors::{Ctx, Params}, Row};

    /// Business-like dates (monotonic, valid) and a random walk with a slow trend cycle so trend filters flip both ways.
    fn fixture(n: usize, seed: u64, vol: f64) -> &'static [Row] {
        let mut s = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let mut next = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((s >> 11) as f64) / ((1u64 << 53) as f64) };
        let mut c = 100.0;
        let rows: Vec<Row> = (0..n).map(|i| {
            let drift = 0.0015 * (i as f64 / 90.).sin();
            let o = c * (1. + (next() - 0.5) * vol * 0.5);
            let cl = o * (1. + drift + (next() - 0.5) * vol * 2.);
            let h = o.max(cl) * (1. + next() * vol);
            let l = o.min(cl) * (1. - next() * vol);
            c = cl;
            Row { ticker: "SPX".into(), date: format!("{}-{:02}-{:02}", 2000 + i / 250, 1 + (i % 250) / 21, 1 + i % 21), open: o, high: h, low: l, close: cl, volume: Some(1e6) }
        }).collect();
        Box::leak(rows.into_boxed_slice())
    }
    fn fixtures() -> Vec<&'static [Row]> { vec![fixture(1500, 1, 0.02), fixture(1500, 2, 0.02), fixture(1500, 3, 0.05), fixture(1500, 4, 0.08)] }

    #[test] fn every_template_parses_names_its_method_and_comes_from_an_admitted_book_method() {
        for t in TEMPLATES {
            let s = parsed(t);
            let d = method(t.method).unwrap_or_else(|| panic!("{}: not in the book catalogue", t.method));
            assert_eq!(s.origin, Some(rules::Origin { book: d.book.into(), method: t.method.into(), version: t.version }), "{}", t.method);
            assert_eq!(book_status(&d).0, "live", "{}: templates come only from admitted (live) methods", t.method);
            assert_eq!(s.exec.short, d.dir == "Short", "{}: direction", t.method);
            assert!(unavailable(&d).is_none());
        }
        assert!(TEMPLATES.iter().all(|t| !NO_TEMPLATE.iter().any(|(id, _)| *id == t.method)), "a method is either templated or has a reason");
    }
    #[test] fn book_status_matches_the_guru_catalogue() {
        for g in ["connors", "ross", "chande"] {
            let cat = crate::lab::json::parse(&crate::catalogue_json(g)).unwrap();
            let crate::lab::json::Json::Arr(items) = cat else { panic!() };
            for it in items {
                let id = it.get("id").unwrap().as_str().unwrap();
                let d = method(id).unwrap();
                assert_eq!(book_status(&d).0, it.get("status").unwrap().as_str().unwrap(), "{id}");
            }
        }
    }
    #[test] fn unavailable_reasons_are_specific() {
        assert!(unavailable(&method("r3").unwrap()).unwrap().contains("scale-in"));
        assert!(unavailable(&method("vix2").unwrap()).unwrap().contains("VIX"));
        let not_live = connors::catalogue().into_iter().find(|d| book_status(d).0 != "live").unwrap();
        assert!(unavailable(&not_live).unwrap().starts_with("the book method is not admitted for backtesting"));
    }
    #[test] fn templates_reproduce_the_book_engine_on_fixtures() {
        let mut report = Vec::new();
        for t in TEMPLATES {
            let spec = parsed(t);
            let (mut trades, mut ok) = (0, true);
            for rows in fixtures() {
                let ctx = Ctx::new(rows, rows, &[]);
                let to = rows.len() - 1;
                let book = connors::run(t.method, &ctx, 0, to, &Params::new()).unwrap();
                let fr = Frames::new(Bars::from_rows(rows));
                let lab = simulate(&compile(&spec, &fr, &mut Cache::new()).unwrap(), &fr.d, 0, to);
                let c = compare(&book, &lab);
                trades += book.len();
                if !c.diffs.is_empty() {
                    ok = false;
                    let (b, l) = c.diffs[0];
                    report.push(format!("{}: {}/{} matched; first difference book {:?} lab {:?}", t.method, c.matched, book.len(),
                        b.map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.legs.len())), l.map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px))));
                }
            }
            if ok && trades == 0 { report.push(format!("{}: no book trades on any fixture (vacuous)", t.method)); }
        }
        assert!(report.is_empty(), "{}", report.join("\n"));
    }
    #[test] fn spmt_and_rsi25_templates_are_listed() {
        for m in ["spmt_rsi2_4", "spmt_rsi4_25", "spmt_rsi_div8", "spmt_rsi_div2", "spmt_pctb_1", "spmt_dmi_1", "spmt_dmi_2", "spmt_dmi_3", "spmt_dmi_4", "spmt_dmi_5",
                  "spmt_short_4up", "spmt_short_strong", "r25_rsi25_basic_no_filter", "r25_rsi25_official_sma200"] {
            assert!(template(m).is_some(), "{m}");
        }
    }
    #[test] fn every_live_method_has_a_template_or_a_named_reason() {
        // A new admitted book method fails here until it gets a template or a NO_TEMPLATE reason.
        let vague: Vec<&str> = connors::catalogue().iter().filter(|d| book_status(d).0 == "live" && unavailable(d).as_deref() == Some(NOT_YET)).map(|d| d.id).collect();
        assert!(vague.is_empty(), "{vague:?}");
    }
    #[test] fn the_last_batch_is_listed_and_two_amibroker_methods_are_named() {
        for m in ["ststw_10d_high_buy", "ststw_10d_high_short", "ststw_10d_low_buy", "ststw_rsi2_under5", "ststw_sp_short", "hpetf_3dhl_long", "hpetf_rsi75_short", "hpetf_r3_long",
                  "hbqts_ma5_25", "hbqts_profit_target", "hbqts_ma1_36"] {
            assert!(template(m).is_some(), "{m}");
        }
        assert_eq!(TEMPLATES.len(), 43);
        assert!(unavailable(&method("hbqts_limit_entry").unwrap()).unwrap().contains("AmiBroker BuyPrice"));
        assert!(unavailable(&method("hbqts_entry_id1").unwrap()).unwrap().contains("AmiBroker BarsSince"));
    }
}
