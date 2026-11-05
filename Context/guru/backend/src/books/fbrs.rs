//! Mebane T. Faber, "Relative Strength Strategies for Investing" (2010), reprinted as CMT Level II reading Ch.45.
//! Book id `fbrs`. Source: research share Prasad/ALL BOOKS/CMT 2-1.pdf, PDF p.1270-1291 (rules printed p.1276-1278,
//! tables 45.3-45.8 p.1278-1280). The rotation runs in books::rotation on the SPY calendar (lane K28, 3 Nov 2026;
//! check authors/titles-1030/K28w/k28_ref.py). Not coded: the 10-month SMA cash filter (Table 45.10) and the
//! 'buy top 3, sell below top 5' variant (p.1288) as separate cards; the global asset-class version (no commodity index).
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fbrs";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "fbrs_combo_top3", name: "Sector relative strength: top 3 by the 1/3/6/9/12-month average return, monthly", group: "Ch.45 · Relative strength sector rotation",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|\"The 1-, 3-, 6-, 9-, and 12-month combination simply takes an average of the five rolling returns for each sector every month to sort the 10 sectors\" (p.1277)",
            "E|\"The system invests in the top X sectors. ... For Top 3, the system is 33% invested in each of the top three sectors\" (p.1277)",
            "X|\"if a sector falls out of the top X sectors it is sold at the monthly rebalance and replaced with the sector in the top X\" (p.1278)",
            "F|\"All entry and exit prices are on the day of the signal at the close. The model is only updated once a month on the last day of the month\" (p.1278)"],
        notes: "Universe: the nine Select Sector SPDRs XLB XLE XLF XLI XLK XLP XLU XLV XLY as a declared application (the book ranks the ten Fama-French industries 1928-2009 on total returns; these ETF closes are price only). Readings: an n-month return = this month's last close / the last close n months earlier - 1; Top 3 is the default because the book uses it as its implementation example (p.1288); p.top = 1-9 and p.months = 1, 3, 6, 9 or 12 (0 = the combination) are the book's other tables. '33% invested in each' is read as equal weights reset at every month end, so a continuing holding is one trade per month. Runs on the SPY calendar; the holdings are the ETFs named in each trade. US only (no NIFTY sector index series, decision #11).",
        claim: claim("n/a", "n/a", "CAGR 13.75%", "monthly", "Fama-French 10 industries, total return, 1928-2009, combination Top 3: CAGR 13.75%, SD 18.45%, max DD -72.65% (Table 45.8, p.1279-1280)") }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "fbrs_combo_top3").then_some(("fbrs_combo_top3", "Faber sector relative strength", "Combination, Top 3"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "fbrs_combo_top3").then(|| vec![
        ParamDef { key: "top", label: "Sectors held (book: Top 1-7 tables; 3 = its example)", def: 3., min: 1., max: 9., step: 1. },
        ParamDef { key: "months", label: "Ranking period in months (0 = 1/3/6/9/12 combination; book tables 1, 3, 6, 9, 12)", def: 0., min: 0., max: 12., step: 1. }])
}
/// The rotation runs on its own account (books::rotation), not as one-ticker trades.
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
