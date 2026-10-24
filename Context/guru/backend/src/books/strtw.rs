//! Thomas Stridsman, "Trading Systems That Work" (McGraw-Hill, 2000). Every Live strategy is the book's printed
//! TradeStation code (or a rule list the book gives with every number, coded with the book's own code template).
//! The book tested S&P 500 futures (RAD contract) and 14-16 futures markets: here one index/ETF at a time (instrument
//! swap), full equity per trade (= the book's percent-based trade results). Page refs, oracle and verification:
//! library/strtw/. Weekly/monthly closes (CloseW, CloseM, Data2 weekly bars) are built from the daily bars: a week is
//! Monday-Sunday, a month a calendar month, a weekly bar is complete on its last trading day.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "strtw";
/// Admitted ids (authors/stridsman/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["strtw_gd1", "strtw_gd2", "strtw_gd2_5d", "strtw_gd", "strtw_gd_bj", "strtw_meander1_long", "strtw_meander1_short", "strtw_meander_wk", "strtw_meander_wk_x", "strtw_meander_bj", "strtw_meander_bj_ma", "strtw_bj1", "strtw_bj_bj", "strtw_bj_obv", "strtw_dirslope", "strtw_dirslope_x", "strtw_dbs1a", "strtw_dbs1b", "strtw_dbs2a", "strtw_dbs2b", "strtw_dbs_x", "strtw_sdb", "strtw_sdb_x"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! sd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $vol:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: $vol, needs_vix: false,
            needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const SP: &str = "Book market: S&P 500 futures (RAD contract), run on the index (instrument swap). Fills: stop/limit at the level or at the open when the bar gaps through it; a stop and a target inside one bar -> stop first. Full equity per trade (the book reports percent per trade).";
const FUT: &str = "Book markets: 14-16 futures (point-based or ratio-adjusted continuous contracts), run on the index (instrument swap). Stop-and-reverse: entry stops are live while in a position, so a reversal exits and re-enters at the same price; at most one exit and one entry per bar; no exit on the entry bar. Full equity per trade.";
const BJX: &str = "Black Jack exits (ch.11 p.205-216): research code p.206-207 (\"If BarsSinceEntry >= 1 Then\" gate on the stops, so the first full bar after a close entry has no stop - the book's own TradeStation workaround) with the final values p.216: stop loss 1.1%, minimum profit 0.6% (once a close is 0.6% in profit, a stop at entry +0.6%), profit target 2.8% (limit), exit at the close once BarsSinceEntry >= 8.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        sd!("strtw_gd1", "Gold Digger I", "Ch 5 · Data mining", "Long/Short", Live, false, &[
            "E|Condition1 = CloseM(1) > C and CloseW(1) > C and C[1] > C; if Condition1 and MarketPosition = 0: Buy at open (next bar) (p.59)",
            "E|Condition2 = CloseM(2) < CloseM(1) and CloseM(1) < C and CloseW(2) < CloseW(1) and CloseW(1) < C and C[2] < C[1] and C[1] < C: Sell at open (p.59)",
            "X|If C[2] < C[1] and C[1] < C: ExitLong at close; if C[1] > C: ExitShort at close (p.59)",
        ], SP),
        sd!("strtw_gd2", "Gold Digger II", "Ch 5 · Data mining", "Long/Short", Live, false, &[
            "E|Condition1 = CloseW(2) > CloseW(1) and CloseW(1) > C and C[2] > C[1] and C[1] > C: Buy at open (p.61)",
            "E|Condition2 = CloseW(2) < CloseW(1) and CloseW(1) < C and C[2] < C[1] and C[1] < C: Sell at open (p.61-62)",
            "X|If C[2] < C[1] and C[1] < C: ExitLong at close; if C[1] > C: ExitShort at close (p.61-62)",
        ], SP),
        sd!("strtw_gd2_5d", "Gold Digger II · five-day version", "Ch 11 · Gold Digger", "Long/Short", Live, false, &[
            "E|Gold Digger II entries (p.191-192)",
            "X|If BarsSinceEntry >= 5: ExitLong / ExitShort at close (the only exit) (p.192)",
        ], SP),
        sd!("strtw_gd", "Gold Digger (five-day + stops)", "Ch 11 · Gold Digger", "Long/Short", Live, false, &[
            "E|Gold Digger II entries; exit at the close once BarsSinceEntry >= 5 (p.192)",
            "X|ExitLong tomorrow at 1.038 x EntryPrice limit; at 0.99 x EntryPrice stop; if Close > 1.014 x EntryPrice: stop at 1.014 x EntryPrice (shorts 0.962 / 1.01 / 0.986) (p.194-195)",
        ], SP),
        sd!("strtw_gd_bj", "Gold Digger entry / Black Jack exits", "Ch 11 · Black Jack exits", "Long/Short", Live, false, &[
            "E|Gold Digger II entries (p.217)", "X|Black Jack exits: 1.1% stop, 0.6% minimum profit, 2.8% target, out at the close of day 8 (p.216)",
        ], BJX),
        sd!("strtw_meander1_long", "Meander I (daily, long side)", "Ch 5 · Better use of your data", "Long", Live, false, &[
            "F|VS = (O, H, L, C of each of the last 5 bars - previous close) / previous close (20 points); VSLow = C x (1 + AvgVS - StdVS x VSStd), VSHigh = C x (1 + AvgVS + StdVS x VSStd); StdVS = SquareRoot(DiffVS / 20); VSStd(1); levels of the prior bar (p.64-66)",
            "E|=IF(Open < VSLow; Open; IF(Low < VSLow; VSLow; \"\")) - buy at the open if it is below VSLow, else at VSLow if the low trades below it (p.66)",
            "X|Risk level = entry x (1 - 0.75%); exit =IF(Low < risk; risk; IF(AND(High > VSHigh; Close < VSHigh); VSHigh; Close)) - all trades closed the same day (p.66-68)",
        ], "Book Table 5.7 (S&P 500 futures, Jan 1995-Oct 1999, 0.75% risk): long 522 trades, 52.30% winners, 0.09% per trade. Intraday on daily data with the book's own conservative ordering (the stop is assumed hit first)."),
        sd!("strtw_meander1_short", "Meander I (daily, short side)", "Ch 5 · Better use of your data", "Short", Live, false, &[
            "E|Reverse of the long formula: sell at the open if it is above VSHigh, else at VSHigh if the high trades above it (p.66)",
            "X|Risk level = entry x (1 + 0.75%); exit at the risk level if the high reaches it, else at VSLow if the low is below it and the close above it, else at the close (p.66-68)",
        ], "Book Table 5.7: short 459 trades, 43.79% winners, 0.03% per trade."),
        sd!("strtw_meander_wk", "Meander (weekly levels, daily trading)", "Ch 11 · Meander (weekly data)", "Long/Short", Live, false, &[
            "F|VS from the last 5 completed WEEKLY bars (Data2), VSStd(2): VSLow = C Data2 x (1 + (AvgVS - StdVS x 2)), VSHigh = C Data2 x (1 + (AvgVS + StdVS x 2)) (p.197-199)",
            "E|If MarketPosition = 0: Buy tomorrow at VSLow limit; Sell tomorrow at VSHigh limit (p.199)",
            "X|If BarsSinceEntry >= 5: ExitLong / ExitShort at close (p.199)",
        ], "Book Table 11.7 (S&P 500, Jan 1995-Oct 1999): 55 trades, 52.73%, 0.67% per trade. A weekly bar counts once its last trading day has closed (TradeStation Data2)."),
        sd!("strtw_meander_wk_x", "Meander weekly + stops", "Ch 11 · Meander (weekly data)", "Long/Short", Live, false, &[
            "E|strtw_meander_wk entries and five-day exit (p.199)",
            "X|ExitLong tomorrow at 1.032 x EntryPrice limit; 0.982 x EntryPrice stop; if Close > 1.0064 x EntryPrice: stop at 1.0064 x EntryPrice (shorts 0.968 / 1.018 / 0.9936) (p.201)",
        ], "Book Table 11.9 (1995-Oct 1999): 73 trades, 65.75%, 0.52%; Table 11.11 (1985-1994): 121 trades, 64.46%, 0.52%."),
        sd!("strtw_meander_bj", "Meander entry / Black Jack exits", "Ch 11 · Black Jack exits", "Long/Short", Live, false, &[
            "E|Meander weekly limit entries (p.199, 219)", "X|Black Jack exits (p.216)",
        ], BJX),
        sd!("strtw_meander_bj_ma", "Meander / Black Jack exits / MA-slope filter", "Ch 12 · Filtering", "Long/Short", Live, false, &[
            "F|Moving-average slope filter, 70 days (Table 12.4): long only if Average(Close, 70) > Average(Close, 70)[1]; short only if Average(Close, 70) < Average(Close, 70)[1] (filter code p.232)",
            "E|Meander weekly limit entries in the filter's direction (p.239)", "X|Black Jack exits (p.216, 239)",
        ], "Book Tables 12.11 (1985-1994: 60 trades, -0.07%) and 12.12 (1995-Oct 1999: 29 trades, 72.41%, 0.77%)."),
        sd!("strtw_bj1", "Black Jack I", "Ch 5 · Basic exit techniques", "Long/Short", Live, false, &[
            "F|Trigger = Average(C, 200) (p.72)",
            "E|Retracement: C > Trigger and (L < XAverage(L, 6) or L crosses above XAverage(L, 6)) and C crosses above XAverage(C, 6) and H < XAverage(H, 6): Buy at C; BSLevel = C (p.72) - shorts mirrored",
            "E|Breakout: C > Trigger and H crosses above XAverage(H, 6) and C crosses above XAverage(C, 6): Buy at C (p.73) - shorts mirrored",
            "X|BarNo 11: if BarsSinceEntry >= 11: ExitLong tomorrow at C Stop (time-based trailing stop, p.76)",
            "X|PT 9.5: ExitLong tomorrow at BSLevel x (1 + 9.5%) limit; MP 1: if C > BSLevel x 1.01: stop at BSLevel x 1.01 (p.73, 76-77)",
            "X|SL 1.5: stop loss at BSLevel x (1 - 1.5%) (p.77-78). The printed code has BSLevel x (1 + SL x 0.01) for longs (sign typo: a long stop loss above the entry); the text calls it a stop loss tested 0.5-2.5% (p.77) - minus used; p.sl_sign=1 gives the printed sign",
        ], "Book Table 5.14 (S&P 500 futures, Jan 1985-Oct 1999): 280 trades, 52.86% winners, avg 0.18%, PF 1.25, largest win 9.50%, largest loss -2.23%. Printed \"If TS <> 0\" (undeclared input) read as MP."),
        sd!("strtw_bj_bj", "Black Jack entry / Black Jack exits", "Ch 11 · Black Jack exits", "Long/Short", Live, false, &[
            "E|Black Jack I entries (200-day filter, retracement + breakout, at the close) (p.220)", "X|Black Jack exits (p.216)",
        ], "Book Tables 11.20 (1995-Oct 1999: 87 trades, 67.82%, 0.50%) and 11.21 (1985-1994: 202 trades, 48.02%, -0.08%)."),
        sd!("strtw_bj_obv", "Black Jack (OBV-slope filter) / Black Jack exits", "Ch 12 · Filtering", "Long/Short", Live, true, &[
            "F|The 200-day filter replaced by the OBV slope filter, 70 days (Table 12.4): long if Average(OBV, 70) > Average(OBV, 70)[1], short if < (p.232, 240)",
            "E|Black Jack retracement/breakout entries at the close", "X|Black Jack exits (p.216)",
        ], "Book Tables 12.13 (202 trades, 57.92%, 0.19%) and 12.14 (1995-Oct 1999: 92 trades, 65.22%, 0.46%). OBV starts at 0 on the first bar."),
        sd!("strtw_dirslope", "Directional slope 18/8 (weekly)", "Ch 7 · Moving averages", "Long/Short", Live, false, &[
            "F|Weekly bars: EntryVal = Average(Close, 18), ExitVal = Average(Close, 8); Condition1 = EntryVal > EntryVal[1]; Condition2 = ExitVal > ExitVal[1] (p.88, 92)",
            "E|If Condition1 and Condition2: Buy at Close; if both false: Sell at Close (weekly close) (p.88)",
            "X|If Condition2 = False: ExitLong at Close; if Condition2 = True: ExitShort at Close (p.88)",
        ], "Book Tables 7.2/7.3 (16 futures, weekly, 1979-Oct 1994 / Oct 1994-Oct 1999): S&P 500 PF 0.63, 33.33% winners / PF 0.72, 40.91%. Weekly close = the close of the week's last trading day."),
        sd!("strtw_dirslope_x", "Directional slope 18/12 with MAE/MFE exits (weekly)", "Ch 9 · Sweeney's MAE/MFE", "Long/Short", Live, false, &[
            "E|Directional slope with Entry MA 18, Exit MA 12 (weekly) (p.124, 159)",
            "X|ExitLong at EntryPrice x 1.60 Limit; if BarsSinceEntry >= 40: ExitLong at Close (p.160)",
            "X|If BarsSinceEntry >= 14: if Highest(High, BarsSinceEntry) x 0.96 < EntryPrice x (1 + 0.0074 x BarsSinceEntry): stop at Highest(High, BarsSinceEntry) x 0.96; ExitLong at EntryPrice Stop (p.160)",
            "X|ExitLong at EntryPrice x 0.96 Stop; shorts mirrored (0.40 limit, 1.04 stops) (p.160)",
        ], "Weekly bars; the weekly stop/limit orders are worked on the daily bars of the following week. Book Tables 9.8/9.9 (16 futures): S&P 500 PF 1.01 before, 0.67 after the exits. The book adjusted losses > 5% to 4% by hand in Excel (p.160-161); not done here."),
        sd!("strtw_dbs1a", "Dynamic Breakout System 1a", "Ch 7 · Dynamic breakout systems", "Long/Short", Live, false, &[
            "F|HistVol = StdDev(C, 30); DeltaHistVol = (HistVol - HistVol[1]) / HistVol; EntryLB = EntryLB[1] x (1 + DeltaHistVol), bounded 20..60, starting at 20; ExitLB = EntryLB x 0.5 (p.98)",
            "E|Buy Tomorrow at Highest(High, EntryLB) Stop; Sell Tomorrow at Lowest(Low, EntryLB) Stop (p.98)",
            "X|ExitLong Tomorrow at Lowest(Low, ExitLB) Stop; ExitShort Tomorrow at Highest(High, ExitLB) Stop (p.99)",
        ], FUT),
        sd!("strtw_dbs1b", "Dynamic Breakout System 1b", "Ch 7 · Dynamic breakout systems", "Long/Short", Live, false, &[
            "F|EntryLB as 1a (20..60); ExitLB = ExitLB[1] x (1 - DeltaHistVol), bounded 10..30 (p.101)",
            "E|Buy / Sell Tomorrow at Highest(High, EntryLB) / Lowest(Low, EntryLB) Stop (p.101)",
            "X|ExitLong / ExitShort Tomorrow at Lowest(Low, ExitLB) / Highest(High, ExitLB) Stop (p.101)",
        ], "Book Table 7.5 (16 futures Jan 1980-Oct 1999): S&P 500 PF 1.00, 139 trades, 36.69%; Table 7.7 (1980-1989) S&P PF 1.25, 56 trades, 39.29%; Table 7.8 (1990-Oct 1999) PF 0.92, 79 trades, 34.18%. Lookback lengths are fractional: Highest/Lowest use floor(length) bars (TradeStation For-loop). The recursion starts at the 60th bar with HistVol[1] = 0 (variable default)."),
        sd!("strtw_dbs2a", "Dynamic Breakout System 2a", "Ch 7 · Dynamic breakout systems", "Long/Short", Live, false, &[
            "F|VolMethod(1): HistVol = StdDev(C, 30) x VolFactor(0.5) (VolMethod 0: AvgTrueRange(30) x 0.5); HalfHistVol = HistVol x 0.5 (p.102)",
            "E|Buy Tomorrow at Highest(High, 20) + HistVol Stop; Sell at Lowest(Low, 20) - HistVol Stop (p.102)",
            "X|ExitLong at Lowest(Low, 20) - HalfHistVol Stop; ExitShort at Highest(High, 20) + HalfHistVol Stop (p.102)",
        ], FUT),
        sd!("strtw_dbs2b", "Dynamic Breakout System 2b", "Ch 7 · Dynamic breakout systems", "Long/Short", Live, false, &[
            "E|As 2a: Buy Tomorrow at Highest(High, 20) + HistVol Stop; Sell Tomorrow at Lowest(Low, 20) - HistVol Stop (p.102)",
            "F|As 2a, but \"change the plus sign to a minus sign when calculating the long exit, and the minus sign to a plus sign when calculating the short exit\" (p.102)",
            "X|ExitLong at Lowest(Low, 20) + HalfHistVol Stop; ExitShort at Highest(High, 20) - HalfHistVol Stop",
        ], FUT),
        sd!("strtw_dbs_x", "DBS 1b with MAE/MFE exits", "Ch 10 · Adding long-term exits", "Long/Short", Live, false, &[
            "E|DBS version 1b (p.168)",
            "X|\"Exit with a loss if the market moves the wrong way by 4%\"; \"small loss/breakeven ... at or after bar 30\"; \"trailing stop, after bar 30, if the market moves the wrong way by 4% or more and ends up below the regression line for the original average trade\" (y = 0.0023x, Fig. 10.1); \"limit order if the open profit exceeds 70%\"; \"profit if the trade lasts for more than 100 bars\" (p.172)",
            "X|Coded with the book's own exit template for the same rule list (directional slope code p.160): stop EntryPrice x 0.96; if BarsSinceEntry >= 30: stop at EntryPrice, and Highest(High, BarsSinceEntry) x 0.96 when that is below EntryPrice x (1 + 0.0023 x BarsSinceEntry); limit EntryPrice x 1.70; exit at the close when BarsSinceEntry > 100",
        ], "Book Tables 10.8/10.9: S&P 500 PF 0.80 before, 0.83 after the exits (16 futures, 1980-Oct 1999). The DBS breakout stops remain."),
        sd!("strtw_sdb", "Standard Deviation Breakout (long-term Bollinger)", "Ch 7 · Standard deviation breakout", "Long/Short", Live, false, &[
            "F|UpBand = XAverage(High, 60) + 2 x StdDev(High, 60); LoBand = XAverage(Low, 60) - 2 x StdDev(Low, 60) (p.112)",
            "E|Buy tomorrow at UpBand Stop; Sell tomorrow at LoBand Stop (p.112)",
            "X|ExitLong tomorrow at XAverage(Low, 60) Stop; ExitShort tomorrow at XAverage(High, 60) Stop (p.112)",
        ], "Book Table 7.9 (16 futures, Jan 1980-Dec 1992; no S&P 500): average PF 3.82, 51.74% winners."),
        sd!("strtw_sdb_x", "SDB with MAE/MFE exits", "Ch 10 · Adding long-term exits", "Long/Short", Live, false, &[
            "E|SDB system (p.177)",
            "X|\"Exit with a loss if the market moves the wrong way by 5.5%\"; breakeven at or after bar 40; trailing 5.5% after bar 40 below the regression line (y = 0.0018x, Fig. 10.2); \"limit order if the open profit exceeds 75%\"; \"profit if the trade lasts for more than 105 bars\" (p.181-182)",
            "X|Coded with the book's exit template (p.160) with these numbers",
        ], "Book Tables 10.18/10.19 (16 futures, no S&P 500)."),
        sd!("strtw_gd_ratio", "Gold Digger / T-bill ratio cross filter", "Ch 12 · Filtering", "Long/Short",
            NeedsData("T-bill price series (ratio of the market to the T-bill price, 70-day cross; Table 12.4) - no T-bill futures/price history in the vault"), false, &[
            "F|Ratio = market / T-bill price; long only if Ratio > Average(Ratio, 70) (p.231-232)", "E|Gold Digger entries, Black Jack exits (p.236)",
        ], "Book Tables 12.5/12.6."),
        sd!("strtw_gd_mod_break", "Gold Digger (modified) / breakout filter", "Ch 12 · Filtering", "Long/Short",
            NotTestable("the modified entry is only described as loosening the entry to \"only look at the latest week's market action\" (p.237) - the exact condition is not given"), false, &[
            "E|Gold Digger with a loosened weekly condition; 60-day breakout filter (p.237-238)",
        ], "Book Tables 12.7-12.10."),
        sd!("strtw_lt_vol_filters", "DBS / SDB / directional slope with ADX or true-range filters", "Ch 13 · Long-term volatility filters", "Long/Short",
            NotTestable("the filter acts on \"the true range [ADX] for the day of the breakout\" (p.241-242) - a stop entry happens inside that day, so whether the filter uses the breakout day (look-ahead) or the prior day is not stated; no code printed"), false, &[
            "F|Do not take the trade if the true range of the breakout day exceeds ATR(n) x multiplier, or ADX(n) is below the trigger (Table 13.1 settings) (p.241-246)",
        ], "Seen and listed."),
        sd!("strtw_random", "Random-entry studies (Black Jack exits, trend filters)", "Ch 11-12", "Long/Short",
            NotTestable("random-number entries (IntPortion(Random(5))); results are averages over 10-12 random runs per market"), false, &[
            "E|PositionGenerator = IntPortion(Random(5)): 3 = buy at close, 4 = sell at close (p.206)",
        ], "The derived Black Jack exits are Live in strtw_gd_bj / strtw_meander_bj / strtw_bj_bj."),
        sd!("strtw_money_mgmt", "Fixed fractional money management / portfolio strategies", "Part 5 · Ch 15", "Long/Short",
            NotTestable("portfolio money management (fixed fractional f across several market/system combinations traded together); the vault runs one market at a time"), false, &[
            "A|Fixed fractional sizing on the short-term and long-term strategies, portfolio composition (p.271-330)",
        ], "Position sizing, not entry/exit rules."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "strtw_gd1" => ("strtw_gd", "Gold Digger", "I (ch.5)"),
        "strtw_gd2" => ("strtw_gd", "Gold Digger", "II (ch.5)"),
        "strtw_gd2_5d" => ("strtw_gd", "Gold Digger", "II five-day (ch.11)"),
        "strtw_gd" => ("strtw_gd", "Gold Digger", "five-day + stops (ch.11)"),
        "strtw_gd_bj" => ("strtw_gd", "Gold Digger", "Black Jack exits (ch.11)"),
        "strtw_gd_ratio" => ("strtw_gd", "Gold Digger", "T-bill ratio filter (data missing)"),
        "strtw_gd_mod_break" => ("strtw_gd", "Gold Digger", "modified / breakout filter"),
        "strtw_meander1_long" => ("strtw_meander", "Meander", "I daily, long"),
        "strtw_meander1_short" => ("strtw_meander", "Meander", "I daily, short"),
        "strtw_meander_wk" => ("strtw_meander", "Meander", "weekly levels"),
        "strtw_meander_wk_x" => ("strtw_meander", "Meander", "weekly + stops"),
        "strtw_meander_bj" => ("strtw_meander", "Meander", "Black Jack exits"),
        "strtw_meander_bj_ma" => ("strtw_meander", "Meander", "Black Jack exits + MA slope filter"),
        "strtw_bj1" => ("strtw_bj", "Black Jack", "I (ch.5)"),
        "strtw_bj_bj" => ("strtw_bj", "Black Jack", "Black Jack exits (ch.11)"),
        "strtw_bj_obv" => ("strtw_bj", "Black Jack", "OBV slope filter (ch.12)"),
        "strtw_dirslope" => ("strtw_dirslope", "Directional slope (weekly)", "18/8"),
        "strtw_dirslope_x" => ("strtw_dirslope", "Directional slope (weekly)", "18/12 + exits"),
        "strtw_dbs1a" => ("strtw_dbs", "Dynamic Breakout System", "1a"),
        "strtw_dbs1b" => ("strtw_dbs", "Dynamic Breakout System", "1b"),
        "strtw_dbs2a" => ("strtw_dbs", "Dynamic Breakout System", "2a"),
        "strtw_dbs2b" => ("strtw_dbs", "Dynamic Breakout System", "2b"),
        "strtw_dbs_x" => ("strtw_dbs", "Dynamic Breakout System", "1b + exits"),
        "strtw_sdb" => ("strtw_sdb", "Standard Deviation Breakout", "original"),
        "strtw_sdb_x" => ("strtw_sdb", "Standard Deviation Breakout", "+ exits"),
        "strtw_lt_vol_filters" | "strtw_random" | "strtw_money_mgmt" => ("strtw_other", "Filters, random entries, money management", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    // stops gated by BarsSinceEntry >= 1 in the research code (p.206); the printed Meander code (p.287-288) has no gate
    let gate = if id.starts_with("strtw_meander") { 0. } else { 1. };
    let bj_exits = move || vec![p("sl", "Stop loss %", 1.1, 0.5, 3., 0.1), p("mp", "Minimum profit %", 0.6, 0., 2., 0.1),
        p("pt", "Profit target %", 2.8, 1., 6., 0.2), p("days", "Exit at close when BarsSinceEntry >=", 8., 2., 15., 1.),
        p("bse_gate", "Stops only once BarsSinceEntry >= (1 = p.206 code, 0 = p.287 code)", gate, 0., 1., 1.)];
    let meander = |std: f64| vec![p("vs_std", "VSStd (standard deviations)", std, 0.5, 3., 0.5)];
    let cat = |mut a: Vec<ParamDef>, b: Vec<ParamDef>| { a.extend(b); a };
    Some(match id {
        "strtw_gd1" | "strtw_gd2" => vec![],
        "strtw_gd2_5d" => vec![p("days", "Exit at close when BarsSinceEntry >=", 5., 1., 15., 1.)],
        "strtw_gd" => vec![p("days", "Exit at close when BarsSinceEntry >=", 5., 1., 15., 1.), p("pt", "Profit target %", 3.8, 1., 8., 0.2),
            p("sl", "Stop loss %", 1., 0.2, 4., 0.1), p("mp", "Profit protection %", 1.4, 0., 3., 0.1)],
        "strtw_gd_bj" | "strtw_bj_bj" | "strtw_meander_bj" => cat(if id == "strtw_meander_bj" { meander(2.) } else { vec![] }, bj_exits()),
        "strtw_meander_bj_ma" => cat(cat(meander(2.), bj_exits()), vec![p("ma", "MA slope filter length", 70., 20., 250., 10.)]),
        "strtw_bj_obv" => cat(bj_exits(), vec![p("obv_len", "OBV slope filter length", 70., 20., 250., 10.)]),
        "strtw_meander1_long" | "strtw_meander1_short" => cat(meander(1.), vec![p("risk", "Percentage risk", 0.75, 0.25, 3., 0.25)]),
        "strtw_meander_wk" => cat(meander(2.), vec![p("days", "Exit at close when BarsSinceEntry >=", 5., 1., 15., 1.)]),
        "strtw_meander_wk_x" => cat(meander(2.), vec![p("days", "Exit at close when BarsSinceEntry >=", 5., 1., 15., 1.),
            p("pt", "Profit target %", 3.2, 1., 8., 0.2), p("sl", "Stop loss %", 1.8, 0.2, 4., 0.1), p("mp", "Profit protection %", 0.64, 0., 3., 0.02)]),
        "strtw_bj1" => vec![p("bar_no", "BarNo (time-based trailing stop after)", 11., 2., 20., 1.), p("pt", "PT %", 9.5, 5., 15., 0.5),
            p("mp", "MP minimum profit %", 1., 0., 5., 0.5), p("sl", "SL stop loss %", 1.5, 0.5, 3., 0.25), p("trend", "Trend filter SMA", 200., 50., 300., 10.),
            p("sl_sign", "Stop-loss sign: 0 = below entry (text), 1 = printed code (above)", 0., 0., 1., 1.)],
        "strtw_dirslope" => vec![p("entry_ma", "EntryMA (weeks)", 18., 10., 20., 1.), p("exit_ma", "ExitMA (weeks)", 8., 5., 15., 1.)],
        "strtw_dirslope_x" => vec![p("entry_ma", "EntryMA (weeks)", 18., 10., 20., 1.), p("exit_ma", "ExitMA (weeks)", 12., 5., 15., 1.)],
        "strtw_dbs1a" => vec![p("max_lb", "MaxLB", 60., 30., 100., 5.), p("min_lb", "MinLB", 20., 10., 40., 5.)],
        "strtw_dbs1b" | "strtw_dbs_x" => vec![p("max_lb", "MaxEntryLB", 60., 30., 100., 5.), p("min_lb", "MinEntryLB", 20., 10., 40., 5.),
            p("max_xlb", "MaxExitLB", 30., 10., 60., 5.), p("min_xlb", "MinExitLB", 10., 5., 30., 5.)],
        "strtw_dbs2a" | "strtw_dbs2b" => vec![p("vol_method", "VolMethod (1 = StdDev, 0 = ATR)", 1., 0., 1., 1.), p("vol_factor", "VolFactor", 0.5, 0.1, 2., 0.1)],
        "strtw_sdb" | "strtw_sdb_x" => vec![p("band_len", "BandLen", 60., 20., 120., 5.)],
        _ => return None,
    })
}

// ---------------------------------------------------------------- helpers

fn stdev_pop(v: &[f64], n: usize) -> Vec<f64> {
    let m = sma(v, n);
    (0..v.len()).map(|i| if i + 1 < n || !m[i].is_finite() { f64::NAN } else {
        (v[i + 1 - n..=i].iter().map(|x| (x - m[i]).powi(2)).sum::<f64>() / n as f64).sqrt()
    }).collect()
}
/// TradeStation XAverage: alpha 2/(n+1), seeded with the first value.
fn xavg(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = Vec::with_capacity(v.len());
    for (i, &x) in v.iter().enumerate() { out.push(if i == 0 { x } else { out[i - 1] + a * (x - out[i - 1]) }); }
    out
}
fn hi(v: &[f64], i: usize, n: usize) -> f64 { if n == 0 || i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) } }
fn lo(v: &[f64], i: usize, n: usize) -> f64 { if n == 0 || i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().copied().fold(f64::INFINITY, f64::min) } }
fn atr_simple(x: &Ctx, n: usize) -> Vec<f64> { sma(&crate::connors::true_range(x.r), n) }
fn obv(x: &Ctx) -> Vec<f64> {
    let mut o = vec![0.; x.c.len()];
    for i in 1..o.len() {
        let v = x.r[i].volume.unwrap_or(0.);
        o[i] = o[i - 1] + if x.c[i] > x.c[i - 1] { v } else if x.c[i] < x.c[i - 1] { -v } else { 0. };
    }
    o
}

fn days_from_civil(date: &str) -> i64 {
    let f = |a: usize, b: usize| date.get(a..b).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let (y, m, d) = (f(0, 4), f(5, 7), f(8, 10));
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * m + 2) / 5 + d - 1 - 719468
}
/// Calendar periods: week (Monday-based) and month keys per bar, and for every bar the last close of the
/// previous 1 and 2 completed periods (CloseW(1), CloseW(2) / CloseM(1), CloseM(2)).
struct Periods { wk: Vec<i64>, w1: Vec<f64>, w2: Vec<f64>, m1: Vec<f64>, m2: Vec<f64> }
fn periods(x: &Ctx) -> Periods {
    let n = x.c.len();
    let wk: Vec<i64> = x.r.iter().map(|r| (days_from_civil(&r.date) + 3).div_euclid(7)).collect();
    let mo: Vec<i64> = x.r.iter().map(|r| r.date.get(0..4).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0) * 12
        + r.date.get(5..7).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0)).collect();
    let prev = |key: &[i64]| {
        let (mut a, mut b) = (vec![f64::NAN; n], vec![f64::NAN; n]);
        let (mut p1, mut p2) = (f64::NAN, f64::NAN);
        for i in 0..n {
            if i > 0 && key[i] != key[i - 1] { p2 = p1; p1 = x.c[i - 1]; }
            a[i] = p1; b[i] = p2;
        }
        (a, b)
    };
    let (w1, w2) = prev(&wk);
    let (m1, m2) = prev(&mo);
    Periods { wk, w1, w2, m1, m2 }
}
/// Completed weekly bars: (index of the week's last daily bar, open, high, low, close).
fn weekly_bars(x: &Ctx, wk: &[i64]) -> Vec<(usize, f64, f64, f64, f64)> {
    let mut out: Vec<(usize, f64, f64, f64, f64)> = vec![];
    let mut start = 0;
    for i in 0..x.c.len() {
        if i + 1 == x.c.len() || wk[i + 1] != wk[i] {
            let h = x.h[start..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let l = x.l[start..=i].iter().copied().fold(f64::INFINITY, f64::min);
            out.push((i, x.o[start], h, l, x.c[i]));
            start = i + 1;
        }
    }
    out
}

/// Meander VS levels from 5 bars (o,h,l,c) and their previous closes: (VSLow, VSHigh).
fn vs_levels(b: &[(f64, f64, f64, f64)], prev: &[f64], base: f64, k: f64) -> (f64, f64) {
    let mut d = Vec::with_capacity(20);
    for (j, &(o, h, l, c)) in b.iter().enumerate() { for v in [o, h, l, c] { d.push((v - prev[j]) / prev[j]); } }
    let avg = d.iter().sum::<f64>() / 20.;
    let sd = (d.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / 20.).sqrt();
    (base * (1. + (avg - sd * k)), base * (1. + (avg + sd * k)))
}

// ---------------------------------------------------------------- generic next-bar-order simulator

#[derive(Clone, Copy)]
enum Ord { Mkt, Stop(f64), Limit(f64), Close }
struct Pos { t: Trade, bse: usize, ep: f64 }
#[derive(Default)]
struct Ex { stop: Option<f64>, limit: Option<f64> }

fn open_trade(dir: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn close_trade(mut t: Trade, i: usize, px: f64, why: &str) -> Trade { t.exit_i = i; t.exit_px = px; t.reason = why.into(); t.signal_i = i; t }
fn fill_entry(x: &Ctx, i: usize, dir: f64, o: Ord) -> Option<f64> {
    match o {
        Ord::Mkt => Some(x.o[i]),
        Ord::Close => None,
        Ord::Stop(s) | Ord::Limit(s) if !s.is_finite() => None,
        Ord::Stop(s) => if dir > 0. { (x.h[i] >= s).then(|| x.o[i].max(s)) } else { (x.l[i] <= s).then(|| x.o[i].min(s)) },
        Ord::Limit(s) => if dir > 0. { (x.l[i] <= s).then(|| x.o[i].min(s)) } else { (x.h[i] >= s).then(|| x.o[i].max(s)) },
    }
}
fn fill_exit(x: &Ctx, i: usize, dir: f64, e: &Ex) -> Option<(f64, &'static str)> {
    let against = |a: f64, b: f64| if dir > 0. { a <= b } else { a >= b };
    let (o, adv, fav) = if dir > 0. { (x.o[i], x.l[i], x.h[i]) } else { (x.o[i], x.h[i], x.l[i]) };
    let s = e.stop.filter(|v| v.is_finite());
    let t = e.limit.filter(|v| v.is_finite());
    if let Some(s) = s { if against(o, s) { return Some((o, "stop")); } }
    if let Some(t) = t { if against(t, o) { return Some((o, "target")); } }
    if let Some(s) = s { if against(adv, s) { return Some((s, "stop")); } }
    if let Some(t) = t { if against(t, fav) { return Some((t, "target")); } }
    None
}
fn tighter(dir: f64, a: Option<f64>, b: f64) -> Option<f64> {
    if !b.is_finite() { return a; }
    Some(a.map_or(b, |a| if dir > 0. { a.max(b) } else { a.min(b) }))
}

/// TradeStation next-bar orders: at the close of bar i, entry orders (flat) or exit orders (in a position) for bar i+1;
/// "at close" entries fill on bar i itself; "at close" exits (on_close) act on bar i (not on the bar of a close entry).
fn sim(x: &Ctx, from: usize, to: usize,
       mut entry: impl FnMut(usize) -> Vec<(f64, Ord)>,
       mut on_close: impl FnMut(usize, &Pos) -> Option<&'static str>,
       mut exits: impl FnMut(usize, &Pos) -> Ex) -> Vec<Trade> {
    let mut out = vec![];
    let (mut pend_e, mut pend_x): (Vec<(f64, Ord)>, Ex) = (vec![], Ex::default());
    let mut pos: Option<Pos> = None;
    for i in from..=to {
        let mut entered_at_close = false;
        if let Some(p) = pos.as_mut() {
            p.bse += 1;
            if let Some((px, why)) = fill_exit(x, i, p.t.dir, &pend_x) {
                out.push(close_trade(pos.take().unwrap().t, i, px, why));
            }
        } else if !pend_e.is_empty() {
            let mut best: Option<(f64, f64, f64, &'static str)> = None;
            for &(dir, o) in &pend_e {
                if let Some(px) = fill_entry(x, i, dir, o) {
                    let d = if matches!(o, Ord::Mkt) { -1. } else { (px - x.o[i]).abs() };
                    let kind = match o { Ord::Mkt => "open", Ord::Stop(_) => "stop", _ => "limit" };
                    if best.map_or(true, |b| d < b.2) { best = Some((dir, px, d, kind)); }
                }
            }
            if let Some((dir, px, _, kind)) = best { pos = Some(Pos { t: open_trade(dir, i, px, kind, i - 1), bse: 0, ep: px }); }
        }
        pend_e.clear();
        pend_x = Ex::default();
        if pos.is_none() && i < to {
            let orders = entry(i);
            if let Some(&(dir, _)) = orders.iter().find(|o| matches!(o.1, Ord::Close)) {
                pos = Some(Pos { t: open_trade(dir, i, x.c[i], "close", i), bse: 0, ep: x.c[i] });
                entered_at_close = true;
            } else { pend_e = orders; continue; }
        }
        if let Some(p) = pos.as_ref() {
            if !entered_at_close {
                if let Some(why) = on_close(i, p) { out.push(close_trade(pos.take().unwrap().t, i, x.c[i], why)); continue; }
            }
            if i < to { pend_x = exits(i, p); }
        }
    }
    if let Some(p) = pos { let mut t = close_trade(p.t, to, x.c[to], "open"); t.open = true; out.push(t); }
    out
}

/// Stop-and-reverse simulator for the breakout systems: every bar a buy stop, a sell stop and the two exit stops are
/// live (TradeStation places them every bar). Extra exits (stop/limit for the open position, exit at the close) come
/// from `extra`. At most one exit and one entry per bar; exits are not checked on the entry bar.
fn rev_sim(x: &Ctx, from: usize, to: usize,
           mut levels: impl FnMut(usize) -> (f64, f64, f64, f64),
           mut extra: impl FnMut(usize, &Pos) -> (Ex, bool)) -> Vec<Trade> {
    let mut out = vec![];
    let mut pos: Option<Pos> = None;
    let mut lv = (f64::NAN, f64::NAN, f64::NAN, f64::NAN);
    let mut ex = Ex::default();
    for i in from..=to {
        let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
        let (buy, sell, xl, xs) = lv;
        let mut entered = false;
        if let Some(p) = pos.as_mut() {
            p.bse += 1;
            let d = p.t.dir;
            let stop = tighter(d, ex.stop, if d > 0. { xl } else { xs });
            let rev = if d > 0. { sell } else { buy };
            let against = |a: f64, b: f64| if d > 0. { a <= b } else { a >= b };
            let rev = Some(rev).filter(|v| v.is_finite());
            // the first adverse level reached when the price moves against the position
            let first = [stop, rev].into_iter().flatten().fold(None, |acc: Option<f64>, v| tighter(d, acc, v));
            let adv = if d > 0. { l } else { h };
            let tgt = ex.limit.filter(|v| v.is_finite());
            // (exit price, why, reversal price)
            let mut hit: Option<(f64, &'static str, Option<f64>)> = None;
            if let Some(f) = first {
                if against(o, f) {
                    let r = rev.and_then(|r| if against(o, r) { Some(o) } else if against(adv, r) { Some(r) } else { None });
                    hit = Some((o, "stop", r));
                }
            }
            if hit.is_none() { if let Some(t) = tgt { if against(t, o) { hit = Some((o, "target", None)); } } }
            if hit.is_none() {
                if let Some(f) = first {
                    if against(adv, f) {
                        let r = rev.and_then(|r| if against(adv, r) { Some(r) } else { None });
                        hit = Some((f, "stop", r));
                    }
                }
            }
            if hit.is_none() { if let Some(t) = tgt { if against(t, if d > 0. { h } else { l }) { hit = Some((t, "target", None)); } } }
            if let Some((px, why, r)) = hit {
                let t = pos.take().unwrap().t;
                let reversal_same = r.map_or(false, |r| (r - px).abs() < 1e-12);
                out.push(close_trade(t, i, px, if reversal_same { "reversal" } else { why }));
                if let Some(rpx) = r { pos = Some(Pos { t: open_trade(-d, i, rpx, "stop", i - 1), bse: 0, ep: rpx }); entered = true; }
            }
        } else {
            let lb = fill_entry(x, i, 1., Ord::Stop(buy));
            let ls = fill_entry(x, i, -1., Ord::Stop(sell));
            let pick = match (lb, ls) {
                (Some(b), Some(s)) => Some(if (b - o).abs() <= (s - o).abs() { (1., b) } else { (-1., s) }),
                (Some(b), None) => Some((1., b)),
                (None, Some(s)) => Some((-1., s)),
                _ => None,
            };
            if let Some((d, px)) = pick { pos = Some(Pos { t: open_trade(d, i, px, "stop", i - 1), bse: 0, ep: px }); entered = true; }
        }
        let _ = entered;
        ex = Ex::default();
        if let Some(p) = pos.as_ref() {
            let (e, close_exit) = extra(i, p);
            if close_exit && p.bse > 0 { out.push(close_trade(pos.take().unwrap().t, i, x.c[i], "time (close)")); }
            else { ex = e; }
        }
        lv = if i < to { levels(i) } else { (f64::NAN, f64::NAN, f64::NAN, f64::NAN) };
    }
    if let Some(p) = pos { let mut t = close_trade(p.t, to, x.c[to], "open"); t.open = true; out.push(t); }
    out
}

/// The book's MAE/MFE exit template (p.160) with a chapter's numbers.
fn template_exits(x: &Ctx, i: usize, p: &Pos, loss: f64, after: usize, slope: f64, target: f64) -> Ex {
    let (d, ep, n) = (p.t.dir, p.ep, p.bse);
    let mut e = Ex { stop: Some(ep * (1. - d * loss)), limit: Some(ep * (1. + d * target)) };
    if n >= after {
        e.stop = tighter(d, e.stop, ep);
        if d > 0 as f64 {
            let tr = hi(&x.h, i, n) * (1. - loss);
            if tr < ep * (1. + slope * n as f64) { e.stop = tighter(d, e.stop, tr); }
        } else {
            let tr = lo(&x.l, i, n) * (1. + loss);
            if tr > ep * (1. - slope * n as f64) { e.stop = tighter(d, e.stop, tr); }
        }
    }
    e
}

/// Black Jack exits (p.206-207, final values p.216).
fn bj_exits(c: f64, p: &Pos, sl: f64, mp: f64, pt: f64, gate: usize) -> Ex {
    if p.bse < gate { return Ex::default(); }
    let (d, ep) = (p.t.dir, p.ep);
    let mut e = Ex { stop: Some(ep * (1. - d * sl)), limit: Some(ep * (1. + d * pt)) };
    if mp > 0. && d * (c - ep * (1. + d * mp)) > 0. { e.stop = tighter(d, e.stop, ep * (1. + d * mp)); }
    e
}

fn gd2_entry<'a>(x: &'a Ctx<'a>, pr: &'a Periods) -> impl FnMut(usize) -> Vec<(f64, Ord)> + 'a {
    move |i| {
        if i < 2 { return vec![]; }
        let c = &x.c;
        if pr.w2[i] > pr.w1[i] && pr.w1[i] > c[i] && c[i - 2] > c[i - 1] && c[i - 1] > c[i] { return vec![(1., Ord::Mkt)]; }
        if pr.w2[i] < pr.w1[i] && pr.w1[i] < c[i] && c[i - 2] < c[i - 1] && c[i - 1] < c[i] { return vec![(-1., Ord::Mkt)]; }
        vec![]
    }
}

/// Meander VS levels from the last 5 completed weekly bars as of each day (Data2).
fn weekly_vs(x: &Ctx, wk: &[i64], k: f64) -> Vec<(f64, f64)> {
    let wb = weekly_bars(x, wk);
    let mut out = vec![(f64::NAN, f64::NAN); x.c.len()];
    let mut j = 0; // number of weekly bars complete as of day i
    for i in 0..x.c.len() {
        while j < wb.len() && wb[j].0 <= i { j += 1; }
        if j >= 6 {
            let bars: Vec<(f64, f64, f64, f64)> = (0..5).map(|s| { let w = wb[j - 1 - s]; (w.1, w.2, w.3, w.4) }).collect();
            let prev: Vec<f64> = (0..5).map(|s| wb[j - 2 - s].4).collect();
            out[i] = vs_levels(&bars, &prev, wb[j - 1].4, k);
        }
    }
    out
}

fn bj_entry<'a>(x: &'a Ctx<'a>, trend: Vec<i8>) -> impl FnMut(usize) -> Vec<(f64, Ord)> + 'a {
    let (xh, xl, xc) = (xavg(&x.h, 6), xavg(&x.l, 6), xavg(&x.c, 6));
    move |i| {
        if i < 1 { return vec![]; }
        let (h, l, c) = (x.h[i], x.l[i], x.c[i]);
        let up = |v: &[f64], a: &[f64]| v[i] > a[i] && v[i - 1] <= a[i - 1];
        let dn = |v: &[f64], a: &[f64]| v[i] < a[i] && v[i - 1] >= a[i - 1];
        if trend[i] > 0 && (((l < xl[i] || up(&x.l, &xl)) && up(&x.c, &xc) && h < xh[i]) || (up(&x.h, &xh) && up(&x.c, &xc))) { return vec![(1., Ord::Close)]; }
        if trend[i] < 0 && (((h > xh[i] || dn(&x.h, &xh)) && dn(&x.c, &xc) && l > xl[i]) || (dn(&x.l, &xl) && dn(&x.c, &xc))) { return vec![(-1., Ord::Close)]; }
        let _ = c;
        vec![]
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let c = &x.c;
    let no_close = |_: usize, _: &Pos| None;
    let pr = periods(x);
    Some(match id {
        "strtw_gd1" => sim(x, from, to, |i| {
                if i < 2 { return vec![]; }
                if pr.m1[i] > c[i] && pr.w1[i] > c[i] && c[i - 1] > c[i] { return vec![(1., Ord::Mkt)]; }
                if pr.m2[i] < pr.m1[i] && pr.m1[i] < c[i] && pr.w2[i] < pr.w1[i] && pr.w1[i] < c[i] && c[i - 2] < c[i - 1] && c[i - 1] < c[i] { return vec![(-1., Ord::Mkt)]; }
                vec![]
            },
            |i, p| if p.t.dir > 0. { (c[i - 2] < c[i - 1] && c[i - 1] < c[i]).then_some("two up closes") } else { (c[i - 1] > c[i]).then_some("down close") },
            |_, _| Ex::default()),
        "strtw_gd2" => sim(x, from, to, gd2_entry(x, &pr),
            |i, p| if p.t.dir > 0. { (c[i - 2] < c[i - 1] && c[i - 1] < c[i]).then_some("two up closes") } else { (c[i - 1] > c[i]).then_some("down close") },
            |_, _| Ex::default()),
        "strtw_gd2_5d" => { let days = u("days"); sim(x, from, to, gd2_entry(x, &pr), |_, p| (p.bse >= days).then_some("time (close)"), |_, _| Ex::default()) }
        "strtw_gd" => {
            let (days, pt, sl, mp) = (u("days"), g("pt") / 100., g("sl") / 100., g("mp") / 100.);
            sim(x, from, to, gd2_entry(x, &pr), |_, p| (p.bse >= days).then_some("time (close)"), |i, p| {
                let (d, ep) = (p.t.dir, p.ep);
                let mut e = Ex { stop: Some(ep * (1. - d * sl)), limit: Some(ep * (1. + d * pt)) };
                if mp > 0. && d * (c[i] - ep * (1. + d * mp)) > 0. { e.stop = tighter(d, e.stop, ep * (1. + d * mp)); }
                e
            })
        }
        "strtw_gd_bj" => {
            let (sl, mp, pt, days, gate) = (g("sl") / 100., g("mp") / 100., g("pt") / 100., u("days"), g("bse_gate").round().max(0.) as usize);
            sim(x, from, to, gd2_entry(x, &pr), |_, p| (p.bse >= days).then_some("time (close)"), |i, p| bj_exits(c[i], p, sl, mp, pt, gate))
        }
        "strtw_meander1_long" | "strtw_meander1_short" => {
            let (k, risk) = (g("vs_std"), g("risk") / 100.);
            let long = id.ends_with("long");
            let mut out = vec![];
            for i in from.max(6)..=to {
                let j = i - 1; // levels from the prior bar
                let bars: Vec<(f64, f64, f64, f64)> = (0..5).map(|s| (x.o[j - s], x.h[j - s], x.l[j - s], c[j - s])).collect();
                let prev: Vec<f64> = (0..5).map(|s| c[j - s - 1]).collect();
                let (vlo, vhi) = vs_levels(&bars, &prev, c[j], k);
                let (o, h, l, cl) = (x.o[i], x.h[i], x.l[i], c[i]);
                let (entry, exit, why) = if long {
                    let e = if o < vlo { o } else if l < vlo { vlo } else { continue };
                    let r = e * (1. - risk);
                    if l < r { (e, r, "stop") } else if h > vhi && cl < vhi { (e, vhi, "VS High") } else { (e, cl, "close") }
                } else {
                    let e = if o > vhi { o } else if h > vhi { vhi } else { continue };
                    let r = e * (1. + risk);
                    if h > r { (e, r, "stop") } else if l < vlo && cl > vlo { (e, vlo, "VS Low") } else { (e, cl, "close") }
                };
                let mut t = open_trade(if long { 1. } else { -1. }, i, entry, if entry == o { "open" } else { "limit" }, j);
                t = close_trade(t, i, exit, why);
                out.push(t);
            }
            out
        }
        "strtw_meander_wk" | "strtw_meander_wk_x" | "strtw_meander_bj" | "strtw_meander_bj_ma" => {
            let vs = weekly_vs(x, &pr.wk, g("vs_std"));
            let ma = if id == "strtw_meander_bj_ma" { Some(sma(c, u("ma"))) } else { None };
            let entry = |i: usize| {
                let (lo_, hi_) = vs[i];
                if !lo_.is_finite() { return vec![]; }
                let (mut l_ok, mut s_ok) = (true, true);
                if let Some(m) = &ma { if i < 1 { return vec![]; } l_ok = m[i] > m[i - 1]; s_ok = m[i] < m[i - 1]; }
                let mut v = vec![];
                if l_ok { v.push((1., Ord::Limit(lo_))); }
                if s_ok { v.push((-1., Ord::Limit(hi_))); }
                v
            };
            match id {
                "strtw_meander_wk" => { let days = u("days"); sim(x, from, to, entry, |_, p| (p.bse >= days).then_some("time (close)"), |_, _| Ex::default()) }
                "strtw_meander_wk_x" => {
                    let (days, pt, sl, mp) = (u("days"), g("pt") / 100., g("sl") / 100., g("mp") / 100.);
                    sim(x, from, to, entry, |_, p| (p.bse >= days).then_some("time (close)"), |i, p| {
                        let (d, ep) = (p.t.dir, p.ep);
                        let mut e = Ex { stop: Some(ep * (1. - d * sl)), limit: Some(ep * (1. + d * pt)) };
                        if mp > 0. && d * (c[i] - ep * (1. + d * mp)) > 0. { e.stop = tighter(d, e.stop, ep * (1. + d * mp)); }
                        e
                    })
                }
                _ => {
                    let (sl, mp, pt, days, gate) = (g("sl") / 100., g("mp") / 100., g("pt") / 100., u("days"), g("bse_gate").round().max(0.) as usize);
                    sim(x, from, to, entry, |_, p| (p.bse >= days).then_some("time (close)"), |i, p| bj_exits(c[i], p, sl, mp, pt, gate))
                }
            }
        }
        "strtw_bj1" | "strtw_bj_bj" | "strtw_bj_obv" => {
            let trend: Vec<i8> = if id == "strtw_bj_obv" {
                let a = sma(&obv(x), u("obv_len"));
                (0..c.len()).map(|i| if i > 0 && a[i] > a[i - 1] { 1 } else if i > 0 && a[i] < a[i - 1] { -1 } else { 0 }).collect()
            } else {
                let m = sma(c, if id == "strtw_bj1" { u("trend") } else { 200 });
                (0..c.len()).map(|i| if c[i] > m[i] { 1 } else if c[i] < m[i] { -1 } else { 0 }).collect()
            };
            if id == "strtw_bj1" {
                let (bar_no, pt, mp, sl) = (u("bar_no"), g("pt") / 100., g("mp") / 100., g("sl") / 100.);
                let sign = if g("sl_sign") > 0.5 { -1. } else { 1. };
                sim(x, from, to, bj_entry(x, trend), no_close, |i, p| {
                    let (d, bs) = (p.t.dir, p.ep);
                    let mut e = Ex { stop: None, limit: Some(bs * (1. + d * pt)) };
                    // printed code: long stop BSLevel x (1 + SL), short (1 - SL); sign 1 = text reading (below a long entry)
                    e.stop = tighter(d, e.stop, bs * (1. - d * sign * sl));
                    if mp > 0. && d * (c[i] - bs * (1. + d * mp)) > 0. { e.stop = tighter(d, e.stop, bs * (1. + d * mp)); }
                    if p.bse >= bar_no { e.stop = tighter(d, e.stop, c[i]); }
                    e
                })
            } else {
                let (sl, mp, pt, days, gate) = (g("sl") / 100., g("mp") / 100., g("pt") / 100., u("days"), g("bse_gate").round().max(0.) as usize);
                sim(x, from, to, bj_entry(x, trend), |_, p| (p.bse >= days).then_some("time (close)"), |i, p| bj_exits(c[i], p, sl, mp, pt, gate))
            }
        }
        "strtw_dirslope" | "strtw_dirslope_x" => {
            let wb = weekly_bars(x, &pr.wk);
            let wc: Vec<f64> = wb.iter().map(|w| w.4).collect();
            let (ea, xa) = (sma(&wc, u("entry_ma")), sma(&wc, u("exit_ma")));
            let exits_on = id == "strtw_dirslope_x";
            let wh: Vec<f64> = wb.iter().map(|w| w.2).collect();
            let wl: Vec<f64> = wb.iter().map(|w| w.3).collect();
            let mut out = vec![];
            // position: (trade, entry week index), pending daily stop/limit for the current week
            let mut pos: Option<(Trade, usize)> = None;
            let mut ex = Ex::default();
            let mut w = 0usize; // index of the week bar i belongs to
            for i in 0..=to {
                while w < wb.len() && wb[w].0 < i { w += 1; }
                if i < from { continue; }
                if let Some((t, _)) = pos.as_ref() {
                    if t.legs[0].i < i { // orders are worked from the day after the weekly close
                        if let Some((px, why)) = fill_exit(x, i, t.dir, &ex) {
                            out.push(close_trade(pos.take().unwrap().0, i, px, why));
                            ex = Ex::default();
                        }
                    }
                }
                if w >= wb.len() || wb[w].0 != i { continue; } // decisions only at the weekly close
                if w < 1 || !ea[w].is_finite() || !ea[w - 1].is_finite() || !xa[w - 1].is_finite() { continue; }
                let (c1, c2) = (ea[w] > ea[w - 1], xa[w] > xa[w - 1]);
                let mut flat_now = pos.is_none();
                if let Some((t, ew)) = pos.as_ref() {
                    let bse = w - ew;
                    let exit = (t.dir > 0. && !c2) || (t.dir < 0. && c2) || (exits_on && bse >= 40);
                    if exit { out.push(close_trade(pos.take().unwrap().0, i, c[i], if bse >= 40 && exits_on { "40 bars (close)" } else { "exit MA slope (close)" })); flat_now = true; }
                }
                if pos.is_none() && i < to {
                    let d = if c1 && c2 { 1. } else if !c1 && !c2 { -1. } else { 0. };
                    // a same-close reversal is allowed (Buy/Sell at Close reverse the position)
                    if d != 0. { pos = Some((open_trade(d, i, c[i], "close", i), w)); }
                    let _ = flat_now;
                }
                ex = Ex::default();
                if exits_on && i < to {
                    if let Some((t, ew)) = pos.as_ref() {
                        let (d, ep, n) = (t.dir, t.legs[0].px, w - ew);
                        let mut e = Ex { stop: Some(ep * (1. - d * 0.04)), limit: Some(ep * (1. + d * 0.60)) };
                        if n >= 14 {
                            e.stop = tighter(d, e.stop, ep);
                            if d > 0. { let tr = hi(&wh, w, n) * 0.96; if tr < ep * (1. + 0.0074 * n as f64) { e.stop = tighter(d, e.stop, tr); } }
                            else { let tr = lo(&wl, w, n) * 1.04; if tr > ep * (1. - 0.0074 * n as f64) { e.stop = tighter(d, e.stop, tr); } }
                        }
                        ex = e;
                    }
                }
            }
            if let Some((t, _)) = pos { let mut t = close_trade(t, to, c[to], "open"); t.open = true; out.push(t); }
            out
        }
        "strtw_dbs1a" | "strtw_dbs1b" | "strtw_dbs_x" => {
            let v1b = id != "strtw_dbs1a";
            let hv = stdev_pop(c, 30);
            let (max_lb, min_lb) = (g("max_lb"), g("min_lb"));
            let (max_x, min_x) = if v1b { (g("max_xlb"), g("min_xlb")) } else { (0., 0.) };
            let n = c.len();
            let (mut elb, mut xlb) = (vec![f64::NAN; n], vec![f64::NAN; n]);
            let start = (max_lb.ceil() as usize).max(30);
            let (mut yhv, mut e, mut xl) = (0., 20., 0.);
            for i in start..n {
                let d = (hv[i] - yhv) / hv[i];
                yhv = hv[i];
                e = (e * (1. + d)).max(min_lb).min(max_lb);
                xl = if v1b { (xl * (1. - d)).min(max_x).max(min_x) } else { e * 0.5 };
                elb[i] = e; xlb[i] = xl;
            }
            let levels = |i: usize| {
                if !elb[i].is_finite() { return (f64::NAN, f64::NAN, f64::NAN, f64::NAN); }
                let (en, xn) = (elb[i].floor() as usize, xlb[i].floor() as usize);
                (hi(&x.h, i, en), lo(&x.l, i, en), lo(&x.l, i, xn), hi(&x.h, i, xn))
            };
            if id == "strtw_dbs_x" { rev_sim(x, from, to, levels, |i, p| (template_exits(x, i, p, 0.04, 30, 0.0023, 0.70), p.bse > 100)) }
            else { rev_sim(x, from, to, levels, |_, _| (Ex::default(), false)) }
        }
        "strtw_dbs2a" | "strtw_dbs2b" => {
            let vf = g("vol_factor");
            let hv: Vec<f64> = if g("vol_method") > 0.5 { stdev_pop(c, 30) } else { atr_simple(x, 30) }.into_iter().map(|v| v * vf).collect();
            let b = id == "strtw_dbs2b";
            rev_sim(x, from, to, |i| {
                let (hh, ll, half) = (hi(&x.h, i, 20), lo(&x.l, i, 20), hv[i] * 0.5);
                if !hv[i].is_finite() { return (f64::NAN, f64::NAN, f64::NAN, f64::NAN); }
                (hh + hv[i], ll - hv[i], if b { ll + half } else { ll - half }, if b { hh - half } else { hh + half })
            }, |_, _| (Ex::default(), false))
        }
        "strtw_sdb" | "strtw_sdb_x" => {
            let n = u("band_len");
            let (xh, xl) = (xavg(&x.h, n), xavg(&x.l, n));
            let (sh, sl) = (stdev_pop(&x.h, n), stdev_pop(&x.l, n));
            let levels = |i: usize| if !sh[i].is_finite() { (f64::NAN, f64::NAN, f64::NAN, f64::NAN) } else { (xh[i] + 2. * sh[i], xl[i] - 2. * sl[i], xl[i], xh[i]) };
            if id == "strtw_sdb_x" { rev_sim(x, from, to, levels, |i, p| (template_exits(x, i, p, 0.055, 40, 0.0018, 0.75), p.bse > 105)) }
            else { rev_sim(x, from, to, levels, |_, _| (Ex::default(), false)) }
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn previous_week_and_month_closes() {
        // 2024-01-25 (Thu) .. 2024-02-09 (Fri), weekdays only
        let days = ["2024-01-25", "2024-01-26", "2024-01-29", "2024-01-30", "2024-01-31", "2024-02-01", "2024-02-02", "2024-02-05", "2024-02-06"];
        let r: Vec<Row> = days.iter().enumerate().map(|(i, d)| Row { ticker: "T".into(), date: d.to_string(), open: 1., high: 1., low: 1., close: 10. + i as f64, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p = periods(&x);
        assert_eq!(p.w1[2], 11.); // Monday 01-29: last week closed Friday 01-26 at 11
        assert_eq!(p.w1[7], 16.); // Monday 02-05: week of 01-29..02-02 closed at 16
        assert_eq!(p.w2[7], 11.);
        assert_eq!(p.m1[5], 14.); // Feb 1: January's last close (01-31) = 14
    }
    #[test]
    fn meander_levels_are_symmetric_for_flat_bars() {
        let b = [(100., 101., 99., 100.); 5];
        let (lo_, hi_) = vs_levels(&b, &[100.; 5], 100., 1.);
        assert!((100. - lo_ - (hi_ - 100.)).abs() < 1e-9 && lo_ < 100.);
    }
}
