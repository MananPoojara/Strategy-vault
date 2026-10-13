//! Richard Anderson, Market Timing Models (Irwin, 1997). Every model is a monthly regression on
//! macro / fundamental series the vault does not hold, so nothing is Live (no code is written for
//! them: the rolling re-estimation has no data to run on). Zweig's Four Percent Model (p.174) is
//! already in the vault as mbsot_zweig4. Source notes: library/ramtm/README.md.
use crate::connors::{Claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ramtm";

const ND_STOCK: &str = "needs monthly S&P 500 dividend yield, CPI-U, Moody's Baa and Aaa yields, Fed free reserves, Ibbotson S&P 500 total-return and 1-month T-bill total-return indexes (Table 4-1, p.67); none are in the vault";
const ND_DY: &str = "needs year-end S&P 500 dividend yield and S&P 500 total returns (Table 1-1, p.4); not in the vault";
const ND_BOND: &str = "needs long-term US government bond total returns (the traded asset), CPI, T-bill total-return index and S&P oil and gold stock indexes (pp.84-90); not in the vault";
const ND_SIZE: &str = "needs small-stock vs large-stock total returns, CPI-U, 10-year and 1-year CMT yields, Moody's Baa/Aaa yields (Table 6-2, p.101); not in the vault";
const ND_CAD: &str = "needs CAD/USD exchange rate, Canadian and US CPI, S&P oil stock index, Canadian T-bill rate (Table 7-1, p.114); not in the vault";
const ND_ALLOC: &str = "needs the stock and bond model forecasts above plus bond and T-bill returns (Ch.10, pp.145-168)";
const NT_KERSCHNER: &str = "not testable: inputs are Kerschner's proprietary survey of normalized dividends and growth (pp.176-181)";
const NT_GARZ: &str = "not testable: 14 indicators scored negative/neutral/positive; the scoring of each is not disclosed (pp.181-186)";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, claim,
    };
    vec![
        make("ramtm_stock_model", "Stock market model (5-variable regression, stock weight = P)", "Ch 4 / Ch 9 · Stock market model", "Long", Status::NeedsData(ND_STOCK),
            &["F|Forecast variable: six-month excess return (S6/S0) - (TB6/TB0), S&P 500 total return less 1-month T-bills (pp.62-63)",
              "F|Predictors: dividend yield d0/P0; 12-month CPI-U change (CPI-1/CPI-13)-1; 12-month change in Baa-Aaa spread; ln(P0/P-24) of the S&P 500; free reserves / 1,000 (pp.63-66)",
              "F|Table 4-1 (1964-1993): constant -0.11; X-coefficients 0.07, -2.03, 0.09, -0.18, 0.05 (p.67)",
              "E|Simulation: re-estimate monthly from 1964 through the prior month, \"the stock allocation was set equal to the forecast probability P\" that stocks beat T-bills; rest in T-bills (pp.137-139)"],
            "Monthly asset allocation, not a daily rule. India: no NSE equivalents loaded (dividend yield, CPI, credit spread, free reserves).",
            Some(Claim { trades: "monthly allocation", win: "-", avg: "14.2% CAGR (S&P 500 14.9%); SD 8.7% vs 15.5%; turnover 90%", hold: "-", period: "1984-1993 (Table 9-4, p.139)" })),
        make("ramtm_dy_single", "Single-variable dividend-yield model", "Ch 1 · Single predictor", "Long", Status::NeedsData(ND_DY),
            &["F|Year-end dividend yield of the S&P 500 regressed on the next year's total return (Table 1-1, pp.3-17)"],
            "Teaching example; the book itself warns its yield buckets are in-sample hindsight (p.5).", None),
        make("ramtm_bond_model", "Bond market model", "Ch 5 · Bond model", "Long", Status::NeedsData(ND_BOND),
            &["F|Predictors include real interest rate, CPI-61/CPI-13, (P0/P-24)-1 of S&P total return, (TB0/TB-6)-1, 12-month change in oil and gold stocks (pp.86-90)"], ND_BOND, None),
        make("ramtm_size_model", "Large vs small capitalization model", "Ch 6 · Size model", "Long/Short", Status::NeedsData(ND_SIZE),
            &["F|12-month change CPIU 5.17, yield curve CM10-CM1 0.08, 12-month change credit spread -0.06, constant 0.66 (Table 6-2, p.101)"], ND_SIZE, None),
        make("ramtm_cad_model", "Canadian dollar model", "Ch 7 · Currency model", "Long/Short", Status::NeedsData(ND_CAD),
            &["F|Relative Canada/US CPI 0.60, 2-yr change oil stocks 0.07, 2-yr change C$/US$ -0.34, Canadian T-bill -0.01, constant 0.72 (Table 7-1, p.114)"], ND_CAD, None),
        make("ramtm_three_asset", "Three-asset allocation (stocks / bonds / bills)", "Ch 10 · Investment strategy", "Long", Status::NeedsData(ND_ALLOC),
            &["F|Turn the model forecasts into stock, bond and bill weights (pp.145-168)"], ND_ALLOC, None),
        make("ramtm_kerschner", "Kerschner dividend discount model allocation", "Ch 11 · Wall Street strategists", "Long", Status::NotTestable(NT_KERSCHNER),
            &["F|r = D/P + g from survey-normalized dividends; compare stock/bond/cash spreads with their 20-year averages; \"flexible\" portfolio: stock allocation equals the probability that stocks outperform bonds and cash (pp.179-181)"], NT_KERSCHNER, None),
        make("ramtm_garzarelli", "Garzarelli 14-indicator composite (buy >= 65, sell <= 20)", "Ch 11 · Wall Street strategists", "Long", Status::NotTestable(NT_GARZ),
            &["E|\"records a buy signal when the indicators reach 65 or higher\"", "X|\"remains in place until the score falls to 20 or below\" (p.182)"], NT_GARZ, None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("ramtm_") { return None; }
    Some(("ramtm_models", "Anderson regression timing models", "Data missing / not testable"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn nothing_is_live() {
        assert!(super::catalogue().iter().all(|d| d.status != crate::connors::Status::Live));
    }
}
