//! Ed Downs, "Fine Tuning Oscillators", Technical Analysis of Stocks & Commodities V.10:11 (1992) pp.488-493, with the sidebar
//! "Oscillator trading systems defined" (MetaStock code; research share Trading Knowledge I/Stocks & Commodities DVD 11.26
//! (1982 - 2008) (traders.com)/VOLUMES/V10/C11/FINETUN.PDF and SIDEOSC.pdf). Book id `scedosc`: the peak/trough reversal system on
//! three oscillators; the Chaikin version is held. Lane K41 (batch T7a, 4 Nov 2026); independent check
//! authors/titles-1030/K41w/k41_ref.py.
use crate::books::msws::{Book, Sys, EXEC_DEFAULT};
use crate::connors::{Ctx, ParamDef, Params, StrategyDef, Trade};
#[allow(unused_imports)]
use crate::connors::Status::{Live, NeedsData, NotTestable};
use std::sync::OnceLock;

pub const BOOK: &str = "scedosc";
pub const REVIEWED: &[&str] = &["scedosc_stoch", "scedosc_cci", "scedosc_rsi"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const CONV: &str = "[Declared readings] The sidebar's MetaStock 3.0 WHEN() rules are written in the later MetaStock syntax (WHEN(a,<,b) = a < b). 'positions were taken at the closing price on the day that the signal was generated': fills at the signal close. MetaStock's System Tester reverses on an opposite Enter (7.2 manual, Order of Evaluation; decision 184); option reverse = 0 ignores it while a position is open. Default lookbacks = the article's defaults, Figure 4 (14-period CCI, 9-period stochastic, 14 RSI; Figure 3 lists RSI 14, stochastic 5 to 9, CCI 12 to 14); the American Airlines optimised values (CCI 8, stochastic 17, RSI 20; the optimisation grids are in each card) are options through the Book constant. The test charged 1% on entry and on exit ('2% total') and invested 100% of capital: the vault's results are before costs. Article results (American Airlines 1991, defaults): RSI -3%, stochastic -16%, CCI -17%. HELD: the Chaikin version (BL/SL = -/+ OPT1, 1,000 to 10,000 in volume units chosen per stock; no level is defined for our instruments, and the printed 'BL: OPT1 / SL: -OPT1' contradicts the text). Runs on indices, ETFs and stocks, US and India, are a declared application.";
pub static SCEDOSC: Book = Book { id: BOOK, sys: SYS, fml: &[], conv: CONV, cat: OnceLock::new(), par: OnceLock::new() };
pub fn catalogue() -> Vec<StrategyDef> { SCEDOSC.catalogue() }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { SCEDOSC.params(id) }
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> { SCEDOSC.run(id, x, from, to, p) }
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { SCEDOSC.family_of(id) }

const FAM: (&str, &str) = ("scedosc_peak_trough", "Peak/trough oscillator reversal (Downs, S&C V.10:11)");
const G: &str = "S&C: fine tuning oscillators (Downs, V.10:11)";
const SYS: &[Sys] = &[
    Sys { id: "scedosc_stoch", name: r##"Stochastic %K(9,3) peak/trough reversal, 20/80"##, group: G, family: FAM, page: "SIDEOSC.pdf p.1-2 (system), FINETUN p.4-5 (Figures 3-4)", status: Live,
        f: [Some(r##"Ref(Stoch(9,3),-1) < 20 AND Mov(Stoch(9,3),2,S) <= Stoch(9,3)"##), Some(r##"Ref(Stoch(9,3),-1) >= 20 AND Stoch(9,3) < 20"##),
            Some(r##"Ref(Stoch(9,3),-1) > 80 AND Mov(Stoch(9,3),2,S) >= Stoch(9,3)"##), Some(r##"Ref(Stoch(9,3),-1) <= 80 AND Stoch(9,3) > 80"##)],
        exec: EXEC_DEFAULT, notes: r##"IND = stoch(OPT1,3), BL +20, SL +80, OPT1 = 5,9,13,17,21,25 (sidebar p.2); default 9 (Figure 4), American Airlines optimum 17."## },
    Sys { id: "scedosc_cci", name: r##"CCI(14) peak/trough reversal, -100/+100"##, group: G, family: FAM, page: "SIDEOSC.pdf p.2-3 (system), FINETUN p.4-5 (Figures 3-4)", status: Live,
        f: [Some(r##"Ref(CCI(14),-1) < -100 AND Mov(CCI(14),2,S) <= CCI(14)"##), Some(r##"Ref(CCI(14),-1) >= -100 AND CCI(14) < -100"##),
            Some(r##"Ref(CCI(14),-1) > 100 AND Mov(CCI(14),2,S) >= CCI(14)"##), Some(r##"Ref(CCI(14),-1) <= 100 AND CCI(14) > 100"##)],
        exec: EXEC_DEFAULT, notes: r##"IND = cci(OPT1), BL -100, SL +100, OPT1 = 5,8,11,14,17,20,23 (sidebar p.3); default 14 (Figure 4), American Airlines optimum 8. The Book constant 100 sets both levels."## },
    Sys { id: "scedosc_rsi", name: r##"RSI(14) peak/trough reversal, 30/70"##, group: G, family: FAM, page: "SIDEOSC.pdf p.3 (system), FINETUN p.4-5 (Figures 3-4)", status: Live,
        f: [Some(r##"Ref(RSI(14),-1) < 30 AND Mov(RSI(14),2,S) <= RSI(14)"##), Some(r##"Ref(RSI(14),-1) >= 30 AND RSI(14) < 30"##),
            Some(r##"Ref(RSI(14),-1) > 70 AND Mov(RSI(14),2,S) >= RSI(14)"##), Some(r##"Ref(RSI(14),-1) <= 70 AND RSI(14) > 70"##)],
        exec: EXEC_DEFAULT, notes: r##"IND = rsi(OPT1), BL +30, SL +70, OPT1 = 8,11,14,17,20,23 (sidebar p.3); default 14 (Figure 3/4), American Airlines optimum 20."## },
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn three_oscillators_evaluate_and_trade() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        assert!(SCEDOSC.check_all(&x).is_empty());
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(!t.is_empty(), "{id}");
            assert!(t.iter().all(|z| z.entry_kind == "close"), "{id}");
        }
    }
}
