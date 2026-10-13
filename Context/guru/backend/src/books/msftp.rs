//! Mark McRae, "Sure Fire Trading Plans" (surefiretradingplans.com, 2006, 138 pp. PDF). Book id `msftp`. Source:
//! research drive "USB HDD 2/tRADING KNO-2/Mark McRae - Surefire Trading Plans (surefiretradingplans.com)/", copied to
//! Books/Mark McRae/. Refs are the printed page numbers. All three trading plans are discretionary by the book's own
//! words (stops at "the nearest support", exits "depending on market conditions", "I just eyeball"), so every entry is
//! NotTestable. `run` returns None.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "msftp";

const P1: &str = "discretionary: the setup is \"a bullish or bearish candlestick pattern at or near a Pivot Level\" (no list or tolerance), the stop goes \"at the nearest resistance or support area of the candlestick formation\" and the trend is read from monthly/weekly/daily pivots without a rule (p.92); stochastic/RSI divergence is judged by eye (p.95-99); shown on 15-minute FX charts";
const P2: &str = "discretionary exits and stops: \"Place your stop loss below the nearest support level ... The middle band can be used for the initial stop loss if it makes more sense\"; \"If the move is thrusting ... close the positions on the first close inside the Bollinger bands. If the market is drifting then the middle band can be used\" (p.108); \"having to use your discretion about how to exit\" (p.110). Entries are on a lower (e.g. 1-hour) time frame, so it would also need intraday bars";
const P3: &str = "discretionary by the book: \"This particular strategy is a lot more discretionary\" (p.117); the squeeze is \"when the 8 EMA comes very close to the 13 EMA\" - \"How close they should be I have never measured. I just eyeball\" (p.124); entry on \"the first solid bar\"; the trailing stop \"can be any one of the averages depending on market conditions\" (p.123)";

fn d(id: &'static str, name: &'static str, group: &'static str, why: &'static str, rules: &'static [&'static str]) -> StrategyDef {
    StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules,
        notes: "Examples are 15-minute / 1-hour forex charts; the book gives no back-test results. Money management (2% risk), the 'Free Trade' scale-out (Bill Poulos, p.72-74, partial exits at an unstated 'modest' target) and the Fibonacci 'Taking Profits' essay (Neal Hughes, p.75-78) are guest commentary without a complete rule.",
        claim: None,
    }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        d("msftp_plan1_pivot_candles", "Trading Plan #1: Pivot points with candlesticks", "p.79-99 · Trading Plan #1", P1, &[
            "F|\"Be aware of where the trend is as determined by the monthly, weekly and daily Pivot Points.\" (p.92)",
            "E|\"Look for a bullish or bearish candlestick pattern at or near a Pivot Level.\"; \"Enter on the first close above or below the candlestick formation.\" (p.92)",
            "X|\"Place your stop loss at the nearest resistance or support area of the candlestick formation.\"; \"Target the nearest Pivot Level.\" (p.92)",
            "F|Indicators for divergence: stochastic %K 15, 3, %D 3; RSI 14 (p.95-96)",
        ]),
        d("msftp_plan2_idrb_bollinger", "Trading Plan #2: Inside Day Reversal Bar + Bollinger Bands", "p.100-116 · Trading Plan #2", P2, &[
            "F|IDRB: an inside day, then a bar with \"a lower high and a lower low than the ID bar for a bullish setup\" / \"a higher low and a higher high than the ID bar for a bearish setup\" on a higher time frame (p.101)",
            "E|\"On the shorter time frame enter long on the first close above the upper Bollinger band and for short trades enter on the first close below the lower Bollinger band.\" Bollinger 20, 2 (p.104, 108)",
            "X|\"Place your stop loss below the nearest support level after entry ... The middle band can be used for the initial stop loss if it makes more sense.\"; \"If the move is thrusting and you are already in profit close the positions on the first close inside the Bollinger bands. If the market is drifting then the middle band can be used as the stop level.\" (p.108)",
        ]),
        d("msftp_plan3_rainbow_squeeze", "Trading Plan #3: Rainbow EMAs squeeze trade (and double squeeze)", "p.117-129 · Trading Plan #3", P3, &[
            "F|\"For short trades the 89 EMA should be below the 144 EMA\"; \"For long trades the 89 EMA should be above the 144 EMA\"; RSI(21) above 50 for longs / below 50 for shorts at entry and during the trade (p.123)",
            "E|\"A setup is identified when the 8 EMA comes very close to the 13 EMA forming a squeezing action.\"; \"The entry bar is the first solid bar in the direction of the trade\" (p.123)",
            "X|\"The stop loss is set initially above the nearest resistance ... Any one of the EMAs can also be used\"; \"Exit when your trailing stop loss is hit. Your trailing stop can be any one of the averages depending on market conditions. Generally the 55 EMA is a dependable trailing stop loss.\" (p.123)",
            "F|Double squeeze: a squeeze \"that has also just pierced the lower or upper Bollinger band but the whole setup remains above/below the 55 EMA, 89 EMA and the 144\" (p.127)",
        ]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id {
        "msftp_plan1_pivot_candles" => "Plan #1 pivots + candlesticks",
        "msftp_plan2_idrb_bollinger" => "Plan #2 IDRB + Bollinger",
        "msftp_plan3_rainbow_squeeze" => "Plan #3 rainbow squeeze",
        _ => return None,
    };
    Some(("msftp_plans", "McRae Sure Fire Trading Plans", v))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn catalogue_is_not_testable_and_owned() {
        for s in &super::catalogue() {
            assert!(s.id.starts_with("msftp_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NotTestable(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
