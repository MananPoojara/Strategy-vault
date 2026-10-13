//! "LSS - An Introduction to the 3-Day Cycle Method" (byline in the text: George Angell; PDF metadata author: Richard
//! Hollander), R:/Trading Knowledge III/!PDFs/LSS. An introduction to the 3-day cycle method.pdf; copy in
//! Books/Richard Hollander/. Book id "rhlss". Notes: library/rhlss/README.md.
//! An introduction only: the LSS envelope formulas are not in the file, so nothing is Live. The Taylor 3-day idea with
//! fixed rules is already in the vault (rgt_taylor, lbrtt).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules, notes| StrategyDef { id, name, group: "LSS 3-day cycle (introduction)", session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "rhlss", rules, notes, claim: None };
    vec![
        d("rhlss_3day_cycle", "Taylor 3-day cycle: buy day / sell day / short-sell day",
          Status::NotTestable("no entry level, exit or stop is stated: the 'buy envelope' and 'sell envelope' numbers are the LSS software's and are not given; the cycle is 'rephased' by judgement; 'the 3-day cycle does not mean you can simply buy or sell every three days'"),
          &["F|\"Buy Day - low made first followed by a rally; Sell Day - market trades at or near the previous day's high; Short Sell Day - high made first followed by a decline\" (Part 2)",
            "E|\"You can buy several times within a so-called 'buy envelope', knowing the support should hold\" (Part 1); envelope formula not given",
            "X|\"you will probably be closing out positions with so-called MOC ('Market on Close') orders\" (Part 1)"],
          "Which of high/low came first needs intraday data, and the envelope levels are missing."),
        d("rhlss_ob_os", "One-day / 10-day overbought-oversold guideline",
          Status::NotTestable("the overbought/oversold indicator's formula is not given, choosing 'non-trending' vs 'trending' days is judgement ('this ability to recognize the kind of day'), and no exit is stated"),
          &["E|\"on the non-trending days, you can use the simple one-day overbought/oversold indicator ... fading rallies if the prior day's trading was in excess of 70 percent or buying dips if the prior day's reading was below 30 percent\" (Part 2)",
            "E|\"A trending market, however, calls for following the trend in the 10-day overbought/oversold oscillator - buying above 50 percent and selling below that level\" (Part 2)"],
          "Recorded so the page shows it; no code."),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("rhlss_").then_some(("rhlss", "LSS 3-day cycle (introduction)", if id == "rhlss_ob_os" { "OB/OS guideline" } else { "3-day cycle" }))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
