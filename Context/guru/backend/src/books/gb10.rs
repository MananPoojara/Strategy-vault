//! Geoff Bysshe (DataView LLC / MarketGauge), *Trading the 10 O'clock Bulls: Winning Strategies for Active Traders*
//! (2004/2006; the research-drive copy titled "Bysshe & Janko" is the same text). Book id `gb10`.
//! Source copies: Books/Geoff Bysshe/. Transcription and testability table: library/gb10/README.md.
//!
//! Every setup is defined on the 30-minute opening range (9:30-10:00 ET) and read from 1- and 5-minute charts with
//! judgement ("well-defined" support, "clean" breakout, "bullish" pattern); the book gives stops but no exit or
//! profit rule for any trade, and no daily-bar rule. Nothing here is Live.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "gb10";

const WHY: &str = "intraday bars (30-minute opening range read on 1-/5-minute charts) AND no exit: the book gives an entry and a stop but no profit exit or time exit for any setup, and the setups require chart judgement (\"well-defined\" OR, \"clean breakout\", swing lows); a daily-bar version would be a different, invented system";

macro_rules! def {
    ($id:expr, $name:expr, $dir:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: "Opening Range (30-minute)", session: 0, dir: $dir, status: Status::NotTestable(WHY),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def!("gb10_orb_initial", "OR breakout: buy the initial breakout", "Long (short mirror)",
            &["F|\"the 30-minute OR is defined as the stock's high and low for the day at 10:00 AM\" (p.8, p.22-23)",
              "E|\"buy the stock as soon as it breaks the OR high ... reserved for the most promising OR setups\" (p.31)",
              "X|\"your risk is defined by the low of the last swing prior to the breakout\" (p.31); worst case \"your maximum risk point for an OR breakout is the low of the OR\" (p.30)"],
            "Filters are judgement: well-defined support/resistance, bullish bias, unusual OR volume (p.24-28); daily filter \"20-day and the 50-day moving averages are positively sloped and the stock is trading above the 20-day\" and above the prior day's high (p.40). No exit is given."),
        def!("gb10_orb_retrace", "OR breakout: buy the retracement to the breakout", "Long (short mirror)",
            &["E|\"looking for the market to create a swing low at or near the breakout price level ... buy the stock with a stop below that swing low\" (p.32)",
              "X|\"or, more conservatively, below the low of the breakout's consolidation\" (p.32)"], "No exit is given."),
        def!("gb10_orb_second", "OR breakout: buy the second breakout", "Long (short mirror)",
            &["E|\"After the stock has consolidated or flagged lower this strategy buys the next breakout to new highs\" (p.34)",
              "X|\"your risk is the low of the second consolidation area\" (p.34)"],
            "\"A good rule of thumb is that it should trade above the OR for a period of time equal to half the time period of the OR\" (p.38). No exit is given."),
        def!("gb10_fade_or_low", "Fade the OR low (failed breakdown)", "Long (short mirror)",
            &["F|\"a second demonstration of support at the low of the day\", big-picture support, intraday relative strength vs the index (p.43)",
              "E|buy when the stock breaks above the consolidation formed after a failed break of the OR low (GTK, SEBL examples p.43-44)",
              "X|\"a stop below the low of the day\" (p.44)"], "No exit is given."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some(("gb10_or", "Bysshe · Opening Range", d.name))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
