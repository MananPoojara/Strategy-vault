//! longtermtrading.com (Alan Pryor), "Anomaly1 - A Long Term Trading System", System Manual v1.02a (30 June 2002) with the
//! TradeStation entry code AN1V101.TXT ("Anomaly1 - with Pivot Point Exit"). Book id `ltan1`. Source: research drive
//! "USB HDD 1/TradeStation Systems & Indicators/Anomaly I + Manual for TS (longtermtrading.com)/" (+ AN1V101.TXT from
//! "Anomaly 1 Trading System (longtermtrading.com)/"), copied to Books/longtermtrading.com/Anomaly I/. Section/page refs
//! are the manual's TOC pages (Sec. IV p.18, Sec. V p.22-29, Sec. VI p.30).
//! The system is defined for, and tested only on, a 21-commodity futures portfolio (no stock-index contract) with a
//! $2,100-per-contract money-management stop and a $5,000-open-profit / 50% retention stop. Those dollar rules have no
//! stated stock-index equivalent and must not be dropped, so the strategy is NeedsData; `run` returns None.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ltan1";

const FUT: &str = "needs weekly back-adjusted futures contracts of the manual's standard 21-commodity portfolio (C, CC, CL, CT, EC, ED, GC, HG, HU, JY, KC, LC, LH, NG, S, SB, SF, SI, SM, TY, US; 1970-2002) with each contract's big-point value, because the $2,100-per-contract money-management stop and the $5,000 / 50% profit-retention stop are dollar rules; the manual defines no stock-index contract, so no index instrument swap exists";
const NOTE: &str = "Weekly bars (a monthly test is also published). Code (AN1V101.TXT) and manual disagree on the entry Lookback table: code bands ADX(13) < 16 -> 5, 16-24 -> 3, 24-32 -> 2, >= 32 -> 1 (Length2 8 + Length3 8 steps); manual table 0-8 -> 5, 8-16 -> 5, 16-24 -> 3, 24-32 -> 2, > 32 -> 1 (same result). Signal #2 in code uses Buy zone 100 - MinDBuy2 and requires DMI+ > DMI- (manual Sec. IV says only 'neutral'); the manual's worked example quotes ADX(8) while Step 1 says 13 bars. The pivot exits are 'IncludeSystem: An1 Long/Short Exit 1.02', whose code is not in the text file; its inputs (LExADXSt 13, LExADXIn 8, long bars 5/3/2/1, short bars 8/5/3/1, i.e. ADX < 21, 21-29, 29-37, >= 37) disagree with the manual's 5-band table in the 0-21 range (manual: 0-13 short 5 / long 8, 13-21 short 8 / long 5); both agree from 21 up (the Trade 3 example, ADX 25.2 -> 3 bars, fits both). Primary entry code uses 'High + 1 Point' while the manual says '+1 tick'. If coded later these must be resolved from the protected ELA, not guessed.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "ltan1_anomaly1", name: "Anomaly1 (SlowD zone breakout, adaptive pivot exits)",
        group: "Sec. IV-V · Anomaly1 system", session: 0, dir: "Long/Short", status: Status::NeedsData(FUT),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"Calculate the 13 Bar ADX, DMI+, and DMI-\" (Sec. V Step 1); SlowD(13) (code: SlowD(Length1), Length1 = 13)",
            "E|Primary: \"Once the SlowD passes above the minimum level of the Buying Zone, an order is placed to Buy at the current Bar's high + 1 tick. If the SlowD passes below the maximum level of the Selling Zone, an order is placed to Sell at the current Bar's low - 1 tick.\" Buy zone minimum = 100 - 34 = 66, sell zone maximum = 100 - 55 = 45 (Sec. V Step 2)",
            "E|Secondary: \"The setup occurs when your market position is neutral and the SlowD has remained in the Buying Zone or Selling Zone continuously for the last 'x' number of bars (the 'Lookback' period)\"; Lookback by ADX: 0-8 -> 5, 8-16 -> 5, 16-24 -> 3, 24-32 -> 2, > 32 -> 1 (Sec. V Step 3); code also requires DMI+ > DMI- (long) / DMI+ < DMI- (short)",
            "X|Money-management stop: \"a stop-loss exit price is also determined that is exactly $2,100 away in the opposite direction from the entry price\" (Sec. IV.4, Sec. V Step 4)",
            "X|Pivot exit: significant low/high pivot when bars from 1 bar before the pivot to the previous high/low >= table value; ADX 0-13: short 5 / long 8, 13-21: 8 / 5, 21-29: 5 / 3, 29-37: 2 / 2, > 37: 1 / 1; \"exit a short position 1 point above the Significant High Pivot Point or exit a long position 1 point below the Significant Low Pivot Point\" (Sec. V Step 5)",
            "X|\"Once $5,000 in open profits is achieved based on the highest high or lowest low achieved from the trade entry point, a trailing stop is calculated based on protecting 50% of all profits.\" (Sec. V Step 6)",
            "F|Published (Sec. VI.1): weekly 04/10/1970-6/30/2002, 21 commodities: 1,768 trades, 41.06% profitable, average trade $951.64, profit factor 1.91, net $1,682,505 after $150/trade; monthly 02/26/1971-6/30/2002: 659 trades, 40.52%",
        ],
        notes: NOTE,
        claim: claim("1,768", "41.06%", "$951.64", "avg 62.1 days", "04/10/1970-6/30/2002, 21 futures, weekly"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ltan1_anomaly1" => ("ltan1_anomaly", "longtermtrading.com Anomaly1", "Weekly, book defaults"),
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
            assert!(s.id.starts_with("ltan1_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NeedsData(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
