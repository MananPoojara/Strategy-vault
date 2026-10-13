//! Larry Connors, "Connors on Advanced Trading Strategies" (1998). Book id "ats".
//! Rules transcribed from the book (extracts library/ats/extract-1..3, cross-checked with the second scan library/coats).
//! The book traded S&P 500 futures (1 contract) and single stocks; the vault runs the same rules on index data.
//!
//! Local helpers (shared code untouched):
//!   * `book_hv`  : the book's Appendix historical volatility (sample sd of the N-1 most recent log returns x sqrt(260) x 100)
//!   * `dmi3`     : Wilder +DI / -DI / ADX (connors::dmi only returns +DI and ADX; -DI = +DI of the mirrored bars)
//!   * `vix_rows` : raw VIX OHLC. `Ctx.vix` only exposes derived VIX facts, so the module re-reads the server's VIX
//!                  directory (3rd command-line argument, same loader as main) and aligns it by date.
//!   * `sim`      : one-position simulator with close entries, next-bar stop entries (valid N bars), intraday stop-loss,
//!                  close / next-open / price exits, stop-and-reverse and same-direction extension.
#![allow(unused_imports, dead_code)]
use crate::connors::*;
use crate::Row;
use std::sync::OnceLock;

pub const BOOK: &str = "ats";

// ---------------------------------------------------------------- catalogue

const CVR_NOTE: &str = "Book traded S&P 500 futures with the CBOE VIX of 1993-97, which is today's VXO (S&P 100 based). The vault runs the rules on the index/ETF with the VIX of its region (India VIX for NSE, CBOE VIX for US; a VXO directory can be passed as the VIX dir). An opposite signal while in a trade closes it and reverses (the appendix trade lists show same-day SExit + Buy rows). Side param: 0 = long and short (book), 1 = long only, -1 = short only. The book's numbers need the VXO: with the VXO the SPY runs reproduce 63/72 (CVR I), 92/97 (CVR II) and 71/75 (CVR III) of the appendix entries; with today's CBOE VIX only 19/72 (CVR I). [Not from the book] VIX bars are aligned to the traded ticker's calendar (VIX rows on days the ticker does not trade are dropped; 'yesterday' = the previous VIX bar, so a missing VIX day is skipped).";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    let vx = |s: StrategyDef| StrategyDef { needs_vix: true, ..s };
    let ser = |s: StrategyDef, n: &'static [&'static str]| StrategyDef { needs_series: n, ..s };
    let opt = NotTestable("needs option chains");
    const G2: &str = "Ch 2 · Connors VIX Reversals";
    vec![
        // ---- Ch 2 CVR
        vx(d("ats_cvr1", "Connors VIX Reversal I (CVR I)", G2, "Long/Short", Live,
            &["E|Buy: today the VIX makes a 15-day high (VIX high = highest high of the last 15 days, today included)",
              "E|Buy: today the close of the VIX is below its open",
              "E|If both are met, buy the market on the close of today",
              "E|Sell: today the VIX makes a 15-day low and closes above its open; sell the market on the close",
              "X|Hold the position at least one to three days: exit on the close of the 3rd day",
              "X|An opposite signal closes the trade and reverses it (appendix: '04/04/96 SS (1 DAY)')"],
            CVR_NOTE, claim("72 (L 50 / S 22)", "65% (L 64% / S 68%)", "$1,130.56 per contract", "3 / 3 bars (win / loss)", "01/01/93-10/01/97, S&P 500 futures, VIX of 1993-97 = today's VXO"))),
        vx(d("ats_cvr2", "Connors VIX Reversal II (CVR II)", G2, "Long/Short", Live,
            &["E|Take a five-period RSI of the closing VIX",
              "E|When a daily RSI reading above 70 is followed by a downtick in RSI, buy the market that day on the close",
              "E|When an RSI reading below 30 is followed by an uptick in RSI, sell the market that day on the close",
              "X|Exit five to eight trading days later: the published test uses an eight-day exit",
              "X|An opposite signal closes the trade and reverses it (appendix: many same-day exit + entry rows)"],
            CVR_NOTE, claim("96 (L 55 / S 41)", "61% (L 67% / S 54%)", "$1,865.63 per contract", "7 / 8 bars (win / loss)", "01/01/93-10/01/97, S&P 500 futures, VIX of 1993-97 = today's VXO"))),
        vx(d("ats_cvr3", "Connors VIX Reversal III (CVR III)", G2, "Long/Short", Live,
            &["E|Buy: \"Today, the low of the VIX must be above its 10-day moving average.\" Coded with yesterday's 10-day MA (ma_lag = 1) [Fitted to the book's own appendix trade list, not the rule text: 71/75 entries with yesterday's MA vs 53/75 with today's; ma_lag = 0 = the text's 'its' (today's) MA]",
              "E|Buy: today the VIX closes at least 10 percent above yesterday's 10-day moving average; buy the market on the close",
              "E|Sell: the VIX high is below yesterday's 10-day MA and the VIX closes at least 10 percent below it; sell on the close",
              "X|Long: exit on the close the day the VIX trades (intraday) below yesterday's 10-day moving average (strictly: VIX low < MA; a touch does not exit)",
              "X|Short: exit on the close the day the VIX trades (intraday) above yesterday's 10-day moving average (strictly: VIX high > MA)"],
            CVR_NOTE, claim("75 (L 41 / S 34)", "72% (L 78% / S 65%)", "$1,609.00 per contract", "4 / 5 bars (win / loss)", "01/01/93-10/01/97, S&P 500 futures, VIX of 1993-97 = today's VXO"))),
        // ---- Ch 3 TRIN Thrust
        ser(d("ats_trin_thrust", "TRIN Thrust · NYSE TRIN", "Ch 3 · TRIN Thrusts", "Long/Short", Live, TRIN_RULES,
            "Series NYSE_TRIN = NYSE Arms index daily close ((adv issues/dec issues)/(adv volume/dec volume)), market-wide. The author rarely uses it stand-alone. A new opposite signal closes and reverses the trade.",
            claim("764 (L 342 / S 422)", "51% (L 57% / S 46%)", "$244.10 per contract", "2 / 1 bars (win / loss)", "01/03/90-10/01/97, S&P 500 futures, 1.5% stop")), &["NYSE_TRIN"]),
        ser(d("ats_trin_thrust_nse", "TRIN Thrust · NSE TRIN", "Ch 3 · TRIN Thrusts", "Long/Short", Live, TRIN_RULES,
            "India mapping: series NSE_TRIN = (advancing/declining NSE issues)/(advancing/declining volume), built from the daily bhavcopy (sum of traded quantity of stocks closing up vs down). Same rules as the NYSE version.",
            None), &["NSE_TRIN"]),
        // ---- Ch 4 PADI
        ser(d("ats_padi", "PADI · NYSE breadth", "Ch 4 · Percent Advance/Decline Indicator", "Long/Short", Live, PADI_RULES,
            "Series NYSE_ADV_DEC.csv with columns Advances, Declines, Unchanged (NYSE issue counts per day). 'Shares traded' in rule 1 means issues. The 100-day MA filter is on this ticker's close. Opposite signal = exit and take the new signal. The text says a five-day exit (default hold=5), but the book's own statistics (14.0 trades/yr, avg bars 6 / 5) fit hold=6 better: SPY 1993-97 hold=6 gives 13.9/yr, avg bars 6.0/4.6 vs hold=5 15.2/yr, 5.0/3.6 (TradeStation counts bars as exit bar - entry bar).",
            claim("109 (L 83 / S 26)", "61% (L 64% / S 50%)", "$1,311.31 per contract", "6 / 5 bars (win / loss)", "01/01/90-10/03/97, S&P 500 futures, 1.5% stop, 5-day exit")),
            &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.Unchanged"]),
        ser(d("ats_padi_nse", "PADI · NSE breadth", "Ch 4 · Percent Advance/Decline Indicator", "Long/Short", Live, PADI_RULES,
            "India mapping: NSE_ADV_DEC.csv with columns Advances, Declines, Unchanged (NSE equity issue counts, nseindia.com market-data/advance or rebuilt from bhavcopy). Same rules.",
            None), &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines", "NSE_ADV_DEC.Unchanged"]),
        // ---- Ch 7
        d("ats_hv_ratio_setup", "Historical volatility ratio under 50% (6/100, 10/100)", "Ch 7 · Additional Volatility Research", "Long/Short",
            NotTestable("setup/filter only: no direction, entry or exit (the HV Thrust adds the trigger)"),
            &["F|Whenever a 10-day (or 6-day) historical volatility is 50 percent or less of the 100-day reading, a large market move is likely",
              "F|Better: the ratio stays under 50 percent for more than one day, or both the 6-day and 10-day readings are under 50 percent",
              "F|After an extraordinary move (1987 crash) wait a full 100 days before trusting the readings"],
            "HV = book Appendix definition. Coded as the filter inside ats_hv_thrust and ats_gipsons.", None),
        d("ats_hv_thrust", "HV Thrust", "Ch 7 · Additional Volatility Research", "Long/Short", Live,
            &["F|The 6-day or 10-day historical volatility is less than 1/2 its 100-day reading (read on the prior day)",
              "E|On the day of the readings under 50 percent look for a daily range larger than the daily range of the previous nine days",
              "E|Trade in the direction of the large-range day (close above the prior close = buy, below = sell) on its close",
              "X|Expect follow-through for the next one to ten days: exit on the close after `hold` days (default 5)"],
            "No statistics in the book. The GBP example has the ratio under 50% on Apr 8 and the range expansion on Apr 9, so the ratio is read on the prior day (param when=1; 0 = same day). HV per the Appendix (sample sd of N-1 log returns x sqrt(260)); hv_rets=0 uses N returns.", None),
        // ---- Ch 8-10 patterns
        d("ats_1234", "1-2-3-4", "Ch 8 · 1-2-3-4", "Long/Short", Live,
            &["F|Buys: 14-day ADX greater than 30 and 14-day +DI greater than -DI (sells: -DI > +DI)",
              "E|Wait for a 1-2-3 correction: three consecutive intraday lower lows, or two lower lows and an inside day (sells: higher highs)",
              "E|On day four only, buy 1 tick above the day-three high (sells: 1 tick below the day-three low)",
              "X|Initial protective stop near the day-three low (sells: day-three high), placed 1 tick beyond it",
              "X|Trail stops as the position moves in your favor: not specified; the vault exits on the close after `hold` bars"],
            "Developed with Jeff Cooper (Hit and Run Trading). 'Inside day' = high <= prior high and low >= prior low; at least two of the three days are lower lows. Day four only; a gap through the entry level fills at the open (Gucci example). The book gives no exit beyond the initial stop ('trail your stops'); the vault adds a time exit (`hold` bars, not in the book). One tick = `tick` in price units (book 1/8; NSE 0.05). two_for_one=1 adds the Ch 28 half-off-at-+1R overlay.", None),
        d("ats_8day_hl_reversal", "8-Day High/Low Reversal", "Ch 9 · 8-Day High/Low Reversal Method", "Long/Short", Live,
            &["E|Buys: day one must be an eight-day low; day two must trade above the day-one high",
              "E|Days three, four, five or six must trade under the day-two low (this can be a new low)",
              "E|When condition three is met, buy one tick above the day-two high if the breakout occurs within four trading days of condition three (the same day counts)",
              "E|Sells are reversed (eight-day high, day two below the day-one low, then above the day-two high, sell 1 tick under the day-two low)",
              "X|Stop one tick below the day-two low (sells: above the day-two high)",
              "X|No exit in the book: the vault exits on the close after `hold` bars"],
            "Microsoft example: conditions 3 and 4 on the same bar are accepted; the fill is then at the entry level. Otherwise a gap through the level fills at the open. No statistics in the book; exit not in the book (time exit added). two_for_one=1 adds the Ch 28 half-off-at-+1R overlay.", None),
        d("ats_smtp", "Spent Market Trading Pattern (SMTP)", "Ch 10 · Spent Market Trading Pattern", "Long/Short", Live,
            &["E|Today the market makes a 10-period low (sells: 10-period high)",
              "E|Today's range is the largest range of the past 10 bars",
              "E|Today's close is in the top 25 percent of today's range (sells: bottom 25 percent)",
              "E|Tomorrow, or the trading day after tomorrow, buy one tick above today's high (sells: one tick below today's low)",
              "X|If filled, initial protective stop at today's (setup day's) low (sells: high)",
              "X|Trail it appropriately: not specified; the vault exits on the close after `hold` bars"],
            "Merged from extract part 1 (rules) and the p.85-86 tail (Fig 10.4 S&P, Fig 10.5 SLOT, summary). No statistics in the book; exit not in the book (time exit added).", None),
        // ---- Section 4 equities
        d("ats_cbp", "Crash, Burn and Profit (CBP)", "Ch 11 · Crash, Burn and Profit", "Short", Live,
            &["F|Weekly ADX reading above 60 (weekly bars, Wilder 14)", "F|Price above 20",
              "E|After the weekly ADX reaches 60, wait for the first lower weekly ADX reading; short on that week's close",
              "X|Risk no more than 15 percent: buy stop 15% above the entry",
              "X|Cover one-half upon a 20 percent profit and use trailing stops on the other half (half=0: cover all at +20%, the old vault reading)",
              "X|[Not from the book] the trail on the other half: the highest high of the prior 20 days (trail_n), from the bar after the half cover"],
            "One signal per excursion: the first lower weekly ADX after it reaches 60; re-armed when the ADX falls back under 60 (once=0 = every downtick, the old vault reading). The half cover is folded into one trade (exit price = average of the two halves). Built for momentum stocks (IBD RS 98-99 scan); run on weekly bars resampled from the index's daily bars, where weekly ADX > 60 is rare (zero trades is an honest answer). min_price 20 is in dollars and does not bind on indices.", None),
        d("ats_late_earnings", "When They're Late, They're Probably Dead", "Ch 12 · Late earnings", "Short",
            NotTestable("needs an expected-vs-actual earnings calendar and put options; 'trending lower' is discretionary"),
            &["F|Company is late reporting its earnings", "F|Its price trends lower before the earnings are due (while the market rises, near the year's lows)", "E|Buy puts"],
            "Baby Superstores example, June 35 puts bought at 2 1/4 went above 10.", None),
        StrategyDef { needs_volume: true, ..d("ats_dvmtm", "Double Volume Market Top Method (DVMTM)", "Ch 13 · Double Volume Market Top Method", "Short", Live,
            &["F|A stock must be trading near or at a three-month high (highest high of the last 63 trading days)",
              "E|Today's volume must be double its 15-day average volume (average of the 15 prior days)",
              "E|Either today, tomorrow or the next day the stock must close below its open (Rule 3 day)",
              "E|Over the next two days sell 1/8 (one tick) under the Rule 3 day low",
              "X|Initial protective stop at the top of the Rule 3 day bar",
              "X|Trailing stops: not specified; the vault covers on the close after `hold` bars"],
            "Built for single stocks (Microsoft, Yahoo, WDC, Fila examples, no statistics); run on index volume. near_pct widens 'near' (0 = at the high).", None) },
        d("ats_cvr_equities", "CVR I on stocks (Merrill Lynch, Intel)", "Ch 14 · Trading Equities with the CVR", "Long/Short",
            NeedsData("single-stock daily data (Merrill Lynch, Intel 1996-97); the rule itself is ats_cvr1 run on a stock"),
            &["E|On a CVR I buy (sell) signal buy (sell short) the stock on the close; front-run the 4:00 VIX close by a few minutes",
              "X|Hold for three days with some sort of protective stop"],
            "Published tables (15 months, CVR I, no costs): Merrill Lynch 25 trades, 18 profitable / 7 unprofitable, +22 1/4 points; Intel 25 trades, 13 / 10 / 2 breakeven, +30 7/8 points. ML lists 02/26/97 where Intel and Ch 17 give 02/20/97 (misprint).",
            claim("25 ML / 25 INTC", "72% ML (18/7) · 52% INTC (13/10/2)", "+0.89 pts ML · +1.24 pts INTC", "3 days", "Jan 1996 - Mar 1997, Merrill Lynch & Intel")),
        d("ats_torpedoes", "Torpedoes (Jeff Cooper)", "Ch 15 · Torpedoes", "Long",
            NotTestable("discretionary: 'market begins to bounce back' and 'strongest stock' are subjective; needs intraday stock universe"),
            &["F|Market leaders / high-volatility names", "E|Stock market down at least 1 percent in the first hour; find leaders up for the day or down only a little",
              "E|If the overall market begins to bounce back, buy the strongest stock(s)", "X|Risk one point from entry; re-entry allowed; let profits run"],
            "No short-sale torpedoes.", None),
        d("ats_hv_ratio_short_strangle", "Trading Volatility with Options (10/100 HV strangle)", "Ch 16 · Trading Volatility with Options", "Short vol", opt,
            &["F|10-day HV reaches 1.65 times the 100-day HV or higher", "E|Wait for two consecutive down-ticks in the 10-day reading, then sell an OTM strangle",
              "X|Exit when HV10 reverts to HV100; close anyway after 10-15 days"],
            "Fernando Diz. Gold, silver, soybean examples. Signal is daily_ohlc; the P&L needs option prices.", None),
        d("ats_cvr_options", "Trading Options with the CVR", "Ch 17 · Trading Options with the CVR", "Long/Short", opt,
            &["E|CVR buy signal: sell near-the-money puts (within two to three strikes)", "E|CVR sell signal: buy puts",
              "X|Hold three days; take profits on half on a sharp early move", "X|Close naked puts if the index reaches the strike; close long puts on a 10 S&P point adverse move"],
            "OEX examples Feb 20 and Feb 28, 1997 (puts sold 12 3/4 and 11 1/2, bought back 7 1/4 and 5 1/4).", None),
        d("ats_split_call_selling", "Options on Stock Splits", "Ch 18 · Options on Stock Splits", "Short", opt,
            &["E|After a split announcement, sell the overpriced calls while implied volatility stays inflated"], "No mechanical thresholds; needs split announcement dates and option IV.", None),
        d("ats_sector_options_overpriced", "Exploiting Overpriced Sector Options", "Ch 19 · Overpriced Sector Options", "Short vol", opt,
            &["E|Sector index whose near-the-money put and call implied volatility (combined) is three times the 50-day HV: sell them",
              "X|Risk 30 to 45 percent of the combined premium"], "RIX example (HV50 4.76%, IVs about 13%).", None),
        // ---- Section 6 day trading
        ser(d("ats_15min_adx_breakout", "15-Minute ADX Breakout", "Ch 20 · 15-Minute ADX Breakout", "Long/Short", Live, ADX15_RULES, ADX15_NOTE, None), ADX15_SERIES),
        ser(d("ats_15min_adx_breakout_equities", "15-Minute ADX Breakout · equities filter", "Ch 20 · 15-Minute ADX Breakout", "Long/Short", Live, ADX15_RULES,
            "Equities filter (p.164): price above $40 and 100-day HV of 40 percent or higher (book HV definition), on top of the ADX filter. $40 does not map to index levels; set min_price for the instrument. Same intraday series as the futures version.",
            None), ADX15_SERIES),
        d("ats_10pct_oops", "10% OOPS", "Ch 21 · 10% OOPS", "Long/Short", Live,
            &["E|Buys: day one closes in the bottom 10 percent of its daily range",
              "E|Day two: the next morning it must gap lower (open below the day-one low)",
              "E|If so, buy at the day-one low (buy stop)",
              "E|Sells: day one closes in the top 10 percent of its range, day two gaps higher, sell at the day-one high",
              "X|Sell MOC (the published 1996 test: mechanical, no stops)",
              "X|Optional (stop=1): initial protective stop near the day-two opening"],
            "Published 1996 S&P list: 15 trades, 12 profitable, +43.75 points (book total $19,880 does not reconcile with its own list at $500/pt). Gap = open beyond the day-one low/high (gap=1); gap=0 uses the close.",
            claim("15", "80% (12 / 3)", "+2.92 S&P pts", "same day (MOC)", "1/1/96-12/31/96, S&P 500 futures, no stops")),
        d("ats_morning_call_reversal", "Reversals off the Morning Call", "Ch 22 · Reversals off the Morning Call", "Long/Short",
            NotTestable("discretionary: needs the pre-open 'called to open' indication and news"),
            &["F|A market is called to open significantly higher on a fundamental event (sells; buys reversed)",
              "F|The call weakens before the open (e.g. called 57-59, lowered to 56-58, opens at the low end)",
              "E|After the opening, place a sell stop at yesterday's closing price",
              "X|On being filled, protective buy stop one tick above today's high so far"],
            "General RE, Orange Juice and Cigna examples (merged from parts 2 and 3).", None),
        // ---- Section 7
        d("ats_wide_range_gap", "Wide Range Exhaustion Gap Reversals", "Ch 23 · Wide Range Exhaustion Gap Reversals", "Long/Short", Live,
            &["E|Day 1: at least a two standard deviation daily move to the downside (sells: upside); 1 SD = previous close x HV100 / sqrt(256)",
              "E|Day 2: if the market gaps lower (open below the day-one low), buy stop near the previous day's close (not its low)",
              "X|Initial protective stop at the day-one low (sells: day-one high)",
              "X|If the position closes in the top 10 to 15 percent of its daily range carry it into the next morning (exit next open), otherwise lock in profits near the close"],
            "2-SD box (Ch 27) prints sqrt(265) but its arithmetic uses 256; sd_days param. HV100 = book Appendix HV of the day before the move. Entry-bar stop: the gap-down low came before the fill, so the stop only counts if the bar closes beyond it. No statistics in the book.", None),
        d("ats_flag_within_a_flag", "Trading Runaway Moves: flag within a flag", "Ch 24 · Trading Runaway Moves", "Long",
            NotTestable("discretionary: run-up, large-range day and half-hour upthrust are not quantified; needs half-hour bars and an RS/new-high universe"),
            &["F|New 52-week high after breaking out of a 17+ day flag that did not retrace 38 percent of the run-up",
              "E|Half-hour chart: a flag of four bars or more not retracing 38 percent; buy above its high", "X|Stop below the half-hour flag low; exit half when the risk is covered"],
            "Mark Boucher. TUBO, CTS, D-Mark examples.", None),
        d("ats_gipsons", "Gipsons", "Ch 25 · Gipsons", "Long/Short", Live,
            &["F|14-period ADX above 25; trade only in the direction of the trend (+DI vs -DI)",
              "F|6/100 day historical volatility reading under .40",
              "E|Long: entry when the high of the signal day is exceeded (short: the low is taken out); next day, re-armed by each new signal day",
              "X|Initial protective stop near the low of the signal day (shorts: the high)",
              "X|Exit (recommendation c): take half off on the close of the day the 6-day HV reaches the 100-day HV and tighten the stop on the other half (exit_all=1: exit everything there)",
              "X|[Not from the book] 'tighten the stop' = trailing stop at the prior 3-day low (shorts: high), from the day after the half exit (trail_n)"],
            "Derek Gipson. IBM, Pediatrix, crude and Dec 96 S&P examples (S&P signal Nov 13, 1996, buy Nov 14). Half exits are folded into one trade (exit price = average of the two halves), as btp's Gipsons. HV per the Appendix (N-1 returns; hv_rets=0 for N).", None),
        d("ats_short_term_decline_playbook", "Maximizing Profits in Short-Term Market Declines", "Ch 26 · Short-Term Market Declines", "Short",
            NotTestable("discretionary: triggered by late-Friday negative news; mostly options"),
            &["E|Buy puts on XBD/NDX/IUX, sell OEX calls, short brokerage and fund stocks, sell S&P futures MOC, or short SPY"], "", None),
        d("ats_large_range_day_combo", "Large-Range Days (sell a combination)", "Ch 27 · Large-Range Days", "Short vol", opt,
            &["F|Market closed two standard deviations from the previous close (1 SD = HV100 / sqrt(256))", "F|Not in trending markets (ADX greater than 25)",
              "E|Sell calls one SD above the large day's high and puts one SD below its low"],
            "The book lists 44 bond 2-SD dates (1995-96) for a detector check (needs T-bond futures).", None),
        d("ats_two_for_one", "Two-for-One Money Management", "Ch 28 · Two-for-One Money Management", "Long/Short",
            Live,
            &["E|One unit = 1,000 shares or two contracts; entry = a base pattern (base 1 = 1-2-3-4, the book's PROX and TOM examples; base 2 = the 8-day high/low reversal, which Ch 9 p.76 says to combine with it)",
              "X|Take half a unit off at a profit equal to the original stop distance (entry +/- initial risk); keep the original stop on the other half",
              "X|On the entry bar the protective stop is checked before the +1R half exit",
              "X|The other half leaves on the base pattern's exit: [Not from the book] the close after `hold` bars (the base patterns have no book exit)"],
            "Money-management overlay run on 1-2-3-4 or the 8-day reversal (also available as two_for_one=1 on ats_1234 / ats_8day_hl_reversal). The half exit is folded into one trade: exit price = 0.5 x half price + 0.5 x rest price (vault convention, as btp/cwin).", None),
        vx(d("ats_after_losses", "CVR after consecutive losses", "Ch 29 · More Thoughts", "Long/Short", Live,
            &["F|The best time to trade a methodology is after it has had two or three consecutive losses",
              "E|Take a CVR signal only when the previous `k` signals of the same CVR (traded or not) were all losers",
              "X|Exit as the base CVR strategy"],
            "Meta-filter from the Q&A applied to the CVR (base=1/2/3). The base CVR runs with its book defaults.", None)),
    ]
}

const TRIN_RULES: &[&str] = &[
    "E|Buy: today's closing TRIN is 30 percent or more below yesterday's closing TRIN; buy the market on the close",
    "E|Sell: today's closing TRIN is 30 percent or more above yesterday's closing TRIN; sell the market on the close",
    "X|Close the position on the close two trading days later",
    "X|Protective stop 1 1/2 percent from the entry (the published test uses 1 1/2 percent)",
];
const PADI_RULES: &[&str] = &[
    "F|Advancing percent = advances / (advances + declines + unchanged); declining percent likewise; take six-day averages",
    "E|Sell on the close if yesterday's advancing average was 40 percent or higher and today's is lower (only if the market is below its 100-day MA)",
    "E|Buy on the close if yesterday's declining average was 40 percent or higher and today's is lower (only if the market is above its 100-day MA)",
    "X|Exit in five to seven days (the published test: five) or on a signal in the opposite direction",
    "X|Protective stop 1 1/2 percent",
];
const ADX15_RULES: &[&str] = &[
    "F|Daily 14-period ADX above 30 and +DI greater than -DI (sells reversed), as of yesterday's close",
    "E|Wait for the first 15 minutes of trading to pass; buy stop one tick above the first 15-minute bar high (sells: below its low)",
    "X|Protective sell stop one tick below the first 15-minute bar low (buys)",
    "X|Close all positions out before the end of the day",
    "E|\"If you are stopped out, it is correct to re-enter. These second entries tend to be the best trades.\" (p.165) \"I will not enter a trade three times in a day. Two losses are my psychological limit.\" (p.156): re-entry at the same buy (sell) stop, at most max_losses = 2 entries a day",
];
const ADX15_SERIES: &[&str] = &["OPEN15_HIGH", "OPEN15_LOW", "LONG_FILLS", "LONG_STOPS", "SHORT_FILLS", "SHORT_STOPS"];
const ADX15_NOTE: &str = "Coded against the per-ticker daily series built from real intraday bars by library/ptjdt/build_series.py (same method as ptjdt_adx15): OPEN15_HIGH / OPEN15_LOW = high / low of the first 15 minutes; LONG_FILLS / LONG_STOPS (SHORT_*) = how many times the buy stop at OPEN15_HIGH + 1 tick filled and the protective stop at OPEN15_LOW - 1 tick was hit, re-entering at the same level after every stop-out (the builder's tick must equal the tick param, 0.05 on NSE). The first max_losses (2) fills are traded: a stopped fill exits at the stop, the last unstopped one MOC at the daily close. [Not from the book] When one intraday bar both fills and crosses the stop level, the builder takes the extreme nearer the bar's open as first. The book's trailing stops / half-off are personal choice, so the position is held to the close.";

// ---------------------------------------------------------------- params

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let side = || p("side", "Side: 0 both, 1 long only, -1 short only", 0.0, -1.0, 1.0, 1.0);
    let rev = || p("reverse", "Opposite signal reverses (1) or is ignored (0)", 1.0, 0.0, 1.0, 1.0);
    let ext = |d| p("extend", "Same-side signal on the exit day keeps the position (1, TradeStation netting)", d, 0.0, 1.0, 1.0);
    let tick = || p("tick", "One tick (price units)", 0.05, 0.0, 1.0, 0.01);
    let hold = |d| p("hold", "Exit on the close after N bars", d, 1.0, 30.0, 1.0);
    let losses = || p("max_losses", "Entries a day: re-enter after a stop-out up to N entries (book p.156: 2; 1 = no re-entry)", 2.0, 1.0, 5.0, 1.0);
    let hvr = || p("hv_rets", "HV uses N-1 returns (1, book) or N (0)", 1.0, 0.0, 1.0, 1.0);
    let t41 = |d| p("two_for_one", "Ch 28 Two-for-One: half off at +1x initial risk (1 = on)", d, 0.0, 1.0, 1.0);
    Some(match id {
        "ats_cvr1" => vec![p("n", "VIX N-day high/low", 15.0, 5.0, 30.0, 1.0), hold(3.0), side(), rev(), ext(1.0)],
        "ats_cvr2" => vec![p("rsi_n", "VIX RSI period", 5.0, 2.0, 14.0, 1.0), p("ob", "VIX RSI overbought (buy) at or above", 70.0, 55.0, 90.0, 5.0),
            p("os", "VIX RSI oversold (sell) at or below", 30.0, 10.0, 45.0, 5.0), hold(8.0), side(), rev(), ext(1.0)],
        "ats_cvr3" => vec![p("ma", "VIX SMA length", 10.0, 5.0, 20.0, 1.0), p("pct", "VIX close % beyond its SMA", 10.0, 3.0, 20.0, 1.0),
            p("ma_lag", "Entry compares with today's (0) or yesterday's (1) SMA", 1.0, 0.0, 1.0, 1.0), side(), rev()],
        "ats_trin_thrust" | "ats_trin_thrust_nse" => vec![p("thrust", "TRIN change % vs yesterday", 30.0, 10.0, 50.0, 5.0), hold(2.0),
            p("stop", "Protective stop %", 1.5, 0.0, 3.0, 0.25), side(), rev()],
        "ats_padi" | "ats_padi_nse" => vec![p("avg", "Breadth average days", 6.0, 3.0, 10.0, 1.0), p("level", "Average % at or above", 40.0, 25.0, 50.0, 1.0),
            p("trend", "Market SMA filter length", 100.0, 50.0, 200.0, 10.0), hold(5.0), p("stop", "Protective stop %", 1.5, 0.0, 3.0, 0.25), side()],
        "ats_hv_thrust" => vec![p("short1", "Short HV period A", 6.0, 3.0, 10.0, 1.0), p("short2", "Short HV period B", 10.0, 5.0, 20.0, 1.0),
            p("long", "Long HV period", 100.0, 50.0, 150.0, 10.0), p("ratio", "Short/long HV below", 0.5, 0.3, 0.7, 0.05),
            p("range_n", "Range larger than previous N days", 9.0, 5.0, 20.0, 1.0), p("when", "Ratio on prior day (1) or same day (0)", 1.0, 0.0, 1.0, 1.0), hold(5.0), hvr(), side()],
        "ats_1234" => vec![p("adx_n", "ADX/DI period", 14.0, 7.0, 21.0, 1.0), p("adx", "ADX above", 30.0, 20.0, 45.0, 5.0), tick(), hold(10.0), side(), t41(0.0)],
        "ats_8day_hl_reversal" => vec![p("n", "Day one N-day low/high", 8.0, 5.0, 15.0, 1.0), p("c3_days", "Condition-3 window (days 3..)", 4.0, 2.0, 6.0, 1.0),
            p("window", "Breakout within N days of condition 3", 4.0, 1.0, 8.0, 1.0), tick(), hold(10.0), side(), t41(0.0)],
        "ats_two_for_one" => vec![p("base", "Entry pattern: 1 = 1-2-3-4, 2 = 8-day high/low reversal", 1.0, 1.0, 2.0, 1.0),
            p("adx_n", "1-2-3-4: ADX/DI period", 14.0, 7.0, 21.0, 1.0), p("adx", "1-2-3-4: ADX above", 30.0, 20.0, 45.0, 5.0),
            p("n", "8-day: day one N-day low/high", 8.0, 5.0, 15.0, 1.0), p("c3_days", "8-day: condition-3 window", 4.0, 2.0, 6.0, 1.0),
            p("window", "8-day: breakout within N days of condition 3", 4.0, 1.0, 8.0, 1.0), tick(), hold(10.0), side()],
        "ats_smtp" => vec![p("n", "N-period low/high and largest range", 10.0, 5.0, 20.0, 1.0), p("loc", "Close in top/bottom % of range", 25.0, 10.0, 40.0, 5.0),
            p("days", "Entry valid N days", 2.0, 1.0, 3.0, 1.0), tick(), hold(10.0), side()],
        "ats_cbp" => vec![p("adx_n", "Weekly ADX period", 14.0, 7.0, 21.0, 1.0), p("adx", "Weekly ADX reaches", 60.0, 40.0, 70.0, 5.0),
            p("min_price", "Price above", 20.0, 0.0, 100.0, 5.0), p("stop", "Stop % above entry", 15.0, 5.0, 25.0, 1.0), p("target", "Cover (half) at % profit", 20.0, 10.0, 40.0, 5.0),
            p("half", "Cover half at the target and trail the rest (1, book) or cover all (0, old vault reading)", 1.0, 0.0, 1.0, 1.0),
            p("trail_n", "[Not from the book] trail the other half at the prior N-day high", 20.0, 5.0, 60.0, 5.0),
            p("once", "First lower ADX reading only, re-armed under the level (1, book) or every downtick (0)", 1.0, 0.0, 1.0, 1.0)],
        "ats_dvmtm" => vec![p("high_n", "Three-month high: N trading days", 63.0, 40.0, 126.0, 1.0), p("near_pct", "Within % of the high", 0.0, 0.0, 3.0, 0.5),
            p("vol_n", "Average volume days", 15.0, 10.0, 30.0, 1.0), p("vol_x", "Volume multiple", 2.0, 1.5, 3.0, 0.25), tick(), hold(5.0)],
        "ats_15min_adx_breakout" => vec![p("adx_n", "ADX/DI period", 14.0, 7.0, 21.0, 1.0), p("adx", "ADX above", 30.0, 20.0, 45.0, 5.0), tick(), losses(), side()],
        "ats_15min_adx_breakout_equities" => vec![p("adx_n", "ADX/DI period", 14.0, 7.0, 21.0, 1.0), p("adx", "ADX above", 30.0, 20.0, 45.0, 5.0), tick(), losses(),
            p("min_price", "Price above", 40.0, 0.0, 200.0, 5.0), p("min_hv", "HV(100) at or above", 40.0, 10.0, 60.0, 5.0), hvr(), side()],
        "ats_10pct_oops" => vec![p("loc", "Close in bottom/top % of range", 10.0, 5.0, 25.0, 5.0), p("gap", "Gap beyond day-1 low/high (1) or close (0)", 1.0, 0.0, 1.0, 1.0),
            p("stop", "Stop at the day-2 open (1) or none (0, book test)", 0.0, 0.0, 1.0, 1.0), side()],
        "ats_wide_range_gap" => vec![p("sd", "Move of at least N standard deviations", 2.0, 1.0, 3.0, 0.25), p("hv_n", "HV period", 100.0, 50.0, 150.0, 10.0),
            p("sd_days", "Days in the SD divisor (sqrt)", 256.0, 252.0, 265.0, 1.0), p("gap", "Gap beyond day-1 low/high (1) or close (0)", 1.0, 0.0, 1.0, 1.0),
            p("carry", "Carry overnight if close in top/bottom % of range", 15.0, 5.0, 25.0, 5.0), hvr(), side()],
        "ats_gipsons" => vec![p("adx_n", "ADX/DI period", 14.0, 7.0, 21.0, 1.0), p("adx", "ADX above", 25.0, 15.0, 45.0, 5.0), p("ratio", "HV6/HV100 below", 0.4, 0.2, 0.6, 0.05),
            p("short", "Short HV period", 6.0, 3.0, 10.0, 1.0), p("long", "Long HV period", 100.0, 50.0, 150.0, 10.0), tick(), hvr(), side(),
            p("exit_all", "Exit (c): half off + trail the rest (0, book) or exit all (1, simplified)", 0.0, 0.0, 1.0, 1.0),
            p("trail_n", "[Not from the book] 'tighten the stop' on the other half = prior N-day low/high", 3.0, 1.0, 10.0, 1.0)],
        "ats_after_losses" => vec![p("base", "Base CVR (1, 2 or 3)", 1.0, 1.0, 3.0, 1.0), p("k", "Consecutive losses before trading", 2.0, 1.0, 4.0, 1.0)],
        _ => return None,
    })
}

// ---------------------------------------------------------------- indicators (local)

/// Book Appendix HV: unbiased sd of the N-1 most recent daily log returns x sqrt(260) x 100 (n_minus_1 = true);
/// with false, N returns (the usual convention).
pub fn book_hv(c: &[f64], n: usize, n_minus_1: bool) -> Vec<f64> {
    let m = if n_minus_1 { n - 1 } else { n };
    let mut out = vec![f64::NAN; c.len()];
    if m < 2 {
        return out;
    }
    for i in m..c.len() {
        let w: Vec<f64> = (i + 1 - m..=i).map(|k| (c[k] / c[k - 1]).ln()).collect();
        let mean = w.iter().sum::<f64>() / m as f64;
        let var = w.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (m as f64 - 1.0);
        out[i] = var.sqrt() * 260f64.sqrt() * 100.0;
    }
    out
}

/// Wilder (+DI, -DI, ADX).
fn dmi3(r: &[Row], n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let (pdi, adx) = dmi(r, n);
    let mir: Vec<Row> = r.iter().map(|x| Row { ticker: String::new(), date: String::new(), open: -x.open, high: -x.low, low: -x.high, close: -x.close, volume: None }).collect();
    (pdi, dmi(&mir, n).0, adx)
}

fn roll(v: &[f64], n: usize, f: fn(f64, f64) -> f64) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 >= n { v[i + 1 - n..=i].iter().copied().reduce(f).unwrap() } else { f64::NAN }).collect()
}

static VIX: OnceLock<Vec<Row>> = OnceLock::new();

/// Raw VIX bars from the server's VIX directory (3rd CLI argument), aligned to x's bars by date:
/// returns the VIX rows and, per bar of x, the VIX row index.
fn vix_rows(x: &Ctx) -> (&'static [Row], Vec<Option<usize>>) {
    let v = VIX.get_or_init(|| std::env::args().nth(3).map(|d| crate::load_vix_dir(&d)).unwrap_or_default());
    let pos: std::collections::HashMap<&str, usize> = v.iter().enumerate().map(|(i, r)| (r.date.as_str(), i)).collect();
    (v, x.r.iter().map(|r| pos.get(r.date.as_str()).copied()).collect())
}

fn align(at: &[Option<usize>], v: &[f64]) -> Vec<f64> {
    at.iter().map(|k| k.map_or(f64::NAN, |k| v[k])).collect()
}

// ---------------------------------------------------------------- simulator

#[derive(Clone, Copy)]
enum Kind {
    Close,
    /// stop entry order for the next `bars` bars at `px`
    Stop { px: f64, bars: usize },
}
#[derive(Clone, Copy)]
struct Ord {
    dir: f64,
    kind: Kind,
    stop: f64, // protective stop price (NaN = none)
}
fn at_close(dir: f64) -> Option<Ord> {
    Some(Ord { dir, kind: Kind::Close, stop: f64::NAN })
}
enum At {
    Close,
    NextOpen,
    Px(f64),
    /// take half the position off on this close (the rest stays on its stop / trail)
    Half,
}
struct Pos {
    dir: f64,
    i0: usize,
    px: f64,
    stop: f64,
    at_open: bool, // filled at the bar's open (the whole bar is after the fill)
    count_i: usize,
    kind: &'static str,
    risk: f64,               // |entry - initial stop|, 0 = no stop
    half: Option<(usize, f64)>, // (bar, price) of the half already taken off
    why_half: String,
}
impl Pos {
    fn new(dir: f64, i0: usize, px: f64, stop: f64, at_open: bool, kind: &'static str) -> Pos {
        Pos { dir, i0, px, stop, at_open, count_i: i0, kind, risk: if stop.is_finite() { (px - stop).abs() } else { 0.0 }, half: None, why_half: String::new() }
    }
}
/// Half exits and trailing stops. Half exits are folded into one Trade: exit_px = 0.5 * half price + 0.5 * rest price
/// (the vault convention, as btp.rs / cwin.rs).
#[derive(Clone, Copy, Default)]
struct Mm {
    half_r: bool,     // Ch 28 Two-for-One: half off at entry +/- initial risk
    half_pct: f64,    // half off at this % profit (CBP 20), 0 = off
    trail: usize,     // trailing stop at the prior N-bar low (long) / high (short), ratchets; 0 = off
    after_half: bool, // the trail starts only after the half is off
}
const MM0: Mm = Mm { half_r: false, half_pct: 0.0, trail: 0, after_half: false };
struct Rules<'a> {
    sig: &'a dyn Fn(usize) -> Option<Ord>,
    /// intraday entry on bar i while flat: (dir, fill price, stop, filled at open)
    touch: Option<&'a dyn Fn(usize) -> Option<(f64, f64, f64, bool)>>,
    exit: &'a dyn Fn(usize, &Pos) -> Option<(At, String)>,
    reverse: bool,
    extend: bool,
    stop_pct: f64, // protective stop % from a close entry (0 = none)
    mm: Mm,
}

fn trade(p: &Pos, i: usize, px: f64, why: String, sig_i: usize) -> Trade {
    let exit_px = p.half.map_or(px, |(_, hp)| 0.5 * hp + 0.5 * px);
    let exits = p.half.map_or(Vec::new(), |(hi, hp)| vec![(hi, hp, 0.5), (i, px, 0.5)]);
    let reason = if p.half.is_some() { format!("{}; rest: {why}", p.why_half) } else { why };
    Trade { dir: p.dir, legs: vec![Leg { i: p.i0, px: p.px, w: 1.0 }], exit_i: i, exit_px, reason, entry_kind: p.kind, signal_i: sig_i, open: false, exits }
}

fn sim(x: &Ctx, from: usize, to: usize, r: &Rules) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = Vec::new();
    let mut pos: Option<Pos> = None;
    let mut pend: Option<(Ord, usize)> = None; // order + last valid bar
    let mut exit_open: Option<String> = None;
    for i in from..=to {
        if let Some(why) = exit_open.take() {
            let p = pos.take().unwrap();
            out.push(trade(&p, i, o[i], why, i - 1));
        }
        if pos.is_none() {
            if let Some((ord, until)) = pend.take() {
                if i <= until {
                    if let Kind::Stop { px, .. } = ord.kind {
                        let fill = if ord.dir > 0.0 { (h[i] >= px).then(|| o[i].max(px)) } else { (l[i] <= px).then(|| o[i].min(px)) };
                        match fill {
                            Some(f) => pos = Some(Pos::new(ord.dir, i, f, ord.stop, f == o[i], "stop")),
                            None => pend = Some((ord, until)),
                        }
                    }
                }
            }
            if pos.is_none() {
                if let Some((dir, px, stop, at_open)) = r.touch.and_then(|t| t(i)) {
                    pos = Some(Pos::new(dir, i, px, stop, at_open, "stop"));
                    pend = None;
                }
            }
        }
        if let Some(p) = pos.as_mut() {
            let mm = r.mm;
            let long = p.dir > 0.0;
            if mm.trail > 0 && i > p.i0 && i >= mm.trail && (!mm.after_half || p.half.is_some()) {
                let w = &(if long { l } else { h })[i - mm.trail..i];
                let lv = if long { w.iter().copied().fold(f64::INFINITY, f64::min) } else { w.iter().copied().fold(f64::NEG_INFINITY, f64::max) };
                p.stop = if long { p.stop.max(lv) } else { p.stop.min(lv) }; // NaN stop -> lv
            }
            // protective stop first (a stop and a target on one bar = stop)
            let s = p.stop;
            let hit = if p.dir > 0.0 { l[i] <= s } else { h[i] >= s };
            let after_fill = i > p.i0 || p.at_open || (if p.dir > 0.0 { c[i] <= s } else { c[i] >= s });
            if s.is_finite() && hit && after_fill {
                let px = if i > p.i0 { if p.dir > 0.0 { o[i].min(s) } else { o[i].max(s) } } else { s };
                out.push(trade(p, i, px, "protective stop".into(), i));
                pos = None;
            } else {
                // half off at a price (stop before target on one bar, so only when the stop did not hit)
                if p.half.is_none() {
                    let tg = if mm.half_r && p.risk > 0.0 {
                        Some((p.px + p.dir * p.risk, "half off at +1R (2-for-1)".to_string()))
                    } else if mm.half_pct > 0.0 {
                        Some((p.px * (1.0 + p.dir * mm.half_pct / 100.0), format!("half off at {}% profit", mm.half_pct)))
                    } else {
                        None
                    };
                    if let Some((tg, why)) = tg {
                        if if long { h[i] >= tg } else { l[i] <= tg } {
                            p.half = Some((i, if i == p.i0 { tg } else if long { o[i].max(tg) } else { o[i].min(tg) }));
                            p.why_half = why;
                        }
                    }
                }
                // TradeStation nets a close exit and a same-side close entry on one bar: the position just continues
                let renew = r.extend && i < to && (r.sig)(i).map_or(false, |o| matches!(o.kind, Kind::Close) && o.dir == p.dir);
                match (r.exit)(i, p) {
                    Some((At::Close, _)) if renew => p.count_i = i,
                    Some((At::Close, why)) => {
                        out.push(trade(p, i, c[i], why, i));
                        pos = None;
                    }
                    Some((At::Px(v), why)) => {
                        out.push(trade(p, i, v, why, i));
                        pos = None;
                    }
                    Some((At::Half, why)) => {
                        if p.half.is_none() {
                            p.half = Some((i, c[i]));
                            p.why_half = why;
                        }
                    }
                    Some((At::NextOpen, why)) => {
                        if i < to {
                            exit_open = Some(why);
                        }
                    }
                    None => {}
                }
            }
        }
        if i == to || exit_open.is_some() {
            continue;
        }
        let Some(ord) = (r.sig)(i) else { continue };
        if let Some(p) = pos.as_mut() {
            if !matches!(ord.kind, Kind::Close) {
                continue;
            }
            if ord.dir == p.dir {
                continue;
            }
            if !r.reverse {
                continue;
            }
            out.push(trade(p, i, c[i], "opposite signal (reverse)".into(), i));
            pos = None;
        }
        match ord.kind {
            Kind::Close => {
                let stop = if r.stop_pct > 0.0 { c[i] * (1.0 - ord.dir * r.stop_pct / 100.0) } else { ord.stop };
                pos = Some(Pos::new(ord.dir, i, c[i], stop, true, "close"));
                pend = None;
            }
            Kind::Stop { bars, .. } => pend = Some((ord, i + bars)),
        }
    }
    if let Some(p) = pos {
        let mut t = trade(&p, to, c[to], "open".into(), to);
        t.open = true;
        out.push(t);
    }
    out
}

fn hold_exit(n: usize) -> impl Fn(usize, &Pos) -> Option<(At, String)> {
    move |i, p| (i >= p.count_i + n).then(|| (At::Close, format!("{n} bars held")))
}

// ---------------------------------------------------------------- strategies

fn cvr(k: u8, x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let (rows, at0) = vix_rows(x);
    // [Not from the book] calendar alignment: keep only VIX rows on the traded ticker's dates (drops CBOE holiday rows),
    // compute the VIX indicators on that sequence, and read "yesterday" as the previous VIX bar (a missing VIX day is skipped,
    // not NaN).
    // VIX history before the ticker's first bar is kept as indicator warm-up.
    let d0 = x.r.first().map_or(i64::MAX, |r| crate::day_number(&r.date));
    let mut keep: Vec<usize> = (0..rows.len()).take_while(|&k| crate::day_number(&rows[k].date) < d0).collect();
    let at: Vec<Option<usize>> = at0.iter().map(|a| a.map(|k| { keep.push(k); keep.len() - 1 })).collect();
    let col = |f: fn(&Row) -> f64| -> Vec<f64> { keep.iter().map(|&k| f(&rows[k])).collect() };
    let prev = |v: &[f64]| -> Vec<f64> { (0..v.len()).map(|k| if k > 0 { v[k - 1] } else { f64::NAN }).collect() };
    let (vo, vh, vl, vc) = (col(|r| r.open), col(|r| r.high), col(|r| r.low), col(|r| r.close));
    let (o, h, l, c) = (align(&at, &vo), align(&at, &vh), align(&at, &vl), align(&at, &vc));
    let side = g("side");
    let ok = move |d: f64| side == 0.0 || side == d;
    let sig: Box<dyn Fn(usize) -> Option<Ord>>;
    let exit: Box<dyn Fn(usize, &Pos) -> Option<(At, String)>>;
    let mut extend = false;
    match k {
        1 => {
            let n = g("n") as usize;
            let (hh, ll) = (align(&at, &roll(&vh, n, f64::max)), align(&at, &roll(&vl, n, f64::min)));
            sig = Box::new(move |i| {
                if ok(1.0) && h[i] >= hh[i] && c[i] < o[i] {
                    at_close(1.0)
                } else if ok(-1.0) && l[i] <= ll[i] && c[i] > o[i] {
                    at_close(-1.0)
                } else {
                    None
                }
            });
            exit = Box::new(hold_exit(g("hold") as usize));
            extend = g("extend") > 0.5;
        }
        2 => {
            let rv = rsi(&vc, g("rsi_n") as usize);
            let (r, rp) = (align(&at, &rv), align(&at, &prev(&rv)));
            let (ob, os) = (g("ob"), g("os"));
            sig = Box::new(move |i| {
                if ok(1.0) && rp[i] >= ob && r[i] < rp[i] {
                    at_close(1.0)
                } else if ok(-1.0) && rp[i] <= os && r[i] > rp[i] {
                    at_close(-1.0)
                } else {
                    None
                }
            });
            exit = Box::new(hold_exit(g("hold") as usize));
            extend = g("extend") > 0.5;
        }
        _ => {
            let mv = sma(&vc, g("ma") as usize);
            let ma1 = align(&at, &prev(&mv)); // yesterday's MA (previous VIX bar)
            let t = g("pct") / 100.0;
            // entry SMA: yesterday's 10-day MA (lag 1) reproduces 71-73 of the 75 appendix entries on VXO, today's only 53-55
            let ma = if g("ma_lag") > 0.5 { ma1.clone() } else { align(&at, &mv) };
            sig = Box::new(move |i| {
                if ok(1.0) && l[i] > ma[i] && c[i] >= ma[i] * (1.0 + t) {
                    at_close(1.0)
                } else if ok(-1.0) && h[i] < ma[i] && c[i] <= ma[i] * (1.0 - t) {
                    at_close(-1.0)
                } else {
                    None
                }
            });
            let (h2, l2) = (align(&at, &vh), align(&at, &vl));
            exit = Box::new(move |i, p| {
                let back = if p.dir > 0.0 { l2[i] < ma1[i] } else { h2[i] > ma1[i] };
                back.then(|| (At::Close, "VIX traded back through yesterday's 10-day MA".to_string()))
            });
        }
    }
    sim(x, from, to, &Rules { sig: &*sig, touch: None, exit: &*exit, reverse: g("reverse") > 0.5, extend, stop_pct: 0.0, mm: MM0 })
}

/// Ch 28 Two-for-One on a base pattern (param two_for_one = 1): half off at +1x initial risk, the rest on the original stop
/// and the base exit.
fn two4one(g: &dyn Fn(&str) -> f64) -> Mm {
    Mm { half_r: g("two_for_one") > 0.5, ..MM0 }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id == "ats_two_for_one" {
        // the overlay on a base pattern: base 1 = 1-2-3-4 (the book's PROX/TOM examples), 2 = 8-day high/low reversal (Ch 9 p.76)
        let base = if p.get("base").copied().unwrap_or(1.0) >= 1.5 { "ats_8day_hl_reversal" } else { "ats_1234" };
        let mut q = p.clone();
        q.insert("two_for_one".into(), 1.0);
        return run(base, x, from, to, &q);
    }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let side = p.get("side").copied().or_else(|| schema.iter().find(|d| d.key == "side").map(|d| d.def)).unwrap_or(0.0);
    let ok = |d: f64| side == 0.0 || side == d;
    let tick = || g("tick");
    let hvr = || g("hv_rets") > 0.5;
    let rng: Vec<f64> = (0..n).map(|i| h[i] - l[i]).collect();
    let no_touch: Option<&dyn Fn(usize) -> Option<(f64, f64, f64, bool)>> = None;
    let trades = match id {
        "ats_cvr1" => cvr(1, x, from, to, &g),
        "ats_cvr2" => cvr(2, x, from, to, &g),
        "ats_cvr3" => cvr(3, x, from, to, &g),
        "ats_after_losses" => {
            let b = u("base").clamp(1, 3);
            let bs = params(["ats_cvr1", "ats_cvr2", "ats_cvr3"][b - 1]).unwrap();
            let gb = |k: &str| bs.iter().find(|d| d.key == k).unwrap().def;
            let all = cvr(b as u8, x, from, to, &gb);
            let k = u("k");
            all.iter().enumerate().filter(|(j, _)| *j >= k && all[j - k..*j].iter().all(|t| t.ret(0.0) <= 0.0)).map(|(_, t)| t.clone()).collect()
        }
        "ats_trin_thrust" | "ats_trin_thrust_nse" => {
            let Some(tr) = x.series(if id == "ats_trin_thrust" { "NYSE_TRIN" } else { "NSE_TRIN" }) else { return Some(vec![]) };
            let th = g("thrust") / 100.0;
            let sig = |i: usize| {
                if i == 0 {
                    None
                } else if ok(1.0) && tr[i] <= tr[i - 1] * (1.0 - th) {
                    at_close(1.0)
                } else if ok(-1.0) && tr[i] >= tr[i - 1] * (1.0 + th) {
                    at_close(-1.0)
                } else {
                    None
                }
            };
            let ex = hold_exit(u("hold"));
            sim(x, from, to, &Rules { sig: &sig, touch: no_touch, exit: &ex, reverse: g("reverse") > 0.5, extend: false, stop_pct: g("stop"), mm: MM0 })
        }
        "ats_padi" | "ats_padi_nse" => {
            let pre = if id == "ats_padi" { "NYSE_ADV_DEC" } else { "NSE_ADV_DEC" };
            let (Some(a), Some(d), Some(un)) = (x.series(&format!("{pre}.Advances")), x.series(&format!("{pre}.Declines")), x.series(&format!("{pre}.Unchanged"))) else { return Some(vec![]) };
            let tot: Vec<f64> = (0..n).map(|i| a[i] + d[i] + un[i]).collect();
            let m = u("avg");
            // windowed mean: a missing breadth day blanks only the windows that contain it (the shared running-sum sma never recovers from a NaN)
            let wma = |v: Vec<f64>| -> Vec<f64> { (0..n).map(|i| if i + 1 < m { f64::NAN } else { v[i + 1 - m..=i].iter().sum::<f64>() / m as f64 }).collect() };
            let ap = wma((0..n).map(|i| a[i] / tot[i]).collect());
            let dp = wma((0..n).map(|i| d[i] / tot[i]).collect());
            let (lv, ma) = (g("level") / 100.0, x.sma_n(u("trend")));
            let sig = |i: usize| {
                if i == 0 {
                    None
                } else if ok(-1.0) && ap[i - 1] >= lv && ap[i] < ap[i - 1] && c[i] < ma[i] {
                    at_close(-1.0)
                } else if ok(1.0) && dp[i - 1] >= lv && dp[i] < dp[i - 1] && c[i] > ma[i] {
                    at_close(1.0)
                } else {
                    None
                }
            };
            let ex = hold_exit(u("hold"));
            sim(x, from, to, &Rules { sig: &sig, touch: no_touch, exit: &ex, reverse: true, extend: false, stop_pct: g("stop"), mm: MM0 })
        }
        "ats_hv_thrust" => {
            let (a, b, lg) = (book_hv(c, u("short1"), hvr()), book_hv(c, u("short2"), hvr()), book_hv(c, u("long"), hvr()));
            let (rt, rn, lag) = (g("ratio"), u("range_n"), if g("when") > 0.5 { 1 } else { 0 });
            let sig = |i: usize| {
                if i < rn + 1 {
                    return None;
                }
                let k = i - lag;
                let low_vol = a[k] / lg[k] < rt || b[k] / lg[k] < rt;
                let wide = rng[i] > rng[i - rn..i].iter().copied().fold(f64::MIN, f64::max);
                if !(low_vol && wide) {
                    return None;
                }
                let d = if c[i] > c[i - 1] { 1.0 } else if c[i] < c[i - 1] { -1.0 } else { 0.0 };
                (d != 0.0 && ok(d)).then(|| Ord { dir: d, kind: Kind::Close, stop: f64::NAN })
            };
            let ex = hold_exit(u("hold"));
            sim(x, from, to, &Rules { sig: &sig, touch: no_touch, exit: &ex, reverse: false, extend: false, stop_pct: 0.0, mm: MM0 })
        }
        "ats_1234" => {
            let (pdi, mdi, adx) = dmi3(x.r, u("adx_n"));
            let (lim, t) = (g("adx"), tick());
            let sig = |i: usize| {
                if i < 3 || !(adx[i] > lim) {
                    return None;
                }
                let ins = |k: usize| h[k] <= h[k - 1] && l[k] >= l[k - 1];
                let ll = |k: usize| l[k] < l[k - 1];
                let hh = |k: usize| h[k] > h[k - 1];
                let days = i - 2..=i;
                if ok(1.0) && pdi[i] > mdi[i] && days.clone().all(|k| ll(k) || ins(k)) && days.clone().filter(|&k| ll(k)).count() >= 2 {
                    return Some(Ord { dir: 1.0, kind: Kind::Stop { px: h[i] + t, bars: 1 }, stop: l[i] - t });
                }
                if ok(-1.0) && mdi[i] > pdi[i] && days.clone().all(|k| hh(k) || ins(k)) && days.filter(|&k| hh(k)).count() >= 2 {
                    return Some(Ord { dir: -1.0, kind: Kind::Stop { px: l[i] - t, bars: 1 }, stop: h[i] + t });
                }
                None
            };
            let ex = hold_exit(u("hold"));
            sim(x, from, to, &Rules { sig: &sig, touch: no_touch, exit: &ex, reverse: false, extend: false, stop_pct: 0.0, mm: two4one(&g) })
        }
        "ats_8day_hl_reversal" => {
            let (nn, c3, win, t) = (u("n"), u("c3_days"), u("window"), tick());
            // For bar i: the most recent day one j whose condition 3 fired at k (day 3..day 3+c3-1) with k <= i <= k+win.
            let setup = |i: usize, dir: f64| -> Option<(f64, f64, usize)> {
                for j in (i.saturating_sub(c3 + win + 1)..i.saturating_sub(1)).rev() {
                    if j + 1 < nn {
                        continue;
                    }
                    let d2 = j + 1;
                    let (day1, day2) = if dir > 0.0 { (x.lowest_low(j, nn), h[d2] > h[j]) } else { (x.highest_high(j, nn), l[d2] < l[j]) };
                    if !(day1 && day2) {
                        continue;
                    }
                    let k = (d2 + 1..=(d2 + c3).min(i)).find(|&k| if dir > 0.0 { l[k] < l[d2] } else { h[k] > h[d2] });
                    if let Some(k) = k {
                        if i <= k + win {
                            return Some(if dir > 0.0 { (h[d2] + t, l[d2] - t, k) } else { (l[d2] - t, h[d2] + t, k) });
                        }
                    }
                }
                None
            };
            let touch = |i: usize| {
                for d in [1.0, -1.0] {
                    if !ok(d) {
                        continue;
                    }
                    if let Some((lvl, stop, k)) = setup(i, d) {
                        if if d > 0.0 { h[i] >= lvl } else { l[i] <= lvl } {
                            let gap = i > k && if d > 0.0 { o[i] > lvl } else { o[i] < lvl };
                            return Some((d, if gap { o[i] } else { lvl }, stop, gap));
                        }
                    }
                }
                None
            };
            let ex = hold_exit(u("hold"));
            sim(x, from, to, &Rules { sig: &|_| None, touch: Some(&touch), exit: &ex, reverse: false, extend: false, stop_pct: 0.0, mm: two4one(&g) })
        }
        "ats_smtp" => {
            let (nn, loc, days, t) = (u("n"), g("loc") / 100.0, u("days"), tick());
            let sig = |i: usize| {
                if i + 1 < nn || !rng[i + 1 - nn..=i].iter().all(|&r| rng[i] >= r) {
                    return None;
                }
                if ok(1.0) && x.lowest_low(i, nn) && x.close_loc(i) >= 1.0 - loc {
                    return Some(Ord { dir: 1.0, kind: Kind::Stop { px: h[i] + t, bars: days }, stop: l[i] });
                }
                if ok(-1.0) && x.highest_high(i, nn) && x.close_loc(i) <= loc {
                    return Some(Ord { dir: -1.0, kind: Kind::Stop { px: l[i] - t, bars: days }, stop: h[i] });
                }
                None
            };
            let ex = hold_exit(u("hold"));
            sim(x, from, to, &Rules { sig: &sig, touch: no_touch, exit: &ex, reverse: false, extend: false, stop_pct: 0.0, mm: MM0 })
        }
        "ats_cbp" => {
            // weekly bars (Monday-based weeks) resampled from the daily bars
            let wk: Vec<i64> = x.r.iter().map(|r| (crate::day_number(&r.date) + 3).div_euclid(7)).collect();
            let mut wrows: Vec<Row> = Vec::new();
            let mut widx = vec![0usize; n];
            for i in 0..n {
                if i == 0 || wk[i] != wk[i - 1] {
                    wrows.push(Row { ticker: String::new(), date: x.r[i].date.clone(), open: o[i], high: h[i], low: l[i], close: c[i], volume: None });
                } else {
                    let w = wrows.last_mut().unwrap();
                    w.high = w.high.max(h[i]);
                    w.low = w.low.min(l[i]);
                    w.close = c[i];
                }
                widx[i] = wrows.len() - 1;
            }
            let (_, wadx) = dmi(&wrows, u("adx_n"));
            let (lim, minp, stp, tgt, tg) = (g("adx"), g("min_price"), g("stop") / 100.0, g("target") / 100.0, g("target"));
            // "after the weekly ADX reaches 60, wait for the FIRST lower reading": one signal per excursion; re-armed once the
            // ADX is back under the level (as btp_cbp_short). once=0 = every downtick while the prior week is >= the level.
            let once = g("once") > 0.5;
            let mut fire = vec![false; wrows.len()];
            let mut armed = true;
            for w in 1..wrows.len() {
                if wadx[w] < lim {
                    armed = true;
                }
                if (armed || !once) && wadx[w - 1] >= lim && wadx[w] < wadx[w - 1] {
                    fire[w] = true;
                    armed = false;
                }
            }
            let sig = |i: usize| {
                let w = widx[i];
                let week_end = i + 1 < n && wk[i + 1] != wk[i];
                (week_end && fire[w] && c[i] > minp).then(|| Ord { dir: -1.0, kind: Kind::Close, stop: c[i] * (1.0 + stp) })
            };
            let half = g("half") > 0.5;
            let ex = |i: usize, p: &Pos| {
                let t = p.px * (1.0 - tgt);
                (!half && l[i] <= t).then(|| (At::Px(o[i].min(t)), format!("{tg}% profit")))
            };
            let mm = if half { Mm { half_pct: tg, trail: u("trail_n"), after_half: true, ..MM0 } } else { MM0 };
            sim(x, from, to, &Rules { sig: &sig, touch: no_touch, exit: &ex, reverse: false, extend: false, stop_pct: 0.0, mm })
        }
        "ats_dvmtm" => {
            if !x.has_volume {
                return Some(vec![]);
            }
            let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
            let (hn, near, vn, vm, t) = (u("high_n"), g("near_pct") / 100.0, u("vol_n"), g("vol_x"), tick());
            let vday = |v: usize| {
                v >= hn.max(vn)
                    && h[v] >= (1.0 - near) * h[v + 1 - hn..=v].iter().copied().fold(f64::MIN, f64::max)
                    && vol[v] >= vm * vol[v - vn..v].iter().sum::<f64>() / vn as f64
            };
            let sig = |r: usize| {
                // r = first close-below-open day among the volume day v, v+1, v+2
                let hit = c[r] < o[r] && (r.saturating_sub(2)..=r).any(|v| vday(v) && (v..r).all(|k| c[k] >= o[k]));
                hit.then(|| Ord { dir: -1.0, kind: Kind::Stop { px: l[r] - t, bars: 2 }, stop: h[r] })
            };
            let ex = hold_exit(u("hold"));
            sim(x, from, to, &Rules { sig: &sig, touch: no_touch, exit: &ex, reverse: false, extend: false, stop_pct: 0.0, mm: MM0 })
        }
        "ats_15min_adx_breakout" | "ats_15min_adx_breakout_equities" => {
            // missing series = no trades (the API reports the data gap before calling run)
            let s = |k: &str| x.series(k);
            let (Some(oh), Some(ol), Some(lf), Some(ls), Some(sf), Some(ss)) = (s("OPEN15_HIGH"), s("OPEN15_LOW"), s("LONG_FILLS"), s("LONG_STOPS"), s("SHORT_FILLS"), s("SHORT_STOPS")) else { return Some(vec![]) };
            let cap = u("max_losses");
            let (pdi, mdi, adx) = dmi3(x.r, u("adx_n"));
            let (lim, t) = (g("adx"), tick());
            let eq = id.ends_with("equities");
            let hv = if eq { book_hv(c, 100, hvr()) } else { vec![] };
            let mut out = Vec::new();
            for i in from.max(1)..=to {
                let k = i - 1;
                if !(adx[k] > lim) || (eq && !(c[k] > g("min_price") && hv[k] >= g("min_hv"))) {
                    continue;
                }
                let (dir, entry, stop, fills, stops) = if pdi[k] > mdi[k] {
                    (1.0, oh[i] + t, ol[i] - t, lf[i], ls[i])
                } else if mdi[k] > pdi[k] {
                    (-1.0, ol[i] - t, oh[i] + t, sf[i], ss[i])
                } else {
                    continue;
                };
                if !ok(dir) || !entry.is_finite() || !stop.is_finite() || fills.is_nan() || stops.is_nan() {
                    continue;
                }
                // fill k (0-based) was stopped iff k < stops; trading stops after `cap` entries ("Two losses are my limit")
                for e in 0..(fills.max(0.0) as usize).min(cap) {
                    let (px, why) = if e < stops.max(0.0) as usize { (stop, "protective stop") } else { (c[i], "MOC") };
                    let why = if e > 0 { format!("re-entry: {why}") } else { why.to_string() };
                    out.push(Trade { dir, legs: vec![Leg { i, px: entry, w: 1.0 }], exit_i: i, exit_px: px, reason: why, entry_kind: "stop", signal_i: i, open: false, exits: Vec::new() });
                }
            }
            out
        }
        "ats_10pct_oops" => {
            let (loc, gap_low, stop_on) = (g("loc") / 100.0, g("gap") > 0.5, g("stop") > 0.5);
            let touch = |i: usize| {
                if i == 0 {
                    return None;
                }
                let k = i - 1;
                let cl = x.close_loc(k);
                if ok(1.0) && cl <= loc && o[i] < if gap_low { l[k] } else { c[k] } && h[i] >= l[k] {
                    return Some((1.0, l[k].max(o[i]), if stop_on { o[i] } else { f64::NAN }, false));
                }
                if ok(-1.0) && cl >= 1.0 - loc && o[i] > if gap_low { h[k] } else { c[k] } && l[i] <= h[k] {
                    return Some((-1.0, h[k].min(o[i]), if stop_on { o[i] } else { f64::NAN }, false));
                }
                None
            };
            let ex = |_: usize, _: &Pos| Some((At::Close, "MOC".to_string()));
            sim(x, from, to, &Rules { sig: &|_| None, touch: Some(&touch), exit: &ex, reverse: false, extend: false, stop_pct: 0.0, mm: MM0 })
        }
        "ats_wide_range_gap" => {
            let hv = book_hv(c, u("hv_n"), hvr());
            let (k_sd, div, gap_low, carry) = (g("sd"), g("sd_days").sqrt(), g("gap") > 0.5, g("carry") / 100.0);
            let touch = |i: usize| {
                if i < 2 {
                    return None;
                }
                let k = i - 1;
                let sd = c[k - 1] * hv[k - 1] / 100.0 / div;
                let mv = c[k] - c[k - 1];
                if ok(1.0) && mv <= -k_sd * sd && o[i] < if gap_low { l[k] } else { c[k] } && h[i] >= c[k] {
                    return Some((1.0, c[k].max(o[i]), l[k], false));
                }
                if ok(-1.0) && mv >= k_sd * sd && o[i] > if gap_low { h[k] } else { c[k] } && l[i] <= c[k] {
                    return Some((-1.0, c[k].min(o[i]), h[k], false));
                }
                None
            };
            let ex = |i: usize, p: &Pos| {
                let strong = if p.dir > 0.0 { x.close_loc(i) >= 1.0 - carry } else { x.close_loc(i) <= carry };
                Some(if strong { (At::NextOpen, "closed in the extreme of its range: exit next morning".to_string()) } else { (At::Close, "locked in near the close".to_string()) })
            };
            sim(x, from, to, &Rules { sig: &|_| None, touch: Some(&touch), exit: &ex, reverse: false, extend: false, stop_pct: 0.0, mm: MM0 })
        }
        "ats_gipsons" => {
            let (pdi, mdi, adx) = dmi3(x.r, u("adx_n"));
            let (s6, s100) = (book_hv(c, u("short"), hvr()), book_hv(c, u("long"), hvr()));
            let (lim, rt, t) = (g("adx"), g("ratio"), tick());
            let sig = |i: usize| {
                if !(adx[i] > lim && s6[i] / s100[i] < rt) {
                    return None;
                }
                if ok(1.0) && pdi[i] > mdi[i] {
                    Some(Ord { dir: 1.0, kind: Kind::Stop { px: h[i] + t, bars: 1 }, stop: l[i] })
                } else if ok(-1.0) && mdi[i] > pdi[i] {
                    Some(Ord { dir: -1.0, kind: Kind::Stop { px: l[i] - t, bars: 1 }, stop: h[i] })
                } else {
                    None
                }
            };
            let all = g("exit_all") > 0.5;
            let ex = |i: usize, p: &Pos| {
                (s6[i] >= s100[i] && p.half.is_none())
                    .then(|| if all { (At::Close, "6-day HV reached the 100-day HV".to_string()) } else { (At::Half, "half off: 6-day HV reached the 100-day HV".to_string()) })
            };
            let mm = if all { MM0 } else { Mm { trail: u("trail_n"), after_half: true, ..MM0 } };
            sim(x, from, to, &Rules { sig: &sig, touch: no_touch, exit: &ex, reverse: false, extend: false, stop_pct: 0.0, mm })
        }
        _ => return None,
    };
    Some(trades)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ats_cvr1" => ("ats_cvr", "Connors VIX Reversals (ATS)", "CVR I · 15-day VIX high/low reversal"),
        "ats_cvr2" => ("ats_cvr", "Connors VIX Reversals (ATS)", "CVR II · VIX RSI(5) 70/30 hook"),
        "ats_cvr3" => ("ats_cvr", "Connors VIX Reversals (ATS)", "CVR III · VIX 10% from its 10-day MA"),
        "ats_cvr_equities" => ("ats_cvr", "Connors VIX Reversals (ATS)", "CVR I on stocks"),
        "ats_cvr_options" => ("ats_cvr", "Connors VIX Reversals (ATS)", "CVR with options"),
        "ats_after_losses" => ("ats_cvr", "Connors VIX Reversals (ATS)", "CVR after consecutive losses"),
        "ats_trin_thrust" => ("ats_trin", "TRIN Thrust", "NYSE TRIN"),
        "ats_trin_thrust_nse" => ("ats_trin", "TRIN Thrust", "NSE TRIN"),
        "ats_padi" => ("ats_padi", "Percent Advance/Decline Indicator (PADI)", "NYSE breadth"),
        "ats_padi_nse" => ("ats_padi", "Percent Advance/Decline Indicator (PADI)", "NSE breadth"),
        "ats_hv_ratio_setup" => ("ats_hv", "Historical Volatility Ratio", "Setup: short/long HV under 50%"),
        "ats_hv_thrust" => ("ats_hv", "Historical Volatility Ratio", "HV Thrust (range expansion)"),
        "ats_hv_ratio_short_strangle" => ("ats_hv_opt", "Trading Volatility with Options", ""),
        "ats_1234" => ("ats_1234", "1-2-3-4", ""),
        "ats_8day_hl_reversal" => ("ats_8day", "8-Day High/Low Reversal", ""),
        "ats_smtp" => ("ats_smtp", "Spent Market Trading Pattern", ""),
        "ats_cbp" => ("ats_cbp", "Crash, Burn and Profit", ""),
        "ats_late_earnings" => ("ats_late", "When They're Late, They're Probably Dead", ""),
        "ats_dvmtm" => ("ats_dvmtm", "Double Volume Market Top Method", ""),
        "ats_torpedoes" => ("ats_torpedoes", "Torpedoes", ""),
        "ats_split_call_selling" => ("ats_split", "Options on Stock Splits", ""),
        "ats_sector_options_overpriced" => ("ats_sector_opt", "Overpriced Sector Options", ""),
        "ats_15min_adx_breakout" => ("ats_adx15", "15-Minute ADX Breakout", "Futures / index"),
        "ats_15min_adx_breakout_equities" => ("ats_adx15", "15-Minute ADX Breakout", "Equities filter"),
        "ats_10pct_oops" => ("ats_oops", "10% OOPS", ""),
        "ats_morning_call_reversal" => ("ats_morning", "Reversals off the Morning Call", ""),
        "ats_wide_range_gap" => ("ats_wrgap", "Wide Range Exhaustion Gap Reversals", ""),
        "ats_flag_within_a_flag" => ("ats_flag", "Flag within a Flag", ""),
        "ats_gipsons" => ("ats_gipsons", "Gipsons", ""),
        "ats_short_term_decline_playbook" => ("ats_decline", "Short-Term Market Declines", ""),
        "ats_large_range_day_combo" => ("ats_lrd", "Large-Range Days", ""),
        "ats_two_for_one" => ("ats_2for1", "Two-for-One Money Management", ""),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(d: usize, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: format!("2020-01-{:02}", d + 1), open: o, high: h, low: l, close: c, volume: Some(1e6) }
    }

    #[test]
    fn adx15_reenters_once_after_a_stop_two_entries_max() {
        // steady uptrend -> ADX > 30, +DI > -DI. Last day: first bar 200-198; the buy stop (200.05) filled 3 times and
        // was stopped 2 times (series counts): book = 2 entries (one re-entry), both stopped at 197.95; max_losses 1 = 1 entry.
        let n = 60;
        let rows: Vec<Row> = (0..n).map(|d| { let p = 100.0 + 2.0 * d as f64; Row { date: format!("2020-{:02}-{:02}", d / 28 + 1, d % 28 + 1), ..row(0, p, p + 1.0, p - 1.0, p + 0.5) } }).collect();
        let mut ctx = Ctx::new(&rows, &rows, &[]);
        let last = |v: f64| { let mut s = vec![f64::NAN; n]; s[n - 1] = v; s };
        for (k, v) in [("OPEN15_HIGH", 200.0), ("OPEN15_LOW", 198.0), ("LONG_FILLS", 3.0), ("LONG_STOPS", 2.0), ("SHORT_FILLS", 0.0), ("SHORT_STOPS", 0.0)] {
            ctx.extra.insert(k.into(), last(v));
        }
        let go = |ctx: &Ctx, cap: f64| { let mut p = Params::new(); p.insert("max_losses".into(), cap); run("ats_15min_adx_breakout", ctx, 0, n - 1, &p).unwrap() };
        let t = go(&ctx, 2.0);
        assert_eq!(t.len(), 2, "{t:?}");
        assert!(t.iter().all(|t| t.dir == 1.0 && (t.legs[0].px - 200.05).abs() < 1e-9 && (t.exit_px - 197.95).abs() < 1e-9));
        assert!(t[1].reason.starts_with("re-entry"));
        assert_eq!(go(&ctx, 1.0).len(), 1);
        // 2 fills, 1 stop: the re-entry holds to the close (MOC)
        ctx.extra.insert("LONG_FILLS".into(), last(2.0));
        ctx.extra.insert("LONG_STOPS".into(), last(1.0));
        let t = go(&ctx, 2.0);
        assert_eq!((t.len(), t[1].exit_px), (2, rows[n - 1].close));
    }

    #[test]
    fn appendix_hv_example() {
        // Appendix p.237: 83.35, 83.43, 82.87, 81.76 -> 11.65% (3 returns = N-1 for N = 4)
        let v = book_hv(&[83.35, 83.43, 82.87, 81.76], 4, true);
        assert!((v[3] - 11.65).abs() < 0.01, "{}", v[3]);
    }

    #[test]
    fn two_for_one_half_at_1r_folds_into_one_trade() {
        // long on the bar-0 close at 100, stop 95 (risk 5); bar 1 trades 106 -> half off at 105; rest out on the bar-2 close 102
        let rows = vec![row(0, 99.0, 100.5, 98.0, 100.0), row(1, 101.0, 106.0, 100.0, 104.0), row(2, 103.0, 104.0, 101.0, 102.0)];
        let ctx = Ctx::new(&rows, &rows, &[]);
        let sig = |i: usize| if i == 0 { Some(Ord { dir: 1.0, kind: Kind::Close, stop: 95.0 }) } else { None };
        let ex = hold_exit(2);
        let r = Rules { sig: &sig, touch: None, exit: &ex, reverse: false, extend: false, stop_pct: 0.0, mm: Mm { half_r: true, ..MM0 } };
        let t = sim(&ctx, 0, 2, &r);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].exit_i, t[0].exit_px), (2, 103.5)); // 0.5 x 105 + 0.5 x 102
        assert!(t[0].reason.starts_with("half off at +1R"), "{}", t[0].reason);
    }

    #[test]
    fn oops_buys_day_one_low_and_exits_moc() {
        // day 0 closes near its low (bottom 10%), day 1 opens below that low, rallies through it, closes 103
        let rows = vec![row(0, 105.0, 106.0, 100.0, 100.2), row(1, 98.0, 104.0, 97.0, 103.0), row(2, 103.0, 104.0, 102.0, 103.0)];
        let ctx = Ctx::new(&rows, &rows, &[]);
        let t = run("ats_10pct_oops", &ctx, 0, 2, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1.0, 1, 100.0, 1, 103.0));
    }
}
