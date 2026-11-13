//! Larry Williams, "Long-Term Secrets to Short-Term Trading" (Wiley 1999). Book id `lwlts`.
//! Source copy: Books/Larry Williams/Long-Term Secrets to Short-Term Trading.pdf. Transcription, page images
//! and checks: library/lwlts/. Page refs below are printed pages (PDF page = printed + 6 roughly; see README).
//!
//! The book tested one S&P 500 futures contract (CSI back-adjusted "S&P 500 IND-9967", System Writer /
//! TradeStation, report header "P. Value $2.500" per 0.01 = $250 per index point). Dollar stops and targets are
//! converted to index points at the book's own $250 per point (`pt_usd`); on other instruments the same POINT
//! distance is used (instrument swap only, no rescaling).
//!
//! Intrabar order (daily bars): TradeStation's default bar path. If the open is nearer the high the bar runs
//! O->H->L->C, else O->L->H->C. Stop entries, protective stops and targets fill at their price when the path
//! crosses it, or at the open when the bar gaps through. Protective stops are armed on the entry bar unless the
//! book says otherwise (`stop_day0`). "Bailout" = exit on the first opening after the entry day that is
//! profitable by any amount (Ch. 11 rule 2, printed p.157).
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwlts";

/// Larry Williams block: rules admitted after the 23 Oct 2026 reading and independent ledgers
/// (authors/lwilliams/admission/SOURCE-CONTRACT.md). SPX_POINTS = dollar stops/targets kept as S&P points: actual SPX only.
pub const REVIEWED: &[&str] = &["lwlts_mon_gap_t500", "lwlts_mon_gap_t1000", "lwlts_mon_gap_close", "lwlts_mon_gap_next_close", "lwlts_mon_gap_6close",
    "lwlts_vb_sp_tdw", "lwlts_vb_sp_40_200", "lwlts_talon_sp", "lwlts_mon_oc", "lwlts_mon_3day",
    "lwlts_tdm6_sp", "lwlts_tdm7_sp", "lwlts_tdm8_sp", "lwlts_tdm19_sp", "lwlts_tdm20_sp", "lwlts_tdm21_sp", "lwlts_tdm22_sp",
    "lwlts_study_all", "lwlts_study_down1", "lwlts_study_down3", "lwlts_study_trend", "lwlts_study_trend_mon",
    "lwlts_outside_down", "lwlts_outside_down_xthu", "lwlts_3up_tue_sell", "lwlts_oops_buy", "lwlts_oops_sell_thu", "lwlts_oops_buy_ma9", "lwlts_oops_sell_ma9",
    "lwlts_month_day1", "lwlts_hh1", "lwlts_hh5", "lwlts_hh10", "lwlts_hh15", "lwlts_hh20"];
pub const SPX_POINTS: &[&str] = &["lwlts_mon_gap_t500", "lwlts_mon_gap_t1000", "lwlts_mon_gap_close", "lwlts_mon_gap_next_close", "lwlts_mon_gap_6close",
    "lwlts_talon_sp", "lwlts_tdm6_sp", "lwlts_tdm7_sp", "lwlts_tdm8_sp", "lwlts_tdm19_sp", "lwlts_tdm20_sp", "lwlts_tdm21_sp", "lwlts_tdm22_sp",
    "lwlts_study_all", "lwlts_study_down1", "lwlts_study_down3", "lwlts_study_trend", "lwlts_study_trend_mon",
    "lwlts_outside_down", "lwlts_outside_down_xthu", "lwlts_3up_tue_sell", "lwlts_oops_buy", "lwlts_oops_sell_thu", "lwlts_oops_buy_ma9", "lwlts_oops_sell_ma9",
    "lwlts_month_day1", "lwlts_hh1", "lwlts_hh5", "lwlts_hh10", "lwlts_hh15", "lwlts_hh20"];

const SP: &str = "Book: one S&P 500 futures contract, CSI back-adjusted continuous data, $250/point as printed in every report header; dollar amounts here are converted at that rate (p.pt_usd). Since 23 Oct 2026 rules with dollar amounts run on actual SPX only (the points mean what the book meant only there), from 2014 when SPX opens are real; rules without dollar amounts run on indices, ETFs and stocks.";
const BONDS: &str = "Needs daily T-Bond futures (and for some rules Gold futures) prices; the vault has no bond or gold futures series.";

macro_rules! live {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: Status::Live, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
}
macro_rules! off {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const MON_F: &str = "E|\"buy on the open every Monday if that open is lower than Friday's close\" (p.51; Monday = the bar's weekday, a Monday holiday means no trade)";
const STOP3000: &str = "X|$3,000 stop (Fig. 3.7-3.11 headers; largest loss $3,050 = stop + $50 commission) = 12 S&P points, armed on the entry day";
const TDM_R: &[&str] = &["E|\"buying ... on the opening of the trading day of the month shown\"; TDM = count of trading days so far this month (p.88)",
    "X|\"exited with either a $2,500 stop in the S&P 500 ... or on the close the third day after entry\" (p.88-89)",
    "X|\"The S&P results reflect no stop on entry day, but a $2,000 stop after entry day\" (p.88): default $2,000 = 8 points from the day after entry (p.stop_usd=2500 is the p.88 figure)"];
/// Lane F1C (R4 #293): Table 6.7's T-bond TDMs on the Norgate T-bond continuous contract ($1,000 per point, so $1,500 = 1.5 points).
const TDMB_R: &[&str] = &["E|\"Our entry rule is to buy or sell on the open of the TDM shown\"; TDM = count of trading days so far this month (p.88)",
    "X|\"exited with either a $2,500 stop in the S&P 500 or $1,500 in the Bonds or on the close the third day after entry\" (p.88)",
    "X|\"the Bonds used a $1,500 stop starting on entry day\" (p.89): 1.5 points of the T-bond contract ($1,000 per point)"];
const TDMB_N: &str = "Table 6.7 (p.89, OCR authors/f1c/lwlts_p93_ocr.txt), T-Bonds 1977-1998, each TDM tested on its own; 'Number of Trades' prints trades/winners. Runs on the Norgate T-bond continuous contract only (lane F1C, R4 #293): signals and points on the back-adjusted series, % per trade on the unadjusted entry fill price, $1,500 = 1.5 points. The trading-day count is the contract's own calendar. Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor.";
const TDM_N: &str = "Table 6.6 (p.89), S&P 1982-1998, each TDM tested on its own; the table's 'Number of Trades' column prints trades/winners. Long only (the table lists rally days).";
const STUDY_X: &str = "X|\"buy ... and exit on the next close with a $3,250 stop\" (p.94); Read: entry on the next open, exit at the close of the day after entry (2-bar hold: fits 2,064 trades in Jul-1982..Feb-1998; 'buy on the open tomorrow and exit on the next day's close' p.95)";
const STOP2000: &str = "X|$2,000 stop (Fig. headers: losers exactly -$2,000) = 8 points, armed on the entry day";
const BAIL: &str = "X|Bailout: exit on the first profitable opening after entry (Ch. 11, p.157)";
const OOPS_B: &str = "E|\"an opening that is below the previous day's low ... The entry comes when ... price then rallies back to the previous day's low\" (p.114): buy stop at yesterday's low, only on a day that opened below it";
const OOPS_S: &str = "E|\"an open greater than the prior day's high ... Our entry then comes from price falling back to the prior high\" (p.114): sell stop at yesterday's high, only on a day that opened above it";
const HH_R: &[&str] = &["F|\"today's high is less than the highest high of the last X days\" (p.239): High < Highest(High, X) of the X bars before today",
    "E|\"and price makes a new high tomorrow, which puts us long buying a new X day high\": buy stop tomorrow at Highest(High, X) including today",
    "X|\"exit 13 days later $3,500 stop\" (Fig. 15.4 caption): close of the 13th bar after entry; $3,500 = 14 points"];
/// Lane F1D (R4 #293, #297): the Aug 1997 letter's bond system (p.212-213) on the Norgate T-bond contract ($1,000 per point).
const B8_R: &[&str] = &["E|\"Rule one was to buy today's close if it was greater than the close 8 days ago\" (p.212): Close > Close 8 bars ago, market on this close",
    "X|\"I placed a stop of $850\" (p.212): 0.85 point below the entry",
    "X|\"held until a profit of about three times the stop was tested. The best was a profit of $3,500\" (p.213): 3.5 points above the entry",
    "X|Unstated in the book: where the stop and target are measured from; vault reading: the entry fill (this close)",
    "E|Unstated in the book: re-entry after an exit; vault reading: a new entry on any close that meets rule one while flat, including the close of the bar that stopped or targeted the trade out"];
const B8_N: &str = "Adapted: 2 unstated details (labelled on the rules; R4 #297). Book (p.213): \"one could have made $36,500 trading one Bond since they began trading ... the average profit per trade, after $50 commissions and slippage, was an attractive $160\" (about 228 trades); window 1977 to the August 1997 letter. Runs on the Norgate T-bond continuous contract only (lane F1D, R4 #293): signals and points on the back-adjusted series, % per trade on the unadjusted entry fill price. A later bar that reaches both the stop and the target follows the TradeStation bar path (open, the nearer extreme, the other extreme, close). On Norgate &ZB, from 1999-12 to 2000-10 the open sits at the previous close (median gap at most 1 tick, a third of days exactly equal), so open fills and open-based levels in those 11 months are not true opening prices (R4 #331; the whole history is kept, as for the NIFTY opens). 16 of this card's &ZB trades are affected (held through those months; none fills at an open there).";
/// Lane F1D (R4 #293, #297): the book's T-bond rules on the Norgate T-bond contract ($1,000 per point; the printed reports read
/// "DAY T-BONDS", $31.25 per 1/32).
const ZB_N: &str = "Runs on the Norgate T-bond continuous contract only (lane F1D, R4 #293): signals and points on the back-adjusted series, % per trade on the unadjusted entry fill price, book dollars at $1,000 per point. Daily bars follow the TradeStation bar path (open, the nearer extreme, the other extreme, close); an opposite signal exits and reverses (Ch. 11 rule 3). On Norgate &ZB, from 1999-12 to 2000-10 the open sits at the previous close (median gap at most 1 tick, a third of days exactly equal), so open fills and open-based levels in those 11 months are not true opening prices (R4 #331; the whole history is kept, as for the NIFTY opens).";
const VBB_R: &[&str] = &["E|\"buying and selling bonds on the open every day at a distance of 100 percent of the previous day's range above the open for a buy and 100 percent below the open for a sell\" (p.61)",
    "X|\"A protective stop of $1,500 or 50 percent of the previous day's range subtracted from our entry\" (p.61). Unstated in the book: which of the two; vault reading: the nearer one (the smaller distance), armed on the entry day",
    "X|\"our exit is the Bail Out or the first profitable opening after entry technique\" (p.61)",
    "X|Unstated in the book: an opposite signal while in a trade; vault reading: it exits and reverses (Ch. 11 rule 3, as the S&P volatility cards)"];
const OUTB_R: &[&str] = &["F|Outside day with a down close below the previous day's low (the S&P pattern, p.95): \"taking all outside day down closes followed by a lower opening the next day in Bonds\" (p.98)",
    "E|Buy on the next day's opening if it opens below the outside day's close",
    "X|\"we will take a $1,500 loss or exit on the first profitable opening\" (p.98): 1.5 points, armed on the entry day"];
const VBB_TDW_R: &[&str] = &[VBB_R[0], "F|\"the best days to buy have been Tuesdays and Thursdays, whereas the best sell days have been Wednesdays and Thursdays\" (p.65)", VBB_R[1], VBB_R[2], VBB_R[3]];
const OUTB_X_R: &[&str] = &[OUTB_R[0], "F|\"If we take the trade on any day but Thursday\" (p.98): the entry day", OUTB_R[1], OUTB_R[2]];
const OOPSB_N: &str = "Oops! (p.114): buy at a stop at yesterday's low on a day that opens below it; sell at a stop at yesterday's high on a day that opens above it.";
const GSVB_R: &[&str] = &["F|Buy setup: \"today's close lower than the close 5 days ago\" and buy only on Tuesday, Wednesday and Friday (p.124)",
    "E|\"the difference from open to the high for each of the past 4 days and divide that by 4 ... a buyer above the opening at an amount equal to 180 percent of the 4-day swing value average\" (p.124)",
    "F|Sell setup: \"Bonds closing greater than the close 6 days ago\" (p.125); sells on any day (the book limits only the buys)",
    "E|Sell: \"the distance from the open to the low for each of the last 4 days and divide by 4 ... multiplied by 180 percent and subtracted from the opening\" (p.125)",
    "F|Option p.gold=1 (not the base rule): \"for even better performance, I would also like to see the price of Gold lower than the price of Gold 20 days ago\" (p.125), read on the Norgate gold contract's own bars as Data2",
    "X|\"my stop is $1,600\" (p.125); \"my computer software does not allow us to bring into play a protective stop on the day of entry\" (p.126): from the day after entry",
    "X|\"I will take profits on the first profitable opening after being in the trade for 2 days\" (p.125). Unstated in the book: the count; vault reading: from the opening of the 2nd bar after the entry bar",
    "X|Unstated in the book: an opposite signal while in a trade; vault reading: it exits and reverses (Ch. 11 rule 3)"];
/// Lane F1D: Ch. 10 bond month trades (p.149-156; reports 01/01/86-08/28/98, OCR authors/f1d/w/lw_p152..157.txt).
const TDMG_R: &[&str] = &["E|\"buying Bonds on the opening of TDM 18 with a $1,500 stop and exiting on the close 3 days after entry\" (p.153; TDM 22: \"delaying our entry until TDM 22\", p.154); TDM = count of trading days so far this month",
    "F|\"trades were only taken if Gold was in a downtrend (i.e., the close of Gold on the day prior to entry was less than 24 days ago)\" (p.153): the Norgate gold contract as Data2, its close on the bond's previous bar (gold's last bar on or before that date) below its close 24 gold bars earlier",
    "X|$1,500 = 1.5 points from the entry day (\"the Bonds used a $1,500 stop starting on entry day\", p.89); out on the close of the third day after entry"];
const TDM12_R: &[&str] = &["E|\"The rules called for selling on the open of TDM 12\" (p.155)",
    "X|\"with our usual 3-day exit and $1,400 stop\" (p.155): out on the close of the third day after entry; $1,400 = 1.4 points from the entry day (the bond TDM convention, p.89)"];
const TDM12G_F: &str = "F|\"trades were only taken when Gold had closed greater than the close 10 days ago\" (p.156-157): the Norgate gold contract as Data2, its close on the bond's previous bar above its close 10 gold bars earlier (reading: the day prior to entry, as the buy filter p.153)";
const TDM12G_R: &[&str] = &[TDM12_R[0], TDM12G_F, TDM12_R[1]];
/// Lane F1D: S&P rules filtered by T-bonds, on the Norgate S&P 500 contract with the Norgate T-bond contract as Data2.
const SPB_N: &str = "Runs on the Norgate S&P 500 continuous contract only, with the Norgate T-bond contract as Data2 (the bond's own bars, carried to the S&P dates, TradeStation Data2; R4 #314 (4)); refused when the bond data is not loaded (lane F1D, R4 #293). Book dollars at the book's $250 per S&P point (report header P. Value $2.50 per 0.01). Published only 1982-04-21..1995-07-31 (R4 #330, mhstp::OPEN_WINDOW): from August 1995 Norgate's &SP open is the overnight-session or a vendor open, not the pit open these entries and gap fills read. Daily bars follow the TradeStation bar path (open, the nearer extreme, the other extreme, close).";
const BCH_E: &str = "E|\"buying the S&P 500 (market on close) any day that the Bond market closed higher than the highest high of the last 14 days\" (p.236): the bond's close above the highest bond high of the 14 bars before it. Unstated in the book: whether the 14 days include today; vault reading: the 14 bars before";
const HH_N: &str = "Fig. 15.4 (p.239), S&P 500; test window not printed. Read: TradeStation Highest(High,X) on today's bar as the stop level, setup uses the X bars before today. Stop armed on the entry day.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{NeedsData, NotTestable};
    vec![
        // ---- Chapter 3: Monday gap-down buy (p.51-54)
        live!("lwlts_mon_gap_t500", "Monday gap-down buy · $500 target", "Ch 3 · Hold to the close", "Long",
            &[MON_F, "X|\"$3,000 ... and an automatic $500 profit\" (p.51): $500 = 2 points limit, else exit on the close", STOP3000],
            SP, claim("389", "59%", "$-20.95 (after $50 comm.)", "same day", "07/02/82-08/24/98")),
        live!("lwlts_mon_gap_t1000", "Monday gap-down buy · $1,000 target", "Ch 3 · Hold to the close", "Long",
            &[MON_F, "X|\"all the same rules except a $1,000 target\" (p.51): 4 points, else exit on the close", STOP3000],
            SP, claim("389", "55%", "$35.31", "same day", "07/02/82-08/24/98")),
        live!("lwlts_mon_gap_close", "Monday gap-down buy · exit on the close", "Ch 3 · Hold to the close", "Long",
            &[MON_F, "X|\"holding until the close and then exiting the position\" (p.52)", STOP3000],
            SP, claim("389", "53%", "$100.45", "same day", "07/02/82-08/24/98")),
        live!("lwlts_mon_gap_next_close", "Monday gap-down buy · exit next close", "Ch 3 · Hold to the close", "Long",
            &[MON_F, "X|\"hold the position ... until the next close; that is, the first close after our entry day or until we are stopped out\" (p.53)", STOP3000],
            SP, claim("397", "55%", "$172.07", "1 day", "07/02/82-08/24/98")),
        live!("lwlts_mon_gap_6close", "Monday gap-down buy · close 6 days after entry", "Ch 3 · Hold to the close", "Long",
            &[MON_F, "X|\"holding for the close 6 days after entry or being stopped out\" (p.53)", STOP3000],
            SP, claim("285", "52%", "$251.23", "6 days", "07/02/82-08/24/98")),
        // ---- Chapter 4: volatility breakouts (p.57-73)
        live!("lwlts_vb_sp_tdw", "S&P volatility breakout · 50% of yesterday's range, TDW", "Ch 4 · Volatility breakouts", "Long/Short",
            &["E|Buy at the open + 50% of the previous day's range, sell at the open - 50% (\"a 50 percent volatility expansion\", p.66; \"add our breakout value to tomorrow's opening\", p.61)",
              "F|\"The best days to be a buyer were all days except Thursday and Friday, while the best sell day was Thursday, with Friday a push, but it is used\" (p.67)",
              "X|\"The exit is the same as with the bonds\" (p.67): the bailout, first profitable opening (p.61)",
              "X|Read: protective stop = 50% of the previous day's range from the entry (the bond model's stop, p.61; the book gives no S&P dollar stop)",
              "X|Ch. 11 rule 3: an opposite signal exits and reverses"],
            SP, claim("1,333", "74%", "$170.91", "1 day", "07/02/82-08/25/98")),
        live!("lwlts_vb_sp_40_200", "S&P volatility breakout · buy 40% / sell 200%", "Ch 4 · Volatility breakouts", "Long/Short",
            &["E|\"the buy entry comes at 40 percent of the previous day's range added to the open, the sell entry at 200 percent of the range subtracted from the open\" (p.71)",
              "F|Same trade days as Fig. 4.18: buys Mon-Wed, sells Thu-Fri (\"use of the preceding rules\")",
              "X|Bailout exit; Read: stop 50% of the previous day's range from the entry; opposite signal reverses"],
            SP, claim("850", "83%", "$251.25", "1 day", "07/02/82-08/25/98")),
        live!("lwlts_talon_sp", "S&P swing-value breakout (\"Talon\")", "Ch 4 · Volatility breakouts", "Long/Short",
            &["F|Swing = the larger of (high 3 days ago - today's low) and (high 1 day ago - low 3 days ago) (p.71)",
              "E|\"buy at 80 percent of the swing value above the opening and sell at 120 percent of the swing value below the opening\" (p.72)",
              "X|\"Use a dollar stop of $1,750 and my bailout exit\" (p.72): 7 points; opposite signal reverses (Ch. 11)"],
            SP, claim("538", "84%", "$228.32", "1 day", "07/02/82-08/25/98")),
        // ---- Chapter 6: trading day of week / month (p.81-92)
        live!("lwlts_mon_oc", "Monday open-to-close (Table 6.4)", "Ch 6 · TDW / TDM", "Long",
            &["E|\"purchased on the open and exited on the close each TDW\" (p.85): Mondays", "X|Exit on the close, no stop"],
            SP, claim("", "57%", "$109", "same day", "S&P since 1982 (p.86)")),
        live!("lwlts_mon_3day", "Monday buy, 3-day hold (Table 6.5)", "Ch 6 · TDW / TDM", "Long",
            &["E|\"buying on the opening and exiting 3 days later\" (p.85): Mondays", "X|Read: exit on the close of the 3rd trading day after entry, no stop"],
            SP, claim("", "57%", "$212", "3 days", "S&P since 1982 (p.86)")),
        live!("lwlts_mon_vb", "Monday open + 5% of Friday's range, exit close", "Ch 6 · TDW / TDM", "Long",
            &["E|\"buying the opening Monday +.05 percent of Friday's range\" (p.86): Read: buy stop at the open + 0.05 x the previous bar's range",
              "X|\"we hold the trade to the close\" (p.86)",
              "X|Read: $3,000 stop (12 points); not in the text, Fig. 6.2's largest loss is exactly $3,000 with $0 commission",
              "X|[intraday wave 1] Intraday wave 1: run on real 1-minute bars by default (p.path=1): the buy stop fills on the first minute that reaches the level, at the level or at that minute's open when it opened through it; out on the official daily close. Instrument: SPX only, because the levels are the book's fixed S&P points (no conversion to SPY or NSE prices is invented); from 2014, where SPX daily opens are real, to 2026-08. Days without minute data make no trade. p.path=0 = the old daily-bar reading [Not from the book]. A minute that fills and also reaches the protective stop counts as filled then stopped, unless it opened beyond the stop: then only if it closed beyond it [convention]."],
            SP, claim("758", "57%", "$125.53", "same day", "07/02/82-08/26/98")),
        live!("lwlts_tdm6_sp", "S&P buy TDM 6", "Ch 6 · TDW / TDM", "Long", TDM_R, TDM_N, claim("166", "58%", "$293", "3 days", "1982-1998")),
        live!("lwlts_tdm7_sp", "S&P buy TDM 7", "Ch 6 · TDW / TDM", "Long", TDM_R, TDM_N, claim("168", "60%", "$322", "3 days", "1982-1998")),
        live!("lwlts_tdm8_sp", "S&P buy TDM 8", "Ch 6 · TDW / TDM", "Long", TDM_R, TDM_N, claim("175", "68%", "$293", "3 days", "1982-1998")),
        live!("lwlts_tdm19_sp", "S&P buy TDM 19", "Ch 6 · TDW / TDM", "Long", TDM_R, TDM_N, claim("145", "57%", "$442", "3 days", "1982-1998")),
        live!("lwlts_tdm20_sp", "S&P buy TDM 20", "Ch 6 · TDW / TDM", "Long", TDM_R, TDM_N, claim("110", "54%", "$505", "3 days", "1982-1998")),
        live!("lwlts_tdm21_sp", "S&P buy TDM 21", "Ch 6 · TDW / TDM", "Long", TDM_R, TDM_N, claim("75", "64%", "$945", "3 days", "1982-1998")),
        live!("lwlts_tdm22_sp", "S&P buy TDM 22", "Ch 6 · TDW / TDM", "Long", TDM_R, TDM_N, claim("61", "65%", "$694", "3 days", "1982-1998")),
        // ---- Chapter 7: patterns (p.93-120)
        live!("lwlts_study_all", "Pattern study · buy every day", "Ch 7 · Patterns", "Long",
            &["E|\"buy the S&P 500 every day\" (p.94), one position at a time", STUDY_X],
            SP, claim("2,064", "52%", "$134", "2 days", "07/1982-02/1998")),
        live!("lwlts_study_down1", "Pattern study · after a down close", "Ch 7 · Patterns", "Long",
            &["F|\"only buy tomorrow if today closed down\" (p.94)", "E|Buy on the next open", STUDY_X, "X|Same $3,250 stop as the baseline (\"Now we add our first pattern\")"],
            SP, claim("1,334", "52%", "$212", "2 days", "07/1982-02/1998")),
        live!("lwlts_study_down3", "Pattern study · after three down closes", "Ch 7 · Patterns", "Long",
            &["F|\"three consecutive down closes\" (p.94)", "E|Buy on the next open", STUDY_X],
            SP, claim("248", "58%", "$353", "2 days", "07/1982-02/1998")),
        live!("lwlts_study_trend", "Pattern study · up 30 days, down 9 days", "Ch 7 · Patterns", "Long",
            &["F|\"today's price to be greater than the close 30 days ago ... today's close lower than the close 9 days ago\" (p.95)",
              "E|\"we will buy on the open tomorrow and exit on the next day's close\" (p.95)", "X|$3,250 stop as the baseline study (Read)"],
            SP, claim("354", "57%", "$421", "2 days", "07/1982-02/1998")),
        live!("lwlts_study_trend_mon", "Pattern study · up 30 / down 9, Mondays", "Ch 7 · Patterns", "Long",
            &["F|Same pattern, \"take these pattern trades on just Monday\" (p.95): the entry day is a Monday",
              "E|Buy on the open, exit on the next day's close", "X|$3,250 stop (Read)"],
            SP, claim("", "59%", "$672", "2 days", "07/1982-02/1998")),
        live!("lwlts_outside_down", "Outside day down close, lower open", "Ch 7 · Patterns", "Long",
            &["F|\"The day's high is greater than the previous day's high and the low is lower than the previous day's low and the close is below the previous day's low\" (p.95)",
              "E|\"tomorrow opens lower than the outside day's down close and we buy on the next day's opening\" (p.95)", STOP2000, BAIL],
            SP, claim("109", "85%", "$477.64", "2 days", "07/02/82-08/27/98")),
        live!("lwlts_outside_down_xthu", "Outside day down close · not Thursday", "Ch 7 · Patterns", "Long",
            &["F|Same outside-day pattern", "F|\"If we buy on any day but Thursday\" (p.97)", "E|Buy on the next day's opening if it opens below the outside day's close",
              "X|\"These results use a $2,000 stop to exit or the first profitable opening exit rule\" (p.97)"],
            SP, claim("90", "86%", "$555.97", "2 days", "07/02/82-08/27/98")),
        live!("lwlts_3up_tue_sell", "Third up close above yesterday's high on Tuesday · sell Wednesday", "Ch 7 · Patterns", "Short",
            &["F|\"any day that closes above the previous day's high and is preceded by two consecutive up closes\" (p.98), on a Tuesday",
              "E|\"setting up sells for Wednesday\" (p.99) [Not from the book: order type not stated; sell on Wednesday's open]",
              "X|\"$2,000 in the S&P 500\" stop (8 points) and \"the simple bailout exit\" (p.99)"],
            SP, claim("25", "76% (19 winners)", "$859 ($21,487 total)", "", "1986-1998")),
        live!("lwlts_oops_buy", "Oops! buy · not Wednesday or Thursday", "Ch 7 · Oops!", "Long",
            &[OOPS_B, "F|\"taking buy signals in the S&P 500 on any day of the week except Wednesday or Thursday\" (p.114)",
              "X|\"The stop was a flat $2,000 loss\" (p.114): 8 points", "X|\"we buy today and are out on the opening tomorrow\": bailout, first profitable opening (Ch. 11)"],
            SP, claim("98", "82%", "$435.59", "1 day", "09/15/87-08/28/98")),
        live!("lwlts_oops_sell_thu", "Oops! sell · Thursdays", "Ch 7 · Oops!", "Short",
            &[OOPS_S, "F|\"In the S&Ps, the best day to sell has been Thursday\" (p.117)", STOP2000, BAIL],
            SP, claim("60", "78%", "$236.67", "2 days", "09/15/87-08/28/98")),
        live!("lwlts_oops_buy_ma9", "Oops! buy · 9-day MA falling, Tue/Wed/Fri", "Ch 7 · Oops!", "Long",
            &[OOPS_B, "F|\"the best buy days, given the oversold criteria as established by the 9-day trend, are Tuesday, Wednesday, and Friday\" (p.119)",
              "F|9-day trend as defined for bonds (p.118): yesterday's 9-day moving average of closes below the day before's",
              STOP2000, "X|Read: bailout (Ch. 11: bailout \"works best for the S&P\"; the 3-day delay was for bonds)"],
            SP, claim("49", "81%", "$456.38", "1 day", "09/15/87-08/28/98")),
        live!("lwlts_oops_sell_ma9", "Oops! sell · 9-day MA rising, Wednesdays", "Ch 7 · Oops!", "Short",
            &[OOPS_S, "F|\"using the 9-day overbought technique is to take sells on Wednesday\" (p.119): yesterday's 9-day MA above the day before's (p.118)",
              STOP2000, "X|Read: bailout (Ch. 11)"],
            SP, claim("39 (text: 35)", "89%", "$486.22", "1 day", "09/15/87-08/28/98")),
        // ---- Chapter 10 (p.147-156)
        live!("lwlts_month_day1", "S&P buy the first trading day of the month", "Ch 10 · Month-end", "Long",
            &["E|\"buying the S&P 500 index on the open of the first trade day of every month\" (p.148)",
              "X|\"exit on the first profitable opening\" (p.148)",
              "X|\"The stop I chose was only $1,500, but was not used on the day of entry\" (p.148): 6 points from the day after entry",
              "F|Option p.skip_jfo=1: skip January, February and October (\"the worst months\", p.148) [non-default]"],
            SP, claim("129", "85%", "$569.28", "1 day", "09/18/87-08/31/98")),
        // ---- Chapter 15 (p.233-243)
        live!("lwlts_hh1", "Buy a new 1-day high, exit 13 days later", "Ch 15 · Buy new highs", "Long", HH_R, HH_N, claim("209", "58%", "$511", "13 days", "S&P (window not printed)")),
        live!("lwlts_hh5", "Buy a new 5-day high, exit 13 days later", "Ch 15 · Buy new highs", "Long", HH_R, HH_N, claim("187", "51%", "$359", "13 days", "S&P (window not printed)")),
        live!("lwlts_hh10", "Buy a new 10-day high, exit 13 days later", "Ch 15 · Buy new highs", "Long", HH_R, HH_N, claim("169", "50%", "$344", "13 days", "S&P (window not printed)")),
        live!("lwlts_hh15", "Buy a new 15-day high, exit 13 days later", "Ch 15 · Buy new highs", "Long", HH_R, HH_N, claim("145", "56%", "$519", "13 days", "S&P (window not printed)")),
        live!("lwlts_hh20", "Buy a new 20-day high, exit 13 days later", "Ch 15 · Buy new highs", "Long", HH_R, HH_N, claim("136", "53%", "$406", "13 days", "S&P (window not printed)")),
        // ---- Needs data
        live!("lwlts_vb_bonds", "Bond volatility breakout 100% (Fig. 4.1)", "Ch 4 · Volatility breakouts", "Long/Short", VBB_R,
            ZB_N, claim("651", "80%", "$112.86 ($0 commission)", "1 day", "01/01/90-08/25/98")),
        live!("lwlts_vb_bonds_tdw", "Bond volatility breakout 100% · best days (Fig. 4.7)", "Ch 4 · Volatility breakouts", "Long/Short",
            VBB_TDW_R,
            ZB_N, claim("326", "84%", "$173.12 ($0 commission)", "1 day", "01/01/90-08/25/98")),
        live!("lwlts_talon_bondfilter", "S&P swing breakout with bond trend (Fig. 4.21)", "Ch 4 · Volatility breakouts", "Long/Short",
            &["F|Swing = the larger of (high 3 days ago - today's low) and (high 1 day ago - low 3 days ago) (p.71)",
              "E|\"buy at 80 percent of the swing value above the opening and sell at 120 percent of the swing value below the opening\" (p.72)",
              "F|\"we will only take buy signals if the closing price of bonds is greater today than 5 days ago, and only take sells if bonds are lower than 35 days ago\" (p.72-73): bond closes on the bond's own bars, read on the day before entry",
              "X|\"Use a dollar stop of $1,750 and my bailout exit\" (p.72): 7 points, armed on the entry day (as lwlts_talon_sp); opposite signal reverses (Ch. 11)",
              "F|'One step further' (p.73) adds the best trading days, which the book does not list: not coded"],
            SPB_N, claim("295", "87%", "$281.31 ($0 commission)", "1 day", "07/02/82-08/25/98")),
        live!("lwlts_tdw_bonds", "Bonds Tuesday open + 70% of Monday's range (Fig. 6.3)", "Ch 6 · TDW / TDM", "Long",
            &["E|\"Bonds make money buying on Tuesday opening plus 70 percent of Monday's range\" (p.87): a buy stop on Tuesdays at the open + 0.7 x the previous bar's range. Unstated in the book: a Tuesday after a Monday holiday; vault reading: the previous bar's range",
              "X|\"buy that day at an expansion off the opening price. Our exit is simple; we hold the trade to the close\" (p.86): no stop (Fig. 6.3's largest loss $1,750 is a one-day move)"],
            ZB_N, claim("334", "53%", "$86.26 ($0 commission)", "same day", "12/02/77-08/26/98")),
        off!("lwlts_tdm_bonds", "Bonds best TDMs (Table 6.7)", "Ch 6 · TDW / TDM", "Long", NotTestable("Split into its printed rules, one per TDM of Table 6.7: lwlts_tdm8_bonds, lwlts_tdm18_bonds ... lwlts_tdm22_bonds (lane F1C, R4 #293), which run on the Norgate T-bond continuous contract."),
            &["E|Buy the open of TDM 8, 18, 19, 20, 21, 22", "X|$1,500 stop from entry day, close of the third day after entry (p.88-89)"], "T-Bonds 1977-1998."),
        live!("lwlts_tdm8_bonds", "T-Bond buy TDM 8", "Ch 6 · TDW / TDM", "Long", TDMB_R, TDMB_N, claim("230", "55%", "$166", "3 days", "1977-1998")),
        live!("lwlts_tdm18_bonds", "T-Bond buy TDM 18", "Ch 6 · TDW / TDM", "Long", TDMB_R, TDMB_N, claim("231", "57%", "$201", "3 days", "1977-1998")),
        live!("lwlts_tdm19_bonds", "T-Bond buy TDM 19", "Ch 6 · TDW / TDM", "Long", TDMB_R, TDMB_N, claim("195", "59%", "$223", "3 days", "1977-1998")),
        live!("lwlts_tdm20_bonds", "T-Bond buy TDM 20", "Ch 6 · TDW / TDM", "Long", TDMB_R, TDMB_N, claim("148", "56%", "$292", "3 days", "1977-1998")),
        live!("lwlts_tdm21_bonds", "T-Bond buy TDM 21", "Ch 6 · TDW / TDM", "Long", TDMB_R, TDMB_N, claim("106", "55%", "$297", "3 days", "1977-1998")),
        live!("lwlts_tdm22_bonds", "T-Bond buy TDM 22", "Ch 6 · TDW / TDM", "Long", TDMB_R, TDMB_N, claim("76", "64%", "$285", "3 days", "1977-1998")),
        off!("lwlts_bond_patterns", "Bond outside-day, 3-up-day and Oops! patterns (Fig. 7.4, 7.5, 7.29, 7.30, 7.32, 7.33)", "Ch 7 · Patterns", "Long/Short", NotTestable("Split into its printed rules (lane F1D, R4 #293), which run on the Norgate T-bond continuous contract: lwlts_outside_down_bonds, lwlts_outside_down_bonds_xthu, lwlts_oops_buy_bonds, lwlts_oops_sell_bonds_wed, lwlts_oops_buy_bonds_ma9, lwlts_oops_sell_bonds_ma9., lwlts_3up_thu_sell_bonds."),
            &["E|Outside day down close + lower open, $1,500 stop, bailout (p.97)", "E|Oops! buys except Wednesday, $1,800 stop, bailout; Oops! sells Wednesday, $1,000 stop, 4-day bailout (p.116)", "E|Oops! with 9-day MA; exit \"the close on the first profitable opening after 3 days\" (p.118)"], "Bond futures results only."),
        live!("lwlts_gsv_bonds", "Greatest Swing Value · bonds (Fig. 8.2)", "Ch 8 · Greatest swing value", "Long/Short", GSVB_R,
            ZB_N, claim("161", "75%", "$328.03 ($0 commission)", "", "01/01/90-08/28/98")),
        live!("lwlts_outside_down_bonds", "Bonds outside day down close, lower open (Fig. 7.4)", "Ch 7 · Patterns", "Long", OUTB_R,
            ZB_N, claim("57", "82%", "$212.54 ($55 commission)", "", "06/10/90-08/27/98")),
        live!("lwlts_outside_down_bonds_xthu", "Bonds outside day down close · not Thursday (Fig. 7.5)", "Ch 7 · Patterns", "Long",
            OUTB_X_R,
            ZB_N, claim("41", "90%", "$420.61 ($55 commission)", "", "06/10/90-08/27/98")),
        live!("lwlts_3up_thu_sell_bonds", "Bonds third up close above yesterday's high on Thursday · sell Friday", "Ch 7 · Patterns", "Short",
            &["F|\"any day that closes above the previous day's high and is preceded by two consecutive up closes\" (p.98); \"In the Bond market, the same pattern set up 28 trades on Thursday, to sell on Friday\" (p.99)",
              "E|Sell on Friday [Unstated in the book: the order type; vault reading: Friday's open, as lwlts_3up_tue_sell]",
              "X|\"A $1,500 stop was used in Bonds\" (p.99): 1.5 points, armed on the entry day; \"we used the simple bailout exit\" (p.99)"],
            ZB_N, claim("28", "89%", "$475 ($13,303 total)", "", "1989-08/1998")),
        live!("lwlts_oops_buy_bonds", "Bonds Oops! buy · not Wednesday (Fig. 7.29)", "Ch 7 · Oops!", "Long",
            &[OOPS_B, "F|\"Here we will take long trades any day of the week except Wednesday\" (p.116)",
              "X|\"a stop-loss of $1,800 from the point of entry\" (p.116): 1.8 points, armed on the entry day", "X|\"Our exit is the bailout technique\" (p.116): first profitable opening"],
            OOPSB_N, claim("138", "86%", "$201.99 (after $50 commission)", "", "01/01/90-08/28/98")),
        live!("lwlts_oops_sell_bonds_wed", "Bonds Oops! sell · Wednesdays (Fig. 7.30)", "Ch 7 · Oops!", "Short",
            &[OOPS_S, "F|\"the rules are to sell on Wednesday if our Oops! opening gap and failure occur\" (p.116)",
              "X|\"using a closer $1,000 stop\" (p.116): 1 point, armed on the entry day",
              "X|\"and 4-day bailout exit\" (p.116-117). Unstated in the book: the count; vault reading: the first profitable opening from the 4th bar after the entry bar"],
            OOPSB_N, claim("55", "56% (31 winners)", "$179.55", "", "01/01/90-08/28/98")),
        live!("lwlts_oops_buy_bonds_ma9", "Bonds Oops! buy · 9-day MA falling, not Thursday (Fig. 7.32)", "Ch 7 · Oops!", "Long",
            &[OOPS_B, "F|\"in the Bonds on any day but Thursday if Friday's 9-day moving average is less than Thursday's\" (p.118): the entry day is not a Thursday and yesterday's 9-day average of closes is below the day before's",
              "X|\"The exit is the close on the first profitable opening after 3 days in the trade\" (p.118). Unstated in the book: the count and the fill; vault reading: the first profitable opening from the 3rd bar after the entry bar",
              "X|Stop: \"using the same rules as above for stop\" (p.118): the bond Oops! buy's $1,800 (p.116) = 1.8 points, armed on the entry day. Unstated in the book which rule 'above' means; vault reading: $1,800"],
            OOPSB_N, claim("66", "81%", "$373.11", "", "01/01/90-08/28/98")),
        live!("lwlts_oops_sell_bonds_ma9", "Bonds Oops! sell · 9-day MA rising, Wednesdays (Fig. 7.33)", "Ch 7 · Oops!", "Short",
            &[OOPS_S, "F|\"taking Oops! sell signals on Wednesday if the 9-day average is greater on Tuesday than Monday\" (p.118)",
              "X|\"using the same rules as above for stop and exit as on the long trade\" (p.118): $1,800 = 1.8 points (reading, as the buy) and the first profitable opening from the 3rd bar after the entry bar (reading)"],
            OOPSB_N, claim("34", "79%", "$394.30", "", "01/01/90-08/28/98")),
        live!("lwlts_gsv_sp", "Greatest Swing Value · S&P with bond filter (Fig. 8.3)", "Ch 8 · Greatest swing value", "Long/Short",
            &["E|\"we will take 180 percent of the 4-day average buy swing value (the highs minus the opens) and, for sell, the 4-day average swing sell value (closes minus the lows)\" (p.125): buy stop at the open + 1.8 x the 4-day average (high - open), sell stop at the open - 1.8 x the 4-day average (close - low), as printed (Fig. 8.3's caption reads '.80'; option p.mult)",
              "F|\"demanding Bonds close higher than 15 days ago for a buy and lower than 15 days ago for a sell\" (p.125): the bond's own bars, on the day before entry",
              "F|\"buy on Monday, Tuesday, or Wednesday. Shorts will be taken any day but Monday. The setup also consists of a close lower than 6 days ago for a buy, higher than six days ago for a sell\" (p.125)",
              "X|\"a flat dollar stop of $2,500 and the bailout exit\" (p.125): 10 points from the day after entry (\"my computer software does not allow us to bring into play a protective stop on the day of entry\", p.125)",
              "X|Unstated in the book: an opposite signal while in a trade; vault reading: it exits and reverses (Ch. 11 rule 3)"],
            SPB_N, claim("247", "67%", "$427.83", "", "S&P (window not legible)")),
        live!("lwlts_gsv_sp_monday", "GSV Monday buy after a down Friday (Fig. 8.4)", "Ch 8 · Greatest swing value", "Long",
            &["F|\"wait for a down close in the S&P 500 on Friday ... I back this with Bonds closing on Friday greater than they did 15 days ago\" (p.127): Friday's close below Thursday's; the bond's own bars",
              "E|\"I then buy Monday at the open plus Friday's high minus Friday's open swing value\" (p.127): a buy stop; only when the bar after the Friday is a Monday",
              "X|\"simply using the bailout exit and a $2,500 stop\" (p.127): 10 points from the day after entry (p.125)"],
            SPB_N, claim("161", "86%", "$354.58 ($0 commission)", "", "09/15/87-08/28/98")),
        live!("lwlts_month_day1_sp_bonds", "S&P first trading day of the month · bond uptrend (Ch. 10)", "Ch 10 · Month-end", "Long",
            &["E|\"buying the S&P 500 index on the open of the first trade day of every month\" (p.148)",
              "F|\"only buy on the first of a month, any month-if Bonds have closed higher the day prior to our anticipated entry than 30 days ago\" (p.149): the bond's own bars",
              "X|\"exit on the first profitable opening\"; \"The stop I chose was only $1,500, but was not used on the day of entry\" (p.148): 6 points from the day after entry"],
            SPB_N, claim("", "", "", "1 day", "no figure printed")),
        live!("lwlts_bonds_channel_bstop", "S&P buys on bond channel breakouts · bond channel exit (Fig. 15.1)", "Ch 15 · Data A and data B", "Long",
            &[BCH_E, "X|\"either a trailing stop of the lowest low of the last 17 days-in Bonds-or a 3,000 stop from the point of entry\" (p.236): $3,000 = 12 points; the bond exit when the bond's low falls below the lowest bond low of the 17 bars before it. Unstated in the book: the S&P fill of that bond stop; vault reading: the S&P close of that day"],
            SPB_N, claim("81", "53%", "$1,750.52 (after $45 commission)", "", "08/09/82-03/01/98")),
        live!("lwlts_bonds_channel_sstop", "S&P buys on bond channel breakouts · S&P 12-day low exit (Fig. 15.2)", "Ch 15 · Data A and data B", "Long",
            &[BCH_E, "X|\"exiting at the lowest low of the last 12 days in the S&P\" (p.237): a sell stop at the lowest S&P low of the 12 bars before each day; with the $3,000 stop of Fig. 15.1 (12 points; reading, the figure is the same test with the S&P exit)"],
            SPB_N, claim("106", "45%", "$1,163.73 (after $45 commission)", "", "08/09/82-03/01/98")),
        off!("lwlts_willspread", "Will-Spread (Ch. 9)", "Ch 9 · Quote screen", "Long/Short", NeedsData("Needs 15/30-minute bars of the traded market and of T-Bonds/T-Bills (or Gold for bonds); intraday and bond data are not in the vault."),
            &["F|Spread = primary / secondary x 100; 5-period EMA minus 20-period EMA (S&P: 3 and 15) (p.138-141)", "E|Zero-line crossing, then the next bar takes out the crossing bar's high (low)"], "Intraday."),
        live!("lwlts_month_day1_bonds", "Bonds buy the first trading day of the month (Fig. 10.2)", "Ch 10 · Month-end", "Long",
            &["E|\"buying the first trading day of every month in the Bonds, as we did in the S&P 500\" (p.150): the open of TDM 1",
              "X|\"using an $1,100 stop and exiting on the first profitable opening\" (p.150): 1.1 points. Unstated in the book: the stop on the entry day; vault reading: armed on the entry day (the bonds' TDM convention, p.89)",
              "F|Option p.skip=1 (not the base rule): \"by-passing the poorer performing months which, as shown in Table 10.2, are January, February, April, and October\" (p.150)"],
            ZB_N, claim("149", "69%", "$218.75", "1 day", "01/01/86-08/28/98")),
        live!("lwlts_tdm18_gold_bonds", "T-Bond buy TDM 18 · gold downtrend (Fig. 10.6)", "Ch 10 · Month-end", "Long", TDMG_R, ZB_N, claim("90 (derived: $32,062.50 / $356.25; count not legible)", "75%", "$356.25", "3 days", "01/01/86-08/28/98")),
        live!("lwlts_tdm22_gold_bonds", "T-Bond buy TDM 22 · gold downtrend (Fig. 10.8)", "Ch 10 · Month-end", "Long", TDMG_R, ZB_N, claim("28", "89%", "$719.87", "3 days", "01/01/86-08/28/98")),
        live!("lwlts_tdm12_sell_bonds", "T-Bond sell TDM 12 (Fig. 10.9)", "Ch 10 · Month-end", "Short", TDM12_R, ZB_N, claim("152", "76%", "$133.43", "3 days", "01/01/86-08/28/98")),
        live!("lwlts_tdm12_sell_gold_bonds", "T-Bond sell TDM 12 · gold uptrend (Fig. 10.10)", "Ch 10 · Month-end", "Short", TDM12G_R, ZB_N, claim("73", "78%", "$359.59", "3 days", "01/01/86-08/28/98")),
        off!("lwlts_month_bonds", "Month-end bond trades and S&P first day with bond trend (Ch. 10)", "Ch 10 · Month-end", "Long/Short", NotTestable("Split into its printed rules (lane F1D, R4 #293): lwlts_month_day1_bonds, lwlts_tdm18_gold_bonds, lwlts_tdm22_gold_bonds, lwlts_tdm12_sell_bonds, lwlts_tdm12_sell_gold_bonds on the Norgate T-bond contract; the S&P first-day rule with the bond trend is lwlts_month_day1_sp_bonds. TDM 18 and TDM 22 without gold (Fig. 10.5, 10.7) are lwlts_tdm18_bonds / lwlts_tdm22_bonds."),
            &["F|S&P first trading day only if Bonds closed higher than 30 days ago (p.149)", "E|Bonds: first TDM ($1,100 stop, bailout); TDM 18 and 22 ($1,500 stop, close 3 days after entry, Gold < 24 days ago); sell TDM 12 ($1,400, Gold > 10 days ago) (p.149-156)"], "Bond and gold futures."),
        off!("lwlts_bonds_channel", "S&P buys on bond channel breakouts (Fig. 15.1, 15.2)", "Ch 15 · Data A and data B", "Long", NotTestable("Split into its printed rules (lane F1D, R4 #293): lwlts_bonds_channel_bstop (Fig. 15.1) and lwlts_bonds_channel_sstop (Fig. 15.2), on the Norgate S&P 500 contract with the T-bond contract as Data2. Fig. 15.2A (very short-term bond breakouts with the 1/1 bailout) prints no breakout length: not coded."),
            &["E|Buy the S&P market on close when Bonds close above the highest high of the last 14 days (p.235)", "X|Lowest low of the last 17 days in Bonds, or a $3,000 stop; Fig. 15.2: lowest low of the last 12 days in the S&P"], "$141,792.50, avg $1,750."),
        live!("lwlts_bonds_8day", "Bond system: close above 8 days ago (Ch. 14, Aug 1997 letter)", "Ch 14 · Letters", "Long", B8_R, B8_N,
            claim("about 228 ($36,500 / $160)", "", "$160 (after $50 commission and slippage)", "", "1977-08/1997")),
        off!("lwlts_ma_cross_futures", "Moving-average crossover studies (Ch. 2)", "Ch 2 · Price and time", "Long/Short", NeedsData("Soybean and British Pound futures data"),
            &["E|Best-fit crossovers (5/25, 25/30 soybeans; 5/45, 20/40 pound), shown only to prove that time-based optimisation fails (p.24-27)"], "An illustration, not a recommended system."),
        // ---- Not testable
        off!("lwlts_smash_day", "Smash day and hidden smash day reversals", "Ch 7 · Patterns", "Long/Short", NotTestable("The setup and entry are exact, but the book gives no stop, exit or results, and says to take them only when 'the stage has been set' by TDW, TDM, market relationships, overbought/oversold and trend (p.104), which is judgement."),
            &["F|Smash buy: close below the previous day's low; hidden: up close in the lower 25% of the range, best below the open (p.101-103)", "E|Next day trades above the smash day's high (sell: below the low)"], "No exit rule."),
        off!("lwlts_specialist_trap", "Specialists' trap", "Ch 7 · Patterns", "Long/Short", NotTestable("'a nice uptrending market that moves sideways in a box or congestion for 5 to 10 days' (p.109) is chart judgement; no stop, exit or results."),
            &["F|Box of 5-10 days, breakout with a naked close beyond the range", "E|Reversal through the breakout day's true low (high) within 1-3 days"], ""),
        off!("lwlts_oops_tdm18", "Oops! after the 17th trading day of the month", "Ch 7 · Oops!", "Long", NotTestable("Stop and exit are not stated ('a reasonable stop and exit', p.120); the Figure 7.35 caption sits on the Wednesday 9-day sell numbers."),
            &["F|Oops! buy when it occurs \"at the end of the month, and trading day after the 17th trading day of the month\" (p.119)"], ""),
        off!("lwlts_swing_points", "Swing-point trend changes and the three-bar high/low system", "Ch 1 / Ch 9 · Quote screen", "Long/Short", NotTestable("Outside days need the intraday open-to-close path ('we will have to study the flow of prices during that day', p.17); the three-bar system is on 5-15 minute bars and passes trades by judgement (p.136-137)."),
            &["F|Short-term high: a high with lower highs on both sides, inside days ignored (p.16-17)", "E|3-bar MA of lows (highs) as entry, 3-bar MA of highs (lows) as target, in the swing trend"], ""),
        off!("lwlts_eom_sp", "S&P end-of-month: third-to-last trading day, 6 sessions", "Ch 10 · Month-end", "Long", NotTestable("Only an equity chart (Fig. 10.4) with no trade figures. Its legend reads \"Buy on open @ 4 trading days starting at 1st day of each month. Exit on close\", which can mean one 4-day hold from the first trading day or four separate one-day trades; the book does not say which, and the bond text rule (third-to-last day, six sessions, $1,500) differs from the Fig. 10.3 legend (fifth-to-last day open to the close of the third day of the next month, no stop)."),
            &["E|Bonds: \"bought one contract ... on the third to the last trading day of each month and held for six trading sessions ... or taking a loss of $1,500\" (p.152)"], ""),
        off!("lwlts_close_65pct", "Buy on the close in the upper 65% of the range (Fig. 15.3)", "Ch 15 · Bad habits", "Long", NotTestable("'With a stop for protection' (p.238) but the stop size and test window are not printed, and the trade counts in the table are inconsistent (533, 334, 537, 199, 178)."),
            &["E|Buy market on close if the close is in the upper 65% of the day's range; exit 5, 10, 15, 20, 25 days later"], ""),
        off!("lwlts_sp_daytrade_stops", "S&P day-trading system with $500/$1,500/$6,000 stops (Fig. 15.5-15.7)", "Ch 15 · Stops", "Long/Short", NotTestable("The entry and exit rules of this system are not given (p.241)."), &["X|Only the stop differs between the three printouts"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |a, b, c| Some((a, b, c));
    match id {
        "lwlts_mon_gap_t500" => f("lwlts_mon_gap", "Monday gap-down buy", "$500 target"),
        "lwlts_mon_gap_t1000" => f("lwlts_mon_gap", "Monday gap-down buy", "$1,000 target"),
        "lwlts_mon_gap_close" => f("lwlts_mon_gap", "Monday gap-down buy", "Exit on the close"),
        "lwlts_mon_gap_next_close" => f("lwlts_mon_gap", "Monday gap-down buy", "Exit next close"),
        "lwlts_mon_gap_6close" => f("lwlts_mon_gap", "Monday gap-down buy", "Close 6 days after entry"),
        "lwlts_vb_sp_tdw" => f("lwlts_vb", "S&P volatility breakout", "50%, TDW"),
        "lwlts_vb_sp_40_200" => f("lwlts_vb", "S&P volatility breakout", "Buy 40% / sell 200%"),
        "lwlts_talon_sp" => f("lwlts_vb", "S&P volatility breakout", "Swing value (Talon)"),
        "lwlts_vb_bonds" => f("lwlts_vb", "S&P volatility breakout", "Bonds 100%"),
        "lwlts_vb_bonds_tdw" => f("lwlts_vb", "S&P volatility breakout", "Bonds 100%, best days"),
        "lwlts_talon_bondfilter" => f("lwlts_vb", "S&P volatility breakout", "Swing value + bond trend"),
        "lwlts_mon_oc" => f("lwlts_tdw", "Trading day of week", "Monday open-to-close"),
        "lwlts_mon_3day" => f("lwlts_tdw", "Trading day of week", "Monday 3-day hold"),
        "lwlts_mon_vb" => f("lwlts_tdw", "Trading day of week", "Monday open + 5% range"),
        "lwlts_tdw_bonds" => f("lwlts_tdw", "Trading day of week", "Bonds Tuesday"),
        "lwlts_tdm6_sp" => f("lwlts_tdm", "Trading day of month", "TDM 6"),
        "lwlts_tdm7_sp" => f("lwlts_tdm", "Trading day of month", "TDM 7"),
        "lwlts_tdm8_sp" => f("lwlts_tdm", "Trading day of month", "TDM 8"),
        "lwlts_tdm19_sp" => f("lwlts_tdm", "Trading day of month", "TDM 19"),
        "lwlts_tdm20_sp" => f("lwlts_tdm", "Trading day of month", "TDM 20"),
        "lwlts_tdm21_sp" => f("lwlts_tdm", "Trading day of month", "TDM 21"),
        "lwlts_tdm22_sp" => f("lwlts_tdm", "Trading day of month", "TDM 22"),
        "lwlts_tdm_bonds" => f("lwlts_tdm", "Trading day of month", "Bonds (split)"),
        "lwlts_tdm8_bonds" => f("lwlts_tdm", "Trading day of month", "Bonds TDM 8"),
        "lwlts_tdm18_bonds" => f("lwlts_tdm", "Trading day of month", "Bonds TDM 18"),
        "lwlts_tdm19_bonds" => f("lwlts_tdm", "Trading day of month", "Bonds TDM 19"),
        "lwlts_tdm20_bonds" => f("lwlts_tdm", "Trading day of month", "Bonds TDM 20"),
        "lwlts_tdm21_bonds" => f("lwlts_tdm", "Trading day of month", "Bonds TDM 21"),
        "lwlts_tdm22_bonds" => f("lwlts_tdm", "Trading day of month", "Bonds TDM 22"),
        "lwlts_study_all" => f("lwlts_study", "Pattern studies", "Every day"),
        "lwlts_study_down1" => f("lwlts_study", "Pattern studies", "After a down close"),
        "lwlts_study_down3" => f("lwlts_study", "Pattern studies", "After 3 down closes"),
        "lwlts_study_trend" => f("lwlts_study", "Pattern studies", "Up 30 / down 9"),
        "lwlts_study_trend_mon" => f("lwlts_study", "Pattern studies", "Up 30 / down 9, Mondays"),
        "lwlts_outside_down" => f("lwlts_outside", "Outside day down close", "All days"),
        "lwlts_outside_down_xthu" => f("lwlts_outside", "Outside day down close", "Not Thursday"),
        "lwlts_3up_tue_sell" => f("lwlts_3up", "Third up day sell", "Tuesday setup"),
        "lwlts_3up_thu_sell_bonds" => f("lwlts_3up", "Third up day sell", "Bonds, Thursday setup"),
        "lwlts_oops_buy" => f("lwlts_oops", "Oops!", "Buy, not Wed/Thu"),
        "lwlts_oops_sell_thu" => f("lwlts_oops", "Oops!", "Sell Thursday"),
        "lwlts_oops_buy_ma9" => f("lwlts_oops", "Oops!", "Buy, 9-day MA"),
        "lwlts_oops_sell_ma9" => f("lwlts_oops", "Oops!", "Sell, 9-day MA"),
        "lwlts_oops_tdm18" => f("lwlts_oops", "Oops!", "After TDM 17"),
        "lwlts_bond_patterns" => f("lwlts_oops", "Oops!", "Bonds (split)"),
        "lwlts_oops_buy_bonds" => f("lwlts_oops", "Oops!", "Bonds buy, not Wed"),
        "lwlts_oops_sell_bonds_wed" => f("lwlts_oops", "Oops!", "Bonds sell Wed"),
        "lwlts_oops_buy_bonds_ma9" => f("lwlts_oops", "Oops!", "Bonds buy, 9-day MA"),
        "lwlts_oops_sell_bonds_ma9" => f("lwlts_oops", "Oops!", "Bonds sell, 9-day MA"),
        "lwlts_outside_down_bonds" => f("lwlts_outside", "Outside day down close", "Bonds"),
        "lwlts_outside_down_bonds_xthu" => f("lwlts_outside", "Outside day down close", "Bonds, not Thursday"),
        "lwlts_month_day1" => f("lwlts_month", "Month-end trades", "S&P first trading day"),
        "lwlts_month_bonds" | "lwlts_eom_sp" => f("lwlts_month", "Month-end trades", "Bonds / end of month"),
        "lwlts_month_day1_bonds" => f("lwlts_month", "Month-end trades", "Bonds first trading day"),
        "lwlts_tdm18_gold_bonds" => f("lwlts_tdm", "Trading day of month", "Bonds TDM 18, gold"),
        "lwlts_tdm22_gold_bonds" => f("lwlts_tdm", "Trading day of month", "Bonds TDM 22, gold"),
        "lwlts_tdm12_sell_bonds" => f("lwlts_tdm", "Trading day of month", "Bonds sell TDM 12"),
        "lwlts_tdm12_sell_gold_bonds" => f("lwlts_tdm", "Trading day of month", "Bonds sell TDM 12, gold"),
        "lwlts_hh1" => f("lwlts_hh", "Buy new X-day highs", "X = 1"),
        "lwlts_hh5" => f("lwlts_hh", "Buy new X-day highs", "X = 5"),
        "lwlts_hh10" => f("lwlts_hh", "Buy new X-day highs", "X = 10"),
        "lwlts_hh15" => f("lwlts_hh", "Buy new X-day highs", "X = 15"),
        "lwlts_hh20" => f("lwlts_hh", "Buy new X-day highs", "X = 20"),
        "lwlts_gsv_bonds" => f("lwlts_gsv", "Greatest swing value", "Bonds"),
        "lwlts_gsv_sp" => f("lwlts_gsv", "Greatest swing value", "S&P + bonds"),
        "lwlts_gsv_sp_monday" => f("lwlts_gsv", "Greatest swing value", "S&P Monday + bonds"),
        "lwlts_month_day1_sp_bonds" => f("lwlts_month", "Month-end trades", "S&P first day, bond uptrend"),
        "lwlts_bonds_channel_bstop" => f("lwlts_bch", "S&P buys on bond channel breakouts", "Bond 17-day low exit"),
        "lwlts_bonds_channel_sstop" => f("lwlts_bch", "S&P buys on bond channel breakouts", "S&P 12-day low exit"),
        "lwlts_willspread" => f("lwlts_willspread", "Will-Spread", ""),
        "lwlts_bonds_channel" | "lwlts_ma_cross_futures" => f("lwlts_other", "Other futures studies", "Needs futures data"),
        "lwlts_bonds_8day" => f("lwlts_b8", "Bonds: close above 8 days ago", "$850 stop / $3,500 target"),
        "lwlts_smash_day" | "lwlts_specialist_trap" | "lwlts_swing_points" => f("lwlts_patterns", "Chart patterns", "Not testable"),
        "lwlts_close_65pct" | "lwlts_sp_daytrade_stops" => f("lwlts_other", "Other studies", "Not testable"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let mut v = params_base(id)?;
    crate::intraday_ep6::add_param(id, &mut v); // EP6 (#243)
    Some(v)
}

fn params_base(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let pt = || p("pt_usd", "Dollars per index point (book: S&P $250)", 250.0, 50.0, 500.0, 50.0);
    let stop = |d: f64| p("stop_usd", "Dollar stop (book $)", d, 0.0, 6000.0, 250.0);
    let day0 = |d: f64| p("stop_day0", "Stop armed on the entry day (1) or from the next day (0)", d, 0.0, 1.0, 1.0);
    let dows = |k, l, d| p(k, l, d, 0.0, 31.0, 1.0);
    let zb = || p("pt_usd", "Dollars per point (T-bond contract: $1,000)", 1000.0, 100.0, 5000.0, 100.0);
    Some(match id {
        "lwlts_mon_gap_t500" => vec![p("target_usd", "Profit target (book $)", 500.0, 250.0, 2000.0, 250.0), stop(3000.0), day0(1.0), pt()],
        "lwlts_mon_gap_t1000" => vec![p("target_usd", "Profit target (book $)", 1000.0, 250.0, 2000.0, 250.0), stop(3000.0), day0(1.0), pt()],
        "lwlts_mon_gap_close" => vec![stop(3000.0), day0(1.0), pt()],
        "lwlts_mon_gap_next_close" => vec![p("hold", "Exit on the close N days after entry", 1.0, 0.0, 10.0, 1.0), stop(3000.0), day0(1.0), pt()],
        "lwlts_mon_gap_6close" => vec![p("hold", "Exit on the close N days after entry", 6.0, 0.0, 10.0, 1.0), stop(3000.0), day0(1.0), pt()],
        "lwlts_vb_sp_tdw" | "lwlts_vb_sp_40_200" => {
            let (b, s) = if id == "lwlts_vb_sp_tdw" { (0.5, 0.5) } else { (0.4, 2.0) };
            vec![p("buy_mult", "Buy: open + x previous range", b, 0.1, 2.0, 0.1), p("sell_mult", "Sell: open - x previous range", s, 0.1, 3.0, 0.1),
                 dows("buy_dow", "Buy days bitmask (Mon=1 Tue=2 Wed=4 Thu=8 Fri=16)", 7.0), dows("sell_dow", "Sell days bitmask", 24.0),
                 p("stop_rng", "Stop: x previous range from entry (Read)", 0.5, 0.0, 2.0, 0.1), day0(1.0)]
        }
        "lwlts_talon_sp" => vec![p("buy_mult", "Buy: open + x swing", 0.8, 0.2, 2.0, 0.1), p("sell_mult", "Sell: open - x swing", 1.2, 0.2, 2.0, 0.1),
            dows("buy_dow", "Buy days bitmask", 31.0), dows("sell_dow", "Sell days bitmask", 31.0), stop(1750.0), day0(1.0), pt()],
        "lwlts_mon_oc" => vec![p("hold", "Exit on the close N days after entry", 0.0, 0.0, 10.0, 1.0), stop(0.0), day0(1.0), pt()],
        "lwlts_mon_3day" => vec![p("hold", "Exit on the close N days after entry", 3.0, 0.0, 10.0, 1.0), stop(0.0), day0(1.0), pt()],
        "lwlts_mon_vb" => vec![p("buy_mult", "Buy: open + x Friday's range", 0.05, 0.0, 1.0, 0.05), stop(3000.0), day0(1.0), pt(),
            // intraday wave 1
            p("path", "1 = real 1-minute bars (fills at the level or the minute's open; days without minute data make no trade), 0 = [Not from the book] daily bar path", 1.0, 0.0, 1.0, 1.0)],
        "lwlts_tdm8_bonds" | "lwlts_tdm18_bonds" | "lwlts_tdm19_bonds" | "lwlts_tdm20_bonds" | "lwlts_tdm21_bonds" | "lwlts_tdm22_bonds" =>
            vec![p("hold", "Exit on the close N days after entry", 3.0, 1.0, 10.0, 1.0), stop(1500.0), day0(1.0), p("pt_usd", "Dollars per point (T-bond contract: $1,000)", 1000.0, 100.0, 5000.0, 100.0)],
        "lwlts_vb_bonds" | "lwlts_vb_bonds_tdw" => {
            let (b, s) = if id == "lwlts_vb_bonds" { (31.0, 31.0) } else { (10.0, 12.0) };
            vec![p("buy_mult", "Buy: open + x previous range", 1.0, 0.1, 3.0, 0.1), p("sell_mult", "Sell: open - x previous range", 1.0, 0.1, 3.0, 0.1),
                 dows("buy_dow", "Buy days bitmask (Mon=1 Tue=2 Wed=4 Thu=8 Fri=16; 31 = every day)", b), dows("sell_dow", "Sell days bitmask", s),
                 stop(1500.0), p("stop_rng", "Stop: x previous range from entry (the nearer of this and the $ stop)", 0.5, 0.0, 2.0, 0.1), day0(1.0), zb()]
        }
        "lwlts_tdw_bonds" => vec![p("buy_mult", "Buy: Tuesday open + x previous range", 0.7, 0.0, 2.0, 0.05), zb()],
        "lwlts_outside_down_bonds" | "lwlts_outside_down_bonds_xthu" | "lwlts_3up_thu_sell_bonds" => vec![stop(1500.0), day0(1.0), zb()],
        "lwlts_oops_buy_bonds" => vec![stop(1800.0), day0(1.0), zb()],
        "lwlts_oops_sell_bonds_wed" => vec![stop(1000.0), day0(1.0), p("bail_after", "Bailout from the opening N bars after entry", 4.0, 1.0, 10.0, 1.0), zb()],
        "lwlts_oops_buy_bonds_ma9" | "lwlts_oops_sell_bonds_ma9" => vec![stop(1800.0), day0(1.0), p("bail_after", "Bailout from the opening N bars after entry", 3.0, 1.0, 10.0, 1.0), zb()],
        "lwlts_gsv_bonds" => vec![p("mult", "Entry: open +/- x the 4-day average swing", 1.8, 0.2, 4.0, 0.1), stop(1600.0), day0(0.0),
            p("bail_after", "Bailout from the opening N bars after entry", 2.0, 1.0, 10.0, 1.0), p("gold", "Sell only if gold closed below its close 20 bars earlier (option, p.125)", 0.0, 0.0, 1.0, 1.0), zb()],
        "lwlts_month_day1_bonds" => vec![stop(1100.0), day0(1.0), p("skip", "Skip January, February, April, October (option, p.150)", 0.0, 0.0, 1.0, 1.0), zb()],
        "lwlts_tdm18_gold_bonds" | "lwlts_tdm22_gold_bonds" => vec![p("hold", "Exit on the close N days after entry", 3.0, 1.0, 10.0, 1.0), stop(1500.0), day0(1.0),
            p("gold_n", "Gold close below its close N gold bars earlier", 24.0, 1.0, 100.0, 1.0), zb()],
        "lwlts_tdm12_sell_bonds" => vec![p("hold", "Exit on the close N days after entry", 3.0, 1.0, 10.0, 1.0), stop(1400.0), day0(1.0), zb()],
        "lwlts_tdm12_sell_gold_bonds" => vec![p("hold", "Exit on the close N days after entry", 3.0, 1.0, 10.0, 1.0), stop(1400.0), day0(1.0),
            p("gold_n", "Gold close above its close N gold bars earlier", 10.0, 1.0, 100.0, 1.0), zb()],
        "lwlts_talon_bondfilter" => vec![p("buy_mult", "Buy: open + x swing", 0.8, 0.2, 2.0, 0.1), p("sell_mult", "Sell: open - x swing", 1.2, 0.2, 2.0, 0.1), stop(1750.0), day0(1.0), pt()],
        "lwlts_gsv_sp" => vec![p("mult", "Entry: open +/- x the 4-day average swing (text 1.8; Fig. 8.3 caption .80)", 1.8, 0.2, 4.0, 0.1), stop(2500.0), day0(0.0), pt()],
        "lwlts_gsv_sp_monday" => vec![stop(2500.0), day0(0.0), pt()],
        "lwlts_month_day1_sp_bonds" => vec![stop(1500.0), day0(0.0), pt()],
        "lwlts_bonds_channel_bstop" | "lwlts_bonds_channel_sstop" => vec![stop(3000.0), pt()],
        "lwlts_bonds_8day" => vec![p("lookback", "Close above the close N bars ago", 8.0, 1.0, 50.0, 1.0), stop(850.0),
            p("target_usd", "Profit target (book $)", 3500.0, 0.0, 10000.0, 250.0), p("pt_usd", "Dollars per point (T-bond contract: $1,000)", 1000.0, 100.0, 5000.0, 100.0)],
        "lwlts_tdm6_sp" | "lwlts_tdm7_sp" | "lwlts_tdm8_sp" | "lwlts_tdm19_sp" | "lwlts_tdm20_sp" | "lwlts_tdm21_sp" | "lwlts_tdm22_sp" =>
            vec![p("hold", "Exit on the close N days after entry", 3.0, 1.0, 10.0, 1.0), stop(2000.0), day0(0.0), pt()],
        "lwlts_study_all" | "lwlts_study_down1" | "lwlts_study_down3" | "lwlts_study_trend" | "lwlts_study_trend_mon" =>
            vec![p("hold", "Exit on the close N days after entry", 1.0, 0.0, 10.0, 1.0), stop(3250.0), day0(1.0), pt()],
        "lwlts_outside_down" | "lwlts_outside_down_xthu" | "lwlts_3up_tue_sell" | "lwlts_oops_buy" | "lwlts_oops_sell_thu" | "lwlts_oops_buy_ma9" | "lwlts_oops_sell_ma9" =>
            vec![stop(2000.0), day0(1.0), pt()],
        "lwlts_month_day1" => vec![stop(1500.0), day0(0.0), p("skip_jfo", "Skip January, February, October (non-default)", 0.0, 0.0, 1.0, 1.0), pt()],
        "lwlts_hh1" | "lwlts_hh5" | "lwlts_hh10" | "lwlts_hh15" | "lwlts_hh20" =>
            vec![p("hold", "Exit on the close N days after entry", 13.0, 1.0, 30.0, 1.0), stop(3500.0), day0(1.0), pt()],
        _ => return None,
    })
}

// ------------------------------------------------------------------ calendar helpers

/// Day of week from "YYYY-MM-DD": 0 = Sunday .. 6 = Saturday.
pub fn dow(date: &str) -> u32 {
    let y: i64 = date[0..4].parse().unwrap_or(1970);
    let m: i64 = date[5..7].parse().unwrap_or(1);
    let d: i64 = date[8..10].parse().unwrap_or(1);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    (days + 4).rem_euclid(7) as u32
}

/// Trading day of the month (1-based) for every bar; 0 = unknown. A series that starts mid-month cannot count that
/// month's trading days, so its first month is 0 unless every earlier day of that month is a weekend day or 1 January
/// (decision 186, FR-08).
fn tdm(x: &Ctx) -> Vec<usize> {
    let mut out = vec![0; x.r.len()];
    let Some(first) = x.r.first() else { return out };
    let d0: u32 = first.date[8..10].parse().unwrap_or(1);
    let whole = (1..d0).all(|d| { let s = format!("{}-{:02}", &first.date[..7], d); matches!(dow(&s), 0 | 6) || s[5..] == *"01-01" });
    for i in 0..x.r.len() {
        out[i] = if i > 0 && x.r[i - 1].date[..7] == x.r[i].date[..7] { if out[i - 1] == 0 { 0 } else { out[i - 1] + 1 } }
            else if i == 0 && !whole { 0 } else { 1 };
    }
    out
}

fn dow_bit(date: &str) -> u32 {
    match dow(date) { d @ 1..=5 => 1 << (d - 1), _ => 0 }
}

// ------------------------------------------------------------------ path simulator

#[derive(Clone, Copy)]
pub(crate) enum Ent { Open, Stop(f64), /** market on this bar's close, if flat after the bar (lane F1D) */ Close }

pub(crate) struct Cfg {
    pub(crate) stop: f64,        // protective stop distance in points (NaN = none)
    pub(crate) stop_rng: f64,    // stop = x * previous bar's range (NaN = not used)
    pub(crate) day0: bool,       // stop armed on the entry bar
    pub(crate) target: f64,      // profit target distance in points (NaN = none)
    pub(crate) hold: Option<usize>, // exit on the close `hold` bars after the entry bar
    pub(crate) bailout: bool,    // exit on the first profitable opening after the entry bar
    pub(crate) bail_after: usize, // lane F1D: the bailout only from the opening `bail_after` bars after entry (0/1 = the next bar)
    pub(crate) reverse: bool,    // an opposite entry signal exits and reverses
    pub(crate) order: bool,      // EP6 (#243): order-dependent days on the real 1-minute path (false = the bar path always)
}

#[derive(Clone)]
struct Pos { dir: f64, i: usize, px: f64, stop: f64, target: f64, od: crate::intraday_ep6::Day, close: bool }

fn close_trade(x: &Ctx, p: &Pos, i: usize, px: f64, why: &str) -> Trade {
    let kind = if p.close { "close" } else if p.px == x.o[p.i] { "open" } else { "stop" };
    let mut t = Trade { dir: p.dir, legs: vec![Leg { i: p.i, px: p.px, w: 1.0 }], exit_i: i, exit_px: px, reason: why.into(),
        entry_kind: kind, signal_i: i, open: false, exits: vec![] };
    crate::intraday_ep6::mark(&mut t, p.od);
    t
}

/// The intrabar state of one bar (EP6): the position, the per-direction fill flags, and the trades closed inside the bar
/// with the mark of their position (applied after the day is resolved).
#[derive(Clone)]
struct Bar3 { pos: Option<Pos>, used: [bool; 2], out: Vec<(Pos, f64, &'static str)> }

/// Bar-by-bar simulation. `sig(i)` returns the (long, short) entry orders for bar i, decided from data before
/// bar i plus bar i's open where the book's rule reads the open.
pub(crate) fn simulate(x: &Ctx, from: usize, to: usize, cfg: &Cfg, sig: impl Fn(usize) -> (Option<Ent>, Option<Ent>)) -> Vec<Trade> {
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<Pos> = None;
    for i in from.max(1)..=to {
        let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        let open_pos = |dir: f64, px: f64| {
            let rs = cfg.stop_rng * (x.h[i - 1] - x.l[i - 1]);
            // lane F1D: "$1,500 or 50 percent of the previous day's range" (p.61) = the nearer of the two when both are set
            let sd = if cfg.stop_rng.is_finite() && cfg.stop.is_finite() { rs.min(cfg.stop) } else if cfg.stop_rng.is_finite() { rs } else { cfg.stop };
            Pos { dir, i, px, stop: if sd.is_finite() && sd > 0.0 { px - dir * sd } else { f64::NAN },
                target: if cfg.target.is_finite() && cfg.target > 0.0 { px + dir * cfg.target } else { f64::NAN }, od: crate::intraday_ep6::Day::Plain, close: false }
        };
        // enter `dir` at `px`: flat -> open; opposite and reverse -> close and open; otherwise ignored
        let enter = |pos: &mut Option<Pos>, out: &mut Vec<Trade>, dir: f64, px: f64| match pos.take() {
            None => *pos = Some(open_pos(dir, px)),
            Some(p) if p.dir != dir && cfg.reverse => { out.push(close_trade(x, &p, i, px, "reverse: opposite signal")); *pos = Some(open_pos(dir, px)); }
            Some(p) => *pos = Some(p),
        };
        // 1. at the open: exits of positions from earlier bars
        if let Some(p) = &pos {
            let why = if cfg.bailout && i >= p.i + cfg.bail_after && p.dir * (o - p.px) > 0.0 { Some("bailout: first profitable opening") }
                else if p.stop.is_finite() && p.dir * (o - p.stop) <= 0.0 { Some("stop (gap at the open)") }
                else if p.target.is_finite() && p.dir * (o - p.target) >= 0.0 { Some("target (gap at the open)") }
                else { None };
            if let Some(w) = why { out.push(close_trade(x, p, i, o, w)); pos = None; }
        }
        // 2. entry orders for this bar; market-on-open orders and stops gapped through fill at the open
        let (le, se) = sig(i);
        let mut used = [false, false]; // one fill per direction per bar
        for (k, e, dir) in [(0usize, le, 1.0), (1, se, -1.0)] {
            let at_open = match e { Some(Ent::Open) => true, Some(Ent::Stop(lv)) => dir * (o - lv) >= 0.0, Some(Ent::Close) | None => false };
            if at_open { used[k] = true; enter(&mut pos, &mut out, dir, o); }
        }
        // 3. intrabar path O -> nearer extreme -> other extreme -> C (TradeStation default); EP6: an order-dependent day
        // of an EP6 id is walked on the real 1-minute path instead (intraday_ep6::resolve)
        let walk = |st: &mut Bar3, segs: &[crate::intraday_ep6::Seg]| -> Vec<crate::intraday_ep6::Ev> {
            let mut ev = vec![];
            for g in segs {
                let (mut cur, end) = (g.a, g.b);
                let up = end > cur;
                loop {
                    let mut best: Option<(f64, u8)> = None; // 0 long entry, 1 short entry, 2 stop, 3 target
                    let mut consider = |lv: f64, a: u8| {
                        let inside = lv.is_finite() && if up { lv > cur && lv <= end } else { lv < cur && lv >= end };
                        if inside && best.map_or(true, |(b, _)| if up { lv < b } else { lv > b }) { best = Some((lv, a)); }
                    };
                    if let Some(p) = &st.pos {
                        let against = (p.dir > 0.0) != up;
                        if against && (p.i < i || cfg.day0) { consider(p.stop, 2); }
                        if !against { consider(p.target, 3); }
                    }
                    let free = |dir: f64| st.pos.as_ref().map_or(true, |p| cfg.reverse && p.dir != dir);
                    if up && !st.used[0] && free(1.0) { if let Some(Ent::Stop(lv)) = le { consider(lv, 0); } }
                    if !up && !st.used[1] && free(-1.0) { if let Some(Ent::Stop(lv)) = se { consider(lv, 1); } }
                    let Some((lv, a)) = best else { break };
                    let px = g.fill(lv);
                    match a {
                        0 | 1 => {
                            let dir = if a == 0 { 1.0 } else { -1.0 };
                            st.used[a as usize] = true;
                            match st.pos.take() {
                                None => st.pos = Some(open_pos(dir, px)),
                                Some(p) if p.dir != dir && cfg.reverse => { st.out.push((p, px, "reverse: opposite signal")); st.pos = Some(open_pos(dir, px)); }
                                Some(p) => st.pos = Some(p),
                            }
                        }
                        2 => { let p = st.pos.take().unwrap(); st.out.push((p, px, "stop")); }
                        _ => { let p = st.pos.take().unwrap(); st.out.push((p, px, "target")); }
                    }
                    ev.push(crate::intraday_ep6::Ev { k: a, px, up });
                    cur = lv;
                }
            }
            ev
        };
        let mut st = Bar3 { pos: pos.take(), used, out: vec![] };
        let day = crate::intraday_ep6::resolve(x, i, cfg.order, h - o <= o - l, &mut st,
            |s, hf| walk(s, &crate::intraday_ep6::bar_segs(o, h, l, c, hf)), &walk);
        for (mut p, px, why) in st.out { p.od = p.od.worst(day); out.push(close_trade(x, &p, i, px, why)); }
        pos = st.pos;
        if let Some(p) = pos.as_mut() { p.od = p.od.worst(day); }
        // 4. at the close: time exits
        if let Some(p) = &pos {
            if cfg.hold.map_or(false, |n| i >= p.i + n) { out.push(close_trade(x, p, i, c, "time exit on the close")); pos = None; }
        }
        // 5. market-on-close entries (lane F1D): only when flat after the bar, so a bar that stopped a trade out can re-enter
        for (e, dir) in [(le, 1.0), (se, -1.0)] {
            if pos.is_none() && matches!(e, Some(Ent::Close)) { let mut p = open_pos(dir, c); p.close = true; pos = Some(p); }
        }
    }
    if let Some(p) = pos {
        let mut t = close_trade(x, &p, to, x.c[to], "open");
        t.open = true;
        out.push(t);
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) }; // EP6 #243 (c)
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let pts = |k: &str| { let v = g(k); if v.is_finite() && v > 0.0 { v / g("pt_usd") } else { f64::NAN } };
    let hold = g("hold");
    let mut cfg = Cfg { stop: pts("stop_usd"), stop_rng: f64::NAN, day0: g("stop_day0") != 0.0, target: pts("target_usd"),
        hold: hold.is_finite().then(|| hold.max(0.0) as usize), bailout: false, bail_after: 0, reverse: false, order: crate::intraday_ep6::on(id, p) };
    let wd = |i: usize| dow(&x.r[i].date);
    let td = tdm(x);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let none = (None, None);
    let t = match id {
        "lwlts_mon_gap_t500" | "lwlts_mon_gap_t1000" | "lwlts_mon_gap_close" => {
            cfg.hold = Some(0);
            simulate(x, from, to, &cfg, |i| if wd(i) == 1 && o[i] < c[i - 1] { (Some(Ent::Open), None) } else { none })
        }
        "lwlts_mon_gap_next_close" | "lwlts_mon_gap_6close" =>
            simulate(x, from, to, &cfg, |i| if wd(i) == 1 && o[i] < c[i - 1] { (Some(Ent::Open), None) } else { none }),
        "lwlts_vb_sp_tdw" | "lwlts_vb_sp_40_200" | "lwlts_talon_sp" => {
            cfg.bailout = true; cfg.reverse = true; cfg.hold = None;
            if id != "lwlts_talon_sp" { cfg.stop_rng = g("stop_rng"); cfg.stop = f64::NAN; }
            let (bm, sm) = (g("buy_mult"), g("sell_mult"));
            let (bd, sd) = (g("buy_dow") as u32, g("sell_dow") as u32);
            let talon = id == "lwlts_talon_sp";
            simulate(x, from, to, &cfg, |i| {
                let base = if talon {
                    if i < 4 { return none; }
                    let s = i - 1;
                    (h[s - 3] - l[s]).max(h[s - 1] - l[s - 3])
                } else { h[i - 1] - l[i - 1] };
                if !(base > 0.0) { return none; }
                let bit = dow_bit(&x.r[i].date);
                // 31 = every session (Talon has no day filter; NSE also has a few Saturday sessions)
                let ok = |m: u32| m == 31 || m & bit != 0;
                (ok(bd).then(|| Ent::Stop(o[i] + bm * base)), ok(sd).then(|| Ent::Stop(o[i] - sm * base)))
            })
        }
        "lwlts_mon_oc" | "lwlts_mon_3day" => simulate(x, from, to, &cfg, |i| if wd(i) == 1 { (Some(Ent::Open), None) } else { none }),
        // ---- intraday wave 1: real 1-minute path (default path = 1); the $ stop is S&P points (pt_usd per point)
        "lwlts_mon_vb" if g("path") >= 0.5 => {
            use crate::intraday::{day_trade, stop_entry_day, Level};
            let Some(mm) = x.minutes else { return Some(vec![]) };
            let (m, sd) = (g("buy_mult"), cfg.stop);
            let mut out = vec![];
            for i in from.max(1)..=to {
                if wd(i) != 1 { continue; }
                let Some(b) = mm.day(&x.r[i].date) else { continue };
                let st = |_: usize, px: f64| (sd.is_finite() && cfg.day0).then(|| Level { up: false, px: px - sd });
                let Some((_, px, xs)) = stop_entry_day(&b, 0, &[Level { up: true, px: o[i] + m * (h[i - 1] - l[i - 1]) }], &st) else { continue };
                out.push(day_trade(1.0, i, px, i, xs.unwrap_or(c[i]), if xs.is_some() { "stop" } else { "time exit on the close" }, "stop", false));
            }
            out
        }
        // ---- end intraday wave 1
        "lwlts_mon_vb" => {
            cfg.hold = Some(0);
            let m = g("buy_mult");
            simulate(x, from, to, &cfg, |i| if wd(i) == 1 { (Some(Ent::Stop(o[i] + m * (h[i - 1] - l[i - 1]))), None) } else { none })
        }
        "lwlts_tdm8_bonds" | "lwlts_tdm18_bonds" | "lwlts_tdm19_bonds" | "lwlts_tdm20_bonds" | "lwlts_tdm21_bonds" | "lwlts_tdm22_bonds" => {
            // lane F1C (R4 #293): only on the Norgate T-bond contract (readiness refuses other tickers with futures::home)
            if crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
            let n: usize = id["lwlts_tdm".len()..id.len() - "_bonds".len()].parse().ok()?;
            simulate(x, from, to, &cfg, |i| if td[i] == n { (Some(Ent::Open), None) } else { none })
        }
        "lwlts_vb_bonds" | "lwlts_vb_bonds_tdw" | "lwlts_tdw_bonds" | "lwlts_outside_down_bonds" | "lwlts_outside_down_bonds_xthu" | "lwlts_oops_buy_bonds"
        | "lwlts_3up_thu_sell_bonds" | "lwlts_oops_sell_bonds_wed" | "lwlts_oops_buy_bonds_ma9" | "lwlts_oops_sell_bonds_ma9" | "lwlts_gsv_bonds" => {
            // lane F1D (R4 #293): the book's T-bond rules, only on the Norgate T-bond contract (readiness refuses other tickers)
            if crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
            cfg.hold = None; cfg.bailout = true;
            let ba = g("bail_after");
            if ba.is_finite() { cfg.bail_after = ba.max(1.0) as usize; }
            let rng = |i: usize| h[i - 1] - l[i - 1];
            match id {
                "lwlts_vb_bonds" | "lwlts_vb_bonds_tdw" => {
                    cfg.reverse = true; cfg.stop_rng = g("stop_rng");
                    let (bm, sm, bd, sd) = (g("buy_mult"), g("sell_mult"), g("buy_dow") as u32, g("sell_dow") as u32);
                    simulate(x, from, to, &cfg, |i| {
                        let (bit, r) = (dow_bit(&x.r[i].date), rng(i));
                        if !(r > 0.0) { return none; }
                        let ok = |m: u32| m == 31 || m & bit != 0;
                        (ok(bd).then(|| Ent::Stop(o[i] + bm * r)), ok(sd).then(|| Ent::Stop(o[i] - sm * r)))
                    })
                }
                "lwlts_tdw_bonds" => {
                    cfg.bailout = false; cfg.hold = Some(0);
                    let m = g("buy_mult");
                    simulate(x, from, to, &cfg, |i| if wd(i) == 2 { (Some(Ent::Stop(o[i] + m * rng(i))), None) } else { none })
                }
                "lwlts_outside_down_bonds" | "lwlts_outside_down_bonds_xthu" => {
                    let xthu = id.ends_with("xthu");
                    simulate(x, from, to, &cfg, |i| {
                        if i < 2 { return none; }
                        let s = i - 1;
                        let pat = h[s] > h[s - 1] && l[s] < l[s - 1] && c[s] < l[s - 1] && o[i] < c[s];
                        if pat && !(xthu && wd(i) == 4) { (Some(Ent::Open), None) } else { none }
                    })
                }
                "lwlts_3up_thu_sell_bonds" => simulate(x, from, to, &cfg, |i| {
                    if i < 4 { return none; }
                    let s = i - 1;
                    let pat = wd(s) == 4 && wd(i) == 5 && c[s] > h[s - 1] && c[s - 1] > c[s - 2] && c[s - 2] > c[s - 3];
                    if pat { (None, Some(Ent::Open)) } else { none }
                }),
                "lwlts_gsv_bonds" => {
                    cfg.reverse = true;
                    let m = g("mult");
                    // option p.gold: gold's own closes (Data2, carried to the bond dates): close below the close 20 gold bars earlier
                    let gold: Option<Vec<f64>> = if g("gold") >= 0.5 {
                        let Some(rows) = crate::futures::rows("&GC") else { return Some(vec![]) }; // readiness refuses this case (data2_issue)
                        let gc = crate::second::align("&GC", x.r, rows);
                        Some(gc.own(|r| (0..r.len()).map(|k| if k >= 20 { (r[k].close < r[k - 20].close) as u8 as f64 } else { f64::NAN }).collect()))
                    } else { None };
                    simulate(x, from, to, &cfg, |i| {
                        if i < 8 { return none; }
                        let bs = (i - 4..i).map(|k| h[k] - o[k]).sum::<f64>() / 4.0;
                        let ss = (i - 4..i).map(|k| o[k] - l[k]).sum::<f64>() / 4.0;
                        let buy = c[i - 1] < c[i - 6] && matches!(wd(i), 2 | 3 | 5) && bs > 0.0;
                        let sell = c[i - 1] > c[i - 7] && ss > 0.0 && gold.as_ref().map_or(true, |v| v[i - 1] == 1.0);
                        (buy.then(|| Ent::Stop(o[i] + m * bs)), sell.then(|| Ent::Stop(o[i] - m * ss)))
                    })
                }
                _ => {
                    let ma = crate::connors::sma(c, 9);
                    simulate(x, from, to, &cfg, |i| {
                        if i < 2 { return none; }
                        let d = wd(i);
                        match id {
                            "lwlts_oops_buy_bonds" if o[i] < l[i - 1] && d != 3 => (Some(Ent::Stop(l[i - 1])), None),
                            "lwlts_oops_buy_bonds_ma9" if o[i] < l[i - 1] && d != 4 && ma[i - 1] < ma[i - 2] => (Some(Ent::Stop(l[i - 1])), None),
                            "lwlts_oops_sell_bonds_wed" if o[i] > h[i - 1] && d == 3 => (None, Some(Ent::Stop(h[i - 1]))),
                            "lwlts_oops_sell_bonds_ma9" if o[i] > h[i - 1] && d == 3 && ma[i - 1] > ma[i - 2] => (None, Some(Ent::Stop(h[i - 1]))),
                            _ => none,
                        }
                    })
                }
            }
        }
        "lwlts_month_day1_bonds" | "lwlts_tdm18_gold_bonds" | "lwlts_tdm22_gold_bonds" | "lwlts_tdm12_sell_bonds" | "lwlts_tdm12_sell_gold_bonds" => {
            // lane F1D (R4 #293): Ch. 10 bond month trades, only on the Norgate T-bond contract
            if crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
            // gold (Data2): its close on the bond's bar i-1 vs gold_n gold bars earlier, on gold's own bars
            let gold = |up: bool| -> Option<Vec<f64>> {
                let n = g("gold_n").max(1.0) as usize;
                let rows = crate::futures::rows("&GC")?;
                let gc = crate::second::align("&GC", x.r, rows);
                Some(gc.own(|r| (0..r.len()).map(|k| if k >= n { ((r[k].close > r[k - n].close) == up && r[k].close != r[k - n].close) as u8 as f64 } else { f64::NAN }).collect()))
            };
            match id {
                "lwlts_month_day1_bonds" => {
                    cfg.bailout = true; cfg.hold = None;
                    let skip = g("skip") >= 0.5;
                    simulate(x, from, to, &cfg, |i| {
                        let m = &x.r[i].date[5..7];
                        if td[i] == 1 && !(skip && matches!(m, "01" | "02" | "04" | "10")) { (Some(Ent::Open), None) } else { none }
                    })
                }
                "lwlts_tdm12_sell_bonds" => simulate(x, from, to, &cfg, |i| if td[i] == 12 { (None, Some(Ent::Open)) } else { none }),
                "lwlts_tdm12_sell_gold_bonds" => {
                    let Some(gu) = gold(true) else { return Some(vec![]) }; // readiness refuses this case (futures::data2_issue)
                    simulate(x, from, to, &cfg, |i| if td[i] == 12 && i > 0 && gu[i - 1] == 1.0 { (None, Some(Ent::Open)) } else { none })
                }
                _ => {
                    let n = if id == "lwlts_tdm18_gold_bonds" { 18 } else { 22 };
                    let Some(gd) = gold(false) else { return Some(vec![]) };
                    simulate(x, from, to, &cfg, |i| if td[i] == n && i > 0 && gd[i - 1] == 1.0 { (Some(Ent::Open), None) } else { none })
                }
            }
        }
        "lwlts_talon_bondfilter" | "lwlts_gsv_sp" | "lwlts_gsv_sp_monday" | "lwlts_month_day1_sp_bonds" | "lwlts_bonds_channel_bstop" | "lwlts_bonds_channel_sstop" => {
            // lane F1D (R4 #293): S&P with the T-bond as Data2, only on the Norgate S&P 500 contract
            if crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
            let Some(rows) = crate::futures::rows("&ZB") else { return Some(vec![]) }; // readiness refuses this case (futures::data2_issue)
            let zb = crate::second::align("&ZB", x.r, rows);
            // bond close vs its close n bond bars earlier (own bars): +1 above, -1 below, 0 equal / unknown
            let bvs = |n: usize| zb.own(|r| (0..r.len()).map(|k| if k >= n { (r[k].close - r[k - n].close).signum() * (r[k].close != r[k - n].close) as u8 as f64 } else { 0.0 }).collect());
            match id {
                "lwlts_talon_bondfilter" => {
                    cfg.bailout = true; cfg.reverse = true; cfg.hold = None;
                    let (bm, sm, b5, b35) = (g("buy_mult"), g("sell_mult"), bvs(5), bvs(35));
                    simulate(x, from, to, &cfg, |i| {
                        if i < 4 { return none; }
                        let s = i - 1;
                        let base = (h[s - 3] - l[s]).max(h[s - 1] - l[s - 3]);
                        if !(base > 0.0) { return none; }
                        ((b5[s] > 0.0).then(|| Ent::Stop(o[i] + bm * base)), (b35[s] < 0.0).then(|| Ent::Stop(o[i] - sm * base)))
                    })
                }
                "lwlts_gsv_sp" => {
                    cfg.bailout = true; cfg.reverse = true; cfg.hold = None;
                    let (m, b15) = (g("mult"), bvs(15));
                    simulate(x, from, to, &cfg, |i| {
                        if i < 8 { return none; }
                        let s = i - 1;
                        let bs = (i - 4..i).map(|k| h[k] - o[k]).sum::<f64>() / 4.0;
                        let ss = (i - 4..i).map(|k| c[k] - l[k]).sum::<f64>() / 4.0;
                        let buy = c[s] < c[s - 6] && b15[s] > 0.0 && matches!(wd(i), 1 | 2 | 3) && bs > 0.0;
                        let sell = c[s] > c[s - 6] && b15[s] < 0.0 && wd(i) != 1 && ss > 0.0;
                        (buy.then(|| Ent::Stop(o[i] + m * bs)), sell.then(|| Ent::Stop(o[i] - m * ss)))
                    })
                }
                "lwlts_gsv_sp_monday" => {
                    cfg.bailout = true; cfg.hold = None;
                    let b15 = bvs(15);
                    simulate(x, from, to, &cfg, |i| {
                        if i < 2 { return none; }
                        let s = i - 1;
                        if wd(i) == 1 && wd(s) == 5 && c[s] < c[s - 1] && b15[s] > 0.0 { (Some(Ent::Stop(o[i] + h[s] - o[s])), None) } else { none }
                    })
                }
                "lwlts_month_day1_sp_bonds" => {
                    cfg.bailout = true; cfg.hold = None;
                    let b30 = bvs(30);
                    simulate(x, from, to, &cfg, |i| if td[i] == 1 && i > 0 && b30[i - 1] > 0.0 { (Some(Ent::Open), None) } else { none })
                }
                _ => {
                    // bond 14-bar breakout -> buy the S&P close; exits: the $ stop (and the S&P 12-bar low for _sstop) as sell stops
                    // (the higher one first; at the open when gapped), or (_bstop) the S&P close of a day the bond's low breaks the
                    // lowest bond low of the 17 bars before it
                    let brk = zb.own(|r| (0..r.len()).map(|k| (k >= 14 && r[k].close > r[k - 14..k].iter().map(|b| b.high).fold(f64::MIN, f64::max)) as u8 as f64).collect());
                    let bdn = zb.own(|r| (0..r.len()).map(|k| (k >= 17 && r[k].low < r[k - 17..k].iter().map(|b| b.low).fold(f64::MAX, f64::min)) as u8 as f64).collect());
                    let sstop = id.ends_with("sstop");
                    let mut out = vec![];
                    let mut pos: Option<Pos> = None;
                    for i in from.max(1)..=to {
                        if let Some(p) = pos.take() {
                            let mut lv = p.stop;
                            if sstop && i >= 12 { lv = lv.max(l[i - 12..i].iter().copied().fold(f64::MAX, f64::min)); }
                            if o[i] <= lv { out.push(close_trade(x, &p, i, o[i], "stop (gap at the open)")); }
                            else if l[i] <= lv { out.push(close_trade(x, &p, i, lv, if lv == p.stop { "stop" } else { "S&P 12-day low" })); }
                            else if !sstop && bdn[i] == 1.0 { out.push(close_trade(x, &p, i, c[i], "bond 17-day low: S&P close")); }
                            else { pos = Some(p); }
                        }
                        if pos.is_none() && brk[i] == 1.0 {
                            let st = if cfg.stop.is_finite() && cfg.stop > 0.0 { c[i] - cfg.stop } else { f64::NAN };
                            pos = Some(Pos { dir: 1.0, i, px: c[i], stop: st, target: f64::NAN, od: crate::intraday_ep6::Day::Plain, close: true });
                        }
                    }
                    if let Some(p) = pos { let mut t = close_trade(x, &p, to, c[to], "open"); t.open = true; out.push(t); }
                    out
                }
            }
        }
        "lwlts_bonds_8day" => {
            // lane F1D (R4 #293): only on the Norgate T-bond contract (readiness refuses other tickers with futures::home)
            if crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
            let n = g("lookback").max(1.0) as usize;
            simulate(x, from, to, &cfg, |i| if i >= n && c[i] > c[i - n] { (Some(Ent::Close), None) } else { none })
        }
        "lwlts_tdm6_sp" | "lwlts_tdm7_sp" | "lwlts_tdm8_sp" | "lwlts_tdm19_sp" | "lwlts_tdm20_sp" | "lwlts_tdm21_sp" | "lwlts_tdm22_sp" => {
            let n: usize = id["lwlts_tdm".len()..id.len() - 3].parse().ok()?;
            simulate(x, from, to, &cfg, |i| if td[i] == n { (Some(Ent::Open), None) } else { none })
        }
        "lwlts_study_all" | "lwlts_study_down1" | "lwlts_study_down3" | "lwlts_study_trend" | "lwlts_study_trend_mon" => {
            simulate(x, from, to, &cfg, |i| {
                let ok = match id {
                    "lwlts_study_all" => true,
                    "lwlts_study_down1" => i >= 2 && c[i - 1] < c[i - 2],
                    "lwlts_study_down3" => i >= 4 && c[i - 1] < c[i - 2] && c[i - 2] < c[i - 3] && c[i - 3] < c[i - 4],
                    "lwlts_study_trend" => i >= 31 && c[i - 1] > c[i - 31] && c[i - 1] < c[i - 10],
                    _ => i >= 31 && c[i - 1] > c[i - 31] && c[i - 1] < c[i - 10] && wd(i) == 1,
                };
                if ok { (Some(Ent::Open), None) } else { none }
            })
        }
        "lwlts_outside_down" | "lwlts_outside_down_xthu" => {
            cfg.bailout = true; cfg.hold = None;
            let xthu = id.ends_with("xthu");
            simulate(x, from, to, &cfg, |i| {
                if i < 2 { return none; }
                let s = i - 1;
                let pat = h[s] > h[s - 1] && l[s] < l[s - 1] && c[s] < l[s - 1] && o[i] < c[s];
                if pat && !(xthu && wd(i) == 4) { (Some(Ent::Open), None) } else { none }
            })
        }
        "lwlts_3up_tue_sell" => {
            cfg.bailout = true; cfg.hold = None;
            simulate(x, from, to, &cfg, |i| {
                if i < 4 { return none; }
                let s = i - 1;
                let pat = wd(s) == 2 && wd(i) == 3 && c[s] > h[s - 1] && c[s - 1] > c[s - 2] && c[s - 2] > c[s - 3];
                if pat { (None, Some(Ent::Open)) } else { none }
            })
        }
        "lwlts_oops_buy" | "lwlts_oops_sell_thu" | "lwlts_oops_buy_ma9" | "lwlts_oops_sell_ma9" => {
            cfg.bailout = true; cfg.hold = None;
            let ma = crate::connors::sma(c, 9);
            simulate(x, from, to, &cfg, |i| {
                if i < 2 { return none; }
                let d = wd(i);
                match id {
                    "lwlts_oops_buy" if o[i] < l[i - 1] && d != 3 && d != 4 => (Some(Ent::Stop(l[i - 1])), None),
                    "lwlts_oops_buy_ma9" if o[i] < l[i - 1] && matches!(d, 2 | 3 | 5) && ma[i - 1] < ma[i - 2] => (Some(Ent::Stop(l[i - 1])), None),
                    "lwlts_oops_sell_thu" if o[i] > h[i - 1] && d == 4 => (None, Some(Ent::Stop(h[i - 1]))),
                    "lwlts_oops_sell_ma9" if o[i] > h[i - 1] && d == 3 && ma[i - 1] > ma[i - 2] => (None, Some(Ent::Stop(h[i - 1]))),
                    _ => none,
                }
            })
        }
        "lwlts_month_day1" => {
            cfg.bailout = true; cfg.hold = None;
            let skip = g("skip_jfo") != 0.0;
            simulate(x, from, to, &cfg, |i| {
                let m = &x.r[i].date[5..7];
                if td[i] == 1 && i > 0 && !(skip && matches!(m, "01" | "02" | "10")) { (Some(Ent::Open), None) } else { none }
            })
        }
        "lwlts_hh1" | "lwlts_hh5" | "lwlts_hh10" | "lwlts_hh15" | "lwlts_hh20" => {
            let n: usize = id["lwlts_hh".len()..].parse().ok()?;
            simulate(x, from, to, &cfg, |i| {
                if i < n + 1 { return none; }
                let s = i - 1;
                let prior = h[s - n..s].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let level = h[s + 1 - n..=s].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                if h[s] < prior { (Some(Ent::Stop(level)), None) } else { none }
            })
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
    fn tdm_unknown_in_a_partial_first_month() {
        let x = |d: &[&str]| { let r: Vec<Row> = d.iter().map(|d| bar(d, 1., 1., 1., 1.)).collect(); tdm(&Ctx::new(&r, &r, &[])) };
        // mid-month start: the first month is unknown (0), the next month counts from 1
        assert_eq!(x(&["1993-01-29", "1993-02-01", "1993-02-02"]), vec![0, 1, 2]);
        // 1983-01-03 is the first weekday of January (1-2 Jan are a weekend): the month counts
        assert_eq!(x(&["1983-01-03", "1983-01-04"]), vec![1, 2]);
        // 1970-01-02: only New Year's Day before it
        assert_eq!(x(&["1970-01-02", "1970-01-05"]), vec![1, 2]);
    }
    #[test]
    fn weekdays() {
        assert_eq!(dow("1987-10-19"), 1); // Black Monday
        assert_eq!(dow("2024-02-29"), 4);
        assert_eq!(dow("1998-08-28"), 5);
    }
    #[test]
    fn monday_gap_close_stop_and_target() {
        // Fri 1998-08-21, Mon 08-24 opens below Friday's close.
        let r = vec![bar("1998-08-20", 100., 101., 99., 100.), bar("1998-08-21", 100., 101., 99., 100.),
            bar("1998-08-24", 99., 100., 98., 99.5), bar("1998-08-25", 99., 100., 98., 99.5)];
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lwlts_mon_gap_close", &x, 1, 3, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (2, 99., 2, 99.5));
        // $500 target = 2 points from 99 -> 101 not reached; $3,000 stop = 12 points not reached
        let t = run("lwlts_mon_gap_t500", &x, 1, 3, &Params::new()).unwrap();
        assert_eq!(t[0].exit_px, 99.5);
        let p = Params::from([("target_usd".to_string(), 250.0)]); // 1 point -> 100, hit on the way up
        let t = run("lwlts_mon_gap_t500", &x, 1, 3, &p).unwrap();
        assert_eq!(t[0].exit_px, 100.0);
    }
    #[test]
    fn oops_buy_fills_at_prior_low_and_bails_out_next_profitable_open() {
        // Tue 1998-08-25 opens below Monday's low 98 and rallies through it.
        let r = vec![bar("1998-08-21", 100., 101., 99., 100.), bar("1998-08-24", 99., 100., 98., 99.),
            bar("1998-08-25", 97., 99., 96.5, 98.5), bar("1998-08-26", 98.2, 99., 97., 98.)];
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lwlts_oops_buy", &x, 1, 3, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px), (2, 98.0));
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (3, 98.2, "bailout: first profitable opening"));
    }
}
