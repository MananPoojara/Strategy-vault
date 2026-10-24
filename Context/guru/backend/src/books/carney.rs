//! Scott M. Carney - harmonic trading. Book ids `smcht` (The Harmonic Trader, 1999), `smcv1` / `smcv2` (Harmonic Trading Vol I / II, 2010).
//! Reading: claude-continuation-20261020/authors/carney/agents/. Catalogue only: every pattern is measured between swing points
//! X-A-B-C-D picked on the chart (no swing or pivot rule is given), and the trade-management rules 'allow for some discretion' (Vol I Ch.11).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Read 2026-10-24 (authors/carney/agents).", claim: None };
    vec![
        d("smcv1_patterns", "Harmonic patterns: AB=CD, Bat, Gartley, Crab, Butterfly", "Harmonic Trading Vol I · Ch.3-8", "smcv1",
          "the swing points X, A, B, C, D are picked on the chart (no swing-size or pivot rule is given) and entries in the Potential Reversal Zone depend on reading the price action",
          &["E|When the legs meet a pattern's Fibonacci ratios (e.g. Bat, p.72: B below a 0.618 retracement of XA, C 0.382-0.886, BC projection at least 1.618, D at the 0.886 XA retracement), buy (sell) in the PRZ at D", "X|Initial profit objective 0.382-0.618 of AD; stop-loss zone beyond the PRZ; 0.382 trailing stop (Ch.9-11)"]),
        d("smcv2_rsibamm", "RSI BAMM and the new patterns (Shark, 5-0)", "Harmonic Trading Vol II · Ch.3-6", "smcv2",
          "needs the hand-picked price pattern, and 'the real application of the RSI BAMM principles requires a degree of flexibility' (p.167)",
          &["F|RSI(14) beyond 30/70 with a trigger bar and a confirmation bar forming a 1.13-1.618 structure, then a harmonic pattern completion (Ch.6)"]),
        d("smcht_patterns", "Harmonic patterns, first edition (AB=CD, Gartley 222, Butterfly, Three Drives)", "The Harmonic Trader (1999)", "smcht",
          "same hand-picked patterns as Vol I, which restates and extends them",
          &["E|Buy or sell at the completion (PRZ) of a Fibonacci-measured pattern"]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id.starts_with("smcht_") || id.starts_with("smcv1_") || id.starts_with("smcv2_")).then_some(("smc_nt", "Carney harmonic patterns (held)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
