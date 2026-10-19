//! Explicit optional report boundary for independently reviewed unit exposure.
//! Arithmetic and construction await the frozen independent v2 reference.
//! This is not a source cash account and is never inferred from Trade fields.

pub const MODEL: &str = "canonical_decimal_unit_exposure_v2";
pub const BOOK: &str = "hbqts";
pub const ID: &str = "hbqts_dema16_8";
pub const CONVENTION: &str = "qts_dema16_8_first_close_index_unit_exposure_v2";

#[derive(Clone, Debug)]
pub struct Day {
    pub source_index: usize,
    pub date: String,
    pub equity: f64,
    pub gross_equity: f64,
    pub benchmark_equity: f64,
    pub net_daily_return: f64,
    pub gross_daily_return: f64,
    pub benchmark_daily_return: f64,
}

#[derive(Clone, Debug)]
pub struct TradeReturn {
    pub trade_index: usize,
    pub entry_source_index: usize,
    pub exit_source_index: usize,
    pub open: bool,
    pub entry_allocation: f64,
    pub gross_return: f64,
    pub net_return: f64,
    pub fee_return: f64,
    pub slippage_return: f64,
    pub gross_contribution: f64,
    pub net_contribution: f64,
    pub fee_contribution: f64,
    pub slippage_contribution: f64,
}

#[derive(Clone, Debug)]
pub struct Report {
    pub days: Vec<Day>,
    pub trade_returns: Vec<TradeReturn>,
}
