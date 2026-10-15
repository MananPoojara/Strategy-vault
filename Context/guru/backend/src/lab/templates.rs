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
    let named = NO_TEMPLATE.iter().find(|(id, _)| *id == d.id).map(|(_, r)| *r);
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
pub const NO_TEMPLATE: &[(&str, &str)] = &[
    ("radge_oc_long", "the admitted Radge stop-free open-to-close cash-index research rule has no source-specific lab template or template-fidelity comparison yet"),
    ("sbma_sma50_long", "the admitted Burns 50-day SMA long/cash stock rule has no source-specific lab template or template-fidelity comparison yet; strict above/below ties and reviewed fill conventions must be preserved"),
    ("sbma_x_5s20s", "the admitted Burns 5/20-day SMA long/cash stock crossover has no source-specific lab template or template-fidelity comparison yet; strict crossover ties and reviewed fill conventions must be preserved"),
    ("sbma_x_5e50s", "the admitted Burns 5-day EMA / 50-day SMA long/cash stock crossover has no source-specific lab template or template-fidelity comparison yet; EMA seeding, crossover ties and reviewed fill conventions must be preserved"),
    ("sbma_x_10e20e", "the admitted Burns 10/20-day EMA long/cash stock crossover has no source-specific lab template or template-fidelity comparison yet; EMA seeding, crossover ties and reviewed fill conventions must be preserved"),
    ("sbma_x_10e50e", "the admitted Burns 10/50-day EMA long/cash stock crossover has no source-specific lab template or template-fidelity comparison yet; EMA seeding, crossover ties and reviewed fill conventions must be preserved"),
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
