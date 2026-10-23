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

const MON_F: &str = "E|\"buy on the open every Monday if that open is lower than Friday's close\" (p.52; Monday = the bar's weekday, a Monday holiday means no trade)";
const STOP3000: &str = "X|$3,000 stop (Fig. 3.7-3.11 headers; largest loss $3,050 = stop + $50 commission) = 12 S&P points, armed on the entry day";
const TDM_R: &[&str] = &["E|\"buying ... on the opening of the trading day of the month shown\"; TDM = count of trading days so far this month (p.88)",
    "X|\"exited with either a $2,500 stop in the S&P 500 ... or on the close the third day after entry\" (p.88-89)",
    "X|\"The S&P results reflect no stop on entry day, but a $2,000 stop after entry day\" (p.89): default $2,000 = 8 points from the day after entry (p.stop_usd=2500 is the p.88 figure)"];
const TDM_N: &str = "Table 6.6 (p.89), S&P 1982-1998, each TDM tested on its own; the table's 'Number of Trades' column prints trades/winners. Long only (the table lists rally days).";
const STUDY_X: &str = "X|\"buy ... and exit on the next close with a $3,250 stop\" (p.95); Read: entry on the next open, exit at the close of the day after entry (2-bar hold: fits 2,064 trades in Jul-1982..Feb-1998; 'buy on the open tomorrow and exit on the next day's close' p.96)";
const STOP2000: &str = "X|$2,000 stop (Fig. headers: losers exactly -$2,000) = 8 points, armed on the entry day";
const BAIL: &str = "X|Bailout: exit on the first profitable opening after entry (Ch. 11, p.157)";
const OOPS_B: &str = "E|\"an opening that is below the previous day's low ... The entry comes when ... price then rallies back to the previous day's low\" (p.114): buy stop at yesterday's low, only on a day that opened below it";
const OOPS_S: &str = "E|\"an open greater than the prior day's high ... Our entry then comes from price falling back to the prior high\" (p.114): sell stop at yesterday's high, only on a day that opened above it";
const HH_R: &[&str] = &["F|\"today's high is less than the highest high of the last X days\" (p.239): High < Highest(High, X) of the X bars before today",
    "E|\"and price makes a new high tomorrow, which puts us long buying a new X day high\": buy stop tomorrow at Highest(High, X) including today",
    "X|\"exit 13 days later $3,500 stop\" (Fig. 15.4 caption): close of the 13th bar after entry; $3,500 = 14 points"];
const HH_N: &str = "Fig. 15.4 (p.239), S&P 500; test window not printed. Read: TradeStation Highest(High,X) on today's bar as the stop level, setup uses the X bars before today. Stop armed on the entry day.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{NeedsData, NotTestable};
    vec![
        // ---- Chapter 3: Monday gap-down buy (p.52-55)
        live!("lwlts_mon_gap_t500", "Monday gap-down buy · $500 target", "Ch 3 · Hold to the close", "Long",
            &[MON_F, "X|\"$3,000 ... and an automatic $500 profit\" (p.52): $500 = 2 points limit, else exit on the close", STOP3000],
            SP, claim("389", "59%", "$-20.95 (after $50 comm.)", "same day", "07/02/82-08/24/98")),
        live!("lwlts_mon_gap_t1000", "Monday gap-down buy · $1,000 target", "Ch 3 · Hold to the close", "Long",
            &[MON_F, "X|\"all the same rules except a $1,000 target\" (p.52): 4 points, else exit on the close", STOP3000],
            SP, claim("389", "55%", "$35.31", "same day", "07/02/82-08/24/98")),
        live!("lwlts_mon_gap_close", "Monday gap-down buy · exit on the close", "Ch 3 · Hold to the close", "Long",
            &[MON_F, "X|\"holding until the close and then exiting the position\" (p.53)", STOP3000],
            SP, claim("389", "53%", "$100.45", "same day", "07/02/82-08/24/98")),
        live!("lwlts_mon_gap_next_close", "Monday gap-down buy · exit next close", "Ch 3 · Hold to the close", "Long",
            &[MON_F, "X|\"hold the position ... until the next close; that is, the first close after our entry day or until we are stopped out\" (p.54)", STOP3000],
            SP, claim("397", "55%", "$172.07", "1 day", "07/02/82-08/24/98")),
        live!("lwlts_mon_gap_6close", "Monday gap-down buy · close 6 days after entry", "Ch 3 · Hold to the close", "Long",
            &[MON_F, "X|\"holding for the close 6 days after entry or being stopped out\" (p.54)", STOP3000],
            SP, claim("285", "52%", "$251.23", "6 days", "07/02/82-08/24/98")),
        // ---- Chapter 4: volatility breakouts (p.57-73)
        live!("lwlts_vb_sp_tdw", "S&P volatility breakout · 50% of yesterday's range, TDW", "Ch 4 · Volatility breakouts", "Long/Short",
            &["E|Buy at the open + 50% of the previous day's range, sell at the open - 50% (\"a 50 percent volatility expansion\", p.66; \"add our breakout value to tomorrow's opening\", p.61)",
              "F|\"The best days to be a buyer were all days except Thursday and Friday, while the best sell day was Thursday, with Friday a push, but it is used\" (p.68)",
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
            &["E|\"buying on the opening and exiting 3 days later\" (p.86): Mondays", "X|Read: exit on the close of the 3rd trading day after entry, no stop"],
            SP, claim("", "57%", "$212", "3 days", "S&P since 1982 (p.86)")),
        live!("lwlts_mon_vb", "Monday open + 5% of Friday's range, exit close", "Ch 6 · TDW / TDM", "Long",
            &["E|\"buying the opening Monday +.05 percent of Friday's range\" (p.87): Read: buy stop at the open + 0.05 x the previous bar's range",
              "X|\"we hold the trade to the close\" (p.87)",
              "X|Read: $3,000 stop (12 points); not in the text, Fig. 6.2's largest loss is exactly $3,000 with $0 commission"],
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
            &["E|\"buy the S&P 500 every day\" (p.95), one position at a time", STUDY_X],
            SP, claim("2,064", "52%", "$134", "2 days", "07/1982-02/1998")),
        live!("lwlts_study_down1", "Pattern study · after a down close", "Ch 7 · Patterns", "Long",
            &["F|\"only buy tomorrow if today closed down\" (p.95)", "E|Buy on the next open", STUDY_X, "X|Same $3,250 stop as the baseline (\"Now we add our first pattern\")"],
            SP, claim("1,334", "52%", "$212", "2 days", "07/1982-02/1998")),
        live!("lwlts_study_down3", "Pattern study · after three down closes", "Ch 7 · Patterns", "Long",
            &["F|\"three consecutive down closes\" (p.95)", "E|Buy on the next open", STUDY_X],
            SP, claim("248", "58%", "$353", "2 days", "07/1982-02/1998")),
        live!("lwlts_study_trend", "Pattern study · up 30 days, down 9 days", "Ch 7 · Patterns", "Long",
            &["F|\"today's price to be greater than the close 30 days ago ... today's close lower than the close 9 days ago\" (p.96)",
              "E|\"we will buy on the open tomorrow and exit on the next day's close\" (p.96)", "X|$3,250 stop as the baseline study (Read)"],
            SP, claim("354", "57%", "$421", "2 days", "07/1982-02/1998")),
        live!("lwlts_study_trend_mon", "Pattern study · up 30 / down 9, Mondays", "Ch 7 · Patterns", "Long",
            &["F|Same pattern, \"take these pattern trades on just Monday\" (p.96): the entry day is a Monday",
              "E|Buy on the open, exit on the next day's close", "X|$3,250 stop (Read)"],
            SP, claim("", "59%", "$672", "2 days", "07/1982-02/1998")),
        live!("lwlts_outside_down", "Outside day down close, lower open", "Ch 7 · Patterns", "Long",
            &["F|\"The day's high is greater than the previous day's high and the low is lower than the previous day's low and the close is below the previous day's low\" (p.96)",
              "E|\"tomorrow opens lower than the outside day's down close and we buy on the next day's opening\" (p.96)", STOP2000, BAIL],
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
            SP, claim("129", "85%", "$569.26", "1 day", "09/18/87-08/31/98")),
        // ---- Chapter 15 (p.233-243)
        live!("lwlts_hh1", "Buy a new 1-day high, exit 13 days later", "Ch 15 · Buy new highs", "Long", HH_R, HH_N, claim("209", "58%", "$511", "13 days", "S&P (window not printed)")),
        live!("lwlts_hh5", "Buy a new 5-day high, exit 13 days later", "Ch 15 · Buy new highs", "Long", HH_R, HH_N, claim("187", "51%", "$359", "13 days", "S&P (window not printed)")),
        live!("lwlts_hh10", "Buy a new 10-day high, exit 13 days later", "Ch 15 · Buy new highs", "Long", HH_R, HH_N, claim("169", "50%", "$344", "13 days", "S&P (window not printed)")),
        live!("lwlts_hh15", "Buy a new 15-day high, exit 13 days later", "Ch 15 · Buy new highs", "Long", HH_R, HH_N, claim("145", "56%", "$519", "13 days", "S&P (window not printed)")),
        live!("lwlts_hh20", "Buy a new 20-day high, exit 13 days later", "Ch 15 · Buy new highs", "Long", HH_R, HH_N, claim("136", "53%", "$406", "13 days", "S&P (window not printed)")),
        // ---- Needs data
        off!("lwlts_vb_bonds", "Bond volatility breakout 100% (Fig. 4.1, 4.7 TDW)", "Ch 4 · Volatility breakouts", "Long/Short", NeedsData(BONDS),
            &["E|Buy open + 100% of the previous day's range, sell open - 100% (p.61)", "X|\"$1,500 or 50 percent of the previous day's range\" stop, bailout (p.61)", "F|TDW: buys Tue/Thu, sells Wed/Thu (p.64)"], "T-Bonds 1990-08/1998: 651 trades 80% $112.86; TDW 84% $173."),
        off!("lwlts_talon_bondfilter", "S&P swing breakout with bond trend (Fig. 4.21, 'One step further')", "Ch 4 · Volatility breakouts", "Long/Short", NeedsData(BONDS),
            &["F|\"only take buy signals if the closing price of bonds is greater today than 5 days ago, and only take sells if bonds are lower than 35 days ago\" (p.72)", "E|Talon entries, $1,750 stop, bailout", "F|'One step further' adds the best TDWs (days not listed)"], "Avg $281, DD $5,250; with TDW 90%, $444/trade."),
        off!("lwlts_tdw_bonds", "Bonds Tuesday open + 70% of Monday's range (Fig. 6.3)", "Ch 6 · TDW / TDM", "Long", NeedsData(BONDS),
            &["E|\"Bonds make money buying on Tuesday opening plus 70 percent of Monday's range\" (p.87)"], "53%, $86/trade."),
        off!("lwlts_tdm_bonds", "Bonds best TDMs (Table 6.7)", "Ch 6 · TDW / TDM", "Long", NeedsData(BONDS),
            &["E|Buy the open of TDM 8, 18, 19, 20, 21, 22", "X|$1,500 stop from entry day, close of the third day after entry (p.88-89)"], "T-Bonds 1977-1998."),
        off!("lwlts_bond_patterns", "Bond outside-day, 3-up-day and Oops! patterns (Fig. 7.4, 7.5, 7.29, 7.30, 7.32, 7.33)", "Ch 7 · Patterns", "Long/Short", NeedsData(BONDS),
            &["E|Outside day down close + lower open, $1,500 stop, bailout (p.97)", "E|Oops! buys except Wednesday, $1,800 stop, bailout; Oops! sells Wednesday, $1,000 stop, 4-day bailout (p.116)", "E|Oops! with 9-day MA; exit \"the close on the first profitable opening after 3 days\" (p.118)"], "Bond futures results only."),
        off!("lwlts_gsv_bonds", "Greatest Swing Value · bonds (Fig. 8.2)", "Ch 8 · Greatest swing value", "Long/Short", NeedsData(BONDS),
            &["F|Buy: close < close 5 days ago, Tue/Wed/Fri; sell: close > close 6 days ago and Gold < Gold 20 days ago (p.123-124)", "E|Open +/- 180% of the 4-day average swing", "X|$1,600 stop, first profitable opening after 2 days"], "Needs T-Bond and Gold futures."),
        off!("lwlts_gsv_sp", "Greatest Swing Value · S&P with bond filter (Fig. 8.3)", "Ch 8 · Greatest swing value", "Long/Short", NeedsData(BONDS),
            &["E|180% of the 4-day average buy swing (highs minus opens) / sell swing (p.124)", "F|\"demanding Bonds close higher than 15 days ago for a buy and lower than 15 days ago for a sell\"; buys Mon/Tue/Wed, sells not Monday; close below (above) 6 days ago", "X|$2,500 stop and bailout"], "247 trades 67% $427.83 (Fig. 8.3). The bond filter is part of the printed result, so the system cannot run without bond data."),
        off!("lwlts_gsv_sp_monday", "GSV Monday buy after a down Friday (Fig. 8.4)", "Ch 8 · Greatest swing value", "Long", NeedsData(BONDS),
            &["F|Friday closes down and Bonds on Friday above 15 days ago (p.127)", "E|Buy Monday at the open + Friday's high minus Friday's open", "X|Bailout and $2,500 stop"], "161 trades 86% $354.58, 09/15/87-08/28/98."),
        off!("lwlts_willspread", "Will-Spread (Ch. 9)", "Ch 9 · Quote screen", "Long/Short", NeedsData("Needs 15/30-minute bars of the traded market and of T-Bonds/T-Bills (or Gold for bonds); intraday and bond data are not in the vault."),
            &["F|Spread = primary / secondary x 100; 5-period EMA minus 20-period EMA (S&P: 3 and 15) (p.138-141)", "E|Zero-line crossing, then the next bar takes out the crossing bar's high (low)"], "Intraday."),
        off!("lwlts_month_bonds", "Month-end bond trades and S&P first day with bond trend (Ch. 10)", "Ch 10 · Month-end", "Long/Short", NeedsData(BONDS),
            &["F|S&P first trading day only if Bonds closed higher than 30 days ago (p.149)", "E|Bonds: first TDM ($1,100 stop, bailout); TDM 18 and 22 ($1,500 stop, close 3 days after entry, Gold < 24 days ago); sell TDM 12 ($1,400, Gold > 10 days ago) (p.149-156)"], "Bond and gold futures."),
        off!("lwlts_bonds_channel", "S&P buys on bond channel breakouts (Fig. 15.1, 15.2)", "Ch 15 · Data A and data B", "Long", NeedsData(BONDS),
            &["E|Buy the S&P market on close when Bonds close above the highest high of the last 14 days (p.235)", "X|Lowest low of the last 17 days in Bonds, or a $3,000 stop; Fig. 15.2: lowest low of the last 12 days in the S&P"], "$141,792.50, avg $1,750."),
        off!("lwlts_bonds_8day", "Bond system: close above 8 days ago (Ch. 14, Aug 1997 letter)", "Ch 14 · Letters", "Long", NeedsData(BONDS),
            &["E|\"buy today's close if it was greater than the close 8 days ago\" (p.213)", "X|$850 stop; $3,500 profit target"], "$36,500, $160/trade."),
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
        "lwlts_vb_bonds" | "lwlts_talon_bondfilter" => f("lwlts_vb", "S&P volatility breakout", "Needs bond data"),
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
        "lwlts_tdm_bonds" => f("lwlts_tdm", "Trading day of month", "Bonds"),
        "lwlts_study_all" => f("lwlts_study", "Pattern studies", "Every day"),
        "lwlts_study_down1" => f("lwlts_study", "Pattern studies", "After a down close"),
        "lwlts_study_down3" => f("lwlts_study", "Pattern studies", "After 3 down closes"),
        "lwlts_study_trend" => f("lwlts_study", "Pattern studies", "Up 30 / down 9"),
        "lwlts_study_trend_mon" => f("lwlts_study", "Pattern studies", "Up 30 / down 9, Mondays"),
        "lwlts_outside_down" => f("lwlts_outside", "Outside day down close", "All days"),
        "lwlts_outside_down_xthu" => f("lwlts_outside", "Outside day down close", "Not Thursday"),
        "lwlts_3up_tue_sell" => f("lwlts_3up", "Third up day sell", "Tuesday setup"),
        "lwlts_oops_buy" => f("lwlts_oops", "Oops!", "Buy, not Wed/Thu"),
        "lwlts_oops_sell_thu" => f("lwlts_oops", "Oops!", "Sell Thursday"),
        "lwlts_oops_buy_ma9" => f("lwlts_oops", "Oops!", "Buy, 9-day MA"),
        "lwlts_oops_sell_ma9" => f("lwlts_oops", "Oops!", "Sell, 9-day MA"),
        "lwlts_oops_tdm18" => f("lwlts_oops", "Oops!", "After TDM 17"),
        "lwlts_bond_patterns" => f("lwlts_oops", "Oops!", "Bonds"),
        "lwlts_month_day1" => f("lwlts_month", "Month-end trades", "S&P first trading day"),
        "lwlts_month_bonds" | "lwlts_eom_sp" => f("lwlts_month", "Month-end trades", "Bonds / end of month"),
        "lwlts_hh1" => f("lwlts_hh", "Buy new X-day highs", "X = 1"),
        "lwlts_hh5" => f("lwlts_hh", "Buy new X-day highs", "X = 5"),
        "lwlts_hh10" => f("lwlts_hh", "Buy new X-day highs", "X = 10"),
        "lwlts_hh15" => f("lwlts_hh", "Buy new X-day highs", "X = 15"),
        "lwlts_hh20" => f("lwlts_hh", "Buy new X-day highs", "X = 20"),
        "lwlts_gsv_bonds" | "lwlts_gsv_sp" | "lwlts_gsv_sp_monday" => f("lwlts_gsv", "Greatest swing value", "Needs bond data"),
        "lwlts_willspread" => f("lwlts_willspread", "Will-Spread", ""),
        "lwlts_bonds_channel" | "lwlts_bonds_8day" | "lwlts_ma_cross_futures" => f("lwlts_other", "Other futures studies", "Needs futures data"),
        "lwlts_smash_day" | "lwlts_specialist_trap" | "lwlts_swing_points" => f("lwlts_patterns", "Chart patterns", "Not testable"),
        "lwlts_close_65pct" | "lwlts_sp_daytrade_stops" => f("lwlts_other", "Other studies", "Not testable"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let pt = || p("pt_usd", "Dollars per index point (book: S&P $250)", 250.0, 50.0, 500.0, 50.0);
    let stop = |d: f64| p("stop_usd", "Dollar stop (book $)", d, 0.0, 6000.0, 250.0);
    let day0 = |d: f64| p("stop_day0", "Stop armed on the entry day (1) or from the next day (0)", d, 0.0, 1.0, 1.0);
    let dows = |k, l, d| p(k, l, d, 0.0, 31.0, 1.0);
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
        "lwlts_mon_vb" => vec![p("buy_mult", "Buy: open + x Friday's range", 0.05, 0.0, 1.0, 0.05), stop(3000.0), day0(1.0), pt()],
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

/// Trading day of the month (1-based) for every bar.
fn tdm(x: &Ctx) -> Vec<usize> {
    let mut out = vec![0; x.r.len()];
    for i in 0..x.r.len() {
        out[i] = if i > 0 && x.r[i - 1].date[..7] == x.r[i].date[..7] { out[i - 1] + 1 } else { 1 };
    }
    out
}

fn dow_bit(date: &str) -> u32 {
    match dow(date) { d @ 1..=5 => 1 << (d - 1), _ => 0 }
}

// ------------------------------------------------------------------ path simulator

#[derive(Clone, Copy)]
pub(crate) enum Ent { Open, Stop(f64) }

pub(crate) struct Cfg {
    pub(crate) stop: f64,        // protective stop distance in points (NaN = none)
    pub(crate) stop_rng: f64,    // stop = x * previous bar's range (NaN = not used)
    pub(crate) day0: bool,       // stop armed on the entry bar
    pub(crate) target: f64,      // profit target distance in points (NaN = none)
    pub(crate) hold: Option<usize>, // exit on the close `hold` bars after the entry bar
    pub(crate) bailout: bool,    // exit on the first profitable opening after the entry bar
    pub(crate) reverse: bool,    // an opposite entry signal exits and reverses
}

struct Pos { dir: f64, i: usize, px: f64, stop: f64, target: f64 }

fn close_trade(x: &Ctx, p: &Pos, i: usize, px: f64, why: &str) -> Trade {
    let kind = if p.px == x.o[p.i] { "open" } else { "stop" };
    Trade { dir: p.dir, legs: vec![Leg { i: p.i, px: p.px, w: 1.0 }], exit_i: i, exit_px: px, reason: why.into(),
        entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}

/// Bar-by-bar simulation. `sig(i)` returns the (long, short) entry orders for bar i, decided from data before
/// bar i plus bar i's open where the book's rule reads the open.
pub(crate) fn simulate(x: &Ctx, from: usize, to: usize, cfg: &Cfg, sig: impl Fn(usize) -> (Option<Ent>, Option<Ent>)) -> Vec<Trade> {
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<Pos> = None;
    for i in from.max(1)..=to {
        let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        let open_pos = |dir: f64, px: f64| {
            let sd = if cfg.stop_rng.is_finite() { cfg.stop_rng * (x.h[i - 1] - x.l[i - 1]) } else { cfg.stop };
            Pos { dir, i, px, stop: if sd.is_finite() && sd > 0.0 { px - dir * sd } else { f64::NAN },
                target: if cfg.target.is_finite() && cfg.target > 0.0 { px + dir * cfg.target } else { f64::NAN } }
        };
        // enter `dir` at `px`: flat -> open; opposite and reverse -> close and open; otherwise ignored
        let enter = |pos: &mut Option<Pos>, out: &mut Vec<Trade>, dir: f64, px: f64| match pos.take() {
            None => *pos = Some(open_pos(dir, px)),
            Some(p) if p.dir != dir && cfg.reverse => { out.push(close_trade(x, &p, i, px, "reverse: opposite signal")); *pos = Some(open_pos(dir, px)); }
            Some(p) => *pos = Some(p),
        };
        // 1. at the open: exits of positions from earlier bars
        if let Some(p) = &pos {
            let why = if cfg.bailout && p.dir * (o - p.px) > 0.0 { Some("bailout: first profitable opening") }
                else if p.stop.is_finite() && p.dir * (o - p.stop) <= 0.0 { Some("stop (gap at the open)") }
                else if p.target.is_finite() && p.dir * (o - p.target) >= 0.0 { Some("target (gap at the open)") }
                else { None };
            if let Some(w) = why { out.push(close_trade(x, p, i, o, w)); pos = None; }
        }
        // 2. entry orders for this bar; market-on-open orders and stops gapped through fill at the open
        let (le, se) = sig(i);
        let mut used = [false, false]; // one fill per direction per bar
        for (k, e, dir) in [(0usize, le, 1.0), (1, se, -1.0)] {
            let at_open = match e { Some(Ent::Open) => true, Some(Ent::Stop(lv)) => dir * (o - lv) >= 0.0, None => false };
            if at_open { used[k] = true; enter(&mut pos, &mut out, dir, o); }
        }
        // 3. intrabar path O -> nearer extreme -> other extreme -> C (TradeStation default)
        let path = if h - o <= o - l { [o, h, l, c] } else { [o, l, h, c] };
        for s in 0..3 {
            let (mut cur, end) = (path[s], path[s + 1]);
            let up = end > cur;
            loop {
                let mut best: Option<(f64, u8)> = None; // 0 long entry, 1 short entry, 2 stop, 3 target
                let mut consider = |lv: f64, a: u8| {
                    let inside = lv.is_finite() && if up { lv > cur && lv <= end } else { lv < cur && lv >= end };
                    if inside && best.map_or(true, |(b, _)| if up { lv < b } else { lv > b }) { best = Some((lv, a)); }
                };
                if let Some(p) = &pos {
                    let against = (p.dir > 0.0) != up;
                    if against && (p.i < i || cfg.day0) { consider(p.stop, 2); }
                    if !against { consider(p.target, 3); }
                }
                let free = |dir: f64| pos.as_ref().map_or(true, |p| cfg.reverse && p.dir != dir);
                if up && !used[0] && free(1.0) { if let Some(Ent::Stop(lv)) = le { consider(lv, 0); } }
                if !up && !used[1] && free(-1.0) { if let Some(Ent::Stop(lv)) = se { consider(lv, 1); } }
                let Some((lv, a)) = best else { break };
                match a {
                    0 => { used[0] = true; enter(&mut pos, &mut out, 1.0, lv); }
                    1 => { used[1] = true; enter(&mut pos, &mut out, -1.0, lv); }
                    2 => { let p = pos.take().unwrap(); out.push(close_trade(x, &p, i, lv, "stop")); }
                    _ => { let p = pos.take().unwrap(); out.push(close_trade(x, &p, i, lv, "target")); }
                }
                cur = lv;
            }
        }
        // 4. at the close: time exits
        if let Some(p) = &pos {
            if cfg.hold.map_or(false, |n| i >= p.i + n) { out.push(close_trade(x, p, i, c, "time exit on the close")); pos = None; }
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
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let pts = |k: &str| { let v = g(k); if v.is_finite() && v > 0.0 { v / g("pt_usd") } else { f64::NAN } };
    let hold = g("hold");
    let mut cfg = Cfg { stop: pts("stop_usd"), stop_rng: f64::NAN, day0: g("stop_day0") != 0.0, target: pts("target_usd"),
        hold: hold.is_finite().then(|| hold.max(0.0) as usize), bailout: false, reverse: false };
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
        "lwlts_mon_vb" => {
            cfg.hold = Some(0);
            let m = g("buy_mult");
            simulate(x, from, to, &cfg, |i| if wd(i) == 1 { (Some(Ent::Stop(o[i] + m * (h[i - 1] - l[i - 1]))), None) } else { none })
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
