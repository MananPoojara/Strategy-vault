//! Virgil Dasilva, "The Supreme Order" (thesupremeorder.com, 2004, 20 pp.). Book id `vdso`.
//! Source copy: Books/Virgil Dasilva/Supreme Order.pdf (from R:/USB HDD 2/tRADING KNO-2/Virgil Dasilva - Supreme Order
//! (thesupremeorder.com)/Supreme Order.pdf). A one-bar trailing stop-and-reverse. The book itself says daily bars hide
//! reversals inside expanding (outside) bars and that the intra-period price action decides them (p.6, p.13), so on
//! daily bars alone the book's own rule is not determined: NeedsData (intraday bars). No code until such data exists.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "vdso";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "vdso_supreme_order", name: "Supreme Order one-bar trailing stop and reverse", group: "The Supreme Order", session: 0, dir: "Long/Short",
        status: Status::NeedsData("intraday bars (per ticker, regular session): the book requires the intra-period price action of expanding (outside) bars to sequence the reversals (\"by looking at the intratime period price action of the expanding price bar, we can bring into view all the price activity ... and determine the correct sequence and number of turning points\", p.13); the vault has daily bars only"),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "E|\"STEP 1- RELATIVE TO A DOWNTREND, PLACE A TRAILING STOP TO BUY ONE (1) CONTRACT AT ONE TICK ABOVE THE HIGH OF THE PRICE BAR WITH THE LOWEST OR EQUAL HIGH RELATIVE TO THE IMMEDIATE PAST.\" (p.15)",
            "X|\"STEP 2- AFTER GOING LONG, IMMEDIATELY PLACE A TRAILING STOP TO SELL TWO (2) CONTRACTS AT ONE TICK BELOW THE IMMEDIATE PAST LOW. THEN AT THE CLOSE OF THE TRADING PERIOD AND BEFORE THE NEXT TRADING PERIOD COMMENCES, ADVANCE THE SELL STOP TO ONE TICK BELOW THE LOW OF THE PRICE BAR WITH THE HIGHEST OR EQUAL LOW IN THE UPTREND.\" (p.15)",
            "E|Short: \"RELATIVE TO AN UPTREND, PLACE A TRAILING STOP TO SELL ONE (1) CONTRACT AT ONE TICK BELOW THE LOW OF THE PRICE BAR WITH THE HIGHEST OR EQUAL LOW RELATIVE TO THE IMMEDIATE PAST\"; then a stop to buy two contracts one tick above the immediate past high, lowered each period to one tick above the high of the bar with the lowest or equal high in the downtrend (p.16)",
            "F|\"Relativisticly, any trend activity occurring within the high and low of the immediate past is always considered neutral\" (p.7); \"EXPANDING TREND ... price action is open to the possibility of unfolding in higher highs and lower lows relative to the immediate past ... multiple trend reversals to be plotted simultaneously\" (p.7)",
            "F|\"At times a SOH or SOL price bar may NOT be perceptible during expanding price action. But, by looking at the intratime period price action of the expanding price bar, we can bring into view all the price activity that made up the expanding price bar and determine the correct sequence and number of turning points.\" (p.13)",
        ],
        notes: "Always in the market (stop and reverse), any market and time frame (p.2, p.14). No published results. Daily-bar reading open points (also why the book's rule needs intraday data): which bar is the 'immediate past' after an inside (neutral) bar, and the order of a reversal and the new opposite stop inside one expanding bar.",
        claim: None,
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "vdso_supreme_order").then_some(("vdso", "The Supreme Order (Dasilva)", "one-bar trailing SAR"))
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
