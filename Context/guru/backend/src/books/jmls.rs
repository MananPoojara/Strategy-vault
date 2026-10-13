//! Josh Murakami, "Trading System Design: The L/S Model Trading System and Other Trading System Analysis" (c.2002).
//! Book id `jmls`. Source copy: Books/Josh Murakami/L-S Model Trading System.pdf. Transcription: library/jmls/.
//! Every system in the book runs on intraday bars (hourly, 15-minute, 30-minute pit S&P); the vault has no
//! intraday S&P/NIFTY bars, so all are NeedsData and `run` returns None (no daily approximation).
use crate::connors::{ParamDef, Params, Status, StrategyDef, Ctx, Trade};

pub const BOOK: &str = "jmls";

macro_rules! nd {
    ($id:expr, $name:expr, $group:expr, $why:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: "Long/Short", status: Status::NeedsData($why),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        nd!("jmls_ls_model", "The L/S Model", "The L/S Model (pp.72-77)",
            "Needs hourly (natural-hour) S&P bars: the closes of the 14:00, 15:00 and 16:00 ET hourly bars each day plus the prior session close; the vault has daily bars only.",
            &["E|\"If the time is greater than 1300 EST and less than 1605 EST and the market is greater than yesterday's close at the close of the hourly bar then go long\" (p.76-77)",
              "E|\"If the time is greater than 1300 EST and less than 1605 EST and the market is less than yesterday's close at the close of the hourly bar then go short\" (p.77)",
              "X|Always in: the opposite signal reverses (\"If there is a reversal in the afternoon and the market has gone against yesterday's close then the system will reverse\", p.78). Code: If time > 1300 and time < sess1endtime - 10 then begin If close>closeD(1) then buy this bar at close; If close<closeD(1) then sell this bar at close; End;"],
            "Pit S&P futures, hourly bars, 1/96-7/02, $250/point, no inputs (p.74). Optimisation of the time filter on natural hour bars (p.80): 1000 $483,012.5 35% $290.80 avg; 1100 $563,962.5 38% $403.70; 1200 $614,812.5 42% $514.49; 1300 $637,137.5 44% $616.78 (maxDD -$49,100, PF 1.55); 1400 $576,712.5 45% $663.65; 1500 $302,912.5 42% $420.13. Performance report pages are images."),
        nd!("jmls_ls_atr_filter", "The L/S Model with ATR(35) > 3 volatility filter", "The L/S Model Dissected (pp.78-84)",
            "Needs hourly S&P bars (35-bar average true range of hourly bars and the hourly L/S signals); the vault has daily bars only.",
            &["F|\"The filter requires the average true range of the past x hourly bars to be greater than 3\" (p.82); code: If avgtruerange(35)>3 then begin {system code here} End;",
              "E|L/S Model entries as in jmls_ls_model", "X|Always in, reversing (p.78)"],
            "December 1983-July 2002 pit S&P (p.81-83), equity curve only."),
        nd!("jmls_bar_breakout", "Bar Breakout System", "The Bottom Line (pp.22-24)",
            "Needs 15-minute S&P bars (25-bar highest high / lowest low stop orders within the session); the vault has daily bars only.",
            &["E|\"on a 15 minute bar chart, buy at the highest high of the last 25 bars and sell at the lowest low of the last 25 bars\" (p.22)",
              "E|Code: Input: EntryBars(25); If time > Sess1StartTime and time < Sess1EndTime then begin Buy next bar at Highest(High, EntryBars) Stop; Sell next bar at Lowest(Low, EntryBars) Stop; End (p.23-24)",
              "X|Stop and reverse; \"this is without any exit techniques\" (p.24)"],
            "Five years: $463,087 net (p.22), historic win rate 42% (p.23), max intraday drawdown $38,550 (p.24); $80,000 per contract FFP version traded up to 130 contracts (p.52)."),
        nd!("jmls_ma_cross_30m", "4/21 SMA crossover on 30-minute bars (optimisation example)", "Views on Optimization (pp.37-41)",
            "Needs 30-minute S&P bars; the vault has daily bars only.",
            &["E|\"long when the fast moving average is above the slower moving average and short when the fast moving average is below the slow moving average. On 30 min bars the fast simple moving average (ma) is set to 4 and the slow ... The best parameter ends up being 21\" (p.37)",
              "X|Always in (reverses on the opposite cross)"],
            "Optimisation table p.37-38: slow 21 -> $86,600, 182 trades, 39% win, $475.82 avg, maxDD -$39,175, PF 1.37. Shown as an example of parameter choice, not recommended by the author (p.41)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "jmls_ls_model" => Some(("jmls_ls", "The L/S Model", "Core, hourly")),
        "jmls_ls_atr_filter" => Some(("jmls_ls", "The L/S Model", "ATR(35) > 3 filter")),
        "jmls_bar_breakout" => Some(("jmls_bb", "Bar Breakout", "25-bar, 15-minute")),
        "jmls_ma_cross_30m" => Some(("jmls_ma", "SMA crossover", "4/21, 30-minute")),
        _ => None,
    }
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> {
    None
}

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> {
    None
}
