//! Alexander Elder, "Study Guide for Entries & Exits" (Wiley, 2006). Book id `elsgee`.
//! Source: research drive `Trading Books/Alexander Elder - Study Guide for Entries & Exits/Alexander Elder - Study Guide for Entries & Exits.pdf` (image scan).
//! Q&A on the material of `elee`; it adds no rule. Notes: library/elsgee/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "elsgee_qa", name: "Study-guide questions (no new rules)", group: "Study Guide", session: 0, dir: "Long/Short",
        status: Status::NotTestable("multiple-choice questions and chart-reading exercises on the 16 traders of Entries & Exits; no entry/exit rule beyond those already catalogued (and not testable) in elee"),
        needs_volume: false, needs_vix: false, needs_series: &[], book: "elsgee",
        rules: &["F|Answers 1-101 (printed p.95-162): organization, psychology, markets, tactics, money management (2% / 6% rules), case studies, traders speak"],
        notes: "Case studies are discretionary chart readings (Impulse colours, divergences, kangaroo tails, channels).", claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == "elsgee_qa").then_some(("elsgee", "Study guide", "Q&A")) }
pub fn params(_: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_: &str, _: &Ctx, _: usize, _: usize, _: &Params) -> Option<Vec<Trade>> { None }
