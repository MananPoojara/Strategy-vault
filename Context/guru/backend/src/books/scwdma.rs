//! Adam White, "The Derivative Moving Average", Technical Analysis of Stocks & Commodities V.14:6 (June 1996) pp.253-257, and its
//! sidebar "MetaStock syntax" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/
//! VOLUMES/V14/C06/THEDERI.pdf). Book id `scwdma`. Lane PD (A1 pool D, 13 Nov 2026), batch-h H4 to-code row. Run on the shared
//! MetaStock engine `msws` with the sidebar's formulas (Fml("TAI") written out in the Close Long formula so the constants stay
//! one set). Independent check authors/pd/w/pd_ref.py.
//! R4 #326 (lane X5, review authors/rev/PD.md B2): the same formulas, engine and fills as the live mstt_the_derivative_ma (whose notes
//! name this article), so the card is marked "Duplicate of mstt_the_derivative_ma" (NotTestable, no ADMISSION) and counted once; the
//! article is cited on mstt_the_derivative_ma's gate row. The test runs these formulas as a Live copy and checks they give the
//! live card's trades.
use crate::books::msws::{Book, Sys, EXEC_DEFAULT};
use crate::connors::{Ctx, ParamDef, Params, StrategyDef, Trade};
use crate::connors::Status::NotTestable;
use std::sync::OnceLock;

pub const BOOK: &str = "scwdma";
pub const REVIEWED: &[&str] = &["scwdma_dma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const DUP: &str = "Duplicate of mstt_the_derivative_ma (R4 #326): Adam White's derivative moving average from S&C V.14:6 (sidebar MetaStock formulas, close fills, long only) is the live card mstt_the_derivative_ma, which runs the same formulas on the same MetaStock engine and cites this article; counted once, not run as a separate strategy";
const CONV: &str = "MetaStock System Tester defaults (the article states no fill): signals at the close, filled at that close; the Close formula is checked from the bar after entry. Long only, as the article tests ('long positions only, ideally entering when the market rises and exiting to cash when the market falls'); no stop ('a strategy rather than a trading system because there is no risk control').";
const SYS: &[Sys] = &[
    Sys { id: "scwdma_dma", name: "White derivative moving average: long when the 28-day SMA turns up from a 4-bar low, out when the trend analysis index falls through 0.40", group: "S&C: The Derivative Moving Average (White, V.14:6)", family: ("scwdma_dma", "White derivative moving average (S&C)"), page: "V.14:6 p.254 (Construction), p.257 (MetaStock syntax)", status: NotTestable(DUP),
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
    fn derivative_ma_is_the_live_mstt_card() {
        let r = super::super::ctm::tests::walk(3000);
        let x = Ctx::new(&r, &r, &[]);
        let s = &SYS[0];
        assert!(run("scwdma_dma", &x, 0, r.len() - 1, &Params::new()).is_none() && family_of("scwdma_dma").is_some());
        // the article's formulas run as a Live copy give exactly the live card's trades
        let live: &'static [Sys] = Box::leak(Box::new([Sys { id: s.id, name: s.name, group: s.group, family: s.family, page: s.page, status: crate::connors::Status::Live, f: s.f, exec: s.exec, notes: "" }]));
        let b: &'static Book = Box::leak(Box::new(Book { id: BOOK, sys: live, fml: &[], conv: CONV, cat: OnceLock::new(), par: OnceLock::new() }));
        assert!(b.check_all(&x).is_empty());
        let t = b.run("scwdma_dma", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let m = super::super::mstt::run("mstt_the_derivative_ma", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().all(|z| z.dir > 0.));
        let key = |v: &[Trade]| v.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px, z.open)).collect::<Vec<_>>();
        assert_eq!(key(&t), key(&m));
    }
}

// No ADMISSION: duplicate of mstt_the_derivative_ma (R4 #326).
