//! Nelson Freeburg, "Mechanical Timing Systems: The Key to Consistent Profits & Sharper Trading"
//! (TAG 21, Nov 1999, INO workshop, 29 pp. scan). Source and checks: library/nfmt/.
//! Same systems as the 1996/2000 workshops (Channel 25, Pathfinder, R-Squared, intermarket T-Bond,
//! CQIII, Value Line swing) - catalogued there. New here: the 1999 Quadram rules with a T-Bond filter.
use crate::connors::{ParamDef, Params, Status, StrategyDef, Trade, Ctx};

pub const BOOK: &str = "nfmt";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "nfmt_quadram_tbond", name: "Quadram S&P system with T-Bond filter (1999)", group: "Quadram S&P Futures Trading System", session: 0,
        dir: "Long/Short", status: Status::NeedsData("daily T-Bond futures (filter T-Bond > T-Bond[5]); not in the vault"),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["E|Long countertrend: \"T-Bond > T-Bond[5] AND CHAD Crosses Above 4-DMA Minus 0.8 Standard Deviation\"; trend-following: \"T-Bond > T-Bond[5] AND CHAD Crosses Above 25-DMA Plus 0.8 Standard Deviation\" (Fig. 43, p.24)",
                 "E|Short countertrend: \"T-Bond <= T-Bond[5] AND CHAD Crosses Below 4-DMA Plus 0.8 Standard Deviation\"; trend-following: \"... Crosses Below 25-DMA Minus 0.8 Standard Deviation\" (Fig. 43)",
                 "X|\"$500 Initial Stop; $500 Trailing Stop\" (Fig. 43)"],
        notes: "Book (Fig. 41, p.23): S&P 500 futures 1982-1999, 2 contracts @ $250/pt: 784 trades, 43%, avg $395.41, PF 1.94. Without the T-Bond filter the 2000 version is Live as nfhp_quadram_*.",
        claim: crate::connors::claim("784", "43%", "$395.41 / trade (2 lots)", "1 bar winners, 0 losers", "S&P 500 futures 1982-1999"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "nfmt_quadram_tbond").then_some(("nf_quadram", "Freeburg Quadram S&P system", "T-Bond filtered (1999) · Data missing"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
