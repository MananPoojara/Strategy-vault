//! Larry Connors & Cesar Alvarez, "High Probability ETF Trading" (2009). Book id "hpetf".
//! Seven strategies, long and short, basic and aggressive (scale-in) versions; every order fills at the close.
//! Book test: 20 non-leveraged ETFs, inception (or 1/1993) to 12/31/2008, no slippage or commissions. The vault runs the
//! rules on index data (NIFTY 50, NIFTY BANK, NIFTY IT, NIFTY MIDCAP 100; US check SPY/QQQ/IWM/DIA-as-DJI), an instrument
//! swap of the book's index ETFs (SPY, QQQQ, IWM, DIA). Sector, country, commodity ETF results are listed but cannot be checked.
//! Skipped as duplicates (identical rules already in the vault): RSI 25 basic long = rsi25 `r25_rsi25_official_sma200`;
//! %b basic long/short = pctb `pbs_percent_b_swing`.
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "hpetf";

/// High Probability ETF Trading seminar manuals (Connors, 2009; Session 2 PDF) rules added 2026-10-21 after the
/// full-source reading (connors/newrules-misc/SOURCE-CONTRACT.md). Listed here, next to the book's ETF rules.
pub const SEM_RSI14: &str = "hpetf_sem_rsi14_tps";
pub const SEM_PAIRS: &str = "hpetf_sem_pairs";
/// Admitted after the source review and an independent chronological ledger check (the pairs rule is held: no data).
pub const REVIEWED_DAILY: &[&str] = &[SEM_RSI14];
pub fn reviewed_daily(id: &str) -> bool {
    REVIEWED_DAILY.contains(&id)
}

/// [Not from the book] Price-vs-SMA tie tolerance (relative), as in ws5/cwin: an SMA can equal a price exactly in decimal
/// arithmetic but differ by float rounding (QQQ 2013-12-16: close 85.32 = SMA5), so within 1e-9 x |SMA| is a tie,
/// which is never "above" or "below".
const TIE: f64 = 1e-9;
fn above(a: f64, ma: f64) -> bool {
    a > ma + TIE * ma.abs()
}
fn below(a: f64, ma: f64) -> bool {
    a < ma - TIE * ma.abs()
}

/// (family, long, aggressive)
fn kind(id: &str) -> Option<(&'static str, bool, bool)> {
    Some(match id {
        "hpetf_3dhl_long" => ("3dhl", true, false),
        "hpetf_3dhl_long_aggr" => ("3dhl", true, true),
        "hpetf_3dhl_short" => ("3dhl", false, false),
        "hpetf_3dhl_short_aggr" => ("3dhl", false, true),
        "hpetf_rsi25_long_aggr" => ("rsi25", true, true),
        "hpetf_rsi75_short" => ("rsi25", false, false),
        "hpetf_rsi75_short_aggr" => ("rsi25", false, true),
        "hpetf_r3_long" => ("r3", true, false),
        "hpetf_r3_long_aggr" => ("r3", true, true),
        "hpetf_r3_short" => ("r3", false, false),
        "hpetf_r3_short_aggr" => ("r3", false, true),
        "hpetf_pctb_long_aggr" => ("pctb", true, true),
        "hpetf_pctb_short_aggr" => ("pctb", false, true),
        "hpetf_mdd_long" => ("mdd", true, false),
        "hpetf_mdd_long_aggr" => ("mdd", true, true),
        "hpetf_mdu_short" => ("mdd", false, false),
        "hpetf_mdu_short_aggr" => ("mdd", false, true),
        "hpetf_rsi10_6_long" => ("rsi10", true, true),
        "hpetf_rsi90_94_short" => ("rsi10", false, true),
        "hpetf_tps_long" => ("tps", true, false),
        "hpetf_tps_short" => ("tps", false, false),
        _ => return None,
    })
}

const NOTE: &str = "All orders on the close; exit not tested on the entry bar. One position at a time. Aggressive version: one extra unit, so a full position is 2 equal units (0.5 each); returns are on the full 2-unit allocation, while the book's 'Average % P/L' is on the cash actually invested (verify_hpetf.py computes both). The add is checked only on bars after the entry (the reading that reproduces QQQ exactly; an entry-bar add does not). Book universe: 20 ETFs from inception (or 1/1993) to 12/31/2008; the vault runs it on the indices (instrument swap of SPY/QQQQ/IWM/DIA).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, rules, claim| StrategyDef {
        id, name, group, session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: NOTE, claim,
    };
    const P: &str = "1/29/93 - 12/31/08, SPY";
    let (g2, g3, g4, g5, g6, g7, g8) = ("Ch 2 · The 3-Day High/Low Method", "Ch 3 · RSI 25 & RSI 75", "Ch 4 · R3 Strategy", "Ch 5 · The %b Strategy",
        "Ch 6 · Multiple Days Up / Down", "Ch 7 · RSI 10/6 & RSI 90/94", "Ch 8 · TPS");
    vec![
        d("hpetf_3dhl_long", "3-Day High/Low · Long", g2, "Long",
          &["F|(p10) \"Today the ETF is above the 200-day moving average.\"", "F|\"Today the ETF closes below its 5-day moving average.\"",
            "E|\"Two days ago the high and low price of the day is below the previous day's high and low. Yesterday the high and low price of the day is below the previous day's. Today's high and low price is below yesterday's.\" (strict <)",
            "E|\"Buy on the close today.\"", "X|\"Exit on the close when the ETF closes above its 5-day simple moving average.\""],
          claim("64", "87.5%", "0.91%", "2.7 days", P)),
        d("hpetf_3dhl_long_aggr", "3-Day High/Low · Long aggressive", g2, "Long",
          &["F|(p10) ETF above its 200-day MA; closes below its 5-day MA", "E|Three days in a row of lower highs and lower lows; buy on the close",
            "A|\"Aggressive Version - Buy a second unit if prices close lower than your initial entry price anytime you're in the position.\"",
            "X|Exit on the close when the ETF closes above its 5-day SMA"],
          claim("64", "90.6%", "1.12%", "2.7 days", P)),
        d("hpetf_3dhl_short", "3-Day High/Low · Short", g2, "Short",
          &["F|(p16) \"The ETF is below the 200-day moving average.\"", "F|\"Today the ETF closes above its 5-day moving average.\"",
            "E|Three days in a row the high and low are above the previous day's (strict >)", "E|\"Sell short on the close today.\"",
            "X|\"Exit on the close when the ETF closes below its 5-day simple moving average.\""],
          claim("32", "84.4%", "0.67%", "3.4 days", P)),
        d("hpetf_3dhl_short_aggr", "3-Day High/Low · Short aggressive", g2, "Short",
          &["F|(p16) ETF below its 200-day MA; closes above its 5-day MA", "E|Three days of higher highs and higher lows; sell short on the close",
            "A|\"Aggressive Version - Short a second unit if prices close higher than your initial entry price anytime you're in the position.\"",
            "X|Exit on the close when the ETF closes below its 5-day SMA"],
          claim("32", "84.4%", "0.89%", "3.4 days", P)),
        d("hpetf_rsi25_long_aggr", "RSI 25 · Long aggressive", g3, "Long",
          &["F|(p24) \"An ETF is above its 200-day moving average.\"", "E|\"The 4-period RSI closes under 25. Buy on the close.\"",
            "A|\"Aggressive Version - Buy a second unit if at anytime while you're in the position the 4-period RSI closes under 20.\"",
            "X|\"Exit when the 4-period RSI closes above 55.\""],
          claim("84", "89.3%", "1.48%", "5.2 days", P)),
        d("hpetf_rsi75_short", "RSI 75 · Short", g3, "Short",
          &["F|(p30) \"The ETF is trading under its 200-day moving average.\"", "E|\"The 4-period RSI is above 75, sell short on the close.\"",
            "X|\"Exit the position on the close when the 4-period RSI closes under 45.\""],
          claim("30", "63.3%", "0.68%", "8.5 days", P)),
        d("hpetf_rsi75_short_aggr", "RSI 75 · Short aggressive", g3, "Short",
          &["F|(p30) ETF under its 200-day MA", "E|4-period RSI above 75; sell short on the close",
            "A|\"Aggressive Version - Sell short another unit if the ETF has a closing 4-period RSI above 80\"", "X|Exit on the close when the 4-period RSI closes under 45"],
          claim("30", "70.0%", "0.94%", "8.5 days", P)),
        d("hpetf_r3_long", "R3 · Long", g4, "Long",
          &["F|(p37) \"The ETF is above the 200-day MA.\"", "E|\"The 2-period RSI drops three days in a row and the first day's drop is from below 60\" (read, from the examples p.41-42: the RSI on the first down day is already below 60)",
            "E|\"The 2-period RSI closes under 10 today. Buy on the close.\"", "X|\"Exit when the 2-period RSI closes above 70.\""],
          claim("73", "84.9%", "1.16%", "4.4 days", P)),
        d("hpetf_r3_long_aggr", "R3 · Long aggressive", g4, "Long",
          &["F|(p37) ETF above the 200-day MA", "E|RSI(2) drops three days in a row, the first drop from below 60, and closes under 10 today; buy on the close",
            "A|\"Aggressive Version - Buy a second unit if prices close lower than your initial entry price anytime you're in the position.\"", "X|Exit when the 2-period RSI closes above 70"],
          claim("73", "89.0%", "1.38%", "4.4 days", P)),
        d("hpetf_r3_short", "R3 · Short", g4, "Short",
          &["F|(p43) \"The ETF is below the 200-day MA.\"", "E|\"The 2-period RSI rises three days in a row and the first day's rise is from above 40\"",
            "E|\"The 2-period RSI closes above 90 today. Sell Short.\"", "X|\"Exit when the 2-period RSI closes below 30.\""],
          claim("34", "70.6%", "1.13%", "5.2 days", P)),
        d("hpetf_r3_short_aggr", "R3 · Short aggressive", g4, "Short",
          &["F|(p43) ETF below the 200-day MA", "E|RSI(2) rises three days in a row, the first rise from above 40, and closes above 90 today; sell short",
            "A|\"Aggressive Version - Short a second unit if prices close higher than your initial entry price anytime you're in the position.\"", "X|Exit when the 2-period RSI closes below 30"],
          claim("34", "76.5%", "1.31%", "5.2 days", P)),
        d("hpetf_pctb_long_aggr", "%b · Long aggressive", g5, "Long",
          &["F|(p50) \"The ETF must be above its 200-day MA.\" %b: 5-period Bollinger Bands, 1 standard deviation (glossary p.111)",
            "E|\"The %b must close under 0.2 for 3 days in a row. If this occurs buy the ETF on the close.\"",
            "A|\"Aggressive Version - Any additional day while you're in the position, if the %b of the ETF closes again below 0.2 buy a second unit on the close.\"",
            "X|\"Exit when the %b closes above 0.8.\""],
          claim("111", "85.6%", "0.95%", "3.9 days", P)),
        d("hpetf_pctb_short_aggr", "%b · Short aggressive", g5, "Short",
          &["F|(p55) \"The ETF closes below its 200-day MA.\"", "E|\"The %b closes above 0.80 for 3 days in a row. Short on the close.\"",
            "A|\"Aggressive Version - If the %b is above 0.80 any additional day you are in the position, short another unit on the close.\"",
            "X|\"Cover your short when the %b closes under 0.20.\""],
          claim("48", "79.2%", "0.98%", "4.1 days", P)),
        d("hpetf_mdd_long", "Multiple Days Down (MDD) · Long", g6, "Long",
          &["F|(p62) \"An ETF is trading above its 200-day moving average.\"", "F|\"The ETF closes below its 5-period moving average on the entry day.\"",
            "E|\"The ETF must drop 4 out of the past 5 days. This means closing prices were lower than the day before for 4 out of the past 5 days. If this happens we buy the ETF on the close today.\"",
            "X|\"Our exit is on the close when the ETF closes above its 5-period simple moving average.\""],
          claim("110", "74.5%", "0.50%", "3.3 days", P)),
        d("hpetf_mdd_long_aggr", "Multiple Days Down (MDD) · Long aggressive", g6, "Long",
          &["F|(p62) ETF above its 200-day MA; closes below its 5-period MA", "E|Lower close 4 of the past 5 days; buy on the close",
            "A|\"Aggressive Version - Buy a second unit if prices close lower than your initial entry price anytime you're in the position.\"", "X|Exit on the close above the 5-period SMA"],
          claim("110", "85.5%", "0.78%", "3.3 days", P)),
        d("hpetf_mdu_short", "Multiple Days Up (MDU) · Short", g6, "Short",
          &["F|(p68) \"The ETF closes under its 200-day moving average.\"", "E|\"The ETF closes higher four out of the past five days.\"", "F|\"The ETF closes above its 5-period moving average.\"",
            "E|Sell short on the close", "X|\"Exit when ETF closes under its 5-period MA.\""],
          claim("50", "80.0%", "0.88%", "3.5 days", P)),
        d("hpetf_mdu_short_aggr", "Multiple Days Up (MDU) · Short aggressive", g6, "Short",
          &["F|(p68) ETF under its 200-day MA; closes above its 5-period MA", "E|Higher close 4 of the past 5 days; sell short on the close",
            "A|\"Aggressive Version - Short a second unit if prices close higher than your initial entry price anytime you're in the position.\"", "X|Exit when the ETF closes under its 5-period MA"],
          claim("50", "80.0%", "1.13%", "3.5 days", P)),
        d("hpetf_rsi10_6_long", "RSI 10/6 · Long", g7, "Long",
          &["F|(p76) \"The ETF is trading above its 200-day moving average.\"", "E|\"We buy when the 2-period RSI of the ETF goes under 10.\" (on the close)",
            "A|\"We buy a second unit if the ETF closes with an RSI reading under 6.\"", "X|\"We exit our position on the close when the ETF closes above its 5-period moving average.\""],
          claim("117", "86.3%", "1.04%", "3.3 days", P)),
        d("hpetf_rsi90_94_short", "RSI 90/94 · Short", g7, "Short",
          &["F|(p81) \"The ETF closes under its 200-day MA.\"", "E|\"The 2-period RSI closes above 90. Short on the close.\"",
            "A|\"If at anytime the 2-period RSI closes above 94, short another unit.\"", "X|\"Exit when the ETF closes under its 5-period moving average.\""],
          claim("47", "76.6%", "1.30%", "3.7 days", P)),
        d("hpetf_tps_long", "TPS 1-2-3-4 · Long", g8, "Long",
          &["F|(p89) \"The ETF is above the 200-day.\"", "E|\"The 2-period RSI is below 25 for two days in a row. Buy 10% of your position on the close.\"",
            "A|\"If prices are lower on the close than your previous entry price, any day you're in the position, buy 20% ... then 30% ... then 40% more of your position\"",
            "A|\"*Do not take a new position anytime the ETF closes under the 200-day MA.\" (adds need Close > SMA200)",
            "X|\"Exit on the close when the 2-period RSI closes above 70.\""],
          claim("133", "93.98%", "1.27% on cash invested", "5.08 bars", P)),
        d("hpetf_tps_short", "TPS 1-2-3-4 · Short", g8, "Short",
          &["F|(p95) \"The ETF is below the 200-day.\"", "E|\"The 2-period RSI is above 75% for two (2) days in a row. Short 10% of your position on the close.\"",
            "A|\"If prices are higher on the close than your previous entry price, any day you're in the position, short 20% ... 30% ... 40% more\" (adds need Close < SMA200)",
            "X|\"Exit on the close when the 2-period RSI closes below 30.\""],
          claim("55", "85.45%", "1.49% on cash invested", "6.04 bars", P)),
        StrategyDef { notes: SEM_RSI14_NOTE, claim: None, ..d(SEM_RSI14, "Seminar · RSI 14 TPS", "Seminar Session 2 · TPS under the 200-day MA", "Long",
          &["F|(S2 p19) \"1. The 200 Day MA is an optional rule and I'll show you the results with and without the 200-day MA.\" Default off; p.trend_filter=1 buys only while the close is under the 200-day SMA (S2 p9: 'TPS strategies which we like to trade ETFs under the 200-day MA') [direction read from p9]",
            "E|\"2. Buy 2 shares (20%) of the ETF when the 14-period RSI closes under 35 and the 2-period RSI is less than 30.\" (on the close)",
            "A|\"3. Buy 3 shares (30%) if the ETF closes lower anytime you're in the position.\" \"4. Buy 5 shares (50%) if the ETF closes lower anytime you're in the position.\" 'Closes lower' = below the previous close (default, p.add_ref=1): the seminar keeps 'closes lower' (S2 p12 rule 3, p13 rule 6) apart from 'closes lower than the previous purchase' (S2 p12 rule 4, p13 rule 5). Option p.add_ref=0 = [Not from the book] below the previous purchase price (the TPS 2-3-5 rule, S1 p83)",
            "X|\"5. Exit when the 2 (two)-period RSI close above 65.\""], None) },
        StrategyDef { notes: "Session 2 pp.99-101 (repeated Session 3 pp.5-7). The pair list and results are only on a spreadsheet not in the PDF. Held: the rule needs two ETFs on the exact same index (e.g. a 1x and a 2x S&P 500 ETF); the vault holds one ETF per index (SPY, QQQ, IWM, MDY, SMH) and an index cannot stand in for an ETF leg, so no pair can be priced. No result is published.",
            status: Status::NeedsData("two ETFs on the exact same index (e.g. SPY with a 2x S&P 500 ETF); the vault holds one ETF per index and an index cannot be an ETF leg, so neither leg pair has real prices"),
            ..d(SEM_PAIRS, "Seminar · Directional 'market neutral' ETF pairs (RSI difference)", "Seminar Session 2 · ETF pairs", "Long/Short",
          &["F|(S2 p101) \"1. Pair two exact index ETFs\"", "E|\"2. The absolute difference of the 2-period RSI in the pair is 10 points or more.\"",
            "E|\"3. Buy the ETF with the lower 2-period RSI and short the ETF with the [higher] 2-period RSI.\" (end of day, p103)",
            "E|\"4. If both sides of the pair are 1x or 2x, then buy and short an equal dollar amount. 5. If one side is a 1x and the second side is a 2x, make sure your 2x position is half the $ amount of your 1x.\"",
            "X|\"6. Exit when the 2 period RSI closes on a cross in the opposite direction.\""], None) },
    ]
}

const SEM_RSI14_NOTE: &str = "High Probability ETF Trading seminar manuals, Session 2 PDF pp.9, 17-20 (rule slides p19-20). The seminar trades US ETFs (examples EWG, XHB, XLF, EWS, IFN) and shows the results only on a spreadsheet that is not in the PDF, so no book figures are claimed. Sizing 20/30/50% of a full position (2/3/5 shares); returns are on the full allocation. All orders on the close; adds are checked on bars after the entry; exit not tested on the entry bar. Index and stock runs are a declared application (user rule 21 Oct 2026), not the author's test.";


pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        SEM_RSI14 => return Some(("hpetf_sem", "HPETF Seminar (2009)", "RSI 14 TPS")),
        SEM_PAIRS => return Some(("hpetf_sem", "HPETF Seminar (2009)", "ETF pairs · data missing")),
        _ => {}
    }
    let (f, long, aggr) = kind(id)?;
    let name = match f {
        "3dhl" => "3-Day High/Low (HPETF)",
        "rsi25" => "RSI 25 & RSI 75 (HPETF)",
        "r3" => "R3 (HPETF)",
        "pctb" => "%b (HPETF)",
        "mdd" => "MDD / MDU (HPETF)",
        "rsi10" => "RSI 10/6 & 90/94 (HPETF)",
        _ => "TPS (HPETF)",
    };
    let v = match (long, aggr || f == "rsi10" || f == "tps") {
        (true, false) => "Long",
        (true, true) if f == "rsi10" || f == "tps" => "Long",
        (true, true) => "Long · aggressive",
        (false, false) => "Short",
        (false, true) if f == "rsi10" || f == "tps" => "Short",
        (false, true) => "Short · aggressive",
    };
    Some(match f {
        "3dhl" => ("hpetf_3dhl", name, v),
        "rsi25" => ("hpetf_rsi25", name, v),
        "r3" => ("hpetf_r3", name, v),
        "pctb" => ("hpetf_pctb", name, v),
        "mdd" => ("hpetf_mdd", name, v),
        "rsi10" => ("hpetf_rsi10", name, v),
        _ => ("hpetf_tps", name, v),
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if id == SEM_RSI14 {
        return Some(vec![p("rsi_n", "Slow RSI period", 14.0, 5.0, 20.0, 1.0), p("slow", "Slow RSI closes under", 35.0, 20.0, 50.0, 5.0),
            p("entry", "and RSI(2) below", 30.0, 5.0, 50.0, 5.0), p("exit", "Exit when RSI(2) closes above", 65.0, 50.0, 80.0, 5.0),
            p("trend_filter", "Optional 200-day rule: 0 = off (default), 1 = buy only while the close is under the trend SMA", 0.0, 0.0, 1.0, 1.0),
            p("trend", "Trend SMA", 200.0, 50.0, 300.0, 10.0),
            p("add_ref", "'Closes lower' = 1 below the previous close (book wording, S2 p19-20), 0 = [Not from the book] below the previous purchase price (S1 p83 TPS)", 1.0, 0.0, 1.0, 1.0)]);
    }
    let (f, long, aggr) = kind(id)?;
    let mut v = vec![p("trend", "Trend SMA", 200.0, 50.0, 300.0, 10.0)];
    let s = |l: f64, sh: f64| if long { l } else { sh };
    match f {
        "3dhl" => v.extend([p("ma", "5-day SMA (filter and exit)", 5.0, 3.0, 10.0, 1.0), p("days", "Days of lower highs and lows", 3.0, 2.0, 5.0, 1.0)]),
        "rsi25" => {
            v.extend([p("rsi_n", "RSI period", 4.0, 2.0, 6.0, 1.0), p("entry", if long { "Buy when RSI closes below" } else { "Short when RSI closes above" }, s(25.0, 75.0), 5.0, 95.0, 5.0),
                      p("exit", if long { "Exit when RSI closes above" } else { "Exit when RSI closes below" }, s(55.0, 45.0), 30.0, 70.0, 5.0)]);
            if aggr {
                v.push(p("add", if long { "Second unit when RSI closes below" } else { "Second unit when RSI closes above" }, s(20.0, 80.0), 5.0, 95.0, 5.0));
            }
        }
        "r3" => v.extend([p("days", "RSI(2) moves N days in a row", 3.0, 2.0, 5.0, 1.0), p("first", if long { "First drop from below" } else { "First rise from above" }, s(60.0, 40.0), 20.0, 80.0, 5.0),
                          p("entry", if long { "RSI(2) today below" } else { "RSI(2) today above" }, s(10.0, 90.0), 5.0, 95.0, 5.0),
                          p("exit", if long { "Exit when RSI(2) closes above" } else { "Exit when RSI(2) closes below" }, s(70.0, 30.0), 20.0, 80.0, 5.0),
                          p("first_ref", "First-drop test on 0 = the first down day's RSI (book examples), 1 = the RSI before the first drop", 0.0, 0.0, 1.0, 1.0)]),
        "pctb" => v.extend([p("n", "%b period (1 std dev)", 5.0, 3.0, 20.0, 1.0), p("entry", if long { "%b below" } else { "%b above" }, s(0.2, 0.8), 0.0, 1.0, 0.05),
                            p("days", "for N days in a row", 3.0, 1.0, 5.0, 1.0), p("exit", if long { "Exit when %b closes above" } else { "Exit when %b closes below" }, s(0.8, 0.2), 0.0, 1.0, 0.05)]),
        "mdd" => v.extend([p("ma", "5-period SMA (filter and exit)", 5.0, 3.0, 10.0, 1.0), p("k", if long { "Down closes, at least" } else { "Up closes, at least" }, 4.0, 2.0, 6.0, 1.0),
                           p("of", "out of the past N days", 5.0, 3.0, 8.0, 1.0)]),
        "rsi10" => v.extend([p("entry", if long { "Buy when RSI(2) below" } else { "Short when RSI(2) above" }, s(10.0, 90.0), 2.0, 98.0, 1.0),
                             p("add", if long { "Second unit when RSI(2) below" } else { "Second unit when RSI(2) above" }, s(6.0, 94.0), 1.0, 99.0, 1.0),
                             p("ma", "Exit on the 5-period SMA", 5.0, 3.0, 10.0, 1.0)]),
        _ => v.extend([p("entry", if long { "RSI(2) below" } else { "RSI(2) above" }, s(25.0, 75.0), 5.0, 95.0, 5.0), p("days", "for N days in a row", 2.0, 1.0, 4.0, 1.0),
                       p("exit", if long { "Exit when RSI(2) closes above" } else { "Exit when RSI(2) closes below" }, s(70.0, 30.0), 20.0, 80.0, 5.0)]),
    }
    Some(v)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() {
        return Some(vec![]);
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    if id == SEM_RSI14 {
        let (c, slow, r2, tr) = (&x.c, x.rsi_n(u("rsi_n").max(2)), &x.rsi2, x.sma_n(u("trend")));
        let (lo, en, ex, filt, prev_close) = (g("slow"), g("entry"), g("exit"), g("trend_filter") >= 0.5, g("add_ref") >= 0.5);
        let e = |i: usize| ((!filt || below(c[i], tr[i])) && slow[i] < lo && r2[i] < en).then_some(Entry::Close);
        let add = |i: usize, t: &Trade| c[i] < if prev_close { c[i - 1] } else { t.legs.last().unwrap().px };
        let xf = |i: usize, _: &Trade| (r2[i] > ex).then(|| Exit::Close(format!("RSI(2) {:.1} > {ex}", r2[i])));
        return Some(simulate(x, &Spec { dir: 1.0, weights: &[0.2, 0.3, 0.5], entry: &e, add: &add, exit: &xf }, from, to));
    }
    let (f, long, aggr) = kind(id)?;
    let (c, h, l) = (&x.c, &x.h, &x.l);
    let dir = if long { 1.0 } else { -1.0 };
    let tr = x.sma_n(u("trend"));
    let trend = |i: usize| if long { above(c[i], tr[i]) } else { below(c[i], tr[i]) };
    let beyond_ma = |a: f64, ma: f64| if long { below(a, ma) } else { above(a, ma) };
    // "beyond" = below for longs, above for shorts (NaN compares false either way)
    let beyond = |a: f64, b: f64| if long { a < b } else { a > b };
    let worse = |i: usize, t: &Trade| beyond(c[i], t.legs[0].px);
    let two = [0.5, 0.5];
    let one = [1.0];
    let w: &[f64] = if aggr { &two } else { &one };
    let run_spec = |weights: &[f64], entry: &dyn Fn(usize) -> bool, add: &dyn Fn(usize, &Trade) -> bool, exit: &dyn Fn(usize) -> Option<String>| {
        let e = |i: usize| entry(i).then_some(Entry::Close);
        let xf = |i: usize, _: &Trade| exit(i).map(Exit::Close);
        simulate(x, &Spec { dir, weights, entry: &e, add, exit: &xf }, from, to)
    };
    Some(match f {
        "3dhl" => {
            let (ma, days) = (x.sma_n(u("ma")), u("days"));
            let ent = |i: usize| {
                i >= days && trend(i) && beyond_ma(c[i], ma[i])
                    && (i + 1 - days..=i).all(|k| if long { h[k] < h[k - 1] && l[k] < l[k - 1] } else { h[k] > h[k - 1] && l[k] > l[k - 1] })
            };
            run_spec(w, &ent, &worse, &|i| beyond_ma(ma[i], c[i]).then(|| "Close back across the 5-day SMA".into()))
        }
        "rsi25" => {
            let (rs, en, ex, ad) = (x.rsi_n(u("rsi_n").max(2)), g("entry"), g("exit"), g("add"));
            run_spec(w, &|i| trend(i) && beyond(rs[i], en), &|i, _| beyond(rs[i], ad),
                     &|i| beyond(ex, rs[i]).then(|| format!("RSI {:.1} beyond {ex}", rs[i])))
        }
        "r3" => {
            let (days, first, en, ex, prior) = (u("days"), g("first"), g("entry"), g("exit"), g("first_ref") >= 0.5);
            let r = &x.rsi2;
            let ent = |i: usize| {
                i > days && trend(i) && (i + 1 - days..=i).all(|k| beyond(r[k], r[k - 1]))
                    && beyond(if prior { r[i - days] } else { r[i + 1 - days] }, first) && beyond(r[i], en)
            };
            run_spec(w, &ent, &worse, &|i| beyond(ex, r[i]).then(|| format!("RSI(2) {:.1} beyond {ex}", r[i])))
        }
        "pctb" => {
            let (pb, en, days, ex) = (x.pctb_n(u("n")), g("entry"), u("days"), g("exit"));
            run_spec(w, &|i| i + 1 >= days && trend(i) && (i + 1 - days..=i).all(|k| beyond(pb[k], en)), &|i, _| beyond(pb[i], en),
                     &|i| beyond(ex, pb[i]).then(|| format!("%b {:.2} beyond {ex}", pb[i])))
        }
        "mdd" => {
            let (ma, k, of) = (x.sma_n(u("ma")), u("k"), u("of"));
            let ent = |i: usize| i >= of && trend(i) && beyond_ma(c[i], ma[i]) && (i + 1 - of..=i).filter(|&j| beyond(c[j], c[j - 1])).count() >= k;
            run_spec(w, &ent, &worse, &|i| beyond_ma(ma[i], c[i]).then(|| "Close back across the 5-day SMA".into()))
        }
        "rsi10" => {
            let (r, en, ad, ma) = (&x.rsi2, g("entry"), g("add"), x.sma_n(u("ma")));
            run_spec(&two, &|i| trend(i) && beyond(r[i], en), &|i, _| beyond(r[i], ad), &|i| beyond_ma(ma[i], c[i]).then(|| "Close back across the 5-day SMA".into()))
        }
        _ => {
            let (r, en, days, ex) = (&x.rsi2, g("entry"), u("days"), g("exit"));
            let last_worse = |i: usize, t: &Trade| trend(i) && beyond(c[i], t.legs.last().unwrap().px);
            run_spec(&[0.1, 0.2, 0.3, 0.4], &|i| i + 1 >= days && trend(i) && (i + 1 - days..=i).all(|k| beyond(r[k], en)), &last_worse,
                     &|i| beyond(ex, r[i]).then(|| format!("RSI(2) {:.1} beyond {ex}", r[i])))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(b: &[(f64, f64, f64)]) -> Vec<Row> {
        b.iter().enumerate().map(|(i, &(h, l, c))| Row { ticker: "T".into(), date: format!("{:04}-01-01", 1000 + i), open: c, high: h, low: l, close: c, volume: None }).collect()
    }
    /// 3-Day High/Low long + aggressive add, by hand.
    #[test]
    fn three_day_high_low_by_hand() {
        let mut b: Vec<(f64, f64, f64)> = (0..210).map(|i| { let c = 100.0 + i as f64; (c + 1.0, c - 1.0, c) }).collect();
        // three lower highs and lower lows, close under the 5-day SMA -> buy bar 212; lower close bar 213 -> add; jump -> exit 214
        b.extend([(309.0, 305.0, 306.0), (308.0, 303.0, 304.0), (307.0, 301.0, 302.0), (306.0, 299.0, 300.0), (330.0, 310.0, 329.0)]);
        let r = rows(&b);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("hpetf_3dhl_long", &x, 0, b.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (212, 214));
        let t = run("hpetf_3dhl_long_aggr", &x, 0, b.len() - 1, &Params::new()).unwrap();
        assert_eq!(t[0].legs.iter().map(|l| (l.i, l.w)).collect::<Vec<_>>(), vec![(212, 0.5), (213, 0.5)]);
    }
    /// Seminar RSI 14 TPS: 20/30/50 adds on lower closes (below the previous purchase), exit RSI(2) > 65; checked against
    /// the conditions recomputed with the crate's indicators (the full ledger check is in connors/newrules-misc).
    #[test]
    fn seminar_rsi14_tps_scales_in_and_exits() {
        let mut b: Vec<(f64, f64, f64)> = (0..60).map(|i| { let c = 100.0 + (i % 2) as f64; (c + 1.0, c - 1.0, c) }).collect();
        for k in 0..12 { let c = 99.0 - 2.0 * k as f64; b.push((c + 1.0, c - 1.0, c)); }
        b.extend([(80.0, 76.0, 79.0), (90.0, 80.0, 89.0), (95.0, 89.0, 94.0)]);
        let r = rows(&b);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(SEM_RSI14, &x, 0, b.len() - 1, &Params::new()).unwrap();
        let (r14, r2) = (crate::connors::rsi(&x.c, 14), &x.rsi2);
        let first = (0..b.len()).find(|&i| r14[i] < 35.0 && r2[i] < 30.0).unwrap();
        assert_eq!(t[0].legs.iter().map(|l| (l.i, l.w)).collect::<Vec<_>>(), vec![(first, 0.2), (first + 1, 0.3), (first + 2, 0.5)]);
        let exit = (first + 1..b.len()).find(|&i| r2[i] > 65.0).unwrap();
        assert_eq!((t[0].exit_i, t[0].open), (exit, false));
        // the 200-day option (close under the 200-day SMA) cannot pass without 200 bars
        assert!(run(SEM_RSI14, &x, 0, b.len() - 1, &Params::from([("trend_filter".to_string(), 1.0)])).unwrap().is_empty());
        assert!(run(SEM_PAIRS, &x, 0, b.len() - 1, &Params::new()).is_none());
    }
}
