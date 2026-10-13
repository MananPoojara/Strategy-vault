//! John F. Ehlers, *MESA and Trading Market Cycles* (Wiley, 1992). Source: library/ehmtc/.
//! Every rule depends on the dominant cycle and predictions produced by the 1992 MESA program
//! (maximum-entropy spectral analysis software; the book gives no code for it) and on reading
//! its spectrum display, so nothing here is run.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ehmtc";
const DC: &str = "MESA_DC: daily dominant cycle length from the 1992 MESA program (maximum entropy spectral analysis; algorithm not given in the book) - no such series exists";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("ehmtc_eli", "Ehlers Leading Indicator (ELI) crossing", "Ch. 6 · Leading indicators", Status::NeedsData(DC),
          &["F|Detrended synthetic price = EMA(alpha = 6/DC) - EMA(alpha = 3/DC) of price (quarter- and half-cycle EMAs). (p.68)",
            "F|ELI = (synthetic price - its EMA with alpha = 6/DC) / .682. (pp.68-69)",
            "E|\"When the indicator crosses the synthetic price from the top, sell on the next period ... buy when the indicator crosses the synthetic price from the bottom.\" (p.67)"],
          "Also open: which price (close or other) feeds the EMAs is not stated. Not coded until a MESA dominant-cycle series exists."),
        d("ehmtc_accel_stop", "Accelerating cycle stop", "Ch. 7 · Setting stops", Status::NotTestable("exit only, attached to discretionary MESA entries; also needs the MESA dominant cycle"),
          &["X|Initial stop: entry-day low less the average daily range over half the dominant cycle (long). (p.72)",
            "X|Stop(D+1) = Low(D) - (1 - 2*Age/DC)*(Low(D) - Stop(D)). (p.73)"], ""),
        d("ehmtc_mode_trading", "Trend-mode / cycle-mode trading with MESA", "Ch. 11-12 · Putting it all together / Trading with MESA",
          Status::NotTestable("judgement: daily procedure reads the MESA spectrum, prediction quality and SAR choice (pp.110-123)"),
          &["E|Trend Mode if price has not touched the instantaneous trendline within the last half dominant cycle; use a trend-following technique such as a double moving average or parabolic SAR.",
            "E|Cycle Mode: trade turning points predicted by MESA when backtested predictions are consistent."], ""),
        d("ehmtc_osc", "Cycle-tuned RSI / Stochastic / MACD", "Ch. 6 · How cycles help trading", Status::NotTestable("indicator tuning advice; no entry/exit rule"),
          &["F|Set the RSI/Stochastic observation period to half the dominant cycle."], ""),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("ehmtc_").then_some(("ehmtc", "Ehlers MESA & Trading Market Cycles (1992)", "MESA program required"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
