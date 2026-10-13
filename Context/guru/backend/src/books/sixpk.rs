//! thedailytrade.com (Jon Andersen), "THE Six-Pack Trading System" (Six_Pack.pdf, 3 pp, plus "SEB Improved version.txt").
//! Book id `sixpk`. Source: research drive "USB HDD 2/tRADING KNO-2/The Six-Pack Trading System (thedailytrade.com)/".
//! Six oscillator systems for the E-mini, each gated by "the 34 sma [21 sma] confirms the direction of the signal".
//! The text never says what "confirms the direction" means (price above/below the SMA, or the SMA rising/falling),
//! the filter cannot be dropped, and no code or example resolves it, so every system is NotTestable. T3Osc also uses
//! the undefined MetaStock custom formulas Fml("T3") and Fml("T5").
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "sixpk";

const AMB: &str = "ambiguous filter: \"Initiate trades only if the 34 sma [21 sma] confirms the direction of the signal (the trend is your friend)\" does not say whether the close must be above/below the SMA or the SMA must be rising/falling, and the filter cannot be dropped; no code or example in the source resolves it";
const T3: &str = "undefined indicator: T3osc = Mov(Fml(\"T3\") - Ref(Fml(\"T5\"),-1),3,S) uses MetaStock custom formulas T3 and T5 that the source never defines; the 34-SMA filter is also ambiguous";
const NOTE: &str = "Written for the E-mini S&P (\"Mechanical Trading the E-Mini's\"); would run on the cash index as an instrument swap if the filter were defined. p.3 'Additional Comments': \"Stops and additional exit criteria can and should be used\" (no stop given) and \"Any momentum oscillators can be substituted\" are generic advice, recorded but not rules.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules| StrategyDef { id, name, group: "Six-Pack pp.1-3", session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: NOTE, claim: None };
    vec![
        d("sixpk_seb_classic", "1 · SEB Classic (Standard Error Bands)", Status::NotTestable(AMB), &[
            "F|\"Initiate trades only if the 34 sma confirms the direction of the signal (the trend is your friend).\"",
            "E|\"Buy the opening after price closes below the lower standard error band.\" / \"Sell short the opening after price closes above the upper standard error band.\"",
            "X|\"Cover the long position if there is a close above the upper standard error band.\" / \"Cover the short position if there is a close below the lower standard error band.\"",
            "F|Formulae p.3: \"SEB = lin. reg:8; 2 std. err.; 3sma\"",
        ]),
        d("sixpk_storsi", "2 · StoRSI (Stochastic RSI)", Status::NotTestable(AMB), &[
            "F|\"Initiate trades only if the 21 sma confirms the direction of the signal\"",
            "E|\"Buy the opening after StoRSI closes below the trigger of '10'.\" / \"Sell short the opening after StoRSI closes above the trigger of '90'.\"",
            "X|\"Cover the long position if there is a close above the trigger of '90'.\" / short: \"close below the trigger of '10'\"",
            "F|\"StoRSI = Mov( ( RSI(8) - LLV( RSI(8) ,8 ) ) / ( ( HHV( RSI(8) ,8) ) - LLV(RSI(8),8) ),3,E) *100\"",
        ]),
        d("sixpk_masterblaster", "3 · MasterBlaster (FibCMO & SEB)", Status::NotTestable(AMB), &[
            "F|\"Initiate trades only if the 34 sma confirms the direction of the signal\"",
            "E|\"Buy the opening after the FibCMO closes below the trigger of '-34'.\" / \"Sell short the opening after FibCMO closes above the trigger of '+34'.\"",
            "X|\"Cover the long position if there is a close above the upper standard error band.\" / short: \"close below the lower standard error band\"",
            "F|\"FibCMO = CMO3+CMO5+CMO8/3\" (operator precedence also unresolved: (CMO3+CMO5+CMO8)/3 or CMO3+CMO5+CMO8/3)",
        ]),
        d("sixpk_awesome", "4 · AWEsome (MovAvg Osc)", Status::NotTestable(AMB), &[
            "F|\"Initiate trades only if the 21 sma confirms the direction of the signal\"; AWE = Mov(Mov(C,3,S)-Mov(C,13,S),3,S)",
            "E|\"Buy the opening after the AWE closes below the trigger of '-.4'.\" / \"Sell short the opening after price closes above the trigger of '+.4'.\" (text says 'price', formula context says AWE)",
            "X|\"Cover the long position if there is a close above '+.4'.\" / \"Cover the short position if there is a close below '-.4'.\" (absolute E-mini points; no index scaling given)",
        ]),
        d("sixpk_t3osc", "5 · T3Osc (T3 Oscillator)", Status::NotTestable(T3), &[
            "F|\"Initiate trades only if the 34 sma confirms the direction of the signal\"",
            "E|\"Buy the opening after the T3Osc closes below the trigger of '-1.5'.\" / short above '+1.5'",
            "X|\"Cover the long position if T3Osc close is above '+1.5'.\" / short below '-1.5'",
            "F|\"T3osc = Mov(Fml(\"T3\")- Ref(Fml(\"T5\"),-1),3,S)\"",
        ]),
        d("sixpk_cmo3", "6 · CMO3 (Chande Momentum Osc)", Status::NotTestable(AMB), &[
            "F|\"Initiate trades only if the 34 sma confirms the direction of the signal\"; \"CMO3 = CMO set to 3 periods\"",
            "E|\"Buy the opening after the CMO3 closes below the trigger of '-21'.\" / \"Sell short the opening after CMO3 closes above the trigger of '+21'.\"",
            "X|\"Cover the long position if the CMO3 closes above the '+21'.\" / \"Cover the short position if the CMO3 closes below '-21'.\"",
        ]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("sixpk_").then_some(("sixpk_six_pack", "Six-Pack Trading System", "Not testable"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn all_not_testable() {
        let c = super::catalogue();
        assert_eq!(c.len(), 6);
        assert!(c.iter().all(|s| matches!(s.status, crate::connors::Status::NotTestable(_)) && super::family_of(s.id).is_some()));
    }
}
