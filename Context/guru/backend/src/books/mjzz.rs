//! Michael S. Johnson, "The Zigzag Signpost Trading Plan!" (+ Q&A Update, Final Update, Extra; superformula.com),
//! R:/USB HDD 2/tRADING KNO-2/Michael Johnson - The Zigzag Signpost Trading Plan! (superformula.com)/; copies in
//! Books/Michael S Johnson/. Book id "mjzz". Notes: library/mjzz/README.md. Every method leaves the exit (or the entry
//! timing) to the trader: nothing is Live.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "mjzz", rules, notes, claim: None };
    vec![
        d("mjzz_hilo_signpost", "Hi-lo signpost day trade (open = high or open = low)", "Part 1-2 · Day trading",
          Status::NotTestable("the entry is taken after watching the first 8-10 minutes until 'little upward movement has shown' plus a 'Market Strength' check (judgement, intraday); targets are 10 cents / 10 SPI points on ASX shares, and 'Make your own judgements on how much profit and how much loss to take'"),
          &["F|\"All we are interested in is The Open and the High being the same or The Open and the Low being the same Or ... within 1 point of each other\" (p.10)",
            "E|\"After this has gone on for 8 to 10 minutes I 'Sell CBA' ... and am in the market at 3219\" (p.27)",
            "X|\"I'm a 10 cent man or a 10 point man on a day trade\"; \"I put the stop loss at the opening price of the day ... or you can take a stop loss of anything between 5 and 10 cents\" (p.27-28)"],
          "Needs intraday ASX share / SPI data and a judgement entry time."),
        d("mjzz_predicted_range", "Next-day predicted high/low 'formula' entries", "Part 3 · The formula",
          Status::NotTestable("the formula that predicts the next day's high and low is not given in the manual (only its outputs, e.g. 'a low of 3004 and a high of 3057'); profit and stop 'Stops can be changed'"),
          &["E|\"place our 'BUY' order within 5 or 6 points of the estimated low\" (p.43)", "X|\"stop profits of 25 cents and stop losses of 15 cents. OCO\" (example, p.45)"],
          "Formula missing."),
        d("mjzz_ema_8_55", "The 8 and 55 technique (EMA crossover)", "Part 4 · Final Zig Zag Signpost",
          Status::NotTestable("no mechanical exit: 'your exit day is when a peak, a high, of your choice is realised and or, as a last resort, when the 8dma moves downwards across the 55dma'; testing the cross alone would drop the stated primary exit; 'Place stop losses after substantial movements' is unquantified"),
          &["E|\"Your entry points are when the 8 day crosses the 55 day moving upwards\" (p.63); moving averages \"are always exponential\" (p.61)",
            "X|\"your exit day is when a peak, a high, of your choice is realised and or, as a last resort, when the 8dma moves downwards across the 55dma\" (p.63)",
            "E|Short: \"when the 8 day crosses the 55 day moving downwards and your exit day is when a trough, a low, of your choice is realised and or, as a last resort, when the 8dma moves upwards across the 55dma\" (p.63)"],
          "Exit is the trader's choice."),
        d("mjzz_dancing_cfds", "Dancing with CFDs: MACD 30/10/10 early warning + 8/13 EMA cross, 21 EMA stop", "Part 4 · Final Zig Zag Signpost",
          Status::NotTestable("entry has 'choices' (early-warning or crossover), 'dancing' periods are judged by eye, the first stop is 'at my discretion' / 'the lowest low of the last few days ... it may be 1, or it may be 5', and profit-taking is 'as soon as I saw the profit takers'"),
          &["E|\"The confirmed entry point is when the 8 day crosses the 13 day moving average\" (p.67); \"it never pays to take the crossover if it does not agree with the early warning direction indicator\" (MACD 30:10:10, p.74)",
            "X|\"If I enter a trade on the crossover day I will place a stop at my discretion for the first day and when the share has also crossed the 21dma I use this as the ongoing daily stop loss\" (p.73)"],
          "Recorded so the page shows it; no code."),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("mjzz_").then_some(("mjzz", "Johnson Zigzag Signpost", match id {
        "mjzz_hilo_signpost" => "Hi-lo signpost day trade", "mjzz_predicted_range" => "Predicted range", "mjzz_ema_8_55" => "8/55 EMA", _ => "Dancing with CFDs" }))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
