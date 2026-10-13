//! Tom Bierovic, *Playing for Keeps in Stocks & Futures* (Wiley, 2002).
//! All three setups (First Prize, R2D2, Triple Play) have objective tools but leave the exit to a
//! discretionary trailing stop ("trail a stop ... while still giving the market some room to run"),
//! and the pattern anchors (impulse wave start, divergence lows, the three declines) are chosen by eye.
//! No exit is added. Source and transcription: library/tbpk/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tbpk";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, why, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("tbpk_first_prize", "First Prize", "Ch 4 · First Prize",
          "the exit after the initial stop is 'trail a stop (raise the stop) to lock in some profits while still giving the market some room to run' (p.69) with no rule, and the start of the impulse wave is 'usually' the 21-day low but 'sometimes other lows ... seem more logical' (p.64-67)",
          &["F|Tools: candlestick chart, 13- and 34-day EMAs, 13-day DMI spread, 3-8-13 MACD, 21-day ATR (p.68)",
            "F|Long setup: 1. 13-day EMA above 34-day EMA. 2. The market makes a new 21-day high, suggesting an impulse wave. 3. The DMI spread is greater than +15 on the day the market makes its most recent 21-day high. 4. Today's close is between the 38.2% and 61.8% retracements of the impulse wave. 5. The MACD line crosses below its signal line for the first time since the signal line crossed above zero, and the signal line remains above zero. 6. The MACD histogram is rising (greater than yesterday). (p.69)",
            "E|\"Buy tomorrow above today's high\" (p.69)",
            "X|\"1. Place an initial protective stop below the low of the corrective wave. 2. As prices rise, trail a stop (raise the stop) to lock in some profits while still giving the market some room to run.\" (p.69-70). Short setup mirror (p.70-71)"],
          "p.71: 'For each trade, I selected an exit that worked very well in that particular example' — the worked examples use different exits (2×ATR chandelier, 5-day low, 3-day high, parabolic, targets, MACD cross, trendlines, EMA34, DMI spread), chosen after the fact, so none is the book's rule. Not run with the initial stop alone (that is not the book's exit)."),
        d("tbpk_r2d2", "R2D2 (reverse divergence)", "Ch 5 · R2D2",
          "the reverse-divergence points A, B, C have no numeric swing definition (p.97-101) and after breakeven the book says 'trail a tight protective stop ... while still giving the market some room to run' (p.102) with no rule",
          &["F|Tools: 55-day EMA, 13-day DMI spread, 3-8-13 MACD (p.101)",
            "F|Long setup: 1. Closing prices above the EMA throughout the setup. 2. MACD line forms a bullish reverse divergence (price low C above low A, MACD at C below MACD at A). 3. DMI spread > +15 at point B. 4. MACD histogram higher than yesterday. (p.101-102)",
            "E|\"Buy today above yesterday's high\" (p.102)",
            "X|Initial stop below the higher of the two lows (point C) or the low of the entry day, whichever is lower; after a close above the highest high between the two lows raise the stop to breakeven and trail a tight stop (p.102). Short mirror (p.102-103)"],
          "Worked examples use different exits per trade (2-day low, trendline, DMI spread, MACD cross, measured move, parabolic)."),
        d("tbpk_triple_play", "Triple Play (multiple divergence)", "Ch 6 · Triple Play",
          "the three (or more) declines/rallies and the multiple divergence are identified on the chart with no numeric definition (p.135-139, Figs 6.1-6.2), entries may come early from 'gaps, candles, trendlines', and the exit is a discretionary trailing stop (p.139)",
          &["F|Tools: candlestick chart, EMA channel of the 13-day EMA of highs and 13-day EMA of lows, 3-8-13 MACD (p.139)",
            "F|Long setup: 1. The lows of the three (or more) declines are all below the EMA channel. 2. The MACD line forms a multiple bullish divergence (p.139)",
            "E|\"1. Buy when prices penetrate the top of the EMA channel. 2. Consider gaps, candles and trendlines for an earlier entry\" (p.139)",
            "X|\"1. Initial stop below the low of candle C. 2. Trail a stop to lock in some profits while still giving the market some room to run.\" (p.139-140). Short mirror (p.140-141)"],
          "Chart-reading pattern; no exit is added."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "tbpk_first_prize" | "tbpk_r2d2" | "tbpk_triple_play" => Some(("tbpk_setups", "Bierovic setups", "Not testable")),
        _ => None,
    }
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn nothing_is_live_and_nothing_runs() {
        let c = super::catalogue();
        assert_eq!(c.len(), 3);
        assert!(c.iter().all(|d| matches!(d.status, crate::connors::Status::NotTestable(_))));
    }
}
