//! Best of the PTJ: Options & Volatility Trading. Book id "ptjopt". Filled in by the conversion pipeline; empty until converted.
#![allow(unused_imports, dead_code)]
use crate::connors::*;

pub const BOOK: &str = "ptjopt";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![]
}

/// Tunable numbers of each strategy (book value = default). None = not a strategy of this book.
pub fn params(_id: &str) -> Option<Vec<ParamDef>> {
    None
}

/// None = not a strategy of this book.
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> {
    None
}

/// (family id, family name, variant label) for the strategy list.
pub fn family_of(_id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    None
}
