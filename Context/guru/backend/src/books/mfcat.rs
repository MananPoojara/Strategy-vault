//! Mindfire Systems (mindfire-systems.com), "Catscan" and "Catscan #2" TradeStation systems, "Catscan Code.pdf"
//! (5 pages of EasyLanguage: p.1 Catscan, p.2-3 Catscan #2, p.3-4 ChopLimits, p.4-5 ChopLimits2, p.5 ChoppyIndex).
//! Book id `mfcat`. Source: research drive "USB HDD 1/TradeStation Systems & Indicators/Catscan for TS
//! (mindfire-systems.com)/", copied to Books/Mindfire Systems/. CATSC.ELA is a protected binary (not read).
//! Both systems are stop-and-reverse futures systems with a "$2,000 MM stop" coded as EntryPrice -/+ 2000/BigPointValue.
//! The source gives no BigPointValue (it is the contract's, and the S&P contract's changed from $500 to $250 in Nov
//! 1997), so the stop has no stated point size on a cash index. It cannot be dropped, so both are NeedsData and `run`
//! returns None. No index instrument swap is offered (the source defines the stop in dollars, not points).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mfcat";

const FUT: &str = "needs S&P 500 futures (SP, CommodityNumber 149) daily bars with the contract's BigPointValue: the \"$2,000 MM stop\" is coded as EntryPrice -/+ (2000/BigPointValue) (p.1-3) and the source gives no point size; the S&P BigPointValue was $500 before Nov 1997 and $250 after, so a cash-index run would pick a stop the source does not state. ChopLimits for the other 38 listed futures likewise need those contracts";

const RULES_COMMON: &[&str] = &[
    "F|ChoppyIndex (p.5): \"If HighestBar(TrueHigh,30) <= LowestBar(TrueLow,30) then ChoppyIndex = 10*(Highest(TrueHigh,30)-Lowest(TrueLow,30)) else ChoppyIndex = - 10*(Highest(TrueHigh,30)-Lowest(TrueLow,30))\"",
    "F|\"ChoppyMarket = -ChopLimits <= CI and CI <= +ChopLimits; {TRUE IF CHOPPY MARKET}\"; S&P 500 (CommodityNumber 149): ChopLimits = 60 (p.4), ChopLimits2 = 60 (p.5)",
    "F|\"XMA1 = 0.818*XMA2+ 0.182*TrueRange; {CHOPPY MKT XAVG OF DAILY RANGES} XMA2 = 0.818*XMA1 + 0.182*TrueRange; {TRENDY MKT XAVG OF DAILY RANGES}\" (vars initialised Xaverage(TrueRange,10))",
    "E|Choppy: \"If MarketPosition <> 1 and ChoppyMarket then Buy('ChoppyBuy') Tomorrow at Open of Tomorrow+1.25*XMA1 Stop\"; \"If MarketPosition <>-1 and ChoppyMarket then Sell('ChoppySell') Tomorrow at Open of Tomorrow-1.25*XMA1 Stop\"",
    "E|Trendy: \"If MarketPosition <> 1 and CI > ChopLimits then Buy('BullBuy') Tomorrow at Open of Tomorrow+0.50*XMA2 Stop\"; \"If MarketPosition <>-1 and CI < -ChopLimits then Sell('BearSell') Tomorrow at Open of Tomorrow-0.50*XMA2 Stop\"",
    "E|Channel: \"If MarketPosition <>1 then Buy('ChannelBuy') Tomorrow at the Highest(TrueHigh,40) + 1 Point Stop\"; \"If MarketPosition <>-1 then Sell('ChannelSell') Tomorrow at the Lowest(TrueLow,40) - 1 Point Stop\" (entries reverse an open position)",
    "X|\"{***$2,000 MM STOP STARTS ON 2ND DAY OF TRADE***} If MarketPosition = 1 then ExitLong('MMLongExitStop') at EntryPrice - (2000/BigPointValue) Stop; If MarketPosition =-1 then ExitShort('MMShortExitStop') at EntryPrice +(2000/BigPointValue) Stop\"",
];

const NOTE: &str = "Catscan and Catscan #2 have identical code (p.1, p.2-3) except that #2 reads ChopLimits2 (\"the original chop limits set by viewing continuous contract files and then testing them using System Writer. They are not the ones set on the 21 Futures Truth back tests\", p.4) instead of ChopLimits. For the S&P 500 both limits are 60, so on SP the two systems are the same. No published results are in the source. Open points if data arrives: 'Open of Tomorrow' is a TS2000 look-ahead stop level (computable on daily bars); '1 Point' is TradeStation's minimum-move unit, not 1.00; when several entry stops trigger on one daily bar the intrabar order is unknown; the MM stop starts on the 2nd day of the trade.";

fn d(id: &'static str, name: &'static str, group: &'static str) -> StrategyDef {
    StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status: Status::NeedsData(FUT),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: RULES_COMMON, notes: NOTE, claim: None,
    }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        d("mfcat_catscan", "Catscan (choppy/trendy stop-and-reverse + 40-bar channel, ChopLimits)", "p.1 · Catscan"),
        d("mfcat_catscan2", "Catscan #2 (same code, ChopLimits2 table)", "p.2-3 · Catscan #2"),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mfcat_catscan" => ("mfcat_catscan", "Mindfire Catscan", "ChopLimits (Futures Truth set)"),
        "mfcat_catscan2" => ("mfcat_catscan", "Mindfire Catscan", "ChopLimits2 (original set)"),
        _ => return None,
    })
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn catalogue_is_needsdata_and_owned() {
        for s in &super::catalogue() {
            assert!(s.id.starts_with("mfcat_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NeedsData(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
