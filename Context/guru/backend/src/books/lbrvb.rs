//! Linda Bradford Raschke, Volatility Breakout Systems (article, 7 pp). Catalogue only: no rule in this source is both exact and complete. Source: library/lbrvb/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "lbrvb", claim: None };
    vec![
        d("lbrvb_template", "Volatility breakout system template", "Article", "Long/Short", Status::NotTestable("a generic template: the percentage of range, range period, base and exit are left to the reader (p.4-5); no exact system is specified"),
          &["E|Buy or Sell on a stop based on the current bar open, plus or minus a percentage of the previous day range (p.4)", "X|Exit choices listed only as examples: 2nd day close, 1st day open, first profitable opening, 1 ATR target, trailing stops (p.5)"],
          "Example filters (narrowest range of last 7 days; new 20-day high/low within 5 days) are given without entry percentage or exit. Related exact Raschke rules are already in the vault (lbrtt, ss)."),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
