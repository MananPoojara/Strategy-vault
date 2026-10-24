//! Mark Crisp, Daily Momentum Trading System (2006) and Momentum Share Trading System (MTS manual). Catalogue only: both systems keep one judgement or undefined piece (re-read 2026-10-24, authors/crisp/agents/mcmts, dmts). Source: library/mcmts/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "mcmts", claim: None };
    vec![
        d("mcmts_daily_momentum", "Daily Momentum Trading System", "Daily Momentum", "Long", Status::NotTestable("the exits are numeric (exit if the stock doubles within a month; once more than 30% up never give back more than half the profit, p.14-15), but the scan \"Price gained > 50% in less than 6 weeks\" has no defined base price (which low, which close), and the trailing stop is \"ABOUT $1/4 below\" each 3-day pivot low: a rough dollar amount on US stocks with no rule for other price levels"),
          &["F|Stocks > $10 < $75, 20-day average volume > 500,000, price gained > 50% in less than 6 weeks", "F|Rule 1: close not more than double the lowest point of the past 4 months", "E|Buy stop on a 2-day pivot high, good for max 10 trading days", "X|10% initial stop; trail about $1/4 below the 3-day pivot low; exit if the stock doubles in < 1 month; never lose more than 50% of profits once > 30% up"],
          "DailyMomentum.pdf p.6-20."),
        d("mcmts_mts", "Momentum Share Trading System (volume surge, all-time high, ceiling/floor pattern)", "MTS", "Long/Short", Status::NotTestable("the order and window are given (\"First comes Volume, then an All Time High, then The Pattern\", p.110; \"surge in volume within the last 6 months\", p.116) and the initial stop is \"no more than 7% away\" (p.116; the floor when closer), but the sector filter is judgement, the volatility-exit stop placement is left to the trader (Part 7), and the all-time-high test needs each share's full price history since listing, which the stock files do not guarantee. The book also trades a mirrored short side on 4-month lows (p.68-75)"),
          &["F|Surge: at least a doubling of the previous 6-monthly average volume (Part II)", "F|Shares at least 12 months listed, hitting all-time highs (Rule 2)", "E|Ceiling = high not exceeded for 3 days; buy stop 1% above; floor = low not exceeded for 3 days after a ceiling; scratch below floor - 1% (Parts 4-5)", "A|Second buy on the next ceiling (Part 6)", "X|Trail 1% below each new floor; volatility exit (bar > 3x 3-month average range); protect 50% of profits once 25% up (Part 7)"],
          "Mts.pdf (131 pp scan)."),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
