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
pub const IDS: [&str; 26] = [INSIDE, OUTSIDE, TURTLE, TURTLE_GAPUP, MAMM, GAP, BLOOD, THURSDAY, THURSDAY_MOD, MANIC, TUMBLES, TUMBLES_MOD,
    TWO_TUMBLE, STEPS, DUDD[0], DUDD[1], DUDD[2], DUDD[3], DUDD[4], DUDD[5], DUDD[6], OPEN_CLOSE, OPEN_CLOSE_LS, MON_FRI, CANDLE, UUDDD];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Kora Reddy", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(INSIDE, "Inside Day", "Short", &[
            "F|Yesterday was an inside day (high below and low above the day before) and an NR4 day (its range narrower than each of the 3 days before)",
            "E|Today opens with a gap down (open below yesterday's low); short at the open less the open % once the low reaches it",
            "X|Cover at today's close",
        ], "Inside Day (Kora Reddy AFL): short, intraday. Open % default 0.25 (team NIFTY setting). Note: the AFL fills at open - % when the day's low reaches it."),
        def(OUTSIDE, "Outside Day", "Long", &[
            "F|Yesterday was an outside day (high above and low below the day before) and was not a WS7 day (its range was not wider than each of the 6 days before)",
            "E|Today opens with a gap up (open above yesterday's high); buy at the open plus the open % once the high reaches it",
            "X|Sell at today's close",
        ], "Outside Day (Kora Reddy AFL): long, intraday. Open % default 0.75 (team NIFTY setting)."),
        def(TURTLE, "Turtle Short", "Short", &[
            "F|Today is the chosen weekday (default Monday) and yesterday's close was the lowest close of the last N days including yesterday (default 14)",
            "E|Today opens with a gap down; short at the open less the open % (default 0.5) once the low reaches it",
            "X|Cover at today's close",
        ], "Turtle Short (Kora Reddy AFL): short, intraday. The AFL text says Friday or Monday; its code tests one chosen weekday, kept here as a setting. Defaults (Monday, 0.5%, 14) are the team NIFTY setting."),
        def(TURTLE_GAPUP, "Turtle Short on GapUp", "Short", &[
            "F|Today is the chosen weekday (default Friday) and yesterday's close was the lowest close of the last N days including yesterday (default 4)",
            "E|Today opens with a gap up; short at the open less the open % (default 1) once the low reaches it",
            "X|Cover at today's close",
        ], "Turtle Short on GapUp (Kora Reddy AFL): short, intraday. Defaults (Friday, 1%, 4) are the team NIFTY setting; the AFL offers Monday or Friday."),
        def(MAMM, "Momentum Aided Manic Monday", "Short", &[
            "F|Today is the entry weekday (default Friday), today's close is below the lowest close of the previous N days (default 14)",
            "F|Today's high is below R1 and today's close is below S1 (pivots from yesterday: PP = (H+L+C)/3, R1 = 2PP - L, S1 = 2PP - H)",
            "E|Short at today's close",
            "X|Cover at the close of the next exit weekday (default Monday)",
        ], "Momentum Aided Manic Monday (Kora Reddy AFL): short at the close, held to the exit weekday's close; if that weekday is a holiday the trade waits for the next one, as in the AFL. Defaults (Friday, Monday, 14) are the team NIFTY setting."),
        def(GAP, "Gap Open to Close", "Long/Short", &[
            "F|Today is the chosen weekday (default Monday) and opens with the chosen gap: 1 gap up, 2 gap down, 3 lap up, 4 lap down",
            "E|Side 1 long: buy at the open plus the open % once the high reaches it; side 2 short: sell at the open less the open % once the low reaches it",
            "X|Exit at today's close",
        ], "Buy at GapUp or GapDown & Sell at Close (Kora Reddy AFL): intraday, long or short. Defaults (Monday, 0.25%) are the team NIFTY setting; gap type and side default to gap up, long."),
        def(BLOOD, "Buy when there is blood on street", "Long", &[
            "F|Today is the entry weekday (default Tuesday) and today's close is below the lowest close of the previous N days (default 10)",
            "F|Today, yesterday and the day before are red candles (open above close); version 3 also needs today's close below yesterday's low",
            "E|Buy at today's close",
            "X|Sell at the next bar's close",
        ], "Buy when there is blood on street (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"down day\" (the AFL reads a down day as a red candle, open above close). Versions 1 and 2 are the same code in the AFL. Defaults (version 1, Tuesday, 10) are the team NIFTY setting; the team ran it at 0.04% commission a side."),
        def(THURSDAY, "Buying Thursday", "Long", &[
            "F|Today is the entry weekday (default Thursday) and today opens above yesterday's close",
            "F|Today's close is above today's open and above R1, and today's low stays above S1 (pivots from yesterday: PP = (H+L+C)/3, R1 = 2PP - L, S1 = 2PP - H)",
            "E|Buy at today's close",
            "X|Sell at the next bar's close",
        ], "Buying Thursday (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's close was up and above R1\" (the AFL tests close above today's open, not above yesterday's close). The team's workbook says 06-09-2021 but its results match a run to 26-10-2021; 0.04% commission a side."),
        def(THURSDAY_MOD, "Buying Thursday (Modified)", "Long", &[
            "F|All the Buying Thursday conditions on the entry weekday (default Thursday)",
            "F|Two bars back is a Tuesday with a red candle; or, through the AFL's state flag, three bars back is a Monday with a red candle two bars back",
            "E|Buy at today's close",
            "X|Sell at the next bar's close",
        ], "Buying Thursday (Modified) (Kora Reddy AFL), coded as the AFL is written, including its abc state flag. Team checklist: Incorrect, remark \"Tuesday's (or Monday's if Tuesday happens to be a holiday) close was down\" (the AFL checks a red candle by bar position). 0.04% commission a side in the team's run."),
        def(MANIC, "Manic Monday", "Short", &[
            "F|Today is the entry weekday (default Friday); today and yesterday are red candles (open above close)",
            "F|Today's high is below R1 (pivot from yesterday: PP = (H+L+C)/3, R1 = 2PP - L)",
            "E|Short at today's close",
            "X|Cover at the close of the next exit weekday (default Monday); the same bar when entry and exit weekday are equal",
        ], "Manic Monday (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\" (the AFL has no test of today's open against yesterday's close). Defaults (Friday, Monday) are the team setting; the team's run used 0.04% commission a side."),
        def(TUMBLES, "Three tumbles and a jump", "Long", &[
            "F|Today is any weekday except Monday (the AFL's weekday setting is not used by its code)",
            "F|The last three bars are red candles (open above close) and today opens with a gap up (open above yesterday's high)",
            "E|Buy at the open plus the open % once the high reaches it",
            "X|Sell at today's close",
        ], "Three tumbles and a jump (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Last three trading days were down closes\" (the AFL tests red candles), and \"Weekdays are not in param\". Default open % 0.25 is the team NIFTY setting."),
        def(TUMBLES_MOD, "Modified Three tumbles and a jump", "Long", &[
            "F|Today is the chosen weekday (default Wednesday) and yesterday's close was the lowest close of the last N days including yesterday (default 6)",
            "F|The last three bars are red candles (open above close); no gap condition",
            "E|Buy at the open plus the open % (default 0.25) once the high reaches it",
            "X|Sell at today's close",
        ], "Modified Three tumbles and a jump (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Previous 3 days were down closes\" (the AFL tests red candles). Defaults (Wednesday, 0.25%, 6) are the team NIFTY setting."),
        def(TWO_TUMBLE, "Two Tumble & Another Tumble", "Short", &[
            "F|Today is the chosen weekday (default Monday) and the last two bars are red candles (close below open)",
            "E|Short at the open less the open % (default 0.25) once the low reaches it",
            "X|Cover at today's close",
        ], "Two Tumble & Another Tumble for Short Intraday FO (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Yesterday and day before yesterday were down closes\" (the AFL tests red candles). The workbook's Info says 01-01-2007, but its results match a run from the start of the data; 0.02% commission a side."),
        def(STEPS, "Three steps and a stumble", "Short", &[
            "F|Today is the chosen weekday (default Tuesday); yesterday's close is above the highest close of the N bars before it (default 16)",
            "F|The last three bars are green candles (close above open) and today opens with a gap up",
            "E|Short at the open plus the open % (default 0.25) once the high reaches it",
            "X|Cover at today's close",
        ], "Three steps and a stumble (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"close up\" (the AFL tests green candles). The AFL shorts above the open. Defaults (Tuesday, 0.25%, 16) are the team NIFTY setting."),
        def(DUDD[0], "DUDD Version 1", "Short", &[
            "F|Today is a WS7 day (today's range wider than each of the 6 days before); today red, yesterday green, the day before green, 3 days ago red",
            "E|Short at today's close",
            "X|Cover at the next bar's close",
        ], "DUDD Version 1 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\" (the AFL tests candle colours). The workbook's Info says 1-1-2007, but its trade lists match a run from the start of the data; 0.04% commission a side."),
        def(DUDD[1], "DUDD Version 2", "Short", &[
            "F|Yesterday was an NR4 day (its range narrower than each of the 3 days before); today opens with a gap down",
            "F|Yesterday red, the day before green, 3 days ago red (candle colours)",
            "E|Short at the open less the open % (default 0.25); the AFL does not check the low reaches it, so the fill is held within the day's range",
            "X|Cover at today's close",
        ], "DUDD Version 2 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"down close\". The AFL sets the short price without checking the day's low; AmiBroker then fills at the low, kept here. Trade lists match a run from the start of the data; 0.02% commission a side."),
        def(DUDD[2], "DUDD Version 3", "Short", &[
            "F|Yesterday was a WS4 day (its range wider than each of the 3 days before); today opens with a gap down",
            "F|Yesterday red, the day before green, 3 days ago red (candle colours)",
            "E|Short at the open less the open % (default 0.25), held within the day's range",
            "X|Cover at today's close",
        ], "DUDD Version 3 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\". Same fill as Version 2; 0.02% commission a side."),
        def(DUDD[3], "DUDD Version 4", "Short", &[
            "F|Yesterday was an NR7 day (its range narrower than each of the 6 days before); today opens with a gap down",
            "F|Yesterday, the day before and 3 days ago were red candles",
            "E|Short at the open less the open % (default 0.25), held within the day's range",
            "X|Cover at today's close",
        ], "DUDD Version 4 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\". Same fill as Version 2; 0.02% commission a side."),
        def(DUDD[4], "DUDD Version 5", "Short", &[
            "F|Yesterday was an NR7 day; today opens with a gap down",
            "F|Yesterday red, the day before green, 3 days ago red (candle colours)",
            "E|Short at the open less the open % (default 0.25), held within the day's range",
            "X|Cover at today's close",
        ], "DUDD Version 5 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\". Same fill as Version 2; 0.02% commission a side."),
        def(DUDD[5], "DUDD Version 6", "Short", &[
            "F|Yesterday was an NR7 day; today opens with a gap down",
            "F|Yesterday green, the day before green, 3 days ago red (candle colours)",
            "E|Short at the open less the open % (default 0.25), held within the day's range",
            "X|Cover at today's close",
        ], "DUDD Version 6 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\". Same fill as Version 2; 0.02% commission a side."),
        def(DUDD[6], "DUDD Version 7", "Short", &[
            "F|Yesterday was a WS7 day (its range wider than each of the 6 days before); today opens with a gap down",
            "F|Yesterday red, the day before green, 3 days ago red (candle colours)",
            "E|Short at the open less the open % (default 0.25), held within the day's range",
            "X|Cover at today's close",
        ], "DUDD Version 7 (Kora Reddy AFL), coded as the AFL is written. Team checklist: Incorrect, remark \"Today's open is down\". Same fill as Version 2; 0.02% commission a side."),
        def(OPEN_CLOSE, "Buy at Open or Close", "Long", &[
            "F|Today is the chosen weekday (default Thursday); no other condition",
            "E|Mode 1: buy at the open plus the open %, with no check that the high reaches it (the fill is held within the day's range); mode 2 (default): buy at the close",
            "X|Mode 1: sell at today's close; mode 2: sell at the next bar's close",
        ], "Buy at Open or Close and Sell at same day Close Or Next day Close (Kora Reddy AFL), coded as the AFL is written; the team's checklist gives no verdict. The team ran mode 1 at 0.02% and mode 2 at 0.04% commission a side over all quotes. Their mode-1 runs lose about 99%, after which AmiBroker skips trades the account cannot buy one unit of; this vault's normalized returns take every trade."),
        def(OPEN_CLOSE_LS, "BuyShort at Open or Close", "Long/Short", &[
            "F|Today is the chosen weekday (default Monday); no other condition",
            "E|Mode 1 (default): enter at the open plus the open % (default 0, the team's setting) for long and short alike, with no check that the bar reaches it (held within the day's range); mode 2: enter at the close",
            "X|Mode 1: exit at today's close; mode 2: exit at the next bar's close",
        ], "BuyShort at Open or Close and SellCover at same day Close Or Next day Close (Kora Reddy AFL), coded as the AFL is written; the team's checklist gives no verdict. Unlike the other Kora strategies, the team ran it on the NSE spot indices, not futures: on NIFTY 50 (03-01-2000 to 31-03-2022) and NIFTY BANK (09-06-2005 to 31-03-2022), open % 0 and no commission, it reproduces all 40 rows of the team's 2022 optimization workbooks. The AFL prices a short at open PLUS the %."),
        def(MON_FRI, "Buying Mondays & Fridays", "Long", &[
            "F|Today is the chosen weekday (default Monday); today opens above yesterday's close and yesterday opened above the close of the day before",
            "F|Yesterday was a green candle (close above open)",
            "E|Buy at the open plus the open % (default 0.25) once the high reaches it",
            "X|Sell at today's close",
        ], "Buying Mondays & Fridays for Intraday FO (Kora Reddy AFL), coded as the AFL is written; the team's checklist gives no verdict. Defaults (Monday, 0.25%) are the team setting; 0.02% commission a side."),
        def(CANDLE, "Entry Based on Candle Color", "Long/Short", &[
            "F|Today's candle has the chosen colour: 1 green (close above open, default), 2 red (open above close)",
            "E|Side 1 long (default) or 2 short at today's close",
            "X|Exit at the next bar's close",
        ], "Entry Based on Candle Color (Kora Reddy AFL), coded as the AFL is written; the team's checklist gives no verdict. The team's trade lists (all four colour and side combinations) match a run from the start of the data, not the 2007 in its info sheet; 0.04% commission a side."),
        def(UUDDD, "UUDDD", "Short", &[
            "F|The last four bars are red candles (open above close) and today opens with a gap down",
            "E|Short at the open less the open % (default 0.25), with no check that the low reaches it (held within the day's range)",
            "X|Cover at today's close",
        ], "UUDDD (Kora Reddy AFL), coded as the AFL is written; the team's checklist gives no verdict. The AFL tests four red candles, not the up/up/down/down closes its name and info sheet describe. Trade lists match a run from the start of the data; 0.04% commission a side."),
    ]
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
        id if id == DUDD[0] => vec![],
        id if DUDD.contains(&id) => vec![pct(0.25)],
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
        let level = if long { x.o[i] * (1.0 + pct / 100.0) } else { x.o[i] * (1.0 - pct / 100.0) };
        let filled = if long { x.h[i] >= level } else { x.l[i] <= level };
        if filled {
            out.push(trade(if long { 1.0 } else { -1.0 }, i, level, "limit", i, x.c[i], "Same-day close"));
        }
    }
    out
}

/// Enter at the close of a setup bar and exit at the next bar's close (the AFL's `else if(Position ...)` exit, so no
/// new entry on the exit bar). An entry on the last bar stays open.
fn close_next(x: &Ctx, from: usize, to: usize, dir: f64, start: usize, setup: impl Fn(usize) -> bool) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut cur: Option<usize> = None;
    for i in from.max(start)..=to {
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
    out
}

/// AFL Buying Thursday (Modified): its `abc` flag, set on the entry weekday from the bar two (or three) back, then the
/// Buying Thursday conditions; buy at the close, sell at the next bar's close.
fn thursday_modified(x: &Ctx, from: usize, to: usize, day: i64) -> Vec<Trade> {
    let mut out = Vec::new();
    let (mut abc, mut cur) = (0, None::<usize>);
    for i in from.max(3)..=to {
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
    out
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
            for i in from.max(1)..=to {
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
            out
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
            for i in from.max(1)..=to {
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
            out
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
                let level = x.o[i] * (1.0 + pct / 100.0);
                if dow(x, i) == day && x.c[i - 1] > highest_close(x, i - 2, len) && green(x, i - 1) && green(x, i - 2) && green(x, i - 3)
                    && gap_up(x, i) && x.h[i] >= level
                {
                    out.push(trade(-1.0, i, level, "limit", i, x.c[i], "Same-day close"));
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
                    .map(|i| trade(dir, i, (x.o[i] * (1.0 + pct / 100.0)).clamp(x.l[i], x.h[i]), "limit", i, x.c[i], "Same-day close"))
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
                    let px = (x.o[i] * (1.0 - pct / 100.0)).clamp(x.l[i], x.h[i]);
                    out.push(trade(-1.0, i, px, "limit", i, x.c[i], "Same-day close"));
                }
            }
            out
        }
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
                    let px = (x.o[i] * (1.0 - pct / 100.0)).clamp(x.l[i], x.h[i]);
                    out.push(trade(-1.0, i, px, "limit", i, x.c[i], "Same-day close"));
                }
            }
            out
        }
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
        _ => return None,
    })
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
        assert!((t[0].1 - 100.5 * 0.9975).abs() < 1e-9 && t[0].4 == -1.0);
        assert!(go(GAP, &r, &[("weekday", 1.0), ("gap", 3.0)]).is_empty());
    }
}
