//! M. A. Perry, "The Strategy Lab: Swing Trading Report" v2.2 (thestrategylab.com, 2008). Catalogue only: no rule set
//! in this report is testable (see library/mapsl/README.md). Source copy: Books/M. A. Perry/The Strategy Lab.pdf.
use crate::connors::{ParamDef, Params, Status, StrategyDef, Trade, Ctx};

pub const BOOK: &str = "mapsl";
const ZONE: &str = "Not testable. (1) Every pattern \"must occur within a WRB S/R Zone ... (no exceptions)\" (p.5) and \"Pattern signals that do not occur within the context of WRB S/R Zones shall be consider not valid for trading\" (p.7); the zone is defined only in the separate paid WRB Analysis Tutorials, not in this report. (2) The exits are not in the report: initial stop, trailing stop, trigger price and profit targets are only at the external link thestrategylab.com/str/trdmgmtstr.htm (p.29, 35, 43 ...), plus discretionary exits (\"Exit your Long position at a small loss prior to the initial stop/loss being reached\", p.27). (3) Recommended swing intervals are 15-60 minute charts (p.18).";
const PAIR: &str = "Not testable: pair divergence needs a second, correlated instrument chart (e.g. ES vs NQ, VIX vs RVX, NYSE A/D line) and chart reading of swing points and head-and-shoulders patterns (p.63-114); exits are only at the external trade-management link; WRB S/R zone context required (p.5).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, rules, why| StrategyDef { id, name, group, session: 0, dir, status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: why, claim: None };
    vec![
        d("mapsl_wrb_harami", "WRB Bullish Harami (AJCTR)", "Candlestick patterns (AJCTR)", "Long", &[
            "F|Dark WRB Hidden GAP (c3) --- White Small Line (c2) --- White Confirmation Line (c1); 11 numbered conditions, e.g. \"1. Dark WRB (c3) is an unfilled Hidden GAP\", \"3. Close of White Confirmation Line (c1) < Open of Dark WRB (c3)\" p.33",
            "F|WRB: \"the body of the interval is greater ... than the bodies of each of the most recent prior three intervals\" p.10",
            "X|\"Your initial stop should be placed at the Close of the second Dark WRB\" (re-entry contingency only) p.35; all other trade management at thestrategylab.com/str/trdmgmtstr.htm"], ZONE),
        d("mapsl_white_hammer", "Bullish White Hammer (AJCTR)", "Candlestick patterns (AJCTR)", "Long", &["F|White hammer body/shadow conditions 1-7 p.21-23"], ZONE),
        d("mapsl_wrb_engulfing", "WRB Bullish Engulfing (AJCTR)", "Candlestick patterns (AJCTR)", "Long", &["F|Pattern conditions p.36-43"], ZONE),
        d("mapsl_bear_engulf_star", "Bearish WRB Engulfing White Shooting Star (AJCTR)", "Candlestick patterns (AJCTR)", "Short", &["F|Pattern conditions p.44-48"], ZONE),
        d("mapsl_dark_inv_hammer", "Bearish Dark Inverted Hammer (AJCTR)", "Candlestick patterns (AJCTR)", "Short", &["F|Pattern conditions p.49-62"], ZONE),
        d("mapsl_pair_div", "Pair divergence and H&S divergence (APAOR / STR)", "Pair divergence (APAOR, STR)", "Long/Short", &["F|Bullish/Bearish Pair Divergence, Bullish Dark WRB Divergence, H&S pair divergences, WRB Bullish Two Soldiers p.63-114"], PAIR),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("mapsl_").then_some(("mapsl", "Strategy Lab swing report", "not testable"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
