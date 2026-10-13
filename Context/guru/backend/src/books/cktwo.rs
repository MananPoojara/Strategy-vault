//! Cynthia A. Kase, "Trading With the Odds: Using the Power of Probability to Profit in the Futures Market" (Irwin, 1996,
//! 163 pp). Book id `cktwo`. Source copy: Books/Cynthia Kase/Cynthia Kase - Trading With The Odds.pdf (printed pages below).
//! Companion sources folded in (cited per rule): Kase StatWare Manual for TradeStation v8 (2004); articles "The Kase
//! Dev-Stop" (MTA, pp.43-48), "Simplified momentum filters improve trading" (Futures, Dec 1993, pp.40-42), "New
//! High-Probability Indicators" (Energy in the News, Spring 1996, pp.26-31), "The Best Momentum Indicators" (Bridge
//! Trader 1997), "The Two Faces of Momentum" (SFO, Oct 2003), "Statistics in action", "Putting the odds on your side",
//! "Using Stochastics to Forecast Market Moves" (NYMEX 1991), "Managing Trade Risk", "Day-traders' Doom".
//! Verdict: Kase sells indicators, not systems ("the Kase indicators are tools that support a methodology and not a 'black
//! box' system", StatWare manual p.i). The PeakOscillator, KaseCD and KEES signals use proprietary cycle-length loops
//! ("shortCycle and longCycle control the range of cycle lengths evaluated in a series of proprietary calculations") and a
//! PeakOut level calibrated on "80 years of commodity history" that is never printed. The Dev-Stop formula is printed
//! but it is an exit with no entry ("It has not been my purpose to design a trading system"), and its look-back is never
//! given. The permission screens are partly judgement ("near the top of the chart and close in value"). So nothing is
//! coded; every item is NotTestable with the reason. No exit or entry of our own is added.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "cktwo";
const PROP: &str = "Proprietary indicator: the cycle-length range (n1..n2 / shortCycle..longCycle), the smoothing and the historical 90th-percentile PeakOut level are never published (book pp.77-90; StatWare manual pp.14-18; 'The Two Faces of Momentum')";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, why, rules, notes| StrategyDef { id, name, group, session: 0, dir,
        status: Status::NotTestable(why), needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("cktwo_dev_stop", "Kase Dev-Stop (three-level adaptive stop)", "Ch 6 Optimal Stop: Kase's Adaptive Dev-Stop", "Long/Short",
          "An exit tool with no entry: 'It has not been my purpose to design a trading system in this context' (Dev-Stop article p.47); the 10/21 SMA switch only positions the plot - 'I do not recommend using simple moving averages as critical information on which to base trades!' (book p.97). The look-back of the true-range average/std-dev (NumBars) is never printed.",
          &["X|'Calculate True Range of Double Bar (use highest of both high's, lowest of both lows)' (article p.45)",
            "X|'Calculate Moving Average of True Ranges, ATR'; 'Calculate Standard Deviation of True Ranges, SDEV' (p.45)",
            "X|'Dev Stop Reversal Value = ATR + (f * (SDEV)), DDEV where f = 1, 2, and 3' (p.45); skew: 'the second standard deviation value is corrected by a factor of 1.1 and the 3rd by 1.2' (p.47); StatWare defaults Dev1/Dev2/Dev3 = 1, 2.2, 3.6 (manual p.24)",
            "X|'Dev Stop Long = Trade High - DDEV; Dev Stop Short = Trade Low + DDEV' (p.45); warning line = mean two-bar reversal (book p.97)",
            "X|'I also assume that two closes beyond the warning line is the same as breaking stop one.' (book p.97)",
            "F|display switch: 'If the 10-period moving average is above the 21, I display the Dev-Stop below the bars, trailing a long position' (book p.97)"],
          "The book says a minimum of 30 periods for ranges (p.90) but that is about the RWI, not a stated Dev-Stop input."),
        d("cktwo_weekly_ma_screen", "Weekly moving-average permission screen (10/21 SMA)", "Ch 4 Screening Using Trending Filters", "Long/Short",
          "No exit: the rules only say which daily crossovers are 'permissioned' (pp.50-52); the daily trades are shown on charts only, with no exit rule.",
          &["F|'If, in the longer time-frame, the fast (shorter) moving average is above the slow moving average, only long trades are permissioned in the shorter time-frame.' (p.50)",
            "F|'If, in the longer time-frame, the fast (shorter) moving average is below the slow moving average, only short trades are permissioned' (p.50)",
            "E|daily 10- and 21-period simple moving average crossovers (Figure 4-1, p.50)",
            "X|(none stated)"],
          "Kase herself lists six missed trades from the lag (p.53)."),
        d("cktwo_permission_stochastic", "Kase Permission Stochastic / Permission Screen", "Ch 4 The Kase Permission Stochastic", "Long/Short",
          "A filter, not a system, and two of its three permission rules are judgement ('near the top of the chart and close in value', 'well above the Permission D'); the synthetic stochastic is 'smoothed as necessary' with no stated smoothing (book pp.55-61; Futures Dec 1993 pp.40-42; StatWare manual pp.21-22).",
          &["F|Synthetic bar (n = 5): high = max(H[0..n-1]), low = min(L[0..n-1]), open = O[n-1], close = C[0], previous close = C[n]; length x n bars (book p.57)",
            "F|'Traders can go long when: The Permission Stochastic values are near the top of the chart and close in value' (p.60)",
            "F|'The Stochastic values are both below 85 percent and above 15 percent and Permission K is above Permission D.' (manual p.21; Futures 1993: 'below 80% and above 15%')",
            "X|(none - screen only)"],
          "Defaults multiplier 5, length 9 (manual p.21)."),
        d("cktwo_timing_trade_plan", "Trade plan: MA-crossover timing with Permission Screen (Walking Through Trades)", "Ch 7 Walking Through Trades", "Long/Short",
          "Intraday monitor/timing charts, proprietary PeakOut/KaseCD warnings, a judgement-based permission screen and discretionary roll-ups ('make sure that a pullback is not imminent'); exits depend on the proprietary indicators (pp.111-113).",
          &["E|'Buy when the fast moving average crosses from below to above the slow moving average AND the Kase Permission Screen confirms the signal. (A 5-period simple fast moving average and a 13-period simple slow moving average are used in this example. In reality, any combination of moving averages or other simple timing method can be used.)' (p.112)",
            "X|'Maintain stops at level 3 if no danger is present'; 'When warning signs appear ... take half of the profit and pull the stop in to Dev-Stop one' (p.113)",
            "X|'If stops are hit before new signals are generated in the opposite direction, exit the trade.' (p.113)"],
          "Worked trades: August 1995 natural gas (pp.114-126), July 1995 (pp.126-138); 50% entries on timing chart, 50% on monitor chart."),
        d("cktwo_scaling_in_time", "Scaling up in time (timing / monitor / daily)", "Ch 4 Scaling In Trades", "Long/Short",
          "Intraday (80- and 20-minute D-mark charts) plus the judgement-based permission screen (pp.63-69).",
          &["A|'trades are entered with 50 percent of the position on a combination of a permission screen and a crossover on the timing chart. The second 50 percent ... on the monitor chart' (p.65)",
            "X|'exited on one close against the trade below or above the slow moving average and two closes above or below the fast moving average in the longest time being traded' (p.65)"],
          "Book trade examples: short loss 13 points vs 82 daily; long +367 vs +175 points (pp.67-69)."),
        d("cktwo_peakoscillator", "Kase PeakOscillator PeakOut / divergence", "Ch 5 Increasing the Probability of Catching Market Turns", "Long/Short", PROP,
          &["F|KSDI_up = ln(high0/low_n)/(v*sqrt(n)), KSDI_dn = ln(high_n/low0)/(v*sqrt(n)) over 'a range of cycle lengths' ('The Best Momentum Indicators')",
            "F|'Kase PeakOscillator = maxKSDI_UP - maxKSDI_DN ... with some smoothing added' (same)",
            "X|'As a general rule, fifty percent of a position is exited on either a peak local or peak global signal' (manual p.14)"],
          "Book: PeakOut line = higher of the historical 90th percentile or 2 std dev above the local mean (p.78)."),
        d("cktwo_kasecd", "KaseCD (KCD) divergence / PeakOut", "Ch 5 Improving Divergence Signals with the KaseCD", "Long/Short", PROP,
          &["F|'KCD histogram = PeakOscillator - average(PeakOscillator, n)' (Energy in the News 1996 p.30)"], ""),
        d("cktwo_exit_guidelines", "StatWare exit guidelines (divergence / PeakOut / inactivity)", "StatWare manual ch.5", "Long/Short", PROP,
          &["X|'Divergence on PeakOscillator AND KaseCD: 100%'; 'OR': '80% + Dev 1'; PeakOut late in trend '80% + Dev 1' (manual p.31)",
            "X|'When in no danger, default to DevStop 3'; 'If there is no profit in the trade after 5 to 8 bars, exit on inactivity.' (manual p.34); daily chart: '3 to 5 bars' (p.35)"],
          "Manual exit study: DevStop 1 hit 95% (20/21) after signal #1, 83% (52/63) after #2, 79% (26/33) after #3 (pp.32-33)."),
        d("cktwo_kees", "Kase Easy Entry System (KEES) first/second/third-class signals", "StatWare manual ch.4-5", "Long/Short",
          "Each signal class requires 'a proprietary first class signal' (manual p.19).",
          &["F|'higher high and higher low for long' plus a proprietary signal; close in the upper third, or close > open and above the lower third (manual pp.19-20)",
            "E|'Buy Entry: A second (or more) uninterrupted long signal.' (manual p.30)"], ""),
        d("cktwo_rwi_crossover", "Kase Random Walk Index crossover", "Energy in the News 1996; book ch.5 pp.88-90", "Long/Short",
          "'The RWI (modified somewhat from the Saitta and Poulous version to improve the math)' is 'evaluated over a number of different look-back lengths, and the largest value chosen' - the modification and the look-back range are not given (1996 article p.28).",
          &["F|'RW_high = (high0 - low_n)/(ATR x sqrt(n)); RW_low = (high_n - low0)/(ATR x sqrt(n))' (p.28)",
            "E|'A crossover of the RWI_plus ... over the RWI_minus ... is a buy signal and vice-versa.' (p.28)"],
          "Book p.90 argues for a 30-period ATR in the RWI denominator."),
        d("cktwo_candle_exits", "Candlestick accelerated exits", "Ch 6 Using Candlestick Patterns to Accelerate Exits", "Long/Short",
          "Patterns are used 'only paying attention to those signals that are confirmed by other indicators' (PeakOut / KaseCD, proprietary) and 'a star is a candlestick in which the open and close are near in price' (undefined) (pp.97-102).",
          &["X|evening star: 'once it appears that we will close below the mid-point of the blank Harami line, we will exit the portion of the trade that we normally would exit on Dev-Stop one' (p.101)"], ""),
        d("cktwo_stochastic_forecast", "Stochastic forecasting: double bottom retracement / double money breakthrough", "NYMEX Energy in the News, Spring 1991 pp.22-23", "Long/Short",
          "Price-target forecasts, no entry or exit; the stochastic length n is 'roughly equivalent to one half the predominant cycle length' found by eye or FFT.",
          &["F|'double money breakthrough occurs when both stochastic lines move from under 25 percent and break through to over 75 percent' -> target = 75% close + (75% close - 25% close) (p.22)"], ""),
        d("cktwo_wilder_bookstaber", "Wilder/Bookstaber volatility system (quoted)", "Dev-Stop article p.43; book p.94", "Long/Short",
          "Not Kase's rule: quoted from Wilder and Bookstaber with 'a factor of about 3' and no ATR length; already in the vault as kftsm_vol_breakout (Kaufman) - duplicate, skipped.",
          &["E|'Buy if the close rises by more than k*TRaverage from the previous close; Sell if the close falls by more than k*TRaverage from the previous close' (article p.43)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("cktwo_").then_some(("cktwo", "Trading With the Odds (Kase)", "Not testable"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn catalogue_ids_unique_and_prefixed() {
        let c = super::catalogue();
        let mut ids: Vec<_> = c.iter().map(|d| d.id).collect();
        ids.sort(); ids.dedup();
        assert_eq!(ids.len(), c.len());
        assert!(c.iter().all(|d| d.id.starts_with("cktwo_") && d.book == super::BOOK));
    }
}
