//! Larry Connors, "5-Week Live Web Seminar" workbook (TradingMarkets / The Connors Group, 2003). Book id "ws5".
//!
//! The workbook publishes no statistics, only worked example trades on US stocks (2002-2003).
//! What is checked against the book: the entry / stop / profit-taking / trailing arithmetic of the
//! 3-day Daily Windows exit procedure and the Weekly Windows money management, replayed on the
//! prices printed in the book (unit tests at the bottom of this file).
//!
//! Conventions
//!   * Signals use data up to the close of bar i; orders work on bar i+1 (stop entries) or fill at close[i].
//!   * Intraday path of a daily bar = adverse extreme first: long O -> L -> H -> C, short O -> H -> L -> C
//!     (the brief's conservative "stop before target"). A stop-entry fills on the way to the favourable extreme,
//!     so the bar's adverse extreme is BEFORE the fill (exactly the book's "outside window" picture).
//!   * Gaps through a stop fill at the open; gaps through a profit limit fill at the open.
//!   * "Points": the book's dollar amounts (1-point limit, 1-point profit steps, 2/1-point trails, 3-point
//!     market-timing stop) are scaled by `pt_mode`: 0 = book dollars, 1 = % of price (`pt_pct`),
//!     2 = multiple of the 90-day average true range (`pt_atr`; the book's own filter #7 says a tradable
//!     stock has ATR(90) >= 1 point, so 1 point = 1 ATR(90) for the threshold stock).
//!   * Partial exits: a Trade keeps one entry leg; exit_px is the fraction-weighted average of all partial
//!     exit prices (so Trade::ret is exact), exit_i is the last exit bar, `reason` lists every piece.
#![allow(unused_imports, dead_code)]
use crate::connors::*;

pub const BOOK: &str = "ws5";

// ------------------------------------------------------------------ catalogue

const FILTERS_L: &str = "F|Filter #9 (do first): no new longs if the VIX is 5% or more below its 10-period SMA; no new shorts if 5% or more above";
const FILTERS_A: &str = "F|Filter #1: 10-period ADX ≥ 30 with +DI > -DI (longs) / -DI > +DI (shorts)";
const FILTERS_T: &str = "F|Filter #4: close above the 200-day SMA (longs) / below it (shorts)";
const FILTERS_S: &str = "F|Filter #8 Taglia Stochastics 21-period %K, 3-period smoothing, 9-period %D: under 80 for longs, above 20 for shorts";
const FILTERS_X: &str = "F|Filters #2 price ≥ $15, #3 volume ≥ 100,000 shares/day for the past 20 days (vol_rule: 1 = every day, the book wording; 0 = 20-day average [Not from the book]), #5 sub-sector vs its 200-day SMA, #6 90-day range > 6 points, #7 ATR(90) ≥ 1 point: stock-scan filters, OFF by default on an index (params stock_on / sector_on)";
const ENTRY: &str = "E|Order placement: buy stop near the Window day's high (short: sell stop near its low), limit 1 point beyond the stop; order good for the next day only";
const NEGATE: &str = "E|An opening gap of 1 point or more beyond the stop price negates the entry (trade not taken)";
const STOP: &str = "X|Initial protective stop near the previous day's low (short: high); if the entry day is an outside day, beyond the entry day's low (high)";
const D1: &str = "X|Day 1: when risk = reward take 1/3 off and move the stop to breakeven; 1/3 of the remaining off every further point; no trailing on Day 1";
const D2: &str = "X|Day 2: trail the stop 2 points from the intraday high (low) once the trade is 2 points in the money; keep taking 1/3 each further point";
const D2MA: &str = "X|Day 2: a close below (short: above) the 4-period SMA takes 1/3 of the remaining off; closes on the wrong side on Day 1 and Day 2 exit the trade";
const D3: &str = "X|Day 3: stop starts 1 point from Day 2's close and trails 1 point from the day's high (low); 1/3 off each further point; exit the rest on the Day 3 close";

const WNOTE: &str = "Book: US stocks from a nightly TC2000 scan, 2002-2003; run here on the index/ETF itself (vault convention). Point amounts scale with pt_mode (default 2: 1 point = 1 × ATR(90); 0 = the book's dollars). Daily bars cannot show the intraday order: the bar path is adverse extreme first (long O→L→H→C), entry stops fill on the way to the favourable extreme, gaps through stops/limits fill at the open. Not modelled: tightening the Day-3 trail to 0.50 'later in the day' and the discretionary market-timing aggressiveness at the end of Day 2 (switch day2_extra adds the examples' discretionary 'take 1/3 at the Day-2 close': 1 = when at most one piece is taken by then, as in GPRO, CAKE and WBSN; 2 = only when none is taken, as in CAKE alone; off by default). VIX filter uses the shared 10-day SMA of the VIX close (CBOE VIX for US; India VIX for India). ADX/DMI = Wilder, stochastic = slow %K (21 raw, 3 SMA) vs 80/20 (stoch_line 1 = the 9-period %D).";

fn wdef(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef {
        id, name, group: "Daily Momentum Windows (Sessions 1-2)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: true, needs_series: &[], book: BOOK, rules, notes, claim: None,
    }
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let mut v = vec![
        wdef("ws5_momentum_windows_any", "Momentum Windows · any Window", &[
            FILTERS_L, FILTERS_A, FILTERS_T, FILTERS_S, FILTERS_X,
            "E|Momentum Window: a strongly trending market pulled away from its 4-period SMA; long: the bar's high below the 4-SMA in an uptrend, short: the bar's low above it in a downtrend",
            "E|Any of the five Window setups (Single covers Double, Hanging and End-of-day Gap; Outside and Intraday Gap are triggered on the entry day)",
            ENTRY, NEGATE, "E|Intraday Gap: if the next day opens beyond the Window bar (long: below its low) and closes the gap, enter at the gap-close level",
            STOP, D1, D2, D2MA, D3],
            "Umbrella of the Nightly Trading Plan (S1 slides 64-65, Blueprint S5 slides 9-15; also extract-2 'ws5_daily_windows', merged here). Worked examples (Session 2 illustration, NOC, AZO, CBK) are checked as unit tests of the exit procedure. Intraday Gap precedence: when the open is beyond the Window bar, the gap-close entry is used. Intraday Gap stop: the lower of 0.50 below the entry or the day's low (short: higher of 0.50 above or the day's high). Stock filters off for indices. Other conventions as the Single Window."),
        wdef("ws5_single_window", "Single Window", &[
            FILTERS_L, FILTERS_A, FILTERS_T, FILTERS_S, FILTERS_X,
            "E|Single Window: a single bar whose high is below the 4-period SMA (long, uptrend) or whose low is above it (short, downtrend)",
            ENTRY, NEGATE, STOP, D1, D2, D2MA, D3], WNOTE),
        wdef("ws5_double_window", "Double Window", &[
            FILTERS_L, FILTERS_A, FILTERS_T, FILTERS_S, FILTERS_X,
            "E|Double Window: two Single Windows in a row (both bars entirely on the far side of the 4-period SMA)",
            ENTRY, NEGATE, STOP, D1, D2, D2MA, D3], WNOTE),
        wdef("ws5_hanging_window", "Hanging Window", &[
            FILTERS_L, FILTERS_A, FILTERS_T, FILTERS_S, FILTERS_X,
            "E|Hanging Window: a Window bar that opens and closes in the top 10% of its daily range for buys (bottom 10% for sells)",
            ENTRY, NEGATE, STOP, D1, D2, D2MA, D3], WNOTE),
        wdef("ws5_gap_window_eod", "End-of-day Gap Window", &[
            FILTERS_L, FILTERS_A, FILTERS_T, FILTERS_S, FILTERS_X,
            "E|End-of-day Gap Window: a gap that creates a Window on the close (long: today's high below yesterday's low and below the 4-SMA; short: mirror)",
            ENTRY, NEGATE, STOP, D1, D2, D2MA, D3], WNOTE),
        wdef("ws5_gap_window_intraday", "Intraday Gap Window", &[
            FILTERS_L, FILTERS_A, FILTERS_T, FILTERS_S, FILTERS_X,
            "E|Intraday Gap Window: a Windows setup that gaps the next day, then the gap is closed intraday",
            "E|Long: next day opens below the Window bar's low, enter when price trades back up to that low (short: opens above the high, enter back at the high)",
            "X|Initial protective stop: the higher of .50 or the high of that day (low of the day for buys)",
            D1, D2, D2MA, D3],
            "The book defines the trigger only loosely (TSCO, IGT charts). Reading: gap beyond the Window bar's extreme, entry at the gap-close level = that extreme; stop = farther of 0.50 point and the entry day's extreme (made before the fill on the adverse-first bar path). Daily-bar approximation of an intraday rule. Other conventions as the Single Window."),
        wdef("ws5_outside_window", "Outside Window", &[
            FILTERS_L, FILTERS_A, FILTERS_T, FILTERS_S, FILTERS_X,
            "E|Buys: a Windows setup that the next day trades below the Window day low and then takes out the Window high, triggering a buy (sells: above the high, then takes out the low)",
            "E|Buy stop at the Window day's high (sell stop at its low), limit 1 point beyond",
            "X|Initial stop beyond TODAY's low (high) because it is an outside day (ABK, WBSN examples)", D1, D2, D2MA, D3],
            "On a daily bar the order 'low first, then high' is assumed (adverse-first path), so any entry day that also took out the Window bar's other extreme before the fill counts. Other conventions as the Single Window."),
    ];
    v.push(StrategyDef {
        id: "ws5_weekly_windows", name: "Weekly Windows", group: "Weekly Windows (Session 3)", session: 0, dir: "Long/Short", status: Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|Sunday evening only: weekly bars and the 4-week SMA",
            "F|Weekly 10-period ADX ≥ 30 with +DI > -DI (longs) / -DI > +DI (shorts)",
            "F|Weekly close above the 200-week SMA (longs) / below it (shorts) (Blueprint says 200-day: param trend_weekly)",
            "F|Taglia Stochastics 21/3/9 on weekly bars: under 80 for longs, above 20 for shorts",
            "F|No VIX confirmation needed. Price $15, 500,000 shares/week for 20 weeks, sub-sector, 90-day range/ATR: stock filters, off on an index",
            "E|Weekly Window: the week's high below the 4-week SMA (long) / the week's low above it (short)",
            "E|Following week: buy stop at the previous week's high (sell stop at its low); a gap through fills at the open",
            "X|Initial protective stop near the previous week's low (short: high)",
            "X|Once risk equals reward take 1/4 off and move the stop to breakeven",
            "X|Trail the stop 2 points from the high (low) once the trade is 2 points in the money",
            "X|Take another 1/4 off at twice the initial risk; then trail 2 points from the high (low) until the stop is hit",
        ],
        notes: "Examples CMA, HIG, ZION (Jan-Feb 2003, shorts) are each checked as unit tests of the money management on the printed prices. The book scans index charts first (ADX/DI/200-period) then their components; here the index is both the scanned chart and the traded instrument. The second 1/4: rule 3 says 'of the remaining', rule 4 says half is then left; default = 1/4 of the original (second_of_orig = 1). Position managed on daily bars after entry, no time limit. Points scale with pt_mode (default 1 point = 1 × daily ATR(90)).",
        claim: None,
    });
    v.push(StrategyDef {
        id: "ws5_non_vix_market_timing", name: "Non-VIX Market Timing", group: "Market Timing (Session 4)", session: 0, dir: "Long/Short", status: Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "E|Buys: today's high of the SPX (NDX, XBD or SOX) is below its 10-period MA and today's close is 2.0% (6.0%, 8.0%, 8.0%) or more below it; buy the market on the close",
            "E|Sells: today's low is above the 10-period MA and the close is 2.0% (6.0%, 8.0%, 8.0%) or more above it; sell the market on the close",
            "X|3-point stop on all positions",
            "X|Exit ½ on the close of the day the price trades (intraday) above yesterday's 10-day MA (sells: below); close the other ½ the following day",
            "X|Or exit within two to four days (default: on the close of day 4)",
        ],
        notes: "Book trades SPY/S&P (2003). Threshold `pct` 2.0 = SPX; book values NDX 6, XBD 8, SOX 8 (use 6 for QQQ / NIFTY IT, 8 for SMH). 3-point stop = SPY points: pt_mode 1 with pt_pct 1.18 (1 point = 1/85 of price, SPY ≈ $85 in the 2003 sizing slide); pt_mode 0 = raw points. Stop checked before the reversion exit on the same bar.",
        claim: claim("-", "\"just over 70%\" (market-timing signals in general)", "-", "2-4 days", "general claim, no test published"),
    });
    v.push(StrategyDef {
        id: "ws5_vix_iii_reversal", name: "VIX III Reversal (CVR III)", group: "Market Timing (Session 4)", session: 0, dir: "Long/Short", status: Live,
        needs_volume: false, needs_vix: false, needs_series: &["VIX.High", "VIX.Low", "VIX.Close"], book: BOOK,
        rules: &[
            "E|Buys: today the low of the VIX is above its 10-day MA and the VIX closes at least 10% above it; buy the market on the close",
            "E|Sells: today the high of the VIX is below its 10-day MA and the VIX closes at least 10% below it; sell the market on the close",
            "X|3-point stop on all positions",
            "X|\"Look to exit ½ position on the close of the day the VIX trades (intraday) above yesterday's 10-day moving average (reversion to the mean). Close out the remaining ½ position the following day.\" (slide 29, one sentence for buys and sells; coded as written: VIX high > yesterday's MA)",
            "X|[Not from the book] literal_exit = 0: buys exit instead when the VIX trades BELOW yesterday's MA (presumed copy of the Non-VIX slide; the reversion reading for a VIX stretched above its MA)",
            "X|Or exit within two to four days (default: on the close of day 4)",
        ],
        notes: "Uses the VIX's daily high/low/close, which the server builds in as series VIX.High, VIX.Low, VIX.Close (CBOE VIX for US, India VIX daily OHLC for India); no extra_data file is needed. The slide's exit says 'trades above yesterday's 10-day MA' for both sides; default literal_exit = 1 codes the slide as written (buys: the VIX was above its MA on the signal day, so the ½ exit usually comes on day 1). [Not from the book] literal_exit = 0 reads it as a copy error and exits buys when the VIX trades BELOW yesterday's MA (the reversion reading). 3-point stop as in the Non-VIX method (pt_pct 1.18 = SPY at $85).",
        claim: claim("-", "\"just over 70%\" (market-timing signals in general)", "-", "2-4 days", "general claim, no test published"),
    });
    v.push(StrategyDef {
        id: "ws5_emini_spy_daytrade", name: "E-mini/SPY VIX Gap Day Trade", group: "Market Timing (Session 4)", session: 0, dir: "Long/Short", status: Live,
        needs_volume: false, needs_vix: false,
        needs_series: &["VIX.Open", "VIX.High", "VIX.Low", "VIX.Close", "VIXFILL_PX", "VIXFILL_HIGH", "VIXFILL_LOW"], book: BOOK,
        rules: &[
            "E|Buys: the VIX closed yesterday above its 4-period MA; today the VIX opens above yesterday's high (gap); buy the E-minis/SPYs immediately when the VIX closes the gap (trades below yesterday's high)",
            "E|Sells: the VIX closed yesterday under its 4-period MA; today the VIX opens below yesterday's low; sell when the VIX trades above yesterday's low",
            "X|Risk 1.50 SPY points (15 E-mini points)",
            "X|Same money management as Windows: at risk = reward take 1/3 off and move the stop to breakeven; 1/3 more each further point",
            "X|Flat at the close",
        ],
        notes: "DATA GAP: no VIXFILL_* series exist in the vault yet (they need synchronous intraday VIX and instrument bars), so every run reports a data gap until they are added. Intraday rule coded against daily series derived from intraday bars: VIXFILL_PX = the traded instrument's price at the first moment the VIX trades back inside yesterday's range after a gap open outside it (NaN when no such fill), VIXFILL_HIGH / VIXFILL_LOW = the instrument's high / low from that moment to the close (per ticker: <TICKER>__VIXFILL_PX.csv etc.). Path after the fill: adverse extreme first. Not modelled: the 0.50 trail 'halfway through the day'.",
        claim: None,
    });
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = ("ws5_momentum_windows", "Momentum Windows (Daily)");
    Some(match id {
        "ws5_momentum_windows_any" => (f.0, f.1, "Any Window"),
        "ws5_single_window" => (f.0, f.1, "Single"),
        "ws5_double_window" => (f.0, f.1, "Double"),
        "ws5_hanging_window" => (f.0, f.1, "Hanging"),
        "ws5_gap_window_eod" => (f.0, f.1, "End-of-day Gap"),
        "ws5_gap_window_intraday" => (f.0, f.1, "Intraday Gap"),
        "ws5_outside_window" => (f.0, f.1, "Outside"),
        "ws5_weekly_windows" => ("ws5_weekly_windows", "Weekly Windows", ""),
        "ws5_non_vix_market_timing" => ("ws5_market_timing", "Connors 2003 Market Timing", "Non-VIX (price vs 10-day MA)"),
        "ws5_vix_iii_reversal" => ("ws5_market_timing", "Connors 2003 Market Timing", "VIX III Reversal"),
        "ws5_emini_spy_daytrade" => ("ws5_market_timing", "Connors 2003 Market Timing", "E-mini/SPY VIX gap day trade"),
        _ => return None,
    })
}

// ------------------------------------------------------------------ params

fn p(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef {
    ParamDef { key, label, def, min, max, step }
}

fn units(pct_def: f64, mode_def: f64) -> Vec<ParamDef> {
    vec![
        p("pt_mode", "Point unit: 0 book $, 1 % of price, 2 × ATR(90)", mode_def, 0.0, 2.0, 1.0),
        p("pt_pct", "1 point = % of price (mode 1)", pct_def, 0.25, 5.0, 0.25),
        p("pt_atr", "1 point = × ATR(90) (mode 2)", 1.0, 0.25, 3.0, 0.25),
    ]
}

fn trend_filters(weekly: bool) -> Vec<ParamDef> {
    let mut v = vec![
        p("side", "Sides: 0 both, 1 long only, -1 short only", 0.0, -1.0, 1.0, 1.0),
        p("adx_n", "ADX / DMI period", 10.0, 7.0, 20.0, 1.0),
        p("adx_min", "ADX at least", 30.0, 15.0, 40.0, 5.0),
        p("trend", if weekly { "Trend SMA length (weeks)" } else { "Trend SMA length (days)" }, 200.0, 50.0, 250.0, 25.0),
        p("window_ma", "Window SMA length", 4.0, 3.0, 8.0, 1.0),
        p("stoch_n", "Stochastic %K period", 21.0, 9.0, 28.0, 1.0),
        p("stoch_smooth", "Stochastic %K smoothing", 3.0, 1.0, 5.0, 1.0),
        p("stoch_d", "Stochastic %D period", 9.0, 3.0, 9.0, 1.0),
        p("stoch_line", "Stochastic line: 0 slow %K, 1 %D", 0.0, 0.0, 1.0, 1.0),
        p("stoch_long", "Longs only if stochastic below", 80.0, 60.0, 100.0, 5.0),
        p("stoch_short", "Shorts only if stochastic above", 20.0, 0.0, 40.0, 5.0),
        p("stock_on", "Stock filters price/volume/range/ATR (0 off for indices)", 0.0, 0.0, 1.0, 1.0),
        p("min_price", "Stock filter: price at least $", 15.0, 5.0, 30.0, 5.0),
        p("min_vol", if weekly { "Stock filter: shares per week at least" } else { "Stock filter: shares per day at least" }, if weekly { 500_000.0 } else { 100_000.0 }, 0.0, 2_000_000.0, 100_000.0),
        p("vol_rule", "Volume filter: 0 = N-period average ≥ min [Not from the book], 1 = every one of the last N ≥ min (book wording)", 1.0, 0.0, 1.0, 1.0),
        p("vol_n", if weekly { "Volume lookback (weeks)" } else { "Volume lookback (days)" }, 20.0, 5.0, 40.0, 5.0),
        p("range_n", "Range / ATR lookback (days)", 90.0, 30.0, 120.0, 10.0),
        p("min_range", "Stock filter: 90-day range above (book $)", 6.0, 0.0, 20.0, 1.0),
        p("min_atr", "Stock filter: ATR(90) at least (book $)", 1.0, 0.0, 5.0, 0.5),
        p("sector_on", "Sub-sector filter on series SUBSECTOR (0 off)", 0.0, 0.0, 1.0, 1.0),
        p("entry_off", "Entry stop: points beyond the bar's extreme", 0.01, 0.0, 0.25, 0.01),
        p("stop_off", "Initial stop: points beyond the bar's extreme", 0.01, 0.0, 0.25, 0.01),
    ];
    if weekly {
        v.push(p("trend_weekly", "Trend SMA on weekly bars (1) or daily (0)", 1.0, 0.0, 1.0, 1.0));
    }
    v
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let windows = |extra: Vec<ParamDef>| {
        let mut v = trend_filters(false);
        v.extend([
            p("vix_on", "VIX market filter (1 on)", 1.0, 0.0, 1.0, 1.0),
            p("vix_pct", "VIX % from its 10-day SMA that blocks entries", 5.0, 2.0, 15.0, 1.0),
            p("gap_neg", "Entry limit / gap negation: points beyond the stop", 1.0, 0.25, 3.0, 0.25),
            p("part", "Take 1/N of the remaining at each target", 3.0, 2.0, 4.0, 1.0),
            p("step", "Further profit every N points", 1.0, 0.25, 3.0, 0.25),
            p("trail2_start", "Day 2: trail once N points in the money", 2.0, 0.5, 4.0, 0.5),
            p("trail2", "Day 2: trail N points from the high", 2.0, 0.5, 4.0, 0.5),
            p("d3_close", "Day 3: stop N points from Day 2's close", 1.0, 0.25, 3.0, 0.25),
            p("d3_trail", "Day 3: trail N points from the high", 1.0, 0.25, 3.0, 0.25),
            p("max_days", "Exit on the close of day N", 3.0, 1.0, 6.0, 1.0),
            p("ma_rule", "Day 2 4-SMA close rules (1 on)", 1.0, 0.0, 1.0, 1.0),
            p("day2_extra", "Extra 1/3 at the Day-2 close (discretionary): 0 off, 1 if at most one piece taken (GPRO, CAKE, WBSN), 2 only if none taken", 0.0, 0.0, 2.0, 1.0),
        ]);
        v.extend(units(3.0, 2.0));
        v.extend(extra);
        v
    };
    Some(match id {
        "ws5_momentum_windows_any" => windows(vec![p("ig_stop", "Intraday Gap: stop at least N points from entry", 0.5, 0.1, 2.0, 0.1)]),
        "ws5_single_window" | "ws5_outside_window" | "ws5_gap_window_eod" => windows(vec![]),
        "ws5_double_window" => windows(vec![p("bars", "Window bars in a row", 2.0, 2.0, 4.0, 1.0)]),
        "ws5_hanging_window" => windows(vec![p("hang_pct", "Open and close within top/bottom % of range", 10.0, 5.0, 30.0, 5.0)]),
        "ws5_gap_window_intraday" => windows(vec![p("ig_stop", "Stop at least N points from entry", 0.5, 0.1, 2.0, 0.1)]),
        "ws5_weekly_windows" => {
            let mut v = trend_filters(true);
            v.extend([
                p("part1", "First piece at risk = reward (fraction)", 0.25, 0.1, 0.5, 0.05),
                p("part2", "Second piece at 2 × risk (fraction)", 0.25, 0.0, 0.5, 0.05),
                p("second_of_orig", "Second piece: 1 = of the original, 0 = of the remaining", 1.0, 0.0, 1.0, 1.0),
                p("trail_start", "Trail once N points in the money", 2.0, 0.5, 4.0, 0.5),
                p("trail", "Trail N points from the high", 2.0, 0.5, 4.0, 0.5),
            ]);
            v.extend(units(3.0, 2.0));
            v
        }
        "ws5_non_vix_market_timing" | "ws5_vix_iii_reversal" => {
            let vix = id == "ws5_vix_iii_reversal";
            let mut v = vec![
                p("side", "Sides: 0 both, 1 long only, -1 short only", 0.0, -1.0, 1.0, 1.0),
                p("ma", "MA length", 10.0, 5.0, 20.0, 1.0),
                p("pct", if vix { "VIX close % beyond its MA" } else { "Close % beyond its MA" }, if vix { 10.0 } else { 2.0 }, 1.0, if vix { 25.0 } else { 10.0 }, if vix { 1.0 } else { 0.5 }),
                p("stop_pts", "Stop (points)", 3.0, 1.0, 6.0, 0.5),
                p("max_hold", "Exit by the close of day N", 4.0, 2.0, 6.0, 1.0),
            ];
            if vix {
                v.push(p("literal_exit", "Buys exit when VIX trades ABOVE yesterday's MA (slide text p-130, 1) / below (0, [Not from the book] presumed copy error)", 1.0, 0.0, 1.0, 1.0));
            }
            v.extend(units(100.0 / 85.0, 1.0));
            v
        }
        "ws5_emini_spy_daytrade" => {
            let mut v = vec![
                p("side", "Sides: 0 both, 1 long only, -1 short only", 0.0, -1.0, 1.0, 1.0),
                p("vix_ma", "VIX MA length", 4.0, 2.0, 10.0, 1.0),
                p("risk_pts", "Initial risk (points)", 1.5, 0.5, 3.0, 0.25),
                p("part", "Take 1/N of the remaining at each target", 3.0, 2.0, 4.0, 1.0),
                p("step", "Further profit every N points", 1.0, 0.25, 3.0, 0.25),
            ];
            v.extend(units(100.0 / 85.0, 1.0));
            v
        }
        _ => return None,
    })
}

// ------------------------------------------------------------------ local indicators

/// Wilder DMI on raw arrays: (+DI, -DI, ADX). Same formulas as connors::dmi, which does not return -DI.
fn dmi3(h: &[f64], l: &[f64], c: &[f64], n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = c.len();
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 1..len {
        let (up, dn) = (h[i] - h[i - 1], l[i - 1] - l[i]);
        pdm[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        mdm[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        tr[i] = (h[i] - l[i]).max((h[i] - c[i - 1]).abs()).max((l[i] - c[i - 1]).abs());
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&tr, n));
    let (mut pdi, mut mdi, mut dx) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 0..len {
        if st[i] > 0.0 {
            let (a, b) = (100.0 * sp[i] / st[i], 100.0 * sm[i] / st[i]);
            pdi[i] = a;
            mdi[i] = b;
            dx[i] = if a + b > 0.0 { 100.0 * (a - b).abs() / (a + b) } else { 0.0 };
        }
    }
    (pdi, mdi, wilder(&dx, n))
}

/// Stochastic: raw %K over n bars, %K smoothed by an SMA(smooth), %D = SMA(d) of that. Returns (slow %K, %D).
fn stoch(h: &[f64], l: &[f64], c: &[f64], n: usize, smooth: usize, d: usize) -> (Vec<f64>, Vec<f64>) {
    let raw: Vec<f64> = (0..c.len())
        .map(|i| {
            if i + 1 < n {
                return f64::NAN;
            }
            let hh = h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max);
            let ll = l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min);
            if hh > ll { 100.0 * (c[i] - ll) / (hh - ll) } else { 50.0 }
        })
        .collect();
    let k = sma_nan(&raw, smooth);
    let dd = sma_nan(&k, d);
    (k, dd)
}

/// SMA that is NaN while any input in its window is NaN (connors::sma would carry a NaN forever).
fn sma_nan(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len())
        .map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 })
        .collect()
}

/// Filter #3 volume series: N-period average (vol_rule 0) or N-period minimum (vol_rule 1); NaN while any input is NaN.
fn vol_filter(v: &[f64], n: usize, min_rule: bool) -> Vec<f64> {
    if !min_rule {
        return sma_nan(v, n);
    }
    (0..v.len())
        .map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().cloned().fold(f64::INFINITY, |a, b| if b.is_nan() || a.is_nan() { f64::NAN } else { a.min(b) }) })
        .collect()
}

/// TradeStation AvgTrueRange: simple mean of the true range.
fn avg_tr(h: &[f64], l: &[f64], c: &[f64], n: usize) -> Vec<f64> {
    let tr: Vec<f64> = (0..c.len())
        .map(|i| if i == 0 { h[0] - l[0] } else { (h[i] - l[i]).max((h[i] - c[i - 1]).abs()).max((l[i] - c[i - 1]).abs()) })
        .collect();
    sma_nan(&tr, n)
}

/// Days since 1970-01-01 for "YYYY-MM-DD".
fn day_num(s: &str) -> i64 {
    let y: i64 = s.get(0..4).and_then(|v| v.parse().ok()).unwrap_or(1970);
    let m: i64 = s.get(5..7).and_then(|v| v.parse().ok()).unwrap_or(1);
    let d: i64 = s.get(8..10).and_then(|v| v.parse().ok()).unwrap_or(1);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

// ------------------------------------------------------------------ exit engines (long space; shorts are negated)

/// Price tolerance for touching a level (book prices are printed to the cent).
const EPS: f64 = 1e-9;

/// [Not from the book] Tie tolerance for price-vs-SMA comparisons: an SMA can equal a price exactly in decimal
/// arithmetic but differ by float rounding, so a strict "above / below" needs a margin of 1e-9 (relative) and a
/// tie counts as "not beyond"; "at least" comparisons accept a tie.
const TIE: f64 = 1e-9;
fn gt(a: f64, b: f64) -> bool {
    a > b + TIE * b.abs()
}
fn lt(a: f64, b: f64) -> bool {
    a < b - TIE * b.abs()
}
fn ge(a: f64, b: f64) -> bool {
    a >= b - TIE * b.abs()
}
fn le(a: f64, b: f64) -> bool {
    a <= b + TIE * b.abs()
}

pub(crate) struct Fill {
    pub day: usize, // 1-based day of the trade
    pub px: f64,
    pub frac: f64, // fraction of the ORIGINAL position
    pub why: String,
}

#[derive(Clone, Copy)]
pub(crate) struct Rules {
    pub pt: f64,
    pub part: f64,         // take 1/part of the remaining at each target
    pub step: f64,         // further target every `step` points
    pub trail2_start: f64, // Day 2+: trail once this many points in the money
    pub trail2: f64,       // ... this many points from today's high
    pub d3_close: f64,     // Day 3 pre-open stop: points from Day 2's close
    pub d3_trail: f64,     // Day 3+: trail points from today's high
    pub max_days: usize,
    pub ma_rule: bool,
    pub day2_extra: Option<usize>, // take 1/part at the Day-2 close if at most this many pieces are taken (None = off)
}

/// The Daily Windows 3-day exit procedure (Session 2), in long space.
/// days[k] = price path of day k+1 (day 1 starts at the entry fill, later days at the open; last point = close),
/// below[k] = that day's close is below the 4-period SMA. Returns the fills and whether a remainder is still open
/// (an open remainder is the last fill, with px NaN and why "open").
pub(crate) fn manage3(e: f64, stop0: f64, r: &Rules, days: &[Vec<f64>], below: &[bool]) -> (Vec<Fill>, bool) {
    let mut fills = Vec::new();
    let risk = e - stop0;
    if !(risk > 0.0) || !(r.pt > 0.0) || !(r.step > 0.0) {
        fills.push(Fill { day: 1, px: e, frac: 1.0, why: "no risk: flat".into() });
        return (fills, false);
    }
    let (mut stop, mut rem, mut tgt, mut profit) = (stop0, 1.0, e + risk, false);
    let mut prev_close = e;
    for (k, path) in days.iter().enumerate() {
        let day = k + 1;
        if day == 3 {
            stop = stop.max(prev_close - r.d3_close * r.pt);
        }
        let mut hi = f64::NEG_INFINITY;
        let mut last = f64::NAN;
        for (j, &px) in path.iter().enumerate() {
            if j == 0 && day > 1 && px <= stop + EPS {
                fills.push(Fill { day, px, frac: rem, why: format!("stop gapped, out at the open (D{day})") });
                return (fills, false);
            }
            if j == 0 || px > last {
                while tgt <= px + EPS && rem > 1e-12 {
                    let f = rem / r.part;
                    let tag = if profit { "+1 pt" } else { "R=R" };
                    fills.push(Fill { day, px: if j == 0 { px } else { tgt }, frac: f, why: format!("1/{} {tag} (D{day})", r.part) });
                    rem -= f;
                    profit = true;
                    stop = stop.max(e);
                    tgt += r.step * r.pt;
                }
                hi = hi.max(px);
                if day >= 2 && hi - e >= r.trail2_start * r.pt {
                    stop = stop.max(hi - r.trail2 * r.pt);
                }
                if day >= 3 {
                    stop = stop.max(hi - r.d3_trail * r.pt);
                }
            } else if px <= stop + EPS {
                let why = if stop > e { "trailing stop" } else if stop == e { "breakeven stop" } else { "stop" };
                fills.push(Fill { day, px: stop, frac: rem, why: format!("{why} (D{day})") });
                return (fills, false);
            }
            last = px;
        }
        prev_close = last;
        if day == 2 && r.ma_rule && below.get(1) == Some(&true) {
            if below[0] {
                fills.push(Fill { day, px: last, frac: rem, why: "2 closes below 4-SMA (D2)".into() });
                return (fills, false);
            }
            let f = rem / r.part;
            fills.push(Fill { day, px: last, frac: f, why: "1/3 close below 4-SMA (D2)".into() });
            rem -= f;
        } else if day == 2 && r.day2_extra.map_or(false, |mx| fills.len() <= mx) {
            // fills holds only the profit pieces here (any stop has returned): GPRO / WBSN had 1, CAKE had 0
            let f = rem / r.part;
            fills.push(Fill { day, px: last, frac: f, why: format!("1/3 at Day-2 close, {} piece(s) taken (D2)", fills.len()) });
            rem -= f;
        }
        if day >= r.max_days {
            fills.push(Fill { day, px: last, frac: rem, why: format!("Day {day} close") });
            return (fills, false);
        }
    }
    fills.push(Fill { day: days.len().max(1), px: f64::NAN, frac: rem, why: "open".into() });
    (fills, true)
}

/// Weekly Windows money management (Session 3 rules 1-4), long space, managed on daily paths, no time limit.
pub(crate) fn manage_weekly(e: f64, stop0: f64, pt: f64, part1: f64, part2: f64, second_of_orig: bool, trail_start: f64, trail: f64, days: &[Vec<f64>]) -> (Vec<Fill>, bool) {
    let mut fills = Vec::new();
    let risk = e - stop0;
    if !(risk > 0.0) || !(pt > 0.0) {
        fills.push(Fill { day: 1, px: e, frac: 1.0, why: "no risk: flat".into() });
        return (fills, false);
    }
    let (mut stop, mut rem, mut n_taken, mut hi) = (stop0, 1.0, 0, f64::NEG_INFINITY);
    for (k, path) in days.iter().enumerate() {
        let day = k + 1;
        let mut last = f64::NAN;
        for (j, &px) in path.iter().enumerate() {
            if j == 0 && day > 1 && px <= stop + EPS {
                fills.push(Fill { day, px, frac: rem, why: format!("stop gapped, out at the open (day {day})") });
                return (fills, false);
            }
            if j == 0 || px > last {
                for (n, lvl) in [(0, e + risk), (1, e + 2.0 * risk)] {
                    if n_taken == n && lvl <= px + EPS {
                        let f = if n == 0 { part1 } else if second_of_orig { part2 } else { rem * part2 };
                        let f = f.min(rem);
                        fills.push(Fill { day, px: if j == 0 { px } else { lvl }, frac: f, why: format!("{} at {}R (day {day})", if n == 0 { "1st piece" } else { "2nd piece" }, n + 1) });
                        rem -= f;
                        n_taken += 1;
                        stop = stop.max(e);
                    }
                }
                hi = hi.max(px);
                if hi - e >= trail_start * pt {
                    stop = stop.max(hi - trail * pt);
                }
            } else if px <= stop + EPS {
                let why = if stop > e { "trailing stop" } else if stop == e { "breakeven stop" } else { "stop" };
                fills.push(Fill { day, px: stop, frac: rem, why: format!("{why} (day {day})") });
                return (fills, false);
            }
            last = px;
        }
    }
    fills.push(Fill { day: days.len().max(1), px: f64::NAN, frac: rem, why: "open".into() });
    (fills, true)
}

/// Long-space daily path, adverse extreme first: [O, L, H, C] (shorts: prices negated, so the high comes first).
fn bar_path(x: &Ctx, i: usize, d: f64) -> Vec<f64> {
    let (a, b) = (d * x.l[i], d * x.h[i]);
    vec![d * x.o[i], a.min(b), a.max(b), d * x.c[i]]
}

/// Long-space fills -> Trade in real prices. Day 1 = bar `entry_i`; an open remainder is marked at close[to].
fn to_trade(x: &Ctx, d: f64, entry_i: usize, e: f64, fills: &[Fill], open: bool, to: usize, kind: &'static str, label: &str) -> Trade {
    let mut px_sum = 0.0;
    let mut exit_i = entry_i;
    let mut parts = Vec::new();
    let mut exits = Vec::new();
    for f in fills {
        let (bar, px) = if f.why == "open" { (to, x.c[to]) } else { ((entry_i + f.day - 1).min(to), d * f.px) };
        px_sum += f.frac * px;
        exits.push((bar, px, f.frac));
        exit_i = exit_i.max(bar);
        parts.push(format!("{} {:.0}% @ {:.2}", f.why, f.frac * 100.0, px));
    }
    Trade {
        dir: d,
        legs: vec![Leg { i: entry_i, px: d * e, w: 1.0 }],
        exit_i,
        exit_px: px_sum,
        reason: format!("{label}: {}", parts.join("; ")),
        entry_kind: kind,
        signal_i: exit_i,
        open,
        exits: if exits.len() > 1 { exits } else { Vec::new() },
    }
}

// ------------------------------------------------------------------ run

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Any,
    Single,
    Double,
    Hanging,
    GapEod,
    GapIntraday,
    Outside,
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, prm: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| prm.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let kind = match id {
        "ws5_momentum_windows_any" => Kind::Any,
        "ws5_single_window" => Kind::Single,
        "ws5_double_window" => Kind::Double,
        "ws5_hanging_window" => Kind::Hanging,
        "ws5_gap_window_eod" => Kind::GapEod,
        "ws5_gap_window_intraday" => Kind::GapIntraday,
        "ws5_outside_window" => Kind::Outside,
        "ws5_weekly_windows" => return Some(weekly(x, from, to, &g, &u)),
        "ws5_non_vix_market_timing" | "ws5_vix_iii_reversal" => return Some(market_timing(id, x, from, to, &g, &u)),
        "ws5_emini_spy_daytrade" => return Some(emini(x, from, to, &g, &u)),
        _ => return None,
    };
    Some(daily_windows(kind, x, from, to, &g, &u))
}

fn sides(g: &dyn Fn(&str) -> f64) -> Vec<f64> {
    match g("side").round() as i64 {
        1 => vec![1.0],
        -1 => vec![-1.0],
        _ => vec![1.0, -1.0],
    }
}

/// Size of one book "point" at bar i.
fn point_fn<'a>(x: &'a Ctx, g: &dyn Fn(&str) -> f64, atr90: &'a [f64]) -> impl Fn(usize) -> f64 + 'a {
    let (mode, pct, k) = (g("pt_mode").round() as i64, g("pt_pct") / 100.0, g("pt_atr"));
    move |i: usize| match mode {
        0 => 1.0,
        1 => x.c[i] * pct,
        _ => atr90[i] * k,
    }
}

/// Filters #1 (ADX/DI), #4 (trend SMA) and #8 (stochastic) on one bar series.
struct Trend {
    pdi: Vec<f64>,
    mdi: Vec<f64>,
    adx: Vec<f64>,
    tr: Vec<f64>,
    st: Vec<f64>,
    adx_min: f64,
    st_long: f64,
    st_short: f64,
}
impl Trend {
    fn new(h: &[f64], l: &[f64], c: &[f64], g: &dyn Fn(&str) -> f64, u: &dyn Fn(&str) -> usize, tr: Vec<f64>) -> Self {
        let (pdi, mdi, adx) = dmi3(h, l, c, u("adx_n"));
        let (k, dd) = stoch(h, l, c, u("stoch_n"), u("stoch_smooth"), u("stoch_d"));
        Trend { pdi, mdi, adx, tr, st: if g("stoch_line") >= 0.5 { dd } else { k }, adx_min: g("adx_min"), st_long: g("stoch_long"), st_short: g("stoch_short") }
    }
    fn ok(&self, i: usize, d: f64, close: f64) -> bool {
        self.adx[i] >= self.adx_min
            && if d > 0.0 {
                self.pdi[i] > self.mdi[i] && gt(close, self.tr[i]) && self.st[i] < self.st_long
            } else {
                self.mdi[i] > self.pdi[i] && lt(close, self.tr[i]) && self.st[i] > self.st_short
            }
    }
}

/// Optional stock-scan filters in book dollars: #2 price, #3 volume (avg or min, see vol_filter), #6 range, #7 ATR, at daily bar i.
fn stock_ok(x: &Ctx, g: &dyn Fn(&str) -> f64, u: &dyn Fn(&str) -> usize, atr: &[f64], close: f64, vol_avg: f64, i: usize) -> bool {
    if g("stock_on") < 0.5 {
        return true;
    }
    let n = u("range_n");
    if i + 1 < n {
        return false;
    }
    let hh = x.h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max);
    let ll = x.l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min);
    close >= g("min_price") && vol_avg >= g("min_vol") && hh - ll > g("min_range") && atr[i] >= g("min_atr")
}

/// Filter #5: sub-sector series vs its trend SMA (off unless sector_on; on without the series = no trades).
fn sector_ok(g: &dyn Fn(&str) -> f64, sec: Option<&(Vec<f64>, Vec<f64>)>, k: usize, d: f64) -> bool {
    if g("sector_on") < 0.5 {
        return true;
    }
    sec.map_or(false, |(s, m)| if d > 0.0 { gt(s[k], m[k]) } else { lt(s[k], m[k]) })
}

fn daily_windows(kind: Kind, x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64, u: &dyn Fn(&str) -> usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let m = sma(c, u("window_ma"));
    let atr90 = avg_tr(h, l, c, u("range_n"));
    let pt = point_fn(x, g, &atr90);
    let tf = Trend::new(h, l, c, g, u, sma(c, u("trend")));
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let vol_avg = vol_filter(&vol, u("vol_n"), g("vol_rule") >= 0.5);
    let sec = x.series("SUBSECTOR").map(|s| (s.to_vec(), sma_nan(s, u("trend"))));
    let (vix_on, vp) = (g("vix_on") >= 0.5, g("vix_pct"));
    let (vix_lo, vix_hi) = (x.vix.stretched(-vp), x.vix.stretched(vp));
    let rules = |i: usize| Rules {
        pt: pt(i), part: g("part"), step: g("step"), trail2_start: g("trail2_start"), trail2: g("trail2"),
        d3_close: g("d3_close"), d3_trail: g("d3_trail"), max_days: u("max_days"), ma_rule: g("ma_rule") >= 0.5,
        day2_extra: match g("day2_extra").round() as i64 { 1 => Some(1), 2 => Some(0), _ => None },
    };
    let (eoff, soff, neg, max_days) = (g("entry_off"), g("stop_off"), g("gap_neg"), u("max_days"));
    let ig = if matches!(kind, Kind::Any | Kind::GapIntraday) { g("ig_stop") } else { 0.0 };
    let bars = if kind == Kind::Double { u("bars") } else { 1 };
    let hang = if kind == Kind::Hanging { g("hang_pct") / 100.0 } else { 0.0 };
    let dirs = sides(g);

    // Window on bar i in direction d (long: bar entirely below the window SMA, in an uptrend)
    let win = |i: usize, d: f64| if d > 0.0 { lt(h[i], m[i]) } else { gt(l[i], m[i]) };
    let setup = |i: usize, d: f64| -> bool {
        if !win(i, d) {
            return false;
        }
        match kind {
            Kind::Double => i + 1 >= bars && (i + 1 - bars..=i).all(|k| win(k, d)),
            Kind::Hanging => {
                let r = h[i] - l[i];
                r > 0.0
                    && if d > 0.0 {
                        (o[i] - l[i]) / r >= 1.0 - hang && (c[i] - l[i]) / r >= 1.0 - hang
                    } else {
                        (h[i] - o[i]) / r >= 1.0 - hang && (h[i] - c[i]) / r >= 1.0 - hang
                    }
            }
            Kind::GapEod => i > 0 && if d > 0.0 { h[i] < l[i - 1] } else { l[i] > h[i - 1] },
            _ => true,
        }
    };
    let filters = |i: usize, d: f64| -> bool {
        let vix_block = vix_on && if d > 0.0 { vix_lo[i] >= 1 } else { vix_hi[i] >= 1 };
        !vix_block && tf.ok(i, d, c[i]) && stock_ok(x, g, u, &atr90, c[i], vol_avg[i], i) && sector_ok(g, sec.as_ref(), i, d)
    };

    let mut trades = Vec::new();
    let mut i = from;
    while i < to.min(n - 1) {
        let e_i = i + 1;
        let mut done = None;
        for &d in &dirs {
            if !(setup(i, d) && filters(i, d)) {
                continue;
            }
            let p = pt(i);
            if !(p > 0.0) {
                continue;
            }
            let (wh, wl) = ((d * h[i]).max(d * l[i]), (d * h[i]).min(d * l[i]));
            let bp = bar_path(x, e_i, d);
            let (bo, bl, bh, bc) = (bp[0], bp[1], bp[2], bp[3]);
            let e_stop = wh + eoff * p;
            // (entry, initial stop, rest of the day-1 path, label), all in long space
            let ent: Option<(f64, f64, Vec<f64>, &str)> = if ig > 0.0 && bo < wl {
                // Intraday Gap Window: opened beyond the Window bar, then closes the gap back to its extreme
                (bh >= wl).then(|| (wl, (wl - ig * p).min(bl), vec![wl, bh, bc], "Intraday Gap Window"))
            } else if kind == Kind::GapIntraday {
                None
            } else if bo >= e_stop {
                // gapped through the entry stop: the stop-limit fills at the open unless the gap negates the entry
                (bo < e_stop + neg * p && kind != Kind::Outside).then(|| (bo, wl - soff * p, vec![bo, bl, bh, bc], "Window, filled at the open"))
            } else if bh >= e_stop {
                let outside = bl < wl;
                if kind == Kind::Outside && !outside {
                    None
                } else {
                    let label = if outside { "Outside Window (stop beyond today's extreme)" } else { "Window" };
                    Some((e_stop, wl.min(bl) - soff * p, vec![e_stop, bh, bc], label))
                }
            } else {
                None
            };
            let Some((e, s0, day1, label)) = ent else { continue };
            let mut days = vec![day1];
            let mut below = vec![lt(d * c[e_i], d * m[e_i])];
            let mut k = e_i + 1;
            while days.len() < max_days && k <= to {
                days.push(bar_path(x, k, d));
                below.push(lt(d * c[k], d * m[k]));
                k += 1;
            }
            let (fills, open) = manage3(e, s0, &rules(i), &days, &below);
            let side = if d > 0.0 { "Long" } else { "Short" };
            done = Some(to_trade(x, d, e_i, e, &fills, open, to, "stop", &format!("{side} {label}, risk {:.2}", e - s0)));
            break;
        }
        match done {
            Some(t) => {
                i = t.exit_i.max(i + 1);
                trades.push(t);
            }
            None => i += 1,
        }
    }
    trades
}

fn weekly(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64, u: &dyn Fn(&str) -> usize) -> Vec<Trade> {
    let n = x.c.len();
    // Monday-based weeks (1970-01-01 was a Thursday)
    let wk: Vec<i64> = x.r.iter().map(|r| (day_num(&r.date) + 3).div_euclid(7)).collect();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        match spans.last_mut() {
            Some(s) if wk[s.0] == wk[i] => s.1 = i,
            _ => spans.push((i, i)),
        }
    }
    let wh: Vec<f64> = spans.iter().map(|s| x.h[s.0..=s.1].iter().cloned().fold(f64::MIN, f64::max)).collect();
    let wl: Vec<f64> = spans.iter().map(|s| x.l[s.0..=s.1].iter().cloned().fold(f64::MAX, f64::min)).collect();
    let wc: Vec<f64> = spans.iter().map(|s| x.c[s.1]).collect();
    let wv: Vec<f64> = spans.iter().map(|s| x.r[s.0..=s.1].iter().map(|r| r.volume.unwrap_or(f64::NAN)).sum()).collect();
    let wm = sma(&wc, u("window_ma"));
    let weekly_trend = g("trend_weekly") >= 0.5;
    let on_weeks = |daily: Vec<f64>| -> Vec<f64> { spans.iter().map(|s| daily[s.1]).collect() };
    let tf = Trend::new(&wh, &wl, &wc, g, u, if weekly_trend { sma(&wc, u("trend")) } else { on_weeks(sma(&x.c, u("trend"))) });
    let atr90 = avg_tr(&x.h, &x.l, &x.c, u("range_n"));
    let pt = point_fn(x, g, &atr90);
    let wvol = vol_filter(&wv, u("vol_n"), g("vol_rule") >= 0.5);
    let sec = x.series("SUBSECTOR").map(|s| {
        let ws: Vec<f64> = spans.iter().map(|sp| s[sp.1]).collect();
        let m = if weekly_trend { sma_nan(&ws, u("trend")) } else { on_weeks(sma_nan(s, u("trend"))) };
        (ws, m)
    });
    let (eoff, soff) = (g("entry_off"), g("stop_off"));
    let (p1, p2, orig, ts, tr) = (g("part1"), g("part2"), g("second_of_orig") >= 0.5, g("trail_start"), g("trail"));
    let dirs = sides(g);
    let mut trades: Vec<Trade> = Vec::new();
    let mut free_from = from; // first bar a new entry may happen
    for w in 0..spans.len().saturating_sub(1) {
        let (s, nx) = (spans[w], spans[w + 1]);
        if nx.1 < free_from || nx.0 > to {
            continue;
        }
        for &d in &dirs {
            let win = if d > 0.0 { lt(wh[w], wm[w]) } else { gt(wl[w], wm[w]) };
            if !win || !tf.ok(w, d, wc[w]) || !stock_ok(x, g, u, &atr90, wc[w], wvol[w], s.1) || !sector_ok(g, sec.as_ref(), w, d) {
                continue;
            }
            let p = pt(s.1);
            if !(p > 0.0) {
                continue;
            }
            let (hi, lo) = ((d * wh[w]).max(d * wl[w]), (d * wh[w]).min(d * wl[w]));
            let (e_stop, s0) = (hi + eoff * p, lo - soff * p);
            // the stop order works every day of the following week
            let mut fill = None;
            for k in nx.0.max(free_from)..=nx.1.min(to) {
                let bp = bar_path(x, k, d);
                if bp[0] >= e_stop {
                    fill = Some((k, bp[0], bp));
                    break;
                }
                if bp[2] >= e_stop {
                    fill = Some((k, e_stop, vec![e_stop, bp[2], bp[3]]));
                    break;
                }
            }
            let Some((k, e, day1)) = fill else { continue };
            let mut days = vec![day1];
            days.extend((k + 1..=to).map(|j| bar_path(x, j, d)));
            let (fills, open) = manage_weekly(e, s0, p, p1, p2, orig, ts, tr, &days);
            let side = if d > 0.0 { "Long" } else { "Short" };
            let t = to_trade(x, d, k, e, &fills, open, to, "stop", &format!("{side} Weekly Window, risk {:.2}", e - s0));
            free_from = t.exit_i + 1;
            trades.push(t);
            break;
        }
    }
    trades
}

fn market_timing(id: &str, x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64, u: &dyn Fn(&str) -> usize) -> Vec<Trade> {
    let vixm = id == "ws5_vix_iii_reversal";
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let nan = vec![f64::NAN; c.len()];
    let (sh, sl, sc): (&[f64], &[f64], &[f64]) = if vixm {
        (x.series("VIX.High").unwrap_or(&nan), x.series("VIX.Low").unwrap_or(&nan), x.series("VIX.Close").unwrap_or(&nan))
    } else {
        (h, l, c)
    };
    let ma = sma_nan(sc, u("ma"));
    let pct = g("pct") / 100.0;
    let atr90 = avg_tr(h, l, c, 90);
    let pt = point_fn(x, g, &atr90);
    let (stop_pts, max_hold, literal) = (g("stop_pts"), u("max_hold"), vixm && g("literal_exit") >= 0.5);
    let dirs = sides(g);
    // The watched series (price, or the VIX) is stretched UP when: non-VIX sell, VIX III buy.
    let up = |d: f64| if vixm { d > 0.0 } else { d < 0.0 };
    let signal = |i: usize, d: f64| -> bool {
        if up(d) { gt(sl[i], ma[i]) && ge(sc[i], ma[i] * (1.0 + pct)) } else { lt(sh[i], ma[i]) && le(sc[i], ma[i] * (1.0 - pct)) }
    };
    // reversion: the series trades (intraday) back through yesterday's MA
    let reverted = |k: usize, d: f64| -> bool {
        if literal && d > 0.0 {
            return gt(sh[k], ma[k - 1]);
        }
        if up(d) { lt(sl[k], ma[k - 1]) } else { gt(sh[k], ma[k - 1]) }
    };
    let mut trades = Vec::new();
    let mut i = from;
    while i < to {
        let Some(&d) = dirs.iter().find(|&&d| signal(i, d)) else {
            i += 1;
            continue;
        };
        let p = pt(i);
        if !(p > 0.0) {
            i += 1;
            continue;
        }
        let e = d * c[i];
        let stop = e - stop_pts * p;
        let mut fills: Vec<Fill> = Vec::new();
        let mut rem = 1.0;
        let mut half_k = None;
        // Fill.day is counted from the entry bar (day 1 = bar i) so to_trade maps it to bar i + day - 1
        for k in i + 1..=to {
            let bp = bar_path(x, k, d);
            let day = k - i + 1;
            if bp[0] <= stop {
                fills.push(Fill { day, px: bp[0], frac: rem, why: "stop gapped, out at the open".into() });
                rem = 0.0;
            } else if bp[1] <= stop {
                fills.push(Fill { day, px: stop, frac: rem, why: "3-point stop".into() });
                rem = 0.0;
            } else if half_k.map_or(false, |hk| k == hk + 1) {
                fills.push(Fill { day, px: bp[3], frac: rem, why: "other ½ the next day".into() });
                rem = 0.0;
            } else if half_k.is_none() && reverted(k, d) {
                fills.push(Fill { day, px: bp[3], frac: 0.5, why: "½ on reversion to the MA".into() });
                rem = 0.5;
                half_k = Some(k);
            }
            if rem > 1e-12 && k - i >= max_hold {
                fills.push(Fill { day, px: bp[3], frac: rem, why: format!("day {} time exit", k - i) });
                rem = 0.0;
            }
            if rem <= 1e-12 {
                break;
            }
        }
        let open = rem > 1e-12;
        if open {
            fills.push(Fill { day: to - i + 1, px: f64::NAN, frac: rem, why: "open".into() });
        }
        let t = to_trade(x, d, i, e, &fills, open, to, "close", if d > 0.0 { "Buy on the close" } else { "Sell on the close" });
        i = t.exit_i.max(i + 1);
        trades.push(t);
    }
    trades
}

fn emini(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64, u: &dyn Fn(&str) -> usize) -> Vec<Trade> {
    let nan = vec![f64::NAN; x.c.len()];
    let s = |k: &str| x.series(k).unwrap_or(&nan);
    let (vo, vh, vl, vc) = (s("VIX.Open"), s("VIX.High"), s("VIX.Low"), s("VIX.Close"));
    let (fp, fh, fl) = (s("VIXFILL_PX"), s("VIXFILL_HIGH"), s("VIXFILL_LOW"));
    let vm = sma_nan(vc, u("vix_ma"));
    let atr90 = avg_tr(&x.h, &x.l, &x.c, 90);
    let pt = point_fn(x, g, &atr90);
    let dirs = sides(g);
    let mut trades = Vec::new();
    for i in from.max(1)..=to {
        let Some(&d) = dirs.iter().find(|&&d| {
            if d > 0.0 { gt(vc[i - 1], vm[i - 1]) && vo[i] > vh[i - 1] && vl[i] < vh[i - 1] } else { lt(vc[i - 1], vm[i - 1]) && vo[i] < vl[i - 1] && vh[i] > vl[i - 1] }
        }) else {
            continue;
        };
        if !(fp[i].is_finite() && fh[i].is_finite() && fl[i].is_finite()) {
            continue;
        }
        let p = pt(i - 1);
        let e = d * fp[i];
        let (a, b) = (d * fh[i], d * fl[i]);
        let r = Rules { pt: p, part: g("part"), step: g("step"), trail2_start: f64::INFINITY, trail2: 0.0, d3_close: 0.0, d3_trail: 0.0, max_days: 1, ma_rule: false, day2_extra: None };
        let (fills, open) = manage3(e, e - g("risk_pts") * p, &r, &[vec![e, a.min(b), a.max(b), d * x.c[i]]], &[false]);
        trades.push(to_trade(x, d, i, e, &fills, open, to, "stop", if d > 0.0 { "Buy on the VIX gap close" } else { "Sell on the VIX gap close" }));
    }
    trades
}

// ------------------------------------------------------------------ tests: the book's worked examples

#[cfg(test)]
mod tests {
    use super::*;

    fn book_rules(pt: f64) -> Rules {
        Rules { pt, part: 3.0, step: 1.0, trail2_start: 2.0, trail2: 2.0, d3_close: 1.0, d3_trail: 1.0, max_days: 3, ma_rule: true, day2_extra: None }
    }
    fn neg(v: &[f64]) -> Vec<f64> {
        v.iter().map(|x| -x).collect()
    }
    fn near(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    /// Session 2 slides 4-16: buy 1000 at 30, stop 28.50. 333 off at 31.50, 222 at 32.50, 148 at 33.50,
    /// the final 297 stopped at 32.50: total $2,315.
    #[test]
    fn session2_illustration_totals_2315() {
        let days = vec![
            vec![30.0, 31.6, 32.2],   // Day 1: R=R 31.50 hit, closes 32.20 (no trailing on Day 1)
            vec![32.2, 32.75, 32.75], // Day 2: 32.50 hit (stop -> 30.50, then 30.75), closes 32.75
            vec![32.75, 33.5, 32.4],  // Day 3: stop 31.75 pre-open, 33.50 hit, trail 1 pt -> 32.50, stopped
        ];
        let (f, open) = manage3(30.0, 28.5, &book_rules(1.0), &days, &[false, false, false]);
        assert!(!open);
        let px: Vec<f64> = f.iter().map(|f| f.px).collect();
        assert_eq!(px, vec![31.5, 32.5, 33.5, 32.5]);
        let profit: f64 = f.iter().map(|f| 1000.0 * f.frac * (f.px - 30.0)).sum();
        assert!(near(profit, 2315.0, 1.0), "profit {profit}");
        assert!(near(f[1].frac * 1000.0, 222.2, 0.1) && near(f[2].frac * 1000.0, 148.1, 0.1) && near(f[3].frac * 1000.0, 296.3, 0.1));
    }

    /// Example #7 NOC short: 105.35, stop 109.26 (risk 3.91 > 2): Day 2 trails 2 pts off the low 101.75 -> 103.75,
    /// 1/3 at 101.44 (R=R), stop 103.44; Day 3 stop 102.99 (1 pt above the 101.99 close), 1/3 at 100.44,
    /// stop 101.44, stopped out on the same bar.
    #[test]
    fn example7_noc_short() {
        let days = vec![
            neg(&[105.35, 104.2, 106.0, 105.05]),
            neg(&[105.6, 101.75, 103.3, 101.44, 101.99]),
            neg(&[102.8, 100.44, 101.6]),
        ];
        let (f, open) = manage3(-105.35, -109.26, &book_rules(1.0), &days, &[false, false, false]);
        assert!(!open);
        let got: Vec<(usize, f64)> = f.iter().map(|f| (f.day, -f.px)).collect();
        assert_eq!(got.len(), 3, "{got:?}");
        assert!(got[0].0 == 2 && near(got[0].1, 101.44, 1e-9), "{got:?}");
        assert!(got[1].0 == 3 && near(got[1].1, 100.44, 1e-9), "{got:?}");
        assert!(got[2].0 == 3 && near(got[2].1, 101.44, 1e-9), "{got:?}");
    }

    /// Example #10 AZO short: 72.50, stop 73.51. 1/3 at 71.50 (BE), 70.50, 69.50 with the stop at 71.50 (2-pt trail);
    /// Day 3 stop 70.68 (1 pt above the close), 1/3 at 68.50, stop 69.25 (1-pt trail off the 68.25 low), rest out at the 67.80 close.
    #[test]
    fn example10_azo_short() {
        let days = vec![
            neg(&[72.5, 72.9, 71.5, 71.8]),
            neg(&[71.6, 71.9, 69.49, 69.68]),
            neg(&[69.7, 70.1, 68.25, 67.8]),
        ];
        let (f, _) = manage3(-72.5, -73.51, &book_rules(1.0), &days, &[false, false, false]);
        let px: Vec<f64> = f.iter().map(|f| -f.px).collect();
        // the book rounds the 1.01 risk to whole levels: 71.50 / 70.50 / 69.50 / 68.50
        let want = [71.49, 70.49, 69.49, 68.49, 67.8];
        assert_eq!(px.len(), want.len(), "{px:?}");
        for (a, b) in px.iter().zip(want) {
            assert!(near(*a, b, 0.011), "{px:?}");
        }
        assert_eq!(f.last().unwrap().why, "Day 3 close");
    }

    /// Example #6 CBK short: 26.02, stop 27.07; 1/3 at 24.97 (R=R), stop BE; next day gaps higher, out at the open 26.85.
    #[test]
    fn example6_cbk_gap_stop() {
        let days = vec![neg(&[26.02, 26.3, 24.9, 25.4]), neg(&[26.85, 27.2, 26.5, 26.9])];
        let (f, _) = manage3(-26.02, -27.07, &book_rules(1.0), &days, &[false, false]);
        let got: Vec<f64> = f.iter().map(|f| -f.px).collect();
        assert!(near(got[0], 24.97, 1e-9) && near(got[1], 26.85, 1e-9) && f[1].day == 2, "{got:?}");
    }

    /// Weekly Example #2 HIG short: 46.81, stop 49.14 (risk 2.33). Trail starts at 44.81 (stop 46.81), 1/4 at 44.48,
    /// trail to 44.59 off the 42.59 low, 1/4 at 42.15 (2R), stop 44.00, then 40.82 off the 38.82 low; stopped at 40.82.
    #[test]
    fn weekly_example2_hig() {
        let path = neg(&[46.81, 44.48, 42.59, 43.5, 42.15, 42.0, 43.9, 38.82, 41.0]);
        let (f, open) = manage_weekly(-46.81, -49.14, 1.0, 0.25, 0.25, true, 2.0, 2.0, &[path]);
        assert!(!open);
        let got: Vec<(f64, f64)> = f.iter().map(|f| (-f.px, f.frac)).collect();
        assert_eq!(got.len(), 3, "{got:?}");
        assert!(near(got[0].0, 44.48, 1e-9) && near(got[1].0, 42.15, 1e-9) && near(got[2].0, 40.82, 1e-9), "{got:?}");
        assert!(near(got[2].1, 0.5, 1e-12));
    }

    /// Order placement (S1 slide 63: "Buy stop 29.23, limit 30.23") and the MO negation: an open between the stop and
    /// stop + 1 fills at the open, an open 1 point or more above the stop is not taken, otherwise the fill is at the stop.
    /// Runs daily_windows end to end on synthetic bars: a steady uptrend, then a Window bar (high 29.75 below the
    /// 4-SMA 29.80) -> buy stop 29.76 (high + 0.01), limit 30.76.
    #[test]
    fn entry_stop_limit_and_negation() {
        use crate::Row;
        let mk = |next_open: f64, next_high: f64| {
            let mut rows: Vec<Row> = (0..260)
                .map(|k| {
                    let b = 4.0 + 0.1 * k as f64;
                    Row { ticker: "T".into(), date: format!("{}-{:02}-{:02}", 1990 + k / 252, 1 + (k % 252) / 21, 1 + (k % 252) % 21), open: b, high: b + 0.3, low: b - 0.3, close: b + 0.1, volume: None }
                })
                .collect();
            rows.push(Row { ticker: "T".into(), date: "1999-01-01".into(), open: 29.6, high: 29.75, low: 29.45, close: 29.5, volume: None });
            rows.push(Row { ticker: "T".into(), date: "1999-01-02".into(), open: next_open, high: next_high, low: next_open.min(29.5), close: next_high - 0.1, volume: None });
            rows
        };
        let mut p = Params::new();
        for (k, v) in [("pt_mode", 0.0), ("vix_on", 0.0), ("stoch_long", 101.0), ("side", 1.0)] {
            p.insert(k.into(), v);
        }
        for (open, high, want) in [(29.6, 30.0, Some(29.76)), (30.0, 30.3, Some(30.0)), (30.8, 31.0, None)] {
            let rows = mk(open, high);
            let x = Ctx::new(&rows, &rows, &[]);
            let n = rows.len();
            assert!(x.h[n - 2] < sma(&x.c, 4)[n - 2], "synthetic Window bar");
            let t = run("ws5_single_window", &x, n - 2, n - 1, &p).unwrap();
            match want {
                Some(e) => assert!(t.len() == 1 && near(t[0].legs[0].px, e, 1e-9), "open {open}: {:?}", t.iter().map(|t| t.legs[0].px).collect::<Vec<_>>()),
                None => assert!(t.is_empty(), "open {open} should negate"),
            }
        }
    }

    /// Weekly Example #1 CMA short: 43.40, stop 46.74 (risk 3.34). Trail starts at 41.40 (stop 43.40), 1/4 at 40.06 (R=R),
    /// trail 41.79 off the 39.79 low, then 41.40 off 39.40; the rest (3/4) stopped at 41.40.
    #[test]
    fn weekly_example1_cma() {
        let path = neg(&[43.40, 41.40, 42.0, 40.06, 39.79, 41.0, 39.40, 41.50]);
        let (f, open) = manage_weekly(-43.40, -46.74, 1.0, 0.25, 0.25, true, 2.0, 2.0, &[path]);
        assert!(!open);
        let got: Vec<(f64, f64)> = f.iter().map(|f| (-f.px, f.frac)).collect();
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(near(got[0].0, 40.06, 1e-9) && near(got[0].1, 0.25, 1e-12), "{got:?}");
        assert!(near(got[1].0, 41.40, 1e-9) && near(got[1].1, 0.75, 1e-12), "{got:?}");
    }

    /// Weekly Example #3 ZION short: 41.75, stop 43.15 (risk 1.40). 1/4 at 40.35 (R=R), stop BE; trail to 41.34
    /// (2 off the 39.34 low); the rest stopped at 41.34.
    #[test]
    fn weekly_example3_zion() {
        let path = neg(&[41.75, 40.35, 41.0, 39.34, 41.50]);
        let (f, open) = manage_weekly(-41.75, -43.15, 1.0, 0.25, 0.25, true, 2.0, 2.0, &[path]);
        assert!(!open);
        let got: Vec<(f64, f64)> = f.iter().map(|f| (-f.px, f.frac)).collect();
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(near(got[0].0, 40.35, 1e-9) && near(got[1].0, 41.34, 1e-9) && near(got[1].1, 0.75, 1e-12), "{got:?}");
    }

    /// day2_extra (discretionary Day-2-close piece): GPRO (p-046, 1 piece taken), CAKE (p-049, none), WBSN (p-065, 1 piece).
    /// Setting 1 (at most one piece taken) reproduces all three; setting 2 (none taken, the old reading) only CAKE.
    #[test]
    fn day2_extra_gpro_cake_wbsn() {
        let run = |e: f64, s: f64, days: Vec<Vec<f64>>, mx: Option<usize>| -> Vec<f64> {
            let d = if e > s { 1.0 } else { -1.0 };
            let days: Vec<Vec<f64>> = days.iter().map(|v| v.iter().map(|p| d * p).collect()).collect();
            let r = Rules { day2_extra: mx, ..book_rules(1.0) };
            manage3(d * e, d * s, &r, &days, &[false, false, false]).0.iter().map(|f| d * f.px).collect()
        };
        let gpro = || vec![vec![26.30, 27.05, 26.68], vec![26.60, 26.40, 27.85, 27.73], vec![27.60, 26.90, 28.45, 27.88]];
        let cake = || vec![vec![29.45, 29.34], vec![30.05, 30.15, 28.85, 28.91], vec![29.30, 29.40, 28.46, 28.75]];
        let wbsn = || vec![vec![23.05, 23.65, 23.60], vec![23.65, 23.40, 24.95, 24.70], vec![24.10, 23.90, 26.08, 26.00]];
        let eq = |a: Vec<f64>, b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| near(*x, *y, 1e-9));
        let g1 = run(26.30, 25.22, gpro(), Some(1));
        assert!(eq(g1.clone(), &[27.38, 27.73, 28.38, 27.88]), "GPRO {g1:?}");
        let g0 = run(26.30, 25.22, gpro(), Some(0));
        assert!(eq(g0.clone(), &[27.38, 28.38, 27.88]), "GPRO old {g0:?}");
        for mx in [Some(1), Some(0)] {
            let c = run(29.45, 30.25, cake(), mx);
            assert!(eq(c.clone(), &[28.91, 28.65, 28.75]), "CAKE {mx:?} {c:?}");
        }
        let w1 = run(23.05, 21.83, wbsn(), Some(1));
        assert!(eq(w1.clone(), &[24.27, 24.70, 25.27, 26.00]), "WBSN {w1:?}");
        let w0 = run(23.05, 21.83, wbsn(), Some(0));
        assert!(eq(w0.clone(), &[24.27, 25.27, 26.00]), "WBSN old {w0:?}");
    }

    /// Tie tolerance: 0.1 + 0.2 is 0.30000000000000004 in floats, so a raw `<` calls 0.3 "below" it; lt/gt treat it as a tie.
    #[test]
    fn sma_price_tie_tolerance() {
        let m = sma_nan(&[0.1, 0.2], 2)[1] * 2.0; // the 2-bar SMA of 0.1, 0.2, doubled
        assert!(0.3 < m && !lt(0.3, m) && !gt(m, 0.3) && ge(0.3, m) && le(m, 0.3));
        assert!(lt(0.29, m) && gt(0.31, m));
    }
}
