//! Vladimir Daragan, How to Win the Stock Market Game / Developing Short-Term Stock Trading Strategies (same text) and Short-Term Trading Analysis (1999). Catalogue only: no rule in this source is both exact and complete. Source: library/dgstt/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "dgstt", claim: None };
    vec![
        d("dgstt_basic", "Basic Trading Strategy", "STTA ch.5", "Long", Status::NeedsData("cross-sectional selection: each day buy the 2 stocks with the lowest CLO1/CLO0 among the potentially bullish list of ~1,000 US active stocks (volume > 200,000, price > $10), with T normalised against all active stocks that day; the engine runs one instrument at a time and the vault has no such universe"),
          &["F|Potentially bullish = active stock in T-D cell 13 (T < -1 and D < -1, normalised) in the 16 or 32 day frame (STTA p.24-27)", "E|Before the close of day 1 buy the two stocks with maximal % drop on day 1 (p.31)", "X|Sell at the open of day 4 (p.31)"],
          "Book: 11/1995-5/1999, 861 days, 3.25% per purchase +/- 8.2, 67.8% positive (p.33)."),
        d("dgstt_low_risk", "Low risk strategy (both picks down > 10%)", "STTA ch.6", "Long", Status::NeedsData("same cross-sectional US active-stock universe as the Basic Trading Strategy"),
          &["E|Two stocks with maximum decline on day 1, both down more than 10% (Table 6.1, p.36)", "X|Sell at the open of day 4"],
          "Book: 112 trading days 1995-1999, 6.95% per purchase, 77.7% positive (p.38)."),
        d("dgstt_very_low_risk", "Very low risk strategy (two-day drop)", "STTA ch.6.3", "Long", Status::NeedsData("same cross-sectional universe; K1/K2 choice only tabulated"),
          &["E|Buy at CLO2 if CLO1 < K1*CLO0 and CLO2 < K2*CLO1 (p.39)", "X|Sell on the morning of day 5 (p.40)"],
          "Returns tabulated by K1, K2 (Tables 6.5-6.6)."),
        d("dgstt_short", "Selling short overbought stocks", "STTA ch.10", "Short", Status::NotTestable("universe-relative T/D thresholds and no single exit (close of day 3 or 4) are given; event study tables only"),
          &["E|Overbought in 16 and 32 day frames, rose the previous day, OPE1 > K1*CLO0; short at the open (p.51-53)", "X|\"The optimal time to close your short position is the market closing on day #3 or #4\" (p.53)"],
          ""),
        d("dgstt_oversold16", "Oversold 16-day (T < -0.5581, D < -1.185) event study", "How to Win part 4", "Long", Status::NotTestable("an event study: the holding period N = 1..9 days is the study variable, no exit rule is fixed (p.44)"),
          &["E|Oversold within 16 days: T < -0.5581 and D < -1.185 (p.44)"],
          ""),
        d("dgstt_upgrades", "Analyst upgrade pullback", "STTA ch.11.3", "Long", Status::NotTestable("discretionary (\"buy this stock when pullback is over\") and needs intraday upgrade news"),
          &[],
          ""),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
