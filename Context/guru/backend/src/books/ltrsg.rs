//! longtermtrading.com (Alan Pryor), "Ready-Set-Go - A Long Term Trading System", System Manual v1.02a (July 2002).
//! Book id `ltrsg`. Source: research drive "USB HDD 1/TradeStation Systems & Indicators/Ready-Set-Go 1.02 + Manual for TS
//! (longtermtrading.com)/ReadySetGoManualv102a.doc" (+ Rsg102ap.ela, protected binary), copied to
//! Books/longtermtrading.com/Ready-Set-Go/. Refs are the manual's sections (Sec. V Steps 1-7, Sec. VI.1).
//! The manual defines and tests the system only on a 21-commodity futures portfolio (no stock-index contract); its
//! $2,100-per-contract money-management stop, $5,000 / 50% profit-retention stop and "not a new life of continuous
//! contract high or low" condition need futures contracts. Index use is not defined by the manual, so NeedsData.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ltrsg";

const FUT: &str = "needs weekly back-adjusted continuous futures contracts of the manual's standard 21-commodity portfolio (1970-2002) with each contract's big-point value and life-of-contract highs/lows, because the $2,100-per-contract money-management stop, the $5,000 / 50% profit-retention stop and the 'not a new life of continuous contract high or low' entry condition are futures rules; the manual defines no stock-index contract, so no index instrument swap exists";
const NOTE: &str = "Weekly bars (a monthly test is also published). Text inconsistencies a coder must resolve from the protected Rsg102ap.ela, not guess: Step 4 says the exit ADX ranges step by 13 while its table steps by 8 (0-13, 13-21, 21-29, 29-37, >37); the worked example quotes ADX(8) while Step 1 says 13 bars; the Trade 4 example says 'above a 5 bar Significant Low Pivot Point' for a long. Stop-and-reverse applies only when the new significant pivot is beyond the trailing stop (Step 2).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "ltrsg_ready_set_go", name: "Ready-Set-Go (significant failure-swing pivot breakout, volatility-filtered channel exit)",
        group: "Sec. IV-V · Ready-Set-Go system", session: 0, dir: "Long/Short", status: Status::NeedsData(FUT),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"Calculate the 13 Bar ADX\" (Sec. V Step 1)",
            "E|\"A significant High or Low Pivot Point (i.e. one that is used for entering a new trade) is one that is not a new life of continuous contract high or low and in which the number of bars from 1 bar before the high or low to the previous low or high is greater than or equal to the minimum bars specified\": ADX 0-13: long 5 / short 8, 13-21: 5 / 8, 21-29: 3 / 5, 29-37: 2 / 3, > 37: 1 / 1; \"an order is placed to buy 1 point above the Significant High Pivot Point or sell 1 point below the Significant Low Pivot Point\" (Sec. V Step 2)",
            "X|Money-management stop $2,100 per contract from entry (Sec. V Step 3)",
            "X|Channel exit by ADX: highest high of last 8/8/5/3/1 bars for the buy stop, lowest low of last 5/5/3/2/1 bars for the sell stop (ADX 0-13, 13-21, 21-29, 29-37, > 37), one tick beyond (Sec. V Step 4)",
            "X|Volatility filter: average over 5/5/3/2/1 bars (same ADX bands) of each bar's range = max(|H - C[1]|, |L - C[1]|); \"If long, the Sell Stop is simply the lower of either the Ready-Set-Go Exit Point or the Volatility Filter subtracted from the current bar's close\" (short mirrored) (Sec. V Steps 5-6)",
            "X|\"Once $5,000 in open profits is achieved, a trailing stop is calculated based on protecting 50% of all profits.\" (Sec. V Step 7); a new opposite significant pivot beyond the trailing stop is a stop-and-reverse (Step 2)",
            "F|Published (Sec. VI.1): weekly 04/10/1970-6/30/2002, 21 commodities: 2,240 trades, 44.64% profitable, average trade $647.57, profit factor 1.84, net $1,450,567 after $100/trade; monthly 02/26/1971-6/30/2002: 570 trades, 44.39%",
        ],
        notes: NOTE,
        claim: claim("2,240", "44.64%", "$647.57", "avg 38.9 days", "04/10/1970-6/30/2002, 21 futures, weekly"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ltrsg_ready_set_go" => ("ltrsg_rsg", "longtermtrading.com Ready-Set-Go", "Weekly, book defaults"),
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
            assert!(s.id.starts_with("ltrsg_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NeedsData(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
