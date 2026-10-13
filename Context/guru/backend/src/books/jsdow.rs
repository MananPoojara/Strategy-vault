//! Jack Schannep, *Dow Theory for the 21st Century* (Wiley 2008). Source + audit: library/jsdow/.
//! Every signal rule needs the Dow Jones Transportation Average, the NYSE Composite, Fed data or an undisclosed
//! formula; the book has no DJIA-only trading rule. Nothing is Live.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jsdow";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long", status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("jsdow_dow_classic", "Traditional Dow Theory buy/sell signals (B-1..B-4, S-1..)", "Part I · Ch 2 Signals described",
          NeedsData("Dow Jones Transportation Average daily closes (confirmation by Industrials AND Transports); the 'market lows'/'tops' that start each pattern are also judged, not defined"), &[
            "F|\"After a pullback, there is a bounce that must exceed 3 percent on either one of the averages to be meaningful\"; \"That is really the only hard-and-fast number in the Dow Theory\" (p.17)",
            "E|Classic buy (B-1): \"1. Market lows. 2. Bounce. 3. Pullback (hold above the lows). 4. Break up (above the bounce high)\" by both Industrials and Transports (p.18-19); variations B-2..B-4 (p.19-21); sells are the inverse (p.21-)",
        ], "Appendix A lists the signals since 1897. Patterns need a second average; with the DJIA alone the rule is a different rule."),
        d("jsdow_dt21", "Dow Theory for the 21st Century (two of DJIA, Transports, S&P 500)", "Part III · Ch 9 The heart of the theory",
          NeedsData("Dow Jones Transportation Average (one of the three confirming averages); a DJIA+S&P-only version would drop signals the book takes"), &[
            "E|\"Using the Dow Jones Industrials, Transports, and the S&P 500 Index with any two of the three indices confirming one another\"; \"Whenever either the Industrials or Transports confirm the S&P 500, then a signal is given ... the first to give a signal is the winner!\" (p.99)",
            "A|Scale-in: 25% at capitulation, 25% at a second capitulation, 25% on the 3% pullback, 25% at the bull-market definition, complete 100% on the buy signal (p.100-101)",
        ], "44 signals since 1957, 'average improvement 2.2 percent' per signal (p.101)."),
        d("jsdow_capitulation", "Capitulation Indicator", "Part III · Ch 8 Capitulation",
          NotTestable("no exit of its own (exits are Dow Theory / Schannep Timing Indicator sells); also needs the NYSE Composite and the moving-average type is given two ways ('ten-week, time-weighted' vs 'exponentially smoothed 50-day')"), &[
            "E|\"Market bottoms are identified when the divergence between the three major stock market indices is 10% below their respective ten-week, exponentially time-weighted moving averages ... The date used is the first day the level was attained\" (p.90)",
        ], "Table 8.3: 11 capitulation buys 1962-2002, S&P 500 +11.0%/+18.5%/+24.7%/+25.4% after 3/6/9/12 months (p.93)."),
        d("jsdow_sti", "Schannep Timing Indicator", "Part IV · Ch 10",
          NotTestable("construction not disclosed ('My CPA is privy to my indicator's very specific construction', p.125); uses Fed Funds and free-reserve data"), &[
            "F|\"When the positive or negative momentum trend reaches a 1 percent threshold of change\" plus a favorable/neutral/unfavorable monetary atmosphere (p.124)"], "Signal dates in tables 10.1-10.2 and Appendix D."),
        d("jsdow_composite", "All-Inclusive Composite Indicator", "Part V · Ch 13",
          NotTestable("combines the undisclosed Schannep Timing Indicator with Dow Theory (Transports) and capitulation (NYSE Composite)"), &[
            "E|\"When any one of the four possible buy or one of the three possible sell indicators occurs, you should follow it by committing one-half of your intended investment position\" (p.171)"], ""),
        d("jsdow_bull_bear_defs", "Bull (+19%) / bear (-16%) market definitions", "Ch 1, 5-8",
          NotTestable("definitions used inside the Composite, not a stand-alone trading rule (no instruction to trade on them alone)"), &[
            "F|\"My definition of a bear market is a drop of 16% on both the Dow Jones Industrial Average and the Standard & Poor's 500 Index ... a 19% rise in both ... my definition of a bull market\" (p.86-87)"], ""),
        d("jsdow_three_tops", "Three Tops and a Tumble (volume peak, inverted yield curve, consumer confidence)", "Part IV · Ch 11",
          NotTestable("warning conditions with no entry/exit rule; macro data (NYSE monthly volume, 3-month/10-year yields, Conference Board index)"), &[
            "F|\"as the ratio ... reaches or exceeds 0.95 percent, the economy drops into an official recession\" (p.133); peaks separated by at least 32.5 points of consumer confidence (p.136)"], ""),
        d("jsdow_bonus", "Bonus indicators (first trading day, Jan/Feb barometer, typical year, presidential cycle)", "Part IV · Ch 12",
          NotTestable("statistics, no exit: e.g. 'make your annual investment contribution on the first trading day of January' (p.148)"), &[
            "F|\"when both [January and February] close up in the same year ... the S&P 500 has closed the year higher than February month-end 22 of the 23 times\" (p.153)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "jsdow_dow_classic" | "jsdow_dt21" => Some(("jsdow_dow", "Dow Theory", if id == "jsdow_dt21" { "21st century" } else { "Traditional" })),
        "jsdow_capitulation" | "jsdow_sti" | "jsdow_composite" | "jsdow_bull_bear_defs" => Some(("jsdow_composite", "Schannep composite components", "")),
        "jsdow_three_tops" | "jsdow_bonus" => Some(("jsdow_other", "Schannep other indicators", "")),
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
