//! Gary Ford, Systems Trading for Spread Betting (Harriman House, 2008).
//! Every complete system in the book runs on 60-minute or 6-hour forex bars in NinjaTrader; the vault
//! has daily bars only, so none is Live (a timeframe change is not an allowed mapping). The daily
//! examples in ch.8 are entry illustrations without exits or lengths. Sources: library/gfstsb/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "gfstsb";
const H1: &str = "needs 60-minute bars (book: NinjaTrader 60-minute forex bars); the vault has daily bars only";
const H6: &str = "needs 6-hour bars (book: GBPUSD and 10 other forex pairs on six-hour bars); the vault has daily bars only";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        make("gfstsb_ema_cross", "EMA 9/45 crossover, stop and reverse", "Ch.11 Moving Average Example (p.97-120)", "Long/Short", Status::NeedsData(H1),
            &["E|\"Long entry occurs if the EMA(Close[0], Fast Bar Period) crosses above the EMA(Close[0], Slow Bar Period).\" (p.98)",
              "E|\"Short entry occurs if the EMA(Close[0], Fast Bar Period) crosses below the EMA(Close[0], Slow Bar Period).\" (p.98)",
              "X|\"We will not use a stop or profit target ... always has an open position\"; an opposite signal closes and reverses (p.97-98).",
              "F|Defaults FastEMA 9, SlowEMA 45 (Figure 11.4 parameter page, p.100; \"I have left the values set to the default values\" p.118). Calculate on bar close = fill at the next bar's open (Fig. 11.3 p.100; p.174-175 explains next-bar fills).",
              "F|\"I have configured the strategy to run on a 60 minute bar.\" (p.118); EURJPY, ~8 years of 1-minute data (p.115)."],
            "Results only as a screenshot (Figure 11.23) for EURJPY; optimisation (ch.13, Table 13.4) picked 9/20 on hourly EURJPY 1 Apr 2000 - 1 Jan 2007 (optimised, not the default)."),
        make("gfstsb_channel", "SMA(High/Low,200) channel cross with EMA 5/10 exit", "Ch.14 Putting It All Together (p.149-190)", "Long/Short", Status::NeedsData(H1),
            &["E|\"Open Long: If Close[0] crosses above SMA(High, 200)\"; \"Open Short: If Close[0] crosses below SMA(Low, 200)\" (Table 14.4)",
              "X|\"Close Long: If EMA(FastEMA[0]) crosses below EMA(SlowEMA[0])\"; \"Close Long: If Close[0] < SMA(High, 200)\" (Table 14.4)",
              "X|\"Close Short: If EMA(FastEMA[0]) crosses above EMA(SlowEMA[0])\"; \"Close Short: If Close[0] > SMA(Low, 200)\" (Table 14.4)",
              "F|Defaults FastEMA 5, SlowEMA 10 (Table 14.3, p.155); channel SMA 200 (Table 14.4, p.174). Optimised (190, 2, 20) is a later fitted set (Table 14.13), not the default.",
              "F|60-minute bars, CalculateOnBarClose = true: fills at the open of the next bar (p.174-175; 60-minute bars p.181)."],
            "Forex basket results (Tables 14.5-14.15) are on hourly bars only."),
        make("gfstsb_inside_bar", "Inside bar breakout (trailing stop)", "Appendix A (p.221-227)", "Long/Short", Status::NeedsData(H6),
            &["E|Two inside six-hour bars: High[2] < High[3], Low[2] > Low[3], High[1] < High[2], Low[1] > Low[2]; long if Close[0] > High[3], short if Close[0] < Low[3] (Table 19.1)",
              "X|\"Exit the position on a trailing stop.\" 100 (pips) in Table 19.2 (p.227)"],
            "GBPUSD 1/1/2004-1/1/2007: net -3,560, PF 0.76, 25.71% profitable (Table 19.2). Stop is in forex points; no index equivalent is given."),
        make("gfstsb_inside_bar_sma", "Inside bar with SMA reversion (parts 1 and 2)", "Appendix A (p.228-240)", "Long/Short", Status::NeedsData(H6),
            &["E|Two inside bars and Close[1] < SMA[0] -> long; Close[1] > SMA[0] -> short (Table 19.3); part 2 adds Close[0] > Close[1] / < Close[1] (Table 19.5)",
              "X|Close long when High[0] >= SMA[0]; close short when Low[0] <= SMA[0] (Tables 19.3, 19.5)",
              "X|Trailing stop 200 (part 1, Table 19.4) / static stop-loss 100 (part 2, Table 19.6); SMA 30"],
            "Six-hour forex bars; stop sizes in forex points."),
        make("gfstsb_perfect_order", "Moving-average perfect order", "Appendix A (p.241-256)", "Long/Short", Status::NeedsData(H1),
            &["E|Long: SMA1 > SMA2 > SMA3 > SMA4 > SMA5; short: the reverse (Table 19.11); SMA 10, 20, 50, 100, 200 (Table 19.12)",
              "X|Close long when SMA1 crosses below SMA2; close short when SMA1 crosses above SMA2 (Table 19.11)"],
            "GBPUSD 2006: net 3,020, PF 1.11, 35.64% profitable (Table 19.12); optimisation tables state 60-minute bars (Tables 19.13, 19.16)."),
        make("gfstsb_daily_ema_example", "Daily EMA crossover illustration", "Ch.8 Trade Entry (p.70-71)", "Long/Short",
            Status::NotTestable("EMA lengths are not stated (Figure 8.1 legend is unreadable at the printed resolution); borrowing 9/45 from the hourly ch.11 system would be another system's parameters"),
            &["E|\"When the fast moving average ... crosses up through the slow moving average ... a buy signal is generated. Conversely ... a sell signal\" (p.71)", "X|\"the position is reversed when the signals are generated\" (p.71)"], ""),
        make("gfstsb_macd_example", "MACD entry illustration", "Ch.8 Trade Entry (p.72-73)", "Long/Short",
            Status::NotTestable("entry example only: no exit is given, and 'above its 9 day EMA or the centreline' leaves the trigger open"),
            &["E|\"A bullish signal occurs when MACD ... moves above its 9 day EMA or the centreline\" (12, 26, 9) (p.72)"], "No exit; never add one."),
        make("gfstsb_inside_day_example", "Inside day entry illustration", "Ch.8 Trade Entry (p.73)", "Long",
            Status::NotTestable("'just above the close' is not quantified and there is no profit exit"),
            &["E|\"enter a long order just above the close price of the inside day, with a stop-loss just below the open price of the inside day\" (p.73)"], ""),
        make("gfstsb_exit_menu", "Exit and filter toolbox (ch.9-10)", "Ch.9-10 Exits / Filters (p.75-96)", "Long/Short",
            Status::NotTestable("menu of generic exits and filters (end of day, ATR, Donchian, pivots, Fibonacci, trailing stops, ADX/DMI) with no complete system or numbers"),
            &["X|End of day, time of day, static stops, X-bar high/low, ATR, moving average, Donchian, pivot, Fibonacci, trailing stop, profit target, direction reversal (ch.9)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "gfstsb_ema_cross" | "gfstsb_daily_ema_example" => ("gfstsb_ma", "Ford moving-average crossover", ""),
        "gfstsb_channel" => ("gfstsb_channel", "Ford channel crossover", ""),
        "gfstsb_inside_bar" | "gfstsb_inside_bar_sma" | "gfstsb_inside_day_example" => ("gfstsb_inside", "Ford inside bar", ""),
        "gfstsb_perfect_order" => ("gfstsb_po", "Ford perfect order", ""),
        "gfstsb_macd_example" | "gfstsb_exit_menu" => ("gfstsb_misc", "Ford examples and toolbox", ""),
        _ => return None,
    })
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn nothing_is_live() {
        assert!(super::catalogue().iter().all(|d| d.status != crate::connors::Status::Live));
    }
}
