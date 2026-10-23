//! Jake Bernstein, "Strategies for the Electronic Futures Trader" (McGraw-Hill, 2000). Book id `jbeft`.
//! Source: a 258-page scan, read as page images (printed page = PDF page - 17). Transcription and checks: library/jbeft/.
//! The systems of ch.7-11 are built on 15- or 60-minute (or 1-minute) S&P data; the only daily-bar systems are the
//! S&P day-of-week systems of ch.8, which use the DBO simulator in `jbcdt2`.
use super::jb624::getp;
use super::jbcdt2::dbo;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbeft";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const DOW_RULES: &str = "Setup: \"If the close on Friday is greater than the open on Friday, then a buy condition is set for Monday ... less than the open ... a sell condition\"; trigger: \"on Monday buy on stop x ticks above the Friday high ... sell short on stop x ticks below the Friday low\"; \"Exit either at the indicated stop loss or on the first profitable opening\" (p.131); FPO = \"the first opening after you enter that gives you a profit compared to your entry price\" (p.132). Friday = last trading day of the week, Monday = the next trading day [the book does not cover holidays]; the stop counts on the entry day only if the bar path reaches it after the fill [daily-bar convention].";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        StrategyDef { claim: crate::connors::claim("50", "78%", "$1,066.50 (1 S&P contract)", "2 bars (winners)", "S&P futures 11/03/97-08/31/99, PF 1.87"),
          ..def!("jbeft_dow_fm97", "S&P day-of-week Friday-Monday · 1997-99 inputs (4 / 2 ticks, $5,500)", "Ch.8 · The S&P Day-of-Week System", "Long/Short", Live,
            &["F|Friday close versus Friday open sets up Monday (p.126, 131)",
              "E|\"a 4-tick penetration of Friday's high on Monday for buy signals and a 2-tick penetration of Friday's low on Monday for sell signals\" (p.127) = 0.40 / 0.20 S&P points [Not from the book: the book never states the tick size; 0.10 point is the contract specification of the $250-per-point contract]",
              "X|\"with a $5500 stop loss\" (p.127) = 22 S&P points (\"using the $250 per 100 points S&P value\", p.127; every Fig 8-6 loss is -$5,575); \"Remember that exit is on FPO\" (p.127)"],
            DOW_RULES) },
        StrategyDef { claim: crate::connors::claim("384", "80%", "$370.12 (1 S&P contract)", "2 bars (winners)", "S&P futures 4/21/82-10/31/97, PF 1.58"),
          ..def!("jbeft_dow_wt", "S&P day-of-week Wednesday-Thursday (6 / 22 ticks, $3,850)", "Ch.8 · The S&P Day-of-Week System", "Long/Short", Live,
            &["F|\"Figure 8-7 shows the Wednesday-Thursday relationship in S&P from 1982 to 1997\" (p.127): Wednesday's close versus open sets up Thursday",
              "E|\"based on a 6-tick penetration of Wednesday's high on Thursday and a 22-tick penetration of Wednesday's low on Thursday\" (p.131) = 0.30 / 1.10 S&P points [Not from the book: 0.05-point tick of the $500-per-point contract, from the contract specification]",
              "X|\"with a $3850 stop loss\" (p.131) = 7.7 S&P points at $500 per point; exit on the first profitable opening (p.131)"],
            "Needs a Wednesday bar followed by a Thursday bar. Fills, entry-day stop and FPO as jbcdt2_dbo_thu (CDT II prints other Thursday inputs: 6 / 21 ticks, $3,500).") },
        def!("jbeft_dow_fm82", "S&P day-of-week Friday-Monday · 1982-97 inputs (3 / 26 ticks, $3,400)", "Ch.8 · The S&P Day-of-Week System", "Long/Short", NotTestable("duplicate: identical inputs to jbcdt2_dbo_mon (CDT II Figure 5-8); this book's 1982-97 record (392 trades, 78%, $425.57) is compared there"),
            &["E|\"the trade trigger on Monday a 3-tick penetration of Friday's high for buy signals and a 26-penetration of Friday's low for sell signals, with a $3400 stop loss\" (p.127)"], ""),
        def!("jbeft_parabolic", "Parabolic system for short-term trading", "Ch.7 · Two Trend-Following Strategies", "Long/Short", NeedsData("15- or 60-minute S&P bars (the system is specified only on intraday data, p.107-109)"),
            &["E|\"Buy when price penetrates its parabolic buy number. Sell short or reverse position when price penetrates its parabolic sell number\" (p.107)",
              "X|Money management stop, floor level, trailing stop of x% of open profit (p.107); 15-minute inputs: AF .0171 buy / .0015 sell, $5,000 stop, $2,500 floor, 62% trail; 60-minute: AF .026 / .006, $3,500 long / $5,300 short, $2,600 floor, 90% trail (p.109)"],
            "Book: 101 trades 72% $1,342.57 (15-min, 10/7/98-3/18/99); 117 trades 70% $1,129.70 (60-min, 11/27/97-3/18/99)."),
        def!("jbeft_mom_ma", "MOM/MA system (60-minute S&P)", "Ch.7 · Two Trend-Following Strategies", "Long/Short", NeedsData("60-minute S&P bars"),
            &["E|\"If momentum crosses above its MA for at least two consecutive postings, buy. If momentum crosses below its MA for at least two consecutive postings, sell\" (p.113); momentum 16, MA 19 (p.114)",
              "X|$3,100 long / $4,400 short stop, $2,100 floor, 66% trail (p.119)"], "Daily MOM/MA versions: jbsmsw_mom_ma, jbssf_mom_ma. Book: 176 trades 60% $705.68 (Fig 7-10)."),
        def!("jbeft_mom_rise_fall", "Rising and falling momentum (60-minute S&P)", "Ch.7 · Two Trend-Following Strategies", "Long/Short", NeedsData("60-minute S&P bars; the rule itself is only named (\"Buy signal MA: 7 periods of the closing price; Sell signal MA: 28 periods\", p.119)"),
            &["E|\"use rising and falling momentum ... to generate buy and sell signals without using the moving average of momentum\" (p.119)"], "Book: 171 trades 65% $940.50 (Fig 7-11)."),
        def!("jbeft_oc_osc", "Open/close oscillator system (15-minute S&P)", "Ch.8 · Two Market Pattern Systems", "Long/Short", NeedsData("15-minute S&P bars"),
            &["E|\"when the MA of the open minus the MA of the close crosses above its moving average, the market tends to move higher\" (p.123); MA of open 46, MA of close 34, MA of difference 39",
              "X|$3,700 long / $3,500 short stop, $3,900 floor, 75% trail (p.123)"], "Daily eight-bar open/close: jb30d_oc8. Book: 135 trades 62% $858.70 (Fig 8-4)."),
        def!("jbeft_channel_break", "Channel break system (60-minute S&P)", "Ch.9 · Two Breakout Systems", "Long/Short", NeedsData("60-minute S&P bars (intrabar breakout of 8-bar high / 12-bar low)"),
            &["E|\"buys and sells on any intraday or intrabar penetration\"; bars of high 8, bars of low 12 (p.133-134)",
              "X|$3,400 long / $2,900 short stop, $1,500 floor, 90% trail (p.134)"], "Book: 136 trades 75% $1,030.88 (Fig 9-2)."),
        def!("jbeft_range_breakout", "Range breakout system", "Ch.9 · Two Breakout Systems", "Long/Short", NotTestable("\"Percentage penetration: 96\" of the 15-bar window and the 21-bar \"momentum filter\" are not defined, and no initial stop is given (p.139)"),
            &["E|\"a volatility breakout system that buys or sells on a penetration above or below the true range of a give number of price bars\" (p.139)"], "Book: 72 trades 72% $1,154.17, hourly data (Fig 9-3)."),
        def!("jbeft_3_3_channel", "3/3 high/low channel with momentum trend", "Ch.10 · Support and Resistance", "Long/Short", NotTestable("the author: \"The 3/3 channel method is not a trading system per se\" (p.153); its rules conflict on bar size (\"Be out of all trades by the end of the day\", p.144, versus \"designed for short-term trading on daily data\", p.153), and the momentum length behind the trend (14, p.141-142) comes with no entry order rule beyond \"buy at support\""),
            &["E|\"If momentum has been above the zero line for at least two postings, then the trend is considered up. If the trend is up, then buy at support\" = 3-period MA of lows; \"take profit at resistance\" = 3-period MA of highs (p.144)",
              "X|\"Use a stop loss that is 100 percent of the average of the last 3 trading ranges-close only\"; \"Be out of all trades by the end of the day\" (p.144)"], "The daily 3-bar high/low channel with a MACD trend is jb624_3hl."),
        def!("jbeft_dsi", "Daily Sentiment Index extremes", "Ch.12 · Additional Methods", "Long/Short", NeedsData("Daily Sentiment Index history (MBH), plus intraday timing"),
            &["E|\"I prefer to use the 90 and 10 percent levels as trigger points\"; best \"looking for short-term sell signals on intraday data\" (p.169)"], ""),
        def!("jbeft_gold_kd", "October gold key date · long Sep 4 to Sep 24, 4% stop", "Ch.4 · Seasonality", "Long", NeedsData("October gold futures contract histories (the seasonal belongs to gold; the vault has no metal futures); the book also does not say whether fills are at the open or the close"),
            &["F|Figure 4-1 (p.62): \"OCT GOLD LONG Enter: 9/4 Exit: 9/24 Stop: 4\"; 24 trades 1975-98, 79.17%"], ""),
        def!("jbeft_add", "Advance/decline derivative (ADD)", "Ch.12 · Additional Methods", "Long/Short", NotTestable("\"objective, but not entirely systematic ... you will need to add a risk management stop loss\" (p.173); the daily ADD of another book is jbsmsw_add"),
            &["E|A/D oscillator against its moving average (p.172-173)"], ""),
    ]
}

/// Admission (bernstein-g3, 2026-10-22): the two daily S&P day-of-week systems, on the S&P 500 index itself only (the
/// offsets and stops are S&P points) and only where its opens are real (stop entries and first-profitable-opening exits).
pub const REVIEWED: &[&str] = &["jbeft_dow_fm97", "jbeft_dow_wt"];
pub fn reviewed_daily(id: &str) -> bool {
    REVIEWED.contains(&id)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jbeft_dow_fm97" => ("jbeft_dow", "S&P day-of-week system (EFT)", "Friday-Monday · 1997-99 inputs"),
        "jbeft_dow_wt" => ("jbeft_dow", "S&P day-of-week system (EFT)", "Wednesday-Thursday"),
        "jbeft_dow_fm82" => ("jbeft_dow", "S&P day-of-week system (EFT)", "Friday-Monday 1982-97 · see jbcdt2_dbo_mon"),
        "jbeft_parabolic" | "jbeft_mom_ma" | "jbeft_mom_rise_fall" | "jbeft_oc_osc" | "jbeft_channel_break" => ("jbeft_intraday", "EFT intraday systems", "Needs intraday bars"),
        "jbeft_3_3_channel" => ("jbeft_other", "EFT other methods", "Not testable / needs data"),
        "jbeft_range_breakout" | "jbeft_dsi" | "jbeft_add" | "jbeft_gold_kd" => ("jbeft_other", "EFT other methods", "Not testable / needs data"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let dbo = |b, s, st| vec![p("buy_off", "Buy stop above the setup day's high (points)", b, 0., 10., 0.05), p("sell_off", "Sell stop below the setup day's low (points)", s, 0., 10., 0.05),
        p("stop", "Stop loss from the entry (points)", st, 1., 40., 0.1), p("entry_day", "Entry-day stop: 0 = bar path O-near-far-C, 1 = any touch", 0., 0., 1., 1.)];
    Some(match id {
        "jbeft_dow_fm97" => dbo(0.4, 0.2, 22.0),
        "jbeft_dow_wt" => dbo(0.3, 1.1, 7.7),
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jbeft_") || params(id).is_none() || to >= x.c.len() || from > to {
        return None;
    }
    let g = |k: &str| getp(id, p, k);
    Some(dbo(x, from, to, id == "jbeft_dow_wt", g("buy_off"), g("sell_off"), g("stop"), g("entry_day") >= 0.5))
}
