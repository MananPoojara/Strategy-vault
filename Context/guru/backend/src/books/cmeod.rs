//! Chris Mathews, "End of Day Trading System" (The Traders Club, Extreme Swing method), R:/Trading Knowledge III/End of
//! Day Trading System (thetradersclub.com)/End of Day Trading System.pdf; copy in Books/Chris Mathews/. Book id "cmeod".
//! Notes: library/cmeod/README.md. One method, judgement-based: not testable.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "cmeod_extreme_swing", name: "Extreme Swing: SMA 50/100/200 + BB 100/200 zone, 14/7/3 stochastic, reversal candle",
        group: "Extreme Swing (EOD forex)", session: 0, dir: "Long/Short",
        status: Status::NotTestable("judgement at every step: 'very close (say within 20-30 pips)' to a line, stochastic 'turning and touching' (shown only on charts), 'a clear reversal candle' ('If in doubt - stay out'), steps 4 and 5 'within a day or two'; stops and targets are in forex pips with a 50-150 pip band and 'at least' targets"),
        needs_volume: false, needs_vix: false, needs_series: &[], book: "cmeod",
        rules: &["F|\"Rule #1: Only ever trade when the price is touching or has pierced or is very close (say within 20-30 pips) to a major indicator line on the chart. (50,100 or 200 SMA or 100,200 BB)\" (p.20)",
            "F|\"Rule #2: Trade entries may only be considered if/when the 14/7/3 stochastic is overbought (both lines above 80 ...) or oversold (both lines below 20 ...). Further, the stochastic lines must be 'turning and touching'\" (p.21)",
            "E|\"Rule #4: There must be a clear reversal candle (or bar) on the chart which occurs at one of the zones of probability\" (p.25)",
            "E|\"The trade can be entered as soon as the NY trading day is finished, at the close of the daily candle\" or \"on a retracement\" next 1-2 days (p.43-45)",
            "X|\"The stop loss must be above/below the daily candle which gave the reversal signal - about 10 pips further ... no greater than 150 pips and no less than 50 pips\"; \"The profit target should be at least 150 pips and preferably 200 pips\"; or twice the stop (p.44, p.47)"],
        notes: "Six forex pairs, daily. No published trade statistics (\"aims for 300-400 pips per month\", p.3).", claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "cmeod_extreme_swing").then_some(("cmeod", "Mathews Extreme Swing", "EOD forex"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
