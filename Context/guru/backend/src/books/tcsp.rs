//! Toby Crabel, "System and Portfolio Development with Long-Term Trading Strategies" (TAG 20 seminar, Las Vegas 1998).
//! A 13-page slide handout: performance history, correlations and portfolio statistics. It states no entry/exit rules,
//! so nothing can be coded without inventing rules. Transcription: library/tcsp/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tcsp";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, why, rules, notes| StrategyDef { id, name, group: "Long-term portfolio (seminar handout)", session: 0, dir: "Long/Short",
        status: Status::NotTestable(why), needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("tcsp_20day_breakout", "Simple 20 Day Breakout vs Anticipatory Breakout",
          "the handout prints only results (avg month 0.45% vs 0.47%, monthly stdev 2.25% vs 2.20%, largest drawdown 10.60% vs 9.47%, Sharpe 0.69 vs 0.74; slide 11, p.287); no entry side, exit, stop, sizing or market list is stated, and the anticipatory rule is not defined",
          &["E|\"Simple 20 Day Breakout\" and \"Anticipatory Breakout\" (slide 11) - rules not given", "F|\"anticipatory trend following strategy - if you trend trade, anticipate\" (slide 1)"],
          "A 20-day channel breakout is a common rule, but the handout does not say which version (close or intraday break, exit, reversal), so coding one would be an invented rule."),
        d("tcsp_ltp_components", "CCM Long Term Portfolio: Reverse 1, Reverse 2, Trend 1, Trend 2",
          "only performance statistics and a correlation matrix are printed (slide 7, p.283: avg annual 20.54% / 34.32% / 40.26% / 56.54%, total 37.92%, Sharpe 3.40); the four strategies' rules are not disclosed",
          &["F|\"counter trend strategy a must\", \"lengthen short term ideas - some short term ideas can be applied to longer time frames (ORB)\" (slide 1)"], ""),
        d("tcsp_risk_rules", "Running the business: risk cap and allocation",
          "portfolio/business guidance (\"cap your risk at a realistic level, 4% max. in single day\", fixed vs volatility-adjusted allocation; slide 1), not a trading rule on one instrument",
          &["F|\"Make sure you stay in business - cap your risk at a realistic level, 4% max. in single day\" (slide 1)"], ""),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("tcsp_").then_some(("tcsp", "Crabel · Long-term portfolio (seminar)", ""))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> { id.starts_with("tcsp_").then(Vec::new) }
pub fn run(id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { id.starts_with("tcsp_").then(Vec::new) }
