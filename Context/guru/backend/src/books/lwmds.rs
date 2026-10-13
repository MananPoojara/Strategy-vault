//! Larry Williams, "My Million Dollar Stock Market Concept" / "The Million Dollar Stock Market Idea" (19 pp excerpt, chapter one
//! of "The Secret of Selecting Stocks"). Book id `lwmds`. Source: Books/Larry Williams/Million Dollar Stock Market Idea.pdf
//! (text). Nothing in it is a complete mechanical rule; see library/lwmds/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwmds";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{NeedsData, NotTestable};
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("lwmds_ma_uses", "Three ways to use moving averages", "Moving averages",
            NotTestable("Only ideas: trade with the trend of the average, act on penetrations 'if, and only if, other technical or fundamental criteria have been met', momentum 'discussed later' (p.7-8); no entry, exit or parameters."),
            &["F|\"trade the long side of a stock only when the moving average is up and trade the short side only when the trend of the moving average is down\" (p.7)"], ""),
        d("lwmds_yield_payout", "Stock yield bands and payout time", "Fundamentals",
            NeedsData("Needs each stock's 10-20 year dividend-yield history and earnings growth; the levels are read off each stock's own chart (p.9-10)."),
            &["F|\"Short sale selections should come from stocks showing very low historical yields. Long candidates come from the high yielding stocks\" (p.9)"], ""),
        d("lwmds_accumulation_pattern", "Accumulation / distribution pattern (stock vs market divergence)", "Accumulation",
            NotTestable("Chart comparison by tracing paper ('the greater the divergence ... the larger move', p.15-17); the 'precise formula' for daily buying/selling volume is announced but not given in this excerpt."), &[], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("lwmds_").then_some(("lwmds", "Million Dollar Stock Market Idea", "Not testable"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
