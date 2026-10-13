//! Joe Krutsinger, "Trading Systematically - Ideas for Today and Tomorrow", ZAP Futures seminar slides (May 16 2002).
//! Book id `jkzap`. Source copy: Books/Joe Krutsinger/Zap Seminar/Slide.pdf (37 slides). Library: library/jkzap/.
//! The seminar sells two systems and shows their reports, but never states their rules: nothing is testable.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jkzap";
const SOLD: &str = "rules not disclosed: commercial system ($3,500), the slides give only the concept and a performance report";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules, notes, claim| StrategyDef { id, name, group: "Commercial systems (rules undisclosed)", session: 0, dir,
        status: Status::NotTestable(SOLD), needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("jkzap_traders_dream", "Trader's Dream (book text; code version: upg18_traders_dream)", "Long/Short",
          &["F|'In a short-term down trend always looks to buy; in a short-term up trend always looks to sell' (slide 17)",
            "F|An order is issued every day, no trades overnight, a stop-loss on all orders; only variable is the stop; needs the last three days of closes (slides 16-17)"],
          "@SP daily 5/9/2001-5/10/2002 (Strategy(1250,1)): 90 trades, 45.56%, net $43,800, avg $486.67 (slide 20). Trade list excerpt slide 21.",
          claim("90", "45.56%", "$486.67", "same day", "@SP 5/2001-5/2002 (slide 20)")),
        d("jkzap_prince", "Prince (book text; code version: upg18_prince)", "Long/Short",
          &["F|Short-term down trend: buy, exit on FIRST profitable open; short-term up trend: sell, exit on first profitable open; trades overnight, no stop; needs the last three days of closes (slides 28-29)"],
          "'Short-term trend' is never defined. QQQ daily 5/9/2001-5/10/2002, 1,000 shares: 63 trades, 77.78%, net $27,960, avg $443.81 (slide 31).",
          claim("63", "77.78%", "$443.81", "", "QQQ 5/2001-5/2002 (slide 31)")),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
