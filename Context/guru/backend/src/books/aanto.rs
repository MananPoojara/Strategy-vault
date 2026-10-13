//! Peter W. Aan, "Trend One" (PWA Futures, 1999). Book id `aanto`. Source copy: Books/Peter W. Aan/Trend One/
//! (TrendOne.pdf = 3-page sales brochure; TREND ONE.ELD + Code.rar = TradeStation code). Library: library/aanto/.
//! The brochure gives no rules. The code in Code.rar (Code.txt, extracted read-only) resets its swing
//! high/low at every new DATE and only trades while DATE TOMORROW = DATE, i.e. it is written for intraday bars
//! (the uploader's Readme: "use it with 15min S&P e-mini data"). It contradicts the brochure's description of a
//! position-trading daily system, so its provenance is uncertain; it is listed as NeedsData (intraday bars).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "aanto";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "aanto_trend_one", name: "Trend One (TradeStation code, intraday)", group: "Trend One (1999)", session: 0,
        dir: "Long/Short", status: Status::NeedsData("15-minute intraday S&P bars: the code builds each day's swing high/low from intraday bars and trades only within the day (DATE TOMORROW = DATE)"),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|Inputs BuyRetPcnt 80, BuyThrust 1.3, SellRetPcnt 71, SellThrust 0.8 (Code.txt)",
            "F|Each new date: reset hh/ll; track the day's highest high (bar hb) and lowest low (bar lb); Norm = Average(Range, 20)",
            "E|If flat/short and lb >= first bar of day and hb > lb and hb < CurrentBar and hh-ll > BuyThrust*Norm*SquareRoot(hb-lb): BuyRetLevel = ll+(hh-ll)*80/100; if Low and Low[1] > BuyRetLevel, Buy next bar at BuyRetLevel limit",
            "E|Mirror for shorts: lb > hb, hh-ll > SellThrust*Norm*SquareRoot(lb-hb), SellRetLevel = hh-(hh-ll)*71/100, SellShort next bar at SellRetLevel limit when High and High[1] < it",
            "X|Long: Sell next bar at EntryPrice+(hh-ll)*0.5 limit; if BarsSinceEntry > 16 SellShort next bar at market (LX3)",
            "X|Short: BuyToCover next bar at EntryPrice-(hh-ll)*0.5 limit; if BarsSinceEntry > 16 BuyToCover next bar at market"],
        notes: "Brochure (TrendOne.pdf) shows only a 9-market portfolio equity chart (largest drawdown $48,147, average year $51,555) and no rules. Rules above are from Code.rar (TradeStation 8 export) whose intraday structure does not match the brochure's daily position-trading description; provenance uncertain.",
        claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == "aanto_trend_one").then_some(("aanto_trend_one", "Trend One", "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
