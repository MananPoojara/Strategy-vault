//! Trading Connors VIX Reversals (Larry Connors & Connie Che, 2001). Book id "vixr".
//! CVR 1-11 (CVR 4 was "removed in 1998" and has no rules). Every signal is read on the VIX daily bars of the traded instrument's trading days (param vix_cal),
//! the market is bought / sold on the close of the signal day, the position is held a fixed number of days
//! (CVR 3: until the VIX trades back through yesterday's 10-day MA) and an opposite CVR signal stops and reverses.
//!
//! Engine conventions reproduced from the book's TradeStation results (see library/vixr/implementation-report.md):
//!   * one position; opposite signal = exit on the close and enter the other way on the same close
//!     (book p-040: "When this occurs on any CVR signals, you need to stop and reverse");
//!   * N-day exit = close of the N-th bar after entry; if a same-direction signal fires on that exit bar the
//!     trade stays open and the count restarts (TradeStation nets the exit against the new entry). With this
//!     reading 96 of the 97 published CVR II trades (Connors on Advanced Trading Strategies appendix) match date for date;
//!   * a same-direction signal on any other bar is ignored (1 contract max, as the reports show).
//!
//! VIX OHLC: the shared `VixAligned` only exposes derived VIX facts, so this module reads the VIX directory the
//! server was started with (argv[3], same loader `crate::load_vix_dir`) once and aligns it by date.
#![allow(unused_imports, dead_code)]
use crate::connors::*;
use crate::Row;
use std::collections::HashMap;
use std::sync::OnceLock;

pub const BOOK: &str = "vixr";

/// Exact daily CVR contracts admitted after the 2026-10-21 source review (pilot-vixr/SOURCE-CONTRACT.md) and an
/// independent chronological Python ledger check on actual SPY/SPX (+ NIFTY 50 where listed below).
/// Declared index-timing application of an S&P-futures study: no futures dollars or book performance.
pub const REVIEWED_DAILY: &[&str] = &[
    "vixr_cvr1", "vixr_cvr2", "vixr_cvr3", "vixr_cvr5_1d", "vixr_cvr5_3d", "vixr_cvr6",
    "vixr_cvr7", "vixr_cvr8", "vixr_cvr9", "vixr_cvr10", "vixr_cvr11",
];
pub fn reviewed_daily(id: &str) -> bool { REVIEWED_DAILY.contains(&id) }
/// Rules that read only India VIX close/high/low with scale-free thresholds. Rules reading the VIX open
/// (India VIX open ~ prior close) or fixed VIX-point thresholds are not admitted on NIFTY 50.
pub const INDIA_VIX_REVIEWED: &[&str] = &["vixr_cvr2", "vixr_cvr3", "vixr_cvr6"];
/// Actual SPX (cash index, declared timing adaptation) and actual SPY with CBOE VIX; NIFTY 50 with India VIX
/// only for `INDIA_VIX_REVIEWED`. Other indices/ETFs/stocks are not reviewed applications.
pub fn admitted_symbol(id: &str, ticker: &str) -> bool {
    reviewed_daily(id) && (matches!(ticker, "SPX" | "SPY") || (ticker == "NIFTY 50" && INDIA_VIX_REVIEWED.contains(&id)))
}
/// The served CBOE VIX file holds close-only bars (open = high = low = close) for 1990-1991.
pub const REAL_VIX_OHLC_FROM: &str = "1992-01-01";

/// Strategy note + the shared book note (+ the VIX-open caveat for rules that read the VIX open).
macro_rules! note {
    ($s:literal, open_flag) => {
        concat!("INDIA: read with care: on India VIX \"VIX close vs its open\" is effectively \"close vs yesterday's close\" (open ~ prior close). ", note!($s, open))
    };
    ($s:literal) => {
        concat!($s, "Book: 1 S&P 500 futures contract, signals on the original CBOE VIX (OEX-based, today's VXO), 1/28/93-8/6/01, no stops, no costs. Here the traded index is the ticker you pick and the VIX is the server's VIX (India VIX for NIFTY; CBOE VIX for US ETFs). Long and short in one position: an opposite CVR signal stops and reverses.")
    };
    ($s:literal, open_void) => {
        concat!("INDIA: NOT MEANINGFUL on India VIX (its open is ~ the prior close, so the open-in-range test is degenerate; NIFTY gives ~289 S / 45 L). ", note!($s, open))
    };
    ($s:literal, open) => {
        concat!(note!($s), " Uses the VIX OPEN: India VIX opens almost exactly at the prior close (opens above the prior high on 0.28% of days vs 19.67% for the CBOE VIX), so on Indian data \"VIX close vs its open\" behaves like \"VIX close vs yesterday's close\" and the open-in-range tests are degenerate; kept exactly as written (it still fires on NIFTY, see the report).")
    };
}

const PERIOD: &str = "1/28/93 - 8/6/01, 1 S&P futures contract";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, status, rules, notes, claim| StrategyDef {
        id, name, group: "Connors VIX Reversals", session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: true,
        needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("vixr_cvr1", "CVR 1 · VIX 5-day extreme reverses", Live,
          &["E|Buy: today the VIX must make a five-day high (VIX high = highest high of the last 5 days)",
            "E|Buy: today, the close of the VIX must be below the open",
            "E|Sell: today the VIX must make a five-day low and the VIX close must be above the open",
            "E|Buy (sell) the S+Ps on the close of today",
            "X|Hold the position at least one to three days: exit on the close 3 days after entry (code EXBAR = 3)",
            "X|An opposite CVR 1 signal stops and reverses"],
          note!("Code input LENGTH = 5 (the original CVR I used 15: set len = 15). Five-day high/low = VIX intraday high/low incl. today. ", open_flag),
          claim("255 (165 L / 90 S)", "58.0%", "$541.97", "3 bars", PERIOD)),
        d("vixr_cvr2", "CVR 2 · RSI(5) of the VIX turns from an extreme", Live,
          &["E|Take a five-period RSI of the closing VIX",
            "E|Buy: yesterday's RSI was 70 or above and today's RSI is lower (downtick)",
            "E|Sell: yesterday's RSI was 30 or below and today's RSI is higher (uptick)",
            "E|Buy (sell) the market that day on the close",
            "X|Exit two to three trading days later: close 2 days after entry (Section Four: two days for CVR 2)",
            "X|An opposite CVR 2 signal stops and reverses"],
          note!("RSI = Wilder (TradeStation RSI). The TradeStation input EXITDAYS defaults to 7 and the earlier CVR II test used an 8-day exit (hold = 8 reproduces its trade list); this book's report (avg 2 bars) and Section Four use 2 days. "),
          claim("239 (125 L / 114 S)", "56.5%", "$662.17", "2 bars", PERIOD)),
        d("vixr_cvr3", "CVR 3 · VIX 10% from its 10-day MA", Live,
          &["E|Buy: today, the low of the VIX must be above its 10-day moving average",
            "E|Buy: today, the VIX must close at least 10% above its 10-day moving average",
            "E|Sell: the high of the VIX below its 10-day MA and the close at least 10% below it",
            "E|Buy (sell) the market on the close",
            "X|Exit (on the close) the day the VIX trades (intraday) below (sells: above) yesterday's 10-day moving average (strictly: low < MA, high > MA)",
            "X|[Not from the book] exit_touch = 1: a touch also exits (low <= MA, high >= MA); fitted to the ATS CVR III list, which exits on the 1993-11-11 tie. Default 0 = the book's \"below\" / \"above\"",
            "X|An opposite CVR 3 signal stops and reverses"],
          note!("Entry compares today's VIX low / close with today's 10-day MA, as the rules say (\"its 10-day moving average\"; only the exit says \"yesterday's\"), param ma_lag = 0. [Not from the book] ma_lag = 1 (yesterday's MA) is fitted to the published CVR III list of the ATS book: 73 of its 75 trades vs 52 with today's MA. No time cap (\"or exit within two to four days\" is an alternative; max_hold > 0 adds it). "),
          claim("162 (89 L / 73 S)", "68.5%", "$1,477.47", "4 / 6 bars (win / loss)", PERIOD)),
        d("vixr_cvr5_1d", "CVR 5 · VIX opens & closes at a range extreme (1-day exit)", Live,
          &["E|Buy: today's VIX must open and close in the bottom third of its trading range",
            "E|Buy: the close of the VIX must be above its 10-day moving average",
            "E|Sell (reversed): VIX opens and closes in the top third of its range and closes below its 10-day MA",
            "E|Buy (sell) the market on the close",
            "X|Exit on the close of the next day (book's 1-day exit test)",
            "X|An opposite CVR 5 signal stops and reverses"],
          note!("Zero-range VIX days give no signal. ", open_void),
          claim("118", "61.0%", "$379.94", "1 bar", PERIOD)),
        d("vixr_cvr5_3d", "CVR 5 · VIX opens & closes at a range extreme (3-day exit)", Live,
          &["E|Buy: today's VIX must open and close in the bottom third of its trading range",
            "E|Buy: the close of the VIX must be above its 10-day moving average",
            "E|Sell (reversed): VIX opens and closes in the top third of its range and closes below its 10-day MA",
            "E|Buy (sell) the market on the close",
            "X|Exit on the close 3 days after entry (book's 3-day exit test)",
            "X|An opposite CVR 5 signal stops and reverses"],
          note!("Zero-range VIX days give no signal. ", open_void),
          claim("111 (82 L / 29 S)", "59.5%", "$877.39", "3 bars", PERIOD)),
        d("vixr_cvr6", "CVR 6 · VIX 20-day extreme two of three days", Live,
          &["E|Buy: the VIX must make a new 20-period high two out of the past three days",
            "E|Sell: the VIX must make a new 20-period low two out of the past three days",
            "E|New 20-period high = that day's VIX high is the highest of the last 20 days incl. itself (ref = 0, the literal reading)",
            "E|[Not from the book] ref = 1: high above the 20-day high set before those 3 days (fitted to the report's trade count)",
            "E|Buy (sell) the market on the close",
            "X|Exit in two to three days (p.40): close 3 days after entry, the published CVR 6 test (p.42 report: winners average 3 bars)",
            "X|Option hold = 2: Section Four (p.82) portfolio rule \"exited in three days (two days for the CVR 2, 6 and 7)\"",
            "X|An opposite CVR 6 signal stops and reverses"],
          note!("Exit: the rule says \"Exit in two to three days\" (p.40). Default hold = 3 = the published CVR 6 test (p.42 report \"CVR 6 VIXdouble Top/Bot\": 144 trades, 62.5%, avg $1,048.33, winners average 3 bars; in every CVR report the winners' average bars equals the fixed exit), so the claim below belongs to this default. Option hold = 2 = Section Four's portfolio rule (p.82, \"two days for the CVR 2, 6 and 7\"). The TradeStation input EXT_DAYS defaults to 5 (used by neither). New 20-period high = that day's VIX high is the highest of the last 20 days incl. itself (param ref = 0, the text as written; TradeStation inputs HILODAYS = 20, _2_3DAYS = 3). Reference model on SPX with VXO, 1/28/93-8/6/01, close to close: ref 0 hold 3 = 102 trades (49 L / 53 S), 58.8% (hold 2: 103, 49 / 54, 59.2%); book 144 (72 / 72), 62.5%. [Not from the book] ref = 1 (above the highest high of the 20 days before the 3-day window) is fitted to the report's trade count, not stated in the text: hold 3 = 131 trades (66 L / 65 S), 59.5% (hold 2: 137, 69 / 68, 59.9%). p.43 summary: \"this simple situation has occurred 131 times ... and has correctly identified two-day market reversals about 62% of the time\" (not the report's 144 trades). "),
          claim("144 (72 L / 72 S)", "62.5%", "$1,048.33", "3 / 4 bars (win / loss)", PERIOD)),
        d("vixr_cvr7", "CVR 7 · Flushing out the weak money", Live,
          &["E|Buy: the VIX close of two days ago above the 10-period MA made three days ago",
            "E|Buy: yesterday's VIX close above the 10-period MA made two days ago",
            "E|Buy: today's VIX close above yesterday's 10-period MA",
            "E|Buy: the VIX close of two days ago below its open; yesterday's VIX close above its open",
            "E|Buy: yesterday's VIX high above the high of the previous day or of the day before that",
            "E|Buy: today's VIX close below today's VIX open",
            "E|Sell: the mirror (closes below the MAs, 2 days ago close > open, yesterday close < open, yesterday's low below the previous day's low or the one before, today close > open)",
            "E|Buy (sell) the market on close",
            "X|Exit in two days (close 2 days after entry, code EXITBAR = 2)",
            "X|An opposite CVR 7 signal stops and reverses"],
          note!("Rule 6 uses \"or\" as printed (sell rule 6 corrected by hand in the book: \"See Buy Rule 6\"). ", open_flag),
          claim("92 (72 L / 20 S)", "68.5%", "$1,740.46", "2 bars", PERIOD)),
        d("vixr_cvr8", "CVR 8 · VIX price oscillator", Live,
          &["E|Oscillator = (today's 10-period EMA of the VIX - yesterday's) - (today's 100-period EMA - yesterday's)",
            "E|Buy: today's oscillator is the highest of the past 40 trading days and above 0.20",
            "E|Sell: the oscillator is the lowest of the past 40 trading days and below -0.20",
            "E|Buy (sell) on the close",
            "X|Exit in three days (close 3 days after entry)",
            "X|An opposite CVR 8 signal stops and reverses"],
          note!("EMA = TradeStation XAverage (2/(n+1)), seeded at the first VIX close; no trades in the first 100 VIX bars. 0.20 is in VIX points (India VIX trades at similar levels). "),
          claim("103 (52 L / 51 S)", "66.0%", "$1,974.27", "3 bars", PERIOD)),
        d("vixr_cvr9", "CVR 9 · VIX range expansion reversal", Live,
          &["E|Buy: today's VIX range is the largest range of the past three days",
            "E|Buy: today's VIX high is the highest of the past 10 days",
            "E|Buy: yesterday's VIX close is greater than yesterday's open; today's VIX close is less than the open",
            "E|Sell: largest range of 3 days, VIX low the lowest of 10 days, yesterday close < open, today close > open",
            "E|Buy (sell) on the close",
            "X|Exit in two to three days: close 3 days after entry (report: 3 bars)",
            "X|An opposite CVR 9 signal stops and reverses"],
          note!("Range = VIX high - low; largest = strictly larger than each of the 2 prior days. ", open_flag),
          claim("74 (56 L / 18 S)", "64.9%", "$1,413.21", "3 bars", PERIOD)),
        d("vixr_cvr10", "CVR 10 · Tying in S&P prices", Live,
          &["E|Buy: yesterday's VIX close greater than yesterday's VIX open",
            "E|Buy: yesterday's market close less than yesterday's open",
            "E|Buy: today's VIX close less than today's VIX open",
            "E|Buy: today's VIX high must be the highest of eight days",
            "E|Sell: the mirror (VIX yesterday close < open, market yesterday close > open, VIX today close > open, VIX low the lowest of 8 days)",
            "E|Buy (sell) on the close",
            "X|Exit in two to three days: close 3 days after entry (report: 3 bars)",
            "X|An opposite CVR 10 signal stops and reverses"],
          note!("\"Market\" = the traded index's own open and close (the book used S&P futures). ", open_flag),
          claim("135 (87 L / 48 S)", "66.7%", "$1,711.37", "3 bars", PERIOD)),
        d("vixr_cvr11", "CVR 11 · VIX 3/10 oscillator", Live,
          &["E|Oscillator = 3-period MA of the VIX - 10-period MA of the VIX",
            "E|Buy: today the VIX must make a 10-period high; yesterday's oscillator must be one or greater",
            "E|Sell: today the VIX makes a 10-period low; yesterday's oscillator -1 or less",
            "E|Buy (sell) on the close",
            "X|Exit in two to three days: close 3 days after entry (examples: \"three days later\")",
            "X|An opposite CVR 11 signal stops and reverses"],
          note!("Simple MAs of VIX closes; 10-period high/low = VIX intraday high/low incl. today. The examples check the oscillator on the signal day; the rule (yesterday's) is coded and reproduces the report better. ±1 is in VIX points. "),
          claim("120 (58 L / 62 S)", "66.7%", "$2,127.65", "3 bars", PERIOD)),
        d("vixr_all", "All CVR signals together (Section Four)", NotTestable("multi-contract portfolio: one contract per signal, positions overlap; the engine runs one position per ticker"),
          &[], "\"one S&P ($250/point) for each signal (multiple signals trade multiple contracts equal to the number of signals) with no stops ... and exited in three days (two days for the CVR 2, 6 and 7)\": over $1.8 million 1/1/93-8/6/01, nearly 3 of 4 months profitable. Each CVR is coded on its own above.",
          claim("~1300 signals", "~65%", "-", "2-3 days", "1/1/93 - 8/6/01")),
        d("vixr_stocks", "Trading CVR signals with stocks", NotTestable("discretionary"),
          &[], "On a CVR buy, buy SPY or the strongest heavy-weight stocks by relative strength (sell the weakest on sells). No relative-strength formula or universe is given.", None),
        d("vixr_options", "Trading CVR signals with options", NotTestable("needs option chains"),
          &[], "Buy signals: sell OEX puts / put spreads; no mechanical rules.", None),
    ]
}

/// Tunable numbers of each strategy (book value = default). None = not a strategy of this book.
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let hold = |def| p("hold", "Exit on the close N days after entry", def, 1.0, 10.0, 1.0);
    let mut v = match id {
        "vixr_cvr1" => vec![p("len", "VIX high/low of N days", 5.0, 2.0, 20.0, 1.0), hold(3.0)],
        "vixr_cvr2" => vec![p("rsi_n", "RSI period (VIX close)", 5.0, 2.0, 14.0, 1.0), p("ob", "Buy: yesterday's RSI at or above", 70.0, 55.0, 90.0, 5.0),
            p("os", "Sell: yesterday's RSI at or below", 30.0, 10.0, 45.0, 5.0), hold(2.0)],
        "vixr_cvr3" => vec![p("ma", "VIX moving average length", 10.0, 5.0, 20.0, 1.0), p("pct", "VIX close at least % beyond the MA", 10.0, 3.0, 20.0, 1.0),
            p("ma_lag", "Entry MA as of N days ago (0 = today's, book p.31; 1 = yesterday's, fitted to the ATS list)", 0.0, 0.0, 1.0, 1.0), p("max_hold", "Also exit after N days (0 = off)", 0.0, 0.0, 10.0, 1.0),
            p("exit_touch", "Exit also on a touch of yesterday's MA (0 = strictly below/above, book p.31; 1 = <= / >=, fitted to the ATS list)", 0.0, 0.0, 1.0, 1.0)],
        "vixr_cvr5_1d" | "vixr_cvr5_3d" => vec![p("ma", "VIX moving average length", 10.0, 5.0, 20.0, 1.0),
            p("zone", "Open & close within % of the range from the extreme", 100.0 / 3.0, 10.0, 50.0, 5.0), hold(if id == "vixr_cvr5_1d" { 1.0 } else { 3.0 })],
        "vixr_cvr6" => vec![p("hilo", "New N-period VIX high/low", 20.0, 5.0, 40.0, 1.0), p("need", "…on at least this many days", 2.0, 1.0, 3.0, 1.0),
            p("look", "…of the past N days", 3.0, 2.0, 5.0, 1.0),
            p("ref", "New high/low vs: 1 = the N days before the window (fitted), 0 = last N days incl. that day (literal, book p.40)", 0.0, 0.0, 1.0, 1.0),
            p("hold", "Exit on the close N days after entry (3 = published CVR 6 test, p.42; 2 = Section Four p.82)", 3.0, 1.0, 10.0, 1.0)],
        "vixr_cvr7" => vec![p("ma", "VIX moving average length (LEN)", 10.0, 5.0, 20.0, 1.0), hold(2.0)],
        "vixr_cvr8" => vec![p("fast", "Fast EMA", 10.0, 5.0, 20.0, 1.0), p("slow", "Slow EMA", 100.0, 50.0, 200.0, 10.0),
            p("look", "Oscillator highest/lowest of N days", 40.0, 10.0, 60.0, 5.0), p("thr", "Oscillator beyond ± (VIX points)", 0.2, 0.0, 1.0, 0.05), hold(3.0)],
        "vixr_cvr9" => vec![p("rng_n", "Largest VIX range of N days", 3.0, 2.0, 5.0, 1.0), p("hl_n", "VIX high/low of N days", 10.0, 5.0, 20.0, 1.0), hold(3.0)],
        "vixr_cvr10" => vec![p("hl_n", "VIX high/low of N days", 8.0, 5.0, 20.0, 1.0), hold(3.0)],
        "vixr_cvr11" => vec![p("fast", "Fast MA", 3.0, 2.0, 5.0, 1.0), p("slow", "Slow MA", 10.0, 6.0, 20.0, 1.0), p("thr", "Yesterday's oscillator beyond ± (VIX points)", 1.0, 0.25, 3.0, 0.25),
            p("hl_n", "VIX high/low of N days", 10.0, 5.0, 20.0, 1.0), hold(3.0)],
        "vixr_all" | "vixr_stocks" | "vixr_options" => return Some(vec![]),
        _ => return None,
    };
    v.push(p("vix_cal", "VIX calendar: 0 = the VIX bars on the traded instrument's days only (holiday VIX bars dropped, as ATS / PTJ / S&P MT), 1 = [Not from the book] previous reading: the VIX's own calendar", 0.0, 0.0, 1.0, 1.0));
    v.push(p("exit_restart", "Same-direction signal on the exit bar keeps the trade, count restarts (1 = on, TradeStation booking)", 1.0, 0.0, 1.0, 1.0));
    Some(v)
}

// ---------------------------------------------------------------- VIX OHLC

/// The VIX the server was started with (argv[3], default: the project's india_vix dir), loaded once.
/// ponytail: re-reads the VIX dir instead of sharing main's copy; expose VIX OHLC on `VixAligned` to drop this.
fn server_vix() -> &'static [Row] {
    static V: OnceLock<Vec<Row>> = OnceLock::new();
    V.get_or_init(|| {
        let dir = std::env::args().nth(3).unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../../../india_vix").into());
        crate::load_vix_dir(&dir)
    })
}

/// VIX bars (instrument calendar, or own with vix_cal = 1) + map from the instrument's bar to the VIX bar of the same date.
struct Vx {
    o: Vec<f64>,
    h: Vec<f64>,
    l: Vec<f64>,
    c: Vec<f64>,
    at: Vec<Option<usize>>,
}

impl Vx {
    /// own_cal = false (default): only the VIX rows on the instrument's own trading days, plus the VIX history before its
    /// first bar as warm-up (as ats/ptjmt/spmt), so VIX bars on days the instrument does not trade (US holidays in the CBOE
    /// file) never enter N-day highs/lows, MAs, RSI or "yesterday". own_cal = true: the previous reading, the VIX's own calendar.
    fn new(r: &[Row], vix: &[Row], own_cal: bool) -> Self {
        let dates: std::collections::HashSet<&str> = r.iter().map(|x| x.date.as_str()).collect();
        let d0 = r.first().map_or(i64::MAX, |x| crate::day_number(&x.date));
        let kept: Vec<Row> = if own_cal { vix.to_vec() } else { vix.iter().filter(|v| crate::day_number(&v.date) < d0 || dates.contains(v.date.as_str())).cloned().collect() };
        let vix = &kept[..];
        let pos: HashMap<&str, usize> = vix.iter().enumerate().map(|(k, v)| (v.date.as_str(), k)).collect();
        Vx {
            o: vix.iter().map(|v| v.open).collect(),
            h: vix.iter().map(|v| v.high).collect(),
            l: vix.iter().map(|v| v.low).collect(),
            c: vix.iter().map(|v| v.close).collect(),
            at: r.iter().map(|x| pos.get(x.date.as_str()).copied()).collect(),
        }
    }
    /// VIX high is the highest high of the last n VIX bars (incl. this one).
    fn hh(&self, k: usize, n: usize) -> bool {
        k + 1 >= n && self.h[k + 1 - n..=k].iter().all(|&x| self.h[k] >= x)
    }
    fn ll(&self, k: usize, n: usize) -> bool {
        k + 1 >= n && self.l[k + 1 - n..=k].iter().all(|&x| self.l[k] <= x)
    }
}

/// Simple MA summed over the window each bar and rounded to 1e-9, so a VIX price equal to the MA in decimals
/// compares as equal (no rolling-sum drift). Used by CVR 3, whose exit tie handling is a param (exit_touch).
fn sma_exact(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|k| if k + 1 < n { f64::NAN } else { (v[k + 1 - n..=k].iter().sum::<f64>() / n as f64 * 1e9).round() / 1e9 }).collect()
}

/// TradeStation XAverage: alpha 2/(n+1), seeded with the first value.
fn xaverage(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut prev = f64::NAN;
    v.iter().map(|&x| { prev = if prev.is_nan() { x } else { prev + a * (x - prev) }; prev }).collect()
}

// ---------------------------------------------------------------- simulator

/// Stop-and-reverse, one position, entries and exits on the close.
/// `sig(i)` = +1 buy / -1 sell / 0 on instrument bar i. `hold` = exit N bars after entry (0 = no time exit);
/// a same-direction signal on the exit bar keeps the trade and restarts the count (`restart`; off = exit + re-enter). `xf(i, dir)` = extra exit rule.
fn sar(c: &[f64], from: usize, to: usize, sig: &dyn Fn(usize) -> i8, hold: usize, restart: bool, xf: Option<(&dyn Fn(usize, f64) -> bool, &str)>) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut cur: Option<(Trade, usize)> = None; // trade + bar the hold count runs from
    let open = |i: usize, s: i8| Trade {
        dir: s as f64,
        legs: vec![Leg { i, px: c[i], w: 1.0 }],
        exit_i: i,
        exit_px: c[i],
        reason: String::new(),
        entry_kind: "close",
        signal_i: i,
        open: false, exits: Vec::new()
    };
    let close = |t: &mut Trade, i: usize, why: String| {
        t.exit_i = i;
        t.exit_px = c[i];
        t.signal_i = i;
        t.reason = why;
    };
    for i in from..=to {
        let s = sig(i);
        if let Some((mut t, ci)) = cur.take() {
            if s != 0 && s as f64 == -t.dir {
                close(&mut t, i, "Opposite CVR signal: stop and reverse".into());
                out.push(t);
                if i < to {
                    cur = Some((open(i, s), i));
                }
                continue;
            }
            let rule = xf.and_then(|(f, why)| f(i, t.dir).then(|| why.to_string()));
            let timed = (hold > 0 && i - ci >= hold).then(|| format!("{hold} days held"));
            match rule.or(timed) {
                Some(_) if restart && s == t.dir as i8 && i < to => cur = Some((t, i)), // same-direction re-signal on the exit bar: stay in, count restarts
                Some(why) => {
                    close(&mut t, i, why);
                    out.push(t);
                    if s != 0 && i < to {
                        cur = Some((open(i, s), i)); // exit_restart off: book it as exit + new trade on the same close
                    }
                }
                None => cur = Some((t, ci)),
            }
            continue;
        }
        if s != 0 && i < to {
            cur = Some((open(i, s), i));
        }
    }
    if let Some((mut t, _)) = cur {
        close(&mut t, to, "open".into());
        t.open = true;
        out.push(t);
    }
    out
}

// ---------------------------------------------------------------- rules

/// None = not a strategy of this book.
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    params(id)?;
    let own_cal = p.get("vix_cal").copied().unwrap_or(0.0) >= 0.5;
    Some(run_with(id, x, &Vx::new(x.r, server_vix(), own_cal), from, to, p))
}

fn run_with(id: &str, x: &Ctx, v: &Vx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let schema = params(id).unwrap_or_default();
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let (vo, vh, vl, vc) = (&v.o, &v.h, &v.l, &v.c);
    let sign = |b: bool, s: bool| if b { 1i8 } else if s { -1 } else { 0 };
    // signal computed on the VIX's own bar k, mapped to the instrument's bar i (0 where the VIX has no bar)
    let on_vix = |f: &dyn Fn(usize) -> i8| -> Vec<i8> { v.at.iter().map(|k| k.map_or(0, |k| f(k))).collect() };
    let restart = p.get("exit_restart").copied().unwrap_or(1.0) >= 0.5; // NotTestable ids have no schema
    let simple = |sig: Vec<i8>, hold: usize| sar(&x.c, from, to, &|i| sig[i], hold, restart, None);
    match id {
        "vixr_cvr1" => {
            let len = u("len");
            simple(on_vix(&|k| sign(v.hh(k, len) && vc[k] < vo[k], v.ll(k, len) && vc[k] > vo[k])), u("hold"))
        }
        "vixr_cvr2" => {
            let (r, ob, os) = (rsi(vc, u("rsi_n")), g("ob"), g("os"));
            simple(on_vix(&|k| sign(k > 0 && r[k - 1] >= ob && r[k] < r[k - 1], k > 0 && r[k - 1] <= os && r[k] > r[k - 1])), u("hold"))
        }
        "vixr_cvr3" => {
            let (ma, pct) = (sma_exact(vc, u("ma")), g("pct") / 100.0);
            let (lag, cap) = (g("ma_lag").round().max(0.0) as usize, g("max_hold").round().max(0.0) as usize);
            let m = |k: usize| if k >= lag { ma[k - lag] } else { f64::NAN };
            let sig = on_vix(&|k| sign(vl[k] > m(k) && vc[k] >= m(k) * (1.0 + pct), vh[k] < m(k) && vc[k] <= m(k) * (1.0 - pct)));
            let touch = g("exit_touch") >= 0.5;
            let back = |i: usize, dir: f64| v.at[i].map_or(false, |k| k > 0 && if dir > 0.0 { vl[k] < ma[k - 1] || touch && vl[k] == ma[k - 1] } else { vh[k] > ma[k - 1] || touch && vh[k] == ma[k - 1] });
            sar(&x.c, from, to, &|i| sig[i], cap, restart, Some((&back, "VIX traded back through yesterday's MA")))
        }
        "vixr_cvr5_1d" | "vixr_cvr5_3d" => {
            let (ma, z) = (sma(vc, u("ma")), g("zone") / 100.0);
            simple(on_vix(&|k| {
                let r = vh[k] - vl[k];
                let (bot, top) = (vl[k] + r * z, vh[k] - r * z);
                sign(r > 0.0 && vo[k] <= bot && vc[k] <= bot && vc[k] > ma[k], r > 0.0 && vo[k] >= top && vc[k] >= top && vc[k] < ma[k])
            }), u("hold"))
        }
        "vixr_cvr6" => {
            // "new 20-period high" = above the highest high of the 20 days BEFORE the 3-day window (reference fixed
            // for the window, TradeStation Highest(High, 20)[3]); counted on `need` of the last `look` days
            let (hl, need, look, fixed) = (u("hilo"), u("need"), u("look"), g("ref") >= 0.5);
            simple(on_vix(&|k| {
                if !fixed {
                    let w = (k + 1).saturating_sub(look)..=k;
                    return sign(w.clone().filter(|&j| v.hh(j, hl)).count() >= need, w.filter(|&j| v.ll(j, hl)).count() >= need);
                }
                if k < look + hl - 1 {
                    return 0;
                }
                let (w, refw) = (k + 1 - look..=k, k + 1 - look - hl..k + 1 - look);
                let (top, bot) = (vh[refw.clone()].iter().fold(f64::MIN, |a, &b| a.max(b)), vl[refw].iter().fold(f64::MAX, |a, &b| a.min(b)));
                sign(w.clone().filter(|&j| vh[j] > top).count() >= need, w.filter(|&j| vl[j] < bot).count() >= need)
            }), u("hold"))
        }
        "vixr_cvr7" => {
            let m = sma(vc, u("ma"));
            simple(on_vix(&|k| {
                if k < 3 {
                    return 0;
                }
                let buy = vc[k - 2] > m[k - 3] && vc[k - 1] > m[k - 2] && vc[k] > m[k - 1] && vc[k - 2] < vo[k - 2] && vc[k - 1] > vo[k - 1]
                    && (vh[k - 1] > vh[k - 2] || vh[k - 1] > vh[k - 3]) && vc[k] < vo[k];
                let sell = vc[k - 2] < m[k - 3] && vc[k - 1] < m[k - 2] && vc[k] < m[k - 1] && vc[k - 2] > vo[k - 2] && vc[k - 1] < vo[k - 1]
                    && (vl[k - 1] < vl[k - 2] || vl[k - 1] < vl[k - 3]) && vc[k] > vo[k];
                sign(buy, sell)
            }), u("hold"))
        }
        "vixr_cvr8" => {
            let (f, s, look, thr) = (xaverage(vc, u("fast")), xaverage(vc, u("slow")), u("look"), g("thr"));
            let osc: Vec<f64> = (0..vc.len()).map(|k| if k == 0 { f64::NAN } else { (f[k] - f[k - 1]) - (s[k] - s[k - 1]) }).collect();
            let warm = u("slow").max(look); // let the slow EMA settle before trading
            simple(on_vix(&|k| {
                if k < warm {
                    return 0;
                }
                let w = &osc[k + 1 - look..=k];
                sign(osc[k] > thr && w.iter().all(|&o| osc[k] >= o), osc[k] < -thr && w.iter().all(|&o| osc[k] <= o))
            }), u("hold"))
        }
        "vixr_cvr9" => {
            let (rn, hl) = (u("rng_n"), u("hl_n"));
            let big = |k: usize| k + 1 >= rn && (k + 1 - rn..k).all(|j| vh[k] - vl[k] > vh[j] - vl[j]);
            simple(on_vix(&|k| sign(
                k > 0 && big(k) && v.hh(k, hl) && vc[k - 1] > vo[k - 1] && vc[k] < vo[k],
                k > 0 && big(k) && v.ll(k, hl) && vc[k - 1] < vo[k - 1] && vc[k] > vo[k],
            )), u("hold"))
        }
        "vixr_cvr10" => {
            let hl = u("hl_n");
            let sig: Vec<i8> = (0..x.c.len())
                .map(|i| match v.at[i] {
                    Some(k) if k > 0 && i > 0 => sign(
                        vc[k - 1] > vo[k - 1] && x.c[i - 1] < x.o[i - 1] && vc[k] < vo[k] && v.hh(k, hl),
                        vc[k - 1] < vo[k - 1] && x.c[i - 1] > x.o[i - 1] && vc[k] > vo[k] && v.ll(k, hl),
                    ),
                    _ => 0,
                })
                .collect();
            simple(sig, u("hold"))
        }
        "vixr_cvr11" => {
            let (f, s, thr, hl) = (sma(vc, u("fast")), sma(vc, u("slow")), g("thr"), u("hl_n"));
            simple(on_vix(&|k| sign(k > 0 && f[k - 1] - s[k - 1] >= thr && v.hh(k, hl), k > 0 && f[k - 1] - s[k - 1] <= -thr && v.ll(k, hl))), u("hold"))
        }
        _ => Vec::new(), // listed, not testable
    }
}

/// (family id, family name, variant label) for the strategy list.
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "vixr_cvr5_1d" => ("vixr_cvr5", "CVR 5 · VIX range extremes", "1-day exit"),
        "vixr_cvr5_3d" => ("vixr_cvr5", "CVR 5 · VIX range extremes", "3-day exit"),
        "vixr_cvr1" => ("vixr_cvr1", "CVR 1 · VIX 5-day extreme reverses", ""),
        "vixr_cvr2" => ("vixr_cvr2", "CVR 2 · VIX RSI(5) reversal", ""),
        "vixr_cvr3" => ("vixr_cvr3", "CVR 3 · VIX 10% from its 10-day MA", ""),
        "vixr_cvr6" => ("vixr_cvr6", "CVR 6 · Multiple-day sentiment extremes", ""),
        "vixr_cvr7" => ("vixr_cvr7", "CVR 7 · Flushing out the weak money", ""),
        "vixr_cvr8" => ("vixr_cvr8", "CVR 8 · VIX price oscillator", ""),
        "vixr_cvr9" => ("vixr_cvr9", "CVR 9 · VIX range", ""),
        "vixr_cvr10" => ("vixr_cvr10", "CVR 10 · Tying in S&P prices", ""),
        "vixr_cvr11" => ("vixr_cvr11", "CVR 11 · VIX 3/10 oscillator", ""),
        "vixr_all" => ("vixr_all", "All CVR signals (portfolio)", ""),
        "vixr_stocks" => ("vixr_stocks", "CVR signals with stocks", ""),
        "vixr_options" => ("vixr_options", "CVR signals with options", ""),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(d: usize, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: format!("2020-{:02}-{:02}", d / 28 + 1, d % 28 + 1), open: o, high: h, low: l, close: c, volume: None }
    }

    #[test]
    fn cvr1_buy_then_stop_and_reverse_then_3_day_exit() {
        // VIX flat; day 6 makes a 5-day high and closes below its open -> buy on day 6's close.
        // Day 8 makes a 5-day low and closes above its open -> sell signal while long: reverse on day 8's close.
        // Day 11 = 3 days after the short entry -> time exit.
        let mut vix: Vec<Row> = (0..13).map(|d| row(d, 20.0, 20.5, 19.5, 20.0)).collect();
        vix[6] = row(6, 24.0, 25.0, 21.0, 22.0);
        vix[8] = row(8, 18.0, 19.0, 17.0, 18.5);
        let mkt: Vec<Row> = (0..13).map(|d| { let p = 100.0 + d as f64; row(d, p, p + 1.0, p - 1.0, p) }).collect();
        let ctx = Ctx::new(&mkt, &mkt, &[]);
        let t = run_with("vixr_cvr1", &ctx, &Vx::new(&mkt, &vix, false), 0, 12, &Params::new());
        assert_eq!(t.len(), 2, "{t:?}");
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (1.0, 6, 8));
        assert!(t[0].reason.contains("reverse"));
        assert_eq!((t[1].dir, t[1].legs[0].i, t[1].exit_i, t[1].open), (-1.0, 8, 11, false));
    }

    #[test]
    fn resignal_on_exit_bar_extends_the_trade() {
        let c: Vec<f64> = (0..10).map(|i| 100.0 + i as f64).collect();
        // buy on 1, buy again on 3 (= exit bar of a 2-day hold) -> stays long, exits on 5
        let t = sar(&c, 0, 9, &|i| if i == 1 || i == 3 { 1 } else { 0 }, 2, true, None);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (1, 5));
    }

    #[test]
    fn restart_off_books_exit_and_new_trade() {
        let c: Vec<f64> = (0..10).map(|i| 100.0 + i as f64).collect();
        let t = sar(&c, 0, 9, &|i| if i == 1 || i == 3 { 1 } else { 0 }, 2, false, None);
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(1, 3), (3, 5)]);
    }

    #[test]
    fn cvr3_exit_is_strictly_below_yesterdays_ma_touch_is_a_param() {
        // VIX 20 for 10 days; day 10 low 21 / close 23 vs today's MA 20.3 -> buy (ma_lag 0).
        // Day 11 low = yesterday's MA 20.3 exactly (a touch): no exit by default ("below"); exit_touch = 1 exits there.
        // Day 12 low 20 < yesterday's MA 20.45 -> exit.
        let mut vix: Vec<Row> = (0..14).map(|d| row(d, 20.0, 20.0, 20.0, 20.0)).collect();
        vix[10] = row(10, 22.0, 23.5, 21.0, 23.0);
        vix[11] = row(11, 22.0, 22.0, 20.3, 21.5);
        vix[12] = row(12, 21.0, 21.5, 20.0, 20.5);
        let mkt: Vec<Row> = (0..14).map(|d| { let p = 100.0 + d as f64; row(d, p, p + 1.0, p - 1.0, p) }).collect();
        let ctx = Ctx::new(&mkt, &mkt, &[]);
        let run = |touch: f64| {
            let mut p = Params::new();
            p.insert("exit_touch".into(), touch);
            run_with("vixr_cvr3", &ctx, &Vx::new(&mkt, &vix, false), 0, 13, &p)
        };
        let t = run(0.0);
        assert_eq!((t.len(), t[0].dir, t[0].legs[0].i, t[0].exit_i), (1, 1.0, 10, 12), "{t:?}");
        let t = run(1.0);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (10, 11), "{t:?}");
    }

    #[test]
    fn cvr6_default_exit_is_the_published_3_day_test() {
        let def = |k: &str| params("vixr_cvr6").unwrap().into_iter().find(|d| d.key == k).unwrap().def;
        assert_eq!(def("hold"), 3.0);
        assert!(catalogue().iter().find(|d| d.id == "vixr_cvr6").unwrap().rules.iter().any(|r| r.contains("close 3 days after entry")));
    }

    #[test]
    fn sma_exact_ties_compare_equal() {
        assert_eq!(sma_exact(&[12.91; 10], 10)[9], 12.91);
        assert_eq!(sma_exact(&[0.1, 0.2, 0.3], 3)[2], 0.2);
    }
}
