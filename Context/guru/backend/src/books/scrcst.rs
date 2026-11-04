//! Jayanthi Gopalakrishnan, "Interview: Robert W. Colby, CMT", Technical Analysis of Stocks & Commodities V.24:12 (2006) pp.48-56,
//! sidebar "Stochastics with long-term EMA filter with MetaStock" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26
//! (1982 - 2008) (traders.com)/VOLUMES/V24/C12/231INT.pdf). Book id `scrcst`. Lane K41 (batch T7a, 4 Nov 2026); independent check
//! authors/titles-1030/K41w/k41_ref.py.
use crate::books::msws::{Book, Sys, EXEC_DEFAULT};
use crate::connors::{Ctx, ParamDef, Params, StrategyDef, Trade};
#[allow(unused_imports)]
use crate::connors::Status::{Live, NeedsData, NotTestable};
use std::sync::OnceLock;

pub const BOOK: &str = "scrcst";
pub const REVIEWED: &[&str] = &["scrcst_stoch271"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const CONV: &str = "[Declared readings] The printed MetaStock system-testing rules with their current values OPT1 = 7, OPT2 = 3, OPT3 = 20, OPT4 = 271 substituted (the code's stochastic uses closes for the 7-day range and averages the ratio). The sidebar prose compares the close with 'yesterday's 271-day EMA'; the printed code (which produced the report) uses today's Mov(CLOSE,271,E), and the code is the default. 'Trades are executed at the closing price on the signal date': close fills. MetaStock's System Tester reverses on an opposite Enter (7.2 manual, Order of Evaluation; decision 184); option reverse = 0 ignores it. Sidebar report (S&P 500 futures CSI perpetual contract from 4/21/82 and the cash index from 11/28/80 to 5/10/01, $100 start, profits reinvested, no costs): 275 closed trades (224 long, 51 short), 72% winners (long 78.13%, short 45.1%), net +825.49%. Runs on indices, ETFs and stocks, US and India, are a declared application.";
pub static SCRCST: Book = Book { id: BOOK, sys: SYS, fml: &[], conv: CONV, cat: OnceLock::new(), par: OnceLock::new() };
pub fn catalogue() -> Vec<StrategyDef> { SCRCST.catalogue() }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { SCRCST.params(id) }
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> { SCRCST.run(id, x, from, to, p) }
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { SCRCST.family_of(id) }

const SYS: &[Sys] = &[
    Sys { id: "scrcst_stoch271", name: r##"7-day stochastic (3-day average) below 30 / above 70 with a 271-day EMA trend filter, at the close"##, group: "S&C: stochastics with a 271-day EMA filter (Colby interview, V.24:12)",
        family: ("scrcst_stoch271", "Stochastic with long-term EMA filter (Colby, S&C)"), page: "4-5 (sidebar code and report)", status: Live,
        f: [Some(r##"Mov((C-LLV(C,7))/(HHV(C,7)-LLV(C,7)),3,S) < .5-.01*20 AND CLOSE > Mov(CLOSE,271,E)"##),
            Some(r##"Mov((C-LLV(C,7))/(HHV(C,7)-LLV(C,7)),3,S) > .5+.01*20 OR CLOSE < Mov(CLOSE,271,E)"##),
            Some(r##"Mov((C-LLV(C,7))/(HHV(C,7)-LLV(C,7)),3,S) > .5+.01*20 AND CLOSE < Mov(CLOSE,271,E)"##),
            Some(r##"Mov((C-LLV(C,7))/(HHV(C,7)-LLV(C,7)),3,S) < .5-.01*20 OR CLOSE > Mov(CLOSE,271,E)"##)],
        exec: EXEC_DEFAULT, notes: r##"Enter long: %K(7) smoothed 3 < 30 and close > the 271-day EMA; close long: %K > 70 or close < the EMA; enter short: %K > 70 and close < the EMA; close short: %K < 30 or close > the EMA (sidebar p.4-5)."## },
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn colby_code_evaluates() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        assert!(SCRCST.check_all(&x).is_empty());
        let t = run("scrcst_stoch271", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty() && t.iter().all(|z| z.legs[0].i >= 270));
    }
}
