//! Lee E. Gettess, "Trading in the Trenches" (SS-TDA, Intraday Market Mapping, 1993-95) bound with the "Scorpion S&P"
//! methodology and "Lee Gettess on Day Trading" special report. Book id `lgsc`. Source copy:
//! Books/Lee E. Gettess/Gettes.pdf (54 scanned two-page spreads). Library: library/lgsc/.
//! Nothing here is testable on the book's own terms: the SS-TDA tests print results but the text never states the
//! exit of the base test; the rest is chart reading / intraday swing judgement or leaves the key number to the reader.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lgsc";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("lgsc_sstda", "SS-TDA volatility breakout (65% of 2-day average true range)", "Trading in the Trenches · SS-TDA (p.5-14)",
          NotTestable("no exit stated: the text gives only the entry (open +/- 65% of the 2-day average true range); the printed SystemWriter tests (ATRBO test 5) imply but never state stop-and-reverse - adding it would be our exit, not the book's"),
          &["E|\"a volatility breakout based on 65% of the two day true range added and subtracted from the opening price ... Take the true range for the last two days, add them together, divide by 2, then multiply by 0.65\" (p.5)"],
          "Printed results (ATRBO test 5, 7/1/83-6/30/93): S&P INDEX 708 trades 45%, net $165,700; T-Bonds 665 trades 43%; D-Mark 552 trades 43%; Crude 666 trades 40% (p.6-9)."),
        d("lgsc_sstda_fpo", "SS-TDA with first-profitable-open exit", "Trading in the Trenches · SS-TDA (p.10-14)",
          NotTestable("only the profit exit (\"taken off when the market opened profitably basis the position\") is stated; no stop or loss exit is given"),
          &["E|Same SS-TDA entry (p.5)", "X|\"a trade was taken off when the market opened profitably basis the position ... my 'first profitable open' exit\" (p.10)"],
          "Printed results (ATRBO test 6): S&P INDEX 976 trades 73%, net $204,225 (p.13); T-Bonds 926 trades 74%; D-Mark 808 trades 71%; Crude 936 trades 68%."),
        d("lgsc_intraday_123", "Intraday Market Mapping (1-2-3 patterns, 3-bar filter)", "Trading in the Trenches · Intraday (p.15-30)",
          NotTestable("discretionary: 1-2-3 swings 'of some degree' on 15-30 minute charts chosen by the trader; needs intraday bars as well"),
          &["F|\"A 1-2-3 ... is formed when the market has made a low of some degree, swung up in the next few bars to make a high, then fallen back down to a low that is higher than the previous low\" (p.17)"], ""),
        d("lgsc_scorpion", "Scorpion S&P methodology (25/175 oscillator, Fibonacci retracements)", "Scorpion S&P (p.1-32 of part 2)",
          NotTestable("the author calls it 'a methodology, rather than a system': swing selection, divergences and stop placement are left to the trader"),
          &["F|\"we create a 25/175 period oscillator from exponential moving averages of the closes, then we also plot a 20 period moving average of the oscillator\" (p.4)",
            "F|\"I have deliberately not given hard and fast parameters for how much should be risked or how quickly to begin taking profits\" (p.29)"], ""),
        d("lgsc_daytrade_report", "Lee Gettess on Day Trading (special report)", "Day Trading report (p.1-12)",
          NotTestable("the displacement percentage is left to the reader ('Exactly what percentage to use is still a question') and no exit/stop is fixed"),
          &["E|Example: 55% of yesterday's range (87 points) added to / subtracted from the June 1997 S&P open (p.10-11)"], ""),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
