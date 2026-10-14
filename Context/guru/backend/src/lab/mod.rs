#![allow(dead_code)]
//! Indicators lab — user-defined indicator research. Separate from book strategies:
//! nothing here is read by connors.rs, books/* or the admission files.
//! Conventions: series are Vec<f64> aligned to bars; f64::NAN = no value; comparisons with NaN are false;
//! formulas follow TradingView built-in chart indicators (see each definition string).
pub mod bars;
pub mod catalogue;
pub mod ma;
pub mod momentum;
pub mod price;
pub mod resample;
pub mod ta;
pub mod trend;
pub mod volatility;
pub mod volume;
#[cfg(test)]
pub mod testdata;
#[cfg(test)]
pub mod parity;
