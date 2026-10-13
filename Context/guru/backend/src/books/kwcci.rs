//! Ken Wood ("Woodie"), "Woodies CCI System" v1.3 (Woodies CCI Club / PFG Best, 2007). Catalogue only: the patterns
//! are intraday and rely on undefined proprietary indicators (see library/kwcci/README.md).
//! Source copy: Books/Ken Wood/Woodies CCI System 1.3.pdf.
use crate::connors::{ParamDef, Params, Status, StrategyDef, Trade, Ctx};

pub const BOOK: &str = "kwcci";
const ZLR: &str = "Not testable. Intraday (charts \"up to 60 minutes\" use CCI 14, p.11; entry \"with 20 seconds left in the bar\", p.22) - needs intraday bars - and not codable even with them: the ZLR also requires the Chop Zone Indicator \"brown for at least three bars\" and the Sidewinder in agreement (p.16-17), proprietary indicators whose formulas the book does not give; the 15-20 CCI-point hook rule has no single number; the exit is \"one or more\" of 9 listed exits whose details are left to the club forum (p.24).";
const TL: &str = "Discretionary: needs hand-drawn CCI trend lines (TLB, GB100, Ghost) or judged \"rounding\" shapes (Vegas, 3-10 bars), plus the proprietary LSMA colour/CZI; exits are the undefined menu of p.24. Intraday charts.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, why| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: why, claim: None };
    use Status::*;
    vec![
        d("kwcci_zlr", "Zero-line reject (ZLR)", "Trend continuation patterns", NotTestable(ZLR), &[
            "F|\"A trend is established any time the CCI (black line) is either over or below the zero-line (ZL) for six or more consecutive time bars\" p.12",
            "E|ZLR: trend established, CCI reaches +/-100, retraces inside the 100 line towards the ZL, then hooks back with the trend by 15-20 CCI points; CZI and Sidewinder confirm p.16-17",
            "X|Initial stop \"one or two ticks above or below the entry bar\" p.23; exits: 9 listed methods (trend line break, 100-line cross, TCCI hook, extremes hook, profit target, flat top, horizontal line, LSMA disagreement, zero-line cross) p.24"], ZLR),
        d("kwcci_tlb", "Trend line break (TLB) and GB100", "Trend continuation patterns", NotTestable(TL), &["E|\"When the CCI line crosses the trend line a trade ... signal\" p.18; GB100 p.20"], TL),
        d("kwcci_famir_vegas_ghost", "Famir, Vegas and Ghost (trend-changing)", "Trend changing patterns", NotTestable(TL), &["E|Famir: a ZLR that immediately fails p.25; Vegas: CCI past +/-200, rounding of 3-10 bars, break of the horizontal line p.26; Ghost: three CCI peaks with a neckline p.27"], TL),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("kwcci_").then_some(("kwcci", "Woodies CCI patterns", "intraday"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
