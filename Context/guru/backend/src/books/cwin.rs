//! Larry Connors, "The TradingWindows Strategy" (2002) and the course "How To Successfully Trade The Connors
//! Windows Strategy". Book id "cwin".
//!
//! A Window: in an uptrend (10-period ADX >= 30 and +DI > -DI, or 14-period +DI >= 30) the whole bar sits below
//! its 4-day SMA (High < SMA4); in a downtrend the whole bar sits above it (Low > SMA4). Trade in the trend's
//! direction when it resumes.
//!
//! The shared simulator has no stop entries, same-bar stops or scale-outs, so this module has its own
//! (`sim`). A position is ONE `Trade` with one entry leg; partial exits (1/3 pieces) are folded into
//! `exit_px` = the size-weighted average exit price, which gives exactly the blended return of the pieces.
//! `exit_i` is the bar of the last piece; the equity curve marks the full position until then.
//! Conventions: stop entries fill at max(open, trigger) (long); a stop gapped through fills at the open;
//! when a stop and the entry fall on the same bar the stop is assumed hit after the fill (conservative).
#![allow(unused_imports, dead_code, unused_variables)]
use crate::connors::*;
use crate::Row;

pub const BOOK: &str = "cwin";

// ---------------------------------------------------------------- rules text (quoted from the book)

const TREND_L: &str = "F|Uptrend (p12): \"we must have a 10-period ADX reading of 30 or higher and a +DI reading above the -DI reading. Or we must have a 14-period +DI of 30 or higher (with no ADX reading required).\"";
const TREND_S: &str = "F|Downtrend (p12): \"the 10-period ADX must be 30 or higher and the -DI must be greater than the +DI. Or, we must have a 14-period -DI reading of above 30 (with no ADX reading required).\" (p22: \"greater than or equal to 30\")";
const WINDOW: &str = "F|Window (p12): \"on an uptrend, the high of today's price is below its moving average. For a downtrending stock, the low of the day must be above its moving average.\" \"A 4-day simple moving average.\"";
const X_STOP_NEXT: &str = "X|Stop (p27): \"The initial protective stops for the SingleWindows and DoubleWindows are at the bottom/top of the previous day's bar.\"";
const X_GAP10: &str = "X|Situation 10 (p47): \"When the market gaps above the 4-day moving average, it's still a buy, but you will want to raise your initial stop higher in this situation, likely to the Window day close.\"";
const X_STOP_CLOSE: &str = "X|Stop (p29): \"because we are entering on the close, our stop is at the bottom of today's bar for buys and at the top of today's bar for sells.\"";
const X_MEAN: &str = "X|Reversion to the mean (p31): \"we exit 1/4 to 1/3 our position when the market closes above its 4-day simple moving average for buys, or below its 4-day simple moving average for sells. At the same time we exit, we will move our stop on the remaining 1/2 to 3/4 of our position to breakeven.\"";
const X_DAY2: &str = "X|Course (TradeMGMTintro): \"On the second day ... exit at least another 1/4 on the close ... the goal for day three is to lock in all your gains\"";
const X_TIME: &str = "X|Time (p56): \"3 days is the optimal time period ... I want to be locking in profits throughout days 1-3 and sometimes, I'll let it go to day 4.\"";
const X_PROFIT: &str = "X|First piece at the first close beyond the 4-day SMA, as the p31 rule sentence says (\"we exit 1/4 to 1/3 our position when the market closes above its 4-day simple moving average for buys\"), even if that close is below the entry. [Not from the book] first_needs_profit 1 = only when that close is also in profit (from the p31 worked example, \"guaranteeing us a profitable trade\")";
const X_TIE: &str = "X|[Not from the book] Price-vs-SMA comparisons use a 1e-9 relative tolerance: an exact decimal tie (e.g. Low == SMA4) is not beyond the SMA, which floating-point sums would otherwise decide at random";
const X_TRAIL: &str = "X|Trailing stops after the first piece are discretionary (p31: \"Discretion, market strength/weakness, etc., will help you decide\"): not coded";

const SINGLE: &[&str] = &[TREND_L, TREND_S, WINDOW,
    "E|(p15) \"SingleWindows ... are simply a single bar which has its low trading above the 4-period moving average or its high of the day below the 4-period moving average. You will enter tomorrow if the market trades above today's high (for buys) or below today's low (for sells).\"",
    "E|Next day only: buy stop at the Window-day high (sell-short stop at its low); fill at the stop, or at the open if it gaps through",
    X_STOP_NEXT, X_GAP10, X_MEAN, X_PROFIT, X_DAY2, X_TIME, X_TRAIL, X_TIE];
const DOUBLE: &[&str] = &[TREND_L, TREND_S, WINDOW,
    "F|(p17) \"A 'DoubleWindow' is two 'SingleWindows' in a row\": today and yesterday are both Windows",
    "F|Reading: the trend filter is checked on the second (setup) bar only (default); double_trend_both 1 = trend on both bars (strict \"two SingleWindows\")",
    "E|(p17) \"The entry is the same as a SingleWindow. You will enter uptrending stocks on the buy side if tomorrow trades above today's high and you will enter downtrending stocks on the short side tomorrow if it trades under today's low.\"",
    X_STOP_NEXT, X_GAP10, X_MEAN, X_PROFIT, X_DAY2, X_TIME, X_TRAIL, X_TIE];
const AGGRESSIVE: &[&str] = &[TREND_L, TREND_S, WINDOW,
    "F|(p18) \"For sell setups, they are single or double Windows that close in the bottom 5% of their daily range and close below their open. For buys, they close in the top 5% of their daily range and above the open.\"",
    "E|(p18) \"you are buying on the close today in the direction of the trend\"",
    "E|Alternative (Situation 2, p39): \"You can also enter on the short side the next morning if it takes out the Window day low. This is a more conservative strategy.\" (entry_mode = 1)",
    X_STOP_CLOSE, X_MEAN, X_PROFIT, X_DAY2, X_TIME, X_TRAIL, X_TIE];
const HANGING: &[&str] = &[TREND_L, TREND_S, WINDOW,
    "F|(p20) \"HangingWindows occur when a market or stock opens and closes in the bottom 10% of its daily range for sells and in the top 10% of its daily range for buys.\"",
    "E|(p21) \"They should be entered on the close but if you miss them, then you can enter on the opening the next day.\" (entry_mode = 1: next open)",
    X_STOP_CLOSE, X_MEAN, X_PROFIT, X_DAY2, X_TIME, X_TRAIL, X_TIE];
const GAP_INTRA: &[&str] = &[
    "F|[long] (p22) \"you must have a 10-period ADX ≥ 30 and its +DI > -DI, or a 14-period +DI by itself that is greater than or equal to 30. Also, yesterday must be a SingleWindow or DoubleWindow day.\"",
    "E|[long] (p22) \"If today gaps open below yesterday's low, we buy at yesterday's low.\"",
    "F|[short] (p22) \"ADX ≥ 30, -DI > +DI, or a 14-period -DI by itself that is greater than or equal to 30. Also, yesterday must be either a SingleWindow or DoubleWindow day.\"",
    "E|[short] (p22) \"If today gaps open above yesterday's high, we sell at yesterday's high.\"",
    "X|Stop (p30): \"our stop for long positions is either today's open or today's low (your choice for buys). For short positions, our stop is either today's open or today's high (your choice).\" (stop_mode 1 = today's low/high from the next day; 0 = today's open)",
    X_MEAN, X_PROFIT, X_DAY2, X_TIME, X_TRAIL, X_TIE];
const GAP_CLOSE: &[&str] = &[TREND_L, TREND_S, WINDOW,
    "E|(p24) \"Close Of The Day GapWindows simply have a strong trend that gaps in the opposite direction, closes above its open for buys (below its open for sells), and makes a Window. We buy on the close when this occurs.\"",
    X_STOP_CLOSE, X_MEAN, X_PROFIT, X_DAY2, X_TIME, X_TRAIL, X_TIE];
const TWO_RULE: &[&str] = &[TREND_L, TREND_S, WINDOW,
    "E|Entry not stated in the book; default = the SingleWindow entry (next-day stop beyond the Window-day high/low), 1 unit (entry_mode switches the reading)",
    "X|(p32) \"These two tests have only two rules. You exit if the second day is not profitable or if it closes above/below the 4-day moving average two days in a row.\"",
    "X|(p32) \"Please note: These are mechanical, single rule exits and I do not trade Windows in this manner.\" No protective stop (use_window_stop = 1 adds one)",
    X_TIE,
];
const FIRST: &[&str] = &[TREND_L, TREND_S, WINDOW,
    "F|(course, FirstWindowsintro) \"What is a First Window? It's the first time a Window is formed after the ADX goes above 30. Using this criteria alone as a filter will increase your returns even more.\"",
    "E|Entry as SingleWindow: next-day stop beyond the Window-day high (buys) / low (sells)",
    X_STOP_NEXT, X_GAP10, X_MEAN, X_PROFIT, X_DAY2, X_TIME, X_TRAIL, X_TIE];
const WEEKLY: &[&str] = &[TREND_L, TREND_S, WINDOW,
    "F|Same rules on weekly bars (ADX/DI, 4-week SMA)",
    "E|(p53) \"Windows work in nearly all time frames. Here is a weekly chart of the S&P 500 ... You can trade these setups as 1-3 week moves.\" (p58) \"it works equally well on weekly charts.\"",
    "E|SingleWindow entry on weekly bars: next-week stop beyond the Window-week high/low",
    "X|Stop at the Window-week low/high; book management on weekly closes (1/3 at the first close beyond the 4-week SMA, stop to breakeven, 1/3 on week 2 if profitable, rest at the close of week 3)",
    X_PROFIT, X_TIE,
];
const VIX_GAP: &[&str] = &[
    "F|[short] (course) \"Yesterday, the VIX high of the day must be under the 4-period moving average, and its high of the day must be below the previous day's low. (It's a gap that occurs under the 4-day MA.)\"",
    "E|[short] (course) \"Today only, as soon as the VIX takes out yesterday's high, short the SPYs or S&P E-minis.\"",
    "F|[long] \"Buys are reversed\": yesterday the VIX low was above its 4-day MA and above the previous day's high",
    "E|[long] Today only, as soon as the VIX trades under yesterday's low, buy",
    "X|(course) \"Your initial stop will be at today's high of the market you enter.\" (low for buys)",
    "X|(course) \"Trailing stops will get you out.\" Amount not given: default day trade (out at the close); hold_days > 0 trails the stop at the prior bar's high/low",
    "F|(course) \"You can apply the same strategy with the QQQs and Nasdaq E-minis using the VXN.\"",
    X_TIE,
];

macro_rules! note {
    ($s:expr) => {
        concat!($s, " Exit (book management made mechanical; the book calls it discretionary): initial stop as quoted; 1/3 out at the first close beyond the 4-day SMA (first_needs_profit 1 = [Not from the book] only if that close is also in profit; first_rule 1 = the course's \"profit equals initial risk\"), then the stop moves to breakeven; another 1/3 at the close of day 2 if it is profitable; the rest at the close of day 3 (max_hold_days). Day 1 = the entry day for intraday entries, the day after the entry for entries on the close. Trailing stops beyond breakeven are discretionary and not coded. Pieces are folded into one trade at the size-weighted exit price.",
            " Trend: ADX(10)/DI(10) (charts label +DMI(10)) or DI(14) alone, Wilder/TradeStation formulas; -DI = +DI of the price-mirrored bars. The short DI-only threshold is \">= 30\" (p22) on both sides; p12 says \"above 30\". SMA4 includes the Window bar's own close. The book trades stocks/HOLDRs/futures; the vault runs it on indices/ETFs (NIFTY 50, NIFTY BANK, ...; SPY/QQQ in the US), 1 unit, prices in index points.",
            " Differences vs the 5-Week Seminar's Momentum Windows (book ws5): this book also accepts DI(14) >= 30 with no ADX; it has no price/volume/200-SMA/range/ATR/stochastic/VIX filters (only a 100k-share volume suggestion); it has AggressiveWindows and no Outside Windows; no opening-gap negation (Situation 10: a gap beyond the MA is still a buy, raise the stop to the Window-day close); the EOD GapWindow needs a gap against the trend AND a close beyond the open, entered on the close; the intraday GapWindow stop is today's open or today's low/high (ws5: the higher of 0.50 or the day's high); HangingWindows enter on the close (ws5: the generic next-day stop); exits scale out 1/4-1/3 at the close beyond the SMA4 with a breakeven stop and are out by day 3 (ws5: 1/3 at R and each further point, 2-pt then 1-pt trail).")
    };
}

/// Daily VIX + intraday-derived entry series for the VIX Gap Window (defined in the notes / report).
const VIX_SERIES: &[&str] = &["VIX_HIGH", "VIX_LOW", "VIX_CLOSE", "VIXUP_PX", "VIXUP_HOD", "VIXDN_PX", "VIXDN_LOD"];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim,
    };
    const P2: &str = "Part Two · Rules and Variations";
    vec![
        d("cwin_single_window", "SingleWindows", P2, SINGLE,
          note!("Next-day entry order only; a new Window the next day replaces it (that bar is then a DoubleWindow). Any Window bar counts, so this includes the second bar of a Double. Situation 10's raised stop (open beyond the Window-day SMA4 -> stop at the Window-day close) is on by default (gap_stop). Ranking when several setups appear (p57): Gap, Hanging, Double, Single, Aggressive."), None),
        d("cwin_double_window", "DoubleWindows", P2, DOUBLE,
          note!("Two Window bars in a row; the trend filter is checked on the second (setup) bar (double_trend_both 1 = on both bars, the strict \"two SingleWindows\" reading). Stop at the second Window bar's low/high. Situation 10's raised stop on by default."), None),
        d("cwin_aggressive_window", "AggressiveWindows", P2, AGGRESSIVE,
          note!("Market-on-close fill at the close price. Zero-range bars excluded. \"you will be wrong more than normal\" (p18); lowest rank (p57)."), None),
        d("cwin_hanging_window", "HangingWindows", P2, HANGING,
          note!("The definition sentence does not repeat \"Window\", but the text and the KLAC example require the whole bar beyond the SMA4: required here. Open and close both in the top (bottom) 10% of the range. Entry on the close (entry_mode 1 = next open, stop still the HangingWindow bar's low/high)."), None),
        d("cwin_gap_window_intraday", "GapWindows (intraday entry)", P2, GAP_INTRA,
          note!("Trend measured on yesterday (the Window day). Daily-bar fill: long when Open < yesterday's low and High >= it, at yesterday's low (exact on daily bars). Stop default = entry-day low/high, known only at the close, so active from the next day. stop_mode 0 (today's open): a same-day stop-out is counted when the close is back beyond the open (price must have crossed the open after the fill), exit at the open. Course: Street Smarts' \"ADX Gappers\" with a Window first."), None),
        d("cwin_gap_window_close", "Close Of The Day GapWindows", P2, GAP_CLOSE,
          note!("Gap = Open below yesterday's low for buys / above yesterday's high for sells (gap_ref 1 = beyond yesterday's close). Yesterday need not be a Window; today must be (High < SMA4 for buys). Entry on the close; stop at today's low/high."), None),
        d("cwin_window_two_rule_exit_test", "Windows · mechanical two-rule exit test", "Part Three · Exit Strategies", TWO_RULE,
          "The book's only published statistics: US T-Bond futures 01/05/1990-09/09/2002: 47 trades, 81% (long 32/81%, short 15/80%), avg $347.07, avg bars W1 L2. AMEX Broker/Dealer index XBD 04/15/1994-09/11/2002: 103 trades, 64% (long 67/70%, short 36/53%), avg 3.82 pts, avg bars W1 L1. TradeStation2000i, 1 contract, no costs. We have neither series; SPY/QQQ/IWM/MDY over the same window are a sanity comparison, not a reproduction (see implementation-report.md). Readings: entry_mode 0 = next-day stop at the Window-day high/low (default), 1 = Window-day close, 2 = next open, 3 = all five setups; loss_check_day = the trade day (entry day = 1) whose close must be profitable; the SMA4 count includes the entry day's close. No protective stop (\"only two rules\"). Trend and Window as in the other variants; run on indices/ETFs.",
          claim("103 (XBD)", "64%", "3.82 pts", "1 bar", "AMEX Broker/Dealer index 04/15/1994-09/11/2002")),
        d("cwin_first_window", "First Windows", "Course · First Windows", FIRST,
          note!("Course only. \"First\" = the first Window bar (long or short, trend filter as usual) after ADX(10) crosses from below 30 to 30 or more; the count resets whenever ADX falls back under 30. Entry, stop and exits as SingleWindows."), None),
        d("cwin_weekly_window", "Windows on weekly bars", "Part Four · Other Time Frames", WEEKLY,
          note!("Daily bars are resampled to Monday-Friday weeks using only data up to the test end (no lookahead); signals, stops and the 1-3 week management run on weekly bars (days read as weeks, max_hold_weeks), then each trade is placed on the day its entry or stop actually traded (first day in the week through the price) and on the week's last day for close exits. Fig 4.16 is captioned \"Daily\" but the text says weekly."), None),
        StrategyDef { needs_series: VIX_SERIES, ..d("cwin_vix_gap_window", "VIX Gap Window", "Course · VIX Gap Window", VIX_GAP,
          "Course only (\"new research\"); no ADX filter; MA = SMA of VIX closes. Intraday rule, coded against per-day series built from synchronized intraday bars: VIX_HIGH / VIX_LOW / VIX_CLOSE = daily VIX (market-wide file; a per-ticker file such as QQQ__VIX_HIGH.csv carries the VXN for QQQ; India: India VIX for NIFTY 50). VIXUP_PX = this ticker's price at the first intraday bar on which the VIX trades above the previous day's VIX high; VIXUP_HOD = this ticker's session high from the open through that bar. VIXDN_PX / VIXDN_LOD = the same when the VIX first trades below the previous day's VIX low (session low). Short at VIXUP_PX, stop = VIXUP_HOD: hit the same day if the day's high exceeds it. Default exit at the entry-day close (\"and daytrade\"); hold_days > 0 trails the stop at the prior bar's high/low. Run on SPY / NIFTY 50 (index or futures). No intraday VIX data exists in the vault, so every run reports a data gap.", None) },
    ]
}

fn trend_params() -> Vec<ParamDef> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    vec![
        p("adx_len", "ADX/DI length (ADX branch)", 10.0, 7.0, 14.0, 1.0),
        p("adx_min", "ADX threshold", 30.0, 20.0, 40.0, 5.0),
        p("di_only_len", "DI length (DI-only branch)", 14.0, 10.0, 20.0, 2.0),
        p("di_only_min", "DI-only threshold", 30.0, 25.0, 40.0, 5.0),
        p("ma_len", "Window SMA length", 4.0, 3.0, 6.0, 1.0),
        p("side", "Side: 0 both, 1 long only, -1 short only", 0.0, -1.0, 1.0, 1.0),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mgmt = |hold_key: &'static str, hold_label: &'static str| vec![
        p("first_piece_frac", "Fraction exited per piece (1/3)", 0.33, 0.25, 0.5, 0.08),
        p(hold_key, hold_label, 3.0, 1.0, 5.0, 1.0),
        p("first_rule", "First piece: 0 close beyond SMA, 1 profit >= initial risk (course)", 0.0, 0.0, 1.0, 1.0),
        p("first_needs_profit", "First piece needs the close in profit (0 = p31 rule as written, 1 = [Not from the book] only in profit)", 0.0, 0.0, 1.0, 1.0),
    ];
    let stop_entry = || vec![
        p("entry_tick", "Points beyond the Window high/low for the stop entry", 0.0, 0.0, 2.0, 0.25),
        p("gap_stop", "Situation 10: open beyond SMA4 raises stop to Window close (1 on)", 1.0, 0.0, 1.0, 1.0),
    ];
    let days = || mgmt("max_hold_days", "Final time exit (day)");
    let mut v = trend_params();
    match id {
        "cwin_single_window" | "cwin_double_window" | "cwin_first_window" => {
            v.extend(days());
            v.extend(stop_entry());
            if id == "cwin_double_window" {
                v.push(p("double_trend_both", "Trend on: 0 second bar only, 1 both bars", 0.0, 0.0, 1.0, 1.0));
            }
        }
        "cwin_aggressive_window" => {
            v.extend(days());
            v.push(p("close_pct", "Close within top/bottom fraction of range", 0.05, 0.02, 0.15, 0.01));
            v.push(p("entry_mode", "Entry: 0 on the close, 1 next-day stop (Situation 2)", 0.0, 0.0, 1.0, 1.0));
        }
        "cwin_hanging_window" => {
            v.extend(days());
            v.push(p("oc_pct", "Open and close within top/bottom fraction of range", 0.1, 0.05, 0.2, 0.05));
            v.push(p("entry_mode", "Entry: 0 on the close, 1 next open", 0.0, 0.0, 1.0, 1.0));
        }
        "cwin_gap_window_intraday" => {
            v.extend(days());
            v.push(p("stop_mode", "Stop: 0 entry-day open, 1 entry-day low/high", 1.0, 0.0, 1.0, 1.0));
        }
        "cwin_gap_window_close" => {
            v.extend(days());
            v.push(p("gap_ref", "Gap beyond: 0 prior low/high, 1 prior close", 0.0, 0.0, 1.0, 1.0));
        }
        "cwin_weekly_window" => {
            v.extend(mgmt("max_hold_weeks", "Final time exit (week)"));
            v.extend(stop_entry());
        }
        "cwin_window_two_rule_exit_test" => v.extend([
            p("entry_mode", "Entry: 0 next-day stop, 1 Window close, 2 next open, 3 all five setups", 0.0, 0.0, 3.0, 1.0),
            p("loss_check_day", "Exit if not profitable at the close of trade day N (entry day = 1)", 2.0, 1.0, 3.0, 1.0),
            p("ma_closes_in_row", "Closes beyond SMA4 in a row to exit", 2.0, 1.0, 3.0, 1.0),
            p("ma_exit_next_open", "SMA exit: 0 at that close, 1 next open", 0.0, 0.0, 1.0, 1.0),
            p("use_window_stop", "Add the Window-bar protective stop (book: none)", 0.0, 0.0, 1.0, 1.0),
            p("entry_tick", "Points beyond the Window high/low for the stop entry", 0.0, 0.0, 2.0, 0.25),
        ]),
        "cwin_vix_gap_window" => {
            return Some(vec![
                p("vix_ma_len", "VIX SMA length", 4.0, 3.0, 6.0, 1.0),
                p("hold_days", "Hold days (0 = day trade, out at the close)", 0.0, 0.0, 5.0, 1.0),
                p("side", "Side: 0 both, 1 long only, -1 short only", 0.0, -1.0, 1.0, 1.0),
            ]);
        }
        _ => return None,
    }
    Some(v)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let w = |v| Some(("cwin_windows", "Connors TradingWindows", v));
    match id {
        "cwin_single_window" => w("SingleWindows"),
        "cwin_double_window" => w("DoubleWindows"),
        "cwin_aggressive_window" => w("AggressiveWindows (on the close)"),
        "cwin_hanging_window" => w("HangingWindows"),
        "cwin_gap_window_intraday" => w("GapWindows (intraday)"),
        "cwin_gap_window_close" => w("Close Of The Day GapWindows"),
        "cwin_window_two_rule_exit_test" => w("Two-rule exit test (published)"),
        "cwin_first_window" => w("First Windows (course)"),
        "cwin_weekly_window" => w("Weekly Windows"),
        "cwin_vix_gap_window" => Some(("cwin_vix_gap_window", "VIX Gap Window", "VIX Gap Window (course)")),
        _ => None,
    }
}

// ---------------------------------------------------------------- indicators

/// Price-mirrored bars: +DI of these is -DI of the originals (up move <-> down move, same true range).
fn mirror(r: &[Row]) -> Vec<Row> {
    r.iter().map(|x| Row { ticker: String::new(), date: String::new(), open: -x.open, high: -x.low, low: -x.high, close: -x.close, volume: None }).collect()
}

/// (uptrend, downtrend, ADX) per bar.
fn trend(r: &[Row], adx_len: usize, adx_min: f64, di_len: usize, di_min: f64) -> (Vec<bool>, Vec<bool>, Vec<f64>) {
    let m = mirror(r);
    let (pdi, adx) = dmi(r, adx_len);
    let (mdi, _) = dmi(&m, adx_len);
    let (p14, _) = dmi(r, di_len);
    let (m14, _) = dmi(&m, di_len);
    let up = (0..r.len()).map(|i| (adx[i] >= adx_min && pdi[i] > mdi[i]) || p14[i] >= di_min).collect();
    let dn = (0..r.len()).map(|i| (adx[i] >= adx_min && mdi[i] > pdi[i]) || m14[i] >= di_min).collect();
    (up, dn, adx)
}

/// SMA as a fresh sum of the last n values (TradeStation's Average). The shared running-sum `sma` drifts by
/// ~1e-12 over thousands of bars, which flips exact ties such as Low == SMA4 (common with cent prices).
/// NaN when any of the last n values is missing, and a missing value does not poison later bars.
fn sma_exact(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}

/// [Not from the book] Price-vs-SMA tie tolerance (relative). Sums of cent prices are inexact in binary, so an exact
/// decimal tie such as Low == SMA4 can land either side; within 1e-9·|SMA| counts as equal (not beyond).
const EPS: f64 = 1e-9;
fn lt_ma(x: f64, ma: f64) -> bool { x < ma - EPS * ma.abs() }
fn gt_ma(x: f64, ma: f64) -> bool { x > ma + EPS * ma.abs() }
/// x strictly beyond ma in direction d (above for d > 0, below for d < 0), tie-safe.
fn beyond_ma(d: f64, x: f64, ma: f64) -> bool { if d > 0.0 { gt_ma(x, ma) } else { lt_ma(x, ma) } }

// ---------------------------------------------------------------- simulator

struct Bars<'a> {
    o: &'a [f64],
    h: &'a [f64],
    l: &'a [f64],
    c: &'a [f64],
    ma: &'a [f64],
    wk: Option<Wk<'a>>, // weekly bars: fills are priced and booked on the daily bars inside each week
}

/// Daily detail under weekly bars: span[w] = first..=last daily index of week w.
struct Wk<'a> {
    span: &'a [(usize, usize)],
    o: &'a [f64],
    h: &'a [f64],
    l: &'a [f64],
}

impl Wk<'_> {
    /// First day k0..=end of week w whose range reaches t (d > 0: high >= t, else low <= t),
    /// filled at t, or at that day's open if it gapped through.
    fn touch(&self, w: usize, k0: usize, d: f64, t: f64) -> Option<(usize, f64)> {
        (k0..=self.span[w].1).find_map(|k| {
            if d > 0.0 { (self.h[k] >= t).then(|| (k, self.o[k].max(t))) } else { (self.l[k] <= t).then(|| (k, self.o[k].min(t))) }
        })
    }
}

impl Bars<'_> {
    /// Daily bar of something booked on bar i's open / close (bar i itself on daily bars).
    fn first(&self, i: usize) -> usize { self.wk.as_ref().map_or(i, |w| w.span[i].0) }
    fn last(&self, i: usize) -> usize { self.wk.as_ref().map_or(i, |w| w.span[i].1) }
}

#[derive(Clone, Copy, Debug)]
enum Ord {
    Close,         // market on close of the setup bar
    NextOpen,      // next bar's open
    NextStop(f64), // next bar only: stop through the price; fill at the stop or a gap-through open
    NextGap(f64),  // next bar only: opens beyond the price (against the setup) and trades back to it; fill at the price
}

#[derive(Clone, Copy, Debug)]
enum StopRule {
    None,
    Level(f64), // known at the signal
    EntryBar,   // entry-day low (long) / high (short), active from the next day
    EntryOpen,  // entry-day open
}

#[derive(Clone, Copy, Debug)]
struct Setup {
    dir: f64,
    ord: Ord,
    stop: StopRule,
    /// Situation 10: (Window-day SMA4, Window-day close). If the entry day opens beyond the SMA the stop moves to the close.
    gap_raise: Option<(f64, f64)>,
}

#[derive(Clone, Copy)]
enum Mgmt {
    Book { frac: f64, hold: usize, by_risk: bool, needs_profit: bool },
    TwoRule { check_day: usize, in_row: usize, next_open: bool },
}

struct Pos {
    dir: f64,
    e: usize,
    de: usize, // daily bar of the entry fill (= e on daily bars)
    px: f64,
    day1: usize,
    stop: f64,
    stop_from: usize,
    open_stop_today: bool, // EntryOpen: same-day stop-out test on the entry bar
    risk: f64,
    rem: f64,
    acc: f64, // sum of fraction * exit price of pieces already out
    took_first: bool,
    took_day2: bool,
    in_row: usize,
    exit_next_open: bool,
    parts: Vec<String>,
    pieces: Vec<(usize, f64, f64)>, // (bar, price, fraction) of pieces already out
    kind: &'static str,
}

/// Entry fill on bar i: (price, daily bar it traded on).
fn fill(b: &Bars, i: usize, s: &Setup) -> Option<(f64, usize)> {
    if let (Some(w), Ord::NextStop(t)) = (&b.wk, s.ord) {
        return w.touch(i, w.span[i].0, s.dir, t).map(|(k, px)| (px, k));
    }
    let (o, h, l) = (b.o[i], b.h[i], b.l[i]);
    let px = match s.ord {
        Ord::Close => None,
        Ord::NextOpen => Some(o),
        Ord::NextStop(t) => if s.dir > 0.0 { (h >= t).then(|| o.max(t)) } else { (l <= t).then(|| o.min(t)) },
        Ord::NextGap(t) => if s.dir > 0.0 { (o < t && h >= t).then_some(t) } else { (o > t && l <= t).then_some(t) },
    };
    px.map(|px| (px, if matches!(s.ord, Ord::NextOpen) { b.first(i) } else { i }))
}

fn open_pos(b: &Bars, i: usize, px: f64, de: usize, s: &Setup) -> Pos {
    let intraday = !matches!(s.ord, Ord::Close);
    let d = s.dir;
    let (mut stop, stop_from) = match s.stop {
        StopRule::None => (f64::NAN, i),
        StopRule::Level(x) => (x, if intraday { i } else { i + 1 }),
        StopRule::EntryBar => (if d > 0.0 { b.l[i] } else { b.h[i] }, i + 1),
        StopRule::EntryOpen => (b.o[i], i + 1),
    };
    if let Some((ma, cl)) = s.gap_raise {
        if beyond_ma(d, b.o[i], ma) {
            stop = cl;
        }
    }
    let kind = match s.ord {
        Ord::Close => "close",
        Ord::NextOpen => "open",
        Ord::NextStop(_) | Ord::NextGap(_) => "stop",
    };
    Pos {
        dir: d, e: i, de, px, day1: if intraday { i } else { i + 1 }, stop, stop_from,
        open_stop_today: matches!(s.stop, StopRule::EntryOpen), risk: (px - stop).abs(), rem: 1.0, acc: 0.0,
        took_first: false, took_day2: false, in_row: 0, exit_next_open: false, parts: vec![], pieces: vec![], kind,
    }
}

fn close_trade(p: Pos, i: usize, px: f64, why: String, signal_i: usize, open: bool) -> Trade {
    let mut parts = p.parts;
    parts.push(why);
    let mut exits = p.pieces;
    if !exits.is_empty() {
        exits.push((i, px, p.rem));
    }
    Trade {
        dir: p.dir,
        legs: vec![Leg { i: p.de, px: p.px, w: 1.0 }],
        exit_i: i,
        exit_px: p.acc + p.rem * px,
        reason: parts.join("; "),
        entry_kind: p.kind,
        signal_i,
        open,
        exits,
    }
}

fn fmt(x: f64) -> String {
    format!("{}", (x * 100.0).round() / 100.0)
}

/// Piece out at bar i's close, booked on daily bar `at`.
fn take(p: &mut Pos, i: usize, at: usize, frac: f64, px: f64, why: &str) {
    let f = frac.min(p.rem);
    p.acc += f * px;
    p.rem -= f;
    p.pieces.push((at, px, f));
    p.parts.push(format!("{}% at {} ({why})", (f * 100.0).round(), fmt(px)));
    // breakeven stop on the rest, from the next bar
    p.stop = p.px;
    p.stop_from = i + 1;
    p.open_stop_today = false;
}

/// Manage an open position on bar i. Some = closed on this bar: (trade, stop level if a stop closed it).
fn manage(b: &Bars, i: usize, pos: &mut Option<Pos>, m: Mgmt, to: usize) -> Option<(Trade, Option<f64>)> {
    let p = pos.as_mut()?;
    let d = p.dir;
    let at = b.last(i); // closes of bar i are booked on its last daily bar
    let mut out: Option<(f64, String, Option<f64>, usize, usize)> = None; // (px, why, stop, signal bar, booked bar)
    let on_entry = |px: f64| if d > 0.0 { p.stop.min(px) } else { p.stop.max(px) };
    let stop_hit = if !p.stop.is_finite() || i < p.stop_from {
        None
    } else if let Some(w) = &b.wk {
        // first day of the week (from the entry day in the entry week) through the stop
        w.touch(i, if i == p.e { p.de } else { w.span[i].0 }, -d, p.stop).map(|(k, px)| (k, if k == p.de { on_entry(p.px) } else { px }))
    } else if if d > 0.0 { b.l[i] <= p.stop } else { b.h[i] >= p.stop } {
        Some((i, if i == p.e { on_entry(p.px) } else if d > 0.0 { b.o[i].min(p.stop) } else { b.o[i].max(p.stop) }))
    } else {
        None
    };
    if p.exit_next_open {
        out = Some((b.o[i], "SMA exit, out next open".into(), None, b.last(i - 1), b.first(i)));
    } else if p.open_stop_today && i == p.e && d * (b.c[i] - b.o[i]) < 0.0 {
        out = Some((b.o[i], format!("stop at entry-day open {}", fmt(b.o[i])), Some(b.o[i]), at, at));
    } else if let Some((k, px)) = stop_hit {
        let what = if p.took_first || p.took_day2 { "breakeven stop" } else { "stop" };
        out = Some((px, format!("{what} {} hit, out at {}", fmt(p.stop), fmt(px)), Some(p.stop), k, k));
    } else if i >= p.day1 {
        let day = i - p.day1 + 1;
        let c = b.c[i];
        let profit = d * (c - p.px) > 0.0;
        let beyond = beyond_ma(d, c, b.ma[i]);
        match m {
            Mgmt::Book { frac, hold, by_risk, needs_profit } => {
                let first = if by_risk { p.risk > 0.0 && d * (c - p.px) >= p.risk } else { beyond && (profit || !needs_profit) };
                if !p.took_first && first {
                    p.took_first = true;
                    take(p, i, at, frac, c, if by_risk { "profit >= initial risk" } else { "close beyond SMA4" });
                }
                if day == 2 && !p.took_day2 && profit && p.rem > 1e-9 {
                    p.took_day2 = true;
                    take(p, i, at, frac, c, "day-2 close in profit");
                }
                if p.rem <= 1e-9 || day >= hold {
                    out = Some((c, format!("rest at the day-{day} close {}", fmt(c)), None, at, at));
                }
            }
            Mgmt::TwoRule { check_day, in_row, next_open } => {
                p.in_row = if beyond { p.in_row + 1 } else { 0 };
                if p.in_row >= in_row {
                    if next_open && i < to {
                        p.exit_next_open = true;
                    } else {
                        out = Some((c, format!("{in_row} closes beyond SMA4, out at {}", fmt(c)), None, at, at));
                    }
                } else if day == check_day && !profit {
                    out = Some((c, format!("day {day} not profitable, out at {}", fmt(c)), None, at, at));
                }
            }
        }
    }
    let (px, why, stop, sig, at) = out?;
    let p = pos.take().unwrap();
    Some((close_trade(p, at, px, why, sig, false), stop))
}

/// One position at a time over bars [from, to]. `sig(i)` = setups seen on bar i's close: Close orders fill now,
/// the others are next-bar orders (tried in the order given). Returns trades + the stop level that closed each (if any).
fn sim(b: &Bars, sig: &dyn Fn(usize) -> Vec<Setup>, m: Mgmt, from: usize, to: usize) -> Vec<(Trade, Option<f64>)> {
    let mut out = Vec::new();
    let mut pend: Vec<Setup> = Vec::new();
    let mut pos: Option<Pos> = None;
    for i in from..=to {
        if pos.is_none() {
            if let Some((s, (px, de))) = pend.iter().find_map(|s| fill(b, i, s).map(|f| (s, f))) {
                pos = Some(open_pos(b, i, px, de, s));
            }
        }
        pend.clear();
        let mut exited = false;
        if let Some(t) = manage(b, i, &mut pos, m, to) {
            out.push(t);
            exited = true;
        }
        if pos.is_some() || i == to {
            continue;
        }
        let ss = sig(i);
        if !exited {
            if let Some(s) = ss.iter().find(|s| matches!(s.ord, Ord::Close)) {
                pos = Some(open_pos(b, i, b.c[i], b.last(i), s));
                continue;
            }
        }
        pend = ss.into_iter().filter(|s| !matches!(s.ord, Ord::Close)).collect();
    }
    if let Some(p) = pos {
        out.push((close_trade(p, b.last(to), b.c[to], "open".into(), b.last(to), true), None));
    }
    out
}

// ---------------------------------------------------------------- weekly bars

fn days_from_civil(date: &str) -> i64 {
    let (y, m, d): (i64, i64, i64) = (date[0..4].parse().unwrap_or(1970), date[5..7].parse().unwrap_or(1), date[8..10].parse().unwrap_or(1));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468
}

/// Monday-based weeks from daily rows [0, to]: (weekly rows, first..=last daily index of each week).
fn weekly(r: &[Row], to: usize) -> (Vec<Row>, Vec<(usize, usize)>) {
    let (mut w, mut span): (Vec<Row>, Vec<(usize, usize)>) = (vec![], vec![]);
    let mut key = i64::MIN;
    for (i, x) in r[..=to].iter().enumerate() {
        let k = (days_from_civil(&x.date) + 3).div_euclid(7); // 1970-01-01 was a Thursday
        if k != key {
            key = k;
            w.push(Row { ticker: String::new(), date: x.date.clone(), open: x.open, high: x.high, low: x.low, close: x.close, volume: None });
            span.push((i, i));
        } else {
            let b = w.last_mut().unwrap();
            b.high = b.high.max(x.high);
            b.low = b.low.min(x.low);
            b.close = x.close;
            span.last_mut().unwrap().1 = i;
        }
    }
    (w, span)
}

// ---------------------------------------------------------------- strategies

/// Next-day stop entry beyond the Window bar with the stop at the Window bar's other end.
fn stop_order(b: &Bars, i: usize, d: f64, tick: f64, gap: bool) -> Setup {
    Setup {
        dir: d,
        ord: Ord::NextStop(if d > 0.0 { b.h[i] + tick } else { b.l[i] - tick }),
        stop: StopRule::Level(if d > 0.0 { b.l[i] } else { b.h[i] }),
        gap_raise: gap.then(|| (b.ma[i], b.c[i])),
    }
}

/// Single/Double/First/Weekly: a Window bar (with trend) on bar i, if `ok`.
fn single_setups(b: &Bars, up: &[bool], dn: &[bool], i: usize, tick: f64, gap: bool, ok: bool) -> Vec<Setup> {
    let mut v = Vec::new();
    if ok && up[i] && lt_ma(b.h[i], b.ma[i]) {
        v.push(stop_order(b, i, 1.0, tick, gap));
    }
    if ok && dn[i] && gt_ma(b.l[i], b.ma[i]) {
        v.push(stop_order(b, i, -1.0, tick, gap));
    }
    v
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let side = g("side").round();
    let keep = |v: Vec<Setup>| -> Vec<Setup> { v.into_iter().filter(|s| side == 0.0 || s.dir == side).collect() };

    if id == "cwin_vix_gap_window" {
        return Some(vix_gap(x, from, to, u("vix_ma_len"), g("hold_days").round().max(0.0) as usize, side));
    }

    let mgmt = |hold_key: &str| Mgmt::Book { frac: g("first_piece_frac"), hold: u(hold_key), by_risk: g("first_rule") >= 0.5, needs_profit: g("first_needs_profit") >= 0.5 };
    let trend_of = |r: &[Row]| trend(r, u("adx_len"), g("adx_min"), u("di_only_len"), g("di_only_min"));

    if id == "cwin_weekly_window" {
        let (w, span) = weekly(x.r, to);
        let (up, dn, _) = trend_of(&w);
        let col = |f: fn(&Row) -> f64| w.iter().map(f).collect::<Vec<f64>>();
        let (o, h, l, c) = (col(|r| r.open), col(|r| r.high), col(|r| r.low), col(|r| r.close));
        let ma = sma_exact(&c, u("ma_len"));
        let b = Bars { o: &o, h: &h, l: &l, c: &c, ma: &ma, wk: Some(Wk { span: &span, o: &x.o, h: &x.h, l: &x.l }) };
        let (tick, gap) = (g("entry_tick"), g("gap_stop") >= 0.5);
        let sig = |i: usize| keep(single_setups(&b, &up, &dn, i, tick, gap, true));
        let tw = w.len() - 1;
        let fw = match span.iter().position(|s| s.0 >= from) {
            Some(f) if f < tw => f,
            _ => return Some(vec![]),
        };
        return Some(sim(&b, &sig, mgmt("max_hold_weeks"), fw, tw).into_iter().map(|(t, _)| t).collect());
    }

    let (up, dn, adx) = trend_of(x.r);
    let ma = sma_exact(&x.c, u("ma_len"));
    let b = Bars { o: &x.o, h: &x.h, l: &x.l, c: &x.c, ma: &ma, wk: None };
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let wl = |i: usize| up[i] && lt_ma(h[i], ma[i]);
    let ws = |i: usize| dn[i] && gt_ma(l[i], ma[i]);
    let win = |i: usize, d: f64| if d > 0.0 { wl(i) } else { ws(i) };
    let rng = |i: usize| h[i] - l[i];
    // close/open within the top (long) or bottom (short) fraction k of the range
    let near = |i: usize, d: f64, v: f64, k: f64| if d > 0.0 { v >= l[i] + (1.0 - k) * rng(i) } else { v <= l[i] + k * rng(i) };
    let at_close = |i: usize, d: f64| Setup { dir: d, ord: Ord::Close, stop: StopRule::Level(if d > 0.0 { l[i] } else { h[i] }), gap_raise: None };
    let both = |i: usize, f: &dyn Fn(usize, f64) -> Option<Setup>| keep([f(i, 1.0), f(i, -1.0)].into_iter().flatten().collect());

    let trades = match id {
        "cwin_single_window" | "cwin_double_window" => {
            let (tick, gap, dbl) = (g("entry_tick"), g("gap_stop") >= 0.5, id == "cwin_double_window");
            // Double: yesterday's bar was also wholly beyond its SMA on the same side (and, if double_trend_both, in trend)
            let tb = dbl && g("double_trend_both") >= 0.5;
            let prev = |i: usize| i > 0 && ((lt_ma(h[i - 1], ma[i - 1]) && lt_ma(h[i], ma[i]) && (!tb || up[i - 1]))
                || (gt_ma(l[i - 1], ma[i - 1]) && gt_ma(l[i], ma[i]) && (!tb || dn[i - 1])));
            let sig = |i: usize| keep(single_setups(&b, &up, &dn, i, tick, gap, !dbl || prev(i)));
            sim(&b, &sig, mgmt("max_hold_days"), from, to)
        }
        "cwin_first_window" => {
            let (tick, gap, amin) = (g("entry_tick"), g("gap_stop") >= 0.5, g("adx_min"));
            let mut first = vec![false; c.len()];
            let mut armed = false;
            for i in 1..c.len() {
                if adx[i] >= amin && !(adx[i - 1] >= amin) {
                    armed = true; // ADX crossed up through the threshold
                }
                if !(adx[i] >= amin) {
                    armed = false;
                }
                if armed && (wl(i) || ws(i)) {
                    first[i] = true;
                    armed = false;
                }
            }
            let sig = |i: usize| keep(single_setups(&b, &up, &dn, i, tick, gap, first[i]));
            sim(&b, &sig, mgmt("max_hold_days"), from, to)
        }
        "cwin_aggressive_window" => {
            let (k, next) = (g("close_pct"), g("entry_mode") >= 0.5);
            let sig = |i: usize| both(i, &|i, d| {
                let ok = win(i, d) && rng(i) > 0.0 && d * (c[i] - o[i]) > 0.0 && near(i, d, c[i], k);
                ok.then(|| if next { stop_order(&b, i, d, 0.0, false) } else { at_close(i, d) })
            });
            sim(&b, &sig, mgmt("max_hold_days"), from, to)
        }
        "cwin_hanging_window" => {
            let (k, next) = (g("oc_pct"), g("entry_mode") >= 0.5);
            let sig = |i: usize| both(i, &|i, d| {
                (win(i, d) && rng(i) > 0.0 && near(i, d, o[i], k) && near(i, d, c[i], k))
                    .then(|| if next { Setup { ord: Ord::NextOpen, ..at_close(i, d) } } else { at_close(i, d) })
            });
            sim(&b, &sig, mgmt("max_hold_days"), from, to)
        }
        "cwin_gap_window_intraday" => {
            let st = if g("stop_mode") >= 0.5 { StopRule::EntryBar } else { StopRule::EntryOpen };
            let sig = |i: usize| both(i, &|i, d| win(i, d).then(|| Setup { dir: d, ord: Ord::NextGap(if d > 0.0 { l[i] } else { h[i] }), stop: st, gap_raise: None }));
            sim(&b, &sig, mgmt("max_hold_days"), from, to)
        }
        "cwin_gap_window_close" => {
            let by_close = g("gap_ref") >= 0.5;
            let sig = |i: usize| both(i, &|i, d| {
                if i == 0 {
                    return None;
                }
                let gapped = if d > 0.0 { o[i] < if by_close { c[i - 1] } else { l[i - 1] } } else { o[i] > if by_close { c[i - 1] } else { h[i - 1] } };
                (win(i, d) && gapped && d * (c[i] - o[i]) > 0.0).then(|| at_close(i, d))
            });
            sim(&b, &sig, mgmt("max_hold_days"), from, to)
        }
        "cwin_window_two_rule_exit_test" => {
            let (mode, stop, tick) = (g("entry_mode").round() as i32, g("use_window_stop") >= 0.5, g("entry_tick"));
            let m = Mgmt::TwoRule { check_day: u("loss_check_day"), in_row: u("ma_closes_in_row"), next_open: g("ma_exit_next_open") >= 0.5 };
            let st = |i: usize, d: f64| if stop { StopRule::Level(if d > 0.0 { l[i] } else { h[i] }) } else { StopRule::None };
            let one = |i: usize, d: f64| -> Vec<Setup> {
                if !win(i, d) {
                    return vec![];
                }
                let s = Setup { dir: d, ord: Ord::Close, stop: st(i, d), gap_raise: None };
                let stop_px = if d > 0.0 { h[i] + tick } else { l[i] - tick };
                let mut v = vec![];
                if mode == 3 {
                    // all five setups: close entries (Aggressive 5%, Hanging 10%, Close-of-day Gap) first,
                    // else next-day orders: the intraday Gap (at the open) is tried before the Single stop
                    let aggressive = rng(i) > 0.0 && d * (c[i] - o[i]) > 0.0 && near(i, d, c[i], 0.05);
                    let hanging = rng(i) > 0.0 && near(i, d, o[i], 0.1) && near(i, d, c[i], 0.1);
                    let gap_close = i > 0 && d * (c[i] - o[i]) > 0.0 && if d > 0.0 { o[i] < l[i - 1] } else { o[i] > h[i - 1] };
                    if aggressive || hanging || gap_close {
                        return vec![s];
                    }
                    v.push(Setup { ord: Ord::NextGap(if d > 0.0 { l[i] } else { h[i] }), stop: if stop { StopRule::EntryBar } else { StopRule::None }, ..s });
                }
                v.push(match mode {
                    1 => s,
                    2 => Setup { ord: Ord::NextOpen, ..s },
                    _ => Setup { ord: Ord::NextStop(stop_px), ..s },
                });
                v
            };
            let sig = |i: usize| keep([one(i, 1.0), one(i, -1.0)].concat());
            sim(&b, &sig, m, from, to)
        }
        _ => return None,
    };
    Some(trades.into_iter().map(|(t, _)| t).collect())
}

/// VIX Gap Window against the per-day series in VIX_SERIES (see the catalogue notes).
fn vix_gap(x: &Ctx, from: usize, to: usize, n: usize, hold: usize, side: f64) -> Vec<Trade> {
    let s = |k: &str| x.series(k).map(|v| v.to_vec()).unwrap_or_else(|| vec![f64::NAN; x.c.len()]);
    let (vh, vl, vc) = (s("VIX_HIGH"), s("VIX_LOW"), s("VIX_CLOSE"));
    let (up_px, up_hod, dn_px, dn_lod) = (s("VIXUP_PX"), s("VIXUP_HOD"), s("VIXDN_PX"), s("VIXDN_LOD"));
    let vma = sma_exact(&vc, n);
    let mut out = vec![];
    let mut i = from.max(2);
    while i <= to {
        // short: yesterday's VIX bar wholly under its SMA and gapped under the day before; today the VIX takes out yesterday's high
        let short = lt_ma(vh[i - 1], vma[i - 1]) && vh[i - 1] < vl[i - 2] && up_px[i].is_finite() && up_hod[i].is_finite();
        let long = gt_ma(vl[i - 1], vma[i - 1]) && vl[i - 1] > vh[i - 2] && dn_px[i].is_finite() && dn_lod[i].is_finite();
        let d = if short && side != 1.0 { -1.0 } else if long && side != -1.0 { 1.0 } else { 0.0 };
        if d == 0.0 {
            i += 1;
            continue;
        }
        let (px, stop0) = if d > 0.0 { (dn_px[i], dn_lod[i]) } else { (up_px[i], up_hod[i]) };
        let ext = |k: usize| if d > 0.0 { x.l[k] } else { x.h[k] };
        let mut t = Trade { dir: d, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "stop", signal_i: i, open: false, exits: Vec::new() };
        if d * (stop0 - ext(i)) > 0.0 {
            // the day's extreme went beyond the extreme-so-far at entry, so the stop traded after the fill
            t.exit_px = stop0;
            t.reason = format!("stop at the session {} at entry {}", if d > 0.0 { "low" } else { "high" }, fmt(stop0));
        } else if hold == 0 || i == to {
            t.reason = if hold == 0 { "day trade, out at the close".into() } else { "open".into() };
            t.open = hold > 0;
        } else {
            let mut k = i + 1;
            loop {
                let st = ext(k - 1); // trailing stop: prior bar's low (long) / high (short)
                if d * (st - ext(k)) >= 0.0 {
                    t.exit_px = if d > 0.0 { x.o[k].min(st) } else { x.o[k].max(st) };
                    t.reason = format!("trailing stop {}", fmt(st));
                    break;
                }
                if k - i >= hold || k == to {
                    t.exit_px = x.c[k];
                    t.open = k - i < hold;
                    t.reason = if t.open { "open".into() } else { format!("out at the day-{} close", k - i) };
                    break;
                }
                k += 1;
            }
            t.exit_i = k;
            t.signal_i = k;
        }
        i = t.exit_i + 1;
        out.push(t);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_management_scales_out_in_thirds() {
        // bar 0: long Window (H 10 < SMA 11), buy stop at 10, stop at 9
        // bar 1 (day 1): opens 9.8, trades to 10.5 -> filled at 10; closes 10.4 (< SMA 10.5): no piece
        // bar 2 (day 2): closes 10.8 > SMA 10.6 -> 1/3 at 10.8; day 2 in profit -> another 1/3 at 10.8; stop to 10
        // bar 3 (day 3): low 10.2 (> breakeven 10), closes 10.6 -> rest at 10.6
        let (o, h, l, c, ma) = ([9.5, 9.8, 10.5, 10.7], [10.0, 10.5, 10.9, 10.9], [9.0, 9.7, 10.3, 10.2], [9.6, 10.4, 10.8, 10.6], [11.0, 10.5, 10.6, 10.7]);
        let b = Bars { o: &o, h: &h, l: &l, c: &c, ma: &ma, wk: None };
        let sig = |i: usize| if i == 0 { vec![stop_order(&b, 0, 1.0, 0.0, false)] } else { vec![] };
        let t = sim(&b, &sig, Mgmt::Book { frac: 1.0 / 3.0, hold: 3, by_risk: false, needs_profit: true }, 0, 3);
        assert_eq!(t.len(), 1);
        let t = &t[0].0;
        assert_eq!((t.legs[0].i, t.legs[0].px, t.exit_i, t.open), (1, 10.0, 3, false));
        let want = (10.8 + 10.8 + 10.6) / 3.0;
        assert!((t.exit_px - want).abs() < 1e-12, "{} vs {want}: {}", t.exit_px, t.reason);
    }

    #[test]
    fn breakeven_stop_gapped_through() {
        // buy stop at 10 filled at the gap-up open 10.1; day 1 closes 10.6 > SMA 10.5 -> half out at 10.6, stop to 10.1;
        // day 2 opens 9.9 under the breakeven stop -> rest out at the open 9.9
        let (o, h, l, c, ma) = ([9.5, 10.1, 9.9], [10.0, 10.7, 10.0], [9.0, 10.0, 9.5], [9.6, 10.6, 9.8], [11.0, 10.5, 10.4]);
        let b = Bars { o: &o, h: &h, l: &l, c: &c, ma: &ma, wk: None };
        let sig = |i: usize| if i == 0 { vec![stop_order(&b, 0, 1.0, 0.0, false)] } else { vec![] };
        let t = sim(&b, &sig, Mgmt::Book { frac: 0.5, hold: 3, by_risk: false, needs_profit: true }, 0, 2);
        let t = &t[0].0;
        assert_eq!((t.legs[0].px, t.exit_i), (10.1, 2));
        assert!((t.exit_px - (0.5 * 10.6 + 0.5 * 9.9)).abs() < 1e-12, "{}", t.reason);
    }

    #[test]
    fn first_piece_needs_profit_and_ties_are_not_beyond() {
        // long filled at 10 on bar 1; bar 1 closes 9.95 > SMA 9.9 but at a loss -> no piece (first_needs_profit 1);
        // bar 2 closes 9.98 (no piece, no profit); bar 3 (day 3) -> all out at its close 10.4
        let (o, h, l, c, ma) = ([9.5, 9.8, 9.9, 10.0], [10.0, 10.1, 10.2, 10.5], [9.0, 9.7, 9.8, 9.9], [9.6, 9.95, 9.98, 10.4], [11.0, 9.9, 10.2, 10.5]);
        let b = Bars { o: &o, h: &h, l: &l, c: &c, ma: &ma, wk: None };
        let sig = |i: usize| if i == 0 { vec![stop_order(&b, 0, 1.0, 0.0, false)] } else { vec![] };
        let run = |np| sim(&b, &sig, Mgmt::Book { frac: 1.0 / 3.0, hold: 3, by_risk: false, needs_profit: np }, 0, 3);
        let t = &run(true)[0].0;
        assert!((t.exit_px - 10.4).abs() < 1e-12, "{}", t.reason);
        // old reading: 1/3 at the losing 9.95 close, breakeven stop 10 then hit on bar 2 (low 9.8) at the open 9.9
        let t = &run(false)[0].0;
        assert!((t.exit_px - (9.95 / 3.0 + 2.0 / 3.0 * 9.9)).abs() < 1e-12, "{}", t.reason);
        // a float-noise "tie" is not beyond; a real gap is (SPY 2002-05-08: L 106.79 vs fresh-sum SMA4 106.78999999999999)
        assert!(!beyond_ma(1.0, 106.79, 106.78999999999999) && !lt_ma(106.79, 106.79000000000001) && beyond_ma(-1.0, 106.78, 106.79));
    }

    #[test]
    fn two_rule_exit_short() {
        // short Window: L 11 > SMA 10; sell stop at 11 next bar (opens 11.2, low 9.8 -> fill 11)
        // entry-day close 9.9 < SMA 10.0 (1), next close 9.5 < SMA 9.8 (2) -> cover at 9.5
        let (o, h, l, c, ma) = ([11.5, 11.2, 9.8], [12.0, 11.3, 9.9], [11.0, 9.8, 9.4], [11.6, 9.9, 9.5], [10.0, 10.0, 9.8]);
        let b = Bars { o: &o, h: &h, l: &l, c: &c, ma: &ma, wk: None };
        let sig = |i: usize| if i == 0 { vec![Setup { stop: StopRule::None, ..stop_order(&b, 0, -1.0, 0.0, false) }] } else { vec![] };
        let t = sim(&b, &sig, Mgmt::TwoRule { check_day: 2, in_row: 2, next_open: false }, 0, 2);
        let t = &t[0].0;
        assert_eq!((t.dir, t.legs[0].px, t.exit_i, t.exit_px), (-1.0, 11.0, 2, 9.5));
    }

    #[test]
    fn weekly_fills_are_priced_and_booked_on_daily_bars() {
        // week 0 (days 0-4): Window bar H 10 < SMA 11 -> buy stop 10, stop 9.
        // week 1 (days 5-9): day 5 dips to 8.8 (under the stop, before any fill), day 7 gaps open at 10.2 -> long 10.2 on day 7;
        //   the pre-entry dip is no stop-out; week close 10.7 > SMA 10.6 -> 1/3 out, booked on day 9; breakeven stop 10.2.
        // week 2 (days 10-11): day 11 opens 10.8 and trades to 10.1 -> rest out at 10.2 on day 11.
        let mut dl: Vec<(f64, f64, f64, f64)> = vec![(9.5, 10.0, 9.0, 9.6); 5];
        dl.extend([(9.5, 9.8, 8.8, 9.6), (9.7, 9.9, 9.5, 9.8), (10.2, 10.5, 10.1, 10.4), (10.4, 10.6, 10.3, 10.5), (10.5, 10.8, 10.4, 10.7)]);
        dl.extend([(10.6, 10.9, 10.5, 10.8), (10.8, 10.9, 10.1, 10.3)]);
        let (dop, dh, dlo): (Vec<f64>, Vec<f64>, Vec<f64>) = (dl.iter().map(|x| x.0).collect(), dl.iter().map(|x| x.1).collect(), dl.iter().map(|x| x.2).collect());
        let span = [(0, 4), (5, 9), (10, 11)];
        let (o, h, l, c, ma) = ([9.5, 9.5, 10.6], [10.0, 10.8, 10.9], [9.0, 8.8, 10.1], [9.6, 10.7, 10.3], [11.0, 10.6, 10.5]);
        let b = Bars { o: &o, h: &h, l: &l, c: &c, ma: &ma, wk: Some(Wk { span: &span, o: &dop, h: &dh, l: &dlo }) };
        let sig = |i: usize| if i == 0 { vec![stop_order(&b, 0, 1.0, 0.0, false)] } else { vec![] };
        let t = sim(&b, &sig, Mgmt::Book { frac: 1.0 / 3.0, hold: 3, by_risk: false, needs_profit: true }, 0, 2);
        assert_eq!(t.len(), 1);
        let t = &t[0].0;
        assert_eq!((t.legs[0].i, t.legs[0].px, t.exit_i, t.open), (7, 10.2, 11, false), "{}", t.reason);
        assert_eq!(t.exits.iter().map(|e| (e.0, e.1)).collect::<Vec<_>>(), vec![(9, 10.7), (11, 10.2)]);
        for &(k, px, _) in t.exits.iter().chain([(t.legs[0].i, t.legs[0].px, 0.0)].iter()) {
            assert!(dlo[k] <= px && px <= dh[k], "fill {px} outside day {k}");
        }
    }

    #[test]
    fn mirrored_plus_di_is_minus_di() {
        let r: Vec<Row> = (0..60).map(|k| {
            let p = 100.0 - k as f64; // straight decline: -DI must dominate
            Row { ticker: String::new(), date: String::new(), open: p, high: p + 0.5, low: p - 1.0, close: p - 0.8, volume: None }
        }).collect();
        let (up, dn, adx) = trend(&r, 10, 30.0, 14, 30.0);
        assert!(dn[59] && !up[59] && adx[59] > 30.0);
    }
}
