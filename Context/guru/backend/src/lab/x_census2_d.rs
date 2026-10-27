//! Census 2 group "d" (2026-10-27): indicators from trading_indicator_census.html that the lab did not have.
//! Formulas: see each definition string. Listed-only entries carry "not available yet — needs …" (data the lab does not hold).
#![allow(dead_code)]
use super::bars::Bars;
use super::catalogue::{IndicatorDef, Needs, Output, ParamSpec, Params, Placement};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
/// Listed in the picker, not computed: the data it needs is not admitted (the app shows the note as "data not available").
const fn listed(id: &'static str, name: &'static str, category: &'static str, needs: Needs, definition: &'static str, note: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category, params: &[], outputs: &["value"], placement: Placement::Pane, needs, definition,
                   first: &[usize::MAX], sparse: false, compute: None, note }
}

pub const DEFS: &[IndicatorDef] = &[];
pub const SOURCES: &[(&str, &str, &str)] = &[];

#[cfg(test)]
mod tests {}
