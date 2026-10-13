//! "Uncle Bill", Dow 100-year VMA timing system (Word document UncleBill.doc + Profit and Loss.csv, unclebillsplace Yahoo group).
//! Source: research drive MetaStock/@Systems & Indicators/UncleBill Timing System/; copies in Books/Uncle Bill/VMA Timing System/.
//! Book id "ubvma". Notes: library/ubvma/README.md. Nothing is Live: the trade rules include visual checks and an
//! oscillator whose lengths are printed as "??".
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "ubvma_ribbon_vmacdp", name: "Uncle Bill VMA ribbon + VMACDP timing (Dow)", group: "UncleBill.doc", session: 0, dir: "Long/Short",
        status: Status::NotTestable("entries and exits must also pass the VMACDP rules, which the document says 'MUST BE OBSERVED VISUALLY', and the VMACDP formula '(Mov(CLOSE,??,VOL) / Mov(CLOSE,??,S)) - 1' has no lengths; the ribbon alone would drop a stated condition. It also needs Dow volume back to 1897 for the printed test"),
        needs_volume: true, needs_vix: false, needs_series: &[], book: "ubvma",
        rules: &[
            "F|Ribbon green (bullish / long): \"(ROC(Mov(C,20,S),1,$) > 0 AND (Mov(C,20,VOL) > Mov(C,20,S))) OR (ROC(Mov(C,20,VOL),1,$) > 0 AND (Mov(C,20,VOL) > Mov(C,20,S)))\"; red (bearish / short) otherwise",
            "F|\"Not included in ribbon MUST BE OBSERVED VISUALLY ((ROC(Mov(C,20,S),1,$) > 0) AND (ROC(Mov(C,20,VOL),1,$) > 0))\"",
            "E|\"When the ribbon turns to green (first few days only) the blue line should be above the red line, or at least the green line should be above the blue line and the blue line should be rising.\" (VMACDP window)",
            "X|\"When the ribbon turns to red the blue line should be below the red line, or at least the green line should be below the blue and the blue should be falling.\"",
            "F|VMACDP: \"(Mov(CLOSE,??,VOL) / Mov(CLOSE,??,S)) - 1\"; \"if a red or green signal line has a skull and cross bones it is not a valid entry or exit point because of one of the visual rules\"",
        ],
        notes: "Profit and Loss.csv: Dow 1897-1899, 16 alternating long/short trades, totals +81.70 points, 80% winners (12 of 15 closed), average gain 13.2%, average loss -0.8%. Not reproducible: rules partly visual, lengths missing, and the vault has no 1897 Dow volume.",
        claim: None,
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "ubvma_ribbon_vmacdp").then_some(("ubvma", "Uncle Bill VMA timing (not testable)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
