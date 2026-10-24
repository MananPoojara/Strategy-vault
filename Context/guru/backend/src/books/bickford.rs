//! James L. Bickford - forex charting and wave books. Book ids `jlbcmf` (Charting the Major Forex Pairs), `jlbfwt` (Forex Wave Theory),
//! `jlbfsa` (Forex Shockwave Analysis). Reading: claude-continuation-20261020/authors/bickford/agents/. Catalogue only: the wave theory
//! forecasts wave sizes and 'does not trigger specific market actions' (Forex Wave Theory p.37); the indicator chapter is a survey.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Read 2026-10-24 (authors/bickford/agents).", claim: None };
    vec![
        d("jlbfwt_waves", "Two- to six-wave cycle forecasts on swing charts", "Forex Wave Theory · Parts 4-10", "jlbfwt",
          "a forecasting method without entry or exit ('wave theory ... does not trigger specific market actions', p.37); swing reversal amounts are forex pips",
          &["F|Forecast the height and width of the next wave from the preceding waves of a swing chart"]),
        d("jlbfwt_cross", "Crossover systems survey (MACD, RSI, stochastics, Bollinger)", "Forex Wave Theory · Ch.5", "jlbfwt",
          "a textbook survey: periods 'can be optimized' and thresholds are offered as alternatives (RSI 30/70 or 20/80); not the author's rule",
          &["E|E.g. RSI buy signals 'traditionally' at 30 and sells at 70 (p.31)"]),
        d("jlbfsa_activity", "Activity/range charts and shockwave forecasts", "Forex Shockwave Analysis", "jlbfsa",
          "forecasts and descriptive charts without entry or exit; needs intraday forex prices and news releases",
          &["F|Forecast the third to sixth waves; activity and range by time of day"]),
        d("jlbcmf_composite", "Composite activity charts by hour and weekday", "Charting the Major Forex Pairs", "jlbcmf",
          "descriptive charts; no entry or exit",
          &["F|Average activity and range by time of day and day of week for EUR, GBP, CHF, JPY"]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id.starts_with("jlbfwt_") || id.starts_with("jlbfsa_") || id.starts_with("jlbcmf_")).then_some(("jlb_nt", "Bickford forex wave methods (held)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
