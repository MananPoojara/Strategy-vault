//! SwingTraderGuide.com (Kevin Brown), "The Definitive Guide to Swing Trading Stocks" Rev 4.0 (2003-2007).
//! Catalogue only (see library/stgdg/README.md). Source copy: Books/SwingTraderGuide/Definitive_Guide_to_Swing_Trading_Stocks.pdf.
use crate::connors::{ParamDef, Params, Status, StrategyDef, Trade, Ctx};

pub const BOOK: &str = "stgdg";
const WHY: &str = "Not testable as a system. The author's checklist (p.52) requires conditions the book states but does not quantify: \"Trend line not violated?\" (trend lines drawn by hand on closes, p.28), \"Linear regression channel slope\" (p.30), a profit objective at one of the Swing Trade Levels \"usually between 2% and 5%\" (which level is the trader's pick, p.41), and an initial stop at a \"fixed value\" with no size given (p.48). The entry indicators are offered as alternatives (\"MACD turns up two days in a row and/or initially crosses the zero line\", ROC(2) <= 0, magic-day confirmation, p.33-38) and the author says he acts on signals \"in a subjective manner\" (p.16). Stock universe: $5-$25 stocks in the top 50% of 90-day volume, beta filter (p.21-24). The 10/20 (50) SMA trend filter alone duplicates vault MA filters.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "stgdg_checklist", name: "Trading campaign checklist (trend + ROC/MACD + buy stop above prior high)", group: "Swing trading checklist",
        session: 0, dir: "Long/Short", status: Status::NotTestable(WHY), needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|\"I use a 10 day and 20 day simple moving average to measure trend direction ... For more conservative trades, I add the 50 day moving average\" p.29-30",
            "E|\"I place a Buy order above the HIGH of the previous day to go LONG in an uptrend when the indicator [Close - Close of 2 days ago] is equal to or less than zero.\" p.33",
            "X|\"I place profit objective exit orders at Swing Trade Levels for all positions. I ensure these are usually between 2% and 5%.\" p.41; \"I always lock in profits by moving my initial stop-loss to 3% of the previous day's closing price\" p.49"],
        notes: WHY, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "stgdg_checklist").then_some(("stgdg", "SwingTraderGuide checklist", "not testable"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
