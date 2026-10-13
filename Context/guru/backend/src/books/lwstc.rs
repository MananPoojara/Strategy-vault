//! Larry Williams, "Sure Thing Commodity Trading" (seminar manual, 2006, 91 pp). Book id `lwstc`.
//! Source copy: Books/Larry Williams/Sure Thing Commodity Trading.pdf (text, read in full). Catalogue only: every rule needs
//! futures/COT/sentiment data, has no stated exit, or is already in the vault; see library/lwstc/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwstc";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{NeedsData, NotTestable};
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("lwstc_setups", "Set-up markets: WillVal, commercials (COT), large traders, public sentiment, ADX +60, pinch/paunch", "Section 1 · Set ups",
            NeedsData("Needs commodity and gold/bond/dollar futures (WillVal), weekly CFTC COT, Market Vane / Genesis sentiment; and \"the best plays come from a cluster of at least four\" set-ups (p.5), a judgement; no entry or exit."),
            &["F|WillVal: commodity / gold x 100, 2- and 22-period averages, 3-year %R; highs ~75, lows ~15 (p.6-7)", "F|COT index 3-year look-back, > 75% bullish, < 25% bearish; 12-month highs/lows; the Crouch (p.15-17)",
              "F|Large trader index (15 weeks) > 80% at tops, < 20% at bottoms (p.18)", "F|ADX > 60 near tops/bottoms; paunch: weekly 7-bar ADX above 40 with stochastic < 25% \"does not cause the entry\" (p.31-35)"], ""),
        d("lwstc_seasonals_oi_spreads", "Seasonal trades, open interest play, price premiums", "Section 1 · Set ups",
            NeedsData("Needs commodity futures, open interest and nearby/deferred contract spreads; the monthly lists give no buy/sell dates (p.38)."),
            &["F|Monthly tendencies and \"most reliable trades\" lists (p.39-41); soybeans from Oct 30 rallied 33 of 43 years for 5 days, avg +2% / -1.3% (p.42); lumber from end of October 77% for 20 days, +5.4% (p.42)",
              "F|OI decline ~20% in a consolidation after an uptrend = rally (p.56); premium coming on = bullish (p.58)"], ""),
        d("lwstc_entries", "Channel breakout, 18-day MA two-bar entry, qualified trend line, swing points, volatility breakout, 60/40 oscillator, Van Div", "Section 2 · Entry techniques",
            NotTestable("Entries to be taken only in a set-up market and without stated exits; trend lines and \"when primed\" are judgement. The Turtle rules quoted (20-day breakout / 10-day stop, 50-day / 20-day, p.60) are already coded as cov_turtle_s1/s2. The 18-day rule is lwicw_18day_ma (no exit)."),
            &["E|\"two consecutive days with lows that are above ... the 18 day moving average ... Neither of these days can be an inside day ... go long at the highest high of these two bars\" (p.62)",
              "E|\"buy at the opening price of the next day plus 90% of the previous day's range\" when \"primed\" (p.67)"],
            "18-day results table (p.63): profits in $000 over 10 years, e.g. B Pound 132.9, J Yen 147.5, Coffee 188.0 (commodities, all buys and sells; exit not stated)."),
        d("lwstc_exits", "Dollar stop, 3-bar exit (WillStop), Target Shooter", "Section 3 · Exit techniques",
            NotTestable("Exits only, used \"then and only then\" in a \"Rock and Roll\" market out of a trading range (p.70): judgement; target shooter needs intermediate highs/lows drawn by eye."),
            &["X|\"I'm willing to use a stop of about $1,500\" (p.70)", "X|3-bar: lowest true low of the last three non-inside days counted back from the highest close, a tick below (p.70)"], ""),
        d("lwstc_money_mgmt", "Money management, Monday gap bailout example, failure-day patterns, ProGo, specialist trap", "Sections 4-5",
            NotTestable("Money management is sizing, not a rule. The Monday gap bailout example (172 trades, 69%, $460.47, p.79) is lwicw_mon_gap_bail. Oops!, fake out, naked close, smash day and specialist trap have entries but no exits (lwicw_failure_days); ProGo crossing rules were \"provided at the seminar\" (p.85)."),
            &["E|Original Oops!, fake out, naked close, smash day (p.82-84); specialist trap after a 6-20 day box (p.87)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("lwstc_").then_some(("lwstc", "Sure Thing Commodity Trading", if id.contains("setups") || id.contains("seasonals") { "Needs data" } else { "Not testable" }))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
