//! Larry Williams, "The Secret of Selecting Stocks for Immediate and Substantial Gains" (Windsor Books, 1972; 1986 preface).
//! Book id `lwsss`. Source (read in place; COPY PENDING): research drive "Trading Books/Larry Williams - The Secret of Selecting
//! Stocks for Immediate and Substantial Gains/...pdf" (text, 114 pp, read in full). Chapter one is lwmds. Catalogue only.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwsss";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{NeedsData, NotTestable};
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("lwsss_ad_line", "Stock accumulation/distribution line, basic buy/sell divergence, immediate profit signal", "Ch 2-4 · Stock selection",
            NotTestable("The A/D line formula is exact, but the signals are chart divergences (\"a new price low not confirmed by a new A/D low\", p.34) and a \"tremendous surge\" out of a base (p.39), with no levels, order or exit; stock selection by comparing tracings of the stock and the Dow (p.15-20)."),
            &["F|Net daily A/D = (close - open) / (high - low) x volume, added to a running line (p.30-31)"], "Overlaps lwmds_accumulation_pattern (chapter one)."),
        d("lwsss_breadth_timing", "Short and intermediate timing: 10-day A/D net, 10-day Trading Index, Will-Go, 20 most actives", "Ch 5 · Market timing",
            NeedsData("Needs NYSE breadth (advancing/declining issues and volume, i.e. the Trading Index/TRIN), the stock/bond yield spread from Barron's, and the weekly 20 most active stocks; and the action rule is a reversal in the trend of the momentum index (judgement), with no exit."),
            &["F|\"An oversold market is indicated when this index falls below 2,000. An overbought market is signaled by the index rising above - and then falling back below - 1,000\" (p.48)", "F|10-day Trading Index: oversold above .99, overbought below .90 (p.48)",
              "F|Will-Go: cumulative 5-week change of the stock/bond yield spread, plotted 10 weeks ahead (p.51-52)"], ""),
        d("lwsss_long_term", "Selling climax, S&P yield < 2.8%, money supply, short interest, fund cash, secondaries, 37-year cycle, presidential cycle", "Ch 6-7 · Long-term timing",
            NeedsData("Needs monthly short interest, mutual fund cash/redemptions, NYSE member short sales, secondary offerings, S&P dividend yield and money supply; the indices are combined by majority vote; no entry order or exit."),
            &["F|\"a Bear market to begin anytime the yield on the S&P 500 falls below 2.8%\" (p.66)", "F|first presidential year averages -1.2%, third year +16% (p.61)"], "The 37-year 'master pattern' and the cycle counts are forecasts, not rules."),
        d("lwsss_hints", "Stops (3 points / 5%), 4-day time stop, buy on down days, Friday sells, 3-hour cycle", "Ch 8-12 · Hints",
            NotTestable("Exits and order tactics for stocks picked by the A/D work and the check-list (judgement); no stand-alone entry."),
            &["X|\"place a stop loss order 3 points above a short or 3 points below a long ... on stocks selling for more than $90 ... 5% below your cost\" (p.110)", "X|\"If the stock has not begun its move within 4 trading days, close out the position\" (p.111)",
              "F|\"Fridays are up days about 65% of the time\"; the day before a holiday up about 70% (p.89)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("lwsss_").then_some(("lwsss", "Secret of Selecting Stocks", if id.contains("breadth") || id.contains("long_term") { "Needs data" } else { "Not testable" }))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
