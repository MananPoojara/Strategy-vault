//! Adam White, "The Derivative Moving Average", Technical Analysis of Stocks & Commodities V.14:6 (June 1996) pp.253-257, and its
//! sidebar "MetaStock syntax" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/
//! VOLUMES/V14/C06/THEDERI.pdf). Book id `scwdma`. Lane PD (A1 pool D, 13 Nov 2026), batch-h H4 to-code row. Run on the shared
//! MetaStock engine `msws` with the sidebar's formulas (Fml("TAI") written out in the Close Long formula so the constants stay
//! one set). Independent check authors/pd/w/pd_ref.py.
use crate::books::msws::{Book, Sys, EXEC_DEFAULT};
use crate::connors::{Ctx, ParamDef, Params, StrategyDef, Trade};
use crate::connors::Status::Live;
use std::sync::OnceLock;

pub const BOOK: &str = "scwdma";
pub const REVIEWED: &[&str] = &["scwdma_dma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const CONV: &str = "MetaStock System Tester defaults (the article states no fill): signals at the close, filled at that close; the Close formula is checked from the bar after entry. Long only, as the article tests ('long positions only, ideally entering when the market rises and exiting to cash when the market falls'); no stop ('a strategy rather than a trading system because there is no risk control').";
const SYS: &[Sys] = &[
    Sys { id: "scwdma_dma", name: "White derivative moving average: long when the 28-day SMA turns up from a 4-bar low, out when the trend analysis index falls through 0.40", group: "S&C: The Derivative Moving Average (White, V.14:6)", family: ("scwdma_dma", "White derivative moving average (S&C)"), page: "V.14:6 p.254 (Construction), p.257 (MetaStock syntax)", status: Live,
        f: [Some("Ref(Mov(C,28,S),-1) = LLV(Mov(C,28,S),4)"),
            Some("((HHV(Mov(C,28,S),5)-LLV(Mov(C,28,S),5))/C)*100 < 0.4 AND Ref(((HHV(Mov(C,28,S),5)-LLV(Mov(C,28,S),5))/C)*100,-1) >= 0.4"), None, None],
        exec: EXEC_DEFAULT, notes: "[Declared readings] The sidebar prints (MetaStock for Windows): Enter Long: when(ref(mov(c,28,s),-1),=,llv(mov(c,28,s),4)); Close Long: when(fml(\"TAI\"),<,0.4) and when(ref(fml(\"TAI\"),-1),>=,0.4); Fml(\"TAI\") is ((hhv(mov(c,28,s),5)-llv(mov(c,28,s),5))/c)*100. The article's text: 'Enter long when the moving average's previous value is the lowest value in the last four bars' (the four bars include today, as llv(...,4) does); the TAI exit level 0.40. The article tested daily S&P 500 data (its Figure 5 results are optimised and printed only as an image, so there is no book table to check). Runs on indices, ETFs and stocks, US and India, are a declared application." },
];
static WD: Book = Book { id: BOOK, sys: SYS, fml: &[], conv: CONV, cat: OnceLock::new(), par: OnceLock::new() };

pub fn catalogue() -> Vec<StrategyDef> { WD.catalogue() }
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { WD.family_of(id) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { WD.params(id) }
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> { WD.run(id, x, from, to, p) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn derivative_ma_trades_long() {
        let r = super::super::ctm::tests::walk(3000);
        let x = Ctx::new(&r, &r, &[]);
        assert!(WD.check_all(&x).is_empty());
        let t = run("scwdma_dma", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().all(|z| z.dir > 0.));
        for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i - 1); }
        assert!(family_of("scwdma_dma").is_some() && params("scwdma_dma").is_some());
    }
}

// Instrument admission: S&C White derivative moving average 2026-11-13 (lane PD).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scwdma" && reviewed(def.id),
    label: "Adam White illustrated the strategy on the daily S&P 500; runs on other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.14:6 pp.253-257 (The Derivative Moving Average) and its MetaStock sidebar; independent check authors/pd/w/pd_ref.py",
};
