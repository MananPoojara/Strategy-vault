//! Mark Minervini - SEPA growth-stock method. Book ids `wizard` (Trade Like a Stock Market Wizard, 2013; printed page =
//! pdf page - 15) and `champion` (Think & Trade Like a Champion, 2017). Reading: claude-continuation-20261020/authors/
//! minervini/agents/. Catalogue only: the Trend Template is a qualifier, not a buy signal (Wizard p.80); the buy is a
//! volatility-contraction pivot read by eye; fundamentals and the IBD relative-strength rank need data we do not hold.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, book, st, rules| StrategyDef { id, name, group, session: 0, dir: "Long", status: st, needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Read 2026-10-24 (authors/minervini/agents).", claim: None };
    vec![
        d("mmw_trend_template", "Trend Template: stage 2 qualifier (150/200-day, 50-day, 52-week range, RS rank 70+)", "Ch.5 (p.79-80)", "wizard",
          NotTestable("a qualifier, not an entry: 'we are not buying a stock just because it happens to be in a stage 2 uptrend' (p.80). The buy is a contraction pivot read by eye and the template has no exit; criterion 8 needs the IBD relative-strength rank of all stocks"),
          &["F|Price above the 150- and 200-day averages; 150 above 200; 200-day rising at least 1 month; 50 above 150 and 200; price above the 50-day; 30%+ above the 52-week low; within 25% of the 52-week high; RS rank 70+ (p.79)"]),
        d("mmw_vcp", "Volatility Contraction Pattern pivot breakout", "Ch.10 (p.189-258)", "wizard",
          NotTestable("the base, its 2-6 shrinking pullbacks and the pivot are identified by eye; the book gives no numeric definition"),
          &["E|Buy as the price moves above the pivot of a tightening stage 2 base on rising volume"]),
        d("mmw_sepa_fundamentals", "SEPA earnings, sales, margin and estimate filters", "Ch.7-8 (p.117-160)", "wizard",
          NeedsData("quarterly earnings, sales, margins and analyst estimates are not in the vault"), &["F|Accelerating EPS and sales, expanding margins, positive surprises and revisions"]),
        d("mmw_risk", "Maximum 10% stop, breakeven stop after a multiple of the risk", "Ch.12-13 (p.269-316)", "wizard",
          NotTestable("money management for the discretionary SEPA entry; it has no entry of its own"), &["X|Loss never more than 10%, average 6-7% (p.276); raise the stop to breakeven once the gain is a multiple of the stop (p.296)"]),
        d("mmc_cheat_entries", "3-C cup completion cheat and low-cheat entries", "Sections 6-7", "champion",
          NotTestable("earlier entries inside a cup or base, read by eye"), &["E|Buy a pause in the cup before the pivot"]),
        d("mmc_violations", "Post-breakout violations: close below the 20-day, three lower lows, heavy close below the 50-day", "Section 1", "champion",
          NotTestable("exit warnings tied to the discretionary breakout entry; 'soon after a breakout' is not defined"), &["X|Reduce or exit on a close below the 20-day soon after a breakout"]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id.starts_with("mmw_") || id.starts_with("mmc_")).then_some(("mm_nt", "Minervini SEPA (held)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
