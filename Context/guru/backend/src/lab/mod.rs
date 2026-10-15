#![allow(dead_code)]
//! Indicators lab — user-defined indicator research. Separate from book strategies:
//! nothing here is read by connors.rs, books/* or the admission files.
//! Conventions: series are Vec<f64> aligned to bars; f64::NAN = no value; comparisons with NaN are false;
//! formulas follow TradingView built-in chart indicators (see each definition string).
pub mod api;
pub mod bars;
pub mod bands;
pub mod candles;
pub mod breadth;
pub mod catalogue;
pub mod fidelity;
pub mod json;
pub mod ledger;
pub mod listed;
pub mod markets;
pub mod model;
pub mod ma;
pub mod ma2;
pub mod momentum;
pub mod osc2;
pub mod patterns;
pub mod periods;
pub mod price;
pub mod ratings;
pub mod resample;
pub mod rules;
pub mod series;
pub mod simulate;
pub mod sources;
pub mod stats;
pub mod stops;
pub mod templates;
pub mod ta;
pub mod trend;
pub mod vol2;
pub mod volatility;
pub mod volume;
pub mod volume2;
#[cfg(test)]
pub mod testdata;
#[cfg(test)]
pub mod parity;
#[cfg(test)]
pub mod pinned;
