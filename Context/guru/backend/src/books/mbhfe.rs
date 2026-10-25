//! Mark Boucher, "The Hedge Fund Edge: Maximum Profit/Minimum Risk Global Trend Trading Strategies"
//! (Wiley 1999). Book id "mbhfe". Library notes: library/mbhfe/README.md.
//! Page refs are the printed pages (PDF page = (printed + 14) / 2 in the two-up scan).
//! Live: the two NYSE breadth thrust models of Ch 4 (System Spreadsheets 4.2 and 4.3), plus NSE breadth variants.
//! Everything else in the book is a macro / fundamental / foreign-index filter (NeedsData) or a discretionary pattern
//! (NotTestable). Duplicates not repeated here: OTC 7.9% model = mmft_nasdaq79, DJ20 bond ROC = mmft_bond_roc,
//! Appendix A "flag within a flag" = ats_flag_within_a_flag.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mbhfe";
/// Admitted ids (authors/boucher/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["mbhfe_upvol77", "mbhfe_upvol77_nse", "mbhfe_ad11", "mbhfe_ad11_nse"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const UPVOL_NYSE: &[&str] = &["NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume"];
const UPVOL_NSE: &[&str] = &["NSE_ADV_DEC.UpVolume", "NSE_ADV_DEC.DownVolume"];
const AD_NYSE: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines"];
const AD_NSE: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines"];

const UPVOL_RULES: &[&str] = &[
    "F|\"Here you are simply taking the five-day moving average of advancing volume and dividing it by the five-day moving average of total volume.\" (p.136) Total volume = up volume + down volume: \"Data Used: Daily NYSE Up Volume and Down Volume\" (System Spreadsheet 4.2, p.138). [Fitted reading: up/(up+down) reproduces the printed entry dates; adding unchanged volume does not - p.unch=1 adds it, NYSE only]",
    "E|\"System Description: Buy S&P when the 5-day moving average of the percent of total volume that is up volume > 77%.\" (System Spreadsheet 4.2, p.138) Buy on the signal day's close (the printed entry prices are that day's S&P close).",
    "X|\"Exit after three months.\" (System Spreadsheet 4.2, p.138) [Fitted reading: 63 trading days after entry - the printed exits are 63 sessions after entry in 8 of 9 trades; a calendar 3 months misses 3 of them]. Exit on the close.",
    "F|One position at a time: signals while long are ignored (the printed list has no overlapping trades).",
];
const AD_RULES: &[&str] = &[
    "F|\"Simply take the 11-day moving average of advances and divide it by the 11-day moving average of declines\" (p.136) (Spreadsheet 4.3 says \"11-day average of the advance/decline ratio\"; the p.136 ratio of averages reproduces all 8 printed entry dates our breadth covers, the average of daily ratios does not).",
    "E|\"System Description: Buy when the 11-day average of the advance/decline ratio > 1.9.\" (System Spreadsheet 4.3, p.139) \"This gets bullish on moves over 1.9.\" (p.136-139) Buy the S&P on the signal day's close.",
    "X|\"Exit three months later.\" (System Spreadsheet 4.3, p.139) [Fitted reading: 63 trading days, as for Spreadsheet 4.2]. Exit on the close.",
    "F|One position at a time: signals while long are ignored.",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, series: &'static [&'static str], rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: series, book: BOOK, rules, notes, claim,
    };
    let nd = |id, name, group, why, rules, notes, claim| d(id, name, group, "Long", Status::NeedsData(why), &[], rules, notes, claim);
    use Status::*;
    vec![
        // ---- Ch 4 · technical breadth models (Live)
        d("mbhfe_upvol77", "Up volume > 77% of total volume (5-day) · NYSE breadth", "Ch 4 · Macro technical tools: breadth", "Long", Live, UPVOL_NYSE, UPVOL_RULES,
          "Book: S&P 500 1/4/43-12/31/97 with NYSE up/down volume (Spreadsheet 4.2 prints all 23 trades). Vault: NYSE breadth 1965-2020 (extra_data/us), so the S&P entries 1984-1997 can be checked trade by trade on SPX (DJI from 1970). Run on SPX/SPY/DJI. India: mbhfe_upvol77_nse.",
          claim("23", "71.9% (printed; the list shows 16 of 23)", "+6.7% (153.70% total)", "1,522 days in trade", "S&P 500 1/4/43-12/31/97: 25.3% annual rate when invested, max DD 10.5% (Spreadsheet 4.2)")),
        d("mbhfe_upvol77_nse", "Up volume > 77% of total volume (5-day) · NSE breadth (India)", "Ch 4 · Macro technical tools: breadth", "Long", Live, UPVOL_NSE, UPVOL_RULES,
          "India mapping: NSE up volume / (up + down volume) from extra_data/india/NSE_ADV_DEC.csv (1999-) instead of NYSE; trade NIFTY 50 / NIFTY BANK. 77% and 63 days kept (no re-scaling). NSE has no unchanged-volume column, so p.unch has no effect.",
          None),
        d("mbhfe_ad11", "11-day advance/decline ratio > 1.9 · NYSE breadth", "Ch 4 · Macro technical tools: breadth", "Long", Live, AD_NYSE, AD_RULES,
          "Book: S&P 500 1/29/43-12/31/97 with NYSE advances/declines (Spreadsheet 4.3 prints all 20 trades). Vault NYSE breadth starts 1965, so 1967-1991 entries can be checked (S&P prices on SPX from 1983).",
          claim("20", "85% (17 of 20 in the list)", "+7.67% (153.46% total)", "1,317 days in trade", "S&P 500 1/29/43-12/31/97: 29.2% annual rate when invested, 9% of days invested (Spreadsheet 4.3)")),
        d("mbhfe_ad11_nse", "11-day advance/decline ratio > 1.9 · NSE breadth (India)", "Ch 4 · Macro technical tools: breadth", "Long", Live, AD_NSE, AD_RULES,
          "India mapping: NSE advances/declines (extra_data/india/NSE_ADV_DEC.csv, 1999-) instead of NYSE; 1.9 and 63 days kept.", None),
        // ---- Ch 2 · monetary and economic gauges (macro data)
        nd("mbhfe_tbill_roc", "3-month T-bill yield 12-month ROC 6% (Spreadsheets 2.1-2.2)", "Ch 2 · Liquidity: monetary gauges",
           "monthly 3-month T-bill yield history (1943-) - no rates data in the vault",
           &["E|\"Buy S&P when the 12-month rate of change of 3-Month T-Bill Yield < 6.0%. Exit when > 6.0%.\" (Spreadsheet 2.1, p.64)",
             "E|Bearish twin: \"Buy S&P when the 12-month rate of change of 3-Month T-Bill Yield > 6.0%. Exit when < 6.0%.\" (Spreadsheet 2.2, p.66)"],
           "Monthly closes of the T-bill yield and the S&P. India would need 91-day T-bill yields (RBI) - not in the vault.",
           claim("", "", "", "", "S&P 1/29/43-12/31/97: 18.8% a.r. when ROC <= 6%, 2.2% when > 6% (Master spreadsheet p.363)")),
        nd("mbhfe_tbond_roc", "30-year Treasury yield 12-month ROC 9% (Spreadsheets 2.5-2.6)", "Ch 2 · Liquidity: monetary gauges",
           "monthly 30-year Treasury bond yield history - no rates data in the vault",
           &["E|\"Buy S&P when the 12-month rate of change of 30-Year Treasury Bond Yield < 9%. Exit when > 9%.\" (Spreadsheet 2.5, p.70)",
             "E|Bearish twin: \"> 9%. Exit when < 9%.\" (Spreadsheet 2.6, p.70)"],
           "", claim("", "", "", "", "S&P 1/29/43-12/31/97: 14.8% a.r. (< 9%) vs 0.9% (> 9%)")),
        nd("mbhfe_yield_curve", "Yield curve: 30-year / 3-month ratio (Spreadsheets 2.7-2.8)", "Ch 2 · Liquidity: monetary gauges",
           "monthly 30-year Treasury and 3-month T-bill yields - not in the vault",
           &["E|\"Buy S&P when the ratio of 30-Year Treasury Bonds Yields to 3-Month Treasury Bills is < 1. Exit when the ratio > 1.\" (Spreadsheet 2.7, p.72)",
             "E|\"Buy S&P when the ratio ... is > 1.15. Exit when the ratio < 1.15.\" (Spreadsheet 2.8, p.72)"],
           "", claim("", "", "", "", "S&P 1/29/43-12/31/97: -7.5% a.r. (< 1.0) vs 19.1% (> 1.15)")),
        nd("mbhfe_monetary_composite", "Monetary composite (Spreadsheet 2.9)", "Ch 2 · Liquidity: monetary gauges",
           "monthly T-bill yield, Dow Jones 20 Bond Average and 30-year Treasury yield - not in the vault",
           &["E|\"Buy S&P when 3-Month T-Bill Yield 12-Month ROC <= 6.0%, Dow Jones 20 Bond Average 12-Month ROC > -1.5%, and 30-Year Treasury Bond Rate 12-Month ROC < 9%. Exit when any of these is no longer true.\" (Spreadsheet 2.9, p.74)"],
           "", claim("", "", "", "", "S&P 1/43-12/97: 20.1% a.r. when invested")),
        nd("mbhfe_capacity_util", "Capacity utilization 81.5% / 88.5% (Spreadsheets 2.10-2.11)", "Ch 2 · Economic gauges",
           "monthly US capacity utilization rate - not in the vault",
           &["E|\"Buy S&P when the Capacity Utilization Rate < 81.5%. Exit when > 81.5%.\" (Spreadsheet 2.10, p.78)",
             "E|\"Buy S&P when the Capacity Utilization Rate > 88.5%. Exit when < 88.5%.\" (Spreadsheet 2.11, p.78)"],
           "", claim("", "", "", "", "S&P 1/29/43-12/31/97: 20.4% a.r. (<= 81.5%) vs -6.3% (> 88.5%)")),
        nd("mbhfe_ind_production", "Industrial production 12-month ROC (Spreadsheets 2.12-2.13)", "Ch 2 · Economic gauges",
           "monthly Industrial Production Index - not in the vault",
           &["E|\"Buy S&P when the 12-month rate of change of the Industrial Production Index close < 5.6%\" (Spreadsheet 2.12, p.80); exit when no longer true (Master spreadsheet p.363)",
             "E|\"Buy S&P when the 12-month rate of change of the Industrial Production Index close > 7.0%\" (Spreadsheet 2.13, p.80)"],
           "", claim("", "", "", "", "S&P 1/31/45-12/29/97: 13.2% a.r. (< 5.6%) vs 1.1% (> 7.0%)")),
        nd("mbhfe_unemployment", "Unemployment vs 29 months ago (Spreadsheets 2.14-2.15)", "Ch 2 · Economic gauges",
           "monthly civilian unemployment rate - not in the vault",
           &["E|\"Buy when current unemployment > that 29 months ago. Exit when current unemployment < unemployment rate 29 months ago.\" (Spreadsheet 2.14, p.82)",
             "E|\"Buy when current unemployment < that 29 months ago.\" (Spreadsheet 2.15, p.82)"],
           "", claim("", "", "", "", "S&P 1/31/47-12/31/97: 16.1% a.r. (rising) vs 4.4% (falling)")),
        nd("mbhfe_cpi_gdp", "CPI inflation 3.2% and quarterly GDP growth 4% (Spreadsheets 2.16-2.17)", "Ch 2 · Economic gauges",
           "monthly CPI and quarterly GDP growth - not in the vault",
           &["E|\"Buy S&P when CPI Inflation < 3.2% and Quarterly GDP Growth < 4%. Exit when both move above their respective thresholds.\" (Spreadsheet 2.16, p.84)",
             "E|\"Buy S&P when CPI Inflation > 3.2% and Quarterly GDP Growth > 4%. Exit when both fall below their respective thresholds.\" (Spreadsheet 2.17, p.84)"],
           "", claim("", "", "", "", "S&P 4/30/59-12/29/97: 19.2% a.r. vs 2.9%")),
        nd("mbhfe_cpi_fast_slow", "CPI fast vs slow rate of change (Spreadsheets 2.18-2.19)", "Ch 2 · Economic gauges",
           "monthly CPI - not in the vault",
           &["E|\"Buy when the fast CPI ROC < slow CPI ROC\" ... \"Exit when fast CPI ROC > slow CPI ROC.\" (Spreadsheet 2.18, p.86)"],
           "Fast ROC = CPI vs a year ago; slow ROC = 15-month CPI average vs the prior 15-month average (Science of Trading workbook, Timing Models p.13).",
           claim("", "", "", "", "S&P 1/29/43-12/31/97: 13.2% a.r. (fast < slow) vs 1.0%")),
        nd("mbhfe_crb_roc", "CRB index 12-month ROC (Spreadsheets 2.20-2.22)", "Ch 2 · Economic gauges",
           "monthly CRB index - not in the vault",
           &["E|\"Buy S&P when the 12-month rate of change of the CRB Index < -1%. Exit when > -1%.\" (Spreadsheet 2.20, p.88)",
             "E|\"Buy S&P when 5% > 12-month rate of change of the CRB Index < -1. Exit when CRB Index is outside this range.\" (Spreadsheet 2.21, p.90)",
             "E|\"Buy S&P when the 12-month rate of change of the CRB Index > 5%. Exit when < 5%.\" (Spreadsheet 2.22, p.90)"],
           "", claim("", "", "", "", "S&P 1/31/77-12/31/97: 21.5% / 21.2% / -4.8% a.r.")),
        nd("mbhfe_sensitive_materials", "Sensitive Materials Index 18-month ROC 18% (Spreadsheets 2.23-2.24)", "Ch 2 · Economic gauges",
           "monthly Sensitive Materials Index - not in the vault",
           &["E|\"Buy S&P when the 18-month rate of change of the Sensitive Materials Index close is < 18%. Exit when > 18%.\" (Spreadsheet 2.23, p.92)"],
           "", claim("", "", "", "", "S&P 1/31/57-12/31/97: 18.4% a.r. vs -10.4%")),
        nd("mbhfe_sp_40wk_rates", "S&P > 40-week MA and T-bill or T-bond yield < its 40-week MA (Spreadsheet 2.25)", "Ch 2 · Liquidity: trend + rates",
           "weekly 3-month T-bill and Treasury bond yields - not in the vault (the S&P leg alone would be a different system)",
           &["E|\"Buy S&P 500 when S&P 500 > S&P 500 40-week moving average (MA) and either 3-month T-Bill Yield is < 3-month T-Bill Yield 40-week MA or T-Bond Yield < T-Bond Yield 40-week MA.\" (Spreadsheet 2.25, p.102)",
             "X|\"Exit S&P 500 when the above criteria are not met and hold 3-month T-Bills.\" (p.102)"],
           "Weekly closes. India would need G-sec and T-bill yield history.", claim("", "", "", "", "S&P 1/2/43-12/31/97: 10.2% compound (Master p.364)")),
        nd("mbhfe_global_40wk", "Global countries: index and bond or short-rate prices over 40-week MAs", "Ch 2-3 · Global liquidity model",
           "weekly stock indexes, bond yields and short rates for the world's markets - not in the vault",
           &["E|\"We will only invest in markets where either their bond prices are above the 40-week moving average (or bond interest rates are below 40-wk MA) or their short-rate prices are above the 40-week MA (or short rates below 40-wk MA), and their equity index is above its 40-week moving average.\" (p.102)"],
           "Book: compound 12.6% vs 8.5% US / 8.8% global (p.102-106).", None),
        nd("mbhfe_global_rs_4040", "40/40 global relative-strength country portfolio", "Ch 4 · Relative strength",
           "weekly stock index, bond and bill prices for many countries - not in the vault",
           &["F|Countries qualify when the stock index and (bond or bill prices) are above their 40-week MAs; rank by six-month RS of index averaged with interest-rate RS (p.146)",
             "E|\"invested 20 percent in each of the top five markets ... a maximum of 25 percent in each market ... residual in T-bills\"; rebalance January 1 and July 1; exit a country dropped from the table (p.146)",
             "F|Book's RS: (2 x (1-wk RS + 5-wk RS + 8-wk RS + 13-wk RS) + (26-wk RS + 40-wk RS + 52-wk RS)) / 11 (p.148)"],
           "Figure 4.1: drawdown cut from 54% to 12.2%, reliability ~80% (p.148).", None),
        // ---- Ch 4 · sentiment / employment
        nd("mbhfe_consumer_sentiment", "Consumer sentiment 99.5 (Spreadsheets 4.4-4.5)", "Ch 4 · Sentiment",
           "monthly University of Michigan Consumer Sentiment Index - not in the vault",
           &["E|\"Buy S&P when the consumer sentiment index is > 99.5.\" (Spreadsheet 4.4, p.140) / \"< 99.5\" (Spreadsheet 4.5, p.142); exit on the opposite reading"],
           "", claim("", "", "", "", "S&P 1/31/51-12/31/97: 14.2% a.r. (<= 99.5) vs -2.5% (> 99.5)")),
        nd("mbhfe_help_wanted", "Help Wanted Index ROC (Spreadsheets 4.6-4.7)", "Ch 4 · Sentiment",
           "monthly Help Wanted Advertising Index - not in the vault",
           &["E|\"When the 17-month Rate of Change in the Help Wanted Advertising Index is less than or equal to -21 percent\" buy the S&P (Spreadsheet 4.6, p.142)",
             "E|\"Buy S&P when the 16-month rate of change of the Help Wanted Index > 24%. Exit when < 24%.\" (Spreadsheet 4.7, p.144)"],
           "", claim("", "", "", "", "S&P 1/31/51-12/31/97: 27.8% a.r. vs -2.1%")),
        // ---- Ch 8 · other asset classes
        nd("mbhfe_bond_bill_utility", "Bond-Bill-Utility bond model (Spreadsheet 8.1)", "Ch 8 · Bonds",
           "weekly 30-year yield, T-bill yield, Dow Jones Utility Average and a bond total-return index - not in the vault",
           &["E|\"you buy bonds when the 30-year government bond yield is below its 10-week moving average and either (a) the 3-month government T-bill yield is below its 38-week moving average, or (b) the Dow Jones Utility Average Index is above its 10-week moving average.\" (p.300)",
             "X|Exit the bond total return index and hold T-bills when the conditions fail (Spreadsheet 8.1, p.301)"],
           "", claim("", "", "", "", "Bonds 1/2/43-12/31/97: 14.6% a.r. (Master p.364), max DD 3.2% (p.300)")),
        nd("mbhfe_gold_system", "Gold system: GMI, gold and silver over their MAs (Spreadsheet 8.2)", "Ch 8 · Gold",
           "weekly Barron's Gold Mining Index, gold and silver prices - not in the vault",
           &["E|\"Buy GMI Index when GMI Index > GMI Index 16-week MA and gold prices > gold price 68-week MA and silver prices > 12-week MA. Exit GMI and buy 3-month T-bills when these conditions are no long true.\" (Spreadsheet 8.2, p.312)"],
           "", claim("", "", "", "", "GMI 3/1/75-12/31/97: 8.7% compound (Master p.364)")),
        nd("mbhfe_k_ratio", "Kaeppel/Davis/Freeburg K-ratio gold system", "Ch 8 · Gold",
           "weekly Barron's Gold Mining Index and Handy & Harman gold price - not in the vault",
           &["F|\"K-ratio = Barron's Gold Mining Index (GMI) divided by Handy Harmon's gold bullion price, on a weekly basis.\" (p.310)",
             "E|Long-term: \"Buy when the 46-week MA falls below the lower SD line [2.3 SD] and then rises above it again. Sell when the 46-week MA rises above the upper SD line and then falls below it again.\" (p.311)",
             "E|Intermediate: 4-week MA with 1.3 SD bands, \"Buy when the 4-week MA moves above the upper SD band and then falls below it; and sell when the 4-week MA moves below the lower SD band and then moves above it\"; take any buy and any exit (p.311)"],
           "", claim("", "2/3 profitable", "", "13 weeks", "Gold shares late 1975-1997: 28.8% GPA, max DD 21% (p.311)")),
        nd("mbhfe_gmi_momentum", "GMI-Bullion-Swiss Franc momentum model", "Ch 8 · Gold",
           "weekly Gold Mining Index, gold bullion and Swiss franc - not in the vault",
           &["E|\"Buy when the weekly GMI close is 8 percent above a weekly GMI low close; and the Swiss Franc (versus dollar) is above its 9 wk MA; and gold bullion is higher than it was 26 weeks ago.\" (p.312)",
             "X|\"Once long exit when the GMI weekly falls 8 percent or more\" from a weekly high close (p.312)"],
           "", claim("", "59%", "", "", "Since 1975: 21.3% GPA, max DD 19%, invested 29% (p.312)")),
        nd("mbhfe_commodity_model", "Industrial commodity cycle model (OECD + CRB metals / raw industrials)", "Ch 8 · Commodities",
           "OECD industrial production / leading indicator diffusion and weekly CRB metals and raw industrials indexes - not in the vault",
           &["F|\"an increase in the Organization for Economic Cooperation and Development (OECD) industrial production figures; or a rising OECD Leading Economic Indicator Diffusion Index\" (p.326)",
             "E|\"a weekly close by the CRB metals index that is 5 percent or more above the lowest weekly close since the uptrend began; and (b) a weekly close by the CRB raw industrial index that is 4 percent or more above the lowest weekly close\" (p.326)",
             "X|\"Exit on any move down by the weekly CRB metals index close by 5 percent or more from its highest weekly close since the uptrend began, and a move down by the CRB raw industrial index weekly close of 4 percent\" (p.326)"],
           "", None),
        // ---- Ch 7 · stock selection (stock universe + fundamentals)
        d("mbhfe_flag_breakout", "Flag pattern breakout in runaway stocks with fuel (Figure 7.11)", "Ch 7 · Stock trading method", "Long",
          NotTestable("needs a fundamentals-screened stock universe (\"Has fuel\", RS, EPS, institutional ownership), the flag's \"sharp run up\" is not quantified, and the only exits are fundamental (RS < 60, P/E, earnings, institutions) or chart patterns"),
          &[],
          &["F|\"1. Has fuel; maximum fuel is better. 2. At least 5 runaway characteristics, usually, 10 or more.\" (Figure 7.11, p.274)",
            "F|\"First, there must be a sharp run up in the price followed by a tight consolidation. The consolidation needs to have at least four weeks in the pattern with the highs within 3 ticks of the highest high of the first two weeks of the pattern. The flag cannot correct 37% or more of the previous move\" (p.274)",
            "E|\"The pattern is triggered when one of four accumulation indicators (OBV, WAD, SWAD, or VOLAC) breaks out to new highs before price breaks above the flag highs. At that time, enter a buy stop a few ticks above the flag high with an OPS a few ticks below the flag low.\" (p.274)",
            "X|Simple exit rules 1-9 (p.280): trail the OPS under each 20-day consolidation; sell when RS < 60; half profits at P/E > 2 x S&P P/E or P/E = growth rate or institutions > 40%; earnings misses; 10% below the 200-day MA; weekly head-and-shoulders / double top; negative reaction to good news"],
          "Book: >72% of these patterns profitable 1992-1997 on PSL stock lists (p.276).", None),
        d("mbhfe_ma200_reaction", "40-week MA reaction buy in runaway fuel stocks (Figure 7.12)", "Ch 7 · Stock trading method", "Long",
          NotTestable("needs runaway fuel stocks (fundamental screen); stop and entry offsets are in stock points tied to price bands, and the exits are the fundamental exit rules"),
          &[],
          &["E|\"buy runaway fuel stocks whenever they decline to within one point of the 40-week MA with an OPS three points below the MA on the week the buy is triggered. (Use 1.5 points for stocks where the high prior to the decline is under 10; six points under MA when stock's high is 60 or more, etc.)\" (Figure 7.12, p.274)"],
          "\"about 10 percent less reliable than the flag pattern\" (p.276).", None),
        d("mbhfe_short_sales", "Minimum-fuel short sales and their exits", "Ch 7 · Stock trading method", "Short",
          NeedsData("stock universe with quarterly earnings, sales, P/S, P/E, RS/EPS ranks, debt, institutional ownership and Value Line/Zacks/Lowry ratings - not in the vault"),
          &[],
          &["F|Short-sale criteria 1-9 (earnings declines, runaway technicals down, EPS and RS rank < 50, yield <= 5%, debt, institutional ownership, worst ratings) (p.280-281)",
            "X|Exit shorts on earnings turnaround, P/E below growth, 10% above the 200 MA, half profits at -40% then trail, RS/EPS rank above 50, weekly double bottom (p.281)"],
          "", None),
        // ---- Ch 7 · runaway characteristics (definitions only)
        d("mbhfe_tbblbg_runaway", "TBBLBG runaway characteristics and the 5-in-21 rule", "Ch 7 · Identifying meteors and fixed stars", "Long/Short",
          NotTestable("market-selection definitions only: no entry order, stop or exit is attached; \"resistance\" is a judged level"),
          &[],
          &["F|\"A thrust pattern is simply a day in which the range (high-low) is two or more times the average range of the preceding 20 days; the volume is higher than the volume of the prior day; and the close is in the top 1/3 of the day's range.\" (p.250)",
            "F|Breaklap up: low above the prior close but below the prior high; breakaway gap: low above the prior high; each must break resistance / support (p.250-252)",
            "F|\"when you see a stock that has five laps, gaps, and/or thrusts in any 21-day period during the course of its move in a particular direction, you can count it as a runaway trend\" (p.252)"],
          "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |v| Some(("mbhfe_breadth", "Hedge Fund Edge breadth thrusts (3-month hold)", v));
    match id {
        "mbhfe_upvol77" => f("Up volume > 77% · NYSE"),
        "mbhfe_upvol77_nse" => f("Up volume > 77% · NSE"),
        "mbhfe_ad11" => f("11-day A/D > 1.9 · NYSE"),
        "mbhfe_ad11_nse" => f("11-day A/D > 1.9 · NSE"),
        "mbhfe_flag_breakout" | "mbhfe_ma200_reaction" | "mbhfe_short_sales" | "mbhfe_tbblbg_runaway" =>
            Some(("mbhfe_stocks", "Hedge Fund Edge stock selection (stock universe)", "not testable / needs data")),
        _ if id.starts_with("mbhfe_") => Some(("mbhfe_macro", "Hedge Fund Edge macro timing models", "needs data")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let hold = ParamDef { key: "hold", label: "Bars held (\"three months\")", def: 63.0, min: 21.0, max: 126.0, step: 1.0 };
    match id {
        "mbhfe_upvol77" | "mbhfe_upvol77_nse" => Some(vec![
            ParamDef { key: "ma", label: "Moving-average days", def: 5.0, min: 3.0, max: 20.0, step: 1.0 },
            ParamDef { key: "level", label: "Up volume % of total above", def: 77.0, min: 60.0, max: 90.0, step: 0.5 },
            hold,
            ParamDef { key: "unch", label: "Add unchanged volume to total (1 = yes; not the book's data) ", def: 0.0, min: 0.0, max: 1.0, step: 1.0 },
        ]),
        "mbhfe_ad11" | "mbhfe_ad11_nse" => Some(vec![
            ParamDef { key: "ma", label: "Moving-average days", def: 11.0, min: 5.0, max: 30.0, step: 1.0 },
            ParamDef { key: "level", label: "Advances / declines ratio above", def: 1.9, min: 1.2, max: 3.0, step: 0.05 },
            hold,
        ]),
        _ => None,
    }
}

/// Trailing n-bar simple average; NaN until n finite values are available (any NaN in the window = NaN).
fn avg(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}

/// Long at the close of a bar where sig > level while flat; exit at the close `hold` bars later.
fn hold_after(x: &Ctx, sig: &[f64], level: f64, hold: usize, from: usize, to: usize) -> Vec<Trade> {
    let c = &x.c;
    let (mut out, mut i) = (vec![], from);
    while i <= to {
        if sig[i].is_finite() && sig[i] > level && i < to {
            let e = i + hold;
            let (ex, open) = if e <= to { (e, false) } else { (to, true) };
            out.push(Trade { dir: 1.0, legs: vec![Leg { i, px: c[i], w: 1.0 }], exit_i: ex, exit_px: c[ex],
                reason: if open { "open".into() } else { format!("{hold} bars after entry (three months)") },
                entry_kind: "close", signal_i: ex, open, exits: vec![] });
            i = ex + 1;
        } else {
            i += 1;
        }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (ma, level, hold) = (g("ma").round().max(1.0) as usize, g("level"), g("hold").round().max(1.0) as usize);
    let nse = id.ends_with("_nse");
    let sig: Vec<f64> = if id.starts_with("mbhfe_upvol77") {
        let s = if nse { UPVOL_NSE } else { UPVOL_NYSE };
        let (Some(up), Some(dn)) = (x.series(s[0]), x.series(s[1])) else { return Some(vec![]) };
        let un: Vec<f64> = match (g("unch") >= 0.5, nse, x.series("NYSE_ADV_DEC.UnchVolume")) {
            (true, false, Some(u)) => u.to_vec(),
            _ => vec![0.0; up.len()],
        };
        let (au, ad, an) = (avg(up, ma), avg(dn, ma), avg(&un, ma));
        (0..up.len()).map(|i| 100.0 * au[i] / (au[i] + ad[i] + an[i])).collect()
    } else {
        let s = if nse { AD_NSE } else { AD_NYSE };
        let (Some(a), Some(d)) = (x.series(s[0]), x.series(s[1])) else { return Some(vec![]) };
        let (aa, dd) = (avg(a, ma), avg(d, ma));
        (0..a.len()).map(|i| aa[i] / dd[i]).collect()
    };
    Some(hold_after(x, &sig, level, hold, from, to))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hold_63_and_no_overlap() {
        let r: Vec<Row> = (0..200).map(|i| { let c = 100.0 + i as f64; Row { ticker: "T".into(), date: format!("d{i:03}"), open: c, high: c, low: c, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let mut sig = vec![0.0; 200];
        for i in [10, 20, 80] { sig[i] = 80.0; }
        let t = hold_after(&x, &sig, 77.0, 63, 0, 199);
        // 10 -> exit 73 (signal 20 ignored while long), 80 -> exit 143
        let got: Vec<(usize, usize)> = t.iter().map(|t| (t.legs[0].i, t.exit_i)).collect();
        assert_eq!(got, vec![(10, 73), (80, 143)]);
        assert!((t[0].exit_px - 173.0).abs() < 1e-9);
        // average helper: 5-bar mean of 1..=5 = 3
        assert!((avg(&[1.0, 2.0, 3.0, 4.0, 5.0], 5)[4] - 3.0).abs() < 1e-12);
    }
}
