//! William J. O'Neil - CANSLIM. Book id `canslim` (How to Make Money in Stocks, 2nd ed. 1995); the short-selling book
//! (with Gil Morales, 2005), 24 Essential Lessons, The Successful Investor and the 1999 AAII sell-rules article restate it.
//! Reading: claude-continuation-20261020/authors/oneil/agents/. Catalogue only: base breakouts are read on charts, the
//! C-A-N-S-L-I screens need fundamentals and the IBD relative-strength rank, and market direction (follow-through day,
//! distribution) uses total NYSE volume with the sell signal described rather than defined.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, st, rules| StrategyDef { id, name, group, session: 0, dir, status: st, needs_volume: false,
        needs_vix: false, needs_series: &[], book: "canslim", rules, notes: "Read 2026-10-24 (authors/oneil/agents).", claim: None };
    vec![
        d("onc_base_breakout", "CANSLIM base breakout (cup with handle) with earnings, RS and sponsorship screens", "C-A-N-S-L-I chapters", "Long",
          NotTestable("bases and pivots are read on charts; the screens need quarterly earnings, institutional ownership and the IBD relative-strength rank of all stocks"),
          &["E|Buy a breakout from a sound base on volume well above average in a stock meeting the C-A-N-S-L-I criteria"]),
        d("onc_market_direction", "M: follow-through day to buy, distribution to sell", "M = Market Direction (p.48-59)", "Long",
          NotTestable("the follow-through is 'day 3 to 10 of an attempted rally, total market volume up, Dow or S&P up 1% or more' (p.58-59), but the rally start, the distribution sell signal and the exit are described, not defined, and the volume is total NYSE volume"),
          &["E|Follow-through on rally day 3-10: volume above the day before and index up 1%+ (p.58-59)", "X|Heavy volume without further price progress on one or two days (p.50-51)"]),
        d("onc_sell_rules", "7-8% loss rule and 20-25% profit-taking", "Selling chapters; AAII 1999 article", "Long",
          NotTestable("exits for the discretionary base-breakout entry; no entry of their own"), &["X|Sell 7-8% below cost; take most profits at 20-25%"]),
        d("onc_short", "Short former leaders on the second break below the 50-day line", "How to Make Money Selling Stocks Short (2005)", "Short",
          NotTestable("'former big leader', 'abnormal break' and the 5-7 month wait are judgement; no exit rule"), &["E|Short a heavy-volume break back below the 50-day average after 2-4 rallies above it"]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("onc_").then_some(("onc_nt", "O'Neil CANSLIM (held)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
