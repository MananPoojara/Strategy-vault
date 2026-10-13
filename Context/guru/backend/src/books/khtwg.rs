//! Kevin Haggerty, "Trading With The Generals III" live web seminar workbook (Raven Capital Management, 2003, 558 pp).
//! Book id `khtwg`. Primary source: Books/Kevin Haggerty/Trading With The Generals III.pdf (printed page numbers below).
//! Companion sources folded in (same author, same set-ups, cited per rule):
//!   TWTG workbook 2004 "TWTGenerals.pdf" (chapter-page refs like "14-1"); TWTG 2002 workbook docs 1-4; the Five-Part
//!   Daytrading Course (TradingMarkets 2000); Inner Circle Training Program notes Oct 2004 - Mar 2005 (22 PDFs, dated);
//!   TradingMarkets articles "Entry Patterns At Alert Zones", "A Profitable Trade Using Multiple Patterns",
//!   "Nine Ways To Improve Your Trading Results".
//! Verdict: Haggerty states it himself - "'Sequence' is an approach to trading the markets, not a mechanical system. It
//! doesn't stand alone. Judgment and discipline are necessary." (TWTG 2002 workbook 3) and "it is art and judgment that
//! determines the success of your trade. Dynamics make the trade, not the pattern." No set-up in any source has a stated
//! profit exit (exits are "take some profits", "after the initial contra move", "a trailing stop that gives it some room"),
//! and most entries require "market dynamics" (NYSE TICK, futures premium, bid/ask) read by eye. So nothing is coded:
//! every set-up is NotTestable, with the exact reason. A rule with no exit is never completed with an exit of our own.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "khtwg";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, why, rules, notes| StrategyDef { id, name, group, session: 0, dir,
        status: Status::NotTestable(why), needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("khtwg_123_closes", "1,2,3 Closes", "IX. 1,2,3 Patterns (daily or any time frame)", "Long/Short",
          "No exit anywhere in the sources (TWTG III pp.273-286, TWTG 2004 ch.14, Inner Circle 1/24/2005): only an entry pattern. Entry is also conditional on 'if you feel the dynamics are compelling at the time of entry' (p.273).",
          &["F|'Must be at least a seven-bar low.' (p.274; sells reversed)",
            "F|'It must have an up close. (This up close can also be on the same bar as the significant lows.)' (p.274)",
            "F|'Next, it must have a down close.' (p.274)",
            "E|'You enter long above the high of the down close bar provided it has formed a swing point low when it changes direction.' (p.274)",
            "E|'If you get a second down close and its high is lower than the high of the first down close bar, you just lower your entry to one tick above the high of the second down close bar.' (p.275)",
            "F|'If there is a new lower low before you get entry, then the count starts over again.' (p.276)",
            "F|'Ninety-five percent of the time you get entry no later than the eighth bar ... Anything past that and it becomes an invalid trade.' (p.276); TWTG 2004 p.14-1: 'must be complete within 10 bars'; Inner Circle 1/24/05: 'no more than 10 bars'",
            "X|(none stated)"],
          "Entry rules are nearly mechanical (exception 5a: up-close bar closing above its midpoint + inside down-close bar). The three sources disagree on the bar limit (8 vs 10). Not in the vault yet (no duplicate)."),
        d("khtwg_rst", "RST (Reverse Symmetrical Triangle)", "XIII. RSTs (position and intraday)", "Long/Short",
          "Pattern validity is judgement ('if the RST pattern is symmetrical and one or more of the \"tools\" are in play', p.490; Fibonacci/volatility-band confluence) and the only exits are an initial stop plus 'some Fibonacci retracement' profit objective and 'a trailing stop that gives it some room' (intraday rules 4-5, pp.492-493).",
          &["F|Five swing points 1-5 with 'ascending highs and descending lows'; 'minimum number of bars needed to form the pattern is seven' (RSTs - Identify the Pattern)",
            "E|'Entry for position trades is on the close above the high of the low day for buys and below the low of the high day for sells.' (Entry Guidelines 1, p.488)",
            "X|'Initial stops for position trades should be placed below or above the 5 point low or high unless it is beyond your percentage loss rule' (Guidelines 2, p.488)",
            "X|'The first profit objective I look for is some Fibonacci retracement to the previous swing point high ... take early profit on 40% - 50% of a position and let the balance ride with a trailing stop that gives it some room.' (Intraday RST Entry 5, p.493)"],
          "Swing point = 'a low with a higher low on each side' (1-3 bar swing points, gaps must be closed). Also TWTG 2004 ch.15."),
        d("khtwg_generals_pullback", "The Generals' Pullback", "V.3 Selection of Daily Chart Setups", "Long/Short",
          "Entry needs an undefined 'reversal pattern entry in the zone' or a re-cross 'on a volume increase'; the pullback must be 'because of general market conditions ... not on company-specific news' (judgement); no exit beyond a stop 'usually just below the 200 DEMA' (pp.79-86).",
          &["F|'the 25 DEMA must be above the 200 DEMA and the 75 DEMA must be above the 200 DEMA' (p.80)",
            "F|'The Sweet Buy Zone is between the 75 and 200 DEMAs'; 'the recent high must be a new high in the recent trend, not a 1,2,3 lower high' (pp.80-81)",
            "E|reversal pattern entry in the zone, else nearest Fibonacci level, else 're-crosses the 200 DEMA to the upside on a volume increase' (p.81)",
            "X|stop 'usually just below the 200 DEMA' (p.82); no profit exit"],
          "Weekly 5/15/40 EMAs used to identify; 20 or 14,3,3 slow stochastic on 80/20 'preferred'."),
        d("khtwg_820_trend", "\"820\" Trend Identification Method", "V.2 Selection of Daily Chart Setups", "Long/Short",
          "A trend filter only (weekly/daily/60/15/5-minute 8 and 20 EMAs); no entry order or exit (pp.64-78).",
          &["F|'The strongest trend will have the 8 EMA trading above the 20 EMA with price above both rising EMAs (sells reversed).' (p.65)",
            "F|'for intermediate trades, the 8 EMA must be above a rising 20 EMA on the weekly chart' (p.67)"],
          "Used to narrow a stock list chosen by screens."),
        d("khtwg_523_trend", "\"523\" Trend Method", "V.4 (120-minute chart)", "Long/Short",
          "120-minute bars (intraday) and no defined entry or exit: 'Retracements to a rising 20 and 34 EMA are excellent setups'; 'Initial stops are money stops' (pp.87-93; Inner Circle 1/31/2005).",
          &["F|'The ideal trend is the 5 EMA > 20 EMA which is > the 34 EMA.' (120-minute chart, p.88)",
            "X|'Initial stops are money stops and best when below both the 20 and 34 EMAs' (p.89); no profit exit"],
          "Would also need 120-minute bars (not in the vault)."),
        d("khtwg_roofers", "Roofers", "V.5 Selection of Daily Chart Setups", "Long",
          "Discretionary chart patterns (triangles, flags, cup and handles) within 10% of 52-week highs in stocks with 'strong RS', played with straddles; no entry level or exit (pp.94-103; TWTG 2004 ch.12).",
          &["F|'breakouts of continuation patterns ... that are no more than 10% from their 52-week highs' (p.94)"],
          "Option straddle variant needs option chains."),
        d("khtwg_pressure_screens", "Buying/Selling Pressure, 3-Day Wake Up Call, Change-In-Direction screens", "V.1 Buying/Selling Pressure", "Long/Short",
          "Stock-selection screens over the S&P 500 / NDX 100 universe; they pick candidates, they give no entry order or exit (pp.37-63).",
          &["F|3-Day Wake Up Call: 'closed above the previous two days' highs, has the largest daily range for the last three days, and the most daily volume in the same three days' (p.50)",
            "F|Change In Direction: 'at least a five-day low and yesterday made a higher high, higher low and closed above the high of the low day' (pp.51-52)"],
          "Would need a stock universe; no trade rules."),
        d("khtwg_volatility_bands", "Volatility Bands / Fading the Open", "VII. Volatility Bands; TWTG 2004 ch.11", "Long/Short",
          "Needs each stock's ATM call and put implied volatility (option data) and the fade is taken only 'when you get a setup'; covers are discretionary ('Short was covered at 119 5/8 by 9:45', TWTG 2004 p.11-4).",
          &["F|'STDV x AIV x Closing Price x Constant = Volatility Band'; constant = sqrt(1/365) = 0.0523421; STDV 1.0/1.28/1.5/2.0 (p.124)",
            "E|'enter short and long limit orders on the opening at the various bands' (TWTG 2004 p.11-4)",
            "X|(none stated)"],
          "Not mapped to VIX/India VIX: the book uses the stock's own ATM option IV."),
        d("khtwg_opening_reversal", "Opening Reversals (ORs)", "X.1 First Hour Strategies", "Long/Short",
          "5-minute intraday bars, and entry 'should be confirmed by improving market dynamics' (NYSE TICK divergence, futures premium, bid/ask); profit exit is 'Be quick to take some profits' (TWTG 2004 p.4-2).",
          &["F|'the stock sells off no more than 30% of its average daily range' (p.292)",
            "E|'Enter the trade one tick above the opening price, or first five-minute bar high if it is close to the opening price.' (p.292)",
            "X|'Keep a tight money stop just below the open (not more than .30)' (p.292)"],
          "Intraday; stop is a stock-dollar amount."),
        d("khtwg_trap_door", "Trap Doors", "X.2 First Hour Strategies", "Long/Short",
          "5-minute intraday bars; entry requires 'some market dynamics improving'; profit exit is 'take profits on one-half of your position after the initial contra move and move to breakeven ... ASAP' (Trap Door Rules 5, 7, pp.347-348).",
          &["F|gap open (TWTG 2004 p.5-1: 'gaps of 3% or more' 'seems to work best'); ADX > 20 and rising, RSI > 50, above 50- and 200-day EMAs (TWTG 2004 p.5-1)",
            "F|'at least two bars, excluding the opening bar that will have a lower high and lower low than the preceding bar' (Rule 3, p.346)",
            "E|'Entry is above the high of the signal bar' (Rule 5, p.347); signal bar 'should close above the mid-point' (Rule 4)",
            "X|'The initial stop is placed below the signal bar low' (Rule 6, p.347); profit exit discretionary (Rule 7, p.348)"],
          "Intraday."),
        d("khtwg_gap_pullback", "Gap Pullbacks (Net Plus / Net Minus)", "X.3 First Hour Strategies", "Long/Short",
          "5-minute intraday bars with undefined volume terms ('good volume', 'less volume', 'pick up in volume'); the 50% retracement 'is a guideline, not an absolute'; only an initial stop, no profit exit.",
          &["F|'Gap opening bar on good volume'; pullback bars on lighter volume; signal bar 'partially closes the gap and remains above the previous day's close and high', closes above the open in the top 25% of its range (Net Plus Buy 1-5, pp.369-370)",
            "E|'trade-through entry above the high of the signal bar' (6)",
            "X|'Initial stop is below the low of the signal bar, unless it's a wide-range bar ... or else a money stop' (7)"],
          "Intraday."),
        d("khtwg_flip_top", "Flip Tops", "X.4 First Hour Strategies; TWTG 2004 ch.6", "Long/Short",
          "5-minute intraday; 'it's not the pattern that makes this trade, it is your feel for the dynamics' (p.379); stop is a stock-dollar amount '1/4 - 1/2 point'; no profit exit.",
          &["E|'Stock opens, then trades at least 1/4 point above the open'; reverses below the open; 'Enter 1/8 above the high of step 1' (TWTG 2004 p.6-2)",
            "F|'Only take the trade if the spread between the high (1) and the low (2) is no more than half of the stock's average daily range' (p.6-3)",
            "X|'Stops should be no more than 1/4 - 1/2 point from entry' (p.6-2)"],
          "Intraday."),
        d("khtwg_slim_jim", "Slim Jims", "XI. Slim Jims; TWTG 2004 ch.9", "Long/Short",
          "5-minute intraday consolidation 'at or near the current day's high'; entry also needs 'Market dynamics must be positive, with the futures moving up and your quote screen showing most of the stocks positive' (Entry Rule 3, p.418); no profit exit.",
          &["F|'a minimum seven-bar consolidation on the five-minute intraday chart at or just below the high of the day' (Entry Rule 1, p.416)",
            "E|buy the breakout of the consolidation",
            "X|stop just under the consolidation; alternate stop 'no more than .25 to .30' (p.449)"],
          "The separate TradingMarkets 'Slim Jim' course (SlimJim.rar) could not be opened (no RAR extractor) - see README."),
        d("khtwg_volcano", "Volcanoes", "TWTG 2004 ch.7", "Long",
          "Intraday rising-wedge breakout; 'The market dynamics must be positive, your screen mostly green'; stop 'risk no more than 1/4 to 1/2 point'; no profit exit (pp.7-1/7-2).",
          &["F|'Must be a minimum of five bars in pattern.' (7-1)", "E|'Buy breakout of pattern' (7-1)", "X|'risk no more than 1/4 to 1/2 point' (7-1)"],
          "Intraday."),
        d("khtwg_first_consolidation", "First Consolidation Breakouts To New Intraday Highs", "XII", "Long/Short",
          "Intraday consolidation breakout read by eye ('take early entry with a tight stop. Only do this if ...' dynamics); no exit (pp.457-471).",
          &["E|first consolidation breakout to new intraday highs"], "Intraday."),
        d("khtwg_fib_above_open", "Fibonacci Retracements Above the Open", "TWTG 2004 ch.8", "Long/Short",
          "Intraday, entry only 'on a reversal-bar pattern' at the .618 level ('you don't take a trade just because it fits a number'); stop .786; no profit exit (pp.8-1/8-2).",
          &["E|'only play the .618 level' on a reversal bar (8-1)", "X|'Your stop should never be lower than the .786 level' (8-1)"], "Intraday."),
        d("khtwg_double_bottoms", "Double Bottoms & Tops (1,2,3 higher / lower 'Shake & Bake' / same)", "TWTG 2004 ch.13; TWTG III IX", "Long/Short",
          "Trade criteria are 'First entry zone bar patterns', 'RSI divergence', 'Volume clues', 'Market and Stock Dynamics (are they coming for the stock?)' - judgement; trendlines drawn by eye; no exit (13-1).",
          &["F|1,2,3 swing-point bottoms after a near-term trendline break (TWTG III pp.199-272)"], ""),
        d("khtwg_sequence", "Sequence trading (Fibonacci retracements/extensions, parallel lines, time counts)", "VIII. Trading Tools; Inner Circle 2/7/2005", "Long/Short",
          "Haggerty: \"'Sequence' is an approach to trading the markets, not a mechanical system. It doesn't stand alone. Judgment and discipline are necessary.\" (TWTG 2002 workbook 3, Sequence Trading).",
          &["F|retracements .38/.50/.618/.786, extensions 1.27/1.618, extreme 2.0/2.24/2.618; entry only on 'a definite reversal bar pattern and confirming market/sector dynamics'"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("khtwg_").then_some(("khtwg", "Trading With The Generals (Haggerty)", "Not testable"))
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
        assert!(c.iter().all(|d| d.id.starts_with("khtwg_") && d.book == super::BOOK));
    }
}
