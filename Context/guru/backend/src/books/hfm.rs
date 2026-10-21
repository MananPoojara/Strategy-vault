//! Investment Secrets of a Hedge Fund Manager (Larry Connors & Blake E. Hayward, Probus 1995). Book id "hfm".
//! Extraction: library/hfm/extract-1.md (Ch. 1-3) + extract-2.md (Ch. 4-9). Report: library/hfm/implementation-report.md.
//!
//! Conventions (beyond connors.rs):
//!   * The book's point offsets (S&P / sector-index points of 1988-94) are expressed as % of price, default =
//!     book points / the book's price level, so one number works on SPY, QQQ and NIFTY alike.
//!   * Stop entries fill at the stop level, or at the open if the bar gaps through it.
//!   * Protective stops on later bars: a gap through fills at the open, otherwise at the stop.
//!   * Stop on the bar a stop entry filled (order inside the bar unknown on daily data) [Not from the book]: News and
//!     VIX `stop_mode` 2 (default) = TradeStation bar path (O-H-L-C if the open is nearer the high, else O-L-H-C; the
//!     stop counts only if reached after the fill), 0 = close beyond the stop (optimistic upper bound),
//!     1 = conservative (News) / any touch (VIX). Undeniables: touched = hit.
#![allow(unused_imports, dead_code)]
use crate::connors::*;
use std::collections::HashMap;
use std::sync::OnceLock;

pub const BOOK: &str = "hfm";

/// Daily rules admitted after the 2026-10-21 source review (connors/hfm-admission/SOURCE-CONTRACT.md) and an independent
/// chronological Python ledger check. Declared application: the book traded S&P 500 futures on NYSE breadth.
/// Held (not listed): HV signal (no direction, entry or exit), Undeniables (an index-option selling method), NDX-SPX
/// (LEADER_CLOSE series not loaded), News Reversals and the VIX program (gate withheld), Globex and Bond (data).
pub const REVIEWED_DAILY: &[&str] = &["hfm_chadtp", "hfm_chadtp_nse"];
pub fn reviewed_daily(id: &str) -> bool { REVIEWED_DAILY.contains(&id) }
/// CHADTP on NYSE breadth: actual SPX and SPY; the NSE-breadth version: NIFTY 50.
pub fn admitted_symbol(id: &str, ticker: &str) -> bool {
    match id {
        "hfm_chadtp" => matches!(ticker, "SPX" | "SPY"),
        "hfm_chadtp_nse" => ticker == "NIFTY 50",
        _ => false,
    }
}

const NEWS_RULES: &[&str] = &[
    "F|Rule #1: wait for an extremely bullish or bearish event after the close (discretionary; coded as the book's \"aggressive traders will take all news stories\": every qualifying gap)",
    "E|Rule #2: the market must gap open above or below the previous day's high or low",
    "E|Rule #3: on openings that gap higher, place a sell stop one tick below the previous day's high; on openings that gap lower, place a buy stop one tick above the previous day's low (gap day only)",
    "X|Rule #4: after you are filled, stops should be placed at the morning's opening price",
    "X|[Not from the book] Whether the opening-price stop traded after the fill on the entry day is decided by the TradeStation bar path (O-H-L-C if the open is nearer the high, else O-L-H-C); stop_mode 0 = only if the close is beyond the open (optimistic upper bound), 1 = conservative (the extreme went more than one tick beyond the open and the close is no better than the fill)",
    "X|Exit on the close of the entry day (the book values its examples as a \"day trading profit\"); hold = N keeps the opening-price stop for N days",
];

const UND_RULES: &[&str] = &[
    "F|1. Take the weekly chart of a stock index group (weekly bars built from daily bars, Monday-Friday)",
    "E|2. Identify an intraweek price that has made a six-week high or six-week low",
    "E|3. For weeks that made lows, the close of that week must be above the open; 4. for weeks that made highs, the close must be below the open",
    "E|5. For lows, a buy signal is triggered the next week if the index trades .25 points above the previous week's high; 6. for highs, a sell signal the next week if it trades .25 points below the previous week's low",
    "E|9./10. If the following week does not trade beyond the reversal week's high (low), there is no signal",
    "X|7. Stops on buys are one tick below the reversal week low; 8. stops on sells are one tick above the reversal week high",
    "X|Hold for the signal week and three additional weeks (exit on the close of the 4th week), unless stopped out",
];

const CHADTP_RULES: &[&str] = &[
    "F|CHADTP = (sum of the past five days' advancing issues - sum of the past five days' declining issues) / 5, calculated after the close",
    "E|CHADTP +400 or more (overbought): sell only after the S&P trades .10 points below the previous day's low",
    "E|CHADTP -400 or less (oversold): buy only after tomorrow's S&P trades .10 points above today's high",
    "E|While the reading stays beyond the threshold, the trigger is re-referenced to each latest day's low/high (\"and so on\")",
    "X|Exit on the close of the sixth trading day (entry day = day 1)",
];

const NDX_RULES: &[&str] = &[
    "F|1. Take today's closing change of the NASDAQ 100 (series LEADER_CLOSE) and the S&P 500 (this ticker)",
    "E|2. If the NASDAQ 100 performance is stronger than the S&P 500, buy on the opening the next day",
    "E|3. The day after the S&P 500 is stronger than the NASDAQ 100, sell (short) on the opening",
    "X|Stop and reverse: always in the market; a tie keeps the current position",
];

const VIX_RULES: &[&str] = &[
    "F|1. The VIX must close above 15 or below 11 for a signal; readings in between are ignored",
    "E|2. After a reading above 15 the next opening must not gap higher; after a reading below 11 it must not gap lower",
    "E|3. The day after a VIX close of 15 or more, a buy stop .50 points above the previous day's high; after 11 or less, a sell stop .50 points below the previous day's low",
    "E|4. If the day after the signal is an inside day, the signal is carried into the next trading day",
    "X|5. A stop 2.00 points below the fill on buys, 2.00 points above on sells",
    "X|[Not from the book] On the fill day the stop is decided by the TradeStation bar path (O-H-L-C if the open is nearer the high, else O-L-H-C; counts only after the fill); stop_mode 0 = only if the close is beyond the stop (optimistic upper bound), 1 = any touch (pessimistic bound)",
    "X|6. Use a trailing stop to lock in profits or cover market on close (coded: market on close)",
];

const GLOBEX_RULES: &[&str] = &[
    "F|1. Find the fair value difference of the S&P futures to the S&P cash market",
    "E|2. At 4:03 P.M. (EST) take the closing cash value of the S&P 500 and add the fair value (rounded to the futures tick)",
    "E|3./4. Add .50 points and place a sell order on the S&P 500 futures at that price (\"We do not trade this method from the long side\")",
    "X|5. If filled, cover at fair value (the fill minus .50) during Globex trading",
    "X|6. Close the position before the 8:30 A.M. (EST) economic news the next morning",
];

const BOND_RULES: &[&str] = &[
    "F|1. There must be a major economic report tomorrow morning (unemployment, C.P.I., P.P.I., trade deficit)",
    "E|2. If bonds trade within three ticks of the day-session lows in the final five minutes, sell the bonds M.O.C.",
    "X|3. Cover in the evening session at least five ticks lower or M.O.C.; stops five ticks above the day-session fill",
];

const HV_RULES: &[&str] = &[
    "F|Signal: the 10-day historical volatility reaches half or less of its 100-day historical volatility (\"a major move for that market is nearing\")",
    "F|Counted once per episode: the first day the ratio drops to 0.5 or below",
    "E|The book gives no direction (\"a personal choice\": straddles, trend-line breaks, breakouts). Coded with the breakout choice: buy stop at the signal day's high / sell stop at its low, whichever trades first within N days",
    "X|Exit on the close N days after entry (the book's moves play out over 2 days to 6 weeks; 20 days by default). The book gives no stop",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    const G2: &str = "Ch. 2 · News Reversals";
    const G3: &str = "Ch. 3 · Historical Volatility";
    const G4: &str = "Ch. 4 · Undeniables";
    const G5: &str = "Ch. 5 · CHADTP";
    const G6: &str = "Ch. 6 · New Markets, New Indicators";
    const NEWS_NOTE: &str = "Book: futures and stocks, Oct-Nov 1994 examples; run here on the indices/ETFs (vault convention). The news filter is discretionary, so every gap beyond the prior high/low is taken (the book's \"aggressive trader\"). [Not from the book] Entry offset = one instrument tick (tick = 0: 0.05 for NSE indices, 0.01 otherwise; off > 0 switches back to the old % of price). [Not from the book] The opening-price stop needs intraday data; default stop_mode 2 = TradeStation bar path. Checked on real NIFTY 50 1-minute bars 2008-2024 (706 day trades): true 439 stop-outs, win% 29.7 / avg +4.0 pts; bar path 271 stop-outs, 47.7% / +16.9 (agrees on 72% of trades); close-only 234, 50.6% / +17.9. So even the bar path stays optimistic on NIFTY (it misses about 4 in 10 real stop-outs); read the result as an upper bound. stop_mode 0 = close beyond the open (old default, optimistic upper bound); 1 = conservative variant (stopped when the extreme went more than one tick beyond the open and the close is no better than the fill: 347 stop-outs, +14.5 pts; not a strict bound). Plain any-touch is not offered: the open is always inside the bar, so every trade would be stopped.";
    const UND_NOTE: &str = "Book trades sector-index options (sells OTM puts on buy signals, calls on sell signals) on XOI/XAU-type indices; here the underlying signal is traded on the index/ETF. The book's 72% counts option-selling wins, so it is not comparable with the underlying win rate shown here; the comparable number is the \"not stopped within 4 weeks\" rate (trades not closed by the stop): SPY 1993-2026 57.6% (106 of 184), NIFTY 50 55.2% (91 of 165) (verify_hfm.py). .25 points ≈ 0.10% of the 1990-94 XOI (≈240); one tick = 0.01 point ≈ 0.004%. If the stop and the entry fall in the same daily bar, the stop counts as hit (conservative). Long & Short reverses on an opposite signal (book, Feb 1992) and allows re-entry after a stop.";
    const CH_NOTE: &str = "NYSE_ADV / NYSE_DECL = NYSE advancing / declining issues per day. Indicator checked against Unicorn NYSE breadth: Feb 2 1994 +400.6, Oct 10 1990 -403.6, Jan 2 1990 +405.8, Sep 23 1994 -608.6 match the book exactly. .10 S&P point ≈ 0.03% (S&P 300-480 in 1988-94). One position at a time. The book traded S&P futures; run here on SPY / the index. Coverage: the Unicorn NYSE file ends 2020-02-10 (the site stopped), so trades end there but the stats (years, CAGR) still span the whole run; pick to = 2020-02-10 for comparable CAGR.";
    const VIX_NOTE: &str = "The book's VIX is the old OEX VIX (VXO; 1993-94 mostly 11-12.5). Coded on the vault's VIX (CBOE VIX for US data, India VIX for India); the thresholds are parameters and need recalibrating for the modern VIX / India VIX. use_vxo = 1 reads series VXO (book reproduction). .50 point ≈ 0.11% and 2.00 points ≈ 0.43% of the S&P at 465. [Not from the book] The 2-point stop on the fill day follows the TradeStation bar path by default (stop_mode 2). On real NIFTY 1-minute bars 2015-2024 (192 trades): true win% 45.3 / avg +3.4 pts, bar path 45.3% / +6.9, close-only 49.0% / +10.5; stop_mode 0 (close beyond the stop, the old default) is an optimistic upper bound, 1 (any touch) a pessimistic bound. Day trade, out on the close; the trailing stop is not modelled. An inside-day carry re-references the stop to the inside day's high/low and repeats the gap check.";
    let news_claim = || claim("-", "≈70%", "-", "day trade", "futures research summary (no count given)");
    vec![
        d("hfm_news", "News Reversals · Long & Short", G2, "Long/Short", Live, NEWS_RULES, NEWS_NOTE, news_claim()),
        d("hfm_news_long", "News Reversals · Long (gap down)", G2, "Long", Live, NEWS_RULES, NEWS_NOTE, news_claim()),
        d("hfm_news_short", "News Reversals · Short (gap up)", G2, "Short", Live, NEWS_RULES, NEWS_NOTE, news_claim()),
        d("hfm_hv", "Connors-Hayward Historical Volatility · breakout", G3, "Long/Short", Live, HV_RULES,
          "The book's signal is a volatility-expansion warning without direction, entry or exit; the breakout entry and the 20-day exit are a choice the book allows, not the book's rules. HV = annualised stdev of daily log returns (population by default, TradeStation StdDev; the annualisation cancels in the ratio). Book examples: Nikkei, DJIA 1987, soybeans, yen, XAU (22 signals, 1 called false), no statistics.",
          None),
        d("hfm_und", "Undeniables · Long & Short", G4, "Long/Short", Live, UND_RULES, UND_NOTE, claim("18", "72% (option-selling wins; compare the not-stopped rate in the notes)", "-", "4 weeks", "XOI weekly 11/24/89 - 11/4/94 (option selling)")),
        d("hfm_und_long", "Undeniables · Buy reversal", G4, "Long", Live, UND_RULES, UND_NOTE, None),
        d("hfm_und_short", "Undeniables · Sell reversal", G4, "Short", Live, UND_RULES, UND_NOTE, None),
        StrategyDef { needs_series: &["NYSE_ADV", "NYSE_DECL"], ..d("hfm_chadtp", "CHADTP · NYSE breadth", G5, "Long/Short", Live, CHADTP_RULES,
          CH_NOTE, claim("-", "74%", "-", "6 days", "S&P futures 1/1/88 - 11/29/94")) },
        StrategyDef { needs_series: &["NSE_ADV", "NSE_DECL"], ..d("hfm_chadtp_nse", "CHADTP · NSE breadth (India)", G5, "Long/Short", Live, CHADTP_RULES,
          "India mapping: NSE_ADV / NSE_DECL = NSE advancing / declining issues per day (NSE market-activity report; the stock universe of the file is under review). NSE issues traded (adv + dec) rose from about 1,500 a day in 2012 to about 3,500 in 2026, so a fixed ±400 changes meaning over time. Default scale = 0: fixed ±400 issues, as the book writes it (the NYSE version). [Not from the book] scale = 1: the threshold is a share of the issues traded, |CHADTP| ≥ thr / base × average (adv + dec) over the same window, with base = 1,900 (the NYSE's average adv + dec in the book's 1988-94 period), so the book's ±400 = ±21% of issues traded.",
          None) },
        StrategyDef { needs_series: &["LEADER_CLOSE"], ..d("hfm_ndxspx", "NDX-SPX stop and reverse", G6, "Long/Short", Live, NDX_RULES,
          "LEADER_CLOSE = daily close of the leader index, per ticker: SPY / SPX -> NASDAQ 100 (FRED NASDAQ100), NIFTY 50 -> NIFTY IT. The book compares point changes (\"if the NASDAQ is up 3.61 points today, and the S&P 500 cash market is up 2.41, you will buy\"); default pct = 0 is that book reading. Caveat: points compare like with like only for SPX vs NDX (similar levels in 1991-94); on SPY vs NDX (SPY points are ~1/10 of SPX) or NIFTY 50 vs NIFTY IT the signal becomes mostly \"which index is higher-priced\". [Not from the book] pct = 1 compares % changes, which any other pair needs. Book check (SPX cash 1991-94, pct 0): 442 trades vs the book's 440 and the 9/26/94 trade sheet match; the P/L is not comparable (no SPX opens then, so entries at the prior close inflate P/L by about 50%). The book traded S&P futures at the open; P/L here is on the ticker. The book's Dow table applies the same signal to the DJIA.",
          claim("440", "56%", "$303.92", "2 bars", "S&P futures 1/2/91 - 11/30/94")) },
        StrategyDef { needs_vix: true, ..d("hfm_vix", "VIX day-trading program", G6, "Long/Short", Live, VIX_RULES, VIX_NOTE,
          claim("24", "58.3% (14/10)", "$453 ($10,875 total, after $50 costs)", "day trade", "S&P futures 1993 - 11/20/94")) },
        StrategyDef { needs_series: &["FV_PREMIUM", "FUT_ON_HIGH", "FUT_ON_LOW_AFTER_HIGH", "FUT_0830"], ..d("hfm_globex_fv", "Globex S&P Fair Value Program", G6, "Short", NeedsData("index futures overnight series FV_PREMIUM / FUT_ON_HIGH / FUT_ON_LOW_AFTER_HIGH / FUT_0830 (none in the vault)"), GLOBEX_RULES,
          "Intraday rule coded against per-day series of the index futures (index points, one row per trading day t, ticker = the cash index): FV_PREMIUM = fair value of the futures over cash; FUT_ON_HIGH = highest futures trade from 16:03 ET day t to 08:30 ET day t+1; FUT_ON_LOW_AFTER_HIGH = lowest futures trade after that high and before 08:30 ET t+1 (conservative: the real fill can come before the high); FUT_0830 = last futures trade before 08:30 ET t+1. India: NIFTY futures vs spot, overnight leg on GIFT Nifty. The MOC-sell variant (futures far above fair value + .50) is not modelled.",
          None) },
        d("hfm_bond_evening", "Bond Market Evening Session Price Persistency", G6, "Short", NotTestable("needs 30-year T-bond futures day + evening session intraday data and an economic calendar; no Indian equivalent"), BOND_RULES,
          "30-year Treasury bond futures only; the vault's universe is equity indices/ETFs and India has no liquid evening session for government-bond futures. The book gives no statistics (\"a few trades per month\").", None),
    ]
}

/// Tunable numbers of each strategy (book value = default). None = not a strategy of this book.
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "hfm_news" | "hfm_news_long" | "hfm_news_short" => vec![
            p("tick", "Entry: points beyond prior high/low (1 tick; 0 = instrument tick)", 0.0, 0.0, 5.0, 0.01),
            p("off", "Entry: % of price instead of tick (0 = use tick; old default 0.01)", 0.0, 0.0, 0.5, 0.01),
            p("hold", "Exit on the close of day N (1 = same day)", 1.0, 1.0, 10.0, 1.0),
            p("stop_mode", "Entry-day stop: 2 = bar path O-H-L-C (default), 0 = close only (optimistic), 1 = conservative", 2.0, 0.0, 2.0, 1.0),
        ],
        "hfm_hv" => vec![
            p("n_short", "Short HV period", 10.0, 5.0, 30.0, 1.0),
            p("n_long", "Long HV period", 100.0, 50.0, 200.0, 10.0),
            p("ratio", "Signal: short HV / long HV at or below", 0.5, 0.3, 0.7, 0.05),
            p("ddof", "Stdev: 0 = population, 1 = sample", 0.0, 0.0, 1.0, 1.0),
            p("arm", "Breakout window after signal, days (not in book)", 5.0, 1.0, 20.0, 1.0),
            p("hold", "Exit N days after entry (not in book)", 20.0, 5.0, 40.0, 5.0),
        ],
        "hfm_und" | "hfm_und_long" | "hfm_und_short" => vec![
            p("weeks", "N-week high/low (incl. the reversal week)", 6.0, 4.0, 12.0, 1.0),
            p("off", "Trigger: % beyond reversal week high/low (.25 pt)", 0.10, 0.0, 0.5, 0.05),
            p("tick", "Stop: % beyond reversal week low/high (1 tick)", 0.004, 0.0, 0.5, 0.01),
            p("hold_w", "Hold N weeks incl. the entry week", 4.0, 1.0, 8.0, 1.0),
        ],
        "hfm_chadtp" | "hfm_chadtp_nse" => vec![
            p("thr", "CHADTP threshold (±)", 400.0, 200.0, 800.0, 50.0),
            p("n", "Days summed", 5.0, 3.0, 10.0, 1.0),
            p("off", "Trigger: % beyond prior low/high (.10 pt)", 0.03, 0.0, 0.2, 0.01),
            p("hold", "Exit on the close of day N (entry = day 1)", 6.0, 3.0, 10.0, 1.0),
            p("scale", "Threshold: 0 = fixed issues (book), 1 = scaled to issues traded (thr/base × adv+dec) [Not from the book]", 0.0, 0.0, 1.0, 1.0),
            p("base", "Issues traded the book's thr refers to (NYSE 1988-94 ≈ 1,900)", 1900.0, 1000.0, 4000.0, 100.0),
        ],
        "hfm_ndxspx" => vec![p("pct", "Compare: 0 = point change (book), 1 = % change [Not from the book]", 0.0, 0.0, 1.0, 1.0)],
        "hfm_vix" => vec![
            p("hi", "VIX close at or above (buy setup)", 15.0, 10.0, 40.0, 1.0),
            p("lo", "VIX close at or below (sell setup)", 11.0, 8.0, 25.0, 0.5),
            p("off", "Entry: % beyond prior high/low (.50 pt)", 0.11, 0.0, 0.5, 0.01),
            p("stop", "Stop: % from fill (2.00 pts)", 0.43, 0.1, 2.0, 0.05),
            p("gap_mode", "No-gap test vs: 0 = prior high/low, 1 = prior close", 0.0, 0.0, 1.0, 1.0),
            p("stop_mode", "Entry-day stop: 2 = bar path O-H-L-C (default), 0 = close only (optimistic), 1 = any touch (pessimistic)", 2.0, 0.0, 2.0, 1.0),
            p("use_vxo", "VIX source: 0 = vault VIX, 1 = series VXO", 0.0, 0.0, 1.0, 1.0),
        ],
        "hfm_globex_fv" => vec![
            p("off", "Sell: points above cash + fair value", 0.5, 0.0, 2.0, 0.05),
            p("cover", "Cover: points below the fill", 0.5, 0.1, 2.0, 0.05),
            p("tick", "Futures tick (fair value rounding)", 0.05, 0.05, 0.25, 0.05),
        ],
        _ => return None,
    })
}

// ---------------------------------------------------------------- helpers

fn mk(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: Vec::new() }
}

fn shut(mut t: Trade, i: usize, px: f64, why: String) -> Trade {
    t.exit_i = i;
    t.exit_px = px;
    t.signal_i = i;
    t.reason = why;
    t
}

fn mark_open(t: Trade, x: &Ctx, to: usize) -> Trade {
    Trade { open: true, ..shut(t, to, x.c[to], "open".into()) }
}

/// Stop entry at `lvl` on bar k: fills at the level, or at the open when the bar gaps through it.
fn stop_fill(x: &Ctx, k: usize, dir: f64, lvl: f64) -> Option<f64> {
    if dir > 0.0 { (x.h[k] >= lvl).then(|| x.o[k].max(lvl)) } else { (x.l[k] <= lvl).then(|| x.o[k].min(lvl)) }
}

/// Protective stop on a bar after the entry bar: a gap through fills at the open, otherwise at the stop.
fn stop_hit(x: &Ctx, k: usize, dir: f64, stop: f64) -> Option<f64> {
    let ext = if dir > 0.0 { x.l[k] } else { x.h[k] };
    if dir * (x.o[k] - stop) <= 0.0 {
        Some(x.o[k])
    } else if dir * (ext - stop) <= 0.0 {
        Some(stop)
    } else {
        None
    }
}

/// Stop on the bar a stop entry filled: conservative = touched; otherwise only if the close is beyond it.
fn entry_bar_stop(x: &Ctx, k: usize, dir: f64, stop: f64, conservative: bool) -> bool {
    let px = if !conservative { x.c[k] } else if dir > 0.0 { x.l[k] } else { x.h[k] };
    dir * (px - stop) <= 0.0
}

/// TradeStation bar path [Not from the book]: O-H-L-C if the open is nearer the high, else O-L-H-C. The entry stop at
/// `lvl` fills on the first leg that reaches it (or at the open); the protective stop counts only if a later vertex
/// of the path reaches it.
fn bar_path_stop(x: &Ctx, k: usize, dir: f64, lvl: f64, stop: f64) -> bool {
    let (o, h, l, c) = (x.o[k], x.h[k], x.l[k], x.c[k]);
    let v = if h - o <= o - l { [o, h, l, c] } else { [o, l, h, c] };
    let j = if dir * (o - lvl) >= 0.0 { Some(1) } else { (1..4).find(|&j| dir * (v[j] - lvl) >= 0.0) };
    j.map_or(false, |j| v[j..].iter().any(|p| dir * (p - stop) <= 0.0))
}

/// Daily VIX closes by date, from the VIX directory the server was started with (argument 3, same default as main).
/// ponytail: re-reads the VIX files once per process because `VixAligned` keeps its closes private;
/// drop this when connors.rs exposes the aligned VIX close.
fn vix_closes() -> &'static HashMap<String, f64> {
    static V: OnceLock<HashMap<String, f64>> = OnceLock::new();
    V.get_or_init(|| {
        let dir = std::env::args().nth(3).unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../../../india_vix").into());
        crate::load_vix_dir(&dir).into_iter().map(|r| (r.date, r.close)).collect()
    })
}

fn dirs_of(id: &str) -> &'static [f64] {
    if id.ends_with("_long") {
        &[1.0]
    } else if id.ends_with("_short") {
        &[-1.0]
    } else {
        &[1.0, -1.0]
    }
}

// ---------------------------------------------------------------- strategies

/// None = not a strategy of this book.
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let pct = |k: &str| g(k) / 100.0;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let from = from.max(1); // every rule looks at the prior bar
    let mut out: Vec<Trade> = Vec::new();
    if from > to {
        return Some(out);
    }
    match id {
        "hfm_news" | "hfm_news_long" | "hfm_news_short" => {
            let (off, hold, mode) = (pct("off"), u("hold"), g("stop_mode").round());
            let tick = if g("tick") > 0.0 { g("tick") } else if x.r[0].ticker.to_uppercase().contains("NIFTY") { 0.05 } else { 0.01 };
            let dirs = dirs_of(id);
            let mut i = from;
            while i <= to {
                // long: gap below the prior low, buy stop one tick above it; short: mirror
                let side = if o[i] < l[i - 1] { 1.0 } else if o[i] > h[i - 1] { -1.0 } else { 0.0 };
                let ref_ = if side > 0.0 { l[i - 1] } else { h[i - 1] };
                let lvl = ref_ + side * if off > 0.0 { ref_ * off } else { tick };
                let Some(px) = (side != 0.0 && dirs.contains(&side)).then(|| stop_fill(x, i, side, lvl)).flatten() else {
                    i += 1;
                    continue;
                };
                let (t, stop, last) = (mk(side, i, px, "stop"), o[i], i + hold - 1);
                let stopped = match mode as i32 {
                    0 => entry_bar_stop(x, i, side, stop, false),
                    // the extreme went more than one tick beyond the open and the close is no better than the fill
                    1 => side * ((if side > 0.0 { l[i] } else { h[i] }) - (stop - side * tick)) < 0.0 && side * (c[i] - px) <= 0.0,
                    _ => bar_path_stop(x, i, side, lvl, stop),
                };
                let done = if stopped {
                    shut(t, i, stop, "Stopped at the day's open".into())
                } else if last == i {
                    shut(t, i, c[i], "Day trade: out on the close".into())
                } else {
                    let mut r = None;
                    for k in i + 1..=last.min(to) {
                        if let Some(sp) = stop_hit(x, k, side, stop) {
                            r = Some((k, sp, "Stopped at the entry day's open".to_string()));
                            break;
                        }
                        if k == last {
                            r = Some((k, c[k], format!("Close of day {hold}")));
                        }
                    }
                    match r {
                        Some((k, px, why)) => shut(t, k, px, why),
                        None => mark_open(t, x, to),
                    }
                };
                i = done.exit_i + 1;
                out.push(done);
            }
        }
        "hfm_hv" => {
            let (ns, nl, thr, ddof) = (u("n_short"), u("n_long"), g("ratio"), g("ddof").round());
            let (arm, hold) = (u("arm"), u("hold"));
            let (a, b) = (hv_ddof(c, ns, ddof), hv_ddof(c, nl, ddof));
            let ratio: Vec<f64> = a.iter().zip(&b).map(|(a, b)| a / b).collect();
            let mut i = from;
            while i <= to {
                if !(ratio[i] <= thr && !(ratio[i - 1] <= thr)) {
                    i += 1;
                    continue;
                }
                let s = i;
                let mut entered = None;
                for k in s + 1..=(s + arm).min(to) {
                    let (up, dn) = (stop_fill(x, k, 1.0, h[s]), stop_fill(x, k, -1.0, l[s]));
                    // both sides trade on one bar: take the side nearer the open (likely first)
                    entered = match (up, dn) {
                        (Some(u), Some(d)) => Some(if u - o[k] <= o[k] - d { (k, 1.0, u) } else { (k, -1.0, d) }),
                        (Some(u), None) => Some((k, 1.0, u)),
                        (None, Some(d)) => Some((k, -1.0, d)),
                        _ => None,
                    };
                    if entered.is_some() {
                        break;
                    }
                }
                let Some((k, dir, px)) = entered else {
                    i = s + 1;
                    continue;
                };
                let (t, e) = (mk(dir, k, px, "stop"), k + hold);
                let done = if e <= to { shut(t, e, c[e], format!("{hold} days after entry")) } else { mark_open(t, x, to) };
                i = done.exit_i + 1;
                out.push(done);
            }
        }
        "hfm_und" | "hfm_und_long" | "hfm_und_short" => {
            let (n, off, tick, hold_w) = (u("weeks"), pct("off"), pct("tick"), u("hold_w"));
            let dirs = dirs_of(id);
            // weekly bars (Monday-Friday) over all data, so the N-week extremes have history
            let mut wk: Vec<[usize; 2]> = Vec::new(); // [first bar, last bar]
            let mut widx = vec![0usize; c.len()];
            let mut prev = i64::MIN;
            for (i, r) in x.r.iter().enumerate() {
                let w = (crate::day_number(&r.date) + 3).div_euclid(7); // 1970-01-01 was a Thursday
                if w != prev {
                    wk.push([i, i]);
                    prev = w;
                }
                wk.last_mut().unwrap()[1] = i;
                widx[i] = wk.len() - 1;
            }
            let wh: Vec<f64> = wk.iter().map(|w| h[w[0]..=w[1]].iter().cloned().fold(f64::MIN, f64::max)).collect();
            let wl: Vec<f64> = wk.iter().map(|w| l[w[0]..=w[1]].iter().cloned().fold(f64::MAX, f64::min)).collect();
            // +1 buy reversal (N-week low, close > open), -1 sell reversal (N-week high, close < open)
            let setup: Vec<f64> = (0..wk.len())
                .map(|w| {
                    if w + 1 < n {
                        return 0.0;
                    }
                    let (wo, wc) = (o[wk[w][0]], c[wk[w][1]]);
                    if (w + 1 - n..=w).all(|k| wl[w] <= wl[k]) && wc > wo {
                        1.0
                    } else if (w + 1 - n..=w).all(|k| wh[w] >= wh[k]) && wc < wo {
                        -1.0
                    } else {
                        0.0
                    }
                })
                .collect();
            let trig = |w: usize, d: f64| if d > 0.0 { wh[w] * (1.0 + off) } else { wl[w] * (1.0 - off) };
            let stop_of = |w: usize, d: f64| if d > 0.0 { wl[w] * (1.0 - tick) } else { wh[w] * (1.0 + tick) };
            let mut cur: Option<(Trade, f64, usize)> = None; // trade, stop, last bar
            let mut used = usize::MAX; // reversal week already traded
            for i in from..=to {
                let w = widx[i];
                let (pw, sig) = if w > 0 { (w - 1, setup[w - 1]) } else { (usize::MAX, 0.0) };
                let fill = (sig != 0.0 && pw != used && dirs.contains(&sig)).then(|| stop_fill(x, i, sig, trig(pw, sig))).flatten();
                if let Some((t, stop, last)) = cur.take() {
                    let dir = t.dir;
                    if let Some(sp) = stop_hit(x, i, dir, stop) {
                        out.push(shut(t, i, sp, "Stopped beyond the reversal week".into()));
                    } else if let (Some(px), true) = (fill, sig == -dir) {
                        out.push(shut(t, i, px, "Opposite signal: reverse".into()));
                    } else {
                        if i == last {
                            out.push(shut(t, i, c[i], format!("Close of week {hold_w}")));
                        } else {
                            cur = Some((t, stop, last));
                        }
                        continue;
                    }
                }
                let Some(px) = fill else { continue };
                used = pw;
                let (t, stop) = (mk(sig, i, px, "stop"), stop_of(pw, sig));
                let last = wk.get(w + hold_w - 1).map_or(usize::MAX, |b| b[1]);
                if entry_bar_stop(x, i, sig, stop, true) {
                    out.push(shut(t, i, stop, "Stopped on the entry day".into()));
                } else if last == i {
                    out.push(shut(t, i, c[i], format!("Close of week {hold_w}")));
                } else {
                    cur = Some((t, stop, last));
                }
            }
            if let Some((t, _, _)) = cur {
                out.push(mark_open(t, x, to));
            }
        }
        "hfm_chadtp" | "hfm_chadtp_nse" => {
            let (a, dc) = if id == "hfm_chadtp" { ("NYSE_ADV", "NYSE_DECL") } else { ("NSE_ADV", "NSE_DECL") };
            let (Some(adv), Some(dec)) = (x.series(a), x.series(dc)) else { return Some(out) };
            let (thr, n, off, hold) = (g("thr"), u("n"), pct("off"), u("hold"));
            // (sum of n days' advances - sum of n days' declines) / n; NaN if any day in the window is missing
            let ch: Vec<f64> = (0..c.len()).map(|i| if i + 1 < n { f64::NAN } else { (i + 1 - n..=i).map(|k| adv[k] - dec[k]).sum::<f64>() / n as f64 }).collect();
            // [Not from the book] scale = 1: threshold as a share of issues traded, thr/base × mean(adv + dec) of the window
            let scaled = g("scale") >= 0.5;
            let base = g("base");
            let thr_at = |i: usize| if scaled && i + 1 >= n { thr / base * (i + 1 - n..=i).map(|k| adv[k] + dec[k]).sum::<f64>() / n as f64 } else { thr };
            let mut i = from;
            while i <= to {
                let t = thr_at(i - 1);
                let side = if ch[i - 1] >= t { -1.0 } else if ch[i - 1] <= -t { 1.0 } else { 0.0 };
                let lvl = if side > 0.0 { h[i - 1] * (1.0 + off) } else { l[i - 1] * (1.0 - off) };
                let Some(px) = (side != 0.0).then(|| stop_fill(x, i, side, lvl)).flatten() else {
                    i += 1;
                    continue;
                };
                let (t, e) = (mk(side, i, px, "stop"), i + hold - 1);
                let done = if e <= to { shut(t, e, c[e], format!("Close of day {hold}")) } else { mark_open(t, x, to) };
                i = done.exit_i + 1;
                out.push(done);
            }
        }
        "hfm_ndxspx" => {
            let Some(lc) = x.series("LEADER_CLOSE") else { return Some(out) };
            let pc = g("pct") >= 0.5;
            let chg = |v: &[f64], i: usize| if pc { v[i] / v[i - 1] - 1.0 } else { v[i] - v[i - 1] };
            let mut want = 0.0; // side wanted from the latest decisive close (NaN / tie keeps it)
            let mut cur: Option<Trade> = None;
            for i in from..=to {
                // act at today's open on yesterday's signal
                if want != 0.0 && cur.as_ref().map_or(true, |t| t.dir != want) {
                    if let Some(t) = cur.take() {
                        out.push(shut(t, i, o[i], "Reverse on the open".into()));
                    }
                    cur = Some(mk(want, i, o[i], "open"));
                }
                let (dl, ds) = (chg(lc, i), chg(c, i));
                if dl > ds {
                    want = 1.0;
                } else if dl < ds {
                    want = -1.0;
                }
            }
            if let Some(t) = cur {
                out.push(mark_open(t, x, to));
            }
        }
        "hfm_vix" => {
            let (hi, lo, off, stp) = (g("hi"), g("lo"), pct("off"), pct("stop"));
            let (gap_close, mode) = (g("gap_mode") >= 0.5, g("stop_mode").round());
            let vx: Vec<f64> = match (g("use_vxo") >= 0.5).then(|| x.series("VXO")).flatten() {
                Some(s) => s.to_vec(),
                None => {
                    let m = vix_closes();
                    x.r.iter().map(|r| m.get(&r.date).copied().unwrap_or(f64::NAN)).collect()
                }
            };
            let mut carry = 0.0; // side carried into this bar by an inside day
            for i in from..=to {
                let side = if vx[i - 1] >= hi { 1.0 } else if vx[i - 1] <= lo { -1.0 } else { carry };
                carry = 0.0;
                if side == 0.0 {
                    continue;
                }
                let refp = if gap_close { c[i - 1] } else if side > 0.0 { h[i - 1] } else { l[i - 1] };
                if side * (o[i] - refp) > 0.0 {
                    continue; // gapped in the signal's direction: no trade
                }
                let lvl = if side > 0.0 { h[i - 1] * (1.0 + off) } else { l[i - 1] * (1.0 - off) };
                match stop_fill(x, i, side, lvl) {
                    Some(px) => {
                        let (t, stop) = (mk(side, i, px, "stop"), px * (1.0 - side * stp));
                        let stopped = if mode >= 1.5 { bar_path_stop(x, i, side, lvl, stop) } else { entry_bar_stop(x, i, side, stop, mode >= 0.5) };
                        out.push(if stopped { shut(t, i, stop, "Stopped".into()) } else { shut(t, i, c[i], "Market on close".into()) });
                    }
                    None if h[i] < h[i - 1] && l[i] > l[i - 1] => carry = side,
                    None => {}
                }
            }
        }
        "hfm_globex_fv" => {
            let (Some(fv), Some(hi), Some(lo), Some(last)) = (x.series("FV_PREMIUM"), x.series("FUT_ON_HIGH"), x.series("FUT_ON_LOW_AFTER_HIGH"), x.series("FUT_0830")) else {
                return Some(out);
            };
            let (off, cover, tick) = (g("off"), g("cover"), g("tick"));
            for i in from..to {
                let lvl = ((c[i] + fv[i]) / tick).round() * tick + off;
                if !(hi[i] >= lvl) {
                    continue;
                }
                let (t, tgt) = (mk(-1.0, i, lvl, "limit"), lvl - cover);
                if lo[i] <= tgt {
                    out.push(shut(t, i + 1, tgt, "Covered at fair value".into()));
                } else if last[i].is_finite() {
                    out.push(shut(t, i + 1, last[i], "Flat before 8:30 news".into()));
                }
            }
        }
        _ => return None,
    }
    Some(out)
}

/// (family id, family name, variant label) for the strategy list.
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "hfm_news" => ("hfm_news", "News Reversals", "Long & Short"),
        "hfm_news_long" => ("hfm_news", "News Reversals", "Long (gap down)"),
        "hfm_news_short" => ("hfm_news", "News Reversals", "Short (gap up)"),
        "hfm_hv" => ("hfm_hv", "Connors-Hayward Historical Volatility", ""),
        "hfm_und" => ("hfm_und", "Undeniables", "Long & Short"),
        "hfm_und_long" => ("hfm_und", "Undeniables", "Buy reversal"),
        "hfm_und_short" => ("hfm_und", "Undeniables", "Sell reversal"),
        "hfm_chadtp" => ("hfm_chadtp", "CHADTP (Advance-Decline Pattern)", "NYSE breadth"),
        "hfm_chadtp_nse" => ("hfm_chadtp", "CHADTP (Advance-Decline Pattern)", "NSE breadth"),
        "hfm_ndxspx" => ("hfm_ndxspx", "NDX-SPX", ""),
        "hfm_vix" => ("hfm_vix", "VIX Day-Trading Program", ""),
        "hfm_globex_fv" => ("hfm_globex_fv", "Globex S&P Fair Value Program", ""),
        "hfm_bond_evening" => ("hfm_bond_evening", "Bond Evening Session Persistency", ""),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn rows(b: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        b.iter()
            .enumerate()
            .map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("1994-11-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None })
            .collect()
    }

    fn ps(kv: &[(&str, f64)]) -> Params {
        kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn book_worked_examples() {
        // VIX program, Nov 7-8 1994 (S&P Dec futures): VIX 16.36, prior high 464.90, buy stop .50 above = 465.40,
        // protective stop 463.40, close 466.95 -> +1.55 on the day.
        let r = rows(&[(463.0, 464.90, 462.0, 464.0), (464.5, 467.5, 464.0, 466.95)]);
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert("VXO".into(), vec![16.36, 14.0]);
        let t = run("hfm_vix", &x, 0, 1, &ps(&[("use_vxo", 1.0), ("off", 0.5 / 464.90 * 100.0), ("stop", 2.0 / 465.40 * 100.0)])).unwrap();
        assert_eq!(t.len(), 1);
        assert!(t[0].dir > 0.0 && (t[0].legs[0].px - 465.40).abs() < 1e-9 && t[0].exit_px == 466.95);
        assert_eq!(t[0].reason, "Market on close"); // low 464.00 never reached the 463.40 stop

        // News reversal, Copley Pharmaceutical Oct 11 1993: Friday high 50, opens 53, short 1/8 below at 49 7/8, closes 46.
        let r = rows(&[(48.0, 50.0, 47.5, 49.75), (53.0, 53.0, 45.5, 46.0)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("hfm_news_short", &x, 0, 1, &ps(&[("off", 0.125 / 50.0 * 100.0)])).unwrap();
        assert_eq!(t.len(), 1);
        assert!((t[0].legs[0].px - 49.875).abs() < 1e-9 && t[0].exit_px == 46.0); // +3 7/8 points as in the book
        assert!(run("hfm_news_long", &x, 0, 1, &Params::new()).unwrap().is_empty());
    }

    #[test]
    fn bar_path_stop_order() {
        // long fill at 101 (prior low 100 + 1 tick, opened 99.5), stop = open 99.5.
        // open nearer the low -> O-L-H-C: the low comes before the fill; close 100 above the stop -> not stopped
        let r = rows(&[(101.0, 102.0, 100.0, 101.0), (99.5, 103.0, 99.0, 100.0)]);
        let x = Ctx::new(&r, &r, &[]);
        assert!(!bar_path_stop(&x, 1, 1.0, 101.0, 99.5));
        // open nearer the high -> O-H-L-C: fill on the way up, then the low 99.0 hits the stop
        let r = rows(&[(101.0, 102.0, 100.0, 101.0), (99.5, 101.5, 97.0, 100.0)]);
        let x = Ctx::new(&r, &r, &[]);
        assert!(bar_path_stop(&x, 1, 1.0, 101.0, 99.5));
        let t = run("hfm_news_long", &x, 0, 1, &ps(&[("tick", 1.0)])).unwrap();
        assert_eq!((t.len(), t[0].exit_px), (1, 99.5));
        assert_eq!(run("hfm_news_long", &x, 0, 1, &ps(&[("tick", 1.0), ("stop_mode", 0.0)])).unwrap()[0].exit_px, 100.0);
    }

    #[test]
    fn globex_oct_14_1994() {
        // Cash 469.10 + fair value 1.42 = 470.52 -> 470.50, + .50 = sell 471.00; futures closed 471.25,
        // Sunday evening low 469.60: covered at 470.50 for .50.
        // the example day is bar 1 (every rule starts at bar 1)
        let r = rows(&[(468.0, 469.5, 467.9, 469.0), (468.0, 469.5, 467.9, 469.10), (469.0, 470.0, 468.5, 469.5)]);
        let mut x = Ctx::new(&r, &r, &[]);
        for (k, v) in [("FV_PREMIUM", 1.42), ("FUT_ON_HIGH", 471.25), ("FUT_ON_LOW_AFTER_HIGH", 469.60), ("FUT_0830", 470.2)] {
            x.extra.insert(k.into(), vec![f64::NAN, v, f64::NAN]);
        }
        let t = run("hfm_globex_fv", &x, 0, 2, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert!(t[0].dir < 0.0 && t[0].legs[0].i == 1 && (t[0].legs[0].px - 471.0).abs() < 1e-9);
        assert!((t[0].exit_px - 470.5).abs() < 1e-9 && t[0].exit_i == 2);
    }

    #[test]
    fn chadtp_feb_2_1994() {
        // CHADTP +400.6 on day 5 -> next day sell .10 below the prior low 479.65 = 479.55, out on the close of day 6.
        let mut b = vec![(480.0, 481.0, 479.65, 480.5); 6];
        b.extend(vec![(480.0, 480.0, 479.0, 479.2); 6]);
        let r = rows(&b);
        let mut x = Ctx::new(&r, &r, &[]);
        let mut adv = vec![f64::NAN; 12];
        for (k, v) in [1279.0 - 851.0, 400.0, 400.0, 400.0, 375.0].into_iter().enumerate() {
            adv[1 + k] = v; // 5-day sum 2003 -> 400.6
        }
        x.extra.insert("NYSE_ADV".into(), adv);
        x.extra.insert("NYSE_DECL".into(), vec![0.0; 12]);
        let t = run("hfm_chadtp", &x, 0, 11, &ps(&[("off", 0.10 / 479.65 * 100.0)])).unwrap();
        assert_eq!(t.len(), 1);
        assert!(t[0].dir < 0.0 && t[0].legs[0].i == 6 && (t[0].legs[0].px - 479.55).abs() < 1e-9);
        assert_eq!(t[0].exit_i, 11);
    }
}
