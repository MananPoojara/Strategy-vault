//! Alexander Elder, "Study Guide for Sell & Sell Short" (Wiley, 2008). Book id `elsgss`.
//! Source: research drive `Trading Books/Alexander Elder - Study Guide for Sell & Sell Short/Alexander Elder - Study Guide for Sell & Sell Short.pdf` (text layer).
//! Q&A on the material of `elsss`; it adds no rule. Notes: library/elsgss/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "elsgss_qa", name: "Study-guide questions (no new rules)", group: "Study Guide", session: 0, dir: "Long/Short",
        status: Status::NotTestable("questions 1-115 restate Sell & Sell Short; SafeZone is \"hardly a mechanical approach\" (answer 70), NH-NL spikes, channels and engine-noise selling are judgement calls"),
        needs_volume: false, needs_vix: false, needs_series: &[], book: "elsgss",
        rules: &["F|Trade grade = % of the daily channel captured: A >= 30%, B 20%, C 10%, D loss (answers 29, 35) - a grading rule, not a trade rule"],
        notes: "No entry/exit rule beyond those already catalogued (and not testable) in elsss.", claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == "elsgss_qa").then_some(("elsgss", "Study guide", "Q&A")) }
pub fn params(_: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_: &str, _: &Ctx, _: usize, _: usize, _: &Params) -> Option<Vec<Trade>> { None }
