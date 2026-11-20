//! YT Strategies · Kora Reddy: the team's AmiBroker AFLs for the strategies the team checked as "correct" (Checklist,
//! Dec 2021). There is no book PDF in the repository; the rules come from the AFLs and the newer "Kora Reddy Rules &
//! summary sheet" (Aug 2022), whose NIFTY parameters are the defaults here. The team ran these on NSE near-month
//! futures prices, so instrument admission allows only the NIFTY / BANKNIFTY near-month futures series (NSE daily
//! bhavcopy), never the cash index. Checked against the team's optimization tables and trade lists: BankNifty matches
//! every row; Nifty differs only where the share lacks 17 NSE daily files (Jul-Oct 2000).
//!
//! Conventions: daily bars, one position at a time. Weekday 1 = Monday .. 5 = Friday (AFL `DayOfWeek()`); bars on
//! other days never match. Gap up = open above the previous high; gap down = open below the previous low; lap up =
//! open above the previous close but below the previous high; lap down = open below the previous close but above the
//! previous low. Intraday methods enter at the open +/- a percentage when the day's high (long) or low (short)
//! reaches that level, and exit at the same day's close.
#![allow(dead_code)]
use crate::books::ytmr::day_number;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_kr";
pub const INSIDE: &str = "ytkr_inside_day";
pub const OUTSIDE: &str = "ytkr_outside_day";
pub const TURTLE: &str = "ytkr_turtle_short";
pub const TURTLE_GAPUP: &str = "ytkr_turtle_short_gapup";
pub const MAMM: &str = "ytkr_momentum_manic_monday";
pub const GAP: &str = "ytkr_gap_open_to_close";
// The 15 strategies the team's Checklist.xlsx marks "Incorrect" (the AFL does not follow the book's wording in the
// condition its Remark names). They are coded as the AFLs are written, which is what the team's workbooks test.
pub const BLOOD: &str = "ytkr_blood_on_street";
pub const THURSDAY: &str = "ytkr_buying_thursday";
pub const THURSDAY_MOD: &str = "ytkr_buying_thursday_modified";
pub const MANIC: &str = "ytkr_manic_monday";
pub const TUMBLES: &str = "ytkr_three_tumbles";
pub const TUMBLES_MOD: &str = "ytkr_modified_three_tumbles";
pub const TWO_TUMBLE: &str = "ytkr_two_tumble";
pub const STEPS: &str = "ytkr_three_steps";
pub const DUDD: [&str; 7] = ["ytkr_dudd1", "ytkr_dudd2", "ytkr_dudd3", "ytkr_dudd4", "ytkr_dudd5", "ytkr_dudd6", "ytkr_dudd7"];
// The 5 the checklist gives no verdict for, coded as their AFLs are written.
pub const OPEN_CLOSE: &str = "ytkr_buy_open_or_close";
pub const OPEN_CLOSE_LS: &str = "ytkr_buyshort_open_or_close";
pub const MON_FRI: &str = "ytkr_buying_mondays_fridays";
pub const CANDLE: &str = "ytkr_candle_colour";
pub const UUDDD: &str = "ytkr_uuddd";
// kora reddy all day.afl ("Weekday Short Long", added 20-11-2026); its code is in the section at the end of the module.
pub const WEEKDAY: &str = "ytkr_weekday_short_long";
pub const IDS: [&str; 29] = [INSIDE, OUTSIDE, TURTLE, TURTLE_GAPUP, MAMM, GAP, BLOOD, THURSDAY, THURSDAY_MOD, MANIC, TUMBLES, TUMBLES_MOD,
    TWO_TUMBLE, STEPS, DUDD[0], DUDD[1], DUDD[2], DUDD[3], DUDD[4], DUDD[5], DUDD[6], OPEN_CLOSE, OPEN_CLOSE_LS, MON_FRI, CANDLE, UUDDD,
    WEEKNUM, WEEKNUM_MF, WEEKDAY];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Kora Reddy", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    let mut v = vec![
        def(INSIDE, "Inside Day", "Short", &[
            "F|Yesterday's High < High 2 days ago and Yesterday's Low > Low 2 days ago (inside day)",
            "F|Yesterday's range < the range of each of the 3 days before (NR4)",
            "F|Today's Open < Yesterday's Low (gap down)",
            "E|Short at Open − Open % once the Low reaches it",
            "X|Cover at today's Close",
        ], "Inside Day (Kora Reddy AFL): short, intraday. Open % default 0.25 (team NIFTY setting). Note: the AFL fills at open - % when the day's low reaches it."),
        def(OUTSIDE, "Outside Day", "Long", &[
            "F|Yesterday's High > High 2 days ago and Yesterday's Low < Low 2 days ago (outside day)",
            "F|Yesterday's range is not > the range of each of the 6 days before (not WS7)",
            "F|Today's Open > Yesterday's High (gap up)",
            "E|Buy at Open + Open % once the High reaches it",
            "X|Sell at today's Close",
        ], "Outside Day (Kora Reddy AFL): long, intraday. Open % default 0.75 (team NIFTY setting)."),
        def(TURTLE, "Turtle Short", "Short", &[
            "F|Today = Entry weekday (default Monday)",
            "F|Yesterday's Close = lowest Close of the last N days, including yesterday (default 14)",
            "F|Today's Open < Yesterday's Low (gap down)",
            "E|Short at Open − Open % (default 0.5) once the Low reaches it",
            "X|Cover at today's Close",
        ], "Turtle Short (Kora Reddy AFL): short, intraday. The AFL text says Friday or Monday; its code tests one chosen weekday, kept here as a setting. Defaults (Monday, 0.5%, 14) are the team NIFTY setting."),
        def(TURTLE_GAPUP, "Turtle Short on GapUp", "Short", &[
            "F|Today = Entry weekday (default Friday)",
            "F|Yesterday's Close = lowest Close of the last N days, including yesterday (default 4)",
            "F|Today's Open > Yesterday's High (gap up)",
            "E|Short at Open − Open % (default 1) once the Low reaches it",
            "X|Cover at today's Close",
        ], "Turtle Short on GapUp (Kora Reddy AFL): short, intraday. Defaults (Friday, 1%, 4) are the team NIFTY setting; the AFL offers Monday or Friday."),
        def(MAMM, "Momentum Aided Manic Monday", "Short", &[
            "F|Today = Entry weekday (default Friday)",
            "F|Today's Close < lowest Close of the previous N days (default 14)",
            "F|Today's High < R1",
            "F|Today's Close < S1",
            "F|Pivots from yesterday: PP = (H+L+C)/3, R1 = 2PP − L, S1 = 2PP − H",
            "E|Short at today's Close",
            "X|Cover at the Close of the next Exit weekday (default Monday)",
        ], "Momentum Aided Manic Monday (Kora Reddy AFL): short at the close, held to the exit weekday's close; if that weekday is a holiday the trade waits for the next one, as in the AFL. Defaults (Friday, Monday, 14) are the team NIFTY setting."),
        def(GAP, "Gap Open to Close", "Long/Short", &[
            "F|Today = Entry weekday (default Monday)",
            "F|Today's Open has the chosen gap: 1 gap up, 2 gap down, 3 lap up, 4 lap down",
            "E|Side 1, long: Buy at Open + Open % once the High reaches it",
            "E|Side 2, short: Short at Open − Open % once the Low reaches it",
            "X|Exit at today's Close",
        ], "Buy at GapUp or GapDown & Sell at Close (Kora Reddy AFL): intraday, long or short. Defaults (Monday, 0.25%) are the team NIFTY setting; gap type and side default to gap up, long."),
        def(BLOOD, "Buy when there is blood on street", "Long", &[
            "F|Today = Entry weekday (default Tuesday)",
            "F|Today's Close < lowest Close of the previous N days (default 10)",
            "F|Today, Yesterday and 2 days ago: Open > Close (red candles)",
            "F|Version 3 only: Today's Close < Yesterday's Low",
            "E|Buy at today's Close",
            "X|Sell at the next day's Close",
        ], "Buy when there is blood on street (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"down day\" (the AFL reads a down day as a red candle, open above close). Versions 1 and 2 are the same code in the AFL. Defaults (version 1, Tuesday, 10) are the team NIFTY setting; the team ran it at 0.04% commission a side."),
        def(THURSDAY, "Buying Thursday", "Long", &[
            "F|Today = Entry weekday (default Thursday)",
            "F|Today's Open > Yesterday's Close",
            "F|Today's Close > Today's Open",
            "F|Today's Close > R1",
            "F|Today's Low > S1",
            "F|Pivots from yesterday: PP = (H+L+C)/3, R1 = 2PP − L, S1 = 2PP − H",
            "E|Buy at today's Close",
            "X|Sell at the next day's Close",
        ], "Buying Thursday (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's close was up and above R1\" (the AFL tests close above today's open, not above yesterday's close). The team's workbook says 06-09-2021 but its results match a run to 26-10-2021; 0.04% commission a side."),
        def(THURSDAY_MOD, "Buying Thursday (Modified)", "Long", &[
            "F|All the Buying Thursday conditions, on the Entry weekday (default Thursday)",
            "F|2 days ago is a Tuesday with a red candle (Open > Close)",
            "F|Or, through the AFL's state flag: 3 days ago is a Monday and 2 days ago is a red candle",
            "E|Buy at today's Close",
            "X|Sell at the next day's Close",
        ], "Buying Thursday (Modified) (Kora Reddy AFL), coded as the AFL is written, including its abc state flag. Team checklist: Incorrect, remark \"Tuesday's (or Monday's if Tuesday happens to be a holiday) close was down\" (the AFL checks a red candle by bar position). 0.04% commission a side in the team's run."),
        def(MANIC, "Manic Monday", "Short", &[
            "F|Today = Entry weekday (default Friday)",
            "F|Today and Yesterday: Open > Close (red candles)",
            "F|Today's High < R1 (pivot from yesterday: PP = (H+L+C)/3, R1 = 2PP − L)",
            "E|Short at today's Close",
            "X|Cover at the Close of the next Exit weekday (default Monday)",
            "X|If Entry and Exit weekday are the same: Cover at the same day's Close",
        ], "Manic Monday (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\" (the AFL has no test of today's open against yesterday's close). Defaults (Friday, Monday) are the team setting; the team's run used 0.04% commission a side."),
        def(TUMBLES, "Three tumbles and a jump", "Long", &[
            "F|Today ≠ Monday (the AFL's weekday setting is not used by its code)",
            "F|Today, Yesterday and 2 days ago: Open > Close (red candles)",
            "F|Today's Open > Yesterday's High (gap up)",
            "E|Buy at Open + Open % once the High reaches it",
            "X|Sell at today's Close",
        ], "Three tumbles and a jump (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Last three trading days were down closes\" (the AFL tests red candles), and \"Weekdays are not in param\". Default open % 0.25 is the team NIFTY setting."),
        def(TUMBLES_MOD, "Modified Three tumbles and a jump", "Long", &[
            "F|Today = Entry weekday (default Wednesday)",
            "F|Yesterday's Close = lowest Close of the last N days, including yesterday (default 6)",
            "F|Today, Yesterday and 2 days ago: Open > Close (red candles)",
            "F|No gap condition",
            "E|Buy at Open + Open % (default 0.25) once the High reaches it",
            "X|Sell at today's Close",
        ], "Modified Three tumbles and a jump (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Previous 3 days were down closes\" (the AFL tests red candles). Defaults (Wednesday, 0.25%, 6) are the team NIFTY setting."),
        def(TWO_TUMBLE, "Two Tumble & Another Tumble", "Short", &[
            "F|Today = Entry weekday (default Monday)",
            "F|Yesterday and 2 days ago: Close < Open (red candles)",
            "E|Short at Open − Open % (default 0.25) once the Low reaches it",
            "X|Cover at today's Close",
        ], "Two Tumble & Another Tumble for Short Intraday FO (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Yesterday and day before yesterday were down closes\" (the AFL tests red candles). The workbook's Info says 01-01-2007, but its results match a run from the start of the data; 0.02% commission a side."),
        def(STEPS, "Three steps and a stumble", "Short", &[
            "F|Today = Entry weekday (default Tuesday)",
            "F|Yesterday's Close > highest Close of the N days before it (default 16)",
            "F|Today, Yesterday and 2 days ago: Close > Open (green candles)",
            "F|Today's Open > Yesterday's High (gap up)",
            "E|Short at Open + Open % (default 0.25) once the High reaches it",
            "X|Cover at today's Close",
        ], "Three steps and a stumble (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"close up\" (the AFL tests green candles). The AFL shorts above the open. Defaults (Tuesday, 0.25%, 16) are the team NIFTY setting."),
        def(DUDD[0], "DUDD Version 1", "Short", &[
            "F|Today's range > the range of each of the 6 days before (WS7)",
            "F|Today red, Yesterday green, 2 days ago green, 3 days ago red",
            "E|Short at today's Close",
            "X|Cover at the next day's Close",
        ], "DUDD Version 1 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\" (the AFL tests candle colours). The workbook's Info says 1-1-2007, but its trade lists match a run from the start of the data; 0.04% commission a side."),
        def(DUDD[1], "DUDD Version 2", "Short", &[
            "F|Yesterday's range < the range of each of the 3 days before (NR4)",
            "F|Today's Open < Yesterday's Low (gap down)",
            "F|Yesterday red, 2 days ago green, 3 days ago red",
            "E|Short at Open − Open % (default 0.25)",
            "E|No check that the Low reaches it; the fill is held within the day's range",
            "X|Cover at today's Close",
        ], "DUDD Version 2 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"down close\". The AFL sets the short price without checking the day's low; AmiBroker then fills at the low, kept here. Trade lists match a run from the start of the data; 0.02% commission a side."),
        def(DUDD[2], "DUDD Version 3", "Short", &[
            "F|Yesterday's range > the range of each of the 3 days before (WS4)",
            "F|Today's Open < Yesterday's Low (gap down)",
            "F|Yesterday red, 2 days ago green, 3 days ago red",
            "E|Short at Open − Open % (default 0.25)",
            "E|The fill is held within the day's range",
            "X|Cover at today's Close",
        ], "DUDD Version 3 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\". Same fill as Version 2; 0.02% commission a side."),
        def(DUDD[3], "DUDD Version 4", "Short", &[
            "F|Yesterday's range < the range of each of the 6 days before (NR7)",
            "F|Today's Open < Yesterday's Low (gap down)",
            "F|Yesterday, 2 days ago and 3 days ago: red candles",
            "E|Short at Open − Open % (default 0.25)",
            "E|The fill is held within the day's range",
            "X|Cover at today's Close",
        ], "DUDD Version 4 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\". Same fill as Version 2; 0.02% commission a side."),
        def(DUDD[4], "DUDD Version 5", "Short", &[
            "F|Yesterday was an NR7 day",
            "F|Today's Open < Yesterday's Low (gap down)",
            "F|Yesterday red, 2 days ago green, 3 days ago red",
            "E|Short at Open − Open % (default 0.25)",
            "E|The fill is held within the day's range",
            "X|Cover at today's Close",
        ], "DUDD Version 5 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\". Same fill as Version 2; 0.02% commission a side."),
        def(DUDD[5], "DUDD Version 6", "Short", &[
            "F|Yesterday was an NR7 day",
            "F|Today's Open < Yesterday's Low (gap down)",
            "F|Yesterday green, 2 days ago green, 3 days ago red",
            "E|Short at Open − Open % (default 0.25)",
            "E|The fill is held within the day's range",
            "X|Cover at today's Close",
        ], "DUDD Version 6 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\". Same fill as Version 2; 0.02% commission a side."),
        def(DUDD[6], "DUDD Version 7", "Short", &[
            "F|Yesterday's range > the range of each of the 6 days before (WS7)",
            "F|Today's Open < Yesterday's Low (gap down)",
            "F|Yesterday red, 2 days ago green, 3 days ago red",
            "E|Short at Open − Open % (default 0.25)",
            "E|The fill is held within the day's range",
            "X|Cover at today's Close",
        ], "DUDD Version 7 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\". Same fill as Version 2; 0.02% commission a side."),
        def(OPEN_CLOSE, "Buy at Open or Close", "Long", &[
            "F|Today = Entry weekday (default Thursday)",
            "F|No other condition",
            "E|Mode 1: Buy at Open + Open %, with no check that the High reaches it (the fill is held within the day's range)",
            "E|Mode 2 (default): Buy at today's Close",
            "X|Mode 1: Sell at today's Close",
            "X|Mode 2: Sell at the next day's Close",
        ], "Buy at Open or Close and Sell at same day Close Or Next day Close (Kora Reddy AFL), coded as the AFL is written; the team's checklist gives no verdict. The team ran mode 1 at 0.02% and mode 2 at 0.04% commission a side over all quotes. Their mode-1 runs lose about 99%, after which AmiBroker skips trades the account cannot buy one unit of; this vault's normalized returns take every trade."),
        def(OPEN_CLOSE_LS, "BuyShort at Open or Close", "Long/Short", &[
            "F|Today = Entry weekday (default Monday)",
            "F|No other condition",
            "E|Mode 1 (default): Enter at Open + Open % (default 0, the team's setting), long and short alike, with no check that the bar reaches it (held within the day's range)",
            "E|Mode 2: Enter at today's Close",
            "X|Mode 1: Exit at today's Close",
            "X|Mode 2: Exit at the next day's Close",
        ], "BuyShort at Open or Close and SellCover at same day Close Or Next day Close (Kora Reddy AFL), coded as the AFL is written; the team's checklist gives no verdict. Unlike the other Kora strategies, the team ran it on the NSE spot indices, not futures: on NIFTY 50 (03-01-2000 to 31-03-2022) and NIFTY BANK (09-06-2005 to 31-03-2022), open % 0 and no commission, it reproduces all 40 rows of the team's 2022 optimization workbooks. The AFL prices a short at open PLUS the %."),
        def(MON_FRI, "Buying Mondays & Fridays", "Long", &[
            "F|Today = Entry weekday (default Monday)",
            "F|Today's Open > Yesterday's Close",
            "F|Yesterday's Open > Close 2 days ago",
            "F|Yesterday's Close > Yesterday's Open (green candle)",
            "E|Buy at Open + Open % (default 0.25) once the High reaches it",
            "X|Sell at today's Close",
        ], "Buying Mondays & Fridays for Intraday FO (Kora Reddy AFL), coded as the AFL is written; the team's checklist gives no verdict. Defaults (Monday, 0.25%) are the team setting; 0.02% commission a side."),
        def(CANDLE, "Entry Based on Candle Color", "Long/Short", &[
            "F|Today's candle has the chosen colour",
            "F|Colour 1 (default): Close > Open (green)",
            "F|Colour 2: Open > Close (red)",
            "E|Side 1 (default): Buy at today's Close",
            "E|Side 2: Short at today's Close",
            "X|Exit at the next day's Close",
        ], "Entry Based on Candle Color (Kora Reddy AFL), coded as the AFL is written; the team's checklist gives no verdict. The team's trade lists (all four colour and side combinations) match a run from the start of the data, not the 2007 in its info sheet; 0.04% commission a side."),
        def(UUDDD, "UUDDD", "Short", &[
            "F|Today, Yesterday, 2 days ago and 3 days ago: Open > Close (red candles)",
            "F|Today's Open < Yesterday's Low (gap down)",
            "E|Short at Open − Open % (default 0.25)",
            "E|No check that the Low reaches it; the fill is held within the day's range",
            "X|Cover at today's Close",
        ], "UUDDD (Kora Reddy AFL), coded as the AFL is written; the team's checklist gives no verdict. The AFL tests four red candles, not the up/up/down/down closes its name and info sheet describe. Trade lists match a run from the start of the data; 0.04% commission a side."),
    ];
    v.extend(weeknum_catalogue());
    v.push(weekday_def());
    v
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let pct = |d: f64| p("open_pct", "Open % (entry level from the open)", d, 0.0, 4.0, 0.25);
    let day = |d: f64| p("weekday", "Weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", d, 1.0, 5.0, 1.0);
    Some(match id {
        INSIDE => vec![pct(0.25)],
        OUTSIDE => vec![pct(0.75)],
        TURTLE => vec![day(1.0), pct(0.5), p("low_n", "Lowest close period (days, incl. yesterday)", 14.0, 2.0, 25.0, 1.0)],
        TURTLE_GAPUP => vec![day(5.0), pct(1.0), p("low_n", "Lowest close period (days, incl. yesterday)", 4.0, 2.0, 25.0, 1.0)],
        MAMM => vec![
            p("entry_day", "Entry weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 5.0, 1.0, 5.0, 1.0),
            p("exit_day", "Exit weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 1.0, 1.0, 5.0, 1.0),
            p("low_n", "Lowest close period (previous days)", 14.0, 2.0, 24.0, 1.0),
        ],
        GAP => vec![
            p("gap", "Gap type (1 gap up, 2 gap down, 3 lap up, 4 lap down)", 1.0, 1.0, 4.0, 1.0),
            p("side", "Side (1 long, 2 short)", 1.0, 1.0, 2.0, 1.0),
            day(1.0),
            pct(0.25),
        ],
        BLOOD => vec![
            p("version", "Version (1 Version 1, 2 Version 2, the same AFL code as version 1, 3 Version 3, adds close below yesterday's low)", 1.0, 1.0, 3.0, 1.0),
            p("entry_day", "Entry weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 2.0, 1.0, 5.0, 1.0),
            p("low_n", "Lowest close period (previous days)", 10.0, 2.0, 24.0, 1.0),
        ],
        THURSDAY | THURSDAY_MOD => vec![p("entry_day", "Entry weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 4.0, 1.0, 5.0, 1.0)],
        MANIC => vec![
            p("entry_day", "Entry weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 5.0, 1.0, 5.0, 1.0),
            p("exit_day", "Exit weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 1.0, 1.0, 5.0, 1.0),
        ],
        TUMBLES => vec![p("open_pct", "Open % (entry level from the open)", 0.25, 0.0, 3.0, 0.25)],
        TUMBLES_MOD => vec![day(3.0), pct(0.25), p("low_n", "Lowest close period (days, incl. yesterday)", 6.0, 2.0, 24.0, 1.0)],
        TWO_TUMBLE => vec![day(1.0), pct(0.25)],
        STEPS => vec![day(2.0), pct(0.25), p("high_n", "Highest close period (days before yesterday)", 16.0, 2.0, 25.0, 1.0)],
        OPEN_CLOSE => vec![p("mode", "Mode (1 buy at open + %, sell at close; 2 buy at close, sell next close)", 2.0, 1.0, 2.0, 1.0), day(4.0), pct(0.25)],
        OPEN_CLOSE_LS => vec![
            p("mode", "Mode (1 enter at open + %, exit at close; 2 enter at close, exit next close)", 1.0, 1.0, 2.0, 1.0),
            p("side", "Side (1 long, 2 short)", 1.0, 1.0, 2.0, 1.0),
            day(1.0),
            pct(0.0),
        ],
        MON_FRI => vec![day(1.0), pct(0.25)],
        CANDLE => vec![p("colour", "Candle colour (1 green, 2 red)", 1.0, 1.0, 2.0, 1.0), p("side", "Side (1 long, 2 short)", 1.0, 1.0, 2.0, 1.0)],
        UUDDD => vec![pct(0.25)],
        WEEKDAY => weekday_params(),
        id if id == DUDD[0] => vec![],
        id if DUDD.contains(&id) => vec![pct(0.25)],
        WEEKNUM | WEEKNUM_MF => weeknum_params(id),
        _ => return None,
    })
}

/// AFL DayOfWeek(): 0 = Sunday .. 6 = Saturday.
fn dow(x: &Ctx, i: usize) -> i64 {
    day_number(&x.r[i].date).map_or(-1, |d| (d + 4).rem_euclid(7))
}
fn range(x: &Ctx, i: usize) -> f64 {
    x.h[i] - x.l[i]
}
/// Lowest close of bars `end + 1 - n ..= end` (AFL LLV(C, n) at `end`). Like AmiBroker, the first bars of the data use
/// the closes available so far (no n-bar warm-up); the team's MAMM BankNifty table matches only this way.
fn lowest_close(x: &Ctx, end: usize, n: usize) -> f64 {
    x.c[(end + 1).saturating_sub(n)..=end].iter().cloned().fold(f64::INFINITY, f64::min)
}
/// Highest close of bars `end + 1 - n ..= end` (AFL HHV(C, n) at `end`), using the closes available near the start.
fn highest_close(x: &Ctx, end: usize, n: usize) -> f64 {
    x.c[(end + 1).saturating_sub(n)..=end].iter().cloned().fold(f64::NEG_INFINITY, f64::max)
}
/// AFL `Open > Close` (a red candle) and `Open < Close` (a green candle).
fn red(x: &Ctx, i: usize) -> bool {
    x.o[i] > x.c[i]
}
fn green(x: &Ctx, i: usize) -> bool {
    x.o[i] < x.c[i]
}
/// Yesterday's floor pivots (R1, S1): PP = (H+L+C)/3, R1 = 2PP - L, S1 = 2PP - H.
fn pivots(x: &Ctx, i: usize) -> (f64, f64) {
    let pp = (x.h[i - 1] + x.l[i - 1] + x.c[i - 1]) / 3.0;
    (2.0 * pp - x.l[i - 1], 2.0 * pp - x.h[i - 1])
}
fn gap_up(x: &Ctx, i: usize) -> bool {
    x.o[i] > x.h[i - 1]
}
fn gap_down(x: &Ctx, i: usize) -> bool {
    x.o[i] < x.l[i - 1]
}
fn lap_up(x: &Ctx, i: usize) -> bool {
    x.o[i] > x.c[i - 1] && x.o[i] < x.h[i - 1]
}
fn lap_down(x: &Ctx, i: usize) -> bool {
    x.o[i] < x.c[i - 1] && x.o[i] > x.l[i - 1]
}

fn trade(dir: f64, entry: usize, px: f64, kind: &'static str, exit: usize, exit_px: f64, reason: &str) -> Trade {
    Trade { dir, legs: vec![Leg { i: entry, px, w: 1.0 }], exit_i: exit, exit_px, reason: reason.into(), entry_kind: kind, signal_i: exit, open: false, exits: Vec::new() }
}

/// Intraday open-to-close trades: `setup(i)` decides the day, then a long fills at open x (1 + pct) if the high reaches it,
/// a short at open x (1 - pct) if the low reaches it; the exit is the same day's close.
fn intraday(x: &Ctx, from: usize, to: usize, long: bool, pct: f64, setup: impl Fn(usize) -> bool) -> Vec<Trade> {
    let mut out = Vec::new();
    for i in from.max(1)..=to {
        if !setup(i) {
            continue;
        }
        let level = if long { crate::books::ab::addmul(x.o[i], pct / 100.0) } else { crate::books::ab::addmul(x.o[i], -pct / 100.0) };
        let filled = if long { x.h[i] >= level } else { x.l[i] <= level };
        if filled {
            out.push(trade(if long { 1.0 } else { -1.0 }, i, crate::books::ab::ab_fill(level, x.l[i], x.h[i]), "limit", i, x.c[i], "Same-day close"));
        }
    }
    out
}

/// The AFLs keep their position in a loop over every bar of the data (`for(i = 1; i < BarCount; i++)`), so a run
/// that starts later still carries the position held on the day before it; AmiBroker then takes only the entries on
/// or after the start date. The loops here run from the start of the data and keep the trades entered from `from`.
fn from_start(mut out: Vec<Trade>, from: usize) -> Vec<Trade> {
    out.retain(|t| t.legs[0].i >= from);
    out
}

/// Enter at the close of a setup bar and exit at the next bar's close (the AFL's `else if(Position ...)` exit, so no
/// new entry on the exit bar). An entry on the last bar stays open.
fn close_next(x: &Ctx, from: usize, to: usize, dir: f64, start: usize, setup: impl Fn(usize) -> bool) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut cur: Option<usize> = None;
    for i in start..=to {
        match cur {
            None if setup(i) => cur = Some(i),
            None => {}
            Some(e) => {
                out.push(trade(dir, e, x.c[e], "close", i, x.c[i], "Next bar close"));
                cur = None;
            }
        }
    }
    if let Some(e) = cur {
        let mut t = trade(dir, e, x.c[e], "close", to, x.c[to], "open");
        t.open = true;
        out.push(t);
    }
    from_start(out, from)
}

/// AFL Buying Thursday (Modified): its `abc` flag, set on the entry weekday from the bar two (or three) back, then the
/// Buying Thursday conditions; buy at the close, sell at the next bar's close.
fn thursday_modified(x: &Ctx, from: usize, to: usize, day: i64) -> Vec<Trade> {
    let mut out = Vec::new();
    let (mut abc, mut cur) = (0, None::<usize>);
    for i in 3..=to {
        let e = dow(x, i) == day;
        if e && dow(x, i - 2) == 2 && red(x, i - 2) && abc == 0 {
            abc = 1;
        } else if e && dow(x, i - 2) != 2 {
            abc = 2;
        } else if e && dow(x, i - 3) == 1 && abc == 2 && red(x, i - 2) {
            abc = 3;
        } else if e && dow(x, i - 3) != 1 || !e {
            abc = 0;
        }
        let (r1, s1) = pivots(x, i);
        if cur.is_none() && e && x.o[i] > x.c[i - 1] && green(x, i) && x.c[i] > r1 && x.l[i] > s1 {
            if abc == 1 || abc == 3 {
                cur = Some(i);
            }
        } else if let Some(en) = cur {
            out.push(trade(1.0, en, x.c[en], "close", i, x.c[i], "Next bar close"));
            cur = None;
            abc = 0;
        }
    }
    if let Some(e) = cur {
        let mut t = trade(1.0, e, x.c[e], "close", to, x.c[to], "open");
        t.open = true;
        out.push(t);
    }
    from_start(out, from)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = |k: &str| g(k).round().max(1.0) as usize;
    let pct = g("open_pct");
    Some(match id {
        INSIDE => intraday(x, from, to, false, pct, |i| {
            // AFL: InsideDay[i-1] AND Gap_Down[i] AND NR4_Cnd[i] (yesterday's range below each of the 3 days before it)
            i >= 4 && x.h[i - 1] < x.h[i - 2] && x.l[i - 1] > x.l[i - 2] && gap_down(x, i)
                && (2..=4).all(|k| range(x, i - 1) < range(x, i - k))
        }),
        OUTSIDE => intraday(x, from, to, true, pct, |i| {
            // AFL: OutsideDay[i-1] AND Gap_Up[i] AND WS7_Cnd[i] != 1 (yesterday's range wider than each of the 6 days before it)
            let ws7 = i >= 7 && (2..=7).all(|k| range(x, i - 1) > range(x, i - k));
            i >= 2 && x.h[i - 1] > x.h[i - 2] && x.l[i - 1] < x.l[i - 2] && gap_up(x, i) && !ws7
        }),
        TURTLE | TURTLE_GAPUP => {
            let (day, len, up) = (g("weekday").round() as i64, n("low_n"), id == TURTLE_GAPUP);
            // AFL: C[i-1] <= Ref(LLV(C, N), -1)[i], i.e. yesterday's close is the lowest of the N closes ending yesterday
            intraday(x, from, to, false, pct, |i| {
                dow(x, i) == day && x.c[i - 1] <= lowest_close(x, i - 1, len) && if up { gap_up(x, i) } else { gap_down(x, i) }
            })
        }
        GAP => {
            let (kind, long, day) = (g("gap").round() as i64, g("side").round() as i64 != 2, g("weekday").round() as i64);
            intraday(x, from, to, long, pct, |i| {
                dow(x, i) == day
                    && match kind {
                        1 => gap_up(x, i),
                        2 => gap_down(x, i),
                        3 => lap_up(x, i),
                        4 => lap_down(x, i),
                        _ => false,
                    }
            })
        }
        MAMM => {
            let (entry_day, exit_day, len) = (g("entry_day").round() as i64, g("exit_day").round() as i64, n("low_n"));
            let mut out = Vec::new();
            let mut cur: Option<(usize, f64)> = None;
            for i in 1..=to {
                if cur.is_none() {
                    let pp = (x.h[i - 1] + x.l[i - 1] + x.c[i - 1]) / 3.0;
                    let (r1, s1) = (2.0 * pp - x.l[i - 1], 2.0 * pp - x.h[i - 1]);
                    // AFL: Close[i] < LLV(C, N)[i-1] AND H[i] < R1[i] AND C[i] < S1[i]
                    if dow(x, i) == entry_day && x.c[i] < lowest_close(x, i - 1, len) && x.h[i] < r1 && x.c[i] < s1 {
                        cur = Some((i, x.c[i]));
                    }
                }
                if let Some((e, px)) = cur {
                    if dow(x, i) == exit_day {
                        out.push(trade(-1.0, e, px, "close", i, x.c[i], "Exit weekday close"));
                        cur = None;
                    }
                }
            }
            if let Some((e, px)) = cur {
                // Still short on the last bar of the period: closes there (End of period), unless it was entered on that bar.
                let mut t = trade(-1.0, e, px, "close", to, x.c[to], "End of period");
                if e == to {
                    t.open = true;
                    t.reason = "open".into();
                }
                out.push(t);
            }
            from_start(out, from)
        }
        BLOOD => {
            let (ver, day, len) = (g("version").round() as i64, g("entry_day").round() as i64, n("low_n"));
            // AFL: EntryDay AND C < Ref(LLV(C, N), -1) AND red today, yesterday, day before [AND (V3) C < L[i-1]]
            close_next(x, from, to, 1.0, 2, |i| {
                dow(x, i) == day && x.c[i] < lowest_close(x, i - 1, len) && red(x, i) && red(x, i - 1) && red(x, i - 2)
                    && (ver != 3 || x.c[i] < x.l[i - 1])
            })
        }
        THURSDAY => {
            let day = g("entry_day").round() as i64;
            close_next(x, from, to, 1.0, 1, |i| {
                let (r1, s1) = pivots(x, i);
                dow(x, i) == day && x.o[i] > x.c[i - 1] && green(x, i) && x.c[i] > r1 && x.l[i] > s1
            })
        }
        THURSDAY_MOD => thursday_modified(x, from, to, g("entry_day").round() as i64),
        MANIC => {
            let (entry_day, exit_day) = (g("entry_day").round() as i64, g("exit_day").round() as i64);
            let mut out = Vec::new();
            let mut cur: Option<usize> = None;
            for i in 1..=to {
                // AFL: EntryDay AND Open > Close AND Open[i-1] > Close[i-1] AND H < R1; the cover test follows on the same bar
                if cur.is_none() && dow(x, i) == entry_day && red(x, i) && red(x, i - 1) && x.h[i] < pivots(x, i).0 {
                    cur = Some(i);
                }
                if let Some(e) = cur {
                    if dow(x, i) == exit_day {
                        out.push(trade(-1.0, e, x.c[e], "close", i, x.c[i], "Exit weekday close"));
                        cur = None;
                    }
                }
            }
            if let Some(e) = cur {
                let mut t = trade(-1.0, e, x.c[e], "close", to, x.c[to], "End of period");
                if e == to {
                    t.open = true;
                    t.reason = "open".into();
                }
                out.push(t);
            }
            from_start(out, from)
        }
        TUMBLES => intraday(x, from.max(3), to, true, pct, |i| dow(x, i) != 1 && red(x, i - 1) && red(x, i - 2) && red(x, i - 3) && gap_up(x, i)),
        TUMBLES_MOD => {
            let (day, len) = (g("weekday").round() as i64, n("low_n"));
            intraday(x, from.max(3), to, true, pct, |i| {
                dow(x, i) == day && x.c[i - 1] <= lowest_close(x, i - 1, len) && red(x, i - 1) && red(x, i - 2) && red(x, i - 3)
            })
        }
        TWO_TUMBLE => {
            let day = g("weekday").round() as i64;
            intraday(x, from.max(2), to, false, pct, |i| dow(x, i) == day && red(x, i - 1) && red(x, i - 2))
        }
        STEPS => {
            let (day, len) = (g("weekday").round() as i64, n("high_n"));
            let mut out = Vec::new();
            for i in from.max(3)..=to {
                // AFL: C[i-1] > Ref(HHV(C, N), -1)[i-1], three green candles, gap up; Short at Open + % when H reaches it
                let level = crate::books::ab::addmul(x.o[i], pct / 100.0);
                if dow(x, i) == day && x.c[i - 1] > highest_close(x, i - 2, len) && green(x, i - 1) && green(x, i - 2) && green(x, i - 3)
                    && gap_up(x, i) && x.h[i] >= level
                {
                    out.push(trade(-1.0, i, crate::books::ab::ab_fill(level, x.l[i], x.h[i]), "limit", i, x.c[i], "Same-day close"));
                }
            }
            out
        }
        OPEN_CLOSE | OPEN_CLOSE_LS => {
            let (open, day) = (g("mode").round() as i64 == 1, g("weekday").round() as i64);
            let dir = if id == OPEN_CLOSE_LS && g("side").round() as i64 == 2 { -1.0 } else { 1.0 };
            if !open {
                close_next(x, from, to, dir, 1, |i| dow(x, i) == day)
            } else {
                // AFL: BuyPrice / ShortPrice = Open + Open x % (plus for a short too), with no test of the bar; AmiBroker keeps it inside the bar
                (from.max(1)..=to)
                    .filter(|&i| dow(x, i) == day)
                    .map(|i| trade(dir, i, crate::books::ab::ab_fill(crate::books::ab::addmul(x.o[i], pct / 100.0), x.l[i], x.h[i]), "limit", i, x.c[i], "Same-day close"))
                    .collect()
            }
        }
        MON_FRI => {
            let day = g("weekday").round() as i64;
            // AFL: Open > Ref(C,-1) AND Ref(O,-1) > Ref(C,-2) AND Ref(C,-1) > Ref(O,-1) AND H >= Open + %
            intraday(x, from.max(2), to, true, pct, |i| dow(x, i) == day && x.o[i] > x.c[i - 1] && x.o[i - 1] > x.c[i - 2] && green(x, i - 1))
        }
        CANDLE => {
            let (want_green, dir) = (g("colour").round() as i64 != 2, if g("side").round() as i64 == 2 { -1.0 } else { 1.0 });
            close_next(x, from, to, dir, 1, |i| if want_green { green(x, i) } else { red(x, i) })
        }
        UUDDD => {
            let mut out = Vec::new();
            for i in from.max(4)..=to {
                if gap_down(x, i) && (1..=4).all(|k| red(x, i - k)) {
                    let px = crate::books::ab::ab_fill(crate::books::ab::addmul(x.o[i], -pct / 100.0), x.l[i], x.h[i]);
                    out.push(trade(-1.0, i, px, "limit", i, x.c[i], "Same-day close"));
                }
            }
            out
        }
        WEEKDAY => weekday_short_long(x, from, to, &g),
        id if id == DUDD[0] => close_next(x, from, to, -1.0, 6, |i| {
            // AFL: WS7 today (R1 = today's range > each of the 6 before), red today, green 1 and 2 bars back, red 3 back
            (1..=6).all(|k| range(x, i) > range(x, i - k)) && red(x, i) && green(x, i - 1) && green(x, i - 2) && red(x, i - 3)
        }),
        id if DUDD.contains(&id) => {
            let v = DUDD.iter().position(|d| *d == id).unwrap() + 1;
            // yesterday's range against the bars before it: NR4 / WS4 / NR7 / NR7 / NR7 / WS7, then candle colours 1-3 bars back
            let (back, narrow) = match v { 2 => (3, true), 3 => (3, false), 7 => (6, false), _ => (6, true) };
            let colours: [bool; 3] = match v { 4 => [true, true, true], 6 => [false, false, true], _ => [true, false, true] }; // true = red
            let mut out = Vec::new();
            for i in from.max(back + 1)..=to {
                let rng = (2..=back + 1).all(|k| if narrow { range(x, i - 1) < range(x, i - k) } else { range(x, i - 1) > range(x, i - k) });
                let cols = colours.iter().enumerate().all(|(j, &r)| if r { red(x, i - 1 - j) } else { green(x, i - 1 - j) });
                if rng && gap_down(x, i) && cols {
                    // AFL: ShortPrice = O - O x %; no test that the low reaches it, and AmiBroker keeps a fill inside the bar
                    let px = crate::books::ab::ab_fill(crate::books::ab::addmul(x.o[i], -pct / 100.0), x.l[i], x.h[i]);
                    out.push(trade(-1.0, i, px, "limit", i, x.c[i], "Same-day close"));
                }
            }
            out
        }
        WEEKNUM | WEEKNUM_MF => weeknum_run(id, x, from, to, &g),
        _ => return None,
    })
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        INSIDE => ("ytkr_inside", "Inside Day", "Kora Reddy"),
        OUTSIDE => ("ytkr_outside", "Outside Day", "Kora Reddy"),
        TURTLE => ("ytkr_turtle", "Turtle Short", "Kora Reddy"),
        TURTLE_GAPUP => ("ytkr_turtle_gapup", "Turtle Short on GapUp", "Kora Reddy"),
        MAMM => ("ytkr_mamm", "Momentum Aided Manic Monday", "Kora Reddy"),
        GAP => ("ytkr_gap", "Gap Open to Close", "Kora Reddy"),
        BLOOD => ("ytkr_blood", "Buy when there is blood on street", "Kora Reddy"),
        THURSDAY => ("ytkr_thursday", "Buying Thursday", "Kora Reddy"),
        THURSDAY_MOD => ("ytkr_thursday_mod", "Buying Thursday (Modified)", "Kora Reddy"),
        MANIC => ("ytkr_manic", "Manic Monday", "Kora Reddy"),
        TUMBLES => ("ytkr_tumbles", "Three tumbles and a jump", "Kora Reddy"),
        TUMBLES_MOD => ("ytkr_tumbles_mod", "Modified Three tumbles and a jump", "Kora Reddy"),
        TWO_TUMBLE => ("ytkr_two_tumble", "Two Tumble & Another Tumble", "Kora Reddy"),
        STEPS => ("ytkr_steps", "Three steps and a stumble", "Kora Reddy"),
        "ytkr_dudd1" => ("ytkr_dudd1", "DUDD Version 1", "Kora Reddy"),
        "ytkr_dudd2" => ("ytkr_dudd2", "DUDD Version 2", "Kora Reddy"),
        "ytkr_dudd3" => ("ytkr_dudd3", "DUDD Version 3", "Kora Reddy"),
        "ytkr_dudd4" => ("ytkr_dudd4", "DUDD Version 4", "Kora Reddy"),
        "ytkr_dudd5" => ("ytkr_dudd5", "DUDD Version 5", "Kora Reddy"),
        "ytkr_dudd6" => ("ytkr_dudd6", "DUDD Version 6", "Kora Reddy"),
        "ytkr_dudd7" => ("ytkr_dudd7", "DUDD Version 7", "Kora Reddy"),
        OPEN_CLOSE => ("ytkr_open_close", "Buy at Open or Close", "Kora Reddy"),
        OPEN_CLOSE_LS => ("ytkr_open_close_ls", "BuyShort at Open or Close", "Kora Reddy"),
        MON_FRI => ("ytkr_mon_fri", "Buying Mondays & Fridays", "Kora Reddy"),
        CANDLE => ("ytkr_candle", "Entry Based on Candle Color", "Kora Reddy"),
        UUDDD => ("ytkr_uuddd", "UUDDD", "Kora Reddy"),
        WEEKNUM => ("ytkr_weeknum", "Weeknum Given Month", "Kora Reddy"),
        WEEKNUM_MF => ("ytkr_weeknum_mf", "Weeknum Given Month (Mon-Fri copy)", "Kora Reddy"),
        WEEKDAY => ("ytkr_weekday", "Weekday Short Long", "Kora Reddy"),
        _ => return None,
    })
}

// ---- kora reddy all day (added 20-11-2026)
// kora reddy all day.afl, "Weekday Short Long" (coded by Yashesh Sorathia, 08-09-2022): enter on one weekday and exit
// on another, long and/or short, daily bars. Coded from the AFL's main loop as written (short block, then long block,
// each bar from bar 1 of the data), with the AFL's own Param defaults.

fn weekday_def() -> StrategyDef {
    def(WEEKDAY, "Weekday Short Long", "Long/Short", &[
        "F|Long (on by default): today = Long Entry weekday (default Tuesday) and today's High ≠ Close (no upper-circuit bar)",
        "F|Short (off by default): today = Short Entry weekday (default Tuesday) and today's Low ≠ Close (no lower-circuit bar)",
        "F|Filter (default off): SMA, Close ≥ MA(Close, 10), for longs and shorts alike; or ST, Close above (long) / below (short) the SuperTrend (ATR 5, factor 1, (H+L)/2) of the monthly, weekly or daily bars",
        "E|Enter at today's Close (default) or today's Open, one position per side",
        "X|Exit on the Exit weekday (default Tuesday, so the entry day itself), at the Close (default) or Open",
        "X|Or after MaxBars − 1 bars: MaxBars = 6 if Entry = Exit weekday, Exit − Entry + 1 if Exit is later, else 5 − Entry + Exit + 1",
        "X|Or Hard SL % / Profit booking % (default 0, off) at the level, or at Open ∓ Selling buffer % (default 0.25) when the Open is already past it",
        "X|Or the ST filter turns against the position",
        "X|No exit on a long bar with Low = Open = Close, or a short bar with Open = High = Close: the exit waits for the next bar without one, at its Close / Open",
    ], "Weekday Short Long, kora reddy all day.afl (Kora Reddy folder), coded as the AFL is written; it is not in the team's checklist and no team workbook names it (the Rules & summary sheet describes \"buy at Close on Monday and sell on Tuesday at Close, for all days (L&S)\"). Quirks kept: the exit test runs on the entry bar after the entry, so with the default Entry = Exit weekday (Tuesday) every trade is entered and exited on the same Tuesday at the close (a zero-return trade before costs); the 6-bar MaxBars for equal weekdays is never reached; with an Open exit and a Close entry the same-bar exit is priced at the Open. The profit / stop levels are (1 ± %) x entry price and can be hit on the entry bar. On a gap past the level the exit is Open − buffer for a long and Open + buffer for a short for both the stop and the profit target, as written (a gap through a long's profit target still sells below the open). The SMA filter tests Close ≥ MA for shorts too (the AFL uses SMABuysig for both); SMA period 0 gives no entries. The ST is expanded to days with expandFirst, so each day sees the ST of its whole week or month (look-ahead in the AFL as written). Not modelled, at their AFL defaults: dead-stock backtest NO (no forced exit on the last bar), turnover filter off, penny price 0. The AFL resets its ShortEntry variable to 0 after it has been used (AFL names ignore case), harmless.")
}

fn weekday_params() -> Vec<ParamDef> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    vec![
        p("long_on", "Long condition (0 no, 1 yes)", 1.0, 0.0, 1.0, 1.0),
        p("long_entry_day", "Long entry weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 2.0, 1.0, 5.0, 1.0),
        p("long_entry_at", "Long entry at (0 close, 1 open)", 0.0, 0.0, 1.0, 1.0),
        p("long_exit_day", "Long exit weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 2.0, 1.0, 5.0, 1.0),
        p("long_exit_at", "Long exit at (0 close, 1 open)", 0.0, 0.0, 1.0, 1.0),
        p("short_on", "Short condition (0 no, 1 yes)", 0.0, 0.0, 1.0, 1.0),
        p("short_entry_day", "Short entry weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 2.0, 1.0, 5.0, 1.0),
        p("short_entry_at", "Short entry at (0 close, 1 open)", 0.0, 0.0, 1.0, 1.0),
        p("short_exit_day", "Short exit weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 2.0, 1.0, 5.0, 1.0),
        p("short_exit_at", "Short exit at (0 close, 1 open)", 0.0, 0.0, 1.0, 1.0),
        p("pb_pct", "Profit booking % (0 off)", 0.0, 0.0, 50.0, 1.0),
        p("hsl_pct", "Hard SL % (0 off)", 0.0, 0.0, 50.0, 1.0),
        p("buffer_pct", "Selling buffer % (exit on a gap past the profit / stop level)", 0.25, 0.0, 10.0, 0.05),
        p("filter", "Filter (0 off, 1 SMA, 2 ST)", 0.0, 0.0, 2.0, 1.0),
        p("sma_period", "SMA period", 10.0, 0.0, 50.0, 5.0),
        p("st_tf", "ST filter timeframe (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0),
        p("st_period", "ST filter period (ATR)", 5.0, 5.0, 25.0, 5.0),
        p("st_factor", "ST filter factor", 1.0, 1.0, 5.0, 1.0),
    ]
}

/// AFL MaxBars / MaxBars1 from the entry and exit weekdays.
fn weekday_max_bars(entry: i64, exit: i64) -> i64 {
    if entry == exit {
        6
    } else if entry < exit {
        exit - entry + 1
    } else {
        5 - entry + exit + 1
    }
}

/// The AFL's ST filter line: TimeFrameSet(tf), ATR(period), AvgVal = (H+L)/2, LineUp / LineDn = AvgVal ± ATR x factor,
/// the loop from bar `period` (C > LineUp[i-1] up, C < LineDn[i-1] down, else unchanged; LineDn ratchets up while up,
/// LineUp down while down), STPlot = up ? LineDn : LineUp, then TimeFrameExpand(.., expandFirst): every day of a
/// period gets that period's value. NaN = Null (no ratchet against a Null previous line; comparisons with it false).
fn weekday_st(x: &Ctx, tf: i64, period: usize, factor: f64) -> Vec<f64> {
    use crate::books::ab;
    let b = crate::books::ytsw::compress(x, match tf { 1 => 'M', 2 => 'W', _ => 'D' });
    let m = b.c.len();
    let atr = ab::atr(&b.h, &b.l, &b.c, period);
    let avg: Vec<f64> = (0..m).map(|g| ab::f32r((b.h[g] + b.l[g]) / 2.0)).collect();
    let mut up: Vec<f64> = (0..m).map(|g| atr[g].map_or(f64::NAN, |a| ab::add_mul32(avg[g], a, factor))).collect();
    let mut dn: Vec<f64> = (0..m).map(|g| atr[g].map_or(f64::NAN, |a| ab::add_mul32(avg[g], a, -factor))).collect();
    let mut is_up = vec![true; m];
    for i in period.max(1)..m {
        is_up[i] = if b.c[i] > up[i - 1] { true } else if b.c[i] < dn[i - 1] { false } else { is_up[i - 1] };
        if is_up[i] {
            if !dn[i].is_nan() && !dn[i - 1].is_nan() {
                dn[i] = dn[i].max(dn[i - 1]);
            }
        } else if !up[i].is_nan() && !up[i - 1].is_nan() {
            up[i] = up[i].min(up[i - 1]);
        }
    }
    let st: Vec<f64> = (0..m).map(|g| if is_up[g] { dn[g] } else { up[g] }).collect();
    (0..x.c.len()).map(|i| st[b.group[i]]).collect()
}

struct WeekdaySide {
    long: bool,
    exit_day: i64,
    entry_open: bool,
    exit_open: bool,
    max_bars: i64,
}

/// One side of the AFL loop (`Position`, `SkipSell`, `Bars_Since`, `PB`, `HSL` for the long; their `...1` twins for the
/// short). `entry_ok` is the cash filter (weekday, circuit, filter); `filter_exit` is ST_SELL / ST_Cover.
fn weekday_side(x: &Ctx, to: usize, s: &WeekdaySide, pb_pct: f64, hsl_pct: f64, buf: f64, entry_ok: &[bool], filter_exit: &[bool]) -> Vec<Trade> {
    use crate::books::ab::{addmul, mul32};
    let dir = if s.long { 1.0 } else { -1.0 };
    let kind = if s.entry_open { "open" } else { "close" };
    let mut out = Vec::new();
    let mut pos: Option<(usize, f64)> = None;
    let (mut skip, mut bars) = (false, 0i64);
    let (mut pb, mut hsl) = (f64::NAN, f64::NAN);
    for i in 1..=to {
        let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        // long: OLC = L == C AND L == O; short: OHC = O == H AND H == C
        let circuit = if s.long { l == c && l == o } else { o == h && h == c };
        let exit_px = if s.exit_open { o } else { c };
        bars += 1;
        if pos.is_none() && entry_ok[i] {
            bars = 0;
            let px = if s.entry_open { o } else { c };
            // PB = (1 + Profit_Pct * 0.01) * BuyPrice; short PB1 = (1 - Profit_Pct * 0.01) * ShortPrice (HSL the other way)
            if pb_pct > 0.0 {
                pb = mul32(1.0 + dir * pb_pct * 0.01, px);
            }
            if hsl_pct > 0.0 {
                hsl = mul32(1.0 - dir * hsl_pct * 0.01, px);
            }
            pos = Some((i, px));
        }
        if let Some((e, epx)) = pos {
            if !skip {
                // long: (HARDSL_Pct != 0 AND L <= HSL) OR (Profit_Pct != 0 AND H >= PB); short: H >= HSL1 OR L <= PB1
                let (hit_hsl, hit_pb) = if s.long { (hsl_pct != 0.0 && l <= hsl, pb_pct != 0.0 && h >= pb) } else { (h >= hsl, l <= pb) };
                let sig = dow(x, i) == s.exit_day;
                if hit_hsl || hit_pb || sig || filter_exit[i] || bars >= s.max_bars - 1 {
                    if !circuit {
                        let (mut px, mut why) = (exit_px, if sig { "Exit weekday" } else if filter_exit[i] { "ST filter" } else { "Max bars" });
                        if s.long {
                            if l <= hsl {
                                (px, why) = (if o < hsl { addmul(o, -buf / 100.0) } else { hsl }, "Hard SL");
                            } else if h >= pb {
                                // as written: a gap above the profit target sells at Open - buffer
                                (px, why) = (if o > pb { addmul(o, -buf / 100.0) } else { pb }, "Profit booking");
                            }
                        } else if h >= hsl {
                            (px, why) = (if o > hsl { addmul(o, buf / 100.0) } else { hsl }, "Hard SL");
                        } else if l <= pb {
                            (px, why) = (if o < pb { addmul(o, buf / 100.0) } else { pb }, "Profit booking");
                        }
                        out.push(trade(dir, e, epx, kind, i, px, why));
                        pos = None;
                        (pb, hsl) = (f64::NAN, f64::NAN);
                    } else {
                        skip = true;
                    }
                }
            }
        }
        if let Some((e, epx)) = pos {
            if skip && !circuit {
                out.push(trade(dir, e, epx, kind, i, exit_px, "Exit after circuit bar"));
                pos = None;
                skip = false;
                (pb, hsl) = (f64::NAN, f64::NAN);
            }
        }
    }
    if let Some((e, epx)) = pos {
        let mut t = trade(dir, e, epx, kind, to, x.c[to], "End of period");
        if e == to {
            t.open = true;
            t.reason = "open".into();
        }
        out.push(t);
    }
    out
}

fn weekday_short_long(x: &Ctx, from: usize, to: usize, g: &impl Fn(&str) -> f64) -> Vec<Trade> {
    let n = x.c.len();
    if n == 0 || to >= n {
        return Vec::new();
    }
    let gi = |k: &str| g(k).round() as i64;
    let ftype = gi("filter");
    let sma_ok: Vec<bool> = if ftype == 1 && gi("sma_period") >= 1 {
        let m = crate::books::ab::ma(&x.c.iter().map(|&v| Some(v)).collect::<Vec<_>>(), gi("sma_period") as usize);
        (0..n).map(|i| m[i].map_or(false, |m| x.c[i] >= m)).collect()
    } else {
        vec![false; n]
    };
    let (st_buy, st_short): (Vec<bool>, Vec<bool>) = if ftype == 2 {
        let line = weekday_st(x, gi("st_tf"), gi("st_period").max(1) as usize, g("st_factor"));
        ((0..n).map(|i| x.c[i] > line[i]).collect(), (0..n).map(|i| x.c[i] < line[i]).collect())
    } else {
        (vec![false; n], vec![false; n])
    };
    let none = vec![false; n];
    let flt = |i: usize, st: &[bool]| match ftype { 2 => st[i], 1 => sma_ok[i], _ => true };
    let (pb_pct, hsl_pct, buf) = (g("pb_pct"), g("hsl_pct"), g("buffer_pct"));
    let mut out = Vec::new();
    // the AFL's short block runs before its long block on each bar; the two sides do not interact
    if gi("short_on") == 1 {
        let day = gi("short_entry_day");
        let ok: Vec<bool> = (0..n).map(|i| dow(x, i) == day && x.l[i] != x.c[i] && flt(i, &st_short) && x.c[i] > 0.0).collect();
        let s = WeekdaySide { long: false, exit_day: gi("short_exit_day"), entry_open: gi("short_entry_at") == 1, exit_open: gi("short_exit_at") == 1,
            max_bars: weekday_max_bars(day, gi("short_exit_day")) };
        out.extend(weekday_side(x, to, &s, pb_pct, hsl_pct, buf, &ok, if ftype == 2 { &st_buy } else { &none }));
    }
    if gi("long_on") == 1 {
        let day = gi("long_entry_day");
        let ok: Vec<bool> = (0..n).map(|i| dow(x, i) == day && x.h[i] != x.c[i] && flt(i, &st_buy) && x.c[i] > 0.0).collect();
        let s = WeekdaySide { long: true, exit_day: gi("long_exit_day"), entry_open: gi("long_entry_at") == 1, exit_open: gi("long_exit_at") == 1,
            max_bars: weekday_max_bars(day, gi("long_exit_day")) };
        out.extend(weekday_side(x, to, &s, pb_pct, hsl_pct, buf, &ok, if ftype == 2 { &st_short } else { &none }));
    }
    out.sort_by_key(|t| (t.legs[0].i, t.exit_i));
    from_start(out, from)
}

// ---- kora reddy weeknum given month (added 20-11-2026)
// jay-local/Kora Reddy/kora_reddy_Weeknum_Given_Month.afl (603 lines) and its near-copy in "Mon-Fri Normal Weekly &
// Monthly" (621 lines), whose entry-day test has one more holiday clause and which adds an SMA filter.
pub const WEEKNUM: &str = "ytkr_weeknum_given_month";
pub const WEEKNUM_MF: &str = "ytkr_weeknum_given_month_mf";
/// The two weeknum ids have not been compared with any team result (no trade sheet or optimisation exists).
pub const UNCHECKED: [&str; 2] = [WEEKNUM, WEEKNUM_MF];

const WEEKNUM_RULES: &[&str] = &[
    "F|Today = Entry weekday (default Tuesday)",
    "F|Holiday setting 0: also the bar before a missed Entry weekday (the AFL reads the next bar's weekday)",
    "F|Optional: week number of the month (default off, week 1) and month of the year (default 0, every month)",
    "F|Long: no buy on a bar closing at its High; short: no short on a bar closing at its Low",
    "F|Optional ST filter (default off): Close above (or below) the monthly / weekly / daily Supertrend",
    "E|Side 1 (default): Buy at the Close (or Open); side 2: Short at the Close (or Open)",
    "X|Exit at the Close (or Open) on the Exit weekday (default Tuesday), or the bar before when it is missed",
    "X|Time exit after MaxBars − 1 bars (MaxBars 6 when Entry and Exit weekday are the same)",
    "X|Optional profit booking % and hard SL % (default 0, off): at the level, or the open ∓ buffer % on a gap",
    "X|An exit on a circuit bar (long O = L = C, short O = H = C) waits for the next bar that is not one",
];
const WEEKNUM_MF_RULES: &[&str] = &[
    "F|Today = Entry weekday (default Tuesday)",
    "F|Holiday setting 0: also the bar before a missed Entry weekday, including a week whose first days are missed",
    "F|Optional: week number of the month (default off, week 1) and month of the year (default 0, every month)",
    "F|Long: no buy on a bar closing at its High; short: no short on a bar closing at its Low",
    "F|Optional ST filter (default off) and SMA filter (default off, Close above the 15-day SMA)",
    "E|Side 1 (default): Buy at the Close (or Open); side 2: Short at the Close (or Open)",
    "X|Exit at the Close (or Open) on the Exit weekday (default Tuesday), or the bar before when it is missed",
    "X|Time exit after MaxBars − 1 bars (MaxBars 6 when Entry and Exit weekday are the same)",
    "X|Optional profit booking % and hard SL % (default 0, off): at the level, or the open ∓ buffer % on a gap",
    "X|An exit on a circuit bar (long O = L = C, short O = H = C) waits for the next bar that is not one",
];
const WEEKNUM_NOTES: &str = "kora_reddy_Weeknum_Given_Month.afl (Kora Reddy folder; coded by the team 08-09-2022), coded as the AFL is written with its own defaults: long only, buy Tuesday's close, sell the next Tuesday's close. The week number of the month is the AFL's loop: week 1 starts at the first new week (weekday lower than the bar before) on or after the month's first bar; the days of a new month before it keep the previous month's count. As written: the holiday tests read the NEXT bar's weekday (Ref(DayOfWeek(), 1), a look-ahead; nothing on the last bar); the exit-day test Ref(DayOfWeek(), 1) == LongEntryDay_ compares a weekday with a true/false value, so it holds only when the entry-day flag is set and the next bar is a Monday; the short side uses the long side's 'if Entry day Holiday' toggle (its own is never read); a long gapping through its profit level sells at open MINUS the buffer and a short gapping through its profit level covers at open PLUS the buffer; the ST filter's weekly / monthly line is expanded with expandFirst and no Ref shift, so a day sees its own period's final line (a look-ahead); the filter list's SMA case is unreachable (the list is OFF|ST). Declared adaptations: one side per run (side setting), because the AFL's long and short toggles together would hold both on one symbol, which AmiBroker's backtester does not; the turnover filter (needs OpenInt) and the dead-stock toggle (default NO: no buy on bar 0, no forced exit) are not offered. Entry and exit weekday, entry / exit price and the holiday toggle are shared by the two sides (the AFL's separate short settings have the same defaults). No team result exists for this AFL (the Backtest.xlsx beside the copy is empty), so it is not checked against the team.";
const WEEKNUM_MF_NOTES: &str = "kora_reddy_Weeknum_Given_Month.afl in 'Mon-Fri Normal Weekly & Monthly' (621 lines), a near-copy of the Kora Reddy folder's AFL coded as Weeknum Given Month, with the same quirks and adaptations (see there). Its trading rules differ in two places: the entry-day test has a fourth holiday clause (the next bar is in the next week and both today and the next bar are after the Entry weekday), on the long and the short side; and an SMA filter (default off, period 15, trade above the SMA; Close against MA(C, n), no trade before the average exists) applies to buys and shorts. With the defaults (holiday setting 1, SMA off) it gives the same trades as the root AFL. No team result exists (its Backtest.xlsx is empty).";

fn weeknum_catalogue() -> Vec<StrategyDef> {
    vec![
        def(WEEKNUM, "Weeknum Given Month", "Long/Short", WEEKNUM_RULES, WEEKNUM_NOTES),
        def(WEEKNUM_MF, "Weeknum Given Month (Mon-Fri copy)", "Long/Short", WEEKNUM_MF_RULES, WEEKNUM_MF_NOTES),
    ]
}

fn weeknum_params(id: &str) -> Vec<ParamDef> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![
        p("side", "Side (1 long, the AFL default; 2 short)", 1.0, 1.0, 2.0, 1.0),
        p("entry_day", "Entry weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 2.0, 1.0, 5.0, 1.0),
        p("entry_at", "Entry at (0 close, 1 open)", 0.0, 0.0, 1.0, 1.0),
        p("exit_day", "Exit weekday (1 Monday, 2 Tuesday, 3 Wednesday, 4 Thursday, 5 Friday)", 2.0, 1.0, 5.0, 1.0),
        p("exit_at", "Exit at (0 close, 1 open)", 0.0, 0.0, 1.0, 1.0),
        p("entry_skip", "If the Entry weekday is a holiday (0 enter on the bar before, 1 skip the entry)", 1.0, 0.0, 1.0, 1.0),
        p("pb_pct", "Profit booking % (0 off)", 0.0, 0.0, 50.0, 1.0),
        p("hsl_pct", "Hard SL % (0 off)", 0.0, 0.0, 50.0, 1.0),
        p("buffer_pct", "Selling buffer % (gap through a level)", 0.25, 0.0, 10.0, 0.05),
        p("week_on", "Week number of month filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("week_num", "Week number", 1.0, 0.0, 7.0, 1.0),
        p("month", "Month to trade in (0 for all)", 0.0, 0.0, 12.0, 1.0),
        p("filter", "Filter (0 off, 1 ST)", 0.0, 0.0, 1.0, 1.0),
        p("st_tf", "ST filter timeframe (0 monthly, 1 weekly, 2 daily)", 0.0, 0.0, 2.0, 1.0),
        p("st_period", "ST filter period", 5.0, 5.0, 25.0, 5.0),
        p("st_factor", "ST filter factor", 1.0, 1.0, 5.0, 1.0),
        p("st_type", "ST filter type (0 bearish: Close below ST, 1 bullish: Close above ST)", 1.0, 0.0, 1.0, 1.0),
        p("penny", "Penny stock price (enter only above it)", 0.0, 0.0, 100.0, 1.0),
    ];
    if id == WEEKNUM_MF {
        v.extend([
            p("sma_on", "SMA filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("sma_period", "SMA period", 15.0, 1.0, 100.0, 1.0),
            p("sma_type", "Trade when (0 Close above SMA, 1 Close below SMA)", 0.0, 0.0, 1.0, 1.0),
        ]);
    }
    v
}

/// The AFL's ST filter line on daily bars: ATR(period) and (H + L) / 2 on the timeframe's bars, the AFL's isUp loop
/// from bar `period`, then TimeFrameExpand(expandFirst) with no Ref shift (each day reads its own period's line).
/// NaN where the line is Null.
fn weeknum_st(x: &Ctx, tf: i64, period: usize, factor: f64) -> Vec<f64> {
    let b = crate::books::ytsw::compress(x, match tf { 1 => 'W', 2 => 'D', _ => 'M' });
    let m = b.c.len();
    let atr = crate::books::ab::atr(&b.h, &b.l, &b.c, period);
    let mid = |g: usize| (b.h[g] + b.l[g]) / 2.0;
    let mut up: Vec<f64> = (0..m).map(|g| atr[g].map_or(f64::NAN, |a| mid(g) + a * factor)).collect();
    let mut dn: Vec<f64> = (0..m).map(|g| atr[g].map_or(f64::NAN, |a| mid(g) - a * factor)).collect();
    let mut is_up = vec![true; m];
    for g in period.max(1)..m {
        // a comparison with Null is false; Max / Min with a Null element keep the other value
        is_up[g] = if b.c[g] > up[g - 1] { true } else if b.c[g] < dn[g - 1] { false } else { is_up[g - 1] };
        if is_up[g] {
            dn[g] = dn[g].max(dn[g - 1]);
        } else {
            up[g] = up[g].min(up[g - 1]);
        }
    }
    (0..x.c.len()).map(|i| { let g = b.group[i]; if is_up[g] { dn[g] } else { up[g] } }).collect()
}

/// The AFL's calenderWeekNumberOfMonth loop.
fn weeknum_of_month(x: &Ctx) -> Vec<i64> {
    let n = x.c.len();
    let month = |i: usize| x.r[i].date.get(5..7).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let mut wk = vec![1i64; n];
    let mut month_change = false;
    for i in 0..n {
        if i == 0 || month(i) != month(i - 1) {
            month_change = true; // firstCandleOfMonth (Ref(Month(), -1) is Null on bar 0)
        }
        if i == 0 {
            continue;
        }
        let new_week = dow(x, i) < dow(x, i - 1);
        let first_monday = month_change && new_week;
        if first_monday {
            month_change = false;
        }
        wk[i] = if new_week && first_monday { 1 } else if new_week { wk[i - 1] + 1 } else { wk[i - 1] };
    }
    wk
}

fn weeknum_run(id: &str, x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    use crate::books::ab::{addmul, mul32};
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Vec::new();
    }
    let gi = |k: &str| g(k).round() as i64;
    let mf = id == WEEKNUM_MF;
    let long = gi("side") != 2;
    let (entry_day, exit_day, skip) = (gi("entry_day"), gi("exit_day"), gi("entry_skip") == 1);
    let (entry_open, exit_open) = (gi("entry_at") == 1, gi("exit_at") == 1);
    let (pb_pct, hsl_pct, buf) = (g("pb_pct"), g("hsl_pct"), g("buffer_pct"));
    let max_bars = if entry_day == exit_day { 6 } else if entry_day < exit_day { exit_day - entry_day + 1 } else { 5 - entry_day + exit_day + 1 };
    let d: Vec<i64> = (0..n).map(|i| dow(x, i)).collect();
    // _DayOfWeek1 = Ref(DayOfWeek(), 1): Null on the last bar of the data, where every test with it is false
    let d1 = |i: usize| if i + 1 < n { Some(d[i + 1]) } else { None };
    let entry_flag: Vec<bool> = (0..n)
        .map(|i| {
            let c1 = d[i] < entry_day;
            d[i] == entry_day
                || d1(i).is_some_and(|t| (c1 && t > entry_day) || (c1 && t < d[i]) || (mf && t > entry_day && d[i] > entry_day && t < d[i]))
        })
        .collect();
    let exit_flag: Vec<bool> = (0..n)
        .map(|i| {
            let c2 = d[i] < exit_day;
            // the AFL's last clause compares the next weekday with the entry-day flag (0 / 1)
            d[i] == exit_day || d1(i).is_some_and(|t| (c2 && t > exit_day) || (c2 && t < d[i]) || t == entry_flag[i] as i64)
        })
        .collect();
    let wk = weeknum_of_month(x);
    let (week_on, week_num, month) = (gi("week_on") == 1, gi("week_num"), gi("month"));
    let st: Vec<f64> = if gi("filter") == 1 { weeknum_st(x, gi("st_tf"), g("st_period").round().max(1.0) as usize, g("st_factor")) } else { Vec::new() };
    let bullish = gi("st_type") == 1;
    let sma: Vec<Option<f64>> = if mf && gi("sma_on") == 1 {
        let cc: Vec<Option<f64>> = x.c.iter().map(|&v| Some(v)).collect();
        crate::books::ab::ma(&cc, g("sma_period").round().max(1.0) as usize)
    } else {
        Vec::new()
    };
    let below_sma = gi("sma_type") == 1;
    let penny = g("penny");
    let cash = |i: usize| {
        let sig = if skip { d[i] == entry_day } else { entry_flag[i] };
        let mon = x.r[i].date.get(5..7).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
        sig && (!week_on || wk[i] == week_num)
            && (month <= 0 || month == mon)
            && if long { x.h[i] != x.c[i] } else { x.l[i] != x.c[i] }
            && (st.is_empty() || if bullish { x.c[i] > st[i] } else { x.c[i] < st[i] })
            && (sma.is_empty() || sma[i].is_some_and(|m| if below_sma { x.c[i] < m } else { x.c[i] > m }))
            && x.c[i] > penny
            && i != 0
    };
    // OLC (long) / OHC (short): an exit on such a bar is skipped and taken on the next bar that is not one
    let circuit = |i: usize| if long { x.l[i] == x.c[i] && x.l[i] == x.o[i] } else { x.o[i] == x.h[i] && x.h[i] == x.c[i] };
    let at = |open: bool, i: usize| if open { x.o[i] } else { x.c[i] };
    let dir = if long { 1.0 } else { -1.0 };
    let kind = if entry_open { "open" } else { "close" };
    let mut out = Vec::new();
    let mut pos: Option<(usize, f64)> = None;
    let (mut skip_exit, mut bars_since) = (false, 0i64);
    let (mut pb, mut hsl) = (f64::NAN, f64::NAN);
    for i in 1..=to {
        bars_since += 1;
        match pos {
            None if cash(i) => {
                bars_since = 0;
                let px = at(entry_open, i);
                if pb_pct > 0.0 {
                    pb = if long { mul32(1.0 + pb_pct * 0.01, px) } else { mul32(1.0 - pb_pct * 0.01, px) };
                }
                if hsl_pct > 0.0 {
                    hsl = if long { mul32(1.0 - hsl_pct * 0.01, px) } else { mul32(1.0 + hsl_pct * 0.01, px) };
                }
                pos = Some((i, px));
            }
            Some((e, epx)) if !skip_exit => {
                // long: the AFL guards the levels with their % settings; short: unset levels are Null (false)
                let (hit_sl, hit_pb) = if long {
                    (hsl_pct != 0.0 && x.l[i] <= hsl, pb_pct != 0.0 && x.h[i] >= pb)
                } else {
                    (x.h[i] >= hsl, x.l[i] <= pb)
                };
                if hit_sl || hit_pb || exit_flag[i] || bars_since >= max_bars - 1 {
                    if !circuit(i) {
                        let o = x.o[i];
                        let (px, why) = if long && x.l[i] <= hsl {
                            (if o < hsl { addmul(o, -buf / 100.0) } else { hsl }, "Hard SL")
                        } else if long && x.h[i] >= pb {
                            (if o > pb { addmul(o, -buf / 100.0) } else { pb }, "Profit booking")
                        } else if !long && x.h[i] >= hsl {
                            (if o > hsl { addmul(o, buf / 100.0) } else { hsl }, "Hard SL")
                        } else if !long && x.l[i] <= pb {
                            (if o < pb { addmul(o, buf / 100.0) } else { pb }, "Profit booking")
                        } else if exit_flag[i] {
                            (at(exit_open, i), "Exit weekday")
                        } else {
                            (at(exit_open, i), "Max bars")
                        };
                        out.push(trade(dir, e, epx, kind, i, px, why));
                        pos = None;
                        pb = f64::NAN;
                        hsl = f64::NAN;
                    } else {
                        skip_exit = true;
                    }
                }
            }
            _ => {}
        }
        if skip_exit && !circuit(i) {
            if let Some((e, epx)) = pos {
                out.push(trade(dir, e, epx, kind, i, at(exit_open, i), "After circuit bar"));
            }
            pos = None;
            skip_exit = false;
            pb = f64::NAN;
            hsl = f64::NAN;
        }
    }
    if let Some((e, epx)) = pos {
        let mut t = trade(dir, e, epx, kind, to, x.c[to], "open");
        t.open = true;
        out.push(t);
    }
    from_start(out, from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    /// Consecutive weekdays from Monday 2024-01-01; each bar is (open, high, low, close).
    fn bars(ohlc: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        let mut d = day_number("2024-01-01").unwrap();
        ohlc.iter()
            .map(|&(open, high, low, close)| {
                let z = d;
                d += if (z + 4).rem_euclid(7) == 5 { 3 } else { 1 };
                Row { ticker: "T".into(), date: civil(z), open, high, low, close, volume: None }
            })
            .collect()
    }
    fn civil(z: i64) -> String {
        let z = z + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{:04}-{:02}-{:02}", yoe + era * 400 + i64::from(m <= 2), m, d)
    }
    fn ps(kv: &[(&str, f64)]) -> Params {
        kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }
    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<(usize, f64, usize, f64, f64)> {
        let x = Ctx::new(r, r, &[]);
        run(id, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.dir)).collect()
    }
    const W: (f64, f64, f64, f64) = (100.0, 110.0, 90.0, 100.0); // wide bar, range 20

    #[test]
    fn candle_colour_carries_the_afl_position_into_a_later_start() {
        // all green: the AFL loop buys bar 1, sells bar 2, buys bar 3, ... A run from bar 2 (an exit day) first buys bar 3,
        // as AmiBroker did for Entry Based on Candle Color from 01-01-2015 (no trade on 01-01, first buy 02-01).
        let g = (100.0, 102.0, 99.0, 101.0);
        let r = bars(&[g, g, g, g, g, g]);
        let x = Ctx::new(&r, &r, &[]);
        let t: Vec<usize> = run(CANDLE, &x, 2, 5, &ps(&[])).unwrap().iter().map(|t| t.legs[0].i).collect();
        assert_eq!(t, vec![3, 5]);
        // through the books dispatcher a position held at the end is closed there; an entry on the last bar is left out
        let t = crate::books::run(CANDLE, &x, 2, 4, &ps(&[])).unwrap();
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i, t.open)).collect::<Vec<_>>(), vec![(3, 4, false)]);
        let t = crate::books::run(CANDLE, &x, 1, 4, &ps(&[])).unwrap();
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i, t.open)).collect::<Vec<_>>(), vec![(1, 2, false), (3, 4, false)]);
        let t = crate::books::run(THURSDAY, &x, 0, 5, &ps(&[])).unwrap();
        assert!(t.iter().all(|t| !t.open));
    }

    #[test]
    fn inside_day_nr4_gap_down_short() {
        // bars 0-2 wide; bar 3 inside (102/98, range 4 < 20); bar 4 opens 97 < 98 (gap down), low 96 reaches 97 x 0.9975
        let r = bars(&[W, W, W, (100.0, 102.0, 98.0, 100.0), (97.0, 97.5, 96.0, 96.5)]);
        let t = go(INSIDE, &r, &[]);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].0, t[0].2, t[0].3, t[0].4), (4, 4, 96.5, -1.0));
        assert!((t[0].1 - 97.0 * 0.9975).abs() < 1e-9);
        // open % 2: level 95.06 not reached by the low 96
        assert!(go(INSIDE, &r, &[("open_pct", 2.0)]).is_empty());
    }

    #[test]
    fn outside_day_skips_ws7() {
        // bar 6 outside vs bar 5 and range 30 wider than all six before (WS7): no trade; narrower earlier bar breaks WS7
        let mut b = vec![(100.0, 101.0, 99.0, 100.0); 6];
        b.push((100.0, 116.0, 86.0, 100.0));
        b.push((117.0, 119.0, 116.5, 118.0));
        assert!(go(OUTSIDE, &bars(&b), &[]).is_empty());
        b[1] = (100.0, 130.0, 70.0, 100.0); // range 60 > 30: yesterday is no longer WS7
        let t = go(OUTSIDE, &bars(&b), &[]);
        assert_eq!(t.len(), 1);
        assert!((t[0].1 - 117.0 * 1.0075).abs() < 1e-9);
        assert_eq!((t[0].2, t[0].3, t[0].4), (7, 118.0, 1.0));
    }

    #[test]
    fn turtle_short_needs_weekday_lowest_close_and_gap() {
        // Mon..Fri falling closes, next Monday (bar 5) gaps down below Friday's low
        let r = bars(&[(110.0, 111.0, 104.0, 105.0), (105.0, 106.0, 103.0, 104.0), (104.0, 105.0, 102.0, 103.0), (103.0, 104.0, 101.0, 102.0),
            (102.0, 103.0, 100.0, 101.0), (99.0, 99.5, 97.0, 98.0)]);
        let t = go(TURTLE, &r, &[("low_n", 5.0)]);
        assert_eq!((t[0].0, t[0].2, t[0].3, t[0].4), (5, 5, 98.0, -1.0));
        assert!((t[0].1 - 99.0 * 0.995).abs() < 1e-9);
        assert!(go(TURTLE, &r, &[("low_n", 5.0), ("weekday", 2.0)]).is_empty());
        assert!(go(TURTLE_GAPUP, &r, &[("low_n", 5.0), ("weekday", 1.0)]).is_empty()); // a gap down is not a gap up
        // AmiBroker LLV near the start of the data uses the closes available: a 14-close period still trades on bar 5
        assert_eq!(go(TURTLE, &r, &[("low_n", 14.0)]).len(), 1);
    }

    #[test]
    fn momentum_manic_monday_short_friday_cover_monday() {
        // Mon-Thu closes 105..102; Friday (bar 4) closes 95 below them, high 100 < R1, close < S1; Monday (bar 5) covers
        let r = bars(&[(105.0, 106.0, 104.0, 105.0), (104.0, 105.0, 103.0, 104.0), (103.0, 104.0, 102.0, 103.0), (102.0, 103.0, 101.0, 102.0),
            (100.0, 100.0, 94.0, 95.0), (95.0, 97.0, 93.0, 96.0)]);
        // Thursday pivot: PP 102, R1 103, S1 101
        let t = go(MAMM, &r, &[("low_n", 4.0)]);
        assert_eq!(t, vec![(4, 95.0, 5, 96.0, -1.0)]);
        // exit weekday Tuesday: the data ends on Monday, so the trade closes there as End of period
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(run(MAMM, &x, 0, 5, &ps(&[("low_n", 4.0), ("exit_day", 2.0)])).unwrap()[0].reason, "End of period");
    }

    #[test]
    fn checklist_incorrect_strategies_follow_their_afls() {
        // Buying Thursday: bar 3 (Thursday) opens above Wednesday's close, closes green above R1, low above S1; sell Friday's close
        let mut b = vec![(100.0, 101.0, 99.0, 100.0); 3];
        b.push((100.5, 104.0, 100.2, 103.5));
        b.push((103.0, 105.0, 102.0, 104.0));
        assert_eq!(go(THURSDAY, &bars(&b), &[]), vec![(3, 103.5, 4, 104.0, 1.0)]);
        // entered on the last bar: stays open (no exit bar yet)
        let r = bars(&b[..4]);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run(THURSDAY, &x, 0, 3, &ps(&[])).unwrap()[0].open);
        // Three tumbles: three red candles then a gap up on Monday never trades (the AFL excludes Monday), on Friday it does
        let red3 = [(103.0, 103.5, 101.0, 102.0), (102.0, 102.5, 100.0, 101.0), (101.0, 101.5, 99.0, 100.0)];
        let mut b: Vec<_> = vec![(100.0, 101.0, 99.0, 100.0), (100.0, 101.0, 99.0, 100.0)];
        b.extend(red3); // bars 2-4 Wed-Fri
        b.push((102.0, 103.0, 101.6, 102.5)); // bar 5 Monday: gap up above 101.5
        assert!(go(TUMBLES, &bars(&b), &[("open_pct", 0.25)]).is_empty());
        let mut b: Vec<_> = vec![(100.0, 101.0, 99.0, 100.0)];
        b.extend(red3); // bars 1-3 Tue-Thu
        b.push((102.0, 103.0, 101.6, 102.5)); // bar 4 Friday
        let t = go(TUMBLES, &bars(&b), &[("open_pct", 0.25)]);
        assert!(t.len() == 1 && (t[0].1 - 102.0 * 1.0025).abs() < 1e-9 && t[0].4 == 1.0);
        // DUDD 2: yesterday NR4, red / green / red, gap down; the AFL short price has no low check, so below the low it fills at the low
        let b = bars(&[(100.0, 110.0, 90.0, 100.0), (100.0, 110.0, 90.0, 99.0), (100.0, 110.0, 90.0, 102.0),
            (100.0, 100.5, 99.5, 99.8), (99.0, 99.2, 98.9, 99.1)]);
        assert_eq!(go(DUDD[1], &b, &[("open_pct", 1.0)]), vec![(4, 98.9, 4, 99.1, -1.0)]);
        // Buy at Open (mode 1): every chosen weekday, open + % held inside the bar; a short is priced at open PLUS the %
        let b = bars(&[(100.0, 101.0, 99.0, 100.5), (100.0, 100.2, 99.0, 99.5)]);
        assert_eq!(go(OPEN_CLOSE, &b, &[("mode", 1.0), ("weekday", 2.0), ("open_pct", 1.0)]), vec![(1, 100.2, 1, 99.5, 1.0)]);
        assert_eq!(go(OPEN_CLOSE_LS, &b, &[("side", 2.0), ("weekday", 2.0), ("open_pct", 0.1)]), vec![(1, 100.1, 1, 99.5, -1.0)]);
        // Candle colour (the AFL loop starts at bar 1): red on bar 1 is entered at its close and stays open on the last bar
        let t = go(CANDLE, &b, &[("colour", 2.0), ("side", 2.0)]);
        assert_eq!((t.len(), t[0].0, t[0].1, t[0].4), (1, 1, 99.5, -1.0));
    }

    #[test]
    fn gap_family_types_and_sides() {
        // bar 1 is Tuesday: lap up (open 100.5 between close 100 and high 101)
        let r = bars(&[(100.0, 101.0, 99.0, 100.0), (100.5, 102.0, 100.0, 101.5)]);
        assert!(go(GAP, &r, &[("weekday", 2.0)]).is_empty()); // not a gap up
        let t = go(GAP, &r, &[("weekday", 2.0), ("gap", 3.0), ("open_pct", 1.0)]);
        assert!((t[0].1 - 101.505).abs() < 1e-9 && t[0].3 == 101.5 && t[0].4 == 1.0);
        let t = go(GAP, &r, &[("weekday", 2.0), ("gap", 3.0), ("side", 2.0), ("open_pct", 0.25)]);
        assert!(crate::books::ab::near_ab(t[0].1, 100.5 * 0.9975) && t[0].4 == -1.0);
        assert!(go(GAP, &r, &[("weekday", 1.0), ("gap", 3.0)]).is_empty());
    }

    // ---- kora reddy weeknum given month (added 20-11-2026)
    /// Rows on the given dates, each bar (open, high, low, close).
    fn dated(days: &[&str], ohlc: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        days.iter().zip(ohlc).map(|(d, &(open, high, low, close))| Row { ticker: "T".into(), date: d.to_string(), open, high, low, close, volume: None }).collect()
    }
    const P: (f64, f64, f64, f64) = (100.0, 101.0, 99.0, 100.0); // plain bar: closes below its high, above its low

    #[test]
    fn weeknum_registered_everywhere() {
        let cat = catalogue();
        assert_eq!(cat.len(), IDS.len());
        for id in [WEEKNUM, WEEKNUM_MF] {
            assert!(cat.iter().any(|d| d.id == id) && family_of(id).is_some() && params(id).is_some());
        }
        assert!(params(WEEKNUM_MF).unwrap().iter().any(|p| p.key == "sma_on") && !params(WEEKNUM).unwrap().iter().any(|p| p.key == "sma_on"));
    }

    #[test]
    fn weeknum_defaults_buy_tuesday_close_sell_next_tuesday() {
        // bars 0..10 from Monday 2024-01-01: Tuesdays are bars 1, 6; the exit bar takes no new entry
        let mut b = vec![P; 11];
        b[1].3 = 100.5;
        b[6].3 = 100.8;
        let r = bars(&b);
        assert_eq!(go(WEEKNUM, &r, &[]), vec![(1, 100.5, 6, 100.8, 1.0)]);
        assert_eq!(go(WEEKNUM_MF, &r, &[]), go(WEEKNUM, &r, &[]));
        // a Tuesday closing at its high is not bought (UC); the next Tuesday is
        b[1] = (100.0, 101.0, 99.0, 101.0);
        let t = go(WEEKNUM, &bars(&b), &[]);
        assert_eq!((t[0].0, t[0].4), (6, 1.0));
        // exit weekday Thursday: MaxBars = 4 - 2 + 1 = 3, so the time exit (2 bars) fires on Thursday too
        assert_eq!(go(WEEKNUM, &r, &[("exit_day", 4.0)])[0].2, 3);
        // entry and exit at the open
        let t = go(WEEKNUM, &r, &[("entry_at", 1.0), ("exit_at", 1.0)]);
        assert_eq!((t[0].1, t[0].3), (100.0, 100.0));
    }

    #[test]
    fn weeknum_holiday_entry_reads_the_next_bar() {
        // Tuesday 2024-01-02 is missing: with skip (default) no entry that week; with skip 0 Monday is the entry day
        let r = dated(&["2024-01-01", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-08", "2024-01-09"], &[P; 6]);
        assert!(go(WEEKNUM, &r, &[]).iter().all(|t| t.0 == 5));
        // the AFL never buys bar 0 (dead-stock setting NO), so Monday as bar 0 gives no entry
        assert!(go(WEEKNUM, &r, &[("entry_skip", 0.0)]).iter().all(|t| t.0 == 5));
        // start the data on the Friday before
        let r = dated(&["2023-12-29", "2024-01-01", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-08", "2024-01-09"], &[P; 7]);
        let t = go(WEEKNUM, &r, &[("entry_skip", 0.0)]);
        assert_eq!((t[0].0, t[0].2), (1, 6));
        // Mon-Fri copy: Friday before a week whose Monday and Tuesday are missing is an entry day too
        let r = dated(&["2024-01-01", "2024-01-04", "2024-01-05", "2024-01-10", "2024-01-11", "2024-01-16"], &[P; 6]);
        assert!(go(WEEKNUM, &r, &[("entry_skip", 0.0)]).iter().all(|t| t.0 == 5)); // only the last bar's Tuesday
        let t = go(WEEKNUM_MF, &r, &[("entry_skip", 0.0)]);
        assert_eq!((t[0].0, t[0].2), (2, 5));
    }

    #[test]
    fn weeknum_week_of_month_loop() {
        // weekdays from Monday 2024-01-01 to Tuesday 2024-02-06 (27 bars)
        let r = bars(&[P; 27]);
        let x = Ctx::new(&r, &r, &[]);
        let wk = weeknum_of_month(&x);
        // bar 0 starts the month but the AFL's first-Monday test needs i != 0, so Monday 8 Jan resets to 1 again
        assert_eq!((wk[0], wk[5], wk[10], wk[15], wk[20], wk[23], wk[24], wk[25]), (1, 1, 2, 3, 4, 4, 4, 1));
        assert_eq!(r[25].date, "2024-02-05");
        // week 1 only: Tuesday entries on 2 Jan (bar 1), 9 Jan sells and cannot rebuy, then 6 Feb (bar 26)
        let t = go(WEEKNUM, &r, &[("week_on", 1.0)]);
        assert_eq!(t.iter().map(|t| t.0).collect::<Vec<_>>(), vec![1, 26]);
        // February only
        assert_eq!(go(WEEKNUM, &r, &[("month", 2.0)]).iter().map(|t| t.0).collect::<Vec<_>>(), vec![26]);
    }

    #[test]
    fn weeknum_short_levels_and_circuit_exit() {
        // short Tuesday (bar 1) at 100; Wednesday's low reaches the 1% profit level 99 from an open above it
        let mut b = vec![P; 6];
        b[2] = (99.5, 100.0, 98.5, 99.0 + 0.2);
        let t = go(WEEKNUM, &bars(&b), &[("side", 2.0), ("pb_pct", 1.0)]);
        assert_eq!((t[0].0, t[0].2, t[0].4), (1, 2, -1.0));
        assert!((t[0].3 - 99.0).abs() < 1e-4);
        // a gap below it covers at open PLUS the 0.25% buffer, as the AFL writes it
        b[2] = (98.0, 98.6, 97.5, 98.2);
        let t = go(WEEKNUM, &bars(&b), &[("side", 2.0), ("pb_pct", 1.0)]);
        assert!(crate::books::ab::near_ab(t[0].3, 98.0 * 1.0025));
        // long: the exit Tuesday (bar 6) is a lower-circuit bar O = L = C, so the sell waits for bar 7
        let mut b = vec![P; 9];
        b[6] = (99.0, 100.0, 99.0, 99.0);
        b[7].3 = 100.3;
        let t = go(WEEKNUM, &bars(&b), &[]);
        assert_eq!((t[0].0, t[0].2, t[0].3), (1, 7, 100.3));
    }

    #[test]
    fn weeknum_filters_st_and_sma() {
        // steadily rising closes: daily ST (period 5) is below the close, SMA 3 below the close
        let b: Vec<_> = (0..30).map(|k| { let c = 100.0 + k as f64; (c - 0.5, c + 1.0, c - 1.0, c) }).collect();
        let r = bars(&b);
        let on = go(WEEKNUM, &r, &[("filter", 1.0), ("st_tf", 2.0)]);
        assert!(!on.is_empty() && on.iter().all(|t| t.0 >= 5));
        assert!(go(WEEKNUM, &r, &[("filter", 1.0), ("st_tf", 2.0), ("st_type", 0.0)]).is_empty());
        // fewer than 6 months of data: the monthly ATR(5) line is still Null, so nothing passes
        assert!(go(WEEKNUM, &r, &[("filter", 1.0)]).is_empty());
        // SMA 3 does not exist on bar 1, so the copy's first buy is the next Tuesday
        assert_eq!(go(WEEKNUM, &r, &[])[0].0, 1);
        assert_eq!(go(WEEKNUM_MF, &r, &[("sma_on", 1.0), ("sma_period", 3.0)])[0].0, 6);
        assert!(go(WEEKNUM_MF, &r, &[("sma_on", 1.0), ("sma_period", 3.0), ("sma_type", 1.0)]).is_empty());
    }

    // ---- kora reddy all day (added 20-11-2026)
    fn wk(id_rows: &[Row], kv: &[(&str, f64)]) -> Vec<(usize, f64, usize, f64, f64, String)> {
        let x = Ctx::new(id_rows, id_rows, &[]);
        run(WEEKDAY, &x, 0, id_rows.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.dir, t.reason)).collect()
    }
    const F: (f64, f64, f64, f64) = (100.0, 102.0, 98.0, 101.0); // ordinary bar

    #[test]
    fn weekday_defaults_enter_and_exit_the_same_tuesday() {
        assert_eq!((weekday_max_bars(2, 2), weekday_max_bars(1, 2), weekday_max_bars(4, 2), weekday_max_bars(5, 1)), (6, 2, 4, 2));
        assert!(catalogue().iter().any(|d| d.id == WEEKDAY) && family_of(WEEKDAY).is_some());
        // bar 1 Tuesday: bought and sold at its close (the exit test follows the entry on the same bar); bar 6 Tuesday has
        // High = Close (upper circuit): no entry
        let r = bars(&[F, F, F, F, F, F, (100.0, 103.0, 99.0, 103.0)]);
        assert_eq!(wk(&r, &[]), vec![(1, 101.0, 1, 101.0, 1.0, "Exit weekday".to_string())]);
        // entry at the open, exit at the close: the Tuesday open-to-close
        assert_eq!(wk(&r, &[("long_entry_at", 1.0)])[0].1, 100.0);
    }

    #[test]
    fn weekday_exit_day_max_bars_and_circuit_skip() {
        // Monday entry, Wednesday exit; the AFL loop starts at bar 1, so the Monday of bar 0 is not bought
        let r = bars(&[F, F, F, F, F, F, F, (100.0, 104.0, 99.0, 103.0), F]);
        assert_eq!(wk(&r, &[("long_entry_day", 1.0), ("long_exit_day", 3.0)]), vec![(5, 101.0, 7, 103.0, 1.0, "Exit weekday".to_string())]);
        // the AFL loop starts at bar 1: no entry on bar 0. With Wednesday missing, MaxBars 3 exits on Thursday (2 bars on)
        let mut r = bars(&[F, F, F, (100.0, 104.0, 99.0, 102.0), F, F, F, F, F, F]);
        r.remove(2);
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(dow(&x, 0), 1);
        let t = wk(&r, &[("long_entry_day", 1.0), ("long_exit_day", 3.0)]);
        assert_eq!((t[0].0, t[0].2, t[0].3, t[0].5.as_str()), (4, 6, 101.0, "Exit weekday"));
        let t = wk(&r, &[("long_entry_day", 2.0), ("long_exit_day", 3.0)]);
        assert_eq!((t[0].0, t[0].2, t[0].3, t[0].5.as_str()), (1, 2, 102.0, "Max bars"));
        // the Wednesday exit bar has Low = Open = Close: skipped, sold at Thursday's close
        let r = bars(&[F, F, (100.0, 101.0, 100.0, 100.0), (100.0, 103.0, 99.0, 102.5), F]);
        assert_eq!(wk(&r, &[("long_exit_day", 3.0)]), vec![(1, 101.0, 3, 102.5, 1.0, "Exit after circuit bar".to_string())]);
    }

    #[test]
    fn weekday_hard_sl_profit_booking_and_buffer() {
        use crate::books::ab::{addmul, mul32};
        let entry = (99.0, 101.0, 98.5, 100.0); // Tuesday close 100
        // hard SL 2%: Wednesday opens 97 below 98 -> Open - 0.25%
        let r = bars(&[F, entry, (97.0, 99.0, 96.0, 98.5), F, F]);
        let t = wk(&r, &[("long_exit_day", 4.0), ("hsl_pct", 2.0)]);
        assert_eq!((t[0].2, t[0].5.as_str()), (2, "Hard SL"));
        assert!((t[0].3 - addmul(97.0, -0.0025)).abs() < 1e-9);
        // no gap: the level itself
        let r = bars(&[F, entry, (99.0, 99.5, 97.5, 98.5), F, F]);
        assert_eq!(wk(&r, &[("long_exit_day", 4.0), ("hsl_pct", 2.0)])[0].3, mul32(0.98, 100.0));
        // profit 3% gapped through (open 104 > 103): sells at Open - 0.25% as written
        let r = bars(&[F, entry, (104.0, 105.0, 103.5, 104.5), F, F]);
        let t = wk(&r, &[("long_exit_day", 4.0), ("pb_pct", 3.0)]);
        assert!((t[0].3 - addmul(104.0, -0.0025)).abs() < 1e-9 && t[0].5 == "Profit booking");
        // short: stop 2% above 100, gap to 103 covers at Open + 0.25%
        let r = bars(&[F, entry, (103.0, 104.0, 102.0, 103.5), F, F]);
        let t = wk(&r, &[("long_on", 0.0), ("short_on", 1.0), ("short_exit_day", 4.0), ("hsl_pct", 2.0)]);
        assert_eq!((t[0].0, t[0].1, t[0].2, t[0].4), (1, 100.0, 2, -1.0));
        assert!((t[0].3 - addmul(103.0, 0.0025)).abs() < 1e-9);
    }

    #[test]
    fn weekday_short_side_and_filters() {
        // short Tuesday close, cover Wednesday close; a Tuesday with Low = Close (lower circuit) is not shorted
        let r = bars(&[F, F, (100.0, 101.0, 98.0, 99.0), F, F, F, (100.0, 101.0, 97.0, 97.0), F]);
        let t = wk(&r, &[("long_on", 0.0), ("short_on", 1.0), ("short_exit_day", 3.0)]);
        assert_eq!(t, vec![(1, 101.0, 2, 99.0, -1.0, "Exit weekday".to_string())]);
        // long and short both on: a long and a short on Tuesday bar 1, and a long on bar 6 (its Low = Close only blocks shorts)
        assert_eq!(wk(&r, &[("short_on", 1.0)]).iter().map(|t| (t.0, t.4)).collect::<Vec<_>>(), vec![(1, -1.0), (1, 1.0), (6, 1.0)]);
        // SMA(5) filter: no average yet on bar 1; bar 6 closes above it; shorts also need Close >= MA (as written)
        let r = bars(&[F, F, F, F, F, F, (101.0, 104.0, 100.0, 103.0), F]);
        let t = wk(&r, &[("filter", 1.0), ("sma_period", 5.0)]);
        assert_eq!(t.iter().map(|t| t.0).collect::<Vec<_>>(), vec![6]);
        let t = wk(&r, &[("filter", 1.0), ("sma_period", 5.0), ("long_on", 0.0), ("short_on", 1.0)]);
        assert_eq!(t.iter().map(|t| t.0).collect::<Vec<_>>(), vec![6]);
        assert!(wk(&r, &[("filter", 1.0), ("sma_period", 0.0)]).is_empty());
        // daily ST(5, 1): rising closes put the close above the line, so the long trades; a Null line (start) does not
        let up: Vec<(f64, f64, f64, f64)> = (0..12).map(|k| { let b = 100.0 + 2.0 * k as f64; (b, b + 1.5, b - 0.5, b + 1.0) }).collect();
        let r = bars(&up);
        let x = Ctx::new(&r, &r, &[]);
        let line = weekday_st(&x, 3, 5, 1.0);
        assert!(line[..5].iter().all(|v| v.is_nan()) && (5..12).all(|i| x.c[i] > line[i]));
        let t = wk(&r, &[("filter", 2.0), ("st_tf", 3.0)]);
        assert_eq!(t.iter().map(|t| t.0).collect::<Vec<_>>(), vec![6, 11]);
    }
}
