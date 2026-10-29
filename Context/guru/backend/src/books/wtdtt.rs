//! Walter T. Downs, "Trading for Tigers" (book id `wtdtt`; a .doc of 157 page images, OCR in claude-continuation-20261020/
//! authors/downs; "img N" = page image N, printed page = N-1). Reading and independent check: authors/downs/admission/tig_ref.py.
//! Live: 14 monthly bar patterns (Ch.5), 8 weekly bar patterns (Ch.6), 9 market-on-close patterns on weekly or monthly bars
//! (Ch.7) and 6 MIST daily patterns (Ch.8, US only: the trailing stop starts after a $100 profit).
//! Held: patterns that need open interest (the futures VOI versions, Volume/OI spike MOC), and the code-appendix-only patterns
//! with no text (Fractal Swings, Shark Hybrid, Tiger Engulfing, the weekly composite bars, MOC Gap Reversals).
//! Where the code appendix contradicts the text, the text is the default and the appendix is the labelled option `code`.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const MONTHLY: &[&str] = &["m_thinking_gap", "m_gap_spike", "m_voi_gap", "m_tr_contraction", "m_equi_quads", "m_fractal", "m_cstar1", "m_cstar2",
    "m_exp_series", "m_exp_extremes", "m_fib", "m_triangulating", "m_two_bar_sym", "m_tigers_claw"];
const WEEKLY: &[&str] = &["w_gap_rev", "w_exp_rev", "w_manip_exp_rev", "w_manip_gap", "w_manip_expansion", "w_voi_gap", "w_exp_oc_gap", "w_ext_rev_nogap"];
const MOC: &[&str] = &["moc_ext_rev", "moc_engulf", "moc_ext_rev_exp", "moc_range_exp", "moc_contraction", "moc_inside", "moc_bunny", "moc_manip", "moc_manip_hl"];
const MIST: &[&str] = &["mist1", "mist2", "mist3", "mist4", "mist5", "mist6"];

/// Admitted ids (authors/downs/admission/tig_ref.py (header)).
pub const REVIEWED: &[&str] = &["wtdtt_m_thinking_gap", "wtdtt_m_gap_spike", "wtdtt_m_voi_gap", "wtdtt_m_tr_contraction", "wtdtt_m_equi_quads",
    "wtdtt_m_fractal", "wtdtt_m_cstar1", "wtdtt_m_cstar2", "wtdtt_m_exp_series", "wtdtt_m_exp_extremes", "wtdtt_m_fib", "wtdtt_m_triangulating",
    "wtdtt_m_two_bar_sym", "wtdtt_m_tigers_claw", "wtdtt_w_gap_rev", "wtdtt_w_exp_rev", "wtdtt_w_manip_exp_rev", "wtdtt_w_manip_gap",
    "wtdtt_w_manip_expansion", "wtdtt_w_voi_gap", "wtdtt_w_exp_oc_gap", "wtdtt_w_ext_rev_nogap", "wtdtt_moc_ext_rev", "wtdtt_moc_engulf",
    "wtdtt_moc_ext_rev_exp", "wtdtt_moc_range_exp", "wtdtt_moc_contraction", "wtdtt_moc_inside", "wtdtt_moc_bunny", "wtdtt_moc_manip",
    "wtdtt_moc_manip_hl", "wtdtt_mist1", "wtdtt_mist2", "wtdtt_mist3", "wtdtt_mist4", "wtdtt_mist5", "wtdtt_mist6"];
/// MIST rules: the trailing stop is armed by a dollar amount ($100), so they run on US instruments only.
pub const US_ONLY: &[&str] = &["wtdtt_mist1", "wtdtt_mist2", "wtdtt_mist3", "wtdtt_mist4", "wtdtt_mist5", "wtdtt_mist6"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_BAR: &str = "[Declared readings] Months are calendar months, weeks run Monday to Friday; a month or week is built from the daily bars (open of its first day, highest high, lowest low, close of its last day, summed volume). Month/week 0 is the last completed one (it is complete once the next bar starts a new period). 'Within the top (bottom) x%' = close at or above high - x% of the range (at or below low + x%); '+/-x% of the central point' = the open no further than x% of the range from the mid-point; inside and outside bars are strict; 'largest/smallest range of n' allows ties; a period with no range is skipped. The orders stay live for the whole next period; a stop fills at its level or at a worse opening price; if both stops are touched on the same day, the one nearer that day's open fills (tie: the buy) and the other is cancelled. One position at a time; the exit is the close of the last trading day of the entry period (the book: 'exited at the end of the month/week'). Book markets: 13 futures and 60 stocks to 1996/98; indices, ETFs and stocks, US and India, are a declared application.";
const NOTE_MOC: &str = "[Declared readings] Bars are calendar weeks (Monday-Friday, default) or calendar months (option tf = 1), built from the daily bars. On the last trading day of the week or month the position is taken at that day's close and closed at the next day's open (Ch.7 intro: 'enters a position on the last day of the month or week at the close of the session ... exited at the open of the next session'). 'Within the top (bottom) x%' = close at or above high - x% of the range (at or below low + x%); '+/-22% of the central point' = the open no further than 22% of the range from the mid-point; inside/outside strict; 'largest/smallest of n' allows ties; a bar with no range is skipped. The book tested these on futures only.";
const NOTE_MIST: &str = "[Declared readings] MIST uptrend = the most recent of the two events was 'high at or above the highs of the two previous days and a close above the previous close' (downtrend mirrored); the text says the high 'equals the highest high' of three bars, so ties count. The entry stop at the high (low) of day 0 is live on the next day only and fills at its level or a worse open; both sides on one day: the level nearer the open. Protective stop: the low (short: high) of the bar two bars before the entry day, live from the day after entry, fills at its level or a worse open. '$100' is read as 100 price units per share or index point (US instruments only; on most shares and US indices it is rarely reached, so trades usually run until the protective stop): once the best closing profit reaches it, the stop rises to entry + 62% of that profit ('a trailing stop equal to 38% of the total profit'); option amt changes the amount. Ticks ('one tick under') are ignored. No re-entry on an exit day. Book: futures, daily, 1989-1996.";

fn rule_of(n: &str) -> &'static [&'static str] {
    match n {
        "m_thinking_gap" => &["E|\"1. The open of month 0 must be greater than the high of month 1. 2. The range of month 0 must be less than the range of month 1. 3. When the first two conditions are met, stop orders are placed at the high and low of month 0.\" (img 006, p.5) Reading: the down-gap mirror ('gaps higher or lower') is included.", "X|\"If one of our stops is hit, the position is exited at the end of the month.\" (img 006)", "F|Option - code = 1: the book's code appendix (img 068): up-gap month with range <= previous range -> SELL stop at the high; down-gap -> BUY stop at the low (a fade)."],
        "m_gap_spike" => &["E|\"1. Month 0 must open higher than the high of month 1. 2. Month 0 must then close in the top 25% of the month 0 range. 3. ... a buy-stop order is placed at the high of month 0.\" (img 006; sell reversed)", "X|\"If our stop is hit, the position is exited at the end of the month.\""],
        "m_voi_gap" => &["E|\"The same strategy can be used in stock issues using volume only.\" \"1. The month 0 volume ... must be greater than the volume ... of month 1. 2. If next month opens higher than the high of month 0, we will buy at the market.\" (img 018; sell: a gap opening below the low)", "X|\"3. The position is exited at the end of the month.\""],
        "m_tr_contraction" => &["E|\"1. The true range of month 1 must be greater than the true range of month 2. 2. The true range of month 2 must be greater than the true range of month 3. 3. Month 0 must have a range that is inside the range of month 1. 4. ... a sell-stop order is placed at the low of month 0, and a buy-stop order is placed at the high of month 0.\" (img 020)", "X|\"5. If our stop is hit, then the position is exited at the end of the month.\"", "F|Option - code = 1: code appendix (img 068) places only the sell stop."],
        "m_equi_quads" => &["E|\"1. Month 0 must have a range that is inside the range of month 1. 2. The high of month 0 must be less than or equal to the month 1 high minus 1% of the month 1 range. 3. The Low of month 0 must be greater than or equal to the month 1 low plus 1% of the month 1 range. 4. ... a buy-stop ... at the high of month 0, and a sell-stop ... at the low of month 0.\" (img 022)", "X|\"5. If one of our stops is hit, the position is exited at the end of the month\"", "F|Option - code = 1: code appendix (img 069) places only the sell stop."],
        "m_fractal" => &["E|\"SELL SET-UP (BUY SET-UPS SHOULD BE TESTED SEPERATELY) 1. The range of month 0 must be inside the range of month 1. 2. The range of month 0 must be the smallest range of the last four months. 3. ... a sell-stop order is placed at the low of the set-up month.\" (img 024)", "X|\"4. If our stop is hit, the position is exited at the end of the month.\"", "F|Option - side: 0 = sell stop (book), 1 = buy stop at the high, 2 = both."],
        "m_cstar1" => &["E|\"subtracting 43% of the month's range from the high of the month and by adding 43% of the month's range to the low ... The close must have fallen within this 14% envelope\"; \"a buy-stop order is placed at the high of month 0 and a sell-stop order is placed at the low of month 0.\" (img 026)", "X|\"3. If one of our stops is hit, the position is exited at the end of the month.\""],
        "m_cstar2" => &["E|\"1. The range of month 0 must be the greatest range of the last three months. 2. The close of month 0 must fall within the 44% central envelope [high - 28% .. low + 28%]. 3. ... a buy-stop order is placed at the high of month 0 and a sell-stop order is placed at the low of month 0.\" (img 028)", "X|\"4. If one of our stops is hit, the position is exited at the end of the month.\"", "F|Option - code = 1: code appendix (img 070) places only the buy stop."],
        "m_exp_series" => &["E|\"1. Month 0 must be an outside bar. 2. The range of month 0 must have a range that is greater than the previous three trading periods. 3. ... a buy-stop order is placed at the high of month 0 and a sell-stop order is placed at the low of month 0.\" (img 030)", "X|\"4. If one of our stops is hit, the position is exited at the end of the month.\""],
        "m_exp_extremes" => &["E|\"1 Month 0 must be an outside bar. 2. The closing price of month 0 must be within the top 25% of the month 0 range. 3. ... a buy-stop order is placed at the high of month 0.\" (img 030; sell reversed)", "X|\"4. If our stop is hit, then the position is exited at the end of the month.\""],
        "m_fib" => &["E|\"Price contraction one equals the highest high of the last three trading periods minus the lowest close of the last three trading periods. Price contraction two equals the highest close of the last three periods minus the lowest low of the last three periods. The lesser of these two values is then multiplied by Fibonacci ratios and subtracted from the next open.\" \"a sell-stop order is placed at this level\" (img 032; buy reversed: added to the open)", "X|\"3. If our stop is hit, the position is exited at the end of the month.\"", "F|Reading: the text names no single ratio ('38%, 62% etc.'); the default 138% is the book's code appendix ('Abnormal Volatility Breakouts', img 069: 1.38). Option fib."],
        "m_triangulating" => &["E|\"1. The range of month 0 must be inside the range of month 3. 2. The high of month 1 must be less than or equal to the high of month 3 and the low of month 1 must be greater than or equal to the low of month 3. 3. ... stop orders are placed at the high and low of month 0.\" (img 032)", "X|\"4. If one of our stops is hit, the position is exited at the end of the month.\""],
        "m_two_bar_sym" => &["E|\"1. The range of month 0 must be inside the range of month 1. 2. The close of month 0 must fall within a 10% central envelope [high - 45% .. low + 45%, code img 070]. 3. ... stop orders are placed at the high and low of month 0.\" (img 034)", "X|\"4. If one of our stops is hit, the position is exited at the end of the month.\""],
        "m_tigers_claw" => &["E|\"1. The high of month 2 must be greater than the high of month 1. 2. The high of month 0 must be greater than the high of month 2. 3. The low of month 1 must be less than or equal to the low of the last five months. 4. ... a buy-stop order is placed at the high of month 0.\" (img 034; sell reversed)", "X|\"5. If our stop is hit, the position is exited at the end of the month.\""],
        "w_gap_rev" => &["E|\"1. The close of week 0 must be within the top 25% of the week 0 range and the open of week 0 must be within the bottom 25% ... 2. ... In the following week, if the market opens higher than the high of week 0 then a sell-stop is placed at the high of week 0.\" (img 074; buy reversed)", "X|\"3. If our stop is hit, the position is exited at the end of the week.\""],
        "w_exp_rev" => &["E|\"1. The range of week 0 must be the largest range of the last three trading periods. 2. The open of the next week must be higher than the high of week 0. 3. ... a sell-stop order is placed at the high of week 0.\" (img 076; buy reversed)", "X|\"4. If our stop is hit, the position is exited at the end of the week.\""],
        "w_manip_exp_rev" => &["E|\"1, The range of week 1 must be greater than the range of week 2, 2. The open of week 0 must be within +/- 28% of the central point of the week 0 range and the close of week 0 must be within the bottom 25% of the week 0 range. 3. ... a sell-stop order is placed at the low of week 0.\" (img 078; buy reversed)", "X|\"4. If our stop is hit, the position is exited at the end of the week.\"", "F|Option - code = 1: code appendix (img 100): the open between low + 25% and high - 25% of the range, the close in the top (bottom) 28%."],
        "w_manip_gap" => &["E|\"1 The open of week 0 must be within +/- 28% of the central point of the week 0 range and the close of week 0 must be within the top 25% ... 2. If the open of the next week is higher than the high of week 0 then we will go long the market\" (img 078; sell reversed)", "X|\"3. The position is exited at the end of the week.\"", "F|Option - code = 1: code appendix (img 100): the open between low + 28% and high - 28% of the range."],
        "w_manip_expansion" => &["E|\"1. The open of week 0 must be within +/- 28% of the central point ... and the close of week 0 must be within the top 25% ... 2. The range of week 0 must be greater than the range of week 1. 3. ... a buy order is placed at the high of week 0.\" (img 080; sell reversed)", "X|\"4. If our stop is hit, the position is exited at the end of the week.\"", "F|Option - code = 1: code appendix (img 100-101): the open between low + 28% and high - 28% of the range."],
        "w_voi_gap" => &["E|\"For stocks, volume alone can be used for this pattern.\" \"1 The open of week 0 must be greater than the high of week 1 and the volume ... of week 0 must be greater than the volume ... of week 1. 2. ... a buy-stop order is placed at the high of week 0.\" (img 080; sell reversed)", "X|\"3. If our stop is hit, the position is exited at the end of the week.\"", "F|Option - code = 1: code appendix (img 101): volume above the average of the three previous weeks."],
        "w_exp_oc_gap" => &["E|\"1. The open of week 1 must be within the top 25% of the week 1 range and the close of week 1 must be within the bottom 25% of the week 1 range, and the open of week 0 must be less than the low of week 1. 2. ... a sell-stop order is placed at the low of week 0.\" (img 082; buy reversed)", "X|\"3. If our stop is hit, then the position is exited at the end of the week.\"", "F|Option - code = 1: code appendix (img 102) tests the open and close of week 0 instead of week 1."],
        "w_ext_rev_nogap" => &["E|\"1 The close and open of week 0 must be in the bottom 25% of the week 0 range and the week 0 range must be greater than the range of week 1. 2. The open of the next week must be higher than the low of week 0. 3. ... a sell-stop order is placed at the low of week 0.\" (img 082; buy reversed)", "X|\"4. If our stop is hit, the position is exited at the end of the week.\""],
        "moc_ext_rev" => &["E|\"1. The open of bar 0 must be within the bottom 25% of the bar 0 range and the close of bar 0 must be within the bottom 25% of the bar 0 range. 2. ... the market is sold at the close of bar 0.\" (img 106; buy reversed)", "X|\"3. The position is exited at the open of the following bar.\""],
        "moc_engulf" => &["E|\"1 The high of bar 0 must be greater than the high of bar 1 and the low of bar 0 must be less than the low of bar 1. 2. The close of bar 0 must be in the top 10% of the bar 0 range. 3. ... the market is bought at the close of bar 0.\" (img 106; sell reversed)", "X|\"4, The position is exited at the open of the following bar.\""],
        "moc_ext_rev_exp" => &["E|\"1 The range of bar 0 must be the largest range of the last two trading periods. 2. The open of bar 0 must be within the bottom 25% ... and the close of bar 0 must be within the bottom 25% ... 3. ... the market is sold at the close of bar 0.\" (img 107; buy reversed)", "X|\"4. The position is exited at the open of the following bar.\""],
        "moc_range_exp" => &["E|\"1, The range of bar 0 must be the largest range of 'n' trading periods. 2. The close of bar 0 must be within the top 10% of the bar 0 range. 3. ... the market is bought at the close of bar 0.\" (img 108; sell reversed)", "X|\"4. The position is exited at the open of the following bar.\"", "F|Reading: the text leaves n open; default 2 (the code appendix uses 2, 3 and 5 per market, img 123-134). Option n."],
        "moc_contraction" => &["E|\"1 The range of bar 0 must be the smallest range of 'n' trading periods. 2. The close of bar 0 must be within the bottom 10% of the bar 0 range. 3. ... the market is sold at the close of bar 0.\" (img 108; buy reversed)", "X|\"4. The position is exited at the open of the following bar.\"", "F|Reading: n not fixed in the text; default 2 (code appendix: 2 or 4 per market). Option n."],
        "moc_inside" => &["E|\"When a month or week has a range inside the previous range, combined with a close in the top or bottom twenty percent of the monthly or weekly range ...\" \"1. The range of bar 0 must be inside the range of bar 1, and the close of bar [0] must be within the bottom 20% of the bar 0 range. 2. ... we will sell at the close of bar 0.\" (img 109; buy reversed; the printed 'bar 1' is read as bar 0 per the intro)", "X|\"3. The position is exited at the open of the following bar.\"", "F|Option - pct: the code appendix mostly uses 10% (img 122-135)."],
        "moc_bunny" => &["E|\"1 The close of bar 0 must be greater than the open of bar 0, and the close of bar 1 must be less than the open of bar 1. 2. The high of bar 0 must be less than the high of bar 1, and the close of bar 0 must be within the top 25% of the bar 1 range. 3. ... the market is bought at the close of bar 0.\" (img 109; sell reversed)", "X|\"4. The position is exited at the open of the following bar.\""],
        "moc_manip" => &["E|\"1. The open of bar 0 must be within +/- 22 % of the central point of the bar 0 range, and the close of bar 0 must be within the top 10% of the bar 0 range. 2. ... the market is bought at the close of bar 0.\" (img 110; sell reversed)", "X|\"3. The position is exited at the open of the following bar.\""],
        "moc_manip_hl" => &["E|\"The open of bar 0 must be within +/- 22 % of the central point of the bar 0 range, and the close of bar 0 must be within the bottom 10% ... The high of bar 0 must be the highest high of the last three trading periods. ... the market is sold at the close of bar 0.\" (img 110; buy reversed: close in the top 10%, lowest low of three)", "X|\"3. The position is exited at the open of the following bar.\""],
        "mist1" => &["E|\"1. A MIST uptrend must be in effect. (The most recent market occurrence is highest high of three bars and close greater than close of previous bar.) 2. The close of day 0 must be greater than or equal to the High of day 1, and the range of day 0 must be less than the range of day 1. 3. ... a buy-stop order is placed at the high of day 0.\" (img 140; sell reversed)", "X|\"4. If our stop is hit, a protective stop is placed under the low of two bars ago. After the position is up one hundred dollars or more, we will institute a trailing stop equal to 38% of the total profit.\""],
        "mist2" => &["E|\"1. A MIST uptrend must be in effect. 2. The open of bar 0 must be greater than the high of bar 1, and the close of bar 0 must be less than the high of bar 1. 3. ... a buy-stop order is placed at the high of day 0.\" (img 141; sell reversed)", "X|Protective stop under the low of two bars ago; 38% trailing stop after $100 (img 141)."],
        "mist3" => &["E|\"1. A MIST uptrend must be in effect. 2. the absolute value of the open of bar 0 minus the close of bar 0 must be less than the bar 0 range multiplied by 7%, and the Range of bar 0 must be smaller than the previous three ranges. 3. ... a buy-stop order is placed at the high of day 0.\" (img 142; sell reversed)", "X|Protective stop under the low of two bars ago; 38% trailing stop after $100 (img 142)."],
        "mist4" => &["E|\"1. Yesterday must have been a MIST downtrend day. 2. The low of bar 0 must be greater than the low made on the previous condition two MIST downtrend day, provided the previous occurrence happened in the last twenty trading days. 3. ... a buy-stop order is placed at the high of day 0.\" (img 143; sell reversed)", "X|Protective stop under the low of two bars ago; 38% trailing stop after $100 (img 143).", "F|Reading: 'the last twenty trading days' = the 20 days before day 0 (days -20 .. -2)."],
        "mist5" => &["E|\"1. The high of bar 0 must be greater than the high of bar 1, and the low of bar 0 must be less than the low of bar 1. 2. The close of bar 0 must be less than the close of bar 1, and the low of bar 0 must be less than the low of bar 2. 3. The low of bar 1 must be less than the lows of bar 2 and bar 3, and the close of bar 1 must be less than the close of bar 2. 4. ... a sell-stop order is placed at the low of day 0.\" (img 144; buy reversed)", "X|Protective stop above the high of two bars ago; 38% trailing stop after $100 (img 144)."],
        "mist6" => &["E|\"1. A MIST downtrend must be in effect. 2. The high of bar 0 must be less than or equal to the high of bar 1, and the low of bar 0 must be greater than or equal to the low of bar 1. 3. The high of bar 1 must be less than or equal to the high of bar 2, and the low of bar 1 must be greater than or equal to the low of bar 2. 4. ... a sell-stop order is placed one tick under the low of day 0.\" (img 145; buy set-ups when a MIST uptrend is in effect)", "X|Protective stop above the high of two bars ago; 38% trailing stop after $100 (img 145)."],
        _ => &[],
    }
}

fn title(n: &str) -> &'static str {
    match n {
        "m_thinking_gap" => "Tigers monthly: Thinking Gaps (gap month with a smaller range, stops at both ends)",
        "m_gap_spike" => "Tigers monthly: Gap and Spike Continuation",
        "m_voi_gap" => "Tigers monthly: Volume Gap (stock version of the VOI gap, buy the gap open)",
        "m_tr_contraction" => "Tigers monthly: True Range Contraction",
        "m_equi_quads" => "Tigers monthly: Equilateral Quads",
        "m_fractal" => "Tigers monthly: Fractal Range Contraction (sell stop)",
        "m_cstar1" => "Tigers monthly: Christian's Star #1 (close in the central 14%)",
        "m_cstar2" => "Tigers monthly: Christian's Star #2 (largest range of 3, close in the central 44%)",
        "m_exp_series" => "Tigers monthly: Triangular Expanding Series (outside month)",
        "m_exp_extremes" => "Tigers monthly: Expansion Extremes (outside month, extreme close)",
        "m_fib" => "Tigers monthly: Fibonacci Percentage Breakouts",
        "m_triangulating" => "Tigers monthly: Triangulating Contraction Series",
        "m_two_bar_sym" => "Tigers monthly: Two Bar Symmetrical Occurrences",
        "m_tigers_claw" => "Tigers monthly: Tiger's Claw",
        "w_gap_rev" => "Tigers weekly: Expanding Open/Close with Gap Reversal",
        "w_exp_rev" => "Tigers weekly: Expansion Reversals",
        "w_manip_exp_rev" => "Tigers weekly: Manipulation Bar with Expansion Reversal",
        "w_manip_gap" => "Tigers weekly: Manipulation Bar with Following Gap",
        "w_manip_expansion" => "Tigers weekly: Manipulation Bar with Range Expansion",
        "w_voi_gap" => "Tigers weekly: Volume Gap (stock version of the VOI gap)",
        "w_exp_oc_gap" => "Tigers weekly: Expanding Open/Close with Gap",
        "w_ext_rev_nogap" => "Tigers weekly: Extreme Reversal and Range Expansion without Gap",
        "moc_ext_rev" => "Tigers market-on-close: Extreme Reversals",
        "moc_engulf" => "Tigers market-on-close: Engulfing Bar with Extreme Close",
        "moc_ext_rev_exp" => "Tigers market-on-close: Extreme Reversal and Range Expansion",
        "moc_range_exp" => "Tigers market-on-close: Range Expansion with Extreme Close",
        "moc_contraction" => "Tigers market-on-close: Contraction Set with Extreme Close",
        "moc_inside" => "Tigers market-on-close: Inside Bar with Extreme Close",
        "moc_bunny" => "Tigers market-on-close: The Bunny Hop",
        "moc_manip" => "Tigers market-on-close: Manipulation Bars",
        "moc_manip_hl" => "Tigers market-on-close: Manipulation Bars with New Short-term High or Low",
        "mist1" => "Tigers MIST #1 (trend day closing at the prior high on a smaller range)",
        "mist2" => "Tigers MIST #2 (gap open above the prior high that closes back below it)",
        "mist3" => "Tigers MIST #3 (doji-like narrow range bar in a MIST trend)",
        "mist4" => "Tigers MIST #4 (higher low after a MIST downtrend day)",
        "mist5" => "Tigers MIST #5 (outside reversal bar after a lower low)",
        "mist6" => "Tigers MIST #6 (two inside bars in a MIST trend)",
        _ => "",
    }
}

pub fn catalogue() -> Vec<StrategyDef> {
    let all: Vec<(&'static str, &'static str, &'static str)> = MONTHLY.iter().map(|n| (*n, "Ch.5 Short Term Monthly Patterns (img 006-035)", NOTE_BAR))
        .chain(WEEKLY.iter().map(|n| (*n, "Ch.6 Short Term Weekly Patterns (img 074-082)", NOTE_BAR)))
        .chain(MOC.iter().map(|n| (*n, "Ch.7 Market on Close Patterns (img 105-110)", NOTE_MOC)))
        .chain(MIST.iter().map(|n| (*n, "Ch.8 MIST daily patterns (img 136-145)", NOTE_MIST))).collect();
    all.into_iter().map(|(n, group, notes)| {
        let id = REVIEWED.iter().copied().find(|r| &r[6..] == n).unwrap();
        StrategyDef { id, name: title(n), group, session: 0, dir: if n == "m_fractal" { "Short" } else { "Long/Short" }, status: Status::Live,
            needs_volume: n.ends_with("voi_gap"), needs_vix: false, needs_series: &[], book: "wtdtt", rules: rule_of(n), notes, claim: None }
    }).collect()
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().copied().find(|r| *r == id)?;
    Some((id, "Downs Trading for Tigers", title(&id[6..])))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if !reviewed(id) { return None; }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let n = &id[6..];
    let mut v = vec![];
    if ["m_thinking_gap", "m_tr_contraction", "m_equi_quads", "m_cstar2", "w_manip_exp_rev", "w_manip_gap", "w_manip_expansion", "w_voi_gap", "w_exp_oc_gap"].contains(&n) {
        v.push(p("code", "Use the book's code appendix version (0 = text, book default; 1 = appendix)", 0., 0., 1., 1.));
    }
    match n {
        "m_fractal" => v.push(p("side", "Side: 0 = sell stop (book), 1 = buy stop, 2 = both", 0., 0., 2., 1.)),
        "m_fib" => v.push(p("fib", "Fibonacci multiple of the smaller contraction (code appendix 1.38)", 1.38, 0.1, 5., 0.001)),
        "moc_range_exp" | "moc_contraction" => v.push(p("n", "Ranges compared, including bar 0 ('n'; default 2)", 2., 2., 20., 1.)),
        "moc_inside" => v.push(p("pct", "Extreme close zone % (book text 20; code appendix 10)", 20., 1., 50., 1.)),
        _ => {}
    }
    if n.starts_with("moc_") { v.push(p("tf", "Bar size: 0 = weekly, 1 = monthly", 0., 0., 1., 1.)); }
    if n.starts_with("mist") { v.push(p("amt", "Profit (price units) that starts the 38% trailing stop (book $100)", 100., 0., 100000., 0.01)); }
    Some(v)
}

struct Per { s: usize, e: usize, o: f64, h: f64, l: f64, c: f64, v: f64, r: f64 }

fn day_number(d: &str) -> i64 {
    // days since 1970-01-01 (civil-from-days inverse)
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

fn periods(x: &Ctx, monthly: bool) -> (Vec<Per>, Vec<usize>) {
    let key = |i: usize| { let d = &x.r[i].date; if monthly { d[0..4].parse::<i64>().unwrap_or(0) * 12 + d[5..7].parse::<i64>().unwrap_or(0) } else { (day_number(d) + 3).div_euclid(7) } };
    let (mut ps, mut pk): (Vec<Per>, Vec<usize>) = (vec![], vec![]);
    for i in 0..x.r.len() {
        let v = match x.r[i].volume { Some(v) if v > 0. => v, _ => f64::NAN };
        if i == 0 || key(i) != key(i - 1) {
            ps.push(Per { s: i, e: i, o: x.o[i], h: x.h[i], l: x.l[i], c: x.c[i], v, r: 0. });
        } else {
            let p = ps.last_mut().unwrap();
            p.e = i; p.h = p.h.max(x.h[i]); p.l = p.l.min(x.l[i]); p.c = x.c[i]; p.v += v;
        }
        pk.push(ps.len() - 1);
    }
    for p in ps.iter_mut() { p.r = p.h - p.l; }
    (ps, pk)
}

fn top(p: &Per, f: f64, v: f64) -> bool { v >= p.h - f * p.r }
fn bot(p: &Per, f: f64, v: f64) -> bool { v <= p.l + f * p.r }
fn inside(a: &Per, b: &Per) -> bool { a.h < b.h && a.l > b.l }
fn outside(a: &Per, b: &Per) -> bool { a.h > b.h && a.l < b.l }
fn centre(p: &Per, f: f64) -> bool { (p.o - (p.h + p.l) / 2.).abs() <= f * p.r }

#[derive(Clone, Copy)]
enum Cond { None, Above(f64), Below(f64) }
#[derive(Clone, Copy)]
struct Ord { d: f64, at_open: bool, lvl: f64, cond: Cond }

fn look(n: &str) -> usize {
    match n {
        "m_tr_contraction" | "m_tigers_claw" => 4,
        "m_fractal" | "m_exp_series" | "m_triangulating" | "w_voi_gap" => 3,
        "m_cstar2" | "m_fib" | "w_exp_rev" | "w_manip_exp_rev" => 2,
        "m_cstar1" | "w_gap_rev" | "w_manip_gap" => 0,
        _ => 1,
    }
}

fn orders(n: &str, ps: &[Per], k: usize, code: bool, side: usize, fib: f64, next_open: f64) -> Vec<Ord> {
    let g = |j: usize| &ps[k - j];
    let p0 = g(0);
    if p0.r <= 0. { return vec![]; }
    let stop = |d: f64, lvl: f64| Ord { d, at_open: false, lvl, cond: Cond::None };
    let (buy, sell) = (stop(1., p0.h), stop(-1., p0.l));
    match n {
        "m_thinking_gap" => {
            let (up, dn) = (p0.o > g(1).h, p0.o < g(1).l);
            if !(up || dn) { return vec![]; }
            if code { if p0.r <= g(1).r { return vec![if up { stop(-1., p0.h) } else { stop(1., p0.l) }]; } return vec![]; }
            if p0.r < g(1).r { vec![buy, sell] } else { vec![] }
        }
        "m_gap_spike" => {
            if p0.o > g(1).h && top(p0, 0.25, p0.c) { vec![buy] } else if p0.o < g(1).l && bot(p0, 0.25, p0.c) { vec![sell] } else { vec![] }
        }
        "m_voi_gap" => if p0.v > g(1).v { vec![Ord { d: 1., at_open: true, lvl: 0., cond: Cond::Above(p0.h) }, Ord { d: -1., at_open: true, lvl: 0., cond: Cond::Below(p0.l) }] } else { vec![] },
        "m_tr_contraction" => {
            let tr = |j: usize| g(j).h.max(g(j + 1).c) - g(j).l.min(g(j + 1).c);
            if tr(1) > tr(2) && tr(2) > tr(3) && inside(p0, g(1)) { if code { vec![sell] } else { vec![buy, sell] } } else { vec![] }
        }
        "m_equi_quads" => {
            let p1 = g(1);
            if inside(p0, p1) && p0.h <= p1.h - 0.01 * p1.r && p0.l >= p1.l + 0.01 * p1.r { if code { vec![sell] } else { vec![buy, sell] } } else { vec![] }
        }
        "m_fractal" => {
            if inside(p0, g(1)) && p0.r <= g(1).r.min(g(2).r).min(g(3).r) { match side { 0 => vec![sell], 1 => vec![buy], _ => vec![buy, sell] } } else { vec![] }
        }
        "m_cstar1" => if p0.c >= p0.l + 0.43 * p0.r && p0.c <= p0.h - 0.43 * p0.r { vec![buy, sell] } else { vec![] },
        "m_cstar2" => {
            if p0.r >= g(1).r.max(g(2).r) && p0.c >= p0.l + 0.28 * p0.r && p0.c <= p0.h - 0.28 * p0.r { if code { vec![buy] } else { vec![buy, sell] } } else { vec![] }
        }
        "m_exp_series" => if outside(p0, g(1)) && p0.r > g(1).r.max(g(2).r).max(g(3).r) { vec![buy, sell] } else { vec![] },
        "m_exp_extremes" => {
            if !outside(p0, g(1)) { vec![] } else if top(p0, 0.25, p0.c) { vec![buy] } else if bot(p0, 0.25, p0.c) { vec![sell] } else { vec![] }
        }
        "m_fib" => {
            let hh = p0.h.max(g(1).h).max(g(2).h); let ll = p0.l.min(g(1).l).min(g(2).l);
            let hc = p0.c.max(g(1).c).max(g(2).c); let lc = p0.c.min(g(1).c).min(g(2).c);
            let d = (hh - lc).min(hc - ll) * fib;
            if d > 0. { vec![stop(1., next_open + d), stop(-1., next_open - d)] } else { vec![] }
        }
        "m_triangulating" => if inside(p0, g(3)) && g(1).h <= g(3).h && g(1).l >= g(3).l { vec![buy, sell] } else { vec![] },
        "m_two_bar_sym" => if inside(p0, g(1)) && p0.c >= p0.l + 0.45 * p0.r && p0.c <= p0.h - 0.45 * p0.r { vec![buy, sell] } else { vec![] },
        "m_tigers_claw" => {
            if g(2).h > g(1).h && p0.h > g(2).h && (0..5).all(|j| g(1).l <= g(j).l) { vec![buy] }
            else if g(2).l < g(1).l && p0.l < g(2).l && (0..5).all(|j| g(1).h >= g(j).h) { vec![sell] } else { vec![] }
        }
        "w_gap_rev" => {
            if top(p0, 0.25, p0.c) && bot(p0, 0.25, p0.o) { vec![Ord { cond: Cond::Above(p0.h), ..stop(-1., p0.h) }] }
            else if bot(p0, 0.25, p0.c) && top(p0, 0.25, p0.o) { vec![Ord { cond: Cond::Below(p0.l), ..stop(1., p0.l) }] } else { vec![] }
        }
        "w_exp_rev" => if p0.r >= g(1).r.max(g(2).r) { vec![Ord { cond: Cond::Above(p0.h), ..stop(-1., p0.h) }, Ord { cond: Cond::Below(p0.l), ..stop(1., p0.l) }] } else { vec![] },
        "w_manip_exp_rev" | "w_manip_gap" | "w_manip_expansion" => {
            let (man, up, dn) = if n == "w_manip_exp_rev" && code {
                (p0.o > p0.l + 0.25 * p0.r && p0.o < p0.h - 0.25 * p0.r, top(p0, 0.28, p0.c), bot(p0, 0.28, p0.c))
            } else {
                (if code { p0.o > p0.l + 0.28 * p0.r && p0.o < p0.h - 0.28 * p0.r } else { centre(p0, 0.28) }, top(p0, 0.25, p0.c), bot(p0, 0.25, p0.c))
            };
            if !man { return vec![]; }
            let gate = match n { "w_manip_exp_rev" => g(1).r > g(2).r, "w_manip_expansion" => p0.r > g(1).r, _ => true };
            if !gate { return vec![]; }
            if n == "w_manip_gap" {
                if up { vec![Ord { d: 1., at_open: true, lvl: 0., cond: Cond::Above(p0.h) }] } else if dn { vec![Ord { d: -1., at_open: true, lvl: 0., cond: Cond::Below(p0.l) }] } else { vec![] }
            } else if up { vec![buy] } else if dn { vec![sell] } else { vec![] }
        }
        "w_voi_gap" => {
            let vref = if code { (g(1).v + g(2).v + g(3).v) / 3. } else { g(1).v };
            if !(p0.v > vref) { vec![] } else if p0.o > g(1).h { vec![buy] } else if p0.o < g(1).l { vec![sell] } else { vec![] }
        }
        "w_exp_oc_gap" => {
            let b = if code { p0 } else { g(1) };
            if b.r <= 0. { vec![] }
            else if top(b, 0.25, b.o) && bot(b, 0.25, b.c) && p0.o < g(1).l { vec![sell] }
            else if bot(b, 0.25, b.o) && top(b, 0.25, b.c) && p0.o > g(1).h { vec![buy] } else { vec![] }
        }
        "w_ext_rev_nogap" => {
            if !(p0.r > g(1).r) { vec![] }
            else if bot(p0, 0.25, p0.o) && bot(p0, 0.25, p0.c) { vec![Ord { cond: Cond::Above(p0.l), ..sell }] }
            else if top(p0, 0.25, p0.o) && top(p0, 0.25, p0.c) { vec![Ord { cond: Cond::Below(p0.h), ..buy }] } else { vec![] }
        }
        _ => vec![],
    }
}

/// Stop fill on bar i: the level, or a worse open.
fn stop_px(x: &Ctx, i: usize, d: f64, lvl: f64) -> Option<f64> {
    if d > 0. { if x.o[i] >= lvl { Some(x.o[i]) } else if x.h[i] >= lvl { Some(lvl) } else { None } }
    else if x.o[i] <= lvl { Some(x.o[i]) } else if x.l[i] <= lvl { Some(lvl) } else { None }
}

fn mk(d: f64, ei: usize, ep: f64, kind: &'static str, xi: usize, xp: f64, why: &str, open: bool) -> Trade {
    Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: xi, open, exits: vec![] }
}

fn run_breakout(n: &str, x: &Ctx, from: usize, to: usize, code: bool, side: usize, fib: f64) -> Vec<Trade> {
    let (ps, pk) = periods(x, n.starts_with("m_"));
    let why = if n.starts_with("m_") { "end of the month" } else { "end of the week" };
    let (mut out, mut cur, mut done) = (vec![], None::<(f64, usize, f64, &'static str, usize)>, usize::MAX);
    for i in from..=to {
        if let Some((d, ei, ep, kind, xe)) = cur {
            if i == xe { out.push(mk(d, ei, ep, kind, i, x.c[i], why, false)); cur = None; }
            continue;
        }
        if pk[i] == 0 || pk[i] - 1 < look(n) || pk[i] == done { continue; }
        let (k, t) = (pk[i] - 1, &ps[pk[i]]);
        let first_open = x.o[t.s];
        let mut fills: Vec<(f64, f64, f64, &'static str)> = vec![]; // (distance from the open, -dir, price, kind) ; dir via sign
        for od in orders(n, &ps, k, code, side, fib, first_open) {
            let ok = match od.cond { Cond::None => true, Cond::Above(v) => first_open > v, Cond::Below(v) => first_open < v };
            if !ok { continue; }
            if od.at_open {
                if i == t.s { fills.push((0., -od.d, x.o[i], "open")); }
            } else if let Some(px) = stop_px(x, i, od.d, od.lvl) {
                fills.push((if px == x.o[i] { -1. } else { (od.lvl - x.o[i]).abs() }, -od.d, px, "stop"));
            }
        }
        if fills.is_empty() { continue; }
        fills.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.partial_cmp(&b.1).unwrap()));
        let (_, nd, px, kind) = fills[0];
        done = pk[i];
        if i == t.e { out.push(mk(-nd, i, px, kind, i, x.c[i], why, false)); } else { cur = Some((-nd, i, px, kind, t.e)); }
    }
    if let Some((d, ei, ep, kind, _)) = cur { out.push(mk(d, ei, ep, kind, to, x.c[to], "open", true)); }
    out
}

fn moc_dir(n: &str, ps: &[Per], k: usize, nn: usize, pct: f64) -> f64 {
    let g = |j: usize| &ps[k - j];
    let p0 = g(0);
    if p0.r <= 0. { return 0.; }
    let (o, c) = (p0.o, p0.c);
    let both = |up: bool| if up { top(p0, 0.25, o) && top(p0, 0.25, c) } else { bot(p0, 0.25, o) && bot(p0, 0.25, c) };
    let ext = |f: f64, up_first: bool| -> f64 {
        if up_first { if top(p0, f, c) { 1. } else if bot(p0, f, c) { -1. } else { 0. } } else if bot(p0, f, c) { -1. } else if top(p0, f, c) { 1. } else { 0. }
    };
    match n {
        "moc_ext_rev" => if both(true) { 1. } else if both(false) { -1. } else { 0. },
        "moc_engulf" => if outside(p0, g(1)) { ext(0.1, true) } else { 0. },
        "moc_ext_rev_exp" => if p0.r < g(1).r { 0. } else if both(true) { 1. } else if both(false) { -1. } else { 0. },
        "moc_range_exp" => if (1..nn).any(|j| p0.r < g(j).r) { 0. } else { ext(0.1, true) },
        "moc_contraction" => if (1..nn).any(|j| p0.r > g(j).r) { 0. } else { ext(0.1, false) },
        "moc_inside" => if inside(p0, g(1)) { ext(pct / 100., false) } else { 0. },
        "moc_bunny" => {
            let p1 = g(1);
            if c > o && p1.c < p1.o && p0.h < p1.h && c >= p1.h - 0.25 * p1.r { 1. }
            else if c < o && p1.c > p1.o && p0.l > p1.l && c <= p1.l + 0.25 * p1.r { -1. } else { 0. }
        }
        "moc_manip" => if centre(p0, 0.22) { ext(0.1, true) } else { 0. },
        "moc_manip_hl" => {
            if !centre(p0, 0.22) { 0. }
            else if bot(p0, 0.1, c) && p0.h >= g(1).h.max(g(2).h) { -1. }
            else if top(p0, 0.1, c) && p0.l <= g(1).l.min(g(2).l) { 1. } else { 0. }
        }
        _ => 0.,
    }
}

fn run_moc(n: &str, x: &Ctx, from: usize, to: usize, monthly: bool, nn: usize, pct: f64) -> Vec<Trade> {
    let (ps, pk) = periods(x, monthly);
    let lk = match n { "moc_range_exp" | "moc_contraction" => nn - 1, "moc_ext_rev" | "moc_manip" => 0, "moc_manip_hl" => 2, _ => 1 };
    let mut out = vec![];
    for i in from..to {
        if pk[i + 1] == pk[i] || pk[i] < lk { continue; }
        let d = moc_dir(n, &ps, pk[i], nn, pct);
        if d != 0. {
            out.push(Trade { signal_i: i, ..mk(d, i, x.c[i], "close", i + 1, x.o[i + 1], "next open", false) });
        }
    }
    out
}

fn run_mist(n: &str, x: &Ctx, from: usize, to: usize, amt: f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let len = c.len();
    let r: Vec<f64> = (0..len).map(|i| h[i] - l[i]).collect();
    let c1: Vec<bool> = (0..len).map(|i| i >= 2 && h[i] >= h[i - 1] && h[i] >= h[i - 2] && c[i] > c[i - 1]).collect();
    let c2: Vec<bool> = (0..len).map(|i| i >= 2 && l[i] <= l[i - 1] && l[i] <= l[i - 2] && c[i] < c[i - 1]).collect();
    let mut tr = vec![0i8; len];
    for i in 0..len { tr[i] = if c1[i] { 1 } else if c2[i] { -1 } else if i > 0 { tr[i - 1] } else { 0 }; }
    let prev = |v: &Vec<bool>, i: usize| (i.saturating_sub(20)..=i - 2).rev().find(|&j| v[j]);
    let setup = |i: usize| -> Vec<(f64, f64)> {
        let mut s = vec![];
        if i < 4 { return s; }
        match n {
            "mist1" => {
                if tr[i] > 0 && c[i] >= h[i - 1] && r[i] < r[i - 1] { s.push((1., h[i])); }
                if tr[i] < 0 && c[i] <= l[i - 1] && r[i] < r[i - 1] { s.push((-1., l[i])); }
            }
            "mist2" => {
                if tr[i] > 0 && o[i] > h[i - 1] && c[i] < h[i - 1] { s.push((1., h[i])); }
                if tr[i] < 0 && o[i] < l[i - 1] && c[i] > l[i - 1] { s.push((-1., l[i])); }
            }
            "mist3" => {
                if (o[i] - c[i]).abs() < 0.07 * r[i] && r[i] < r[i - 1].min(r[i - 2]).min(r[i - 3]) {
                    if tr[i] > 0 { s.push((1., h[i])); }
                    if tr[i] < 0 { s.push((-1., l[i])); }
                }
            }
            "mist4" => {
                if c2[i - 1] { if let Some(j) = prev(&c2, i) { if l[i] > l[j] { s.push((1., h[i])); } } }
                if c1[i - 1] { if let Some(j) = prev(&c1, i) { if h[i] < h[j] { s.push((-1., l[i])); } } }
            }
            "mist5" => {
                if h[i] > h[i - 1] && l[i] < l[i - 1] {
                    if c[i] < c[i - 1] && l[i] < l[i - 2] && l[i - 1] < l[i - 2] && l[i - 1] < l[i - 3] && c[i - 1] < c[i - 2] { s.push((-1., l[i])); }
                    if c[i] > c[i - 1] && h[i] > h[i - 2] && h[i - 1] > h[i - 2] && h[i - 1] > h[i - 3] && c[i - 1] > c[i - 2] { s.push((1., h[i])); }
                }
            }
            "mist6" => {
                if h[i] <= h[i - 1] && l[i] >= l[i - 1] && h[i - 1] <= h[i - 2] && l[i - 1] >= l[i - 2] {
                    if tr[i] > 0 { s.push((1., h[i])); }
                    if tr[i] < 0 { s.push((-1., l[i])); }
                }
            }
            _ => {}
        }
        s
    };
    // position: dir, entry bar, entry price, stop, best closing profit
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64, f64, f64)>);
    let trail = |d: f64, ep: f64, s: f64, best: f64| if best >= amt { let t = ep + d * 0.62 * best; if d > 0. { s.max(t) } else { s.min(t) } } else { s };
    for i in from..=to {
        if let Some((d, ei, ep, s, best)) = cur {
            let hit = if d > 0. { if o[i] <= s { Some(o[i]) } else if l[i] <= s { Some(s) } else { None } }
                else if o[i] >= s { Some(o[i]) } else if h[i] >= s { Some(s) } else { None };
            if let Some(px) = hit { out.push(mk(d, ei, ep, "stop", i, px, "stop", false)); cur = None; continue; }
            let best = best.max(d * (c[i] - ep));
            cur = Some((d, ei, ep, trail(d, ep, s, best), best));
            continue;
        }
        if i == from { continue; }
        let mut fills: Vec<(f64, f64, f64)> = vec![];
        for (d, lvl) in setup(i - 1) {
            if let Some(px) = stop_px(x, i, d, lvl) { fills.push((if px == o[i] { -1. } else { (lvl - o[i]).abs() }, -d, px)); }
        }
        if fills.is_empty() { continue; }
        fills.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.partial_cmp(&b.1).unwrap()));
        let (_, nd, px) = fills[0];
        let d = -nd;
        let s0 = if d > 0. { l[i - 2] } else { h[i - 2] };
        let best = d * (c[i] - px);
        cur = Some((d, i, px, trail(d, px, s0, best), best));
    }
    if let Some((d, ei, ep, _, _)) = cur { out.push(mk(d, ei, ep, "stop", to, c[to], "open", true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let n = &id[6..];
    Some(if n.starts_with("mist") { run_mist(n, x, from, to, g("amt")) }
        else if n.starts_with("moc_") { run_moc(n, x, from, to, g("tf") > 0.5, (g("n").round() as usize).max(2), g("pct")) }
        else { run_breakout(n, x, from, to, g("code") > 0.5, g("side").round().clamp(0., 2.) as usize, if g("fib") > 0. { g("fib") } else { 1.38 }) })
}

#[cfg(test)]
mod tests {
    #[test]
    fn day_numbers_and_week_keys() {
        assert_eq!(super::day_number("1970-01-01"), 0);
        assert_eq!(super::day_number("2026-10-29"), 20755);
        // Monday 2026-10-26 starts the week of Thursday 2026-10-29; Sunday 2026-10-25 belongs to the week before
        let w = |d: &str| (super::day_number(d) + 3).div_euclid(7);
        assert_eq!(w("2026-10-26"), w("2026-10-30")); assert_ne!(w("2026-10-25"), w("2026-10-26"));
    }
    #[test]
    fn every_id_has_rules_title_and_params() {
        for d in super::catalogue() { assert!(!d.rules.is_empty() && !d.name.is_empty(), "{}", d.id); assert!(super::params(d.id).is_some()); }
        assert_eq!(super::catalogue().len(), super::REVIEWED.len());
    }
}
