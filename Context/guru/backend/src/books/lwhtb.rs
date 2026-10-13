//! Larry Williams, "How to Trade Better" (adest.com.au article, December 2000). Book id `lwhtb`. Source (read in place; COPY
//! PENDING): research drive "USB HDD 2/tRADING KNO-2/Larry Williams - How To Trade Better (Article)/Larry Williams - How To
//! Trade Better(decrypted) (pdf).pdf" (3 pp). The "E-Book - How to Trade Better" copy in "Trading Books/" is password protected.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwhtb";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "lwhtb_18day_setup", name: "18-day average trend-change set-up", group: "Trend change", session: 0, dir: "Long/Short",
        status: Status::NotTestable("A set-up only: \"two days totally above the average for a buy and two days totally below the average for sells. This is the set up for the change. Then some select short-term buy/sell signals can be used for your entry\" and \"do not take these signals on their own\" (p.2); no entry order or exit."),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|18-day moving average of closes; dots when two days are totally above (buy set-up) or below (sell set-up) the average (p.2)"],
        notes: "The same set-up is in the Inner Circle notes (lwicw_18day_ma). The rest of the article is about fear and greed.", claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "lwhtb_18day_setup").then_some(("lwhtb", "How to Trade Better", "Not testable"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
