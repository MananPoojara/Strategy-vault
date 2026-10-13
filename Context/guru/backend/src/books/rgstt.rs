//! Ruggiero Associates, "Stock Traders Toolbox" v1.0 (TradeStation add-on manual, 2001). Book id `rgstt`.
//! The EasyLanguage source ships only inside STT_TOOLS.ELA/ELS (Omega ORCMP archive whose members are
//! additionally encoded), so the systems are known only from the manual's descriptions. Sources: library/rgstt/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rgstt";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id, name, status, rules: &'static [&'static str], notes| StrategyDef {
        id, name, group: "Stock Traders Toolbox · systems", session: 0, dir: "Long", status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        mk("rgs_bigstock", "Inverse comparative strength (STT_BigStockNormal / DowStockMaster)",
            Status::NotTestable("the manual gives the raw index (CStrength=(Stock/Market)*100, scaled over Window, XAverage 3) but not the system's Period default, which smoothing (raw XAverage(3) or CompareStrenSlow) the system uses, nor the order types; the code is not readable"),
            &["F|CStrength=(TheStock/TheMarket)*100; RawStr=(CStrength-lowest(CStrength,Window))/(Highest(CStrength,Window)-Lowest(CStrength,Window)); STT_CompareStrenRaw=XAverage(RawStr,3) (manual)",
              "E|\"buys when comparative strength is below .45\"", "X|\"and exit the position when it is above .55\""],
            "Manual: MSFT/T/XOM/AXP/EK 1/11/93-3/28/01 (points 41.67/12.06/81.75/56.58/45.75). Would map to NSE large caps vs NIFTY 50 once the rule is complete."),
        mk("rgs_efficiency", "Efficiency system (STT_EfficencySystem)",
            Status::NotTestable("BuyLev, ExitLev and Period have no stated values and the crossing/order logic is not described"),
            &["F|Efficiency = Net change / sum(abs(all changes)) over Period (manual)", "E|Type 0: buy weakness (BuyLev<ExitLev); Type 1: buy strength", "X|Exit at ExitLev"], ""),
        mk("rgs_market_timer", "STT_MarketTimer (S&P500 + advancing/declining issues)",
            Status::NotTestable("the formula combining S&P500 with advance/decline issues is not given (only Btrig 1.05, Strig .995)"),
            &["F|Data2 S&P500, Data3 advancing issues, Data4 declining issues; outputs 1 bullish / -1 bearish (Btrig 1.05, Strig .995)"], ""),
        mk("rgs_indicators", "Indicators: PercentUpDays, RSMI, UpDownCount, Confused, volume spikes",
            Status::NotTestable("indicators with suggested reading levels, no entry/exit system"),
            &["F|PercentUpDays above 60% bullish, below 45% bearish; RSMI above .30 bullish, below -.30 bearish; UpDownCount below -5 (3-month window) bear market"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("rgs_").then_some(("rgs_toolbox", "Stock Traders Toolbox", "Not testable"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
