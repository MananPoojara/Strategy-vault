//! Mark Crisp, Daily Momentum Trading System (2006) and Momentum Share Trading System (MTS manual). Catalogue only: no rule in this source is both exact and complete. Source: library/mcmts/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "mcmts", claim: None };
    vec![
        d("mcmts_daily_momentum", "Daily Momentum Trading System", "Daily Momentum", "Long", Status::NotTestable("the scan rule \"Price gained > 50% in less than 6 weeks\" has no defined base price or measurement; the trailing stop is \"ABOUT $1/4 below\" the pivot low; \"common sense\" exits require judgement (\"draw your line in the sand\")"),
          &["F|Stocks > $10 < $75, 20-day average volume > 500,000, price gained > 50% in less than 6 weeks", "F|Rule 1: close not more than double the lowest point of the past 4 months", "E|Buy stop on a 2-day pivot high, good for max 10 trading days", "X|10% initial stop; trail about $1/4 below the 3-day pivot low; exit if the stock doubles in < 1 month; never lose more than 50% of profits once > 30% up"],
          "DailyMomentum.pdf p.6-20."),
        d("mcmts_mts", "Momentum Share Trading System (volume surge, all-time high, ceiling/floor pattern)", "MTS", "Long", Status::NotTestable("how long a stock stays on the watch list after the volume surge and how the surge, the all-time high and the 3-day ceiling must be ordered are not specified; the initial stop is 7% below the fill (Part 5) vs 8% below the purchase price (Part 4) vs 8% or the floor whichever is closer (chart B1); the volatility exit stop placement is discretionary"),
          &["F|Surge: at least a doubling of the previous 6-monthly average volume (Part II)", "F|Shares at least 12 months listed, hitting all-time highs (Rule 2)", "E|Ceiling = high not exceeded for 3 days; buy stop 1% above; floor = low not exceeded for 3 days after a ceiling; scratch below floor - 1% (Parts 4-5)", "A|Second buy on the next ceiling (Part 6)", "X|Trail 1% below each new floor; volatility exit (bar > 3x 3-month average range); protect 50% of profits once 25% up (Part 7)"],
          "Mts.pdf (131 pp scan)."),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
