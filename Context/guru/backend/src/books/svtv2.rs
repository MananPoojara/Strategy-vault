//! Victor Sperandeo, "Trader Vic II: Principles of Professional Speculation" (Wiley, 1994).
//! Every trading rule in the book needs chart judgement or lacks an exit; nothing is coded.
//! The 200-day MA rule (p. 156) repeats svmws_200d_gordon and the 2B / 1-2-3 repeat svmws_2b / svmws_123 (noted, not re-listed
//! except where the wording adds a mechanical detail). Source: library/svtv2/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "svtv2";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    let g10 = "Ch. 10 The Technical Principles of Market Analysis Applied";
    vec![
        d("svtv2_2b_stop", "2B sell-stop entry", g10,
          NotTestable("which 'previous high' (short-term, intermediate, major swing) is a chart choice; only the protective buy stop is given, no profit exit"),
          &["E|\"In an uptrend, if prices penetrate the previous high, fail to carry through, and then immediately drop below the previous high, then the trend is apt to reverse\" (p. 147)",
            "E|\"the ideal way to capitalize on a 2B is to sell on a stop when prices break below the previous high\" (p. 147)",
            "X|\"After selling, you then put in a buy stop at the 2B high\" (p. 147)"],
          "Same rule as svmws_2b with a stop-order entry. Not coded."),
        d("svtv2_line_breakout", "Trading a market that draws a line", g10,
          NotTestable("the trading range ('line') and 'heavy volume' are judged on the chart; stop only, no profit exit"),
          &["E|\"Draw horizontal lines corresponding to the highest and lowest points of the trading range\" (p. 151)",
            "E|\"If the market breaks out above the top horizontal line on heavy volume, buy the market with a sell stop at the point of the breakout\" (p. 151)"],
          "Not coded."),
        d("svtv2_four_day", "Four-day rule", g10,
          NotTestable("needs an 'intermediate move' top/bottom chosen after the fact; the only exit given is a stop at the low ('should never trade below the low of 10/9/92 for many months', p. 160)"),
          &["E|\"When the market has a reversal, in the form of a 4-day up or down sequence from a high or a low, after an intermediate move has taken place, the odds of the trend having changed is very high\" (p. 160)"],
          "Table 10.1 (DJIA 1926-1985): 25% of intermediate highs/lows immediately followed by a 4-day sequence, 41% within 6 days, 75% within 24 days, 10% none (p. 160). A frequency study, not a trade result. Not coded."),
        d("svtv2_four_day_corollary", "Four-day corollary", g10,
          NotTestable("'after a long move of intermediate proportions' is judgement; stop only ('use the high of the 5-day sequence as your stop', p. 162), no exit"),
          &["E|\"After a long move of intermediate proportions, when you have a 4day (or longer) sequence in the direction of the trend, the first day in the opposite direction often signifies the top or bottom\" (p. 161)"],
          "Not coded."),
        d("svtv2_gap_rule", "Gap rule", "Ch. 11 The Principles of Technical Analysis Applied",
          NotTestable("needs the hand-drawn trend line; no entry order or exit"),
          &["E|\"when you have a gap above or below a trend line, it indicates an important change ... The gap must break the trend line\" (p. 168)"], "Not coded."),
        d("svtv2_3day_hl", "High-low 3-day rule", "Ch. 11 The Principles of Technical Analysis Applied",
          NotTestable("entry needs 'a reversal' 'after a large move (subjective)' and a trend-line break (p. 169); 'the high or low of the third day' is not identified"),
          &["E|\"When a reversal occurs, to the extent that it goes above or below a three day high or low, you go long or short\" (p. 168)",
            "X|\"You use the high or low of the third day as your stop, and if that stop is hit, you then reverse your position\" (p. 168)"],
          "Book: lumber Jan-Jun 1993 'would have made $15,000 per contract' using it as a system (p. 194); he does not use it alone. Not coded."),
        d("svtv2_opening_range", "S&P futures opening-range day trade", "Ch. 13 A Professional Method of Day Trading",
          NotTestable("opening-range length undefined; half the position exits 1 point above the open, the other half has no exit; intraday futures"),
          &["E|\"if the market trades above the opening range today, you should buy with a stop one tick below the high of the range\" (p. 230)",
            "X|\"a key point to take a partial profit ... is one point (one 'handle') above the opening price ... you sell one-half your position\" (p. 231)"],
          "Not coded."),
        d("svtv2_life_expectancy", "Market life-expectancy profiles", "Ch. 9 The Technical Basis of Risk-Reward Analysis",
          NotTestable("a statistical description of Dow trend durations used for odds; no entry or exit"),
          &["F|Duration and extent percentiles of Dow primary/intermediate swings (Ch. 9)"], "Not coded."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if id.starts_with("svtv2_") && catalogue().iter().any(|s| s.id == id) {
        return Some(("svtv2_rules", "Trader Vic II chart rules", "not testable"));
    }
    None
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
