//! Daryl Guppy, "Better Trading with the Guppy Multiple Moving Average" WORKBOOK (guppytraders.com, 27 slides).
//! Book id "dggw". Catalogue only: every GMMA entry and exit in the workbook is chart reading.
//! Source copy: Books/Daryl Guppy/guppymawb.pdf (slides read in full; images library/dggw/two/).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, rules, why| StrategyDef { id, name, group, session: 0, dir, status: Status::NotTestable(why), rules, notes: "GMMA = EMAs 3, 5, 8, 10, 12, 15 (short-term group) and 30, 35, 40, 45, 50, 60 (long-term group) (slide 26).",
        needs_volume: false, needs_vix: false, needs_series: &[], book: "dggw", claim: None };
    vec![
        d("dggw_mid_trend", "GMMA mid-trend entry", "Slides 5-7", "Long",
          &["E|\"Conclusion: Enter when shortest of averages begin to turn up\" after judging \"Long term group character, Short term group character, Relationship between the two groups\" (slides 6-7)"],
          "chart reading: group 'character', 'well separated' and 'turn up' are not defined numerically"),
        d("dggw_breakout", "GMMA breakout decisions", "Slides 8-14", "Long",
          &["E|\"Safe breakout entry ... buy on pullback or evidence of rebound\"; \"Risk of trend failure still high. Use aggressive stops\" (slides 13-14)"],
          "trend lines, 'compression', 'separation' and 'evidence of rebound' are judgement"),
        d("dggw_bubble", "Trading bubbles", "Slides 15-17", "Long",
          &["E|\"Safe entry with stop on lower long term group\" (slide 17)"], "bubble identification is visual; no exit"),
        d("dggw_exit", "GMMA exit confirmation", "Slides 18-21", "Long",
          &["X|\"1 First penetration below long term group since start of up trend 2 Short term group compressed, turned down and has begun to expand 3 Long term group starting to turn down and compress. Conclusion. GMMA confirms other exit signals\" (slide 21)"],
          "exit-only and confirms other (trend-line) exit signals"),
        d("dggw_downtrend", "Downtrend rallies and GMMA short trades", "Slides 22-23", "Long/Short",
          &["E|\"Sell as short term group fails to break above long term group\"; \"Buy rebound\" (slide 22)",
            "E|\"Enter short when previous rally trade is closed ... Short term group turning down. Aggressive entry or wait a few days for confirmation\"; \"Cover shorts\" when the short group is above a compressed, turned-up long group (slide 23)"],
          "'fails to break', 'turning down', 'compressed' are judgement; the entry is 'aggressive or wait a few days'"),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
