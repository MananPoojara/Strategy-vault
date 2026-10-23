//! Jake Bernstein, "Hot Stock Market Strategies" (2005; data to 2004). Book id `jbhsms`.
//! Source: a 249-page scan, read as page images (printed page = PDF page - 22). Transcription and checks: library/jbhsms/.
//! Testable on our data: the S&P, Dow Jones and US-stock key-date seasonal trades of ch.7 (each printed with its full
//! trade history) and the ch.6 gap trade with the printed IBM / EBAY inputs (US stocks, cents per share).
use super::jb624::{finish, getp, hit_after_entry, mark_open, new_trade, stop_fill};
use super::jbcdt2::{days, days_ymd};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbhsms";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const R_ENTRY: &str = "E|\"these trades are always entered on the close of trading and exited on the close of trading on the dates shown\"; \"If a stock is closed on the recommended entry or exit days, then you execute the seasonal trade on the close of trading, the next business day\" (p.107)";
const R_STOP: &str = "X|\"All stop losses for the seasonal stock trades are 'close only.' This means that if you are short, the market must close above the calculated stop loss ... The same stop-loss rule applies for a long position\"; \"The stop loss is a percentage of the price on day of entry\" (p.107); \"Stop %\" = \"the stop loss in percent of the closing price on the date of entry\" (p.108)";
const N_SP: &str = "Book record: S&P 500 futures (\"the same patterns and dates apply to the SPY\", p.108); run on the index (instrument swap). Exit on the first close on or after the exit date; a close below entry x (1 - stop%) before then exits on that close. Stop% 100 = no stop.";
const N_DJ: &str = "Book record: Dow Jones Industrial Average (cash index), entries 1970-2004: compared trade by trade with our DJI series. Exit on the first close on or after the exit date; a close below entry x (1 - stop%) before then exits on that close. Stop% 100 = no stop.";

const N_STK: &str = "Book record: this US stock 1970-2004 (Figures 7.11-7.18). Runs on the stock itself (US stock data, split/dividend-adjusted, so early prices differ from the book's) and, as a declared application, on any other stock, index or ETF. Exit on the first close on or after the exit date; a close below entry x (1 - stop%) before then exits on that close. Stop% 100 = no stop.";

/// (id, entry m, d, exit m, d, stop %)
const KD: [(&str, f64, f64, f64, f64, f64); 21] = [
    ("jbhsms_sp_0113", 1., 13., 1., 18., 2.0),
    ("jbhsms_sp_0226", 2., 26., 3., 5., 3.5),
    ("jbhsms_sp_0301", 3., 1., 3., 6., 1.5),
    ("jbhsms_sp_0414", 4., 14., 4., 22., 3.0),
    ("jbhsms_sp_0601", 6., 1., 6., 6., 2.0),
    ("jbhsms_sp_0710", 7., 10., 7., 15., 1.5),
    ("jbhsms_sp_0815", 8., 15., 8., 25., 3.5),
    ("jbhsms_sp_1220", 12., 20., 1., 7., 3.0),
    ("jbhsms_dj_0405", 4., 5., 4., 22., 100.0),
    ("jbhsms_dj_1117", 11., 17., 1., 4., 9.0),
    ("jbhsms_dj_1203", 12., 3., 1., 6., 9.0),
    ("jbhsms_dj_0218", 2., 18., 5., 4., 15.0),
    ("jbhsms_dj_0313", 3., 13., 3., 18., 3.0),
    ("jbhsms_dj_0407", 4., 7., 4., 16., 15.0),
    ("jbhsms_dj_0528", 5., 28., 6., 6., 100.0),
    ("jbhsms_stk_aa", 1., 22., 2., 18., 15.0),
    ("jbhsms_stk_ko", 3., 31., 6., 6., 100.0),
    ("jbhsms_stk_ge", 5., 25., 6., 6., 6.0),
    ("jbhsms_stk_msft", 8., 31., 2., 1., 100.0),
    ("jbhsms_stk_pg", 10., 26., 12., 27., 100.0),
    ("jbhsms_stk_txn", 12., 15., 4., 19., 100.0),
];

/// Gap trade inputs (Tables 6.1-6.3): (id, gap up above the prior high, gap down below the prior low, penetration down
/// into the prior high, penetration up into the prior low, stop %), all in dollars per share except the stop.
const GAP: [(&str, f64, f64, f64, f64, f64); 3] = [
    ("jbhsms_gap_ibm", 0.20, 0.10, 0.35, 0.50, 3.9),
    ("jbhsms_gap_ibm2", 0.30, 0.90, 0.35, 0.80, 4.75),
    ("jbhsms_gap_ebay", 0.48, 0.48, 0.04, 0.98, 3.55),
];
const R_GAP: [&str; 2] = [
    "E|\"The gap method enters a LONG position if a stock has opened below the low of the previous day by more than a given amount and then enters back into the range of the previous day by penetrating back up through the low of the previous day by a given amount\" (p.94); the SHORT mirror: opened above the previous high by more than a given amount, then penetrates back down through the previous high by a given amount (p.95)",
    "X|\"The trade exists using a stop loss that is a percentage of the entry price or it takes profit on the first profitable opening\" (p.94)",
];
const N_GAP: &str = "Inputs are US cents per share for the stock named in the table, so this runs on actual US stocks only (the cents mean nothing on an index, an ETF or a rupee-priced stock). Daily bars: the entry is a stop order at the previous low + penetration (long) or previous high - penetration (short) on the gap day; the stop rests from the next day; on the entry day it counts only if the bar path (open, nearer extreme, farther extreme, close) reaches it after the fill (p.entry_day=1: any touch) [Not from the book: daily-bar convention]. First profitable opening = the first later open above (long) / below (short) the entry price. No new entry on an exit day. The book's IBM/EBAY tests used 500 shares and $25 commission; vault US prices are split/dividend-adjusted (not the historical quotes), so the cent amounts are a slightly different fraction of price than in 2000-2004.";

macro_rules! kd {
    ($id:expr, $name:expr, $fig:expr, $n:expr, $claim:expr) => {
        StrategyDef { claim: $claim, ..def!($id, $name, "Ch.7 · Short-Term Key Date Seasonal Trades", "Long", Status::Live, &[$fig, R_ENTRY, R_STOP], $n) }
    };
}
use crate::connors::claim as c;

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        kd!("jbhsms_sp_0113", "S&P key date · long Jan 13 to Jan 18, 2% stop", "F|Figure 7.2 (p.109): \"Seasonal Trade in March S&P Jan. 13 to Jan. 18\" Long, Enter 1/13, Exit 1/18, Stop %: 2.00", N_SP, c("22", "90.91%", "8.24 S&P points", "1983-2004", "March S&P futures; avg loss -17.25")),
        kd!("jbhsms_sp_0226", "S&P key date · long Feb 26 to Mar 5, 3.5% stop", "F|Figure 7.3 (p.111): Long, Enter 2/26, Exit 3/5, Stop %: 3.50 (the text's \"April 6\" exit, p.110, contradicts the table's 3/5 and every listed exit date)", N_SP, c("22", "81.82%", "11.28 S&P points", "1983-2004", "March S&P futures")),
        kd!("jbhsms_sp_0301", "S&P key date · long Mar 1 to Mar 6, 1.5% stop", "F|Figure 7.4 (p.112): Long, Enter 3/1, Exit 3/6, Stop %: 1.50", N_SP, c("22", "77.27%", "9.37 S&P points", "1983-2004", "March S&P futures")),
        kd!("jbhsms_sp_0414", "S&P key date · long Apr 14 to Apr 22, 3% stop", "F|Figure 7.5 (p.113): Long, Enter 4/14, Exit 4/22, Stop %: 3.00", N_SP, c("23", "78.26%", "14.51 S&P points", "1982-2004", "June S&P futures")),
        kd!("jbhsms_sp_0601", "S&P key date · long Jun 1 to Jun 6, 2% stop", "F|Figure 7.6 (p.115): Long, Enter 6/1, Exit 6/6, Stop %: 2.00", N_SP, c("23", "78.26%", "8.80 S&P points", "1982-2004", "June S&P futures")),
        kd!("jbhsms_sp_0710", "S&P key date · long Jul 10 to Jul 15, 1.5% stop", "F|Figure 7.7 (p.116): Long, Enter 7/10, Exit 7/15, Stop %: 1.50", N_SP, c("22", "77.27%", "8.31 S&P points", "1982-2003", "September S&P futures")),
        kd!("jbhsms_sp_0815", "S&P key date · long Aug 15 to Aug 25, 3.5% stop", "F|Figure 7.8 (p.117): Long, Enter 8/15, Exit 8/25, Stop %: 3.50", N_SP, c("22", "81.82%", "9.25 S&P points", "1982-2003", "September S&P futures")),
        kd!("jbhsms_sp_1220", "S&P key date · long Dec 20 to Jan 7, 3% stop", "F|Figure 7.10 (p.119): Long, Enter 12/20, Exit 1/7, Stop %: 3.00", N_SP, c("22", "86.36%", "16.44 S&P points", "1983-2004 (entry years)", "March S&P futures")),
        kd!("jbhsms_dj_0405", "Dow key date · long Apr 5 to Apr 22 (no stop shown)", "F|Figure 7.1 (p.105-106): \"Seasonal Pattern in the Dow Jones Industrial Average from April 5 to April 25, 1970 to 2004\"; the table enters 4/5 and exits 4/22 (\"higher on or about 4/22 than it was on or about 4/5\", p.104); no stop column", "Fig. 7.1 prints no stop, so none is used (stop% 100). The title's \"April 25\" is the window of the chart; the trades exit on 4/22. Dow Jones Industrial Average, compared trade by trade with our DJI series.", c("35", "77.14%", "106.50 Dow points", "1970-2004", "avg profit 3.10%, avg loss -2.01%")),
        kd!("jbhsms_dj_1117", "Dow key date · long Nov 17 to Jan 4, 9% stop", "F|Figure 7.19 (p.129): \"Long DJ Industrial Average, Nov. 17 to Jan. 4\" Enter 11/17, Exit 1/4, Stop %: 9", N_DJ, c("34", "94.12%", "128.30 Dow points", "1970-2003 (entry years)", "")),
        kd!("jbhsms_dj_1203", "Dow key date · long Dec 3 to Jan 6, 9% stop", "F|Figure 7.20 (p.130): \"Long DJ Industrial Average, Dec. 3 to Jan. 6\" Enter 12/3, Exit 1/6, Stop %: 9", N_DJ, c("34", "85.29%", "120.22 Dow points", "1970-2003 (entry years)", "")),
        kd!("jbhsms_dj_0218", "Dow key date · long Feb 18 to May 4, 15% stop", "F|Figure 7.21 (p.131): \"Long DJ Industrial Average, Feb. 18 to May 4\" Enter 2/18, Exit 5/4, Stop %: 15", N_DJ, c("35", "82.86%", "175.34 Dow points", "1970-2004", "")),
        kd!("jbhsms_dj_0313", "Dow key date · long Mar 13 to Mar 18, 3% stop", "F|Figure 7.22 (p.132): \"Long DJ Industrial Average, Mar. 13 to Mar. 18\" Enter 3/13, Exit 3/18, Stop %: 3", N_DJ, c("35", "77.14%", "78.73 Dow points", "1970-2004", "")),
        kd!("jbhsms_dj_0407", "Dow key date · long Apr 7 to Apr 16, 15% stop", "F|Figure 7.23 (p.133): \"Long DJ Industrial Average, Apr. 7 to Apr. 16\" Enter 4/7, Exit 4/16, Stop %: 15", N_DJ, c("35", "77.14%", "69.14 Dow points", "1970-2004", "")),
        kd!("jbhsms_dj_0528", "Dow key date · long May 28 to Jun 6 (stop 100% = none)", "F|Figure 7.24 (p.134): \"Long DJ Industrial Average, May 28 to Jun. 6\" Enter 5/28, Exit 6/6, Stop %: 100", N_DJ, c("35", "82.86%", "60.56 Dow points", "1970-2004", "")),
        def!("jbhsms_sp_1027", "S&P key date · long Oct 27 to Nov 1, 1.5% stop", "Ch.7 · Short-Term Key Date Seasonal Trades", "Long", NotTestable("duplicate of jb30d_sp_seasonal (same dates, same 1.5% close-only stop); Figure 7.9 (p.118): 22 trades 1982-2003, 95.45%, avg 15.33 points"),
            &["F|Figure 7.9 (p.118): Long, Enter 10/27, Exit 11/1, Stop %: 1.50"], ""),
        kd!("jbhsms_stk_aa", "Stock key date · Alcoa long Jan 22 to Feb 18, 15% stop", "F|Figure 7.11 (p.121): \"Alcoa-AA, Jan. 22 to Feb. 18\" Long, Enter 1/22, Exit 2/18, Stop %: 15", "Book record: Alcoa (AA) 1972-2004. In the vault's US data the old Alcoa history (1951 on) sits under today's ticker HWM (Howmet, the renamed Alcoa Inc.); today's AA is the 2016 spin-off. Exit on the first close on or after the exit date; a close below entry x (1 - stop%) before then exits on that close.", c("33", "75.76%", "$0.5544 per share", "1972-2004", "Alcoa")),
        kd!("jbhsms_stk_ko", "Stock key date · Coca-Cola long Mar 31 to Jun 6 (no stop)", "F|Figure 7.12 (p.122): \"Coca-Cola-KO, Mar. 31 to Jun. 6\" Long, Enter 3/31, Exit 6/6, Stop %: 100", N_STK, c("35", "80%", "$1.8364 per share", "1970-2004", "Coca-Cola")),
        kd!("jbhsms_stk_ge", "Stock key date · General Electric long May 25 to Jun 6, 6% stop", "F|Figure 7.13 (p.123): \"General Electric-GE, May 25 to Jun. 6\" Long, Enter 5/25, Exit 6/6, Stop %: 6", N_STK, c("35", "82.86%", "$0.2331 per share", "1970-2004", "General Electric")),
        kd!("jbhsms_stk_msft", "Stock key date · Microsoft long Aug 31 to Feb 1 (no stop)", "F|Figure 7.14 (p.124): \"Microsoft-MSFT, Aug. 31 to Feb. 1\" Long, Enter 8/31, Exit 2/1, Stop %: 100", N_STK, c("18", "88.89%", "$2.3881 per share", "1986-2003 (entry years)", "Microsoft")),
        def!("jbhsms_stk_nem", "Stock key date · Newmont Mining Sep 26 to Oct 27 (book contradicts itself)", "Ch.7 · Short-Term Key Date Seasonal Trades", "Long", NotTestable("Figure 7.15 (p.125) is headed \"Long\", but every profit in its table is entry price minus exit price (e.g. 1983: in 21.44, out 19.23, profit +2.21), i.e. a short; the printed 77.27% winners holds only as a short (our NEM data: 5 of 22 long trades win, 17 of 22 as a short). Which side is the rule needs a user decision"),
            &["F|Figure 7.15 (p.125): \"Newmont Mining-NEM, Sep. 26 to Oct. 27\" Long, Enter 9/26, Exit 10/27, Stop %: 100; 22 trades 1982-2003, 77.27%", R_ENTRY], ""),
        kd!("jbhsms_stk_pg", "Stock key date · Procter & Gamble long Oct 26 to Dec 27 (no stop)", "F|Figure 7.16 (p.126): \"Proctor & Gamble-PG, Oct. 26 to Dec. 27\" Long, Enter 10/26, Exit 12/27, Stop %: 100", N_STK, c("34", "85.29%", "$1.1551 per share", "1970-2003", "Procter & Gamble")),
        kd!("jbhsms_stk_txn", "Stock key date · Texas Instruments long Dec 15 to Apr 19 (no stop)", "F|Figure 7.18 (p.128): \"Texas Instruments-TXN, Dec. 15 to Apr. 19\" Long, Enter 12/15, Exit 4/19, Stop %: 100", N_STK, c("34", "85.29%", "$1.7975 per share", "1970-2003 (entry years)", "Texas Instruments")),
        def!("jbhsms_stk_sunw", "Stock key date · Sun Microsystems long Nov 2 to Jan 18 (no stop)", "Ch.7 · Short-Term Key Date Seasonal Trades", "Long", NeedsData("daily prices of Sun Microsystems (SUNW, delisted 2010 after the Oracle takeover); the vault's US stock data has no SUNW history"),
            &["F|Figure 7.17 (p.127): \"Sun Microsystems-SUNW, Nov. 2 to Jan. 18\" Long, Enter 11/2, Exit 1/18, Stop %: 100; 17 trades 1987-2003, 94.12%", R_ENTRY],
            "Same key-date engine as the other stock trades; it would run unchanged on SUNW prices."),
        StrategyDef { claim: c("119", "84.0%", "$442 (500 shares, $25 commission)", "1.55 bars", "IBM 01/20/2000-07/23/2004, PF 2.50"),
          ..def!("jbhsms_gap_ibm", "Gap trade · IBM inputs (gap 20c up / 10c down, penetration 35c / 50c, 3.9% stop, FPO)", "Ch.6 · Gap Your Way to Profits", "Long/Short", Live,
            &[R_GAP[0], R_GAP[1], "F|Table 6.1 (p.91): \"IBM 1998-2004. Gap open 20 cents above previous daily high or gap open 10 cents below previous daily low; penetration of 35 cents back into high of previous day and penetration of 50 cents into previous daily low. Exit on first profitable opening with 3.9 percent stop loss below entry price\""],
            N_GAP) },
        StrategyDef { claim: c("56", "92.9%", "$741 (500 shares, $25 commission)", "2.16 bars", "IBM 01/20/2000-06/09/2004, PF 5.25"),
          ..def!("jbhsms_gap_ibm2", "Gap trade · IBM inputs 2 (gap 30c up / 90c down, penetration 35c / 80c, 4.75% stop, FPO)", "Ch.6 · Gap Your Way to Profits", "Long/Short", Live,
            &[R_GAP[0], R_GAP[1], "F|Table 6.2 (p.93-94): \"Size of opening gap up: 30 cents; Size of opening gap down: 90 cents; Penetration down into previous daily high: 35 cents; Penetration up into previous daily low: 80 cents; Stop loss: 4.75 percent\""],
            N_GAP) },
        StrategyDef { claim: c("57", "87.7%", "$476 (500 shares, $25 commission)", "1.30 bars", "EBAY 11/30/1998-07/23/2004, PF 8.19"),
          ..def!("jbhsms_gap_ebay", "Gap trade · EBAY inputs (gap 48c / 48c, penetration 4c / 98c, 3.55% stop, FPO)", "Ch.6 · Gap Your Way to Profits", "Long/Short", Live,
            &[R_GAP[0], R_GAP[1], "F|Table 6.3 (p.94-95): \"Size of opening gap up: 48 cents; Size of opening gap down: 48 cents; Penetration down into previous daily high: 4 cents; Penetration up into previous daily low: 98 cents; Stop loss: 3.55 percent\""],
            N_GAP) },
        def!("jbhsms_fri_mon", "Friday-Monday pattern (stocks / SPY / QQQ / DIA)", "Ch.8 · Day-of-Week Patterns", "Long/Short", NotTestable("the penetration \"given amount\" and the stop are not given (\"the triggers and stops for each of these stocks are different\", p.151); the S&P inputs of other books are jbcdt2_dbo_mon / jbeft_dow_fm97 / jb624_fri_mon"),
            &["F|\"examine the relationship between the opening price and the closing price in S&P futures on Friday (or the last trading day of the week if the market is closed on Friday)\" (p.142)",
              "E|\"place an order to BUY at a given amount ABOVE the HIGH of Friday\" (p.144); sell short below the Friday low \"by a given amount\" (p.144-145)",
              "X|\"The smart stop for this trading method is a dollar-risk stop or a percentage stop of the entry price\"; \"close out the Friday-Monday trade on the first profitable opening (FPO)\" (p.147-148)"], ""),
        def!("jbhsms_mac", "High-low moving average channel (MAC 10/8)", "Ch.9 · Surfing the Stock Channel", "Long/Short", NotTestable("duplicate: the two-bar MAC trend change is jb624_mac2_sar and the buy-at-support use is jb624_mac_sr / jbsmsw_mac_pullback; this chapter adds no exit rule"),
            &["E|\"Use a 10.MA of highs and an 8.MA of lows. Two consecutive price bars entirely above the high of the MAC suggests a change in trend from down to up\" (p.165)"], ""),
        def!("jbhsms_ma200", "200-day moving average results for SPY", "Ch.9 · Surfing the Stock Channel", "Long/Short", NotTestable("Figure 9.7 (p.163) prints results (41 trades, 26.8%, $2,896 on 1,000 shares) but not the entry/exit rule"),
            &["F|\"The 200-Day MA Results for SPY\" (Figure 9.7, p.163)"], ""),
    ]
}

/// Admission (bernstein-g3, 2026-10-22): every Live key-date and gap id, reviewed with independent chronological ledgers.
pub fn reviewed_daily(id: &str) -> bool {
    KD.iter().any(|k| k.0 == id) || gap_rule(id)
}
/// Gap trades: per-share US cent inputs, US stocks only; they read the open (stop entry, first profitable opening).
pub fn gap_rule(id: &str) -> bool {
    GAP.iter().any(|g| g.0 == id)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if let Some(k) = KD.iter().find(|k| k.0 == id) {
        if id.starts_with("jbhsms_stk_") {
            return Some(("jbhsms_kd_stocks", "Stock key-date seasonals (Hot Stock Market Strategies)", k.0.trim_start_matches("jbhsms_stk_")));
        }
        let sp = id.starts_with("jbhsms_sp_");
        let lbl: &'static str = match id {
            "jbhsms_sp_0113" => "S&P Jan 13-Jan 18", "jbhsms_sp_0226" => "S&P Feb 26-Mar 5", "jbhsms_sp_0301" => "S&P Mar 1-Mar 6",
            "jbhsms_sp_0414" => "S&P Apr 14-Apr 22", "jbhsms_sp_0601" => "S&P Jun 1-Jun 6", "jbhsms_sp_0710" => "S&P Jul 10-Jul 15",
            "jbhsms_sp_0815" => "S&P Aug 15-Aug 25", "jbhsms_sp_1220" => "S&P Dec 20-Jan 7", "jbhsms_dj_0405" => "Dow Apr 5-Apr 22",
            "jbhsms_dj_1117" => "Dow Nov 17-Jan 4", "jbhsms_dj_1203" => "Dow Dec 3-Jan 6", "jbhsms_dj_0218" => "Dow Feb 18-May 4",
            "jbhsms_dj_0313" => "Dow Mar 13-Mar 18", "jbhsms_dj_0407" => "Dow Apr 7-Apr 16", _ => "Dow May 28-Jun 6",
        };
        let _ = k;
        return Some(if sp { ("jbhsms_kd_sp", "S&P key-date seasonals (Hot Stock Market Strategies)", lbl) } else { ("jbhsms_kd_dj", "Dow key-date seasonals (Hot Stock Market Strategies)", lbl) });
    }
    Some(match id {
        "jbhsms_sp_1027" => ("jbhsms_kd_sp", "S&P key-date seasonals (Hot Stock Market Strategies)", "S&P Oct 27-Nov 1 · see jb30d_sp_seasonal"),
        "jbhsms_stk_sunw" => ("jbhsms_kd_stocks", "Stock key-date seasonals (Hot Stock Market Strategies)", "sunw · needs data"),
        "jbhsms_stk_nem" => ("jbhsms_kd_stocks", "Stock key-date seasonals (Hot Stock Market Strategies)", "nem · book contradicts itself"),
        "jbhsms_gap_ibm" => ("jbhsms_gap", "Gap trade in stocks (Hot Stock Market Strategies)", "IBM inputs (Table 6.1)"),
        "jbhsms_gap_ibm2" => ("jbhsms_gap", "Gap trade in stocks (Hot Stock Market Strategies)", "IBM inputs 2 (Table 6.2)"),
        "jbhsms_gap_ebay" => ("jbhsms_gap", "Gap trade in stocks (Hot Stock Market Strategies)", "EBAY inputs (Table 6.3)"),
        "jbhsms_fri_mon" | "jbhsms_mac" | "jbhsms_ma200" => ("jbhsms_other", "Hot Stock Market Strategies · other methods", "Not run"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if let Some(g) = GAP.iter().find(|g| g.0 == id) {
        return Some(vec![p("gap_up", "Opening gap above the prior high ($ per share)", g.1, 0., 5., 0.01), p("gap_dn", "Opening gap below the prior low ($ per share)", g.2, 0., 5., 0.01),
            p("pen_hi", "Sell stop: penetration down into the prior high ($)", g.3, 0., 5., 0.01), p("pen_lo", "Buy stop: penetration up into the prior low ($)", g.4, 0., 5., 0.01),
            p("stop_pct", "Stop loss, % of the entry price", g.5, 0.5, 20., 0.05), p("entry_day", "Entry-day stop: 0 = bar path O-near-far-C, 1 = any touch [Not from the book]", 0., 0., 1., 1.)]);
    }
    let k = KD.iter().find(|k| k.0 == id)?;
    Some(vec![p("entry_m", "Entry month", k.1, 1., 12., 1.), p("entry_d", "Entry day", k.2, 1., 31., 1.), p("exit_m", "Exit month", k.3, 1., 12., 1.),
        p("exit_d", "Exit day", k.4, 1., 31., 1.), p("stop_pct", "Close-only stop, % below the entry close (100 = none)", k.5, 0.5, 100., 0.5)])
}

/// Key-date trade: buy the close of the first bar on or after the entry date; sell the close of the first bar on or
/// after the exit date (next year when the exit month/day precedes the entry's); close-only stop below entry x (1 - s%).
pub fn keydate(x: &Ctx, from: usize, to: usize, em: f64, ed: f64, xm: f64, xd: f64, stop_pct: f64) -> Vec<Trade> {
    let (em, ed, xm, xd) = (em.round() as i64, ed.round() as i64, xm.round() as i64, xd.round() as i64);
    let wraps = (xm, xd) < (em, ed);
    let stop = if stop_pct >= 100.0 { f64::NAN } else { stop_pct / 100.0 };
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (Vec::new(), None);
    let mut exit_k = i64::MAX;
    for i in from.max(1)..=to {
        let di = days(&x.r[i].date);
        if let Some(t) = cur.as_ref() {
            let e = t.legs[0].px;
            if di >= exit_k {
                out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Exit date (close)"));
            } else if x.c[i] < e * (1.0 - stop) {
                out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Close-only stop"));
            }
            continue;
        }
        let y: i64 = x.r[i].date[0..4].parse().unwrap_or(1970);
        let ent = days_ymd(y, em, ed);
        let ext = days_ymd(if wraps { y + 1 } else { y }, xm, xd);
        if i < to && di >= ent && days(&x.r[i - 1].date) < ent && di < ext {
            exit_k = ext;
            cur = Some(new_trade(1.0, i, x.c[i], "close", i));
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

/// Gap trade (ch.6): long when the open is more than `gap_dn` below the prior low and price then trades up to the prior
/// low + `pen_lo` (buy stop); short mirror with `gap_up` / `pen_hi` at the prior high. Exit at the first later open
/// beyond the entry price, or the stop `stop` x entry from it (gap through: at the open; entry day: bar path or any touch).
#[allow(clippy::too_many_arguments)]
pub fn gap_fpo(x: &Ctx, from: usize, to: usize, gap_up: f64, gap_dn: f64, pen_hi: f64, pen_lo: f64, stop: f64, entry_any: bool) -> Vec<Trade> {
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (Vec::new(), None);
    let mut sl = f64::NAN;
    for i in from.max(1)..=to {
        if let Some(t) = cur.as_ref() {
            let (d, e) = (t.dir, t.legs[0].px);
            if (x.o[i] - e) * d > 0.0 {
                out.push(finish(cur.take().unwrap(), i, x.o[i], i, "First profitable opening"));
            } else if let Some(px) = stop_fill(x, i, -d, sl) {
                out.push(finish(cur.take().unwrap(), i, px, i, "Stop loss"));
            }
            continue; // no new entry on an exit day
        }
        if i >= to {
            continue;
        }
        let (d, lvl) = if x.o[i] < x.l[i - 1] - gap_dn { (1.0, x.l[i - 1] + pen_lo) } else if x.o[i] > x.h[i - 1] + gap_up { (-1.0, x.h[i - 1] - pen_hi) } else { continue };
        if let Some(px) = stop_fill(x, i, d, lvl) {
            sl = px * (1.0 - d * stop);
            let t = new_trade(d, i, px, "stop", i - 1);
            let hit = if entry_any { if d > 0.0 { x.l[i] <= sl } else { x.h[i] >= sl } } else { hit_after_entry(x, i, d, lvl, sl) };
            if hit { out.push(finish(t, i, sl, i, "Stop loss (entry day)")); } else { cur = Some(t); }
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jbhsms_") || to >= x.c.len() || from > to {
        return None;
    }
    if params(id).is_none() {
        return if catalogue().iter().any(|s| s.id == id) { Some(vec![]) } else { None };
    }
    let g = |k: &str| getp(id, p, k);
    if GAP.iter().any(|k| k.0 == id) {
        return Some(gap_fpo(x, from, to, g("gap_up"), g("gap_dn"), g("pen_hi"), g("pen_lo"), g("stop_pct") / 100.0, g("entry_day") >= 0.5));
    }
    Some(keydate(x, from, to, g("entry_m"), g("entry_d"), g("exit_m"), g("exit_d"), g("stop_pct")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: c, high: c, low: c, close: c, volume: None }
    }
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "IBM".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn gap_entry_stop_and_first_profitable_opening() {
        // prior low 100; open 99.80 (> 0.10 below) -> buy stop 100.50 fills; next open 100.40 not profitable, 101 is.
        let r = vec![bar("2004-01-05", 101., 102., 100., 101.), bar("2004-01-06", 99.8, 101., 99.7, 100.9), bar("2004-01-07", 100.4, 100.6, 99.9, 100.2),
            bar("2004-01-08", 101., 101.5, 100.8, 101.2), bar("2004-01-09", 101., 101.5, 100.8, 101.2)];
        let x = Ctx::new(&r, &r, &[]);
        let t = gap_fpo(&x, 0, 4, 0.2, 0.1, 0.35, 0.5, 0.039, false);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1, 100.5, 3, 101.));
        // a gap smaller than 0.10: no trade
        let mut r2 = r.clone();
        r2[1].open = 99.95;
        let x2 = Ctx::new(&r2, &r2, &[]);
        assert!(gap_fpo(&x2, 0, 4, 0.2, 0.1, 0.35, 0.5, 0.039, false).is_empty());
        // short: prior high 102, open 102.30 (> 0.20 above) -> sell stop 101.65; 3.9% stop gapped through next day
        let r3 = vec![bar("2004-01-05", 101., 102., 100., 101.), bar("2004-01-06", 102.3, 102.4, 101.5, 101.6), bar("2004-01-07", 106., 107., 105.5, 106.5)];
        let x3 = Ctx::new(&r3, &r3, &[]);
        let t3 = gap_fpo(&x3, 0, 2, 0.2, 0.1, 0.35, 0.5, 0.039, false);
        assert_eq!((t3[0].dir, t3[0].legs[0].px, t3[0].exit_i, t3[0].exit_px), (-1.0, 101.65, 2, 106.));
    }
    #[test]
    fn year_wrap_and_weekend_entry() {
        // Dec 20 2003 is a Saturday -> enter Mon Dec 22; exit Jan 7 2004 (Wed)
        let r = vec![row("2003-12-19", 100.), row("2003-12-22", 101.), row("2003-12-23", 100.), row("2004-01-06", 103.), row("2004-01-07", 104.), row("2004-01-08", 99.)];
        let x = Ctx::new(&r, &r, &[]);
        let ts = keydate(&x, 0, 5, 12., 20., 1., 7., 3.0);
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].i, ts[0].exit_i), (1, 4));
        // close-only stop: 3% below 101 = 97.97
        let r2 = vec![row("2003-12-19", 100.), row("2003-12-22", 101.), row("2003-12-23", 97.9), row("2004-01-07", 104.)];
        let x2 = Ctx::new(&r2, &r2, &[]);
        let t2 = keydate(&x2, 0, 3, 12., 20., 1., 7., 3.0);
        assert_eq!((t2[0].exit_i, t2[0].reason.as_str()), (2, "Close-only stop"));
    }
}
