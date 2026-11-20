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
pub const REVIEWED: &[&str] = &["lwnl_jan_rally", "lwnl_mar_short", "lwnl_oct_td9_buy", "lwnl_oct_td17_sell", "lwnl_nov_td17_db7", "lwnl_jan97_td16_buy", "lwnl_jan97_td2_sell", "lwnl_jul97_td4_buy", "lwnl_jul97_td17_buy", "lwnl_jul97_td16_buy_30d", "lwnl_jul97_td14_sell", "lwnl_aug97_td12_buy_bail", "lwnl_aug97_td18_buy_bail", "lwnl_aug97_td7_sell_bail", "lwnl_aug97_td18_buy_7d", "lwnl_aug97_td2_sell_7d", "lwnl_nov97_td17_buy_7d", "lwnl_nov97_td18_buy_7d", "lwnl_nov97_td6_buy_30d", "lwnl_nov97_td15_buy_30d", "lwnl_dec97_td2_buy_1d", "lwnl_dec97_td14_buy_1d", "lwnl_dec97_td9_sell_1d", "lwnl_pullback", "lwnl_krut_buy", "lwnl_krut_sell", "lwnl_proj_contra", "lwnl_truth1_closehigh", "lwnl_gap_fail_300", "lwnl_gap_fail_525", "lwnl_ndx_vb_oops", "lwnl_tg_close", "lwnl_tg_open8", "lwnl_tg_oc", "lwnl_tg_open3_bail", "lwnl_xmas_bail", "lwnl_xmas_bail9", "lwnl_memorial_bail", "lwnl_memorial_bail6", "lwnl_july4_buy", "lwnl_yearend_sell", "lwnl_engulf_bull_1", "lwnl_engulf_bull_5", "lwnl_engulf_bull_10", "lwnl_engulf_bear_1", "lwnl_engulf_bear_5", "lwnl_engulf_bear_10", "lwnl_keyrev_buy", "lwnl_proj_limit", "lwnl_tdw_oc_tue", "lwnl_tdw_oc_wed", "lwnl_tdw_oc_thu", "lwnl_tdw_oc_fri"];
pub const SPX_POINTS: &[&str] = &["lwnl_jan_rally", "lwnl_mar_short", "lwnl_oct_td9_buy", "lwnl_oct_td17_sell", "lwnl_nov_td17_db7", "lwnl_jan97_td16_buy", "lwnl_jan97_td2_sell", "lwnl_jul97_td4_buy", "lwnl_jul97_td17_buy", "lwnl_jul97_td16_buy_30d", "lwnl_jul97_td14_sell", "lwnl_aug97_td12_buy_bail", "lwnl_aug97_td18_buy_bail", "lwnl_aug97_td7_sell_bail", "lwnl_aug97_td18_buy_7d", "lwnl_aug97_td2_sell_7d", "lwnl_nov97_td17_buy_7d", "lwnl_nov97_td18_buy_7d", "lwnl_nov97_td6_buy_30d", "lwnl_nov97_td15_buy_30d", "lwnl_dec97_td2_buy_1d", "lwnl_dec97_td14_buy_1d", "lwnl_dec97_td9_sell_1d", "lwnl_pullback", "lwnl_krut_buy", "lwnl_krut_sell", "lwnl_proj_contra", "lwnl_truth1_closehigh", "lwnl_gap_fail_300", "lwnl_gap_fail_525"];
pub const NDX_POINTS: &[&str] = &["lwnl_ndx_vb_oops"];
pub const US_ONLY: &[&str] = &["lwnl_tg_close", "lwnl_tg_open8", "lwnl_tg_oc", "lwnl_tg_open3_bail", "lwnl_xmas_bail", "lwnl_xmas_bail9", "lwnl_memorial_bail", "lwnl_memorial_bail6", "lwnl_july4_buy"];

/// Lead decision 197 (K37, 4 Nov 2026): the outside-day cards cite the April 1991 letter, which is not in the research share.
const OD_HELD: &str = "the cited source issue is not on the drive: these rules cite the April 1991 issue of Larry Williams's newsletter (p.1-3), and the share's only copy of the letters (Trading Knowledge II/@NewsLetters-Mags/Larry Williams Newsletters (1994-1997)/Larry Williams Newsletters 1994-1997.zip) has no April 1991 issue (no volume 25 issue 4); the share scan and the title census list no other copy (lead decision 197, share-only rule)";

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
    // Dec 1997 p.2 "we bought on the opening" (decision 186, D1-40); entry_pct = 0.07 (Nov 1997's rule, "as before") is an option
    ("lwnl_dec97_td2_buy_1d", 12, 2, 1.0, 0.0, 3500.0, Some(1), None, false),
    ("lwnl_dec97_td14_buy_1d", 12, 14, 1.0, 0.0, 3500.0, Some(1), None, false),
    ("lwnl_dec97_td9_sell_1d", 12, 9, -1.0, 0.0, 3500.0, Some(1), None, false),
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
    let mut v = catalogue_base();
    // lane F1D (R4 #293): the T-bond seasonal trades (BSEA), Norgate T-bond contract only
    v.extend(BSEA.iter().map(|b| StrategyDef { id: b.0, name: b.1, group: "Bonds · seasonal", session: 0, dir: if b.5 > 0.0 { "Long" } else { "Short" },
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: b.10,
        notes: b.11, claim: claim(b.12[0], b.12[1], b.12[2], "", b.12[3]) }));
    // lane O1 (R4 #367 (2)): commodity systems, Norgate continuous contract of the letter's market only
    v.extend(CSYS.iter().map(|b| StrategyDef { id: b.0, name: b.1, group: "Commodities", session: 0, dir: b.2, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: b.4, notes: b.5, claim: claim(b.6[0], b.6[1], b.6[2], "", b.6[3]) }));
    v
}

fn catalogue_base() -> Vec<StrategyDef> {
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
            &["E|August TD2, sell stop at the opening minus 7% of the previous range (Aug 1997 p.4)", "X|close 7 days later or the $1,500 stop (Aug 1997 p.4)"],
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
            &["E|\"same entry and exit rules as before, which means we bought on the opening of the trading day of the month shown\" (Dec 1997 p.2): buy on the opening (decision 186); p.entry_pct = 0.07 = open + 7% of the previous range, Nov 1997's rule that the letter calls 'as before' [option]",
              "X|\"exit on the close the number of days shown later\" (Dec 1997 p.2): 1 day; \"For the S&P 500 the best buys are day 2 and 14, the sells on day 9 with a $3,500 stop\" (Dec 1997 p.4)"],
            "The letter prints two different result tables for the same trades (p.3 and p.4).", claim("15", "80% / 93%", "$568 ($8,525) / $11,175", "1 day", "S&P (letter Dec 1997, p.3 / p.4)")),
        sd!("lwnl_dec97_td14_buy_1d", "S&P December TD14 buy, 1-day hold, $3,500 stop", "Seasonals · 1997 monthly lists", "Long", Live,
            &["E|December TD14, on the opening (Dec 1997 p.2); p.entry_pct = 0.07 = Nov 1997's open + 7% [option]", "X|close 1 day after entry or the $3,500 stop (Dec 1997 p.2, p.4)"],
            "Two tables: p.3 $11,300 11/14; p.4 $14,250 13/14.", claim("14", "78% / 92%", "$807 ($11,300) / $14,250", "1 day", "S&P (letter Dec 1997, p.3 / p.4)")),
        sd!("lwnl_dec97_td9_sell_1d", "S&P December TD9 sell, 1-day hold, $3,500 stop", "Seasonals · 1997 monthly lists", "Short", Live,
            &["E|December TD9, sell on the opening (Dec 1997 p.2); p.entry_pct = 0.07 = Nov 1997's open - 7% sell stop [option]", "X|close 1 day after entry or the $3,500 stop (Dec 1997 p.4)"],
            SP, claim("14", "64%", "$444 ($6,225)", "1 day", "S&P (letter Dec 1997)")),
        sd!("lwnl_yearend_sell", "S&P year-end short: open after December TD20, first profitable close or 3 days", "Seasonals · trading day of month", "Short", Live,
            &["E|\"sold the S&P 500 on the opening following the 20th trading day\" [of December] (Dec 1995 p.3)",
              "X|\"exited on the first profitable close or 3 days after entry\" (Dec 1995 p.3); no stop stated"],
            SP, claim("13", "92%", "$1,233.85 ($16,040)", "", "S&P (letter Dec 1995)")),
        // ---- Holidays
        sd!("lwnl_tg_close", "Thanksgiving: buy the close before, exit the close after", "Holidays", "Long", Live,
            &["E|\"bought on the close prior to the holiday\" (Vol. 27 No. 11 = Nov 1993 by its volume number, p.3)", "X|\"exited the close the day after\" (Vol. 27 No. 11 = Nov 1993, p.3); no stop"],
            "Book: subscriber's hand count since 1982.", claim("10", "60%", "+0.80 S&P points total", "1 day", "S&P 1982-1991")),
        sd!("lwnl_tg_open8", "Thanksgiving: buy the open 2 days before, exit 8 days later", "Holidays", "Long", Live,
            &["E|\"buy on the open of the trading session two days before the holiday\" (Oct 1994 p.3)", "X|\"and exit 8 days later\" (Oct 1994 p.3): the close of the 8th trading day after entry; no stop is stated"],
            "Printout S&P (03/94 data, Oct 1994 p.4): 12 trades, 11 winners; the one loser lasted 2 bars, which suggests an unstated stop: none is added.",
            claim("12", "91%", "$2,110.42 ($25,325)", "8 days", "S&P to 03/94")),
        sd!("lwnl_tg_oc", "Thanksgiving: buy the open the day before, exit that close", "Holidays", "Long", Live,
            &["E|\"buy the S&P 500 on the open of the day prior to the holiday\" (Vol. 29 No. 10 (Nov 1994) p.3)", "X|\"exit on that days close\" (Vol. 29 No. 10 (Nov 1994) p.3)"],
            SP, claim("12", "83%", "$208 ($2,500)", "same day", "S&P (letter Oct 1995)")),
        sd!("lwnl_tg_open3_bail", "Thanksgiving: buy the open 3 days before, bailout or 9 days", "Holidays", "Long", Live,
            &["E|\"buying the S&P on the open 3 days before Thanksgiving\" (Vol. 29 No. 10 (Nov 1994) p.3; 'This year one would buy on the opening of Monday, the 20th')", "X|\"exiting with my Bailout exit, or the close 9 days later\" (Vol. 29 No. 10 (Nov 1994) p.3)"],
            SP, claim("12", "92%", "$739 ($8,878)", "", "S&P (letter Oct 1995)")),
        sd!("lwnl_xmas_bail", "Christmas: buy the close 5 days before, first profitable opening", "Holidays", "Long", Live,
            &["E|\"buy on the close 5 days before the holiday\" (Nov 1994 p.4)", "X|\"Take your profits on the first open that shows a profit\" (Nov 1994 p.4); no stop stated"],
            "Printout S&P INDEX 03/94 long trades.", claim("12", "100%", "$983.58 ($11,803)", "1 day", "S&P to 03/94")),
        sd!("lwnl_xmas_bail9", "Christmas: buy the close 5 days before, wait 9 days then bailout", "Holidays", "Long", Live,
            &["E|buy on the close 5 days before Christmas (Nov 1994 p.4)", "X|\"wait 9 days, then take the first profitable opening\" (Nov 1994 p.4)"],
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
            "Printout (p.3) S&P 500 INDEX 06/92 Test 3 (start not printed). 20-day MA = simple MA of closes.",
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
        sd!("lwnl_od_buy", "Down-close outside day: buy at the high of the day before, exit next day", "Patterns", "Long", NotTestable(OD_HELD),
            &["F|Outside day = higher high and lower low than the previous day; '- close' = close below the previous close (Apr 1991 p.1-2)",
              "E|\"buying only on the day after the OD - close, at the high of the day prior to the OD\"; \"IF OPEN LESS THAN HIGH OF THAT DAY\" (Apr 1991 p.2-3): buy stop",
              "X|\"results exiting the next day\" (Apr 1991 p.2): Read: the close of the day after entry (the letters' 'X days later' exits are on the close); no stop"],
            "Table 'DAY 1' S&P: $25,275, 28 trades (the '#+' column: $25,275 / $902 = 28), 85%, avg $902.", claim("28", "85%", "$902 ($25,275)", "1 day", "S&P (letter Apr 1991)")),
        sd!("lwnl_od_buy3", "Down-close outside day buy, exit 3 days later", "Patterns", "Long", NotTestable(OD_HELD),
            &["E|same entry as lwnl_od_buy (Apr 1991 p.2-3)", "X|\"BEST DAY 3\" for the S&P: the close of the 3rd day after entry"],
            "Table: S&P best day 3 $34,550, 28 trades, 78%, avg $1,233.", claim("28", "78%", "$1,233 ($34,550)", "3 days", "S&P (letter Apr 1991)")),
        sd!("lwnl_od_sell", "Up-close outside day: sell at the low of the day before, exit next day", "Patterns", "Short", NotTestable(OD_HELD),
            &["E|\"A + close OD sets up a sell at the low of the day just prior\" (Apr 1991 p.2), open above that low", "X|exit the next day (Read: the close), as the buy table"],
            "\"The sell side in the S&P generated 26 trades and 21 winners, for a net profit of $8,750\" (Apr 1991 p.3).", claim("26", "81%", "$337 ($8,750)", "1 day", "S&P (letter Apr 1991)")),
        sd!("lwnl_od_fade_buy", "Open below a down-close outside day: buy at its low, exit on the close", "Patterns", "Long", NotTestable(OD_HELD),
            &["E|\"when the market opens below the low of a down close OD and you then buy at the OD low\" (Apr 1991 p.3): buy stop at the OD low", "X|\"Exit was on close of entry day\" (Apr 1991 p.3)"],
            SP, claim("14", "71%", "$479 ($6,700)", "same day", "S&P (letter Apr 1991)")),
        sd!("lwnl_od_fade_sell", "Open above an up-close outside day: sell at its high, exit on the close", "Patterns", "Short", NotTestable(OD_HELD),
            &["E|\"A sell is given with an open above the high of an outside day up close, and you then sell at the high of the OD\" (Apr 1991 p.3)", "X|close of the entry day (Apr 1991 p.3)"],
            "Only buys are tabulated in the letter.", None),
        sd!("lwnl_gap_fail_300", "S&P gap failure: 2.00-pt gap, enter 3.00 pts back through the open, exit on the close", "Patterns", "Long/Short", Live,
            &["F|\"THIS GAP SHOULD BE AT LEAST 200 POINTS\" from the previous day's close (Jan 1991 p.3): 200 points = 2.00 index points",
              "E|\"FOLLOWING A LOWER GAP OF 200 POINTS ... THEN BUY 300 POINTS ABOVE THE GAP\"; higher gap: sell 300 points below (Jan 1991 p.3): stop at open +/- 3.00",
              "F|\"if there is a gap of more than 10 full points, don't trade this method\" (Jan 1991 p.3)",
              "X|\"exiting on the close of entry day does produce similar results\" (Jan 1991 p.3)"],
            "The printed results use an exit the author calls 'not fully disclosable'; the coded exit is the close he names as giving similar results, so the printed numbers are only indicative.",
            claim("26 + 11", "84% / 90%", "$1,346 / $3,259", "same day", "S&P (letter Jan 1991, undisclosed exit)")),
        sd!("lwnl_gap_fail_525", "S&P gap failure: 2.00-pt gap, enter 5.25 pts back through the open", "Patterns", "Long/Short", Live,
            &["E|\"BUY/SELL WITH A 200 POINT GAP AND BUY/SELL 525 POINT +/- THE GAP\" (Jan 1991 p.3)", "F|no trade when the gap exceeds 10 points (Jan 1991 p.3)", "X|close of the entry day (Jan 1991 p.3)"],
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
        sd!("lwnl_bond_sp_systems", "Bond-driven S&P systems", "Needs data", "Long", NotTestable("Split (lane F1D, R4 #293): the coded rules are the options p.exit_n of lwlts_bonds_channel_bstop / _sstop, lwnl_bond_sp_9ma_1down / _2down and lwnl_sp_hornbrook_im. Not coded: the weekly bond stochastic timer (no exit printed) and the D-mark / dollar index / Eurodollar study (no Deutsche mark or dollar index series in the vault)."),
            &["E|Buy the S&P on the close when Bonds close above their 14-day highest high; exit a 10-day lowest low in Bonds or $3,000 stop (Commodity Timing Vol. 28 No. 4, April 1994 by its volume number, archive folder '1995', p.1-3; 66 trades 50% $112,175), or the S&P 5-day low (94 trades 41% $96,600) = options p.exit_n=10 / 5 of lwlts_bonds_channel_bstop / _sstop (live, lane F1D)",
              "E|9-day MA of Bonds rising + 1 or 2 S&P down closes: buy open + 20% of the last range, $2,500 stop or close 2 days later (Sep 1994 p.3-4; 308 trades 59% $116,900 / 145 trades 58% $43,875) = lwnl_bond_sp_9ma_1down / _2down (coded, lane F1D; withheld under R4 #240: 7.5% / 6.4%)",
              "E|Weekly 14-bar Bond stochastic as an S&P timer (Feb 1992, Jul 1992, Nov 1992, Dec 1993): no exit given",
              "E|Hornbrook: British Pound, crude and gold closes as an S&P filter, buy open exit close (May 1996 p.3): needs BP, crude, gold futures"],
            "All need bond (or other) futures alongside the S&P."),
        sd!("lwnl_cti_bonds", "CTI bond system: 60% of the range above the high / 90% below the low", "Bonds", "Long/Short", Live,
            &["E|\"by adding 60% of today's range to today's high some very nice buy signals developed ... by subtracting 90% of today's range ... from today's low and selling there, on stop\" (Jun 1993 (Vol. 28 No. 6) p.2): tomorrow's buy stop at H + 0.6 x (H - L), sell stop at L - 0.9 x (H - L) (\"chart range not true range\")",
              "X|\"always long/short\" (p.2): an opposite stop exits and reverses",
              "X|\"These results are with a $3250 stop for longs, $750 for shorts\" (p.3): 3.25 / 0.75 points at $1,000 per point, armed on the entry day (the letters' convention for entries during the day); no target (\"None made more money than the no target approach\")"],
            "Printout Jun 1993 (Vol. 28 No. 6) p.3 (TREASURY BONDS, window header illegible; the system was built on 1988 forward and checked on 'unknown' data from June of the previous year): 98 trades (41 won, 57 lost), 41%, $17,756.25, average $181.19 after $50 per trade. Runs on the Norgate T-bond continuous contract only (lane F1D, R4 #293): signals and points on the back-adjusted series, % per trade on the unadjusted entry fill price.",
            claim("98", "41%", "$181.19 (after $50)", "", "window not legible (1988-1992?)")),
        sd!("lwnl_bond_sp_9ma_1down", "S&P buy after a down close in a bond uptrend (9-day MA)", "Bonds -> S&P", "Long", Live,
            &["F|\"We will want Bonds to be in an uptrend, which means a 9 day moving average of Bonds is greater today than it was yesterday\" (Commodity Timing Vol. 28 No. 9 p.3): the bond's 9-bar average of closes (own bars) above yesterday's, on the signal day",
              "F|\"Then we want the S&P 500 to have a pull back ... The first set of results below is based on one down close; the second set requires two consecutive down closes\" (p.3)",
              "E|\"the computer bought at 20% of the last down day's range added to the following open\" (p.3): a buy stop at the open + 0.2 x the down day's range",
              "X|\"we used a $2,500 stop loss, or exited on the close two days later\" (p.3): 5 points at the letters' $500 per point; out on the close of the second day after entry (reading of 'two days later')",
              "X|Unstated in the letter: when the stop starts; vault reading: on the entry day (the letters' convention for entries during the day, module header); option p.day0=0 = from the next day"],
            "Printouts p.3-4 (S&P INDEX, 'from 1982 forward', issue of Vol. 28 No. 9 = September 1994 by its volume number; archive folder '1995'): one down close 308 trades, 59%, $116,900, average $379.55; two down closes 145 trades, 58%, $43,875, average $302.59. Runs on the Norgate S&P 500 continuous contract with the Norgate T-bond contract as Data2 (lane F1D, R4 #293); published 1982-04-21..1995-07-31 (R4 #330). Adapted: 2 unstated details (R4 #297).", claim("308", "59%", "$379.55", "2 days", "1982-1994 (Sep 1994 letter)")),
        sd!("lwnl_bond_sp_9ma_2down", "S&P buy after two down closes in a bond uptrend (9-day MA)", "Bonds -> S&P", "Long", Live,
            &["F|\"We will want Bonds to be in an uptrend, which means a 9 day moving average of Bonds is greater today than it was yesterday\" (Commodity Timing Vol. 28 No. 9 p.3): the bond's 9-bar average of closes (own bars) above yesterday's, on the signal day",
              "F|\"Then we want the S&P 500 to have a pull back ... The first set of results below is based on one down close; the second set requires two consecutive down closes\" (p.3)",
              "E|\"the computer bought at 20% of the last down day's range added to the following open\" (p.3): a buy stop at the open + 0.2 x the down day's range",
              "X|\"we used a $2,500 stop loss, or exited on the close two days later\" (p.3): 5 points at the letters' $500 per point; out on the close of the second day after entry (reading of 'two days later')",
              "X|Unstated in the letter: when the stop starts; vault reading: on the entry day (the letters' convention for entries during the day, module header); option p.day0=0 = from the next day"],
            "Printouts p.3-4 (S&P INDEX, 'from 1982 forward', issue of Vol. 28 No. 9 = September 1994 by its volume number; archive folder '1995'): one down close 308 trades, 59%, $116,900, average $379.55; two down closes 145 trades, 58%, $43,875, average $302.59. Runs on the Norgate S&P 500 continuous contract with the Norgate T-bond contract as Data2 (lane F1D, R4 #293); published 1982-04-21..1995-07-31 (R4 #330). Adapted: 2 unstated details (R4 #297).", claim("145", "58%", "$302.59", "2 days", "1982-1994 (Sep 1994 letter)")),
        sd!("lwnl_bond_projected", "Bonds contrarian at the projected high / low", "Bonds", "Long/Short", Live,
            &["F|Projected high / low (Commodity Timing Vol. 29 No. 9 p.2-3, the 1970 formula): P = (H + L + C) / 3 x 2 of the day; projected low = P - H, projected high = P - L",
              "E|\"If the open tomorrow is less than the projected low from the preceding day I will then buy at the projected low figure\": a buy stop at yesterday's projected low on a day that opens below it; \"buys bonds regardless of what day of the week it is\" (p.3)",
              "E|\"if the open tomorrow is greater than the projected high from the preceding day I will sell should prices come back to that value ... sell Bonds only on Monday, Wednesday or Thursday\" (p.3)",
              "X|\"The exit is my old buddy Bail Out with a $750 Stop in the Bonds\" (p.3): first profitable opening; 0.75 point stop",
              "X|Unstated in the letter: when the stop starts; vault reading: on the entry day (the letters' convention, module header); option p.day0=0 = from the next day",
              "X|Unstated in the letter: an opposite signal while in a trade; vault reading: it exits and reverses (Ch. 11 rule 3 of Long-Term Secrets)"],
            "Printout p.3 (Vol. 29 No. 9 = September 1995 by its volume number; archive folder '1996'; \"Bond data is from 1977 to 1994\"): 352 trades, 257 winners (73%), net about $71,500 (OCR reads $71,472.50; gross $144,842.25 - $73,269.75). Runs on the Norgate T-bond continuous contract only (lane F1D, R4 #293). Adapted: 2 unstated details (R4 #297).",
            claim("352", "73%", "$203 ($71,472.50)", "1 day", "1977-1994")),
        sd!("lwnl_sp_hornbrook_im", "S&P buy the open on a pound / crude / gold intermarket set-up (Hornbrook)", "Intermarket -> S&P", "Long", Live,
            &["F|\"If the BP closes today > the close 8 days ago and Crude Oil close today > yesterday and Gold closes today < the close 5 days ago\" (Commodity Timing Vol. 32 No. 5, archive folder 1996, p.3): each on its own Norgate contract's bars (Data2), read on the signal day",
              "E|\"buy the S&P on the opening\" (p.3)",
              "X|\"exiting on the close with no stop\" (p.3)"],
            "Letter p.3 (Ken Hornbrook's intermarket study): in sample 297 trades, 62% winners, $74,900, average $252; out of sample 72 trades, 62%, average $315 (windows not printed). Runs on the Norgate S&P 500 continuous contract with the Norgate British pound, crude oil and gold contracts as Data2 (lane F1D, R4 #293); published 1982-04-21..1995-07-31 (R4 #330: it buys the &SP open). No unstated detail.",
            claim("297 in sample + 72 out of sample", "62%", "$252 / $315", "same day", "windows not printed")),
        sd!("lwnl_bond_dowm_buy", "T-Bonds: buy 3 ticks under the open after the 4th Mon/Tue/Thu and 5th Mon/Tue of the month", "Bonds · seasonal", "Long", Live,
            &["F|Ken Hornbrook's DOWM function (Commodity Timing Vol. 32 No. 5, archive folder 1996, p.2): week of the month by calendar day (1-7, 8-14, 15-21, 22-28, 29+) and the day of the week",
              "E|\"buying at 3 ticks below the opening of Bonds following the 4th Monday, 4th Tuesday, and 4th Friday of the month as well as following the 5th Monday and Tuesday\" (p.2): a buy limit on the next day at its open - 3/32",
              "F|Unstated (a conflict): the printed code reads DOWM = 40 + 4 (the 4th Thursday) where the text says the 4th Friday; vault reading: the code, the source of the printed test; option p.fri=1 = the text",
              "X|\"exiting with a $1,000 loss, $2,250 profit or on the close 3 days after entry\" (p.2): 1.0 / 2.25 points; the stop armed on the entry day (R4 #335, option p.day0)"],
            "Printout p.2, bonds 1988 to May 1996: 190 trades (114 won, 76 lost; the OCR total reads 290), 60%, $48,031.25, average $252.80. Runs on the Norgate T-bond continuous contract only (lane F1D, R4 #293). Adapted: 2 unstated details (R4 #297).",
            claim("190", "60%", "$252.80", "3 days", "1988-1996")),
        sd!("lwnl_bond_xmas_buy", "T-Bonds: buy 5 trading days before Christmas, out on the open after it", "Bonds · seasonal", "Long", Live,
            &["E|\"Buying Bonds ... 5 trading days before Christmas\" (Commodity Timing Vol. 34 No. 12 = December 1997, p.4): the 5th trading day before Christmas (1st = the last bar before the holiday). Unstated in the letter: the order type; vault reading: market on the open",
              "X|\"and selling on the opening after the holiday\" (p.4): the open of the first trading day after Christmas; no stop is stated, so none is used"],
            "Letter: \"has been a big winner\" (no figures printed), so there is no #168 check. R4 #331: 1 of its &ZB trades falls in the 1999-12..2000-10 months whose Norgate opens sit at the previous close. Runs on the Norgate T-bond continuous contract only (lane F1D, R4 #293, review M34). Adapted: 1 unstated detail (R4 #297).",
            claim("", "", "", "about 1 week", "no figures printed")),
        sd!("lwnl_bond_newyear_sell", "T-Bonds: sell 5 trading days before New Year's, out on the first open of the year", "Bonds · seasonal", "Short", Live,
            &["E|\"selling Bonds 5 trading days before New Years\" (Vol. 34 No. 12, p.4): the 5th trading day before 1 January. Unstated in the letter: the order type; vault reading: market on the open",
              "X|\"and exiting on the open of next year\" (p.4): the open of the first trading day of the new year; no stop is stated, so none is used"],
            "Letter: \"has been a big winner\" (no figures printed), so there is no #168 check. R4 #331: 1 of its &ZB trades falls in the 1999-12..2000-10 months whose Norgate opens sit at the previous close. Runs on the Norgate T-bond continuous contract only (lane F1D, R4 #293, review M34). Adapted: 1 unstated detail (R4 #297).",
            claim("", "", "", "about 1 week", "no figures printed")),
        sd!("lwnl_bond_systems", "T-Bond systems and seasonals", "Needs data", "Long/Short", NotTestable("Split (lane F1D, R4 #293): the coded rules are listed in its lines (lwlts_bonds_8day, lwnl_cti_bonds, lwnl_bond_projected, the lwnl_bond_* seasonal and holiday cards, lwnl_bond_dowm_buy). Not coded: the Oct 1997 bond table (no stop or exit printed; the table is garbled)."),
            &["E|Close > close 8 days ago, $850 stop, ~3x target (Aug 1991 p.2) = lwlts_bonds_8day (live)",
              "E|CTI bond breakout: high + 60% range buy, low - 90% range sell, stops $3,250 / $750 (Jun 1993 (Vol. 28 No. 6) p.2-3) = lwnl_cti_bonds (live, lane F1D)",
              "E|Bond holiday, TDOM and day-of-month trades: Thanksgiving, Christmas, Memorial, July 4th, Labor Day, Jan TD18, Dec TD19, Oct 29th, Nov 3-8, May TD9, July TD12, Sep, Oct, Nov, Dec tables (1993-1997 letters). Coded (lane F1D): lwnl_bond_memorial_buy, lwnl_bond_labor_buy, lwnl_bond_sep_td9_sell, lwnl_bond_sep_td19_buy, lwnl_bond_jul_td12_sell, lwnl_bond_dec_td14_buy, lwnl_bond_dec_td22_buy, lwnl_bond_sep_td4_sell, lwnl_bond_sep_td20_buy; held: lwnl_bond_dec_td7_sell / _td8_sell (3 unstated details). Not coded yet: the Oct 1997 table (no stop or exit printed; the table is garbled) (the Christmas / New Year trades are lwnl_bond_xmas_buy / lwnl_bond_newyear_sell)",
              "E|Projected low/high contrarian on Bonds (Vol. 29 No. 9 p.3: 352 trades 73% $71,472.50) = lwnl_bond_projected (lane F1D); DOWM buys (May 1996 p.2)"],
            ""),
        sd!("lwnl_commodity_systems", "Commodity and currency systems and seasonals (rest of the bundle)", "Not testable", "Long/Short",
            NotTestable("Split (lane O1, 20 Nov 2026, R4 #367 (2)): coded as their own cards: lwnl_gold_13x13 (May 1993 p.1-2) and lwnl_gold_tdom1_4d (Mar 1995 p.3-4), both on the Norgate gold contract. HELD: Larry's Cup of Coffee (Mar 1992 p.3, coffee): 61% of its trades change when the in-bar path of its two live stop orders is flipped (#240 > 5%, no coffee minutes are held) and the engine reads 548 trades to Mar 1992 against the printed 2,117 (4 unstated details: stop arming, both orders live, one fill per side per bar, point value); pork bellies (Nov 1996 p.2, Aug 1996, Dec 1996) are not a held market; lane O2 (20 Nov 2026) coded gold Nov TD16 (lwnl_gold_nov_td16), yen Nov TD10 (lwnl_yen_nov_td10), live cattle Jul TD18 (lwnl_cattle_jul_td18), corn Jul TD2 short (lwnl_corn_jul_td2_short), coffee Aug TD12 open and limit entries (lwnl_coffee_aug_td12_open / _limit) and yen Apr TD4 (lwnl_yen_apr_td4), all UNVERIFIED; HELD: coffee Aug TD12 with the buy stop at the open + 20% of the previous day's range (Aug 1995 p.3 clip three: #240 25% of trades change under the flipped in-bar path); the monthly seasonal lists (Sep 1993 - Dec 1997) are not coded yet (see authors/held-1119/O2/FINAL.md)."),
            &["E|Larry's Cup of Coffee: open + 50% / - 40% of the 4-day average range, $1,750 stop or 6th close (Mar 1992 p.3; 2,117 trades $535,456): held, #240 61%",
              "E|13x13 gold channel (May 1993 p.1-2) = lwnl_gold_13x13; gold TDOM1 > close 4 days ago, 12-day exit (Mar 1995 p.3-4) = lwnl_gold_tdom1_4d (both live)",
              "E|Coded by lane O2 (UNVERIFIED): lwnl_gold_nov_td16, lwnl_yen_nov_td10 (Nov 1996 p.3-4), lwnl_cattle_jul_td18, lwnl_corn_jul_td2_short (Jul 1995 p.3), lwnl_coffee_aug_td12_open / _limit (Aug 1995 p.3), lwnl_yen_apr_td4 (Mar 1997 p.3); coffee Aug TD12 buy stop held (#240 25%); lane O3 (20 Nov 2026) coded the single-market monthly seasonal trades with <= 4 unstated details (Jan 1997 special report pp.3-6, April 1996 list, wheat September, heating oil March; ids lwnl_jan97_*, lwnl_apr96_*, lwnl_wheat_sep_td1_5_buy, lwnl_ho_mar_*); HELD: pound TD3 sell and cattle TD21 buy of Jan 1997 (#240 order-dependent 7.3% / 12.5%), the 1997 month-by-month best-trade tables (rows, not rules), pork bellies and D-Mark (not held), and rules with more than 4 unstated details (see authors/held-1119/O3/FINAL.md); lane O4 (20 Nov 2026) coded gold TD16 every month (lwnl_gold_any_td16_buy), the bonds Oct TD9 buy / TD17 sell, Jan 1997 bonds TD7 buy / TD2 sell, Apr 1996 bonds TD11 / TD13 sells and the held-market rows of the 1997 7-day month tables read so far (April p.2, May p.3-4, July p.2-3, September p.2-3; ids lwnl_apr97_*, lwnl_may97_*, lwnl_jul97_*, lwnl_sep97_*; all UNVERIFIED), HELD: bonds Aug TD19-21 at 20% of the range (#240 11.6%), bonds Dec TD19+1 sell (exit 'first profitable close' is not an engine exit), table rows whose order inside the day changes more than 5% of their trades, the 30-day tables (entry/stop/exit not printed), pork belly / feeder cattle / D-Mark / lumber / S&P rows and the other 1997 tables not yet read (see authors/held-1119/O4/FINAL.md)",
              "E|Pork bellies trades (Nov 1996 p.2, Dec 1996): the market is not held"],
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
    // review M35: the holiday trades sit with the other holiday trades (lwicw_holidays)
    if matches!(id, "lwnl_bond_memorial_buy" | "lwnl_bond_labor_buy" | "lwnl_bond_xmas_buy" | "lwnl_bond_newyear_sell") {
        return f("lwicw_holidays", "Holiday trades", match id { "lwnl_bond_memorial_buy" => "T-Bonds, Memorial Day", "lwnl_bond_labor_buy" => "T-Bonds, Labor Day",
            "lwnl_bond_xmas_buy" => "T-Bonds, Christmas", _ => "T-Bonds, New Year's" });
    }
    if let Some(b) = CSYS.iter().find(|b| b.0 == id) { return f("lwnl_commodity", "Commodity systems", b.1); }
    if let Some(b) = BSEA.iter().find(|b| b.0 == id) { return f("lwnl_seasonal", "Trading-day-of-month seasonals", b.1); }
    if HOL.iter().any(|s| s.0 == id) { return f("lwnl_holiday", "Holiday trades", "S&P"); }
    match id {
        "lwnl_krut_buy" | "lwnl_krut_sell" => f("lwnl_krut", "Krutsinger gap", ""),
        "lwnl_od_buy" | "lwnl_od_buy3" | "lwnl_od_sell" | "lwnl_od_fade_buy" | "lwnl_od_fade_sell" => f("lwnl_od", "Outside days", ""),
        "lwnl_gap_fail_300" | "lwnl_gap_fail_525" => f("lwnl_gap_fail", "Gap failure", ""),
        "lwnl_keyrev_buy" | "lwnl_keyrev_sell" => f("lwnl_keyrev", "Key reversals", ""),
        "lwnl_proj_contra" | "lwnl_proj_limit" => f("lwnl_proj", "Projected high/low", ""),
        s if s.starts_with("lwnl_engulf") => f("lwnl_engulf", "Engulfing candlestick test", ""),
        s if s.starts_with("lwnl_tdw_oc") => f("lwnl_tdw", "Day of week open-to-close", ""),
        "lwnl_cti_bonds" => f("lwnl_cti", "CTI bond system", "60% / 90% range breakout"),
        "lwnl_bond_projected" => f("lwnl_bond_projected", "Projected high/low contrarian", "T-Bonds"),
        "lwnl_sp_hornbrook_im" => f("lwnl_sp_hornbrook_im", "Intermarket S&P set-up (Hornbrook)", "Pound, crude, gold"),
        "lwnl_bond_sp_9ma_1down" => f("lwnl_bond_sp_9ma", "S&P pullback in a bond uptrend", "One down close"),
        "lwnl_bond_sp_9ma_2down" => f("lwnl_bond_sp_9ma", "S&P pullback in a bond uptrend", "Two down closes"),
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
    let mut v = params_base(id)?;
    crate::intraday_ep6::add_param(id, &mut v); // EP6 (#243)
    Some(v)
}

fn params_base(id: &str) -> Option<Vec<ParamDef>> {
    if id == "lwnl_bond_sp_9ma_1down" || id == "lwnl_bond_sp_9ma_2down" {
        return Some(vec![p("entry_pct", "Buy stop: open + x the down day's range", 0.2, 0.0, 1.0, 0.05), stop(2500.0),
            p("hold", "Exit on the close N days after entry", 2.0, 0.0, 10.0, 1.0), pt(500.0), day0()]);
    }
    // lane O5 (R4 #375): a table card has a 'row' parameter (the 1-based printed row; 1 = the first printed row); a one-row card has none
    let rows = MP.iter().filter(|m| m.0 == id).count();
    if rows > 0 { return Some(if rows > 1 { vec![p("row", "Printed row of the book table (see the row table in the card notes; 1 = the first printed row)", 1.0, 1.0, rows as f64, 1.0)] } else { vec![] }); }
    if CSYS.iter().any(|b| b.0 == id) { return Some(vec![]); }
    if let Some(b) = BSEA.iter().find(|b| b.0 == id) {
        return Some(vec![stop(b.7), day0(), pt(1000.0)]);
    }
    if id == "lwnl_bond_xmas_buy" || id == "lwnl_bond_newyear_sell" { return Some(vec![pt(1000.0)]); }
    if id == "lwnl_sp_hornbrook_im" { return Some(vec![pt(500.0)]); }
    if id == "lwnl_bond_dowm_buy" {
        return Some(vec![stop(1000.0), p("target_usd", "Profit target $ (book $2,250)", 2250.0, 0.0, 10000.0, 250.0), day0(),
            p("fri", "4th Friday instead of the code's 4th Thursday (the text)", 0.0, 0.0, 1.0, 1.0), pt(1000.0)]);
    }
    if id == "lwnl_bond_projected" {
        return Some(vec![day0(), stop(750.0), p("sell_dow", "Sell days bitmask (Mon=1 Tue=2 Wed=4 Thu=8 Fri=16; book Mon/Wed/Thu = 13)", 13.0, 0.0, 31.0, 1.0), pt(1000.0)]);
    }
    if id == "lwnl_cti_bonds" {
        return Some(vec![p("buy_pct", "Buy stop: high + x today's range", 0.6, 0.0, 2.0, 0.05), p("sell_pct", "Sell stop: low - x today's range", 0.9, 0.0, 2.0, 0.05),
            stop(3250.0), p("stop_s_usd", "Short-side dollar stop (book $750)", 750.0, 0.0, 6000.0, 250.0), pt(1000.0)]);
    }
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


// ------------------------------------------------------------------ lane F1D: T-bond seasonal trades of the 1995-97 letters
/// (id, name, month (0 = holiday rule), TDOM or trading days before the holiday, holiday, dir, entry on the close, stop $,
/// exit (Some(n) = first profitable opening from bar entry + n; None = hold), hold bars, rules, notes, claim (trades, win, avg, period)).
/// Norgate T-bond contract only; TDOM = lwlts::tdm (decision 186: a partial first month is not counted). Stops armed on the entry
/// day for entries during the day (R4 #335; option p.day0).
pub(crate) type BSea = (&'static str, &'static str, u32, i32, Option<Hol>, f64, bool, f64, Option<usize>, usize, &'static [&'static str], &'static str, [&'static str; 4]);
const BX1: &str = "X|Unstated in the letter: the exit of these 'one day' trades; vault reading: the first profitable opening (the letters' Bail Out)";
const BOPEN: &str = "E|Unstated in the letter: the order type; vault reading: market on the open of that trading day";
pub(crate) const BSEA: &[BSea] = &[
    ("lwnl_bond_memorial_buy", "T-Bonds: buy the close before Memorial Day", 0, 1, Some(Hol::Memorial), 1.0, true, 0.0, Some(1), 0,
     &["E|\"buying Bonds on that close\" = the close of the day before Memorial Day (Commodity Timing Vol. 29 No. 5 = May 1995 by its volume number, p.4)", "X|\"exiting with our first profitable opening rule\"; no stop is stated, so none is used"],
     "Letter: \"10 wins out of 12 trades with a hefty $511 per trade, and total profits of $6,140\" (12 years, to 1994).", ["12", "83% (10 of 12)", "$511", "1983-1994"]),
    ("lwnl_bond_labor_buy", "T-Bonds: buy 3 trading days before Labor Day", 0, 3, Some(Hol::Labor), 1.0, false, 1000.0, Some(2), 0,
     &["E|\"buy on the opening of the third trading day before the holiday\" (Vol. 33 No. 8 = August 1996, p.2)", "X|\"The exit will be a $1,000 stop or the first profitable opening after you have been in the trade for two days\" (p.2): 1 point; the bailout from the 2nd bar after entry (reading of the count)"],
     "Letter: accuracy 83%, average profit $700, the last 13 years.", ["", "83%", "$700", "1983-1995"]),
    ("lwnl_bond_sep_td9_sell", "T-Bonds: sell September trading day 9", 9, 9, None, -1.0, false, 900.0, None, 7,
     &["E|\"a sell on 9/13 (9th trading day of the month) as in 12 of the last 16 years prices have declined\" (Vol. 33 No. 8 = August 1996, p.4)", BOPEN, "X|\"Use a $900 stop, exit 7 days later\": 0.9 point; out on the close of the 7th trading day after entry (reading)"],
     "Letter: 12 of 16 years, $11,293.", ["16", "75% (12 of 16)", "$706 ($11,293)", "1980-1995"]),
    ("lwnl_bond_sep_td19_buy", "T-Bonds: buy September trading day 19", 9, 19, None, 1.0, false, 900.0, None, 7,
     &["E|\"Buying on trading day 19 (9/27 this year) has produced $13,356 profits with 9 of 16 years winning\" (Vol. 33 No. 8, p.4)", BOPEN, "X|\"Use same stop and exit as for the sell\": $900, out on the close 7 trading days after entry (reading)"],
     "Letter: 9 of 16 years, $13,356.", ["16", "56% (9 of 16)", "$835 ($13,356)", "1980-1995"]),
    ("lwnl_bond_jul_td12_sell", "T-Bonds: sell July trading day 12", 7, 12, None, -1.0, false, 1500.0, Some(2), 0,
     &["E|\"sell Bonds on the opening of the 12th trading day of the month\" (Vol. 34 No. 7 = July 1997, p.3: \"the unknown seasonal trade of the month\")", "X|\"use a $1,500 stop and exit on the first profitable opening after being in the trade two days\" (p.3): 1.5 points; bailout from the 2nd bar after entry (reading of the count)"],
     "Letter: 94% accuracy (count and window not legible in the scan).", ["", "94%", "", "July (not legible)"]),
    ("lwnl_bond_dec_td14_buy", "T-Bonds: buy December trading day 14", 12, 14, None, 1.0, false, 1400.0, Some(1), 0,
     &["E|\"The best one day buys in the Bonds for December are the 14th and 22nd trading day of the month with a $1,400 stop\" (Vol. 34 No. 12 = December 1997, p.4)", BOPEN, BX1],
     "Table p.4: day 14 $3,250, 14 of 19 (73%).", ["19", "73% (14 of 19)", "$171 ($3,250)", "1978-1996"]),
    ("lwnl_bond_dec_td22_buy", "T-Bonds: buy December trading day 22", 12, 22, None, 1.0, false, 1400.0, Some(1), 0,
     &["E|\"The best one day buys in the Bonds for December are the 14th and 22nd trading day\" (Vol. 34 No. 12, p.4), $1,400 stop", BOPEN, BX1],
     "Table p.4: day 22 $4,593, 9 of 10 (90%).", ["10", "90% (9 of 10)", "$459 ($4,593)", "1978-1996"]),
    ("lwnl_bond_dec_td7_sell", "T-Bonds: sell December trading day 7", 12, 7, None, -1.0, false, 1400.0, Some(1), 0,
     &["E|\"The sells are day 7 and 8\" (Vol. 34 No. 12, p.4)", BOPEN, "X|Stop: the buys' $1,400 (reading: one stop for the bond table)", BX1],
     "Table p.4: sell day 7 $7,050, 15 of 16 (93%).", ["16", "93% (15 of 16)", "$440 ($7,050)", "1978-1996"]),
    ("lwnl_bond_dec_td8_sell", "T-Bonds: sell December trading day 8", 12, 8, None, -1.0, false, 1400.0, Some(1), 0,
     &["E|\"The sells are day 7 and 8\" (Vol. 34 No. 12, p.4)", BOPEN, "X|Stop: the buys' $1,400 (reading: one stop for the bond table)", BX1],
     "Table p.4: sell day 8 $7,493, 15 of 18 (83%).", ["18", "83% (15 of 18)", "$416 ($7,493)", "1978-1996"]),
    ("lwnl_bond_sep_td4_sell", "T-Bonds: sell September trading day 4", 9, 4, None, -1.0, false, 1200.0, Some(1), 0,
     &["E|\"In T BONDS the best sell was on the 4th trading day of the month, that shows 17 wins in 18 occasions\" (Vol. 34 No. 9 = September 1997, p.3)", BOPEN, "X|\"A $1,200 stop was used\" (p.3)", BX1],
     "Letter: 17 of 18, $6,056.", ["18", "94% (17 of 18)", "$336 ($6,056)", "1979-1996"]),
    ("lwnl_bond_sep_td20_buy", "T-Bonds: buy September trading day 20", 9, 20, None, 1.0, false, 1200.0, Some(1), 0,
     &["E|\"The best buy came on the 20th trading day of the month with net profits of $6,737 with 16 wins out of 16 trades\" (Vol. 34 No. 9, p.3)", BOPEN, "X|\"A $1,200 stop was used\" (p.3)", BX1],
     "Letter: 16 of 16, $6,737.", ["16", "100% (16 of 16)", "$421 ($6,737)", "1979-1996"]),
];


// ------------------------------------------------------------------ lane O1: commodity systems of the 1992-1995 letters
/// (id, name, dir, Norgate market, rules, notes, claim (trades, win, avg, period)). Run on the Norgate continuous contract of the market the letter traded
/// only (R4 #367 (2)); dollar stops are the letter's, converted at the market's own US$ per point (futures::usd_per_point).
pub(crate) type CSys = (&'static str, &'static str, &'static str, &'static str, &'static [&'static str], &'static str, [&'static str; 4]);
pub(crate) const CSYS: &[CSys] = &[
    ("lwnl_gold_13x13", "Gold 13-day channel breakout (13x13)", "Long/Short", "&GC",
     &["E|\"a gold model that bought at the highest high of the last 13 days and sold at the lowest low of the last 13. It is indeed a simple channel breakout. There are no other rules, no stops\" (Vol. 28 No. 5 = May 1993, p.1-2): buy stop at the 13-day highest high, sell stop at the 13-day lowest low, the 13 days before the order day",
       "X|Printed stop-and-reverse (no stops, no other rules): hold until the opposite signal (R4 #344), counted zero",
       "E|Unstated in the letter: the 13 days exclude the order day (the channel is known at the open)",
       "E|Unstated in the letter: the order is a stop order filled at the channel level (gaps fill at the open); the US$ per point (Norgate US$100 per dollar) is used only to report US$"],
     "Printout p.2 (GOLD COMEX, to 06/93 by the scan, start not legible): net $112,910, 175 trades, 40% winners (70), average $645.20.",
     ["175", "40% (70)", "$645.20 ($112,910)", "start not printed"]),
    ("lwnl_gold_tdom1_4d", "Gold: first trading day of the month, close above the close 4 days ago", "Long", "&GC",
     &["E|\"if it is the first trade date of the month and the close is greater than the close 4 days ago, what happens if I buy on the open tomorrow and exit 12 days later with a $2,000 stop?\" (Vol. 29 No. 3 = March 1995, p.3-4; the author says it is a study, not a strategy)",
       "X|Unstated in the letter (1 of 4): \"exit 12 days later\" is read as 12 bars after the entry bar; $2,000 / US$100 per dollar = 20 points",
       "X|Unstated in the letter (2 of 4): the exit is on the close of that bar",
       "X|Unstated in the letter (3 of 4): the stop is armed on the entry day",
       "E|Unstated in the letter (4 of 4): trading day 1 of the month by the vault's count (a partial first month is not counted, decision 186)"],
     "Printout p.4: net $41,850, 84 trades, 45 winners (53%), average $498.21, since 1975.",
     ["84", "53% (45)", "$498.21 ($41,850)", "1975 to Mar 1995"]),    ("lwnl_gold_nov_td16", "Gold: buy the 16th trading day of November, 6-day delayed bailout", "Long", "&GC",
     &["E|\"Gold has been a buy on the 16th trading day of November with a 6 day delayed bailout\" (Vol. 33 No. 11 = November 1996, p.4; the printout shows no stop): buy the open of November's trading day 16",
       "X|Printed exit: a 6-day delayed bailout, no stop. Unstated in the letter (1 of 4): the bailout is the first profitable opening from the opening of bar entry+6+1 (the letter's own wording of Delayed Bailout, Nov 1996 p.1: 'waiting N days ... then exiting on any profitable opening after that point'), profit measured against the entry price",
       "X|Unstated in the letter (2 of 4): a trade that is never profitable at an opening stays open (there is no time exit or stop)",
       "E|Unstated in the letter (3 of 4): trading day 16 by the vault's count of the month's sessions",
       "E|Unstated in the letter (4 of 4): the entry is at the open (the letter says only 'buy on the 16th trading day')",
       "E|Adaptation, not counted: the Norgate back-adjusted contract (the CSI contract file is not held); US$ per point is used only to report US$"],
     "Printout p.4 (COMEX GOLD, 05/28/75 - 10/29/96, commission $50): 19 trades, 100% winners, net $16,560, average $871.58.",
     ["19", "100% (19)", "$871.58 ($16,560)", "05/1975 to 10/1996"]),
    ("lwnl_yen_nov_td10", "Yen: buy the 10th trading day of November, 6-day delayed bailout or $1,600 stop", "Long", "&6J",
     &["E|\"the Yen has shown a pronounced tendency to rally starting the 10th trading day of the month ... Our exit rule is a 6 day Delayed Bailout or $1,600 stop\" (Vol. 33 No. 11 = November 1996, p.3): buy the open of November's trading day 10",
       "X|$1,600 stop = 0.0128 of the Norgate quote at US$125,000 per 1.00 (the letter's own dollars at the contract's own point value)",
       "X|Unstated in the letter (1 of 4): the bailout is the first profitable opening from the opening of bar entry+6+1 (the letter's own wording of Delayed Bailout, Nov 1996 p.1), profit measured against the entry price",
       "X|Unstated in the letter (2 of 4): the stop is armed on the entry day",
       "E|Unstated in the letter (3 of 4): trading day 10 by the vault's count of the month's sessions",
       "E|Unstated in the letter (4 of 4): the entry is at the open (the letter says only 'starting the 10th trading day')",
       "E|Adaptation, not counted: the Norgate back-adjusted contract (the CSI contract file is not held)"],
     "Printout p.3 (JAPANESE YEN, 12/23/76 - 10/29/96, commission $50, point value $12.50): 17 trades, 88% winners (15), net $8,862.50, average $521.32.",
     ["17", "88% (15)", "$521.32 ($8,862.50)", "12/1976 to 10/1996"]),
    ("lwnl_cattle_jul_td18", "Live cattle: July trading day 18, buy stop at the open + 20% of the previous day's range", "Long", "&LE",
     &["E|\"go long Live Cattle at the opening on the 18th trading day of July, plus 20% of the range of the 17th trading day ... IF AND ONLY IF price rallies to that level you go long. Exit 10 market days later, or be stopped out with a $900 loss\" (Vol. 29 No. 7 = July 1995, p.3): buy stop at the open + 0.2 x the previous day's range on July's trading day 18",
       "X|$900 stop = 2.25 cents of the Norgate quote at US$400 per 1.00",
       "X|Unstated in the letter (1 of 4): \"10 market days later\" is read as 10 bars after the entry bar",
       "X|Unstated in the letter (2 of 4): the exit is on the close of that bar",
       "X|Unstated in the letter (3 of 4): the stop is armed on the entry day",
       "E|Unstated in the letter (4 of 4): trading day 18 by the vault's count of the month's sessions"],
     "Printout p.3 (Test 5; window and costs not printed; 'the last 18 years'): 15 trades, 86% winners (13), net $6,861, average $457.40.",
     ["15", "86% (13)", "$457.40 ($6,861)", "about 18 years, not printed"]),
    ("lwnl_corn_jul_td2_short", "Corn: sell short on July trading day 2 at the open - 10% of the previous day's range", "Short", "&ZC",
     &["E|\"selling the September Corn contract on the second trading day of July by taking the opening that day and subtracting 10% of previous day's range. If, in the same day, prices decline to that price we would sell short. Our exit rule is simple: absorb a $1000 loss, or get out on the open 24 trading days later\" (Vol. 29 No. 7 = July 1995, p.3): sell stop at the open - 0.1 x the previous day's range on July's trading day 2",
       "X|$1,000 stop = 20 cents of the Norgate quote at US$50 per 1.00",
       "X|Unstated in the letter (1 of 3): \"24 trading days later\" is read as the open of the 24th bar after the entry bar",
       "X|Unstated in the letter (2 of 3): the stop is armed on the entry day",
       "E|Unstated in the letter (3 of 3): trading day 2 by the vault's count of the month's sessions",
       "E|Adaptation, not counted: the Norgate back-adjusted corn contract stands in for 'the September contract'"],
     "Printout p.3 (SHORT TRADES, Test 24; window and costs not printed): 18 trades, 83% winners (15), net $16,190, average $899.44.",
     ["18", "83% (15)", "$899.44 ($16,190)", "not printed"]),
    ("lwnl_coffee_aug_td12_open", "Coffee: buy the open of August's trading day 12", "Long", "&KC",
     &["E|\"a very strong August seasonal rally for coffee starting on the 12th trading (not calendar) day of the month\" (Vol. 29 No. 8 = August 1995, p.3). Clip one: buy the open of that day. \"The results shown are from 1974 to 1994. The exit rule used is to take a $750 loss or exit on the close six trading days later\"",
       "X|$750 stop = 2.0 cents of the Norgate quote at US$375 per 1.00",
       "X|Unstated in the letter (1 of 3): \"six trading days later\" is read as the close of the 6th bar after the entry bar",
       "X|Unstated in the letter (2 of 3): the stop is armed on the entry day",
       "E|Unstated in the letter (3 of 3): trading day 12 by the vault's count of the month's sessions",
       "E|Adaptation, not counted: the Norgate back-adjusted contract (the CSI file is not held)"],
     "Printout p.3 (1974 to 1994, costs not printed): 20 trades, 60% winners, net $29,411.25, average $1,470.56.",
     ["20", "60% (12)", "$1,470.56 ($29,411.25)", "1974-1994"]),
    ("lwnl_coffee_aug_td12_limit", "Coffee: buy at the open - 20% of the previous day's range on August's trading day 12", "Long", "&KC",
     &["E|\"a very strong August seasonal rally for coffee starting on the 12th trading (not calendar) day of the month\" (Vol. 29 No. 8 = August 1995, p.3). Clip two: buying at 20% of the previous day's range subtracted from the opening of the 12th trading day (a limit buy). \"The results shown are from 1974 to 1994. The exit rule used is to take a $750 loss or exit on the close six trading days later\"",
       "X|$750 stop = 2.0 cents of the Norgate quote at US$375 per 1.00",
       "X|Unstated in the letter (1 of 3): \"six trading days later\" is read as the close of the 6th bar after the entry bar",
       "X|Unstated in the letter (2 of 3): the stop is armed on the entry day",
       "E|Unstated in the letter (3 of 3): trading day 12 by the vault's count of the month's sessions",
       "E|Engine convention, not counted: a limit buy fills when the day trades down to it, and at the open when the open is already at or below it"],
     "Printout p.3 (1974 to 1994, costs not printed): 15 trades, 73% winners, net $30,615.00, average $2,041.00.",
     ["15", "73% (11)", "$2,041.00 ($30,615.00)", "1974-1994"]),
    ("lwnl_yen_apr_td4", "Yen: buy the 4th trading day of April, $2,200 stop, 12-day exit", "Long", "&6J",
     &["E|\"In 12 of the last 20 years (60%) the Yen has been a buy on the 4th trading day of APRIL. These results used a $2,200 stop and 12 day exit\" (Vol. 34 No. 3 = March 1997, p.3): buy the open of April's trading day 4",
       "X|$2,200 stop = 0.0176 of the Norgate quote at US$125,000 per 1.00",
       "X|Unstated in the letter (1 of 4): \"12 day exit\" is read as 12 bars after the entry bar",
       "X|Unstated in the letter (2 of 4): the exit is on the close of that bar",
       "X|Unstated in the letter (3 of 4): the stop is armed on the entry day",
       "E|Unstated in the letter (4 of 4): trading day 4 by the vault's count of the month's sessions",
       "E|Engine convention, not counted: a trading-day entry fills at that day's open"],
     "Printout p.3 (JAPANESE YEN, 11/24/76 - 02/21/97, commission $50, point value $12.50): 20 trades, 60% winners (12), net $20,487.50, average $1,024.37.",
     ["20", "60% (12)", "$1,024.37 ($20,487.50)", "11/1976 to 02/1997"]),
    (r##"lwnl_jan97_list"##, r##"Williams Jan 1997 best seasonal trading-day trades (8 printed rows)"##, r##"Long/Short"##, r##"&KC"##,
     &[r##"E|"buy on the open of X TRADING DAY OF THE MONTH, NOT CALENDAR DAY, +/- 0.05% of the previous days range as proof the market was going in our direction ( + for longs - for shorts). The exit was either a $1,800 stop or getting out on the close of the 5th day after the trade was entered" (Vol. 34 No. 1 = January 1997, p.3, special report on seasonal trades)"##, r##"X|Unstated in the letter: '+/- 0.05% of the previous days range' is read literally as a stop order at the open +/- 0.0005 x the previous bar's range (0.05 of the range, i.e. 5%, is the other reading; not run)"##, r##"X|Unstated in the letter: 'the close of the 5th day after the trade was entered' = the close of the 5th bar after the entry bar"##, r##"X|Unstated in the letter: the $1,800 stop is armed on the entry day (converted at the market's own US$ per point)"##, r##"E|Adaptation, not counted: the Norgate back-adjusted continuous contract (the CSI contract file is not held)"##, r##"E|Trading day (TD) of each row = the vault's count of the month's sessions (an unstated detail of every row; the row's own TD is in the row table)"##, r##"X|Row-specific labelled details (at most 4 per row, #339/#343) and the per-row level, stop, exit, #240 figure and printed record are in the row table in the notes"##, r##"E|Row parameter: 'row' picks the printed row (default 1 = the first printed row); the card runs on the Norgate contract of that row's market only, and a row on another market is refused with a message listing that market's rows"##],
     r##"Williams Jan 1997 best seasonal trading-day trades (8 printed rows). Source: Vol. 34 No. 1 (January 1997) p.3-6. 8 printed rows; ONE card with a row parameter (R4 #375). Rows above the 5% order-dependence line (#175, #240) are not coded.
Row 1: Coffee (&KC) | Short TD16 of Jan | level: stop at the open - 0.05% of the previous range | stop/exit: close 5 bars after entry, $1,800 stop | 4 labelled details: the 0.05% literal reading, the 5th-bar close, stop armed on the entry day, trading-day count | #240 0/39 = 0.0% | printed: Printout p.4 (Test 15, window and costs not printed): 22 trades, 63% winners (14), net $20,901.25, average $950.06; $10,746.25 came from one trade. | engine 17 tr 82% net $24,235.75 avg $1,425.63
Row 2: Gold (&GC) | Short TD14 of Jan | level: stop at the open - 0.05% of the previous range | stop/exit: close 5 bars after entry, $1,800 stop | 4 labelled details: the 0.05% literal reading, the 5th-bar close, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: Printout p.5 (Test 13, window and costs not printed): 20 trades, 60% winners (12), net $31,360, average $1,568.00; $23,800 came from one trade. | engine 18 tr 61% net $33,263.65 avg $1,847.98
Row 3: Orange Juice (&OJ) | Short TD14 of Jan | level: stop at the open - 0.05% of the previous range | stop/exit: close 5 bars after entry, $1,800 stop | 4 labelled details: the 0.05% literal reading, the 5th-bar close, stop armed on the entry day, trading-day count | #240 0/38 = 0.0% | printed: Printout p.5 (Test 17, window and costs not printed): 15 trades, 73% winners (11), net $5,700 (average $380 = net / 15; the text quotes $410). | engine 15 tr 73% net $6,739.08 avg $449.27
Row 4: Orange Juice (&OJ) | Long TD18 of Jan | level: stop at the open + 0.05% of the previous range | stop/exit: close 5 bars after entry, $1,800 stop | 4 labelled details: the 0.05% literal reading, the 5th-bar close, stop armed on the entry day, trading-day count | #240 0/39 = 0.0% | printed: Printout p.5 (Test 13, 'last 15 Januarys', costs not printed): 15 trades, 73% winners (11), net $6,150, average $410.00. | engine 13 tr 85% net $5,427.09 avg $417.47
Row 5: British Pound (&6B) | Long TD22+1 of Jan | level: stop at the open + 0.05% of the previous range (bar after the TD) | stop/exit: close 5 bars after entry, $1,800 stop | 4 labelled details: the 0.05% literal reading, the 5th-bar close, stop armed on the entry day, trading-day count | #240 0/6 = 0.0% | printed: Printout p.6 (Test 22, only Januarys with 22 trading days): 8 trades, 87% winners (7), net $7,312.50, average $914.06. | engine 6 tr 100% net $8,416.56 avg $1,402.76
Row 6: Swiss Franc (&6S) | Long TD22+1 of Jan | level: stop at the open + 0.05% of the previous range (bar after the TD) | stop/exit: close 5 bars after entry, $1,800 stop | 4 labelled details: the 0.05% literal reading, the 5th-bar close, stop armed on the entry day, trading-day count | #240 0/7 = 0.0% | printed: Printout p.6 (Test 22): 7 trades, 86% winners (6), net $4,375, average $625.00. | engine 7 tr 86% net $4,109.87 avg $587.12
Row 7: Wheat (&ZW) | Long TD22+1 of Jan | level: stop at the open + 0.05% of the previous range (bar after the TD) | stop/exit: close 5 bars after entry, $1,800 stop | 4 labelled details: the 0.05% literal reading, the 5th-bar close, stop armed on the entry day, trading-day count | #240 0/7 = 0.0% | printed: Printout p.6 (Test 22): 8 trades, 75% winners (6), net $2,512.50, average $314.06. | engine 6 tr 67% net $1,399.02 avg $233.17
Row 8: Gold (&GC) | Long TD6 of Jan | level: stop at the open + 0.05% of the previous range | stop/exit: close 5 bars after entry, $1,800 stop | 4 labelled details: the 0.05% literal reading, the 5th-bar close, stop armed on the entry day, trading-day count | #240 0/41 = 0.0% | printed: Text p.6 (no printout): 12 of 19 years (63%), net $16,550 ($13,230 from one trade; average $871 = net / 19, derived). | engine 17 tr 65% net $17,995.34 avg $1,058.55"##,
     [r##"22"##, r##"63% (14)"##, r##"$950.06 ($20,901.25)"##, r##"not printed"##]),
    (r##"lwnl_jan97_bonds"##, r##"T-Bonds January 1997: buy TD7 / sell TD2 (2 printed rows)"##, r##"Long/Short"##, r##"&ZB"##,
     &[r##"E|"The best Bond seasonal I could find suggests buying on the opening of the 7th trading day and exiting 8 days later ... The sell is to go short on the opening of the second trade day of January. We used an $1,800 stop for the 68% accuracy and $610 per trade" (Vol. 34 No. 1 = January 1997, p.8)"##, r##"E|Adaptation, not counted: the Norgate back-adjusted continuous contract (the CSI contract file is not held)"##, r##"E|Trading day (TD) of each row = the vault's count of the month's sessions (an unstated detail of every row; the row's own TD is in the row table)"##, r##"X|Row-specific labelled details (at most 4 per row, #339/#343) and the per-row level, stop, exit, #240 figure and printed record are in the row table in the notes"##, r##"E|Row parameter: 'row' picks the printed row (default 1 = the first printed row); the card runs on the Norgate contract of that row's market only, and a row on another market is refused with a message listing that market's rows"##],
     r##"T-Bonds January 1997: buy TD7 / sell TD2 (2 printed rows). Source: Vol. 34 No. 1 (January 1997) p.8. 2 printed rows; ONE card with a row parameter (R4 #375). Rows above the 5% order-dependence line (#175, #240) are not coded.
Row 1: US T-Bond (&ZB) | Long TD7 of Jan | level: open | stop/exit: close 8 bars after entry, $1,800 stop | 4 labelled details: the 8-bar close exit, the stop read as applying to the buy and armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: Printout p.8 (Test 6, LONGS; window and costs not printed, the page's recap runs to 12/31/96): 17 trades, 58% winners (10), net $11,181.25, average $657.72, ratio 1.87. | engine 18 tr 56% net $1,368.02 avg $76.00
Row 2: US T-Bond (&ZB) | Short TD2 of Jan | level: open | stop/exit: close 8 bars after entry, $1,800 stop | 3 labelled details: the buy's 8-bar close exit applied to the sell, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: Printout p.8 (Test 5, SHORTS; window and costs not printed): 16 trades, 68% winners (11), net $9,762.50, average $610.16, ratio 1.61. | engine 18 tr 56% net $3,319.00 avg $184.39"##,
     [r##"17"##, r##"58% (10)"##, r##"$657.72 ($11,181.25)"##, r##"not printed"##]),
    (r##"lwnl_apr96_list"##, r##"Williams April seasonals list (March 1996 issue): 9 printed rows"##, r##"Long/Short"##, r##"&HG"##,
     &[r##"E|"The rules for what follows are to buy/sell on the opening + 10% of previous days range ON THE NTH TRADING DAY OF THE MONTH AS INDICATED, exit on close 3 days later or stopped out at $2,500" (Vol. 31 No. 3 = March 1996, p.4, APRIL SEASONALS)"##, r##"X|Unstated in the letter: 'exit on close 3 days later' = the close of the 3rd bar after the entry bar"##, r##"X|Unstated in the letter: the $2,500 stop is armed on the entry day (converted at the market's own US$ per point)"##, r##"E|Adaptation, not counted: the Norgate back-adjusted continuous contract (the CSI contract file is not held)"##, r##"E|Trading day (TD) of each row = the vault's count of the month's sessions (an unstated detail of every row; the row's own TD is in the row table)"##, r##"X|Row-specific labelled details (at most 4 per row, #339/#343) and the per-row level, stop, exit, #240 figure and printed record are in the row table in the notes"##, r##"E|Row parameter: 'row' picks the printed row (default 1 = the first printed row); the card runs on the Norgate contract of that row's market only, and a row on another market is refused with a message listing that market's rows"##],
     r##"Williams April seasonals list (March 1996 issue): 9 printed rows. Source: Vol. 31 No. 3 (March 1996) p.4. 9 printed rows; ONE card with a row parameter (R4 #375). Rows above the 5% order-dependence line (#175, #240) are not coded.
Row 1: Copper (&HG) | Short TD3 of Apr | level: stop at the open - 10% of the previous range | stop/exit: close 3 bars after entry, $2,500 stop | 4 labelled details: sell-stop sign (sells: open - 10%), 3-bar close, stop armed on the entry day, trading-day count | #240 2/41 = 4.9% (one trade under the 5% line) | printed: Text p.4 (no printout; 'last 16 years' where stated; trade counts and dollars not printed): 75% winners, $318 a trade. | engine 16 tr 62% net $2,632.50 avg $164.53
Row 2: Coffee (&KC) | Short TD17 of Apr | level: stop at the open - 10% of the previous range | stop/exit: close 3 bars after entry, $2,500 stop | 4 labelled details: sell-stop sign (sells: open - 10%), 3-bar close, stop armed on the entry day, trading-day count | #240 1/36 = 2.8% | printed: Text p.4 (no printout; 'last 16 years' where stated; trade counts and dollars not printed): 80% winners, $1,000 a trade (one figure for days 17 and 18). | engine 13 tr 62% net $7,726.88 avg $594.38
Row 3: Coffee (&KC) | Short TD18 of Apr | level: stop at the open - 10% of the previous range | stop/exit: close 3 bars after entry, $2,500 stop | 4 labelled details: sell-stop sign (sells: open - 10%), 3-bar close, stop armed on the entry day, trading-day count | #240 0/32 = 0.0% | printed: Text p.4 (no printout; 'last 16 years' where stated; trade counts and dollars not printed): 80% winners, $1,000 a trade (one figure for days 17 and 18). | engine 10 tr 90% net $11,013.75 avg $1,101.38
Row 4: Gold (&GC) | Short TD18 of Apr | level: stop at the open - 10% of the previous range | stop/exit: close 3 bars after entry, $2,500 stop | 4 labelled details: sell-stop sign (sells: open - 10%), 3-bar close, stop armed on the entry day, trading-day count | #240 1/35 = 2.9% | printed: Text p.4 (no printout; 'last 16 years' where stated; trade counts and dollars not printed): 55% winners, $538 a trade. | engine 15 tr 67% net $10,899.00 avg $726.60
Row 5: British Pound (&6B) | Long TD7 of Apr | level: stop at the open + 10% of the previous range | stop/exit: close 3 bars after entry, $2,500 stop | 3 labelled details: 3-bar close, stop armed on the entry day, trading-day count (buys: no sign detail) | #240 0/41 = 0.0% | printed: Text p.4 (no printout; 'last 16 years' where stated; trade counts and dollars not printed): 65% accurate, about $250 a trade (one figure for the four currencies; D-Mark is not a held market). | engine 16 tr 69% net $12,010.62 avg $750.66
Row 6: Japanese Yen (&6J) | Long TD7 of Apr | level: stop at the open + 10% of the previous range | stop/exit: close 3 bars after entry, $2,500 stop | 3 labelled details: 3-bar close, stop armed on the entry day, trading-day count (buys: no sign detail) | #240 0/37 = 0.0% | printed: Text p.4 (no printout; 'last 16 years' where stated; trade counts and dollars not printed): 65% accurate, about $250 a trade (one figure for the four currencies; D-Mark is not a held market). | engine 16 tr 56% net $8,231.25 avg $514.45
Row 7: Swiss Franc (&6S) | Long TD7 of Apr | level: stop at the open + 10% of the previous range | stop/exit: close 3 bars after entry, $2,500 stop | 3 labelled details: 3-bar close, stop armed on the entry day, trading-day count (buys: no sign detail) | #240 0/40 = 0.0% | printed: Text p.4 (no printout; 'last 16 years' where stated; trade counts and dollars not printed): 65% accurate, about $250 a trade (one figure for the four currencies; D-Mark is not a held market). | engine 15 tr 53% net $7,370.00 avg $491.33
Row 8: US T-Bond (&ZB) | Short TD11 of Apr | level: stop at the open - 10% of the previous range | stop/exit: close 3 bars after entry, $2,500 stop | 4 labelled details: the sell-stop sign, the 3-bar close exit, stop armed on the entry day, trading-day count | #240 0/34 = 0.0% | printed: Text p.4 (no printout; trade counts and dollars not printed): 66% winners, $600 a trade (one figure for days 11 and 13). | engine 11 tr 73% net $6,257.90 avg $568.90
Row 9: US T-Bond (&ZB) | Short TD13 of Apr | level: stop at the open - 10% of the previous range | stop/exit: close 3 bars after entry, $2,500 stop | 4 labelled details: the sell-stop sign, the 3-bar close exit, stop armed on the entry day, trading-day count | #240 0/34 = 0.0% | printed: Text p.4 (no printout; trade counts and dollars not printed): 66% winners, $600 a trade (one figure for days 11 and 13). | engine 12 tr 67% net $5,577.91 avg $464.83"##,
     [r##"not printed"##, r##"75%"##, r##"$318 (per trade)"##, r##"last 16 years"##]),
    (r##"lwnl_wheat_sep_td1_5_buy"##, r##"Wheat: buy at the open + 40% of the previous range within the first 5 trading days of September"##, r##"Long"##, r##"&ZW"##,
     &[r##"E|"The first is to get back into Wheat during the first 5 trading days of September---if---price rallies 40% of the previous days range added to today's opening. If this has not taken place within the first 5 trading days, pass on the deal. Use a stop of $1,100 and exit on the close 33 days later" (Vol. 33 No. 8 = August 1996, p.3, DOUBLE DITTO SEASONALS FOR SEPTEMBER): a buy stop at the open + 0.4 x the previous bar's range on trading days 1 to 5 of September, until it fills"##, r##"X|Unstated in the letter (1 of 4): 'the close 33 days later' = the close of the 33rd bar after the entry bar"##, r##"X|Unstated in the letter (2 of 4): the $1,100 stop is armed on the entry day (converted at the market's own US$ per point)"##, r##"X|Unstated in the letter (3 of 4): one entry a September: after a fill (or a stop-out) the order is not placed again that month"##, r##"E|Unstated in the letter (4 of 4): trading day 1 to 5 by the vault's count of the month's sessions"##, r##"E|Adaptation, not counted: the Norgate back-adjusted continuous contract (the CSI contract file is not held)"##],
     r##"Printout p.3 (Test 1; window and costs not printed, 'since' the first Septembers; the printout's last year is the 1995 season): 25 trades, 80% winners (20), net $14,675, average $587.00."##,
     [r##"25"##, r##"80% (20)"##, r##"$587.00 ($14,675)"##, r##"not printed"##]),
    (r##"lwnl_ho_mar_buy"##, r##"Heating oil March: buy the 7th or 9th trading day (one printout, 2 rows)"##, r##"Long"##, r##"&HO"##,
     &[r##"E|"A simple rule has done well for buying H. OIL in March. It is to buy on the 7th or 9th trading day of the month (not calendar day, which is 3/11 and 3/13) with a $1,100 stop and exit 30 days later" (Vol. 34 No. 3 = March 1997, p.3): the printout is one test for both days, so the card has one row per day (buy the open)"##, r##"X|Unstated in the letter: the entry is at the open (the letter says only 'buy on the 7th or 9th trading day')"##, r##"X|Unstated in the letter: 'exit 30 days later' = the close of the 30th bar after the entry bar (calendar days are the other reading; not run)"##, r##"X|Unstated in the letter: the $1,100 stop is armed on the entry day (converted at the market's own US$ per point)"##, r##"E|Adaptation, not counted: the Norgate back-adjusted continuous contract (the CSI contract file is not held)"##, r##"E|Trading day (TD) of each row = the vault's count of the month's sessions (an unstated detail of every row; the row's own TD is in the row table)"##, r##"X|Row-specific labelled details (at most 4 per row, #339/#343) and the per-row level, stop, exit, #240 figure and printed record are in the row table in the notes"##, r##"E|Row parameter: 'row' picks the printed row (default 1 = the first printed row); the card runs on the Norgate contract of that row's market only, and a row on another market is refused with a message listing that market's rows"##],
     r##"Heating oil March: buy the 7th or 9th trading day (one printout, 2 rows). Source: Vol. 34 No. 3 (March 1997) p.3. 2 printed rows; ONE card with a row parameter (R4 #375). Rows above the 5% order-dependence line (#175, #240) are not coded.
Row 1: Heating Oil (&HO) | Long TD7 of Mar | level: open | stop/exit: close 30 bars after entry, $1,100 stop | 4 labelled details: open entry, 30-bar close, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: Printout p.3 (HEATING OIL, 05/14/87 - 02/21/97, commission $50; the page prints ONE test for 'the 7th or 9th' without saying which day): 8 trades, 100% winners, net $18,953.60, average $2,369.20; compared with both rows. | commission $50 per trade printed and deducted (#363) | engine 9 tr 78% net $16,233.24 avg $1,803.69
Row 2: Heating Oil (&HO) | Long TD9 of Mar | level: open | stop/exit: close 30 bars after entry, $1,100 stop | 4 labelled details: open entry, 30-bar close, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: Printout p.3 (HEATING OIL, 05/14/87 - 02/21/97, commission $50; the page prints ONE test for 'the 7th or 9th' without saying which day): 8 trades, 100% winners, net $18,953.60, average $2,369.20; compared with both rows. | commission $50 per trade printed and deducted (#363) | engine 9 tr 89% net $16,098.42 avg $1,788.71"##,
     [r##"8"##, r##"100% (8)"##, r##"$2,369.20 ($18,953.60)"##, r##"05/1987 to 02/1997"##]),
    (r##"lwnl_bonds_oct"##, r##"T-Bonds October: buy TD9 / sell TD17 (one test, 2 rows)"##, r##"Long/Short"##, r##"&ZB"##,
     &[r##"E|"The entry rule is the same for both markets; buy long on the 9th trading (not calendar) day of the month ... for Bonds, three days after entry with a $900 stop. The sell date is the 17th trading day of the month with the same exit dates and targets" (Vol. 33 No. 9 = September 1996, p.3, OCTOBER OPPORTUNITIES)"##, r##"X|Unstated in the letter: the entry is at the open (the letter says only 'buy long on the 9th trading day'; the sell date has 'the same exit dates and targets')"##, r##"X|Unstated in the letter: 'three days after entry' = the close of the 3rd bar after the entry bar"##, r##"X|Unstated in the letter: the $900 stop is armed on the entry day (converted at the market's own US$ per point)"##, r##"E|Adaptation, not counted: the Norgate back-adjusted continuous contract (the CSI contract file is not held)"##, r##"E|Trading day (TD) of each row = the vault's count of the month's sessions (an unstated detail of every row; the row's own TD is in the row table)"##, r##"X|Row-specific labelled details (at most 4 per row, #339/#343) and the per-row level, stop, exit, #240 figure and printed record are in the row table in the notes"##, r##"E|Row parameter: 'row' picks the printed row (default 1 = the first printed row); the card runs on the Norgate contract of that row's market only, and a row on another market is refused with a message listing that market's rows"##],
     r##"T-Bonds October: buy TD9 / sell TD17 (one test, 2 rows). Source: Vol. 33 No. 9 (September 1996) p.3-4. 2 printed rows; ONE card with a row parameter (R4 #375). Rows above the 5% order-dependence line (#175, #240) are not coded.
Row 1: US T-Bond (&ZB) | Long TD9 of Oct | level: open | stop/exit: close 3 bars after entry, $900 stop | 4 labelled details: entry at the open, the 3-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: Printout p.4 (DAY T-BONDS 01/01/80 - 09/06/96, commission $50, slippage 0; one test for both trades: ALL 32 trades, 72% winners, net $11,743.75, average $685.05): LONG TRADES row of the printout: 16 trades, 13 winners (81%), net $6,637.50, average $414.84. | commission $50 per trade printed and deducted (#363) | engine 16 tr 69% net $2,123.98 avg $132.75
Row 2: US T-Bond (&ZB) | Short TD17 of Oct | level: open | stop/exit: close 3 bars after entry, $900 stop | 4 labelled details: entry at the open, the 3-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: Printout p.4 (DAY T-BONDS 01/01/80 - 09/06/96, commission $50, slippage 0; one test for both trades: ALL 32 trades, 72% winners, net $11,743.75, average $685.05): SHORT side DERIVED as ALL TRADES minus LONG TRADES (the short block is cut off the page): 16 trades, 10 winners (63%), net $5,106.25, average $319.14. | commission $50 per trade printed and deducted (#363) | engine 16 tr 62% net $3,963.98 avg $247.75"##,
     [r##"16"##, r##"81%"##, r##"$414.84 ($6,637.50)"##, r##"01/1980 to 09/1996"##]),
    (r##"lwnl_apr97_7day"##, r##"April 1997 7-day hold table (25 printed rows)"##, r##"Long/Short"##, r##"&LE"##,
     &[r##"E|"The best seasonal 7 day hold trades for April. The entry rule is to buy on the trading day (not calendar) of the month shown with a $1,200 stop or exit on the close 7 trading sessions later. I'm listing the buys, then the sells" (Vol. 34 No. 4 = April 1997, p.2, SEASONAL TRADES FOR APRIL)"##, r##"X|Unstated in the letter: the entry is at the open (the table says only 'buy on the trading day of the month shown'; sells likewise)"##, r##"E|Adaptation, not counted: the Norgate back-adjusted continuous contract (the CSI contract file is not held)"##, r##"E|Trading day (TD) of each row = the vault's count of the month's sessions (an unstated detail of every row; the row's own TD is in the row table)"##, r##"X|Row-specific labelled details (at most 4 per row, #339/#343) and the per-row level, stop, exit, #240 figure and printed record are in the row table in the notes"##, r##"E|Row parameter: 'row' picks the printed row (default 1 = the first printed row); the card runs on the Norgate contract of that row's market only, and a row on another market is refused with a message listing that market's rows"##],
     r##"April 1997 7-day hold table (25 printed rows). Source: Vol. 34 No. 4 (April 1997) p.2. 25 printed rows; ONE card with a row parameter (R4 #375). Rows above the 5% order-dependence line (#175, #240) are not coded.
Row 1: Live Cattle (&LE) | Long TD20 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/37 = 0.0% | printed: CATTLE buy day 20: 10 trades, 80% winners (8), net $5,468, printed average $526 (net / trades = $547); window and costs not printed. | engine 16 tr 69% net $3,690.00 avg $230.62
Row 2: Cocoa (&CC) | Long TD6 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/41 = 0.0% | printed: COCOA buy day 6: 16 trades, 62% winners (10), net $4,380, printed average $273 (net / trades = $274); window and costs not printed. | engine 17 tr 65% net $3,080.00 avg $181.18
Row 3: Cotton (&CT) | Long TD2 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: COTTON buy day 2: 10 trades, 80% winners (8), net $13,620, printed average $1,362 (net / trades = $1,362); window and costs not printed. | engine 18 tr 61% net $13,795.00 avg $766.39
Row 4: Copper (&HG) | Long TD10 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: COPPER buy day 10: 7 trades, 85% winners (6), net $2,437, printed average $348 (net / trades = $348); window and costs not printed. | engine 18 tr 39% net $-5,237.50 avg $-290.97
Row 5: Coffee (&KC) | Long TD9 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: COFFEE buy day 9: 19 trades, 57% winners (11), net $14,305, printed average $752 (net / trades = $753); window and costs not printed. | engine 18 tr 39% net $1,800.00 avg $100.00
Row 6: Orange Juice (&OJ) | Long TD20 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/37 = 0.0% | printed: JUICE buy day 20: 14 trades, 78% winners (11), net $5,450, printed average $389 (net / trades = $389); window and costs not printed. | engine 16 tr 69% net $5,505.00 avg $344.06
Row 7: Silver (&SI) | Long TD21 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/27 = 0.0% | printed: SILVER buy day 21: 11 trades, 63% winners (7), net $6,683, printed average $607 (net / trades = $608); window and costs not printed. | engine 10 tr 40% net $-3,190.00 avg $-319.00
Row 8: Soybeans (&ZS) | Long TD10 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/43 = 0.0% | printed: BEANS buy day 10: 24 trades, 70% winners (17), net $5,750, printed average $239 (net / trades = $240); window and costs not printed. | engine 19 tr 74% net $6,275.00 avg $330.26
Row 9: Sugar No. 11 (&SB) | Long TD10 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: SUGAR buy day 10: 9 trades, 66% winners (6), net $3,122, printed average $346 (net / trades = $347); window and costs not printed. | engine 18 tr 56% net $2,080.00 avg $115.56
Row 10: British Pound (&6B) | Long TD6 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: POUND buy day 6: 20 trades, 65% winners (13), net $9,931, printed average $496 (net / trades = $497); window and costs not printed. | engine 18 tr 67% net $11,450.00 avg $636.11
Row 11: Heating Oil (&HO) | Long TD3 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: H OIL buy day 3: 9 trades, 66% winners (6), net $5,068, printed average $563 (net / trades = $563); window and costs not printed. | engine 18 tr 61% net $6,811.85 avg $378.44
Row 12: US T-Bond (&ZB) | Long TD4 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/43 = 0.0% | printed: BONDS buy day 4: 17 trades, 52% winners (9), net $12,056, printed average $708 (net / trades = $709); window and costs not printed. | engine 19 tr 37% net $5,512.99 avg $290.16
Row 13: US T-Bond (&ZB) | Long TD19 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/43 = 0.0% | printed: BONDS buy day 19: 14 trades, 64% winners (9), net $8,237, printed average $588 (net / trades = $588); window and costs not printed. | engine 19 tr 53% net $10,200.98 avg $536.89
Row 14: Live Cattle (&LE) | Short TD4 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: CATTLE sell day 4: 12 trades, 91% winners (11), net $6,312, printed average $526 (net / trades = $526); window and costs not printed. | engine 18 tr 67% net $4,000.00 avg $222.22
Row 15: Cocoa (&CC) | Short TD16 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/41 = 0.0% | printed: COCOA sell day 16: 14 trades, 64% winners (9), net $4,090, printed average $292 (net / trades = $292); window and costs not printed. | engine 17 tr 59% net $2,870.00 avg $168.82
Row 16: Copper (&HG) | Short TD14 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: COPPER sell day 14: 8 trades, 75% winners (6), net $2,375, printed average $296 (net / trades = $297); window and costs not printed. | engine 18 tr 39% net $-1,375.00 avg $-76.39
Row 17: Orange Juice (&OJ) | Short TD5 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: JUICE sell day 5: 12 trades, 75% winners (9), net $5,542, printed average $461 (net / trades = $462); window and costs not printed. | engine 18 tr 39% net $1,267.50 avg $70.42
Row 18: Sugar No. 11 (&SB) | Short TD2 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: SUGAR sell day 2: 7 trades, 85% winners (6), net $4,421, printed average $631 (net / trades = $632); window and costs not printed. | engine 18 tr 72% net $8,072.00 avg $448.44
Row 19: Wheat (&ZW) | Short TD15 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/43 = 0.0% | printed: WHEAT sell day 15: 26 trades, 61% winners (16), net $3,362, printed average $130 (net / trades = $129); window and costs not printed. | engine 19 tr 47% net $-312.50 avg $-16.45
Row 20: Swiss Franc (&6S) | Short TD7 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: S FRANC sell day 7: 20 trades, 65% winners (13), net $7,025, printed average $351 (net / trades = $351); window and costs not printed. | engine 18 tr 33% net $-5,850.00 avg $-325.00
Row 21: Gold (&GC) | Short TD8 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/43 = 0.0% | printed: GOLD sell day 8: 20 trades, 65% winners (13), net $4,730, printed average $236 (net / trades = $236); window and costs not printed. | engine 19 tr 47% net $2,230.00 avg $117.37
Row 22: Heating Oil (&HO) | Short TD16 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/42 = 0.0% | printed: H OIL sell day 16: 9 trades, 66% winners (6), net $3,200, printed average $354 (net / trades = $356); window and costs not printed. | engine 18 tr 50% net $-614.96 avg $-34.16
Row 23: US T-Bond (&ZB) | Short TD11 of Apr | level: open | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/43 = 0.0% | printed: BONDS sell day 11: 19 trades, 68% winners (13), net $13,831, printed average $727 (net / trades = $728); window and costs not printed. | engine 19 tr 58% net $5,843.00 avg $307.53
Row 24: US T-Bond (&ZB) | Short TD11 of Apr | level: open | stop/exit: close 6 bars after entry, $1,200 stop | 4 labelled details: entry at the open, the 6-bar close exit, stop armed on the entry day, trading-day count | #240 0/43 = 0.0% | printed: BONDS sell day 11: 19 trades, 78% winners (15), net $15,018, no average printed (the column carries the annotation; net / trades used) (net / trades = $790); window and costs not printed. | engine 19 tr 53% net $5,875.01 avg $309.21
Row 25: US T-Bond (&ZB) | Short TD9 of Apr | level: open | stop/exit: close 7 bars after entry, $1,600 stop | 4 labelled details: entry at the open, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/43 = 0.0% | printed: BONDS sell day 9: 19 trades, 84% winners (16), net $16,143, no average printed (the column carries the annotation; net / trades used) (net / trades = $850); window and costs not printed. | engine 19 tr 53% net $3,573.99 avg $188.10"##,
     [r##"10"##, r##"80% (8)"##, r##"$526 ($5,468)"##, r##"not printed"##]),
    (r##"lwnl_may97"##, r##"May 1997 seasonal table (52 printed rows, 7 or 15 days held, no stop)"##, r##"Long/Short"##, r##"&LE"##,
     &[r##"E|"The best seasonals for May. The tabulations show the commodity, then the trading day, not calendar day of the month to B/S, next how many days to hold, then $ profits, # Trades, % winners, drawdown and finally average profit per trade" (Vol. 34 No. 5 = May 1997, p.3, SEASONAL TRADES FOR THE MONTH OF MAY)"##, r##"X|Unstated in the letter: the entry is at the open (the table says only 'buy on the trading day of the month shown'; sells likewise)"##, r##"X|Unstated in the letter: no stop is printed for this table: none is applied (the DRDWN column is a result, not a rule)"##, r##"E|Adaptation, not counted: the Norgate back-adjusted continuous contract (the CSI contract file is not held)"##, r##"E|Trading day (TD) of each row = the vault's count of the month's sessions (an unstated detail of every row; the row's own TD is in the row table)"##, r##"X|Row-specific labelled details (at most 4 per row, #339/#343) and the per-row level, stop, exit, #240 figure and printed record are in the row table in the notes"##, r##"E|Row parameter: 'row' picks the printed row (default 1 = the first printed row); the card runs on the Norgate contract of that row's market only, and a row on another market is refused with a message listing that market's rows"##],
     r##"May 1997 seasonal table (52 printed rows, 7 or 15 days held, no stop). Source: Vol. 34 No. 5 (May 1997) p.3-4. 52 printed rows; ONE card with a row parameter (R4 #375). Rows above the 5% order-dependence line (#175, #240) are not coded.
Row 1: Live Cattle (&LE) | Long TD21 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/30 = 0.0% | printed: CATTLE buy day 21, held 7 days: 8 trades, 62% winners (5), net $2,664, printed average $333 (net / trades = $333); window and costs not printed. | engine 13 tr 46% net $820.00 avg $63.08
Row 2: Cotton (&CT) | Long TD10 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: COTTON buy day 10, held 7 days: 10 trades, 70% winners (7), net $6,090, printed average $609 (net / trades = $609); window and costs not printed. | engine 18 tr 50% net $1,785.00 avg $99.17
Row 3: Cotton (&CT) | Long TD5 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: COTTON buy day 5, held 15 days: 9 trades, 66% winners (6), net $12,740, printed average $1,415 (net / trades = $1,416); window and costs not printed. | engine 18 tr 56% net $9,015.00 avg $500.83
Row 4: Copper (&HG) | Long TD19 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: COPPER buy day 19, held 15 days: 8 trades, 75% winners (6), net $4,628, printed average $575 (net / trades = $578); window and costs not printed. | engine 19 tr 42% net $-3,512.50 avg $-184.87
Row 5: Copper (&HG) | Long TD20 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: COPPER buy day 20, held 15 days: 7 trades, 71% winners (5), net $49,375, printed average $705 (net / trades = $7,054); window and costs not printed. PRINTED FIGURES INCONSISTENT (average x trades = 4,935 is not the printed profit): recorded as printed. | engine 19 tr 47% net $-200.00 avg $-10.53 | OCR-DOUBTFUL: printed net $49,375 = 10 x (average x trades = $4,935); arithmetic value about $4,937; printed figure kept
Row 6: Corn (&ZC) | Long TD20 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: CORN buy day 20, held 15 days: 22 trades, 54% winners (12), net $6,212, printed average $282 (net / trades = $282); window and costs not printed. | engine 19 tr 58% net $6,837.50 avg $359.87
Row 7: Corn (&ZC) | Long TD21 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/31 = 0.0% | printed: CORN buy day 21, held 15 days: 17 trades, 52% winners (9), net $9,206, printed average $541 (net / trades = $542); window and costs not printed. | engine 14 tr 64% net $8,025.00 avg $573.21
Row 8: Coffee (&KC) | Long TD5 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: COFFEE buy day 5, held 7 days: 20 trades, 70% winners (14), net $14,637, printed average $731 (net / trades = $732); window and costs not printed. | engine 18 tr 67% net $10,856.25 avg $603.13
Row 9: Coffee (&KC) | Long TD4 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: COFFEE buy day 4, held 15 days: 21 trades, 52% winners (11), net $18,915, printed average $900 (net / trades = $901); window and costs not printed. | engine 18 tr 50% net $-5,681.25 avg $-315.63
Row 10: Orange Juice (&OJ) | Long TD13 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: JUICE buy day 13, held 7 days: 14 trades, 64% winners (9), net $2,600, printed average $185 (net / trades = $186); window and costs not printed. | engine 18 tr 67% net $3,675.00 avg $204.17
Row 11: Orange Juice (&OJ) | Long TD1 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: JUICE buy day 1, held 7 days: 19 trades, 68% winners (13), net $2,155, printed average $113 (net / trades = $113); window and costs not printed. | engine 18 tr 72% net $4,800.00 avg $266.67
Row 12: Silver (&SI) | Long TD4 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: SILVER buy day 4, held 7 days: 23 trades, 52% winners (12), net $13,040, printed average $569 (net / trades = $567); window and costs not printed. | engine 19 tr 47% net $7,490.00 avg $394.21
Row 13: Soybeans (&ZS) | Long TD4 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: BEANS buy day 4, held 7 days: 25 trades, 64% winners (16), net $14,150, printed average $566 (net / trades = $566); window and costs not printed. | engine 19 tr 42% net $1,925.00 avg $101.32
Row 14: Soybeans (&ZS) | Long TD19 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: BEANS buy day 19, held 15 days: 23 trades, 52% winners (12), net $21,037, printed average $914 (net / trades = $915); window and costs not printed. | engine 19 tr 32% net $2,850.00 avg $150.00
Row 15: Sugar No. 11 (&SB) | Long TD21 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/30 = 0.0% | printed: SUGAR buy day 21, held 7 days: 7 trades, 85% winners (6), net $2,700, printed average $385 (net / trades = $386); window and costs not printed. | engine 13 tr 62% net $-5,611.20 avg $-431.63
Row 16: Sugar No. 11 (&SB) | Long TD19 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: SUGAR buy day 19, held 15 days: 8 trades, 62% winners (5), net $5,492, printed average $686 (net / trades = $686); window and costs not printed. | engine 18 tr 33% net $-5,275.20 avg $-293.07
Row 17: Wheat (&ZW) | Long TD22 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/19 = 0.0% | printed: WHEAT buy day 22, held 7 days: 10 trades, 60% winners (6), net $3,256, printed average $325 (net / trades = $326); window and costs not printed. | engine 9 tr 56% net $1,075.00 avg $119.44
Row 18: Wheat (&ZW) | Long TD21 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/31 = 0.0% | printed: WHEAT buy day 21, held 15 days: 17 trades, 58% winners (10), net $10,481, printed average $616 (net / trades = $617); window and costs not printed. | engine 14 tr 57% net $5,700.00 avg $407.14
Row 19: British Pound (&6B) | Long TD15 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: POUND buy day 15, held 7 days: 21 trades, 52% winners (11), net $7,237, printed average $344 (net / trades = $345); window and costs not printed. | engine 18 tr 44% net $-7,962.50 avg $-442.36
Row 20: British Pound (&6B) | Long TD14 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: POUND buy day 14, held 7 days: 19 trades, 57% winners (11), net $3,262, printed average $171 (net / trades = $172); window and costs not printed. | engine 18 tr 50% net $-3,412.50 avg $-189.58
Row 21: Gold (&GC) | Long TD4 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: GOLD buy day 4, held 7 days: 21 trades, 66% winners (14), net $6,840, printed average $325 (net / trades = $326); window and costs not printed. | engine 19 tr 58% net $5,260.00 avg $276.84
Row 22: Gold (&GC) | Long TD3 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: GOLD buy day 3, held 7 days: 20 trades, 65% winners (13), net $5,350, printed average $267 (net / trades = $268); window and costs not printed. | engine 19 tr 53% net $4,740.00 avg $249.47
Row 23: Heating Oil (&HO) | Long TD2 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: H OIL buy day 2, held 15 days: 9 trades, 77% winners (7), net $2,540, printed average $282 (net / trades = $282); window and costs not printed. | engine 18 tr 67% net $17,463.60 avg $970.20
Row 24: Heating Oil (&HO) | Long TD13 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: H OIL buy day 13, held 7 days: 9 trades, 66% winners (6), net $1,860, printed average $206 (net / trades = $207); window and costs not printed. | engine 18 tr 61% net $7,849.80 avg $436.10
Row 25: US T-Bond (&ZB) | Long TD9 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: BONDS buy day 9, held 15 days: 7 trades, 55% winners (6), net $11,681, printed average $1,668 (net / trades = $1,669); window and costs not printed. PRINTED FIGURES INCONSISTENT (55% is not 6/7): recorded as printed. | engine 19 tr 53% net $15,122.94 avg $795.94 | OCR-DOUBTFUL (bonds TD9): 55% printed with 6 winners of 7 trades (6/7 = 86%); printed figure kept, win rate also compared against 86%
Row 26: US T-Bond (&ZB) | Long TD8 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: BONDS buy day 8, held 15 days: 6 trades, 66% winners (4), net $11,262, printed average $1,877 (net / trades = $1,877); window and costs not printed. | engine 19 tr 58% net $8,095.00 avg $426.05
Row 27: Live Cattle (&LE) | Short TD7 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: CATTLE sell day 7, held 7 days: 14 trades, 64% winners (9), net $53,280, printed average $380 (net / trades = $3,806); window and costs not printed. PRINTED FIGURES INCONSISTENT (average x trades = 5,320 is not the printed profit): recorded as printed. | engine 18 tr 72% net $8,360.00 avg $464.44 | OCR-DOUBTFUL: printed net $53,280 = 10 x (average x trades = $5,320); arithmetic value about $5,328; printed figure kept
Row 28: Live Cattle (&LE) | Short TD4 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: CATTLE sell day 4, held 15 days: 13 trades, 76% winners (10), net $4,010, printed average $308 (net / trades = $308); window and costs not printed. | engine 18 tr 67% net $5,480.00 avg $304.44
Row 29: Cocoa (&CC) | Short TD16 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/41 = 0.0% | printed: COCOA sell day 16, held 15 days: 13 trades, 97% winners (12), net $12,800, printed average $984 (net / trades = $985); window and costs not printed. PRINTED FIGURES INCONSISTENT (97% is not 12/13): recorded as printed. | engine 17 tr 76% net $11,760.00 avg $691.76 | OCR-DOUBTFUL: 97% printed with 12 winners of 13 trades (12/13 = 92%); printed figure kept, win rate also compared against 92%
Row 30: Cocoa (&CC) | Short TD18 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/41 = 0.0% | printed: COCOA sell day 18, held 15 days: 15 trades, 66% winners (10), net $13,340, printed average $889 (net / trades = $889); window and costs not printed. | engine 17 tr 82% net $14,150.00 avg $832.35
Row 31: Corn (&ZC) | Short TD14 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: CORN sell day 14, held 15 days: 7 trades, 71% winners (5), net $5,270, printed average $752 (net / trades = $753); window and costs not printed. | engine 19 tr 47% net $250.00 avg $13.16
Row 32: Coffee (&KC) | Short TD17 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: COFFEE sell day 17, held 7 days: 15 trades, 80% winners (12), net $250,550, printed average $1,670 (net / trades = $16,703); window and costs not printed. PRINTED FIGURES INCONSISTENT (average x trades = 25,050 is not the printed profit): recorded as printed. | engine 18 tr 78% net $31,293.75 avg $1,738.54 | OCR-DOUBTFUL: printed net $250,550 = 10 x (average x trades = $25,050); arithmetic value about $25,055; printed figure kept
Row 33: Coffee (&KC) | Short TD16 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: COFFEE sell day 16, held 15 days: 18 trades, 61% winners (11), net $62,205, printed average $3,455 (net / trades = $3,456); window and costs not printed. | engine 18 tr 72% net $29,925.00 avg $1,662.50
Row 34: Orange Juice (&OJ) | Short TD14 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: JUICE sell day 14, held 15 days: 11 trades, 9% winners (10), net $5,705, printed average $518 (net / trades = $519); window and costs not printed. PRINTED FIGURES INCONSISTENT (9% is not 10/11): recorded as printed. | engine 18 tr 61% net $3,412.50 avg $189.58 | OCR-DOUBTFUL: '9%' printed with 10 winners of 11 trades, most likely 91% (a lost digit); printed figure kept, win rate also compared against 10/11 = 91%
Row 35: Orange Juice (&OJ) | Short TD19 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: JUICE sell day 19, held 15 days: 15 trades, 53% winners (8), net $7,335, printed average $489 (net / trades = $489); window and costs not printed. | engine 18 tr 67% net $8,752.50 avg $486.25
Row 36: Silver (&SI) | Short TD13 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: SILVER sell day 13, held 15 days: 19 trades, 63% winners (12), net $23,575, printed average $1,240 (net / trades = $1,241); window and costs not printed. | engine 19 tr 68% net $2,625.00 avg $138.16
Row 37: Silver (&SI) | Short TD13 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: SILVER sell day 13, held 7 days: 19 trades, 63% winners (12), net $18,745, printed average $98 (net / trades = $987); window and costs not printed. PRINTED FIGURES INCONSISTENT (average x trades = 1,862 is not the printed profit): recorded as printed. | engine 19 tr 47% net $8,080.00 avg $425.26 | OCR-DOUBTFUL: printed average $98 x 19 trades = $1,862, not the printed net $18,745 (net / trades = $987); arithmetic average about $987; printed figures kept
Row 38: Soybeans (&ZS) | Short TD16 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: BEANS sell day 16, held 7 days: 23 trades, 65% winners (15), net $5,493, printed average $238 (net / trades = $239); window and costs not printed. | engine 19 tr 68% net $5,537.50 avg $291.45
Row 39: Soybeans (&ZS) | Short TD10 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: BEANS sell day 10, held 15 days: 25 trades, 60% winners (15), net $6,381, printed average $255 (net / trades = $255); window and costs not printed. | engine 19 tr 74% net $3,112.50 avg $163.82
Row 40: Sugar No. 11 (&SB) | Short TD13 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: SUGAR sell day 13, held 7 days: 7 trades, 85% winners (6), net $4,167, printed average $595 (net / trades = $595); window and costs not printed. | engine 18 tr 56% net $-2,643.20 avg $-146.84
Row 41: Wheat (&ZW) | Short TD5 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: WHEAT sell day 5, held 15 days: 20 trades, 70% winners (14), net $5,181, printed average $259 (net / trades = $259); window and costs not printed. | engine 19 tr 68% net $5,600.00 avg $294.74
Row 42: Wheat (&ZW) | Short TD15 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: WHEAT sell day 15, held 7 days: 24 trades, 62% winners (15), net $5,118, printed average $213 (net / trades = $213); window and costs not printed. | engine 19 tr 68% net $6,362.50 avg $334.87
Row 43: Swiss Franc (&6S) | Short TD3 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: S FRANC sell day 3, held 7 days: 17 trades, 76% winners (13), net $12,037, printed average $708 (net / trades = $708); window and costs not printed. | engine 18 tr 56% net $5,000.00 avg $277.78
Row 44: Swiss Franc (&6S) | Short TD7 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: S FRANC sell day 7, held 7 days: 19 trades, 63% winners (12), net $16,337, printed average $859 (net / trades = $860); window and costs not printed. | engine 18 tr 61% net $15,375.00 avg $854.17
Row 45: British Pound (&6B) | Short TD10 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: POUND sell day 10, held 7 days: 19 trades, 57% winners (11), net $8,506, printed average $447 (net / trades = $448); window and costs not printed. | engine 18 tr 44% net $4,425.00 avg $245.83
Row 46: British Pound (&6B) | Short TD19 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: POUND sell day 19, held 15 days: 17 trades, 52% winners (9), net $18,106, printed average $1,065 (net / trades = $1,065); window and costs not printed. | engine 18 tr 50% net $13,512.50 avg $750.69
Row 47: Gold (&GC) | Short TD13 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: GOLD sell day 13, held 15 days: 17 trades, 58% winners (10), net $10,400, printed average $611 (net / trades = $612); window and costs not printed. | engine 19 tr 53% net $-170.00 avg $-8.95
Row 48: Gold (&GC) | Short TD13 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/43 = 0.0% | printed: GOLD sell day 13, held 7 days: 17 trades, 58% winners (10), net $6,260, printed average $368 (net / trades = $368); window and costs not printed. | engine 19 tr 58% net $4,610.00 avg $242.63
Row 49: Japanese Yen (&6J) | Short TD7 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: YEN sell day 7, held 7 days: 19 trades, 63% winners (12), net $15,175, printed average $798 (net / trades = $799); window and costs not printed. | engine 18 tr 61% net $14,387.50 avg $799.31
Row 50: Japanese Yen (&6J) | Short TD6 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: YEN sell day 6, held 15 days: 18 trades, 61% winners (11), net $21,175, printed average $1,176 (net / trades = $1,176); window and costs not printed. | engine 18 tr 61% net $14,512.50 avg $806.25
Row 51: Heating Oil (&HO) | Short TD21 of May | level: open | stop/exit: close 7 bars after entry, no stop | 4 labelled details: entry at the open, the 7-bar close exit, no stop, trading-day count | #240 0/30 = 0.0% | printed: H OIL sell day 21, held 7 days: 7 trades, 85% winners (6), net $38,330, printed average $547 (net / trades = $5,476); window and costs not printed. PRINTED FIGURES INCONSISTENT (average x trades = 3,829 is not the printed profit): recorded as printed. | engine 13 tr 77% net $9,147.60 avg $703.66 | OCR-DOUBTFUL: printed net $38,330 = 10 x (average x trades = $3,829); arithmetic value about $3,833; printed figure kept
Row 52: Heating Oil (&HO) | Short TD19 of May | level: open | stop/exit: close 15 bars after entry, no stop | 4 labelled details: entry at the open, the 15-bar close exit, no stop, trading-day count | #240 0/42 = 0.0% | printed: H OIL sell day 19, held 15 days: 6 trades, 83% winners (5), net $5,181, printed average $863 (net / trades = $864); window and costs not printed. | engine 18 tr 78% net $11,499.60 avg $638.87"##,
     [r##"8"##, r##"62% (5)"##, r##"$333 ($2,664)"##, r##"not printed"##]),
    (r##"lwnl_jul97_7day"##, r##"July 1997 7-day table (17 printed rows)"##, r##"Long/Short"##, r##"&LE"##,
     &[r##"E|"The following tabulations show the commodity, best trading day of the month (not calendar day) to buy or sell, followed by number of trades, % winners, net profits, average profit per trade, drawdown, and largest win. The entry rule is to buy at 7% of the previous days range added to the opening on the enumerated day. The exit is a $1,500 stop or on the close 7 trading days later" (Vol. 34 No. 7 = July 1997, p.2, BACK TO WORK; BUYS and SELLS tables)"##, r##"E|Adaptation, not counted: the Norgate back-adjusted continuous contract (the CSI contract file is not held)"##, r##"E|Trading day (TD) of each row = the vault's count of the month's sessions (an unstated detail of every row; the row's own TD is in the row table)"##, r##"X|Row-specific labelled details (at most 4 per row, #339/#343) and the per-row level, stop, exit, #240 figure and printed record are in the row table in the notes"##, r##"E|Row parameter: 'row' picks the printed row (default 1 = the first printed row); the card runs on the Norgate contract of that row's market only, and a row on another market is refused with a message listing that market's rows"##],
     r##"July 1997 7-day table (17 printed rows). Source: Vol. 34 No. 7 (July 1997) p.2-3. 17 printed rows; ONE card with a row parameter (R4 #375). Rows above the 5% order-dependence line (#175, #240) are not coded.
Row 1: Live Cattle (&LE) | Long TD21 of Jul | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/27 = 0.0% | printed: CATTLE buy day 21: 13 trades, 84% winners (11), net $7,222, printed average $555 (net / trades = $556); window and costs not printed. | engine 12 tr 75% net $3,856.80 avg $321.40
Row 2: Live Cattle (&LE) | Long TD20 of Jul | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/36 = 0.0% | printed: CATTLE buy day 20: 14 trades, 85% winners (12), net $6,764, printed average $783 (net / trades = $483); window and costs not printed. PRINTED FIGURES INCONSISTENT (average x trades = 10,962 is not the printed profit): recorded as printed. | engine 14 tr 79% net $6,314.30 avg $451.02
Row 3: Cotton (&CT) | Long TD9 of Jul | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/36 = 0.0% | printed: COTTON buy day 9: 9 trades, 66% winners (6), net $5,035, printed average $559 (net / trades = $559); window and costs not printed. | engine 16 tr 69% net $5,469.30 avg $341.83
Row 4: Copper (&HG) | Long TD14 of Jul | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 2/40 = 5.0% (at the 5% line, not above) | printed: COPPER buy day 14: 8 trades, 75% winners (6), net $4,637, printed average $579 (net / trades = $580); window and costs not printed. | engine 18 tr 56% net $2,858.12 avg $158.78
Row 5: Orange Juice (&OJ) | Long TD18 of Jul | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/32 = 0.0% | printed: JUICE buy day 18: 11 trades, 68% winners (6), net $6,790, printed average $424 (net / trades = $617); window and costs not printed. PRINTED FIGURES INCONSISTENT (68% is not 6/11; average x trades = 4,664 is not the printed profit): recorded as printed. | engine 14 tr 71% net $4,363.20 avg $311.66
Row 6: Sugar No. 11 (&SB) | Long TD7 of Jul | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/41 = 0.0% | printed: SUGAR buy day 7: 9 trades, 66% winners (6), net $2,673, printed average $297 (net / trades = $297); window and costs not printed. | engine 18 tr 44% net $-723.94 avg $-40.22
Row 7: Swiss Franc (&6S) | Long TD9 of Jul | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 1/42 = 2.4% | printed: S FRANC buy day 9: 19 trades, 78% winners (15), net $11,862, printed average $624 (net / trades = $624); window and costs not printed. | engine 18 tr 44% net $7,071.00 avg $392.83
Row 8: British Pound (&6B) | Long TD11 of Jul | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/37 = 0.0% | printed: POUND buy day 11: 21 trades, 71% winners (15), net $13,481, printed average $641 (net / trades = $642); window and costs not printed. | engine 16 tr 62% net $8,871.87 avg $554.49
Row 9: US T-Bond (&ZB) | Long TD15 of Jul | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/38 = 0.0% | printed: BONDS buy day 15: 7 trades, 85% winners (6), net $8,931, printed average $1,275 (net / trades = $1,276); window and costs not printed. | engine 15 tr 73% net $9,830.50 avg $655.37
Row 10: Cocoa (&CC) | Short TD19 of Jul | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/38 = 0.0% | printed: COCOA sell day 19: 13 trades, 53% winners (8), net $41,205, printed average $274 (net / trades = $3,170); window and costs not printed. PRINTED FIGURES INCONSISTENT (53% is not 8/13; average x trades = 3,562 is not the printed profit): recorded as printed. | engine 16 tr 62% net $6,119.50 avg $382.47
Row 11: Cotton (&CT) | Short TD22 of Jul | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/15 = 0.0% | printed: COTTON sell day 22: 6 trades, 83% winners (5), net $4,790, printed average $798 (net / trades = $798); window and costs not printed. | engine 5 tr 100% net $5,201.95 avg $1,040.39
Row 12: Cotton (&CT) | Short TD21 of Jul | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/28 = 0.0% | printed: COTTON sell day 21: 8 trades, 87% winners (7), net $4,550, printed average $568 (net / trades = $569); window and costs not printed. | engine 12 tr 83% net $7,021.55 avg $585.13
Row 13: Corn (&ZC) | Short TD9 of Jul | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/35 = 0.0% | printed: CORN sell day 9: 22 trades, 77% winners (17), net $9,062, printed average $411 (net / trades = $412); window and costs not printed. | engine 14 tr 86% net $9,517.13 avg $679.79
Row 14: Corn (&ZC) | Short TD13 of Jul | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/43 = 0.0% | printed: CORN sell day 13: 27 trades, 77% winners (21), net $8,662, printed average $320 (net / trades = $321); window and costs not printed. | engine 19 tr 84% net $9,022.25 avg $474.86
Row 15: Coffee (&KC) | Short TD17 of Jul | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 2/41 = 4.9% (one trade under the 5% line) | printed: COFFEE sell day 17: 21 trades, 57% winners (12), net $22,833, printed average $1,087 (net / trades = $1,087); window and costs not printed. | engine 18 tr 61% net $30,290.63 avg $1,682.81
Row 16: Soybeans (&ZS) | Short TD13 of Jul | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/39 = 0.0% | printed: BEANS sell day 13: 25 trades, 64% winners (16), net $18,600, printed average $744 (net / trades = $744); window and costs not printed. | engine 18 tr 78% net $21,735.38 avg $1,207.52
Row 17: British Pound (&6B) | Short TD19 of Jul | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,500 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/38 = 0.0% | printed: POUND sell day 19: 20 trades, 60% winners (12), net $20,056, printed average $1,002 (net / trades = $1,003); window and costs not printed. | engine 16 tr 69% net $17,546.06 avg $1,096.63"##,
     [r##"13"##, r##"84% (11)"##, r##"$555 ($7,222)"##, r##"not printed"##]),
    (r##"lwnl_sep97_7day"##, r##"September 1997 7-day table (14 printed rows)"##, r##"Long/Short"##, r##"&CC"##,
     &[r##"E|"Keep in mind the day of entry, shown in the listing, is the trade date not calendar date of September. The entry rule is to buy at the open plus 7% of the previous days range ... A stop loss of $1,200 was used, except in the S&P ... The first listing shows the results of holding on for 7 days" (Vol. 34 No. 9 = September 1997, p.2, SEASONALLY SPEAKING; 7 DAY HOLD BUYS and SELLS tables, sells continued on p.3)"##, r##"E|Adaptation, not counted: the Norgate back-adjusted continuous contract (the CSI contract file is not held)"##, r##"E|Trading day (TD) of each row = the vault's count of the month's sessions (an unstated detail of every row; the row's own TD is in the row table)"##, r##"X|Row-specific labelled details (at most 4 per row, #339/#343) and the per-row level, stop, exit, #240 figure and printed record are in the row table in the notes"##, r##"E|Row parameter: 'row' picks the printed row (default 1 = the first printed row); the card runs on the Norgate contract of that row's market only, and a row on another market is refused with a message listing that market's rows"##],
     r##"September 1997 7-day table (14 printed rows). Source: Vol. 34 No. 9 (September 1997) p.2-3. 14 printed rows; ONE card with a row parameter (R4 #375). Rows above the 5% order-dependence line (#175, #240) are not coded.
Row 1: Cocoa (&CC) | Long TD7 of Sep | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/38 = 0.0% | printed: COCOA buy day 7: 15 trades, 66% winners (10), net $5,555, printed average $370 (net / trades = $370); window and costs not printed. | engine 16 tr 75% net $6,793.30 avg $424.58
Row 2: Cotton (&CT) | Long TD6 of Sep | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 1/37 = 2.7% | printed: COTTON buy day 6: 8 trades, 87% winners (7), net $10,025, printed average $1,253 (net / trades = $1,253); window and costs not printed. | engine 17 tr 47% net $3,009.45 avg $177.03
Row 3: Orange Juice (&OJ) | Long TD13 of Sep | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/35 = 0.0% | printed: JUICE buy day 13: 17 trades, 82% winners (14), net $7,305, printed average $429 (net / trades = $430); window and costs not printed. | engine 16 tr 81% net $5,387.55 avg $336.72
Row 4: Sugar No. 11 (&SB) | Long TD3 of Sep | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/39 = 0.0% | printed: SUGAR buy day 3: 8 trades, 62% winners (5), net $2,009, printed average $251 (net / trades = $251); window and costs not printed. | engine 17 tr 59% net $3,771.57 avg $221.86
Row 5: Swiss Franc (&6S) | Long TD11 of Sep | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 1/39 = 2.6% | printed: S FRANC buy day 11: 20 trades, 50% winners (10), net $16,200, printed average $810 (net / trades = $810); window and costs not printed. | engine 18 tr 50% net $11,495.50 avg $638.64
Row 6: British Pound (&6B) | Long TD14 of Sep | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 1/38 = 2.6% | printed: POUND buy day 14: 20 trades, 55% winners (11), net $8,150, printed average $407 (net / trades = $408); window and costs not printed. | engine 17 tr 59% net $10,437.56 avg $613.97
Row 7: US T-Bond (&ZB) | Long TD16 of Sep | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/40 = 0.0% | printed: BONDS buy day 16: 16 trades, 68% winners (11), net $10,300, printed average $643 (net / trades = $644); window and costs not printed. | engine 17 tr 65% net $7,790.93 avg $458.29
Row 8: US T-Bond (&ZB) | Long TD21 of Sep | level: stop at the open + 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 3 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 1/20 = 5.0% (at the 5% line, not above) | printed: BONDS buy day 21: 16 trades, 62% winners (10), net $17,143, printed average $1,071 (net / trades = $1,071); window and costs not printed. | engine 10 tr 60% net $13,210.42 avg $1,321.04
Row 9: Cocoa (&CC) | Short TD15 of Sep | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/33 = 0.0% | printed: COCOA sell day 15: 14 trades, 85% winners (12), net $6,480, printed average $466 (net / trades = $463); window and costs not printed. | engine 15 tr 87% net $9,490.50 avg $632.70
Row 10: Cotton (&CT) | Short TD13 of Sep | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 1/38 = 2.6% | printed: COTTON sell day 13: 10 trades, 70% winners (7), net $4,085, printed average $408 (net / trades = $408); window and costs not printed. | engine 17 tr 71% net $7,341.80 avg $431.87
Row 11: Copper (&HG) | Short TD18 of Sep | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 1/39 = 2.6% | printed: COPPER sell day 18: 7 trades, 85% winners (6), net $2,935, printed average $365 (net / trades = $419); window and costs not printed. | engine 19 tr 63% net $1,885.63 avg $99.24 | OCR-DOUBTFUL: 7 trades x printed average $365 = $2,555, not the printed net $2,935 (net / trades = $419); printed figures kept
Row 12: Orange Juice (&OJ) | Short TD3 of Sep | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/36 = 0.0% | printed: JUICE sell day 3: 18 trades, 55% winners (10), net $4,125, printed average $229 (net / trades = $229); window and costs not printed. | engine 17 tr 59% net $5,367.30 avg $315.72
Row 13: Soybeans (&ZS) | Short TD15 of Sep | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 0/38 = 0.0% | printed: BEANS sell day 15: 26 trades, 73% winners (19), net $17,056, printed average $656 (net / trades = $656); window and costs not printed. | engine 19 tr 79% net $15,812.25 avg $832.22
Row 14: British Pound (&6B) | Short TD3 of Sep | level: stop at the open - 7% of the previous range | stop/exit: close 7 bars after entry, $1,200 stop | 4 labelled details: entry at the open + 7% of the previous range, the 7-bar close exit, stop armed on the entry day, trading-day count | #240 1/39 = 2.6% | printed: POUND sell day 3: 20 trades, 65% winners (13), net $23,268, printed average $1,163 (net / trades = $1,163); window and costs not printed. | engine 16 tr 69% net $18,152.00 avg $1,134.50"##,
     [r##"15"##, r##"66% (10)"##, r##"$370 ($5,555)"##, r##"not printed"##]),
];

// ------------------------------------------------------------------ lane O3/O4/O5: monthly seasonal trading-day tables (Sep 1993 - Dec 1997), one table card per printed table/list (R4 #375)
// (card, Norgate market, direction (+1 buy, -1 sell), month, first trading day, last trading day (> first = the order stands until it fills, once a month), entry on the opening FOLLOWING that day?,
//  entry order % (> 0 = stop in the trade direction at the open +/- % of the previous range; 0 = at the open), $ stop (0 = none printed), close-exit bars after entry). Rows of one card = its printed rows in order; the 'row' parameter is the 1-based position.
/// The printed row `row` (1-based, as printed) of a table card.
pub(crate) fn mp_row(id: &str, row: f64) -> Option<&'static MRow> { MP.iter().filter(|m| m.0 == id).nth((row.round().max(1.0) as usize) - 1) }
/// Readiness: a table card's row runs only on the Norgate contract of its own market; any other context is refused and names the rows it does have.
pub fn row_home(id: &str, ticker: &str, p: &Params) -> Option<String> {
    let rows: Vec<&MRow> = MP.iter().filter(|m| m.0 == id).collect();
    if rows.is_empty() { return None; }
    let r = p.get("row").copied().unwrap_or(1.0).round().max(1.0) as usize;
    let Some(m) = rows.get(r - 1) else { return Some(format!("Not published: this card has {} printed rows; row {r} does not exist", rows.len())) };
    let k = crate::futures::of_ticker(ticker)?;
    if k.sym == m.1 { return None; }
    let name = |s: &str| crate::futures::MARKETS.iter().find(|k| k.sym == s).map_or(s.to_string(), |k| k.name.to_string());
    let mine: Vec<String> = rows.iter().enumerate().filter(|(_, m)| m.1 == k.sym).map(|(i, _)| (i + 1).to_string()).collect();
    Some(format!("Not published on {ticker}: row {r} of this table is a {} row ({}); on this market set row to {} (the table's rows on {}), or choose the {} context",
        name(m.1), m.1, if mine.is_empty() { "none: the table has no row on this market".into() } else { mine.join(", ") }, k.name, name(m.1)))
}
pub(crate) type MRow = (&'static str, &'static str, f64, u32, usize, usize, bool, f64, f64, usize);
pub(crate) const MP: &[MRow] = &[
    ("lwnl_jan97_list", "&KC", -1.0, 1, 16, 16, false, 0.0005, 1800.0, 5),
    ("lwnl_jan97_list", "&GC", -1.0, 1, 14, 14, false, 0.0005, 1800.0, 5),
    ("lwnl_jan97_list", "&OJ", -1.0, 1, 14, 14, false, 0.0005, 1800.0, 5),
    ("lwnl_jan97_list", "&OJ", 1.0, 1, 18, 18, false, 0.0005, 1800.0, 5),
    ("lwnl_jan97_list", "&6B", 1.0, 1, 22, 22, true, 0.0005, 1800.0, 5),
    ("lwnl_jan97_list", "&6S", 1.0, 1, 22, 22, true, 0.0005, 1800.0, 5),
    ("lwnl_jan97_list", "&ZW", 1.0, 1, 22, 22, true, 0.0005, 1800.0, 5),
    ("lwnl_jan97_list", "&GC", 1.0, 1, 6, 6, false, 0.0005, 1800.0, 5),
    ("lwnl_jan97_bonds", "&ZB", 1.0, 1, 7, 7, false, 0.0, 1800.0, 8),
    ("lwnl_jan97_bonds", "&ZB", -1.0, 1, 2, 2, false, 0.0, 1800.0, 8),
    ("lwnl_apr96_list", "&HG", -1.0, 4, 3, 3, false, 0.1, 2500.0, 3),
    ("lwnl_apr96_list", "&KC", -1.0, 4, 17, 17, false, 0.1, 2500.0, 3),
    ("lwnl_apr96_list", "&KC", -1.0, 4, 18, 18, false, 0.1, 2500.0, 3),
    ("lwnl_apr96_list", "&GC", -1.0, 4, 18, 18, false, 0.1, 2500.0, 3),
    ("lwnl_apr96_list", "&6B", 1.0, 4, 7, 7, false, 0.1, 2500.0, 3),
    ("lwnl_apr96_list", "&6J", 1.0, 4, 7, 7, false, 0.1, 2500.0, 3),
    ("lwnl_apr96_list", "&6S", 1.0, 4, 7, 7, false, 0.1, 2500.0, 3),
    ("lwnl_apr96_list", "&ZB", -1.0, 4, 11, 11, false, 0.1, 2500.0, 3),
    ("lwnl_apr96_list", "&ZB", -1.0, 4, 13, 13, false, 0.1, 2500.0, 3),
    ("lwnl_wheat_sep_td1_5_buy", "&ZW", 1.0, 9, 1, 5, false, 0.4, 1100.0, 33),
    ("lwnl_ho_mar_buy", "&HO", 1.0, 3, 7, 7, false, 0.0, 1100.0, 30),
    ("lwnl_ho_mar_buy", "&HO", 1.0, 3, 9, 9, false, 0.0, 1100.0, 30),
    ("lwnl_bonds_oct", "&ZB", 1.0, 10, 9, 9, false, 0.0, 900.0, 3),
    ("lwnl_bonds_oct", "&ZB", -1.0, 10, 17, 17, false, 0.0, 900.0, 3),
    ("lwnl_apr97_7day", "&LE", 1.0, 4, 20, 20, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&CC", 1.0, 4, 6, 6, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&CT", 1.0, 4, 2, 2, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&HG", 1.0, 4, 10, 10, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&KC", 1.0, 4, 9, 9, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&OJ", 1.0, 4, 20, 20, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&SI", 1.0, 4, 21, 21, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&ZS", 1.0, 4, 10, 10, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&SB", 1.0, 4, 10, 10, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&6B", 1.0, 4, 6, 6, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&HO", 1.0, 4, 3, 3, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&ZB", 1.0, 4, 4, 4, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&ZB", 1.0, 4, 19, 19, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&LE", -1.0, 4, 4, 4, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&CC", -1.0, 4, 16, 16, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&HG", -1.0, 4, 14, 14, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&OJ", -1.0, 4, 5, 5, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&SB", -1.0, 4, 2, 2, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&ZW", -1.0, 4, 15, 15, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&6S", -1.0, 4, 7, 7, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&GC", -1.0, 4, 8, 8, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&HO", -1.0, 4, 16, 16, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&ZB", -1.0, 4, 11, 11, false, 0.0, 1200.0, 7),
    ("lwnl_apr97_7day", "&ZB", -1.0, 4, 11, 11, false, 0.0, 1200.0, 6),
    ("lwnl_apr97_7day", "&ZB", -1.0, 4, 9, 9, false, 0.0, 1600.0, 7),
    ("lwnl_may97", "&LE", 1.0, 5, 21, 21, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&CT", 1.0, 5, 10, 10, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&CT", 1.0, 5, 5, 5, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&HG", 1.0, 5, 19, 19, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&HG", 1.0, 5, 20, 20, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&ZC", 1.0, 5, 20, 20, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&ZC", 1.0, 5, 21, 21, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&KC", 1.0, 5, 5, 5, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&KC", 1.0, 5, 4, 4, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&OJ", 1.0, 5, 13, 13, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&OJ", 1.0, 5, 1, 1, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&SI", 1.0, 5, 4, 4, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&ZS", 1.0, 5, 4, 4, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&ZS", 1.0, 5, 19, 19, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&SB", 1.0, 5, 21, 21, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&SB", 1.0, 5, 19, 19, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&ZW", 1.0, 5, 22, 22, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&ZW", 1.0, 5, 21, 21, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&6B", 1.0, 5, 15, 15, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&6B", 1.0, 5, 14, 14, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&GC", 1.0, 5, 4, 4, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&GC", 1.0, 5, 3, 3, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&HO", 1.0, 5, 2, 2, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&HO", 1.0, 5, 13, 13, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&ZB", 1.0, 5, 9, 9, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&ZB", 1.0, 5, 8, 8, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&LE", -1.0, 5, 7, 7, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&LE", -1.0, 5, 4, 4, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&CC", -1.0, 5, 16, 16, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&CC", -1.0, 5, 18, 18, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&ZC", -1.0, 5, 14, 14, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&KC", -1.0, 5, 17, 17, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&KC", -1.0, 5, 16, 16, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&OJ", -1.0, 5, 14, 14, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&OJ", -1.0, 5, 19, 19, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&SI", -1.0, 5, 13, 13, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&SI", -1.0, 5, 13, 13, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&ZS", -1.0, 5, 16, 16, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&ZS", -1.0, 5, 10, 10, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&SB", -1.0, 5, 13, 13, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&ZW", -1.0, 5, 5, 5, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&ZW", -1.0, 5, 15, 15, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&6S", -1.0, 5, 3, 3, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&6S", -1.0, 5, 7, 7, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&6B", -1.0, 5, 10, 10, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&6B", -1.0, 5, 19, 19, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&GC", -1.0, 5, 13, 13, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&GC", -1.0, 5, 13, 13, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&6J", -1.0, 5, 7, 7, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&6J", -1.0, 5, 6, 6, false, 0.0, 0.0, 15),
    ("lwnl_may97", "&HO", -1.0, 5, 21, 21, false, 0.0, 0.0, 7),
    ("lwnl_may97", "&HO", -1.0, 5, 19, 19, false, 0.0, 0.0, 15),
    ("lwnl_jul97_7day", "&LE", 1.0, 7, 21, 21, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&LE", 1.0, 7, 20, 20, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&CT", 1.0, 7, 9, 9, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&HG", 1.0, 7, 14, 14, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&OJ", 1.0, 7, 18, 18, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&SB", 1.0, 7, 7, 7, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&6S", 1.0, 7, 9, 9, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&6B", 1.0, 7, 11, 11, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&ZB", 1.0, 7, 15, 15, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&CC", -1.0, 7, 19, 19, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&CT", -1.0, 7, 22, 22, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&CT", -1.0, 7, 21, 21, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&ZC", -1.0, 7, 9, 9, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&ZC", -1.0, 7, 13, 13, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&KC", -1.0, 7, 17, 17, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&ZS", -1.0, 7, 13, 13, false, 0.07, 1500.0, 7),
    ("lwnl_jul97_7day", "&6B", -1.0, 7, 19, 19, false, 0.07, 1500.0, 7),
    ("lwnl_sep97_7day", "&CC", 1.0, 9, 7, 7, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&CT", 1.0, 9, 6, 6, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&OJ", 1.0, 9, 13, 13, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&SB", 1.0, 9, 3, 3, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&6S", 1.0, 9, 11, 11, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&6B", 1.0, 9, 14, 14, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&ZB", 1.0, 9, 16, 16, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&ZB", 1.0, 9, 21, 21, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&CC", -1.0, 9, 15, 15, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&CT", -1.0, 9, 13, 13, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&HG", -1.0, 9, 18, 18, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&OJ", -1.0, 9, 3, 3, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&ZS", -1.0, 9, 15, 15, false, 0.07, 1200.0, 7),
    ("lwnl_sep97_7day", "&6B", -1.0, 9, 3, 3, false, 0.07, 1200.0, 7),
];

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
    pub(crate) order: bool,         // EP6 (#243): order-dependent days on the real 1-minute path
}

#[derive(Clone)]
struct Pos { dir: f64, i: usize, px: f64, stop: f64, kind: &'static str, od: crate::intraday_ep6::Day }

/// The intrabar state of one bar (EP6): position, 'exited this bar', and the stops filled inside the bar.
#[derive(Clone)]
struct Bar3 { pos: Option<Pos>, exited: bool, out: Vec<(Pos, f64)> }

fn done(p: &Pos, i: usize, px: f64, why: &str) -> Trade {
    let mut t = Trade { dir: p.dir, legs: vec![Leg { i: p.i, px: p.px, w: 1.0 }], exit_i: i, exit_px: px, reason: why.into(),
        entry_kind: p.kind, signal_i: i, open: false, exits: vec![] };
    crate::intraday_ep6::mark(&mut t, p.od);
    t
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
    let mk = |dir: f64, i: usize, px: f64, kind: &'static str| Pos { dir, i, px, stop: px - dir * sd, kind, od: crate::intraday_ep6::Day::Plain };
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
        // 3. intrabar path; EP6: an order-dependent day of an EP6 id is walked on the real 1-minute path (intraday_ep6::resolve)
        let walk = |st: &mut Bar3, segs: &[crate::intraday_ep6::Seg]| -> Vec<crate::intraday_ep6::Ev> {
            let mut evs = vec![];
            for g in segs {
                let (mut cur, end) = (g.a, g.b);
                let up = end > cur;
                loop {
                    // candidate events: (level, Some(dir, kind)) = entry, (level, None) = protective stop
                    let mut best: Option<(f64, Option<(f64, &'static str)>)> = None;
                    let inside = |lv: f64| lv.is_finite() && if up { lv > cur && lv <= end } else { lv < cur && lv >= end };
                    let mut take = |lv: f64, ev: Option<(f64, &'static str)>| {
                        if inside(lv) && best.map_or(true, |(b, _)| if up { lv < b } else { lv > b }) { best = Some((lv, ev)); }
                    };
                    match &st.pos {
                        Some(p) => {
                            let against = (p.dir > 0.0) != up;
                            if against && (p.i < i || cfg.day0) { take(p.stop, None); }
                        }
                        None if !st.exited => {
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
                    let px = g.fill(lv);
                    match ev {
                        Some((dir, kind)) => { st.pos = Some(mk(dir, i, px, kind)); evs.push(crate::intraday_ep6::Ev { k: if dir > 0.0 { 0 } else { 1 }, px, up }); }
                        None => { let p = st.pos.take().unwrap(); st.out.push((p, px)); st.exited = true; evs.push(crate::intraday_ep6::Ev { k: 2, px, up }); }
                    }
                    cur = lv;
                }
            }
            evs
        };
        let mut st = Bar3 { pos: pos.take(), exited, out: vec![] };
        let day = crate::intraday_ep6::resolve(x, i, cfg.order, h - o <= o - l, &mut st,
            |s, hf| walk(s, &crate::intraday_ep6::bar_segs(o, h, l, c, hf)), &walk);
        for (mut p, px) in st.out { p.od = p.od.worst(day); out.push(done(&p, i, px, "stop")); }
        (pos, exited) = (st.pos, st.exited);
        if let Some(p) = pos.as_mut() { p.od = p.od.worst(day); }
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
    let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) }; // EP6 #243 (c)
    let g = getter(&schema, p);
    let has = |k: &str| schema.iter().any(|d| d.key == k);
    let opt = |k: &str| if has(k) && g(k) >= 0.0 { Some(g(k).round() as usize) } else { None };
    let cfg = Cfg { stop: g("stop_usd") / g("pt_usd"), day0: g("day0") >= 0.5, bail: opt("bail_delay"), hold: opt("hold"),
        prof_close: has("prof_close") && g("prof_close") >= 0.5, order: crate::intraday_ep6::on(id, p) };
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
        "lwnl_bond_sp_9ma_1down" | "lwnl_bond_sp_9ma_2down" => {
            // lane F1D (R4 #293): the S&P with the T-bond 9-bar average as Data2, only on the Norgate S&P contract
            if crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
            let Some(rows) = crate::futures::rows("&ZB") else { return Some(vec![]) }; // readiness refuses this case (data2_issue)
            let zb = crate::second::align("&ZB", x.r, rows);
            let up9 = zb.own(|r| { let m = crate::connors::sma(&r.iter().map(|b| b.close).collect::<Vec<_>>(), 9);
                (0..r.len()).map(|k| (k >= 1 && m[k] > m[k - 1]) as u8 as f64).collect() });
            let two = id.ends_with("2down");
            let c2 = crate::books::lwlts::Cfg { stop: g("stop_usd") / g("pt_usd"), stop_s: f64::NAN, stop_rng: f64::NAN, day0: g("day0") >= 0.5, target: f64::NAN,
                hold: Some(g("hold").max(0.0) as usize), bailout: false, bail_after: 0, reverse: false, order: false };
            let ep = g("entry_pct");
            crate::books::lwlts::simulate(x, from, to, &c2, |i| {
                if i < 3 { return (None, None); }
                let s = i - 1;
                let ok = up9[s] == 1.0 && c[s] < c[s - 1] && (!two || c[s - 1] < c[s - 2]);
                (ok.then(|| crate::books::lwlts::Ent::Stop(o[i] + ep * (h[s] - l[s]))), None)
            })
        }
        s if MP.iter().any(|m| m.0 == s) => {
            // lane O5 (R4 #375): one card per printed table of monthly seasonal trading-day trades; the 'row' parameter picks the printed row (default 1),
            // which names its market: only on the Norgate contract of that market (R4 #367 (2)); readiness refuses any other context with row_home's message
            let m = mp_row(s, g("row"))?;
            if crate::futures::ctx_market(x).map(|k| k.sym) != Some(m.1) { return Some(vec![]); }
            let k = crate::futures::ctx_market(x)?;
            let pt = crate::futures::usd_per_point(k, &x.r[n - 1].date)?;
            let dir = m.2;
            use crate::books::lwlts::{Cfg as LC, Ent, simulate};
            let c2 = LC { stop: if m.8 > 0.0 { m.8 / pt } else { f64::NAN }, stop_s: f64::NAN, stop_rng: f64::NAN, day0: true, target: f64::NAN, hold: Some(m.9), bailout: false, bail_after: 0, reverse: false, order: false };
            let td = crate::books::lwlts::tdm(x);
            let mut on = vec![false; n];
            let mut filled = String::new();
            for i in 2..n {
                let b = if m.6 { i - 1 } else { i };
                if mon(b) != m.3 || td[b] < m.4 || td[b] > m.5 { continue; }
                if m.5 > m.4 { // the order stands until the first day it fills, one entry a month
                    if x.r[i].date[..7] == filled[..] { continue; }
                    let lv = o[i] + dir * m.7 * (h[i - 1] - l[i - 1]);
                    if if dir > 0.0 { h[i] >= lv } else { l[i] <= lv } { filled = x.r[i].date[..7].to_string(); }
                }
                on[i] = true;
            }
            simulate(x, from, to, &c2, |i| {
                if !on[i] { return (None, None); }
                let e = if m.7 > 0.0 { Ent::Stop(o[i] + dir * m.7 * (h[i - 1] - l[i - 1])) } else { Ent::Open };
                if dir > 0.0 { (Some(e), None) } else { (None, Some(e)) }
            })
        }
        s if CSYS.iter().any(|b| b.0 == s) => {
            // lane O1 (R4 #367 (2)): only on the Norgate contract of the letter's market; dollar stops at that market's own US$ per point
            let m = CSYS.iter().find(|b| b.0 == s)?;
            if crate::futures::ctx_market(x).map(|k| k.sym) != Some(m.3) { return Some(vec![]); }
            let k = crate::futures::ctx_market(x)?;
            let pt = crate::futures::usd_per_point(k, &x.r[n - 1].date)?;
            use crate::books::lwlts::{Cfg as LC, Ent, simulate};
            let mut c2 = LC { stop: f64::NAN, stop_s: f64::NAN, stop_rng: f64::NAN, day0: true, target: f64::NAN, hold: None, bailout: false, bail_after: 0, reverse: false, order: false };
            match s {
                "lwnl_gold_13x13" => {
                    c2.reverse = true;
                    simulate(x, from, to, &c2, |i| {
                        if i < 13 { return (None, None); }
                        let hh = (i - 13..i).map(|j| h[j]).fold(f64::MIN, f64::max);
                        let ll = (i - 13..i).map(|j| l[j]).fold(f64::MAX, f64::min);
                        (Some(Ent::Stop(hh)), Some(Ent::Stop(ll)))
                    })
                }
                "lwnl_gold_nov_td16" | "lwnl_yen_nov_td10" => {
                    c2.bailout = true; c2.bail_after = 7; c2.hold = None; // "Delayed Bailout" of N=6 days: from the opening of bar entry+N+1 (module header, Nov 1996 p.1)
                    let yen = s == "lwnl_yen_nov_td10";
                    if yen { c2.stop = 1600.0 / pt; }
                    let td = crate::books::lwlts::tdm(x); let n = if yen { 10 } else { 16 };
                    simulate(x, from, to, &c2, |i| if &x.r[i].date[5..7] == "11" && td[i] == n { (Some(Ent::Open), None) } else { (None, None) })
                }
                "lwnl_yen_apr_td4" => {
                    c2.stop = 2200.0 / pt; c2.hold = Some(12);
                    let td = crate::books::lwlts::tdm(x);
                    simulate(x, from, to, &c2, |i| if &x.r[i].date[5..7] == "04" && td[i] == 4 { (Some(Ent::Open), None) } else { (None, None) })
                }
                "lwnl_cattle_jul_td18" => {
                    c2.stop = 900.0 / pt; c2.hold = Some(10);
                    let td = crate::books::lwlts::tdm(x);
                    simulate(x, from, to, &c2, |i| if i >= 1 && &x.r[i].date[5..7] == "07" && td[i] == 18 { (Some(Ent::Stop(o[i] + 0.2 * (h[i - 1] - l[i - 1]))), None) } else { (None, None) })
                }
                "lwnl_corn_jul_td2_short" => {
                    // exit on the OPEN of the 24th bar after entry = one bar after the close of the 23rd; the stop is checked first at that open
                    // (a gap through the stop also exits at the open), so moving a time exit by one bar to its open is exact
                    c2.stop = 1000.0 / pt; c2.hold = Some(23);
                    let td = crate::books::lwlts::tdm(x);
                    let mut t = simulate(x, from, to, &c2, |i| if i >= 1 && &x.r[i].date[5..7] == "07" && td[i] == 2 { (None, Some(Ent::Stop(o[i] - 0.1 * (h[i - 1] - l[i - 1])))) } else { (None, None) });
                    for tr in t.iter_mut().filter(|t| !t.open && t.reason.starts_with("time exit") && t.exit_i < to) { tr.exit_i += 1; tr.exit_px = o[tr.exit_i]; tr.reason = "time exit on the open".into(); }
                    t
                }
                "lwnl_coffee_aug_td12_open" | "lwnl_coffee_aug_td12_limit" => {
                    c2.stop = 750.0 / pt; c2.hold = Some(6);
                    let td = crate::books::lwlts::tdm(x);
                    simulate(x, from, to, &c2, |i| {
                        if !(i >= 1 && &x.r[i].date[5..7] == "08" && td[i] == 12) { return (None, None); }
                        let r = 0.2 * (h[i - 1] - l[i - 1]);
                        (Some(if s == "lwnl_coffee_aug_td12_open" { Ent::Open } else { Ent::Limit(o[i] - r) }), None)
                    })
                }
                _ => { // lwnl_gold_tdom1_4d
                    c2.stop = 2000.0 / pt; c2.hold = Some(12);
                    let td = crate::books::lwlts::tdm(x);
                    simulate(x, from, to, &c2, |i| if i >= 5 && td[i - 1] == 1 && c[i - 1] > c[i - 5] { (Some(Ent::Open), None) } else { (None, None) })
                }
            }
        }
        s if BSEA.iter().any(|b| b.0 == s) => {
            // lane F1D (R4 #293): the T-bond seasonal trades, only on the Norgate T-bond contract
            if crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
            let b = BSEA.iter().find(|b| b.0 == s)?;
            let c2 = crate::books::lwlts::Cfg { stop: g("stop_usd") / g("pt_usd"), stop_s: f64::NAN, stop_rng: f64::NAN, day0: g("day0") >= 0.5,
                target: f64::NAN, hold: if b.8.is_none() { Some(b.9) } else { None }, bailout: b.8.is_some(), bail_after: b.8.unwrap_or(0),
                reverse: false, order: false };
            let td = crate::books::lwlts::tdm(x);
            let mut on = vec![false; n];
            if let Some(hh) = b.4 {
                let (y0, y1): (i32, i32) = (x.r[0].date[..4].parse().ok()?, x.r[n - 1].date[..4].parse().ok()?);
                for y in y0..=y1 + 1 { if let Some(j) = td_rel(x, &holiday(hh, y), b.3) { on[j] = true; } }
            } else {
                for i in 0..n { on[i] = td[i] == b.3 as usize && x.r[i].date[5..7].parse::<u32>().ok() == Some(b.2); }
            }
            let e = if b.6 { crate::books::lwlts::Ent::Close } else { crate::books::lwlts::Ent::Open };
            crate::books::lwlts::simulate(x, from, to, &c2, |i| if on[i] { if b.5 > 0.0 { (Some(e), None) } else { (None, Some(e)) } } else { (None, None) })
        }
        "lwnl_sp_hornbrook_im" => {
            // lane F1D (R4 #293): the S&P with the pound, crude and gold as Data2 (own bars, carried to the S&P dates)
            if crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
            let (Some(bp), Some(cl), Some(gc)) = (crate::futures::rows("&6B"), crate::futures::rows("&CL"), crate::futures::rows("&GC")) else { return Some(vec![]) };
            let flag = |sym: &'static str, rows: &'static [crate::Row], n: usize, up: bool| crate::second::align(sym, x.r, rows)
                .own(|r| (0..r.len()).map(|k| (k >= n && if up { r[k].close > r[k - n].close } else { r[k].close < r[k - n].close }) as u8 as f64).collect());
            let (f1, f2, f3) = (flag("&6B", bp, 8, true), flag("&CL", cl, 1, true), flag("&GC", gc, 5, false));
            let c2 = crate::books::lwlts::Cfg { stop: f64::NAN, stop_s: f64::NAN, stop_rng: f64::NAN, day0: true, target: f64::NAN,
                hold: Some(0), bailout: false, bail_after: 0, reverse: false, order: false };
            crate::books::lwlts::simulate(x, from, to, &c2, |i| {
                let s = i - 1;
                ((f1[s] == 1.0 && f2[s] == 1.0 && f3[s] == 1.0).then_some(crate::books::lwlts::Ent::Open), None)
            })
        }
        "lwnl_bond_dowm_buy" => {
            // lane F1D (R4 #293): only on the Norgate T-bond contract
            if crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
            let c2 = crate::books::lwlts::Cfg { stop: g("stop_usd") / g("pt_usd"), stop_s: f64::NAN, stop_rng: f64::NAN, day0: g("day0") >= 0.5,
                target: g("target_usd") / g("pt_usd"), hold: Some(3), bailout: false, bail_after: 0, reverse: false, order: false };
            let d4 = if g("fri") >= 0.5 { 5 } else { 4 };
            crate::books::lwlts::simulate(x, from, to, &c2, |i| {
                let s = i - 1;
                let dom: u32 = x.r[s].date[8..10].parse().unwrap_or(0);
                let wk = match dom { 1..=7 => 1, 8..=14 => 2, 15..=21 => 3, 22..=28 => 4, _ => 5 };
                let wd = crate::books::lwlts::dow(&x.r[s].date);
                let ok = (wk == 4 && (wd == 1 || wd == 2 || wd == d4)) || (wk == 5 && (wd == 1 || wd == 2));
                (ok.then(|| crate::books::lwlts::Ent::Limit(o[i] - 3.0 / 32.0)), None)
            })
        }
        "lwnl_bond_xmas_buy" | "lwnl_bond_newyear_sell" => {
            // lane F1D (review M34): in at the open 5 trading days before the holiday, out at the first opening after it; no stop
            if crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
            let xmas = id == "lwnl_bond_xmas_buy";
            let (y0, y1): (i32, i32) = (x.r[0].date[..4].parse().ok()?, x.r[n - 1].date[..4].parse().ok()?);
            let mut out = vec![];
            for y in y0..=y1 + 1 {
                let hd = if xmas { holiday(Hol::Christmas, y) } else { holiday(Hol::NewYear, y) };
                let Some(j) = td_rel(x, &hd, 5) else { continue };
                if j < from.max(1) || j > to { continue; }
                let dir = if xmas { 1.0 } else { -1.0 };
                let (k, open) = match td_rel(x, &hd, -1) { Some(k) if k <= to => (k, false), _ => (to, true) };
                let px = if open { c[to] } else { o[k] };
                out.push(Trade { dir, legs: vec![Leg { i: j, px: o[j], w: 1.0 }], exit_i: k, exit_px: px,
                    reason: if open { "open".into() } else { "exit on the first opening after the holiday".into() }, entry_kind: "open", signal_i: j, open, exits: vec![] });
            }
            out
        }
        "lwnl_bond_projected" => {
            // lane F1D (R4 #293): only on the Norgate T-bond contract
            if crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
            let c2 = crate::books::lwlts::Cfg { stop: g("stop_usd") / g("pt_usd"), stop_s: f64::NAN, stop_rng: f64::NAN, day0: g("day0") >= 0.5, target: f64::NAN,
                hold: None, bailout: true, bail_after: 0, reverse: true, order: false };
            let sd = g("sell_dow") as u32;
            crate::books::lwlts::simulate(x, from, to, &c2, |i| {
                let s = i - 1;
                let pp = (h[s] + l[s] + c[s]) / 3.0 * 2.0;
                let (plo, phi) = (pp - h[s], pp - l[s]);
                let wd = crate::books::lwlts::dow(&x.r[i].date);
                let bit = if (1..=5).contains(&wd) { 1u32 << (wd - 1) } else { 0 };
                ((o[i] < plo).then(|| crate::books::lwlts::Ent::Stop(plo)), (o[i] > phi && sd & bit != 0).then(|| crate::books::lwlts::Ent::Stop(phi)))
            })
        }
        "lwnl_cti_bonds" => {
            // lane F1D (R4 #293): only on the Norgate T-bond contract; lwlts::simulate for the stop-and-reverse with per-side stops
            if crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
            let c2 = crate::books::lwlts::Cfg { stop: g("stop_usd") / g("pt_usd"), stop_s: g("stop_s_usd") / g("pt_usd"), stop_rng: f64::NAN, day0: true,
                target: f64::NAN, hold: None, bailout: false, bail_after: 0, reverse: true, order: false };
            let (bp, sp) = (g("buy_pct"), g("sell_pct"));
            crate::books::lwlts::simulate(x, from, to, &c2, |i| {
                let r = h[i - 1] - l[i - 1];
                if !(r > 0.0) { return (None, None); }
                (Some(crate::books::lwlts::Ent::Stop(h[i - 1] + bp * r)), Some(crate::books::lwlts::Ent::Stop(l[i - 1] - sp * r)))
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
