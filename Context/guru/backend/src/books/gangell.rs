//! George Angell: "Sniper Trading Workbook" (Wiley 2002; book id `gasnw`), "Small Stocks for Big Profits" (Wiley 2008; `gassp`), "Sure-Thing
//! Options Trading" (`gasto`) and the Complete Day-Trading Course (`gacdt`). Reading: claude-continuation-20261020/authors/angell/agents/.
//! Catalogue only: Taylor-book buy/sell envelopes and range projections are support and resistance estimates for S&P/bond futures day
//! trades; when to act at them, gap fades and day-of-week buys are judgement. The LSS 3-day article is the existing module rhlss.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/angell/agents).", claim: None };
    vec![
        nt("gasnw_envelopes", "Taylor numbers: buy/sell envelopes and anticipated range", "Sniper Trading Workbook · Ch.1-5", "gasnw",
           "the rally, decline, buying-high and buying-under averages give target levels, but which numbers to use, how many days to average and whether to act at a level are left to the trader; S&P and bond futures day trades",
           &["F|\"The buying high number measures how far above the prior day's high the market traded today\" (p.3)"]),
        nt("gasnw_patterns", "Day-of-week, gap and 3-day patterns", "Sniper Trading Workbook · Ch.11, 13, 17, 26, 28", "gasnw",
           "conditions are judgement ('it helps if you are already in a bull market', 'sharply higher open', 'extreme gaps ... should not be faded') and no exit is given",
           &["E|\"Opening gaps to the upside should be sold; opening gaps to the downside should be bought\" (Ch.17)"]),
        nt("gassp_strategies", "Small-cap stock selection, support, rebound and filters", "Small Stocks for Big Profits · Ch.3-7", "gassp",
           "stock selection by story, insider buying and chart support; 'rule of three' and rebound timing are chart reading without exits",
           &["F|\"On the third attempt, the support or resistance ought to be broken\" (p.42)"]),
        nt("gasto_options", "Option strategies", "Sure-Thing Options Trading", "gasto",
           "option structures priced from quotes and chosen to a market view (scanned copy read by contents)",
           &[]),
        nt("gacdt_course", "One- and four-day percent method, envelopes, gap rules", "Complete Day-Trading Course", "gacdt",
           "S&P futures intraday rules from a decision table with contract counts and point stops; the envelope inputs are judgement",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("gacdt_") || id.starts_with("gasnw_") || id.starts_with("gassp_") || id.starts_with("gasto_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("gangell_nt", "Angell day trading (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
