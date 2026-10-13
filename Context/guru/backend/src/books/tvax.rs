//! TradingVisions Systems (Lincoln Fiske Jr.), "Client Instructions for AXIOM Index Trader" (2004, Word document) with
//! the encrypted TradeStation file "AXIOM INDEX (C).ELD". Book id `tvax`. Source: research drive "USB HDD 1/TradeStation
//! Systems & Indicators/AXIOM Index Swing System (tradingvisions.com)/", copied to Books/TradingVisions/. The .doc has no
//! page numbers; refs are its section headings. The .ELD is encrypted and was not read.
//! Not testable: the document states that "should there be a discrepancy between this document and the software, defer
//! to the software", and its own three statements of the entry EMAs disagree (see notes), so the entry cannot be fixed
//! from the text. It would also need 30-minute (SP/MD/ES) or 60-minute (RL/ER) back-adjusted futures bars and the
//! contracts' point values for its dollar stops. `run` returns None.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tvax";

const WHY: &str = "entry rule not fixed by the text: the document defers to the encrypted TradeStation code, and its Buy/Sell Condition (EMA5hlc vs 'EMA89h', EMA1hlc vs 'EMA144hlc' / 'EMA144h', 'EMA89hlc') disagrees with its own worked example ('89hlc', '144L'); the volatility precondition (\"difference between the highest level ema and the lowest\" < $15,000) does not say which EMAs. Even with a fixed reading it would need 30-minute (SP, MD, ES) / 60-minute (RL, ER) back-adjusted futures bars, the cash daily bars in data2, and each contract's point value for the $ stops, profit locks and target";

const NOTE: &str = "Designed for S&P 500, S&P 400 MidCap, Russell 2000 (full and e-mini) and DAX futures; 'All entry orders are at the market' on the close of a data1 bar; stop-and-reverse. Dollar values are for SP/RL/MD; DAX half, e-minis one-fifth. No published performance table is in the document (only the 4/13/04 and 4/28/04 MidCap examples).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "tvax_axiom", name: "AXIOM Index swing system (Fibonacci EMA confirmation, stop-and-reverse)",
        group: "System Rules for AXIOM", session: 0, dir: "Long/Short", status: Status::NotTestable(WHY),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"A theoretical risk number is derived from the difference between the highest level ema and the lowest. This risk must be less than $15,000\" (Required Preconditions)",
            "F|\"The 2-day average true range (ATR) divided by the 34-day ATR must be less than 1.2\"; \"The average daily ATR (measured over the last 34 days) must be at least $1250\"",
            "E|Buy: \"When, upon the closure of the data1 bar, the 5-period EMA of the high+low+close (of data1) divided by 3 (EMA5hlc) crosses and closes above the EMA89 of the high (EMA89h) and the EMA1hlc is already above the EMA144hlc, or when the EMA1hlc crosses and closes above the EMA144h and the EMA5hlc is already above the EMA89hlc, a signal is alerted to buy at the market.\"",
            "E|Sell: \"When, upon the closure of the data1 bar, the 5-period EMA of the high+low+close (of data1) divided by 3 (EMA5hlc) crosses and closes below the EMA89 of the high (EMA89h) and the EMA1hlc is already below the EMA144hlc, or when the EMA1hlc crosses and closes below the EMA144h and the EMA5hlc is already below the EMA89hlc, we sell at the market.\"",
            "X|\"Immediately upon a long entry, a protective stop is placed to risk the appropriate point equivalent of $3250\"; \"A long position may reverse to short when the sell condition below is met\"",
            "X|Profit locks: contracts $100-$500 per big point (SP): > $2,500 profit -> lock $1,000; contracts >= $500 per point (RL, MD): > $3,000 -> lock $1,000; > $10,000 -> lock $2,000; > $15,000 -> lock $3,000; \"A profit objective limit order of $20,000\"; \"Exits for Shorts: Mirror opposite of longs.\"",
            "F|Chart setup: \"Data1 SP 30-minute (day session only) continuous backadjusted contract, Data2 S&P Daily Bar Cash\" (RL/ER: 60-minute)",
        ],
        notes: NOTE,
        claim: None,
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "tvax_axiom" => ("tvax_axiom", "TradingVisions AXIOM Index", "Document rules"),
        _ => return None,
    })
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn catalogue_is_not_testable_and_owned() {
        for s in &super::catalogue() {
            assert!(s.id.starts_with("tvax_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NotTestable(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
