//! Larry Williams, "Commodity Timing" newsletters 1991-1997 (scans of 85 issues). Book id `lwnl`.
//! Source copy: Books/Larry Williams/Larry Williams Newsletters 1994-1997.zip (cp -n from research drive
//! "Trading Knowledge II/@NewsLetters-Mags/Larry Williams Newsletters (1994-1997)/"). The archive is named 1994-1997 but holds
//! issues dated Jan 1991 to Dec 1997, four pages each (Jan 1997 has eight). Page refs below are "<issue month year> p.<page>".
//! Transcription and checks: library/lwnl/.
//!
//! Conventions (from the letters' own printouts): the S&P tests are System Writer runs on S&P 500 futures with
//! "P. Value 5.000" per 0.01 (= $500 per index point; the contract was split to $250 only in Nov 1997) and $50 commission.
//! Dollar stops are converted at p.pt_usd (500); other instruments use the same point distance (instrument swap only).
//! The Nasdaq-100 test (Sep 1996) uses p.pt_usd = 100 (the 1996 CME contract, $100 x index).
//! "Bail Out" = "get out on the first profitable opening" (Sep 1996 p.1). "Delayed Bailout" = "waiting N days ... then
//! exiting on any profitable opening after that point" (Nov 1996 p.1): the bailout applies from the opening of bar entry+N+1.
//! Stops are armed on the entry day for entries during the day (Oct 1996 printout: losers average 0 bars), from the next day
//! for entries on the close. This module has its own simulator (bar path O-near extreme-far extreme-C, orders gapped through
//! fill at the open, one position at a time); it does not use jkbwts::sim.
use crate::books::lwicw::{dow, getter, holiday, tdm, td_rel, Hol};
use crate::connors::{claim, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwnl";

/// Larry Williams block: rules admitted after the 23 Oct 2026 reading and independent ledgers
/// (authors/lwilliams/admission/SOURCE-CONTRACT.md). SPX_POINTS: S&P-futures dollar stops / point gaps, actual SPX only;
/// NDX_POINTS: the Nasdaq-100 futures dollar stop, actual NDX only; US_ONLY: US-holiday trades, US instruments only.
pub const REVIEWED: &[&str] = &["lwnl_jan_rally", "lwnl_mar_short", "lwnl_oct_td9_buy", "lwnl_oct_td17_sell", "lwnl_nov_td17_db7", "lwnl_jan97_td16_buy", "lwnl_jan97_td2_sell", "lwnl_jul97_td4_buy", "lwnl_jul97_td17_buy", "lwnl_jul97_td16_buy_30d", "lwnl_jul97_td14_sell", "lwnl_aug97_td12_buy_bail", "lwnl_aug97_td18_buy_bail", "lwnl_aug97_td7_sell_bail", "lwnl_aug97_td18_buy_7d", "lwnl_aug97_td2_sell_7d", "lwnl_nov97_td17_buy_7d", "lwnl_nov97_td18_buy_7d", "lwnl_nov97_td6_buy_30d", "lwnl_nov97_td15_buy_30d", "lwnl_dec97_td2_buy_1d", "lwnl_dec97_td14_buy_1d", "lwnl_dec97_td9_sell_1d", "lwnl_pullback", "lwnl_krut_buy", "lwnl_krut_sell", "lwnl_proj_contra", "lwnl_truth1_closehigh", "lwnl_gap_fail_300", "lwnl_gap_fail_525", "lwnl_ndx_vb_oops", "lwnl_tg_close", "lwnl_tg_open8", "lwnl_tg_oc", "lwnl_tg_open3_bail", "lwnl_xmas_bail", "lwnl_xmas_bail9", "lwnl_memorial_bail", "lwnl_memorial_bail6", "lwnl_july4_buy", "lwnl_yearend_sell", "lwnl_od_buy", "lwnl_od_buy3", "lwnl_od_sell", "lwnl_od_fade_buy", "lwnl_od_fade_sell", "lwnl_engulf_bull_1", "lwnl_engulf_bull_5", "lwnl_engulf_bull_10", "lwnl_engulf_bear_1", "lwnl_engulf_bear_5", "lwnl_engulf_bear_10", "lwnl_keyrev_buy", "lwnl_proj_limit", "lwnl_tdw_oc_tue", "lwnl_tdw_oc_wed", "lwnl_tdw_oc_thu", "lwnl_tdw_oc_fri"];
pub const SPX_POINTS: &[&str] = &["lwnl_jan_rally", "lwnl_mar_short", "lwnl_oct_td9_buy", "lwnl_oct_td17_sell", "lwnl_nov_td17_db7", "lwnl_jan97_td16_buy", "lwnl_jan97_td2_sell", "lwnl_jul97_td4_buy", "lwnl_jul97_td17_buy", "lwnl_jul97_td16_buy_30d", "lwnl_jul97_td14_sell", "lwnl_aug97_td12_buy_bail", "lwnl_aug97_td18_buy_bail", "lwnl_aug97_td7_sell_bail", "lwnl_aug97_td18_buy_7d", "lwnl_aug97_td2_sell_7d", "lwnl_nov97_td17_buy_7d", "lwnl_nov97_td18_buy_7d", "lwnl_nov97_td6_buy_30d", "lwnl_nov97_td15_buy_30d", "lwnl_dec97_td2_buy_1d", "lwnl_dec97_td14_buy_1d", "lwnl_dec97_td9_sell_1d", "lwnl_pullback", "lwnl_krut_buy", "lwnl_krut_sell", "lwnl_proj_contra", "lwnl_truth1_closehigh", "lwnl_gap_fail_300", "lwnl_gap_fail_525"];
pub const NDX_POINTS: &[&str] = &["lwnl_ndx_vb_oops"];
pub const US_ONLY: &[&str] = &["lwnl_tg_close", "lwnl_tg_open8", "lwnl_tg_oc", "lwnl_tg_open3_bail", "lwnl_xmas_bail", "lwnl_xmas_bail9", "lwnl_memorial_bail", "lwnl_memorial_bail6", "lwnl_july4_buy"];

macro_rules! sd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        sd!($id, $name, $group, $dir, $status, $rules, $notes, None)
    };
}

const SP: &str = "Book market: S&P 500 futures ($500/pt, $50 commission in the printed $). Rules with dollar or point amounts run on actual SPX only (from 2014, when SPX opens are real); rules without them run on indices, ETFs and stocks.";
const BOND: &str = "Needs daily T-Bond futures (OHLC); the vault has no bond futures series.";
const FUT: &str = "Needs daily commodity / currency futures (OHLC) for the named markets; the vault has none.";

// ------------------------------------------------------------------ seasonal trading-day-of-month table
// (id, month, trading day, dir, entry %, stop $, hold, bailout delay, repeat from td to month end)
// entry %: 0 = market on open; > 0 = stop at open +/- % x previous day's range in the trade direction; < 0 = limit the other way.
type Sea = (&'static str, u32, usize, f64, f64, f64, Option<usize>, Option<usize>, bool);
const SEA: &[Sea] = &[
    ("lwnl_jan_rally", 1, 19, 1.0, 0.10, 2200.0, Some(6), None, true),
    ("lwnl_mar_short", 3, 16, -1.0, 0.10, 2000.0, Some(2), None, true),
    ("lwnl_oct_td9_buy", 10, 9, 1.0, 0.0, 1500.0, Some(2), None, false),
    ("lwnl_oct_td17_sell", 10, 17, -1.0, 0.0, 1500.0, Some(2), None, false),
    ("lwnl_nov_td17_db7", 11, 17, 1.0, 0.0, 3000.0, None, Some(7), false),
    // Larry Williams block (23 Oct 2026): Jan 1997 p.7 trades the S&P "on the opening"; the report's 0.05% range stop is an option.
    ("lwnl_jan97_td16_buy", 1, 16, 1.0, 0.0, 1800.0, Some(5), None, false),
    ("lwnl_jan97_td2_sell", 1, 2, -1.0, 0.0, 1800.0, Some(5), None, false),
    ("lwnl_jul97_td4_buy", 7, 4, 1.0, 0.07, 1500.0, Some(7), None, false),
    ("lwnl_jul97_td17_buy", 7, 17, 1.0, 0.07, 1500.0, Some(7), None, false),
    ("lwnl_jul97_td16_buy_30d", 7, 16, 1.0, 0.07, 1500.0, Some(30), None, false), // Larry Williams block: Jul 1997 p.2 30-day hold
    ("lwnl_jul97_td14_sell", 7, 14, -1.0, 0.0, 2500.0, None, Some(0), false),
    ("lwnl_aug97_td12_buy_bail", 8, 12, 1.0, -0.07, 1500.0, None, Some(0), false),
    ("lwnl_aug97_td18_buy_bail", 8, 18, 1.0, -0.07, 1500.0, None, Some(0), false),
    ("lwnl_aug97_td7_sell_bail", 8, 7, -1.0, 0.07, 1500.0, None, Some(0), false),
    ("lwnl_aug97_td18_buy_7d", 8, 18, 1.0, -0.07, 1500.0, Some(7), None, false),
    ("lwnl_aug97_td2_sell_7d", 8, 2, -1.0, 0.07, 1500.0, Some(7), None, false),
    ("lwnl_nov97_td17_buy_7d", 11, 17, 1.0, 0.07, 2000.0, Some(7), None, false),
    ("lwnl_nov97_td18_buy_7d", 11, 18, 1.0, 0.07, 2000.0, Some(7), None, false),
    ("lwnl_nov97_td6_buy_30d", 11, 6, 1.0, 0.07, 2000.0, Some(30), None, false),
    ("lwnl_nov97_td15_buy_30d", 11, 15, 1.0, 0.07, 2000.0, Some(30), None, false),
    ("lwnl_dec97_td2_buy_1d", 12, 2, 1.0, 0.07, 3500.0, Some(1), None, false),
    ("lwnl_dec97_td14_buy_1d", 12, 14, 1.0, 0.07, 3500.0, Some(1), None, false),
    ("lwnl_dec97_td9_sell_1d", 12, 9, -1.0, 0.07, 3500.0, Some(1), None, false),
];

// ------------------------------------------------------------------ holiday table
// (id, holiday, k = trading day relative to the holiday (1 = last day before, -1 = first day after), dir, on close?, stop $, hold, bail delay)
type HolT = (&'static str, Hol, i32, f64, bool, f64, Option<usize>, Option<usize>);
const HOL: &[HolT] = &[
    ("lwnl_tg_close", Hol::Thanksgiving, 1, 1.0, true, 0.0, Some(1), None),
    ("lwnl_tg_open8", Hol::Thanksgiving, 2, 1.0, false, 0.0, Some(8), None),
    ("lwnl_tg_oc", Hol::Thanksgiving, 1, 1.0, false, 0.0, Some(0), None),
    ("lwnl_tg_open3_bail", Hol::Thanksgiving, 3, 1.0, false, 0.0, Some(9), Some(0)),
    ("lwnl_xmas_bail", Hol::Christmas, 5, 1.0, true, 0.0, None, Some(0)),
    ("lwnl_xmas_bail9", Hol::Christmas, 5, 1.0, true, 0.0, None, Some(9)),
    ("lwnl_memorial_bail", Hol::Memorial, 3, 1.0, true, 0.0, None, Some(0)),
    ("lwnl_memorial_bail6", Hol::Memorial, 3, 1.0, true, 0.0, None, Some(6)),
    ("lwnl_july4_sell", Hol::July4, 3, -1.0, false, 0.0, None, Some(3)),
    ("lwnl_july4_buy", Hol::July4, -2, 1.0, false, 0.0, None, Some(0)),
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        // ---- Seasonal trading-day-of-month trades
        sd!("lwnl_jan_rally", "S&P January rally: TD19 open + 10% range, exit 6 days", "Seasonals · trading day of month", "Long", Live,
            &["E|\"buy the S&P on the opening plus 10% of the previous days range starting the 19th trading (not calendar) day of January\" (Jan 1996 p.2): buy stop = open + 0.10 x previous day's range, placed each day from TD19 to the end of January until filled",
              "X|\"exiting on the close 6 days later\" (Jan 1996 p.2)", "X|\"the best one is probably $2,200\" stop (Jan 1996 p.2; printout largest loss -$2,250 = $2,200 + $50)"],
            "Printout S&P 07/02/82-01/22/96. 'starting the 19th' read as: the order stands from TD19 until it fills, once per January (13 trades in 13 Januaries). p.repeat=0 = TD19 only, 2 = re-enter while flat.",
            claim("13", "76%", "$2,261.54 ($29,400)", "6 days", "S&P 07/02/82-01/22/96")),
        sd!("lwnl_mar_short", "S&P March short: TD16 open - 10% range, exit 2 days", "Seasonals · trading day of month", "Short", Live,
            &["E|\"Selling on the opening, minus 10% of the previous days range, starting on the 16th trading day of the month [March]\" (Feb 1996 p.4)",
              "E|\"the 'system' can sell on the 17th day ... take a profit or be stopped out and re-enter on following days\" (Feb 1996 p.4): the order is placed every March day from TD16 while flat",
              "X|\"get out on the close two days later, or stopped out at a $2,000 loss\" (Feb 1996 p.4)"],
            SP, claim("24", "66%", "$1,425 ($34,200)", "2 days", "S&P (window not printed, letter Feb 1996)")),
        sd!("lwnl_oct_td9_buy", "S&P October TD9 buy, exit 2 days, $1,500 stop", "Seasonals · trading day of month", "Long", Live,
            &["E|\"buy long on the 9th trading (not calendar) day of the month\" [October] (Sep 1996 p.3): Read: on the opening (printout: losers average 0 bars, so the stop could be hit on the entry day)",
              "X|\"For the S&P exit on the close two days after entry or with a $1,500 stop\" (Sep 1996 p.3)"],
            "Printout 07/02/82-01/01/96: long side 14 trades 71% $14,500; all trades (with TD17 sells) 28, 67%, $25,825.",
            claim("14", "71%", "$1,035.71 ($14,500)", "2 days", "S&P 07/02/82-01/01/96")),
        sd!("lwnl_oct_td17_sell", "S&P October TD17 sell, exit 2 days, $1,500 stop", "Seasonals · trading day of month", "Short", Live,
            &["E|\"The sell date is the 17th trading day of the month with the same exit dates and targets\" (Sep 1996 p.3): Read: sell on the opening",
              "X|close two days after entry or a $1,500 stop (Sep 1996 p.3)"],
            "Printout: all trades 28 (67%, $25,825) minus long 14 ($14,500) = short 14 trades, 9 winners, $11,325 (derived).",
            claim("14", "64%", "$808.93 ($11,325, derived)", "2 days", "S&P 07/02/82-01/01/96")),
        sd!("lwnl_nov_td17_db7", "S&P November TD17 buy, 7-day Delayed Bailout, $3,000 stop", "Seasonals · trading day of month", "Long", Live,
            &["E|\"buy on the opening of the 17th trading day of November\" (Nov 1996 p.3)",
              "X|\"use a 7 day Delayed Bailout and $3,000 stop\" (Nov 1996 p.3); Delayed Bailout = \"waiting 9 days ... then exiting on any profitable opening after that point\" (Nov 1996 p.2)"],
            "Printout S&P 09/15/82-10/29/96 Test 6.", claim("13", "76%", "$2,492.31 ($32,400)", "", "S&P 09/15/82-10/29/96")),
        sd!("lwnl_jan97_td16_buy", "S&P January TD16 buy, 5-day exit, $1,800 stop", "Seasonals · 1997 special report", "Long", Live,
            &["E|\"buying the S&P on the opening of the 16th TRADING DAY OF THE MONTH\" [January] (Jan 1997 p.7): market on open (default since 23 Oct 2026)",
              "E|Option p.entry_pct = 0.0005: the special report's \"+/- 0.05% of the previous days range\" stop (Jan 1997 p.3), which the letter gives for the report's markets (the earlier default)",
              "X|\"The exit was $1,800 stop or MOC 5 days after entry\" (Jan 1997 p.7)"],
            SP, claim("13", "76%", "$1,715.38 ($22,300)", "5 days", "S&P (window not printed)")),
        sd!("lwnl_jan97_td2_sell", "S&P January TD2 sell, 5-day exit, $1,800 stop", "Seasonals · 1997 special report", "Short", Live,
            &["E|\"Selling on the opening of the second trade day of the year\" (Jan 1997 p.7): market on open (default since 23 Oct 2026); option p.entry_pct = 0.0005 = the report's open - 0.05% of the previous range (Jan 1997 p.3)",
              "X|the report's exit: \"either a $1,800 stop or getting out on the close of the 5th day after the trade was entered\" (Jan 1997 p.3)"],
            SP, claim("14", "50%", "$1,442.86 ($20,200)", "5 days", "S&P (window not printed)")),
        sd!("lwnl_jul97_td4_buy", "S&P July TD4 buy, 7-day hold, $1,500 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|\"buy at 7% of the previous days range added to the opening on the enumerated day\" (Jul 1997 p.2): S&P buy day 4", "X|\"a $1,500 stop or on the close 7 trading days later\" (Jul 1997 p.2)"],
            SP, claim("14", "64%", "$1,460 ($20,445)", "7 days", "S&P (letter Jul 1997)")),
        sd!("lwnl_jul97_td17_buy", "S&P July TD17 buy, 7-day hold, $1,500 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|open + 7% of the previous day's range on July TD17 (Jul 1997 p.2-3)", "X|$1,500 stop or the close 7 trading days later (Jul 1997 p.2)"],
            SP, claim("13", "61%", "$3,309 ($43,025)", "7 days", "S&P (letter Jul 1997)")),
        // Larry Williams block (23 Oct 2026 reading): the July 1997 30-day hold table, not coded before
        sd!("lwnl_jul97_td16_buy_30d", "S&P July TD16 buy, 30-day hold, $1,500 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|\"The entry rule is to buy at 7% of the previous days range added to the opening on the enumerated day\" (Jul 1997 p.2): '30 DAY HOLD BUYS' table, S&P day 16",
              "X|\"I also ran the best buys for a 30 trading day holding time period\" (Jul 1997 p.2): close 30 trading days after entry",
              "X|Read: the $1,500 stop of the same paragraph (the 30-day table prints no separate stop)"],
            SP, claim("13", "53%", "$2,655 ($34,525)", "30 days", "S&P (letter Jul 1997)")),
        sd!("lwnl_jul97_td14_sell", "S&P July TD14 short, bailout, $2,500 stop", "Seasonals · 1997 monthly lists", "Short", Live,
            &["E|\"selling short on the opening of the 14th trading day of the month\" (Jul 1997 p.3, 'the unknown seasonal trade of the month' in the July letter)",
              "X|\"exiting on the first profitable opening, with a $2,500 stop\" (Jul 1997 p.3)"],
            "Month = July: the item is the July issue's 'seasonal trade of the month'.", claim("15", "86%", "$864 ($12,960)", "1 day", "S&P (letter Jul 1997)")),
        sd!("lwnl_aug97_td12_buy_bail", "S&P August TD12 buy, bailout, $1,500 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|\"buy/sell on the opening of the trading day of the month shown ..., minus 7% of the previous days range\" (Aug 1997 p.2): literal = buy limit at open - 0.07 x range (p.entry_pct = +0.07 for a buy stop above the open as in the other months)",
              "X|\"We exit with a $1,500 loss or the Bail Out exit\" (Aug 1997 p.2)"],
            SP, claim("14", "78%", "$508 ($7,120)", "1 day", "S&P (letter Aug 1997)")),
        sd!("lwnl_aug97_td18_buy_bail", "S&P August TD18 buy, bailout, $1,500 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|August TD18, opening minus 7% of the previous range (Aug 1997 p.2)", "X|$1,500 stop or the Bail Out exit (Aug 1997 p.2)"],
            SP, claim("13", "76%", "$569 ($7,400)", "1 day", "S&P (letter Aug 1997)")),
        sd!("lwnl_aug97_td7_sell_bail", "S&P August TD7 sell, bailout, $1,500 stop", "Seasonals · 1997 monthly lists", "Short", Live,
            &["E|August TD7, sell stop at the opening minus 7% of the previous range (Aug 1997 p.2)", "X|$1,500 stop or the Bail Out exit (Aug 1997 p.2)"],
            SP, claim("12", "75%", "$527 ($6,325)", "1 day", "S&P (letter Aug 1997)")),
        sd!("lwnl_aug97_td18_buy_7d", "S&P August TD18 buy, 7-day hold, $1,500 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|\"Entry rules are the same as above\" (Aug 1997 p.3): August TD18, opening minus 7% of the previous range", "X|\"best 7 day holds with a $1,500 protective stop loss\" (Aug 1997 p.3)"],
            SP, claim("13", "61%", "$959 ($12,470)", "7 days", "S&P (letter Aug 1997)")),
        sd!("lwnl_aug97_td2_sell_7d", "S&P August TD2 sell, 7-day hold, $1,500 stop", "Seasonals · 1997 monthly lists", "Short", Live,
            &["E|August TD2, sell stop at the opening minus 7% of the previous range (Aug 1997 p.3)", "X|close 7 days later or the $1,500 stop (Aug 1997 p.3)"],
            SP, claim("14", "50%", "$988 ($13,845)", "7 days", "S&P (letter Aug 1997)")),
        sd!("lwnl_nov97_td17_buy_7d", "S&P November TD17 buy, 7-day hold, $2,000 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|\"the entry rule is to buy at that days opening plus 7% of the previous days range\" (Nov 1997 p.2): November TD17",
              "X|\"we automatically exited on the close of the 7th trading day after getting in\"; \"A stop loss of $1,200 was used on all trades except the S&P, which was $2,000\" (Nov 1997 p.2)"],
            SP, claim("15", "60%", "$1,788 ($26,825)", "7 days", "S&P (letter Nov 1997)")),
        sd!("lwnl_nov97_td18_buy_7d", "S&P November TD18 buy, 7-day hold, $2,000 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|November TD18, open + 7% of the previous range (Nov 1997 p.2-3)", "X|close of the 7th trading day after entry or the $2,000 stop (Nov 1997 p.2)"],
            SP, claim("14", "64%", "$1,798 ($25,175)", "7 days", "S&P (letter Nov 1997)")),
        sd!("lwnl_nov97_td6_buy_30d", "S&P November TD6 buy, 30-day hold, $2,000 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|November TD6, open + 7% of the previous range (Nov 1997 p.2-3)", "X|\"trades held for 30 days and exited\" (Nov 1997 p.3): close 30 trading days after entry; the $2,000 S&P stop (Nov 1997 p.2)"],
            SP, claim("15", "46%", "$2,163 ($32,450)", "30 days", "S&P (letter Nov 1997)")),
        sd!("lwnl_nov97_td15_buy_30d", "S&P November TD15 buy, 30-day hold, $2,000 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|November TD15, open + 7% of the previous range (Nov 1997 p.3)", "X|close 30 trading days after entry or the $2,000 stop (Nov 1997 p.2-3)"],
            SP, claim("13", "46%", "$1,680 ($21,825)", "30 days", "S&P (letter Nov 1997)")),
        sd!("lwnl_dec97_td2_buy_1d", "S&P December TD2 buy, 1-day hold, $3,500 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|\"same entry and exit rules as before, which means we bought on the opening of the trading day of the month shown\" (Dec 1997 p.2): open + 7% of the previous range (the rule 'as before', Nov 1997 p.2)",
              "X|\"exit on the close the number of days shown later\" (Dec 1997 p.2): 1 day; \"For the S&P 500 the best buys are day 2 and 14, the sells on day 9 with a $3,500 stop\" (Dec 1997 p.4)"],
            "The letter prints two different result tables for the same trades (p.3 and p.4).", claim("15", "80% / 93%", "$568 ($8,525) / $11,175", "1 day", "S&P (letter Dec 1997, p.3 / p.4)")),
        sd!("lwnl_dec97_td14_buy_1d", "S&P December TD14 buy, 1-day hold, $3,500 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|December TD14, open + 7% of the previous range (Dec 1997 p.2-4)", "X|close 1 day after entry or the $3,500 stop (Dec 1997 p.2, p.4)"],
            "Two tables: p.3 $11,300 11/14; p.4 $14,250 13/14.", claim("14", "78% / 92%", "$807 ($11,300) / $14,250", "1 day", "S&P (letter Dec 1997, p.3 / p.4)")),
        sd!("lwnl_dec97_td9_sell_1d", "S&P December TD9 sell, 1-day hold, $3,500 stop", "Seasonals · 1997 monthly lists", "Short", Live,
            &["E|December TD9, sell stop at open - 7% of the previous range (Dec 1997 p.2-4)", "X|close 1 day after entry or the $3,500 stop (Dec 1997 p.4)"],
            SP, claim("14", "64%", "$444 ($6,225)", "1 day", "S&P (letter Dec 1997)")),
        sd!("lwnl_yearend_sell", "S&P year-end short: open after December TD20, first profitable close or 3 days", "Seasonals · trading day of month", "Short", Live,
            &["E|\"sold the S&P 500 on the opening following the 20th trading day\" [of December] (Dec 1995 p.3)",
              "X|\"exited on the first profitable close or 3 days after entry\" (Dec 1995 p.3); no stop stated"],
            SP, claim("13", "92%", "$1,233.85 ($16,040)", "", "S&P (letter Dec 1995)")),
        // ---- Holidays
        sd!("lwnl_tg_close", "Thanksgiving: buy the close before, exit the close after", "Holidays", "Long", Live,
            &["E|\"bought on the close prior to the holiday\" (Nov 1992 p.3)", "X|\"exited the close the day after\" (Nov 1992 p.3); no stop"],
            "Book: subscriber's hand count since 1982.", claim("10", "60%", "+0.80 S&P points total", "1 day", "S&P 1982-1991")),
        sd!("lwnl_tg_open8", "Thanksgiving: buy the open 2 days before, exit 8 days later", "Holidays", "Long", Live,
            &["E|\"buy on the open of the trading session two days before the holiday\" (Oct 1994 p.3)", "X|\"and exit 8 days later\" (Oct 1994 p.3): the close of the 8th trading day after entry; no stop is stated"],
            "Printout S&P (03/94 data): 12 trades, 11 winners; the one loser lasted 2 bars, which suggests an unstated stop: none is added.",
            claim("12", "91%", "$2,110.42 ($25,325)", "8 days", "S&P to 03/94")),
        sd!("lwnl_tg_oc", "Thanksgiving: buy the open the day before, exit that close", "Holidays", "Long", Live,
            &["E|\"buy the S&P 500 on the open of the day prior to the holiday\" (Oct 1995 p.3)", "X|\"exit on that days close\" (Oct 1995 p.3)"],
            SP, claim("12", "83%", "$208 ($2,500)", "same day", "S&P (letter Oct 1995)")),
        sd!("lwnl_tg_open3_bail", "Thanksgiving: buy the open 3 days before, bailout or 9 days", "Holidays", "Long", Live,
            &["E|\"buying the S&P on the open 3 days before Thanksgiving\" (Oct 1995 p.3; 'This year one would buy on the opening of Monday, the 20th')", "X|\"exiting with my Bailout exit, or the close 9 days later\" (Oct 1995 p.3)"],
            SP, claim("12", "92%", "$739 ($8,878)", "", "S&P (letter Oct 1995)")),
        sd!("lwnl_xmas_bail", "Christmas: buy the close 5 days before, first profitable opening", "Holidays", "Long", Live,
            &["E|\"buy on the close 5 days before the holiday\" (Nov 1994 p.3)", "X|\"Take your profits on the first open that shows a profit\" (Nov 1994 p.3); no stop stated"],
            "Printout S&P INDEX 03/94 long trades.", claim("12", "100%", "$983.58 ($11,803)", "1 day", "S&P to 03/94")),
        sd!("lwnl_xmas_bail9", "Christmas: buy the close 5 days before, wait 9 days then bailout", "Holidays", "Long", Live,
            &["E|buy on the close 5 days before Christmas (Nov 1994 p.3)", "X|\"wait 9 days, then take the first profitable opening\" (Nov 1994 p.3)"],
            SP, claim("12", "100%", "$3,396 ($40,753)", "", "S&P (letter Nov 1994)")),
        sd!("lwnl_memorial_bail", "Memorial Day: buy the close 3 days before, first profitable opening", "Holidays", "Long", Live,
            &["E|\"In the S&P 500 buying on the close 3 days prior to the holiday\" (May 1995 p.4)", "X|\"exiting with the same rule\" = \"our first profitable opening rule\" (May 1995 p.4)"],
            SP, claim("12", "75%", "$746 ($8,953)", "", "S&P (letter May 1995)")),
        sd!("lwnl_memorial_bail6", "Memorial Day: buy the close 3 days before, wait 6 days then bailout", "Holidays", "Long", Live,
            &["E|buy on the close 3 days before Memorial Day (May 1995 p.4)", "X|\"wait 6 days, then go into the 'Bail Out' mode\" (May 1995 p.4)"],
            SP, claim("12", "100%", "$2,563 ($30,753)", "", "S&P (letter May 1995)")),
        sd!("lwnl_july4_sell", "July 4th: sell the open 3 days before, bailout from the 2nd day after", "Holidays", "Short", Live,
            &["E|\"sell on the open of the day 3 days before the holiday\" (Jun 1995 p.4)", "X|\"exit at the first profitable opening starting on the second trading day after the July 4th holiday\" (Jun 1995 p.4)"],
            "Printout: 12 trades, 100%, largest win $5,130, max intraday drawdown -$2,500.", claim("12", "100%", "$1,898.75 ($22,785)", "4 bars", "S&P (letter Jun 1995)")),
        sd!("lwnl_july4_buy", "July 4th: buy the open of the 2nd day after, first profitable opening", "Holidays", "Long", Live,
            &["E|\"Go long on the open of the second trading day after the holiday\" (Jun 1995 p.4)", "X|\"exit on the first profitable open\" (Jun 1995 p.4)"],
            SP, claim("13", "85% (11 of 13 years)", "", "", "S&P (letter Jun 1995)")),
        // ---- Patterns
        sd!("lwnl_pullback", "Buying pullbacks in an up-trending market", "Patterns", "Long", Live,
            &["F|\"1. Today's 20 day moving average is greater than the previous days 2. Both yesterdays close and today's are lower than the prior close\" (May 1993 p.2)",
              "E|\"Buy at 10% of yesterdays range added to the next days opening\" (May 1993 p.2): buy stop next day at its open + 0.10 x the set-up day's range",
              "X|\"a $1200 protective stop or an automatic exit on the third close after entry\" (May 1993 p.2)"],
            "Printout S&P 500 INDEX 06/92 Test 3 (start not printed). 20-day MA = simple MA of closes.",
            claim("70", "51%", "$438.21 ($30,675)", "3 days", "S&P to 06/92")),
        sd!("lwnl_krut_buy", "Krutsinger gap buy: high below the prior low, buy the open", "Patterns", "Long", Live,
            &["E|\"If today's high is less than yesterday's low, then buy tomorrow on the open\" (Jul 1991 p.2)",
              "X|\"It is best to give buy signals more time to work out ... exit on the 6th close following entry, and use a $3,000 stop\" (Jul 1991 p.2): close of the 6th day after entry; $3,000 = 6 S&P points at $500"],
            "Default since 23 Oct 2026 = the S&P stop of p.2 ($3,000). The $1,750 long stop is from the NIFE paragraph (p.3: \"I used a $1,750 stop for the longs and a $2,500 stop for the short sales\"), kept as the option p.stop_usd = 1750.",
            claim("53", "60%", "$666 ($35,325)", "6 days", "S&P (letter Jul 1991)")),
        sd!("lwnl_krut_sell", "Krutsinger gap sell: low above the prior high, sell the open", "Patterns", "Short", Live,
            &["E|\"If today's low is greater than yesterday's high, then sell tomorrow on the open\" (Jul 1991 p.2)",
              "X|\"exit sells on the close of the third day following entry. ie. If you sell on a Monday opening you would exit on the close Thursday. The best stop to use, in the past, has been a $2,000 stop\" (Jul 1991 p.2): $2,000 = 4 S&P points at $500"],
            "Default since 23 Oct 2026 = the S&P stop of p.2 ($2,000); the NIFE paragraph's $2,500 (p.3) is the option p.stop_usd = 2500.", claim("68", "61%", "$732 ($49,825)", "3 days", "S&P (letter Jul 1991)")),
        sd!("lwnl_od_buy", "Down-close outside day: buy at the high of the day before, exit next day", "Patterns", "Long", Live,
            &["F|Outside day = higher high and lower low than the previous day; '- close' = close below the previous close (Apr 1991 p.1-2)",
              "E|\"buying only on the day after the OD - close, at the high of the day prior to the OD\"; \"IF OPEN LESS THAN HIGH OF THAT DAY\" (Apr 1991 p.2-3): buy stop",
              "X|\"results exiting the next day\" (Apr 1991 p.2): Read: the close of the day after entry (the letters' 'X days later' exits are on the close); no stop"],
            "Table 'DAY 1' S&P: $25,275, 28 winners, 85% (about 33 trades), avg $902.", claim("33", "85%", "$902 ($25,275)", "1 day", "S&P (letter Apr 1991)")),
        sd!("lwnl_od_buy3", "Down-close outside day buy, exit 3 days later", "Patterns", "Long", Live,
            &["E|same entry as lwnl_od_buy (Apr 1991 p.2-3)", "X|\"BEST DAY 3\" for the S&P: the close of the 3rd day after entry"],
            "Table: S&P best day 3 $34,550, 28 winners, 78% (about 36 trades), avg $1,233.", claim("36", "78%", "$1,233 ($34,550)", "3 days", "S&P (letter Apr 1991)")),
        sd!("lwnl_od_sell", "Up-close outside day: sell at the low of the day before, exit next day", "Patterns", "Short", Live,
            &["E|\"A + close OD sets up a sell at the low of the day just prior\" (Apr 1991 p.2), open above that low", "X|exit the next day (Read: the close), as the buy table"],
            "\"The sell side in the S&P generated 26 trades and 21 winners, for a net profit of $8,750\" (Apr 1991 p.3).", claim("26", "81%", "$337 ($8,750)", "1 day", "S&P (letter Apr 1991)")),
        sd!("lwnl_od_fade_buy", "Open below a down-close outside day: buy at its low, exit on the close", "Patterns", "Long", Live,
            &["E|\"when the market opens below the low of a down close OD and you then buy at the OD low\" (Apr 1991 p.3): buy stop at the OD low", "X|\"Exit was on close of entry day\" (Apr 1991 p.3)"],
            SP, claim("14", "71%", "$479 ($6,700)", "same day", "S&P (letter Apr 1991)")),
        sd!("lwnl_od_fade_sell", "Open above an up-close outside day: sell at its high, exit on the close", "Patterns", "Short", Live,
            &["E|\"A sell is given with an open above the high of an outside day up close, and you then sell at the high of the OD\" (Apr 1991 p.3)", "X|close of the entry day (Apr 1991 p.3)"],
            "Only buys are tabulated in the letter.", None),
        sd!("lwnl_gap_fail_300", "S&P gap failure: 2.00-pt gap, enter 3.00 pts back through the open, exit on the close", "Patterns", "Long/Short", Live,
            &["F|\"THIS GAP SHOULD BE AT LEAST 200 POINTS\" from the previous day's close (Jan 1991 p.2): 200 points = 2.00 index points",
              "E|\"FOLLOWING A LOWER GAP OF 200 POINTS ... THEN BUY 300 POINTS ABOVE THE GAP\"; higher gap: sell 300 points below (Jan 1991 p.2): stop at open +/- 3.00",
              "F|\"if there is a gap of more than 10 full points, don't trade this method\" (Jan 1991 p.3)",
              "X|\"exiting on the close of entry day does produce similar results\" (Jan 1991 p.2)"],
            "The printed results use an exit the author calls 'not fully disclosable'; the coded exit is the close he names as giving similar results, so the printed numbers are only indicative.",
            claim("26 + 11", "84% / 90%", "$1,346 / $3,259", "same day", "S&P (letter Jan 1991, undisclosed exit)")),
        sd!("lwnl_gap_fail_525", "S&P gap failure: 2.00-pt gap, enter 5.25 pts back through the open", "Patterns", "Long/Short", Live,
            &["E|\"BUY/SELL WITH A 200 POINT GAP AND BUY/SELL 525 POINT +/- THE GAP\" (Jan 1991 p.2)", "F|no trade when the gap exceeds 10 points (Jan 1991 p.3)", "X|close of the entry day (Jan 1991 p.2)"],
            "Printed with the undisclosed exit.", claim("18", "100%", "$4,003 ($72,050)", "same day", "S&P (letter Jan 1991, undisclosed exit)")),
        sd!("lwnl_engulf_bull_1", "Bullish engulfing, buy above its high, exit 1 day later", "Patterns · candlestick test", "Long", Live, ENG_B,
            "Feb 1991 p.3 table (81 occurrences). Exit = the close 1 day after entry.", claim("81 patterns", "", "-$20,300 total", "1 day", "S&P (letter Feb 1991)")),
        sd!("lwnl_engulf_bull_5", "Bullish engulfing, buy above its high, exit 5 days later", "Patterns · candlestick test", "Long", Live, ENG_B,
            "Exit = the close 5 days after entry.", claim("81 patterns", "", "-$28,050 total", "5 days", "S&P (letter Feb 1991)")),
        sd!("lwnl_engulf_bull_10", "Bullish engulfing, buy above its high, exit 10 days later", "Patterns · candlestick test", "Long", Live, ENG_B,
            "Exit = the close 10 days after entry.", claim("81 patterns", "", "-$24,550 total", "10 days", "S&P (letter Feb 1991)")),
        sd!("lwnl_engulf_bear_1", "Bearish engulfing, sell below its low, exit 1 day later", "Patterns · candlestick test", "Short", Live, ENG_S,
            "Feb 1991 p.3 table. Exit = the close 1 day after entry.", claim("", "", "-$12,350 total", "1 day", "S&P (letter Feb 1991)")),
        sd!("lwnl_engulf_bear_5", "Bearish engulfing, sell below its low, exit 5 days later", "Patterns · candlestick test", "Short", Live, ENG_S,
            "Exit = the close 5 days after entry.", claim("", "", "-$44,550 total", "5 days", "S&P (letter Feb 1991)")),
        sd!("lwnl_engulf_bear_10", "Bearish engulfing, sell below its low, exit 10 days later", "Patterns · candlestick test", "Short", Live, ENG_S,
            "Exit = the close 10 days after entry.", claim("", "", "-$50,350 total", "10 days", "S&P (letter Feb 1991)")),
        sd!("lwnl_keyrev_buy", "Key reversal buy (bearish reversal above the high 15 days back)", "Patterns", "Long", Live,
            &["F|\"IF LOW > HIGH (VARA) AND HIGH > HIGH (1) AND C < C (1) THEN BUY TOMORROW AT H (1) STOP\" (Oct 1993 p.3, System Writer code; VARA = 15 for the S&P)",
              "E|buy stop the next day at the high of the day before the reversal day (Oct 1993 p.2-3)", "X|\"The exit is ... the Bail Out Stop\" (Oct 1993 p.2): first profitable opening; no money stop stated"],
            "The prose also asks for a higher low (L > L[1]); the code, which wins, does not: p.strict=1 adds it.",
            claim("59", "77%", "$226 ($13,325)", "", "S&P 1982-1993")),
        sd!("lwnl_keyrev_sell", "Key reversal sell (bullish reversal below the low 15 days back)", "Patterns", "Short", Live,
            &["F|mirror of the code: HIGH < LOW(VARA) AND LOW < LOW(1) AND C > C(1) (Oct 1993 p.2: 'lower low, lower high, higher close ... whose high is less than the low of N days back', N usually 15)",
              "E|\"sell the day after the reversal at the low of the day prior to the reversal\" (Oct 1993 p.2)", "X|Bail Out: first profitable opening (Oct 1993 p.2)"],
            "Prose also asks for a lower high (H < H[1]); p.strict=1 adds it.", claim("59", "77%", "$226 ($13,325)", "", "S&P 1982-1993")),
        sd!("lwnl_proj_contra", "Projected high/low contrarian: open beyond the projection, enter back at it (Mon-Wed)", "Patterns", "Long/Short", Live,
            &["F|\"PROJECTED HIGH = Todays High+Low+Close /3 x 2 - Todays Low; PROJECTED LOW = Todays High+Low+Close /3 X 2 - Todays High\" (Sep 1995 p.2)",
              "E|\"If the open tomorrow is less than the projected low from the preceding day I will then buy at the projected low figure. ... if the open tomorrow is greater than the projected high ... I will sell should prices come back to that value\" (Sep 1995 p.3)",
              "F|\"buys the S&P 500 only on Monday, Tuesday and Wednesday ... The S&P are to be sold ... only on Monday, Tuesday and Wednesday\" (Sep 1995 p.3)",
              "X|\"The exit is my old buddy Bail Out with ... $1,750 in the S&P\" (Sep 1995 p.3)"],
            "Printout S&P 1982-1994: long 71 trades 84% $58,880; short 54 trades 83% $30,095.", claim("125", "84%", "$711.80 ($88,975)", "1 day", "S&P 1982-1994")),
        sd!("lwnl_proj_limit", "Projected low limit buy, exit on the close (conventional use)", "Patterns", "Long", Live,
            &["E|\"buy at the projected low, on a limit or better basis, if the opening tomorrow was above the projected low figure\" (Sep 1995 p.2)", "X|\"Our exit was moc at the end of the day\" (Sep 1995 p.2)"],
            SP, claim("1,113", "47%", "-$77.73 (-$86,510)", "same day", "S&P (letter Sep 1995)")),
        sd!("lwnl_truth1_closehigh", "Close on the high with an up close: buy the close, exit 5 days later", "Patterns", "Long", Live,
            &["F|\"If price today closed on the high of the day---and the day was an up close\" (Jun 1992 p.2): Close = High and Close > previous close",
              "E|\"bought on that close\" (Jun 1992 p.2)", "X|S&P row: exit day 5, stop 500 (Jun 1992 p.2-3): close 5 days after entry, $500 stop"],
            SP, claim("5", "", "$1,335 ($6,675)", "5 days", "S&P (letter Jun 1992)")),
        sd!("lwnl_ndx_vb_oops", "Nasdaq-100: open + 35% TR buy, open - 60% TR sell, Oops! buy; bailout, $750 stop", "Patterns", "Long/Short", Live,
            &["E|\"buy at 35% of yesterdays true range added to tomorrow's opening, or sell at 60% of the true range subtracted from the opening\" (Sep 1996 p.1-2)",
              "E|\"OOPS! ... if price opens below yesterdays low, then buy ... at yesterdays low on a stop\" (Sep 1996 p.2)",
              "X|\"get out on the first profitable opening, or be stopped out\"; \"the best stop to use is about $750\" (Sep 1996 p.1)"],
            "Book market: Nasdaq-100 futures (new in 1996, $100 x index: p.pt_usd = 100). 'about $750' coded as $750. One position at a time, no reversal (none stated).",
            claim("52", "84%", "$834.19 ($43,376)", "1 day", "NDX futures 1996 (a few months)")),
        sd!("lwnl_tdw_oc_tue", "Buy Tuesday's open, exit the close", "Day of week (1988-1993 study)", "Long", Live, TDW_R,
            "Jul 1993 p.3, S&P since 1/1/88: Tue +$5,575, 45%. Monday (+$64,600, 55%) is lwlts_mon_oc.", claim("", "45%", "+$5,575 total", "same day", "S&P 1/1/88-mid 1993")),
        sd!("lwnl_tdw_oc_wed", "Buy Wednesday's open, exit the close", "Day of week (1988-1993 study)", "Long", Live, TDW_R,
            "S&P since 1/1/88: Wed +$11,575, 51%.", claim("", "51%", "+$11,575 total", "same day", "S&P 1/1/88-mid 1993")),
        sd!("lwnl_tdw_oc_thu", "Buy Thursday's open, exit the close", "Day of week (1988-1993 study)", "Long", Live, TDW_R,
            "S&P since 1/1/88: Thu -$28,475, 44%.", claim("", "44%", "-$28,475 total", "same day", "S&P 1/1/88-mid 1993")),
        sd!("lwnl_tdw_oc_fri", "Buy Friday's open, exit the close", "Day of week (1988-1993 study)", "Long", Live, TDW_R,
            "S&P since 1/1/88: Fri -$44,950, 51%.", claim("", "51%", "-$44,950 total", "same day", "S&P 1/1/88-mid 1993")),
        // ---- Needs data
        sd!("lwnl_bond_sp_systems", "Bond-driven S&P systems", "Needs data", "Long", NeedsData(BOND),
            &["E|Buy the S&P on the close when Bonds close above their 14-day highest high; exit a 10-day lowest low in Bonds or $3,000 stop (Apr 1993 p.1-3; 66 trades 50% $112,175), or the S&P 5-day low (94 trades 41% $96,600)",
              "E|9-day MA of Bonds rising + 1 or 2 S&P down closes: buy open + 20% of the last range, $2,500 stop or close 2 days later (Sep 1994 p.3-4; 308 trades 59% $116,900 / 145 trades 58% $43,875)",
              "E|Weekly 14-bar Bond stochastic as an S&P timer (Feb 1992, Jul 1992, Nov 1992, Dec 1993): no exit given",
              "E|Hornbrook: British Pound, crude and gold closes as an S&P filter, buy open exit close (May 1996 p.3): needs BP, crude, gold futures"],
            "All need bond (or other) futures alongside the S&P."),
        sd!("lwnl_bond_systems", "T-Bond systems and seasonals", "Needs data", "Long/Short", NeedsData(BOND),
            &["E|Close > close 8 days ago, $850 stop, ~3x target (Aug 1991 p.2) = lwlts_bonds_8day",
              "E|CTI bond breakout: high + 60% range buy, low - 90% range sell, stops $3,250 / $750 (Jun 1993 p.2-3; Jul 1993 p.2)",
              "E|Bond holiday, TDOM and day-of-month trades: Thanksgiving, Christmas, Memorial, July 4th, Labor Day, Jan TD18, Dec TD19, Oct 29th, Nov 3-8, May TD9, July TD12, Sep, Oct, Nov, Dec tables (1993-1997 letters)",
              "E|Projected low/high contrarian on Bonds (Sep 1995 p.3: 352 trades 73% $71,472.50), DOWM buys (May 1996 p.2)"],
            ""),
        sd!("lwnl_commodity_systems", "Commodity and currency systems and seasonals", "Needs data", "Long/Short", NeedsData(FUT),
            &["E|Larry's Cup of Coffee: open + 50% / - 40% of the 4-day average range, $1,750 stop or 6th close (Mar 1992 p.3; 2,117 trades $535,456)",
              "E|13x13 gold channel (May 1993 p.1-2); gold TDOM1 > close 4 days ago, 12-day exit (Mar 1995 p.3-4); gold Nov TD16, Sep TD1/2 (Oct 1995, Aug 1996)",
              "E|Monthly seasonal lists for cattle, bellies, cotton, coffee, cocoa, beans, wheat, corn, sugar, OJ, silver, copper, lumber, heating oil, currencies (Sep 1993 - Dec 1997)",
              "E|Delayed-bailout trades in bellies, yen, gold (Nov 1996 p.2-3)"],
            ""),
        // ---- Not testable
        sd!("lwnl_no_exit", "Rules printed without an exit", "Not testable", "Long/Short",
            NotTestable("The entry is exact but the letter gives no exit (or calls it undisclosed), and an exit may not be invented."),
            &["E|- + + + - closes then a higher open: buy stop at the high of the last + day (Mar 1991 p.3)",
              "E|Double inside days, inside day + lower open, inside day up close below the high 9 days ago, inside day after a down close (Aug 1991 p.2-3)",
              "E|Oops! by day of week (Dec 1991 p.2-3; Sep 1991 p.3), fake-out day (Sep 1991 p.2-3: exit taught at seminars)",
              "E|Monday buy / Friday sell at projected levels, Thursday/Friday sells (Apr 1994 p.2-3)",
              "E|S&P December TDOM 23/2 with $1,250 stop (Nov 1995 p.3: entry order not stated), July TD4 (Jun 1996 p.3: entry not stated)",
              "E|September 1997 best S&P days (TD10/TD20 buys, TD3/TD17 sells, $2,750 stop: exit not stated)",
              "E|Best TDOM table (Jan 1995 p.3), Levin engulfing test (Dec 1997 p.1-2), two undisclosed S&P models (Jan 1997 p.2)"],
            "Truth #2 'buy new X-day highs' (Jun 1992 p.3) is lwlts_hh (S&P X=5, exit 5, no stop: p.hold=5, p.stop_usd=0)."),
        sd!("lwnl_unquantified", "Rules with a stated but unquantified part", "Not testable", "Long/Short",
            NotTestable("A stop is stated but its size is blank or missing, or the order type is not given, so the rule cannot be coded as written."),
            &["X|February 1997 5-day seasonals: \"A $    stop was used\" (amount left blank, Feb 1997 p.3); S&P TD3 buy 6/14 $20,000",
              "X|March 1997 5-day seasonals: no stop given (Mar 1997 p.3-4); S&P TD10 buy, TD19 sell",
              "X|May / June 1997 lists: entry order and stop not given (May 1997 p.3-4, Jun 1997 p.2)",
              "X|October 1997 lists: no S&P stop given; one-day trades use 'Better Bailout' with a stop that is 'the reverse' of the rule (Oct 1997 p.2-4); November 1997 one-day trades: \"The exit is (PICK UP FROM LAST MONTH.)\""],
            ""),
        sd!("lwnl_judgement", "Indicators, sentiment and chart reading", "Not testable", "Long/Short",
            NotTestable("Judgement: divergences, trend lines, swing points or set-ups without entry and exit rules."),
            &["F|Public/pro X-ray lines (Oct 1991, Oct 1992), A/D divergences, sentiment and COT index (1992-1997), Williams Sentiment Approximation Index (Jun 1994 p.3)",
              "F|Megaphone, Zero Balance, Fibonacci time, 1.28 projection, Freight Train, Specialist's Trap (= lwlts_specialist_trap), VanDiv, weekly ADX, holding trends (1992-1996)",
              "F|Presidential cycle, January barometer, P/E and yield allocation, spike charts, put/call ratios, spreads"],
            ""),
    ]
}

const ENG_B: &[&str] = &["F|Bullish engulfing: \"a day opens below the previous day's close and closes above that day's open, and that day has a lower close than open\" (Feb 1991 p.3): O < C[1], C > O[1], C[1] < O[1]",
    "E|\"buy the next day on a break-out above the previous day's high\" (Feb 1991 p.2): buy stop at the pattern day's high, next day only",
    "X|\"exit the trade 1 through 10 days later\" (Feb 1991 p.2): the close N days after entry (table: 1, 5, 10); no stop"];
const ENG_S: &[&str] = &["F|Bearish engulfing (mirror): O > C[1], C < O[1], C[1] > O[1] (Feb 1991 p.3)",
    "E|\"a sell below the previous day's low (if a bearish pattern)\" (Feb 1991 p.2): sell stop at the pattern day's low, next day only",
    "X|the close N days after entry (1, 5, 10); no stop"];
const TDW_R: &[&str] = &["E|\"buy on the open and exit on the close\" each day of the week (Jul 1993 p.3)", "X|close of the entry day; no stop",
    "F|The author: \"Please, don't go buying ... use as a bias\" (Jul 1993 p.3)"];

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("lwnl_") { return None; }
    let f = |fam: &'static str, name: &'static str, v: &'static str| Some((fam, name, v));
    if SEA.iter().any(|s| s.0 == id) { return f("lwnl_seasonal", "Trading-day-of-month seasonals", "S&P"); }
    if HOL.iter().any(|s| s.0 == id) { return f("lwnl_holiday", "Holiday trades", "S&P"); }
    match id {
        "lwnl_krut_buy" | "lwnl_krut_sell" => f("lwnl_krut", "Krutsinger gap", ""),
        "lwnl_od_buy" | "lwnl_od_buy3" | "lwnl_od_sell" | "lwnl_od_fade_buy" | "lwnl_od_fade_sell" => f("lwnl_od", "Outside days", ""),
        "lwnl_gap_fail_300" | "lwnl_gap_fail_525" => f("lwnl_gap_fail", "Gap failure", ""),
        "lwnl_keyrev_buy" | "lwnl_keyrev_sell" => f("lwnl_keyrev", "Key reversals", ""),
        "lwnl_proj_contra" | "lwnl_proj_limit" => f("lwnl_proj", "Projected high/low", ""),
        s if s.starts_with("lwnl_engulf") => f("lwnl_engulf", "Engulfing candlestick test", ""),
        s if s.starts_with("lwnl_tdw_oc") => f("lwnl_tdw", "Day of week open-to-close", ""),
        "lwnl_bond_sp_systems" | "lwnl_bond_systems" | "lwnl_commodity_systems" => f("lwnl_needs_data", "Futures systems", "Needs data"),
        "lwnl_no_exit" | "lwnl_unquantified" | "lwnl_judgement" => f("lwnl_not_testable", "Not testable", ""),
        _ => f("lwnl_patterns", "Newsletter patterns", ""),
    }
}

fn p(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef { ParamDef { key, label, def, min, max, step } }
fn stop(v: f64) -> ParamDef { p("stop_usd", "Protective stop in $ (0 = none)", v, 0.0, 10000.0, 250.0) }
fn pt(v: f64) -> ParamDef { p("pt_usd", "$ per index point (S&P futures 500; NDX 100)", v, 1.0, 1000.0, 1.0) }
fn day0() -> ParamDef { p("day0", "Stop armed on the entry day (1) or from the next day (0)", 1.0, 0.0, 1.0, 1.0) }
fn hold(v: f64) -> ParamDef { p("hold", "Exit on the close N trading days after entry", v, 0.0, 40.0, 1.0) }
fn bail(v: f64) -> ParamDef { p("bail_delay", "Bailout: first profitable opening after N days held", v, 0.0, 15.0, 1.0) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if let Some(s) = SEA.iter().find(|s| s.0 == id) {
        let mut v = vec![p("td", "Trading day of the month", s.2 as f64, 1.0, 23.0, 1.0),
            p("entry_pct", "Entry: open +/- x previous day's range (0 = at the open; + = stop in trade direction; - = limit)", s.4, -0.5, 0.5, 0.01),
            stop(s.5), pt(500.0), day0()];
        if let Some(h) = s.6 { v.push(hold(h as f64)); }
        if let Some(b) = s.7 { v.push(bail(b as f64)); }
        if s.8 { v.push(p("repeat", "Later days of the month: 0 = TD only, 1 = order stands until it fills, 2 = re-enter every day while flat",
            if id == "lwnl_mar_short" { 2.0 } else { 1.0 }, 0.0, 2.0, 1.0)); }
        return Some(v);
    }
    if let Some(s) = HOL.iter().find(|s| s.0 == id) {
        let mut v = vec![p("k", "Trading day relative to the holiday (1 = day before, -1 = day after)", s.2 as f64, -10.0, 10.0, 1.0), stop(s.5), pt(500.0), day0()];
        if let Some(h) = s.6 { v.push(hold(h as f64)); }
        if let Some(b) = s.7 { v.push(bail(b as f64)); }
        return Some(v);
    }
    Some(match id {
        "lwnl_yearend_sell" => vec![p("td", "Sell the open after this December trading day", 20.0, 15.0, 23.0, 1.0), hold(3.0),
            p("prof_close", "Exit on the first profitable close", 1.0, 0.0, 1.0, 1.0), stop(0.0), pt(500.0), day0()],
        "lwnl_pullback" => vec![p("ma", "Moving average length", 20.0, 5.0, 50.0, 1.0), p("entry_pct", "Buy stop: open + x set-up day's range", 0.10, 0.0, 1.0, 0.05), stop(1200.0), hold(3.0), pt(500.0), day0()],
        "lwnl_krut_buy" => vec![stop(3000.0), hold(6.0), pt(500.0), day0()],
        "lwnl_krut_sell" => vec![stop(2000.0), hold(3.0), pt(500.0), day0()],
        "lwnl_od_buy" | "lwnl_od_sell" => vec![hold(1.0), stop(0.0), pt(500.0), day0()],
        "lwnl_od_buy3" => vec![hold(3.0), stop(0.0), pt(500.0), day0()],
        "lwnl_od_fade_buy" | "lwnl_od_fade_sell" => vec![hold(0.0), stop(0.0), pt(500.0), day0()],
        "lwnl_gap_fail_300" | "lwnl_gap_fail_525" => vec![p("gap", "Minimum gap from the previous close (index points)", 2.0, 0.5, 20.0, 0.25),
            p("back", "Entry distance back through the open (index points)", if id.ends_with("525") { 5.25 } else { 3.0 }, 0.5, 20.0, 0.25),
            p("max_gap", "No trade if the gap exceeds (index points; 0 = no limit)", 10.0, 0.0, 100.0, 1.0), hold(0.0), stop(0.0), pt(500.0), day0()],
        s if s.starts_with("lwnl_engulf") => vec![hold(s.rsplit('_').next()?.parse().ok()?), stop(0.0), pt(500.0), day0()],
        "lwnl_keyrev_buy" | "lwnl_keyrev_sell" => vec![p("n", "Look-back N (VARA)", 15.0, 3.0, 30.0, 1.0), p("strict", "Also require the prose's higher low / lower high (1)", 0.0, 0.0, 1.0, 1.0), bail(0.0), stop(0.0), pt(500.0), day0()],
        "lwnl_proj_contra" => vec![p("dows", "Weekdays allowed (bitmask Mon=1 Tue=2 Wed=4 Thu=8 Fri=16)", 7.0, 1.0, 31.0, 1.0), bail(0.0), stop(1750.0), pt(500.0), day0()],
        "lwnl_proj_limit" => vec![hold(0.0), stop(0.0), pt(500.0), day0()],
        "lwnl_truth1_closehigh" => vec![hold(5.0), stop(500.0), pt(500.0), p("day0", "Stop armed on the entry day (entry is on the close: 0)", 0.0, 0.0, 1.0, 1.0)],
        "lwnl_ndx_vb_oops" => vec![p("buy_pct", "Buy stop: open + x yesterday's true range", 0.35, 0.0, 2.0, 0.05), p("sell_pct", "Sell stop: open - x yesterday's true range", 0.60, 0.0, 2.0, 0.05),
            p("oops", "Oops! buy at yesterday's low after a lower open (1)", 1.0, 0.0, 1.0, 1.0), bail(0.0), stop(750.0), pt(100.0), day0()],
        s if s.starts_with("lwnl_tdw_oc") => vec![hold(0.0), stop(0.0), pt(500.0), day0()],
        _ => return None,
    })
}

// ------------------------------------------------------------------ simulator

#[derive(Clone, Copy, Debug)]
pub(crate) enum E { Open, Close, Stop(f64), Limit(f64) }

#[derive(Clone, Copy)]
pub(crate) struct Cfg {
    pub(crate) stop: f64,           // protective stop distance in points (NaN or <= 0 = none)
    pub(crate) day0: bool,          // stop armed on the entry bar (entries at the open / intraday)
    pub(crate) bail: Option<usize>, // exit at the first profitable opening of a bar > entry bar + k
    pub(crate) prof_close: bool,    // exit at the first profitable close after the entry bar
    pub(crate) hold: Option<usize>, // exit on the close `hold` bars after the entry bar (0 = same day)
}

struct Pos { dir: f64, i: usize, px: f64, stop: f64, kind: &'static str }

fn done(p: &Pos, i: usize, px: f64, why: &str) -> Trade {
    Trade { dir: p.dir, legs: vec![Leg { i: p.i, px: p.px, w: 1.0 }], exit_i: i, exit_px: px, reason: why.into(),
        entry_kind: p.kind, signal_i: i, open: false, exits: vec![] }
}

/// One position at a time. `sig(i)` gives bar i's entry orders as (dir, order), decided from bars before i and bar i's open.
/// Order of events in a bar: (1) at the open, bailout and stops gapped through; (2) market-on-open orders and stop / limit
/// orders the open has already passed fill at the open; (3) along the path open -> nearer extreme -> other extreme -> close,
/// stop / limit entries and the protective stop fill in the order the path reaches them (no new entry after an intrabar
/// exit); (4) at the close, time exit, first profitable close, then market-on-close entries.
pub(crate) fn sim(x: &Ctx, from: usize, to: usize, cfg: &Cfg, sig: impl Fn(usize) -> Vec<(f64, E)>) -> Vec<Trade> {
    let mut out = vec![];
    let mut pos: Option<Pos> = None;
    let sd = if cfg.stop.is_finite() && cfg.stop > 0.0 { cfg.stop } else { f64::NAN };
    let mk = |dir: f64, i: usize, px: f64, kind: &'static str| Pos { dir, i, px, stop: px - dir * sd, kind };
    for i in from.max(1)..=to {
        let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        let mut exited = false;
        // 1. at the open
        if let Some(p) = &pos {
            let why = if cfg.bail.map_or(false, |k| i > p.i + k) && p.dir * (o - p.px) > 0.0 { Some("bailout: first profitable opening") }
                else if p.stop.is_finite() && p.dir * (o - p.stop) <= 0.0 { Some("stop (gap at the open)") } else { None };
            if let Some(w) = why { out.push(done(p, i, o, w)); pos = None; }
        }
        let orders = sig(i);
        // 2. fills at the open
        if pos.is_none() {
            for &(dir, od) in &orders {
                let at_open = match od {
                    E::Open => true,
                    E::Stop(lv) => dir * (o - lv) >= 0.0,
                    E::Limit(lv) => dir * (lv - o) >= 0.0,
                    E::Close => false,
                };
                if at_open { pos = Some(mk(dir, i, o, if matches!(od, E::Open) { "open" } else { "stop" })); break; }
            }
        }
        // 3. intrabar path
        let path = if h - o <= o - l { [o, h, l, c] } else { [o, l, h, c] };
        for s in 0..3 {
            let (mut cur, end) = (path[s], path[s + 1]);
            let up = end > cur;
            loop {
                // candidate events: (level, Some(dir, kind)) = entry, (level, None) = protective stop
                let mut best: Option<(f64, Option<(f64, &'static str)>)> = None;
                let inside = |lv: f64| lv.is_finite() && if up { lv > cur && lv <= end } else { lv < cur && lv >= end };
                let mut take = |lv: f64, ev: Option<(f64, &'static str)>| {
                    if inside(lv) && best.map_or(true, |(b, _)| if up { lv < b } else { lv > b }) { best = Some((lv, ev)); }
                };
                match &pos {
                    Some(p) => {
                        let against = (p.dir > 0.0) != up;
                        if against && (p.i < i || cfg.day0) { take(p.stop, None); }
                    }
                    None if !exited => {
                        for &(dir, od) in &orders {
                            match od {
                                E::Stop(lv) if (dir > 0.0) == up => take(lv, Some((dir, "stop"))),
                                E::Limit(lv) if (dir > 0.0) != up => take(lv, Some((dir, "limit"))),
                                _ => {}
                            }
                        }
                    }
                    None => {}
                }
                let Some((lv, ev)) = best else { break };
                match ev {
                    Some((dir, kind)) => pos = Some(mk(dir, i, lv, kind)),
                    None => { let p = pos.take().unwrap(); out.push(done(&p, i, lv, "stop")); exited = true; }
                }
                cur = lv;
            }
        }
        // 4. at the close
        if let Some(p) = &pos {
            let why = if cfg.hold.map_or(false, |n| i >= p.i + n) { Some("time exit on the close") }
                else if cfg.prof_close && i > p.i && p.dir * (c - p.px) > 0.0 { Some("first profitable close") } else { None };
            if let Some(w) = why { out.push(done(p, i, c, w)); pos = None; exited = true; }
        }
        if pos.is_none() && !exited {
            if let Some(&(dir, _)) = orders.iter().find(|(_, od)| matches!(od, E::Close)) { pos = Some(mk(dir, i, c, "close")); }
        }
    }
    if let Some(p) = pos {
        let mut t = done(&p, to, x.c[to], "open");
        t.open = true;
        out.push(t);
    }
    out
}

/// Days since 1970-01-01 of "YYYY-MM-DD".
fn days(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// Entry order at bar i: open +/- pct x previous day's range (pct > 0 = stop in the trade direction, < 0 = limit, 0 = open).
fn rng_order(x: &Ctx, i: usize, dir: f64, pct: f64) -> E {
    let lv = x.o[i] + dir * pct * (x.h[i - 1] - x.l[i - 1]);
    if pct > 0.0 { E::Stop(lv) } else if pct < 0.0 { E::Limit(lv) } else { E::Open }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = getter(&schema, p);
    let has = |k: &str| schema.iter().any(|d| d.key == k);
    let opt = |k: &str| if has(k) && g(k) >= 0.0 { Some(g(k).round() as usize) } else { None };
    let cfg = Cfg { stop: g("stop_usd") / g("pt_usd"), day0: g("day0") >= 0.5, bail: opt("bail_delay"), hold: opt("hold"),
        prof_close: has("prof_close") && g("prof_close") >= 0.5 };
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = x.r.len();
    let mon = |i: usize| -> u32 { x.r[i].date[5..7].parse().unwrap_or(0) };
    let no: Vec<(f64, E)> = vec![];

    if let Some(s) = SEA.iter().find(|s| s.0 == id) {
        let td = tdm(x);
        let (k, pct, dir, m) = (g("td").round() as usize, g("entry_pct"), s.3, s.1);
        let rep = if has("repeat") { g("repeat").round() as i32 } else { 0 };
        // repeat 1: the order stands from TD k until the first day it fills (one trade a month); 2: every day from TD k while flat
        let mut on = vec![false; n];
        let mut filled_month = String::new();
        for i in 1..n {
            if mon(i) != m || td[i] < k || (td[i] > k && rep == 0) { continue; }
            if rep == 1 {
                if x.r[i].date[..7] == filled_month[..] { continue; }
                let fills = match rng_order(x, i, dir, pct) {
                    E::Stop(lv) => if dir > 0.0 { h[i] >= lv } else { l[i] <= lv },
                    E::Limit(lv) => if dir > 0.0 { l[i] <= lv } else { h[i] >= lv },
                    _ => true,
                };
                if fills { filled_month = x.r[i].date[..7].to_string(); }
            }
            on[i] = true;
        }
        return Some(sim(x, from, to, &cfg, |i| if on[i] { vec![(dir, rng_order(x, i, dir, pct))] } else { no.clone() }));
    }
    if let Some(s) = HOL.iter().find(|s| s.0 == id) {
        let k = g("k").round() as i32;
        let mut on = vec![false; n];
        let (y0, y1): (i32, i32) = (x.r[0].date[..4].parse().ok()?, x.r[n - 1].date[..4].parse().ok()?);
        for y in y0..=y1 {
            let hd = holiday(s.1, y);
            if let Some(j) = td_rel(x, &hd, k) {
                // the bar must be within 15 calendar days of the holiday (no stale bars across data gaps)
                if (days(&x.r[j].date) - days(&hd)).abs() <= 15 { on[j] = true; }
            }
        }
        let od = if s.4 { E::Close } else { E::Open };
        return Some(sim(x, from, to, &cfg, |i| if on[i] { vec![(s.3, od)] } else { no.clone() }));
    }
    let t = match id {
        "lwnl_yearend_sell" => {
            let td = tdm(x);
            let k = g("td").round() as usize;
            sim(x, from, to, &cfg, |i| if mon(i - 1) == 12 && td[i - 1] == k { vec![(-1.0, E::Open)] } else { no.clone() })
        }
        "lwnl_pullback" => {
            let ma = sma(c, g("ma").round() as usize);
            let pct = g("entry_pct");
            sim(x, from, to, &cfg, |i| {
                if i < 3 { return no.clone(); }
                let s = i - 1;
                if ma[s] > ma[s - 1] && c[s - 1] < c[s - 2] && c[s] < c[s - 1] { vec![(1.0, E::Stop(o[i] + pct * (h[s] - l[s])))] } else { no.clone() }
            })
        }
        "lwnl_krut_buy" => sim(x, from, to, &cfg, |i| if i >= 2 && h[i - 1] < l[i - 2] { vec![(1.0, E::Open)] } else { no.clone() }),
        "lwnl_krut_sell" => sim(x, from, to, &cfg, |i| if i >= 2 && l[i - 1] > h[i - 2] { vec![(-1.0, E::Open)] } else { no.clone() }),
        "lwnl_od_buy" | "lwnl_od_buy3" | "lwnl_od_sell" | "lwnl_od_fade_buy" | "lwnl_od_fade_sell" => {
            let buy = id.contains("buy");
            let fade = id.contains("fade");
            sim(x, from, to, &cfg, |i| {
                if i < 3 { return no.clone(); }
                let s = i - 1;
                let od = h[s] > h[s - 1] && l[s] < l[s - 1];
                let dn = c[s] < c[s - 1];
                let up = c[s] > c[s - 1];
                if !od { return no.clone(); }
                match (buy, fade) {
                    (true, false) if dn && o[i] < h[s - 1] => vec![(1.0, E::Stop(h[s - 1]))],
                    (false, false) if up && o[i] > l[s - 1] => vec![(-1.0, E::Stop(l[s - 1]))],
                    (true, true) if dn && o[i] < l[s] => vec![(1.0, E::Stop(l[s]))],
                    (false, true) if up && o[i] > h[s] => vec![(-1.0, E::Stop(h[s]))],
                    _ => no.clone(),
                }
            })
        }
        "lwnl_gap_fail_300" | "lwnl_gap_fail_525" => {
            let (gap, back, mx) = (g("gap"), g("back"), g("max_gap"));
            sim(x, from, to, &cfg, |i| {
                let gp = o[i] - c[i - 1];
                if mx > 0.0 && gp.abs() > mx { return no.clone(); }
                if gp <= -gap { vec![(1.0, E::Stop(o[i] + back))] } else if gp >= gap { vec![(-1.0, E::Stop(o[i] - back))] } else { no.clone() }
            })
        }
        s if s.starts_with("lwnl_engulf_bull") => sim(x, from, to, &cfg, |i| {
            if i < 3 { return no.clone(); }
            let s = i - 1;
            if o[s] < c[s - 1] && c[s] > o[s - 1] && c[s - 1] < o[s - 1] { vec![(1.0, E::Stop(h[s]))] } else { no.clone() }
        }),
        s if s.starts_with("lwnl_engulf_bear") => sim(x, from, to, &cfg, |i| {
            if i < 3 { return no.clone(); }
            let s = i - 1;
            if o[s] > c[s - 1] && c[s] < o[s - 1] && c[s - 1] > o[s - 1] { vec![(-1.0, E::Stop(l[s]))] } else { no.clone() }
        }),
        "lwnl_keyrev_buy" | "lwnl_keyrev_sell" => {
            let k = g("n").round() as usize;
            let strict = g("strict") >= 0.5;
            let buy = id.ends_with("buy");
            sim(x, from, to, &cfg, |i| {
                if i < k + 2 { return no.clone(); }
                let s = i - 1;
                if buy && l[s] > h[s - k] && h[s] > h[s - 1] && c[s] < c[s - 1] && (!strict || l[s] > l[s - 1]) { vec![(1.0, E::Stop(h[s - 1]))] }
                else if !buy && h[s] < l[s - k] && l[s] < l[s - 1] && c[s] > c[s - 1] && (!strict || h[s] < h[s - 1]) { vec![(-1.0, E::Stop(l[s - 1]))] }
                else { no.clone() }
            })
        }
        "lwnl_proj_contra" | "lwnl_proj_limit" => {
            let mask = if id == "lwnl_proj_contra" { g("dows").round() as u32 } else { 31 };
            let contra = id == "lwnl_proj_contra";
            sim(x, from, to, &cfg, |i| {
                let s = i - 1;
                let pp = (h[s] + l[s] + c[s]) / 3.0 * 2.0;
                let (ph, pl) = (pp - l[s], pp - h[s]);
                let d = dow(&x.r[i].date);
                if !(1..=5).contains(&d) || mask & (1 << (d - 1)) == 0 { return no.clone(); }
                if !contra { return if o[i] > pl { vec![(1.0, E::Limit(pl))] } else { no.clone() }; }
                if o[i] < pl { vec![(1.0, E::Stop(pl))] } else if o[i] > ph { vec![(-1.0, E::Stop(ph))] } else { no.clone() }
            })
        }
        "lwnl_truth1_closehigh" => sim(x, from, to, &cfg, |i| if c[i] == h[i] && c[i] > c[i - 1] { vec![(1.0, E::Close)] } else { no.clone() }),
        "lwnl_ndx_vb_oops" => {
            let (bp, sp, oops) = (g("buy_pct"), g("sell_pct"), g("oops") >= 0.5);
            sim(x, from, to, &cfg, |i| {
                if i < 2 { return no.clone(); }
                let s = i - 1;
                let tr = h[s].max(c[s - 1]) - l[s].min(c[s - 1]);
                let mut v = vec![(1.0, E::Stop(o[i] + bp * tr)), (-1.0, E::Stop(o[i] - sp * tr))];
                if oops && o[i] < l[s] { v.push((1.0, E::Stop(l[s]))); }
                v
            })
        }
        s if s.starts_with("lwnl_tdw_oc") => {
            let wd = match &s[12..] { "tue" => 2, "wed" => 3, "thu" => 4, _ => 5 };
            sim(x, from, to, &cfg, |i| if dow(&x.r[i].date) == wd { vec![(1.0, E::Open)] } else { no.clone() })
        }
        _ => return None,
    };
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn krutsinger_buy_and_proj_contra_hand_checked() {
        // bar 2's high (99) is below bar 1's low (100): buy bar 3's open 98; exit on bar 9's close (6 bars later) unless stopped
        let rows: Vec<Row> = vec![
            bar("2024-01-01", 101.0, 103.0, 100.0, 102.0), bar("2024-01-02", 101.0, 102.0, 100.0, 101.0),
            bar("2024-01-03", 98.0, 99.0, 97.0, 98.5), bar("2024-01-04", 98.0, 99.0, 97.5, 98.8),
            bar("2024-01-05", 99.0, 100.0, 98.5, 99.5), bar("2024-01-08", 99.5, 101.0, 99.0, 100.5),
            bar("2024-01-09", 100.5, 101.0, 100.0, 100.8), bar("2024-01-10", 100.8, 102.0, 100.5, 101.5),
            bar("2024-01-11", 101.5, 102.0, 101.0, 101.8), bar("2024-01-12", 101.8, 103.0, 101.5, 102.5),
            bar("2024-01-15", 102.5, 103.0, 102.0, 102.2), bar("2024-01-16", 102.0, 102.5, 101.0, 101.2),
        ];
        let x = Ctx::new(&rows, &rows, &[]);
        let mut pr = Params::new();
        pr.insert("stop_usd".into(), 0.0);
        let t = run("lwnl_krut_buy", &x, 1, rows.len() - 1, &pr).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px), (3, 98.0));
        assert_eq!((t[0].exit_i, t[0].exit_px), (9, 102.5));
        // $1,750 stop = 3.5 points: entry 98 -> stop 94.5, not hit
        let t = run("lwnl_krut_buy", &x, 1, rows.len() - 1, &Params::new()).unwrap();
        assert_eq!((t[0].exit_i, t[0].exit_px), (9, 102.5));
        // projected levels of bar 9 (H103 L101.5 C102.5): P = 204.667, PL = 101.667; bar 10 opens 102.5 (Monday, not below PL):
        // the limit model buys at 101.667 only if the low reaches it (low 102.0: no fill)
        let t = run("lwnl_proj_limit", &x, 10, 10, &Params::new()).unwrap();
        assert!(t.is_empty());
    }
}
