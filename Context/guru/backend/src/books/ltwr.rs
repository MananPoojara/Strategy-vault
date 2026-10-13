//! longtermtrading.com (Alan Pryor), "WaveRider" System Manual v1.02a with the website pages (WaveRider Introduction
//! .htm and others). Book id `ltwr`. Source: research drive "USB HDD 1/TradeStation Systems & Indicators/Wave Rider 1.02
//! + Manual for TS (longtermtrading.com)/", copied to Books/longtermtrading.com/WaveRider/. Refs are the manual's
//! sections (TOC: Sec. IV p.18, Sec. V p.21-28 with the ADX table "p.22", Sec. VI p.29, App. D). WRV102AP.ELA is a
//! protected binary and Readme.rar is blocked (no unrar); neither was read.
//! The system is defined for, and tested only on, the vendor's standard 21-commodity futures portfolio (no stock-index
//! contract), with a $2,100-per-contract money-management stop and a $5,000-floor / 50% profit-retention stop. Those
//! dollar rules have no stated stock-index equivalent and cannot be dropped, so the strategy is NeedsData (same verdict
//! as ltan1/ltan2 from the same vendor); `run` returns None.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ltwr";

const FUT: &str = "needs weekly back-adjusted futures contracts of the manual's standard 21-commodity portfolio (1970-2002/2005) with each contract's point value: the money-management stop is \"calculated by dividing $2,100 by the point value of the underlying commodity\" (Sec. V Step 3) and the profit-retention stop starts at $5,000 open profit (Step 4); the manual defines no stock-index contract, so no index instrument swap exists";

const NOTE: &str = "Weekly bars (a monthly test is also published). The ADX is described as Wilder's 13-bar ADX with the ADX taken as \"the arithmetic average of the previous 13 bar's DXs\" (Step 1). Table (Step 2): the 0-13 and 13-26 rows are equal (buy 13 / sell 8), matching App. D's four values per side (Length8-11 = 13/8/5/2 buy, Length4-7 = 8/5/3/1 sell, Length2 13 lowest action level, Length3 13 increment). The worked Japanese-yen example (Sec. V, 4/4/97-3/2/2000) confirms the table rows used. Stops are placed 'one tick' beyond the channel. The manual notes TradeStation's intrabar assumption produced a same-bar re-entry artifact it kept in the published results, and that its Windows software assumes high-before-low when open > close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "ltwr_waverider", name: "WaveRider (ADX-adaptive Fibonacci channel stop-and-reverse)",
        group: "Sec. V · WaveRider turning points", session: 0, dir: "Long/Short", status: Status::NeedsData(FUT),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"Step 1 - Calculate the 13 Bar ADX\" (Sec. V)",
            "E|\"Step 2 - Calculate the WaveRider Stop and Reverse Points based on the Current Bar's ADX Value\": highest high / lowest low within the last N bars for the buy / sell stop: 0 < ADX <= 13 -> 13 / 8; 13 < ADX <= 26 -> 13 / 8; 26 < ADX <= 39 -> 8 / 5; 39 < ADX <= 52 -> 5 / 3; ADX > 52 -> 2 / 1 (table, Sec. V \"p.22\")",
            "E|\"if long with and ADX of 40 (i.e. within the ADX range of 39 - 52), a sell stop is placed one tick below the lowest low of the 3 bars. If short with an ADX of 40, a buy stop is placed one tick above the highest high of the last 5 bars.\"; \"a reversal system in that a trader continually reverses from long to short and vice-versa\" (Sec. IV.1)",
            "X|\"Step 3 - Calculate the Money Management Stop ... designed to limit your losses to not greater than $2,100 per contract ... calculated by dividing $2,100 by the point value of the underlying commodity\"",
            "X|\"Once $5,000 in open profits is achieved, a trailing stop is calculated based on protecting 50% of all profits. If this stop is closer to the current price than the Buy Stop and Reverse point, it becomes our new Buy Stop exit point\" (Step 4); after a money-management or profit-retention exit \"the trader awaits a new signal to enter in the same or opposite direction\" (Sec. IV.1)",
            "F|Published (Sec. VI): weekly 04/10/1970-6/30/2002, 21 commodities, $150/trade: 2,975 trades, 41.68% profitable, average trade $621.50, PF 1.69, max DD $48,957; monthly 02/26/1971-6/30/2002 ($300/trade): 1,004 trades, 45.82%, average $1,365.06, PF 2.18. Website (WaveRider Introduction.htm): weekly 1/1/1970-12/31/2005: 3,357 trades, 40.81%, average $643.42, PF 1.72",
        ],
        notes: NOTE,
        claim: claim("2,975", "41.68%", "$621.50", "avg 45.7 days", "04/10/1970-6/30/2002, 21 futures, weekly"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ltwr_waverider" => ("ltwr_waverider", "longtermtrading.com WaveRider", "Weekly, book defaults"),
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
            assert!(s.id.starts_with("ltwr_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NeedsData(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
