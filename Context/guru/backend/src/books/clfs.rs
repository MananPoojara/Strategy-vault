//! John F. Clayburg, "Four Steps to Trading Success: Using Everyday Indicators to Achieve Extraordinary Profits"
//! (John Wiley & Sons, 2001). Book id `clfs`. Source: research drive ".../John Clayburg - Four Steps to Trading
//! Success/Four Steps To Trading Success by John Clayburg.pdf" (scanned, 158 PDF pages = 2 book pages each) and the
//! book excerpt shipped as "Clayburg Directional Day Filter Manual.doc"; copies in Books/John Clayburg/.
//! Transcription and page refs: library/clfs/.
//!
//! The whole book is a day-trading method on 1- to 5-minute charts of stocks and S&P futures. Its four steps
//! (major trend of the day, minor trend, entry point, exit point) are read off the chart by the trader; none is a
//! complete mechanical rule, so nothing here is backtested and no rule is completed on the book's behalf.
use crate::connors::{ParamDef, Params, Status, StrategyDef, Trade, Ctx};

pub const BOOK: &str = "clfs";

const DDF: &str = "discretionary and intraday: the day's trend is judged by eye from the 'amount of trading activity' above or below the first-5-minute midpoint one hour into the session and whether the 60-minute bar closes 'significantly' above or below it; the book calls it 'more of a chart reading tool than an indicator' and gives no numeric threshold";
const FOUR: &str = "discretionary and intraday: entries combine the Directional Day Filter (judgement) with dual-setting stochastic / RSI / Percent R signals on 1-2 minute charts and 'Category 1-3' support and resistance points chosen by the trader; exits are trailing stops at market-defined support/resistance. No complete numeric rule set is given";
const DDFBO: &str = "incomplete rules, and intraday: the breakout time, profit target and stop are left to the trader ('these system parameters are presented here as examples only. These issues have not been tested'), the stop is a trailing stop 'governed by the appearance of market-generated support and resistance', and no end-of-day exit is stated; it also needs 5-minute stock bars";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("clfs_directional_day_filter", "Directional Day Filter (trend of the day)", "Ch. 10 · Directional Day Filter", Status::NotTestable(DDF),
            &["F|After five minutes record the day's high and low; their average is the critical line for the rest of the session (pp. 139-; DDF manual).",
              "F|One hour into the session: if there is 'substantially greater activity' above the line than below, the trend of the day is up (take only buy signals); more activity below = down day; no observable difference = sideways day.",
              "F|Also note whether the close of the 60-minute bar is 'significantly' above or below the line; observation may be delayed up to 2.5 hours."],
            "A filter, not a trading system: it has no entry or exit of its own. Judgement words ('substantially', 'significantly', 'observation') have no numeric definition in the book. Intraday (needs the first 5 and 60 minutes of each session)."),
        d("clfs_four_step_method", "Four-step day-trading method (dual oscillators + support/resistance)", "Ch. 5-9, 11-12 · Four steps", Status::NotTestable(FOUR),
            &["F|Step 1: define the major trend of the day with the Directional Day Filter.",
              "F|Step 2: define the minor trend within the major trend (dual-sensitivity oscillators, exhausted corrections).",
              "E|Step 3: enter on a dual stochastic / dual RSI / Percent R signal in the direction of the major trend when the trader's support or resistance point is broken.",
              "X|Step 4: exit at a trailing stop at market-generated support/resistance or on an opposite oscillator signal."],
            "All examples are 1-2 minute charts of Nasdaq stocks and S&P futures (2000). Support/resistance 'categories' and trend calls are made by the trader; not mechanical."),
        d("clfs_ddf_breakout", "Directional Day Filter breakout system", "Ch. 13-14 · DDF breakout", Status::NotTestable(DDFBO),
            &["E|Buy above / sell below the intraday range established a chosen number of minutes after the open (the book studies 15-300 minutes and favours 55-110 for QLGC, 65-95 for its 80-stock list) (pp. 235-251).",
              "X|Profit target and stop 'determined by such system testing' for each stock (e.g. PMCS: 75 minutes, $4.25 target, $3 stop — 'examples only'); ch. 13 uses a trailing stop at market-generated support and resistance (p. 245)."],
            "The book's tables are breakout-frequency statistics (share of days with a one-sided breakout), not trade results. Needs 5-minute stock bars and a complete exit rule."),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "clfs_directional_day_filter" => Some(("clfs_ddf", "Clayburg Directional Day Filter", "Trend-of-day filter · not testable")),
        "clfs_four_step_method" => Some(("clfs_ddf", "Clayburg Directional Day Filter", "Four-step method · not testable")),
        "clfs_ddf_breakout" => Some(("clfs_ddf", "Clayburg Directional Day Filter", "Early-range breakout · not testable")),
        _ => None,
    }
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
