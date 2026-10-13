//! longtermtrading.com (Alan Pryor), "Anomaly2" System Manual v1.01 (2002) with the TradeStation entry code AN2V101P.TXT
//! ("Anomaly2 v 1.01, copyright 2001"). Book id `ltan2`. Source: research drive "USB HDD 1/TradeStation Systems &
//! Indicators/Anomaly 2 Trading System (longtermtrading.com)/" (identical manual in "Anomaly II + Manual  for TS
//! (longtermtrading.com)/"), copied to Books/longtermtrading.com/Anomaly II/. Refs are the manual's Steps 1-7.
//! Same entries as Anomaly1 (ltan1) with an adaptive price-channel + volatility-filter trailing exit. The system is
//! defined and tested only on a commodity futures portfolio with a $2,100-per-contract money-management stop and a
//! $5,000 / 50% profit-retention stop; those dollar rules have no stated stock-index equivalent and cannot be dropped,
//! so the strategy is NeedsData and `run` returns None.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ltan2";

const FUT: &str = "needs weekly back-adjusted futures contracts of the manual's commodity portfolio (1970-2002) with each contract's big-point value: the $2,100-per-contract money-management stop (Step 4) and the $5,000 / 50% profit-retention stop are dollar rules, and the manual defines no stock-index contract, so no index instrument swap exists";
const NOTE: &str = "Weekly bars (monthly test also published). Inconsistencies recorded, not resolved: Step 5 prose says the exit ADX bands start at 0-13 and rise by 13 up to > 52, but the table bands are 0-13, 13-21, 21-29, 29-37, > 37 (code inputs LExADXSt 13, LExADXIn 8 give < 21, 21-29, 29-37, >= 37); the manual table's 0-21 rows (buy stop 8 / sell stop 5 for both 0-13 and 13-21) match the code (long 5, short 8 below ADX 21). The Step 6 worked example puts ADX 30 'within the prescribed ADX range of 39-52', which is not a band of either table. The channel exits are 'IncludeSystem: An2 Long/Short Exit 1.02P', not in the text file (protected .ELA). Code Signal #2 requires DMI+ > DMI- (long) and uses MinDBuy2 / MaxDSel2; the print block swaps Buy#1/#2 labels. Entry code mixes 'High + MinMove Point' (signal 2) and 'High + 1 Point' (signal 1).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "ltan2_anomaly2", name: "Anomaly2 (SlowD zone breakout, adaptive channel + volatility-filter exits)",
        group: "Manual Steps 1-7 · Anomaly2 system", session: 0, dir: "Long/Short", status: Status::NeedsData(FUT),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"Calculate the 13 Bar ADX, DMI+, and DMI-\" (Step 1); SlowD(13) (code SlowD(Length1), Length1 = 13)",
            "E|Primary (code 'Signal#1'): \"If SlowD(length1) crosses above (100-MinDBuy1) ... Buy next bar at High + 1 Point stop\"; \"If SlowD(Length1) crosses below (100-MaxDSel1) ... Sell Short next bar at Low - 1 point stop\"; MinDBuy1 34 (buy zone > 66), MaxDSel1 55 (sell zone < 45) (Step 2)",
            "E|Secondary (code 'Signal #2'): SlowD has stayed in the buying (selling) zone for the last LookBak bars and DMI+ > DMI- (DMI+ < DMI-) -> buy (sell) next bar at High + MinMove (Low - MinMove) stop; LookBak by ADX(13): 0-8 -> 5, 8-16 -> 5, 16-24 -> 3, 24-32 -> 2, > 32 -> 1 (Step 3 table; code bands < 16 / 16-24 / 24-32 / >= 32)",
            "X|Money-management stop: losses limited to \"not greater than $2,100 per contract\" from the entry (Step 4)",
            "X|Channel trailing exit (Step 5 table): ADX 0-13 -> buy stop above the highest high of the last 8 bars / sell stop below the lowest low of the last 5; 13-21 -> 8 / 5; 21-29 -> 5 / 3; 29-37 -> 3 / 2; > 37 -> 1 / 1",
            "X|Volatility filter (Step 6 table): average of the last 5 (ADX 0-13), 5 (13-21), 3 (21-29), 2 (29-37), 1 (> 37) bar ranges, range = larger of high - previous close and previous close - low; Step 7: \"If long, the Sell Stop is simply the lower of either the Anomaly2 Exit Point or the Volatility Filter subtracted from the current bar's close\" (short: the higher of the exit point or close + filter)",
            "X|Percent profit retention: \"set at 50% retention with a floor of $5,000\" (manual Sec. 6)",
            "F|Published: weekly 04/10/1970-6/30/2002: 2,612 trades, 43.64% profitable, average trade $687.76, profit factor 1.91, net $1,796,422, max DD $37,319; monthly 02/26/1971-6/30/2002: 776 trades, 42.65%, average $1,588.81, PF 2.50",
        ],
        notes: NOTE,
        claim: claim("2,612", "43.64%", "$687.76", "avg 34.2 days", "04/10/1970-6/30/2002, futures portfolio, weekly"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ltan2_anomaly2" => ("ltan2_anomaly", "longtermtrading.com Anomaly2", "Weekly, book defaults"),
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
            assert!(s.id.starts_with("ltan2_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NeedsData(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
