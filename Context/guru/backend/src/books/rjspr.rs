//! Ryan Jones, *S&P Reloaded - An Intraday Trading System* (Nov 2005, 42 pp, text PDF). Source + audit: library/rjspr/.
//! An intraday S&P E-mini pattern system: nothing here can run on daily bars, so every entry is NeedsData or NotTestable.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rjspr";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("rjspr_reloaded", "S&P Reloaded (initial move, pause, false resumption, reload, 2-bar entry)", "Rules to Reloaded · Exit rules · 2-bar setup",
          NeedsData("intraday S&P 500 E-mini bars (the book's charts are intraday minute charts; the bar interval for the 14-bar %K is not stated) plus a defined session; no daily-bar equivalent"), &[
            "F|\"the market has to make a minimum 4.25 point move within a 45-minute time frame, from top to bottom\" (Rules to Reloaded #1, p.7)",
            "F|\"The rule for an initial retracement is defined by whether the 14-bar %K stochastic crosses the 50% level\" (#2, p.8)",
            "F|False resumption: 2-bar setup - after the minimum retracement, the first bar whose low is lower than the previous bar's; resumption when price breaks the high of the higher of the two bars (#3, p.8-9; 2-Bar Setup Rule p.13-14)",
            "F|\"price action then needs to move back in the direction of the retracement ... and either equal or exceed the highest high of the retracement if the initial move is down, or exceed the lowest low of the retracement if the initial move is higher\" (#4, p.9)",
            "E|\"we are placing a stop order to enter the market\" on the next 2-bar setup (#5, p.9-10)",
            "X|\"The initial protective stop is 1.25 points above the highest high of a subsequent retracement ... never is the protective stop more than 3.00 points away from the entry price\" (p.11) - with three alternative handlings A/B/C and no default",
            "X|\"Exit at profit target one. This is simply a 2.00 point profit target\"; \"if the market moves 1.50 points in our direction ... we will use a trailing stop of 2.00 points\"; optional 4.00-point target \"if the initial move is 6.00 points or more and the reloaded retracement is at or less than 1/2 of the initial move\" (p.11-12, 16)",
            "F|Negations: %K above 80% (below 20%) during the initial move restarts it; retracement over 2 ticks before the start of the initial move cancels (Miscellaneous, p.15-16)",
        ], "Even with intraday data three points stay open: the bar interval (charts suggest 1-minute), the A/B/C choice when the protective stop would exceed 3.00 points (p.11), and the 'congestion' judgement used to qualify an initial move (p.7, 15). \
The 4.25-point minimum is tied to 2005 volatility ('This same point requirement is not valid during periods where the volatility is predominantly higher or lower', p.15). No published trade results; only chart illustrations."),
        d("rjspr_aggressive", "Aggressive variant without the reload (%K extremes)", "Miscellaneous #5",
          NotTestable("optional and discretionary: 'More aggressive traders may consider not waiting for the market to reload ...' (p.16-17); intraday as well"), &[
            "E|\"if the initial retracement causes the %K to hit an extreme level (above 85% after an initial down move and below 15% after an initial up move). If the initial move is 6.00 points or more, 80% and 20% may be used\" (p.16-17)"], ""),
        d("rjspr_fixed_ratio", "Fixed Ratio money management plan", "S&P Reloaded and Money Management",
          NotTestable("position sizing for an account, not a trading signal"), &[
            "F|\"Starting Contracts: 3 ... (Contracts * Delta) + account balance = next increase\" (p.39-40)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "rjspr_reloaded" => Some(("rjspr_reloaded", "S&P Reloaded", "Book rules")),
        "rjspr_aggressive" => Some(("rjspr_reloaded", "S&P Reloaded", "Aggressive (no reload)")),
        "rjspr_fixed_ratio" => Some(("rjspr_reloaded", "S&P Reloaded", "Money management")),
        _ => None,
    }
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn nothing_is_live() { assert!(super::catalogue().iter().all(|d| d.status != crate::connors::Status::Live)); }
}
