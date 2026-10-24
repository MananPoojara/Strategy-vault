//! Charles D. Kirkpatrick II & Julie R. Dahlquist, *Technical Analysis: The Complete Resource for Financial Market
//! Technicians* (FT Press, 2006). Book id `kdta`. Source: research drive "USB HDD 2/NEW BOOKS/Charles D. Kirkpatrick II,
//! Julie R. Dahlquist - Technical analysis (2006, FT Press).pdf" (read in place; the copy into Books/ was refused, see
//! library/kdta/README.md "COPY PENDING"). Page refs are the book's printed pages.
//! A textbook: most rules are cited from Colby, Ned Davis Research, Active Trader Magazine, Carroll, Connors, Crabel,
//! Raschke, etc. Only systems the book states completely (entry AND exit, all numbers) are Live; the rest are listed
//! with the reason. Breadth systems read NYSE advances/declines/TRIN (extra_data/us, 1965-2020) and have `_nse` twins
//! that read NSE breadth (extra_data/india, 1999-) - India = instrument swap only, thresholds unchanged.
//! Every signal is evaluated on the close and filled at that close unless the rule names another order (the book
//! never states the fill for its close-based rules; close-of-signal is the neutral reading, documented per strategy).
use crate::connors::{rsi, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "kdta";
/// Admitted ids (authors/kirkpatrick/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["kdta_vix_ema", "kdta_rsi_wrong_long", "kdta_rsi_wrong_short", "kdta_apple_sar", "kdta_apple_v2", "kdta_may_oct", "kdta_pullback6", "kdta_adline", "kdta_adr191", "kdta_zweig_thrust", "kdta_haurlan200", "kdta_arms265", "kdta_arms_ext", "kdta_arms10ma", "kdta_adline_nse", "kdta_adr191_nse", "kdta_zweig_thrust_nse", "kdta_haurlan200_nse", "kdta_arms265_nse", "kdta_arms_ext_nse", "kdta_arms10ma_nse"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NYSE_AD: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines"];
const NSE_AD: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines"];
const NYSE_TR: &[&str] = &["NYSE_TRIN"];
const NSE_TR: &[&str] = &["NSE_TRIN"];

macro_rules! close_note { () => { "Fill: the book gives no order type; the signal is computed on the close and traded at that close (neutral reading). " }; }
macro_rules! breadth_note { () => { "Book market: NYSE breadth with the DJIA / S&P 500. Here NYSE advances/declines (extra_data NYSE_ADV_DEC, 1965-03-01 .. 2020-02-10) on DJI/SPX; the _nse twin uses NSE advances/declines (1999-) on NIFTY 50 with the same thresholds (instrument swap only; NSE lists fewer issues than the NYSE, so count-based levels are not rescaled). " }; }
macro_rules! arms_note { () => { "Arms Index (TRIN) = (advances/declines)/(advancing volume/declining volume) (p.143). Book market: NYSE. Here NYSE_TRIN (extra_data, 1965-2020) on DJI/SPX; the _nse twin reads NSE_TRIN on NIFTY 50. 'A year' = 252 trading days, the book's own count for 'hold it for a year' (\"the original 252 days\", p.143). Buy at the close of the signal day. " }; }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, vix, series: &'static [&'static str], rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: vix, needs_series: series, book: BOOK, rules, notes, claim: None,
    };
    let mut v = vec![
        d("kdta_vix_ema", "Colby VIX ten-day EMA direction", "Ch 7 · Sentiment, Box 7.3 (p.98)", "Long", Live, true, &[],
          &["F|\"Formula: X = Ten-day exponential moving average of VIX\" (Box 7.3, p.98)",
            "E|\"Buy: When X today is greater than X yesterday.\"",
            "X|\"Close Long: When X today is less than X yesterday.\" \"Sell Short: Never.\""],
          "EMA alpha = 2/(10+1), seeded with the first VIX close. Book test: January 1968-September 2000, 'returns 46.65% greater than a buy-and-hold strategy and a maximum trade drawdown of 46.69%' (Colby) - not reproducible: the CBOE VIX starts in 1990 (the vault's VIX file) and the market traded is not named (run on SPX/NDX/DJI; India VIX on NIFTY 50). Fill: close of the signal day (neutral reading; Colby's encyclopedia does not specify)."),
        d("kdta_rsi_wrong_long", "\"RSI is wrong\" breakout · Long", "Ch 18 · Confirmation, RSI (p.440)", "Long", Live, false, &[],
          &["E|\"Buy when 14-day RSI penetrates above 75\" (Active Trader Magazine test, p.440)",
            "X|\"sell when RSI penetrates 25 or after 20 days\""],
          "RSI = Wilder 14 (engine `rsi`). 'Penetrates above 75' = RSI crosses from <= 75 to > 75; exit when RSI crosses from >= 25 to < 25, or at the close 20 bars after entry. Book test: 19 futures 1994-2003, no numbers printed. Run on the indices (vault convention). Fill: close of the signal / exit day."),
        d("kdta_rsi_wrong_short", "\"RSI is wrong\" breakout · Short", "Ch 18 · Confirmation, RSI (p.440)", "Short", Live, false, &[],
          &["E|\"The opposite rules apply for sells using a break below the RSI 25 level as the initial short sale\" (p.440)",
            "X|Opposite of \"sell when RSI penetrates 25 or after 20 days\": cover when RSI penetrates above 75 or after 20 days"],
          "Mirror of the long side. Fill: close."),
        d("kdta_apple_sar", "Apple-a-Day weekly 20-week channel breakout (stop and reverse)", "Ch 22 · System design, Box 22.1 (p.551-552)", "Long/Short", Live, false, &[],
          &["F|Weekly bars (\"a simple Donchian channel breakout system on weekly highs and lows data\", p.551)",
            "E|\"The Donchian channel breakout system gives a buy signal when the closing price breaks above the highest price over the past x weeks. The sell signal occurs when the closing price breaks below the lowest price over the past x weeks. We will use 20 weeks as x\" (p.551)",
            "X|\"There will initially be no stops or any other exit criteria other than the opposite signal. This is called a 'Stop and Reverse' (SAR) system, and funds are invested at all times, either long or short.\" (p.552)"],
          "Weekly bar = the trading days of one Monday-Sunday calendar week; 'past x weeks' = the x completed weeks before the signal week. Fill: the weekly close (last trading day of the week). Book: AAPL weekly 12/11/1992-12/9/2005, 25 trades, 40% profitable, net +$6,195 (one share-equivalent dollar P&L, p.553-554) - AAPL is not in the vault, so it runs on the indices (instrument swap)."),
        d("kdta_apple_v2", "Apple-a-Day upgrade: 22/20-week breakout, long only, 2% stop", "Ch 22 · System design, Box 22.2 (p.564)", "Long", Live, false, &[],
          &["E|\"changing the buy and sell parameters to 22 and 20, respectively\": buy when the weekly close breaks above the highest price of the past 22 weeks (p.564)",
            "F|\"we also eliminate the short sales, remaining either in a long position or cash\"",
            "X|\"we use a 2% protective stop in case the long position begins to get into trouble\": sell stop 2% below the entry price",
            "X|\"and a 20-week channel breakout downward as a trailing stop\": sell when the weekly close breaks below the lowest price of the past 20 weeks"],
          "Read: the 2% stop is a resting sell stop from the bar after entry (fills at the stop, or at the open when the day opens below it); the 20-week downward breakout uses the weekly close, as the system's breakouts are defined on the close (p.551). After a stop-out a new entry needs a new weekly-close breakout. Book (AAPL 1992-2005): 31 trades, 9.7% profitable, net +$7,291, profit factor 11.0 (p.564)."),
        d("kdta_may_oct", "May-October seasonal system (long Oct 1, short May 15)", "Ch 9 · Temporal patterns, seasonal (p.165)", "Long/Short", Live, false, &[],
          &["E|\"the results of buying on October 1 and selling short on May 15 for ten years, January 1993 through March 2003, showed excellent results\" (Active Trader Magazine, p.165)",
            "X|Stop and reverse on the two dates (\"The only problem was the occurrence of large drawdowns on the short side\")"],
          "Each date = the first trading day on/after October 1 (go long) and on/after May 15 (reverse to short); fill at that day's close (book silent). No numbers printed. Parameter shorts=0 holds cash from May 15 instead (non-default)."),
        d("kdta_pullback6", "6% pullback buy, exit next open", "Ch 17 · Short-term patterns, trend correction (p.387)", "Long", Live, false, &[],
          &["E|\"entering a buy at 6% below the previous bar close\" (Active Trader Magazine, long-only, p.387): buy limit at 0.94 x yesterday's close",
            "X|\"and exiting on the next open\""],
          "Limit fills when the day's low reaches it, at the limit or the open if the day opens below it. Book: 18 stocks 1992-2002, gain equal to buy-and-hold with 17% market exposure (no table). Run on the indices."),
    ];
    for nse in [false, true] {
        let ad = if nse { NSE_AD } else { NYSE_AD };
        let tr = if nse { NSE_TR } else { NYSE_TR };
        macro_rules! id { ($a:expr, $b:expr) => { if nse { $b } else { $a } } }
        v.push(d(id!("kdta_adline", "kdta_adline_nse"), if nse { "One-day change in the advance-decline line · NSE" } else { "One-day change in the advance-decline line · NYSE" },
            "Ch 8 · Measuring market strength (p.133)", "Long/Short", Live, false, ad,
            &["E|\"using simply the one-day change in direction of the cumulative advance-decline line as a signal to buy or sell\" (p.133): long when the A/D line rises (advances > declines)",
              "X|\"a theoretical portfolio, either long or short one 'share' of the DJIA\": reverse to short when the A/D line falls (advances < declines); unchanged line = no change"],
            if nse { concat!(breadth_note!(), "NSE twin (see the NYSE note).") } else { concat!(breadth_note!(), close_note!(), "Book: Colby reports excellent results back to the 1930s; the authors' DJIA equity line fails from 1998-1999 and after April 2000 (Figure 8.3, test to February 2005). Fill: close. Parameter shorts=0 = long/cash (non-default).") }));
        v.push(d(id!("kdta_ad1018", "kdta_ad1018_nse"), if nse { "Colby one-day A/D ratio 1.018 · NSE" } else { "Colby one-day A/D ratio 1.018 · NYSE" },
            "Ch 8 · Advance-decline ratio (p.138)", "Long/Short", Live, false, ad,
            &["E|\"taking a one-day advance-decline ratio and buying the Dow Jones Industrial Average (DJIA) when the ratio crossed above 1.018\" (Colby, p.138)",
              "X|\"and selling it when the ratio declined below 1.018 ... the results were excellent for both longs and shorts\": reverse to short below 1.018"],
            if nse { concat!(breadth_note!(), "NSE twin.") } else { concat!(breadth_note!(), close_note!(), "Ratio = advances / declines of the day. Book: March 1932-August 2000 $100 -> $884,717,056 (no costs), one trade every 3.47 days; the authors' test April 2000-February 2005 with $1/share commission and $0.10 slippage: +56.1% vs buy-and-hold -18.4%, average hold 3.23 days (p.138). Fill: close. shorts=0 = long/cash (non-default).") }));
        v.push(d(id!("kdta_adr191", "kdta_adr191_nse"), if nse { "Ned Davis ten-day A/D ratio above 1.91, hold one year · NSE" } else { "Ned Davis ten-day A/D ratio above 1.91, hold one year · NYSE" },
            "Ch 8 · Advance-decline ratio (p.138)", "Long", Live, false, ad,
            &["E|\"26 buy signals were generated when the ratio of ten-day advances to ten-day declines exceeded 1.91\" (Ned Davis Research, 1947-2005, Figure 8.5, p.138): buy when the 10-day sum of advances / 10-day sum of declines crosses above 1.91",
              "X|\"These signals averaged a 17.6% return over the following year. In only one of the 26 instances did the signal fail, and the loss then was only 5.6%.\" -> sell 252 trading days later (a new crossing while held is ignored)"],
            if nse { concat!(breadth_note!(), "NSE twin.") } else { concat!(breadth_note!(), close_note!(), "Book: 26 signals 1947-2005, +17.6% average over the following year, 1 loss (-5.6%). Our NYSE breadth starts 1965. Fill: close. 'Year' = 252 bars (the book's count, p.143).") }));
        v.push(d(id!("kdta_zweig_thrust", "kdta_zweig_thrust_nse"), if nse { "Zweig breadth thrust (0.659 / 0.366) · NSE" } else { "Zweig breadth thrust (0.659 / 0.366) · NYSE" },
            "Ch 8 · Breadth thrust (p.142)", "Long", Live, false, ad,
            &["F|\"calculating a ten-day simple moving average of advances divided by the sum of advances and declines\" (p.142)",
              "E|\"Traditionally, the long only signal levels in this oscillator were to buy when the index rose above 0.659\"",
              "X|\"and sell when it declined below 0.366\""],
            if nse { concat!(breadth_note!(), "NSE twin.") } else { concat!(breadth_note!(), close_note!(), "Book: 'With these limits, however, this method has not had a signal since 1994' (p.142); the 4-6 day average the authors found profitable is a labelled option (param ma, default 10). Not the same rule as Zweig's own 10-day A/D >= 2 (mzwws). Fill: close.") }));
        v.push(d(id!("kdta_haurlan20", "kdta_haurlan20_nse"), if nse { "Haurlan 20-day EMA of A-D, buy below -200 / sell above +200 · NSE" } else { "Haurlan 20-day EMA of A-D, buy below -200 / sell above +200 · NYSE" },
            "Ch 8 · Haurlan index, Carroll levels (p.136)", "Long", Live, false, ad,
            &["F|\"He calculated a 20-day exponential moving average of advances minus declines\" (Haurlan, p.135-136)",
              "E|\"reliable extreme signals occurred at specific levels in each average. These levels were +/- 200 for the 20-day\" (Carroll, p.136); \"Buying when the averages declined below -200\"",
              "X|\"and selling when they exceeded +200 brought 303.6% return versus a -17.3% return on the buy-and-hold\" (authors' test, April 2000-January 2005, p.136)"],
            if nse { concat!(breadth_note!(), "NSE twin (NSE has far fewer issues than the NYSE; the +/-200 count levels are kept, instrument swap only).") } else { concat!(breadth_note!(), close_note!(), "EMA alpha = 2/(20+1), seeded with the first A-D value. Book result (authors' test April 2000-January 2005): +303.6% vs buy-and-hold -17.3%, RINA 6.78. Fill: close.") }));
        v.push(d(id!("kdta_haurlan200", "kdta_haurlan200_nse"), if nse { "Haurlan 200-day EMA of A-D, levels -550 / +550 · NSE" } else { "Haurlan 200-day EMA of A-D, levels -550 / +550 · NYSE" },
            "Ch 8 · Haurlan index, Carroll levels (p.136)", "Long", Live, false, ad,
            &["E|\"These levels were +/- 200 for the 20-day and +/-550 for the 200-day\" (Carroll, p.136): buy when the 200-day EMA of A-D declines below -550",
              "X|Sell when it exceeds +550 (same rule as the 20-day test)",
              "X|Option (non-default, time_stop): \"The 200-day signals were not as successful until a 50-bar time stop was introduced. This kind of stop closes the position in 50 periods regardless of the gain or loss.\" (p.136)"],
            if nse { concat!(breadth_note!(), "NSE twin.") } else { concat!(breadth_note!(), close_note!(), "EMA alpha = 2/201, seeded with the first A-D value. Default = Carroll's levels; the authors' 50-bar time stop is param time_stop=50 (0 = off, default). Fill: close.") }));
        v.push(d(id!("kdta_arms265", "kdta_arms265_nse"), if nse { "Alphier-Kuhn panic: Arms Index above 2.65, hold a year · NSE" } else { "Alphier-Kuhn panic: Arms Index above 2.65, hold a year · NYSE" },
            "Ch 8 · Arms Index (p.143)", "Long", Live, false, tr,
            &["E|\"A long-standing panic signal, devised by Alphier and Kuhn (1987), is to buy the stock market when the Arms Index exceeds 2.65\" (p.143)",
              "X|\"and hold it for a year\" - \"the original 252 days\" (p.143); option hold=40 is the authors' shortened test (non-default)"],
            if nse { concat!(arms_note!(), "NSE twin.") } else { concat!(arms_note!(), close_note!(), "Signal = the day's TRIN > 2.65 while flat. Book: two signals since 2000 (one win, one loss); with a 40-day hold +223% vs +1.7% buy-and-hold (p.143).") }));
        v.push(d(id!("kdta_arms_ext", "kdta_arms_ext_nse"), if nse { "Arms Index below 0.523 or above 1.444, hold a year (extended) · NSE" } else { "Arms Index below 0.523 or above 1.444, hold a year (extended) · NYSE" },
            "Ch 8 · Arms Index (p.144)", "Long", Live, false, tr,
            &["E|\"buy when the Arms Index declines below 0.523 or rises above 1.444 and hold for a year\" (Colby, p.144)",
              "X|\"The rule also states that if the year runs out but a later signal is still active, the position should be held until a year after the final signal.\""],
            if nse { concat!(arms_note!(), "NSE twin.") } else { concat!(arms_note!(), close_note!(), "Signal = the day's TRIN < 0.523 or > 1.444; every signal while long pushes the exit to 252 bars after it. Book: outstanding for 35 years, fails after 2000 (held through 2000-2002, p.144).") }));
        v.push(d(id!("kdta_arms10ma", "kdta_arms10ma_nse"), if nse { "Ten-day Arms Index average above 1.266, hold a year · NSE" } else { "Ten-day Arms Index average above 1.266, hold a year · NYSE" },
            "Ch 8 · Arms Index (p.145)", "Long", Live, false, tr,
            &["E|\"Another popular method has been to buy the market and hold for a year when the ten-day simple moving average of the Arms Index breaks above 1.266\" (p.145)",
              "X|Hold for a year (252 bars); a new break while held is ignored"],
            if nse { concat!(arms_note!(), "NSE twin.") } else { concat!(arms_note!(), close_note!(), "'Breaks above' = 10-day SMA crosses from <= 1.266 to > 1.266. Book: 'This method no longer works ... disastrous during the 2000-2002 stock market decline' (p.145).") }));
    }
    let nt = |id, name, group, dir, why, rules| d(id, name, group, dir, NotTestable(why), false, &[], rules, "");
    let nd = |id, name, group, dir, why, rules| d(id, name, group, dir, NeedsData(why), false, &[], rules, "");
    v.extend([
        nt("kdta_two_bar", "Two-bar breakout (Active Trader)", "Ch 17 · Short-term patterns (p.382)", "Long/Short",
           "the exit \"on a stop at the then-current day's low\" can be read as the entry day's low or a stop trailed to each new day's low; the book gives nothing to decide (and 'one tick' has no index equivalent stated)",
           &["E|\"the next day buy on a stop one tick above today's high if (1) today's low is less than yesterday's low; (2) today's high is less than yesterday's high; and (3) today's close is less than today's open\" (p.382)", "X|\"Exit on a stop at the then-current day's low. The sell side is just the opposite.\""]),
        nt("kdta_crabel_inside", "Crabel inside-bar open breakouts", "Ch 17 · Inside bar (p.383-384)", "Long/Short",
           "no exit is stated for any of Crabel's inside-bar tests (only winning percentages: 68%, 74%, 80%, 90%)",
           &["E|\"buying at the next opening, if it occurred above the inside bar close and selling at the next opening, if it occurred below the inside bar close\" (p.383)", "E|Four-day pattern: Day 2 higher low, Day 3 inside, Day 4 opens below mid-range and close of the inside day -> sell (p.384)"]),
        nt("kdta_connors_wr", "Connors wide-range bar reversal", "Ch 17 · Wide-range bar (p.391)", "Long/Short",
           "\"a two-standard deviation decline\" is not defined (standard deviation of what, over how long) and the stop is moved by judgement (\"If the buy is near the previous day's low, then lower the stop to give some room\")",
           &["E|\"place a buy entry stop at the first day's close with a protection stop at the first day's low\" (p.391)", "X|\"sell on the close, or if the close on the action day is within 10-15% of the high sell on the next day opening\""]),
        nt("kdta_raschke_nr4", "Raschke NR4 / inside day with 6/100-day volatility", "Ch 17 · Narrow-range bar (p.392)", "Long/Short",
           "duplicate: Raschke's ID/NR4 with the 6/100-day historical-volatility filter is already in the vault (Street Smarts, ss.rs); K&D only cites it",
           &["E|\"If the 6-day historic volatility is 50% less than the 100-day ... The buy and sell entry stops are placed at the high and low of the qualified NR4 or inside day\" (p.392)"]),
        nt("kdta_crabel_nr", "Crabel NR4 / NR7 breakout", "Ch 17 · Narrow-range bar (p.392)", "Long/Short",
           "no exit stated (\"we place a buy and sell entry stop on either side to be acted upon tomorrow or the next day\")",
           &["E|\"Their entry signal is a breakout from the most recent narrow-range day\" (p.392)"]),
        nt("kdta_connors_vix_ma", "Connors VIX 5% / 10% stretch zones", "Ch 17 · VIX (p.392)", "Long",
           "zones for 'when action can be contemplated', not entry/exit rules (\"not necessarily a good time to buy except on throwbacks\"); Connors' own VIX rules are in the vault (ws5, vixr)",
           &["F|\"Connors uses a 5% deviation from a ten-day simple moving average ... When the ratio is above 5%, and even more so when it is above 10% of the SMA, the time is usually excellent to buy\" (p.392)"]),
        nt("kdta_donchian4", "Donchian four-week rule", "Ch 12 · Channels (p.230)", "Long/Short",
           "duplicate: the four-week (20-day) breakout stop-and-reverse is already in the vault (covel/turtle and other channel-breakout modules)",
           &["E|\"Buy when the highest high over the past four weeks is broken, and sell when the lowest low over the past four weeks is broken. A 'stop-and-reverse' strategy\" (p.230)"]),
        nt("kdta_holy_grail", "Raschke Holy Grail", "Ch 18 · ADX (p.448)", "Long/Short",
           "duplicate of Street Smarts' Holy Grail (ss.rs); K&D's version is incomplete (\"comes close to that EMA\", target then a judgement)",
           &["E|\"a 14-day ADX must be above 30 ... enter in the direction of the trend when the price touches or comes close to that EMA\" (p.448)"]),
        nt("kdta_popsteckle", "Popsteckle (Steckler / Bernstein)", "Ch 18 · ADX (p.448)", "Long",
           "setup needs judgement (\"Recent low volatility in price action (dull action)\", \"breaks upward out of congestion\") and states no exit",
           &["F|Setup rules 1-6 (p.448): 14-day ADX < 20, 8-day %K rising above 70, 8-week %K rising above 50, bullish market, breakout on volume 50% above its 50-day SMA"]),
        nt("kdta_hughes", "Hughes breadth oscillator zero cross", "Ch 8 · Hughes breadth oscillator (p.140)", "Long/Short",
           "the smoothing length is not given (\"traditionally smoothed with a moving average\")",
           &["E|\"When the ratio rises above or declines below zero, a buy or sell signal is generated respectively\" (p.140)"]),
        nt("kdta_haurlan_zero", "Haurlan averages zero crossings", "Ch 8 · Haurlan index (p.136)", "Long/Short",
           "Carroll's zero-crossing rule is cited without its side (long-only or reversing) or which of the 3/20/200-day averages; the 3-day rules are \"generated\" by Haurlan but not stated",
           &["E|\"crossovers in each of the Haurlan averages through zero give reliable signals\" (p.136)"]),
        nt("kdta_arms11", "Colby 11-day Arms Index average crossing 0.800", "Ch 8 · Arms Index (p.145)", "Long/Short",
           "the direction of each crossing is not stated (\"buy and sell the market as the 11-day moving average of the Arms Index crosses 0.800\")",
           &["E|\"the method proposed by Colby (2003) is to buy and sell the market as the 11-day moving average of the Arms Index crosses 0.800\" (p.145)"]),
        nt("kdta_plurality", "Colby plurality index Bollinger breakout", "Ch 8 · Plurality index (p.137)", "Long",
           "the time stop is given as \"a 15- or 30-day time stop\" (two values, no default) and the band uses the plurality index whose levels the book calls obsolete",
           &["E|\"Colby uses a long-term (324-day) Bollinger Band ... breakout of the upper two standard deviations for buys and below two standard deviations for a sell to close\" (p.137)"]),
        nt("kdta_absolute_breadth", "Absolute breadth index vs two-day EMA + 81%", "Ch 8 · Absolute breadth index (p.141)", "Long/Short",
           "the signal side and exit are not stated (\"a profitable signal was generated when this index crossed the previous day's two-day exponential moving average plus 81%\")",
           &["F|\"the absolute breadth index uses the absolute difference of the advances minus declines divided by the total issues traded\" (p.140-141)"]),
        nt("kdta_mama_fama", "MAMA / FAMA crossover", "Ch 14 · Moving averages (p.283)", "Long/Short",
           "the MESA adaptive moving average formulas are not given in the book (\"the calculation of these moving averages is complicated\")",
           &["E|\"A buy or sell signal is generated when the MAMA crosses the FAMA\" (p.283)"]),
        nd("kdta_nh_nl", "New highs versus new lows", "Ch 8 · New highs and lows (p.147)", "Long/Short",
           "daily NYSE / NSE new 52-week highs and new lows counts (not in extra_data)",
           &["E|\"buy when the number of new highs exceeds the number of new lows on a daily basis and to sell when the opposite occurs\" (p.147)"]),
        nd("kdta_thrust_weekly", "Five-week breadth thrust above 1.65", "Ch 8 · Breadth thrust (p.141-142)", "Long",
           "weekly NYSE advances/declines/total-issues counts (the vault has daily counts only) and the exit is only an outcome window (\"rose an average of 17.6% over the next 52 weeks\")",
           &["E|\"when the breadth ratio thrust above 1.65 ... calculated as the five-week total of the weekly advances to total issues ratio less the weekly declines to total issues ratio\" (p.141-142)"]),
        nd("kdta_short_sales", "Public / specialist short-sale ratios (Colby)", "Ch 7 · Sentiment (p.116)", "Long/Short",
           "weekly NYSE public, member and specialist short-sale volumes 1946-2000",
           &["E|\"when the current ratio exceeded by 45.7% its 50-week exponential moving average, a buy signal was generated, and when the current ratio declined to 45.7% below its 50-week exponential moving average, a sell signal was generated\" (p.116)"]),
        nd("kdta_margin_debt", "Margin debt 15-month rate of change", "Ch 10 · Flow of funds (p.171)", "Long",
           "monthly NYSE margin debt",
           &["E|\"uses a 15-month rate of change ... When, over this period, the indicator crosses above -21%, a buy signal is generated, and when it crosses below 61%, a sell signal is generated\" (p.171)"]),
        nd("kdta_polls", "Sentiment polls (AAII, Consensus, Market Vane)", "Ch 7 · Polls (p.99-101)", "Long/Short",
           "weekly AAII / Consensus Inc. / Market Vane survey histories (and the Consensus / Market Vane band widths are not fully specified)",
           &["F|AAII: 2-week MA of bulls/(bulls+bears) above 67.5 -> market fell 2.8% p.a.; below 49.0 -> rose 20.6% p.a. (Ned Davis, July 1987-January 2005, p.99)", "F|Consensus Bullish Sentiment: buy above two standard deviations, 25.1% gain 1983-1998 (p.100)", "F|Market Vane: standard deviation band about a 15-day MA, buy when the MA rises above the lower band; 82% profitable 1988-1998 (p.101)"]),
        nd("kdta_williams_oi", "Larry Williams open-interest decline", "Ch 18 · Open interest (p.431)", "Long",
           "futures open interest; also needs a judged trading range",
           &["F|\"a 30% change in open interest is necessary before action is contemplated\" (p.431)"]),
    ]);
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let base = id.trim_end_matches("_nse");
    let mkt = if id.ends_with("_nse") { "NSE" } else { "NYSE" };
    Some(match base {
        "kdta_vix_ema" => ("kdta_vix", "K&D VIX rules", "Colby 10-day EMA"),
        "kdta_rsi_wrong_long" => ("kdta_rsi_wrong", "K&D \"RSI is wrong\"", "Long"),
        "kdta_rsi_wrong_short" => ("kdta_rsi_wrong", "K&D \"RSI is wrong\"", "Short"),
        "kdta_apple_sar" => ("kdta_apple", "K&D Apple-a-Day system", "20/20 stop and reverse"),
        "kdta_apple_v2" => ("kdta_apple", "K&D Apple-a-Day system", "22/20 long only, 2% stop"),
        "kdta_may_oct" => ("kdta_calendar", "K&D seasonal", "May-October"),
        "kdta_pullback6" => ("kdta_short_term", "K&D short-term patterns", "6% pullback"),
        "kdta_adline" | "kdta_ad1018" | "kdta_adr191" | "kdta_zweig_thrust" | "kdta_haurlan20" | "kdta_haurlan200" => ("kdta_breadth", "K&D breadth systems", mkt),
        "kdta_arms265" | "kdta_arms_ext" | "kdta_arms10ma" => ("kdta_arms", "K&D Arms Index systems", mkt),
        _ if id.starts_with("kdta_") => ("kdta_other", "K&D cited rules (not testable)", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let base = id.trim_end_matches("_nse");
    Some(match base {
        "kdta_vix_ema" => vec![p("ema", "VIX EMA length", 10.0, 3.0, 30.0, 1.0)],
        "kdta_rsi_wrong_long" | "kdta_rsi_wrong_short" => vec![
            p("rsi", "RSI length", 14.0, 5.0, 30.0, 1.0), p("hi", "Upper RSI level", 75.0, 55.0, 90.0, 1.0),
            p("lo", "Lower RSI level", 25.0, 10.0, 45.0, 1.0), p("days", "Time exit (bars)", 20.0, 5.0, 60.0, 1.0)],
        "kdta_apple_sar" => vec![p("buy_weeks", "Buy: close above the high of the past N weeks", 20.0, 4.0, 52.0, 1.0), p("sell_weeks", "Sell: close below the low of the past N weeks", 20.0, 4.0, 52.0, 1.0)],
        "kdta_apple_v2" => vec![p("buy_weeks", "Buy: close above the high of the past N weeks", 22.0, 4.0, 52.0, 1.0), p("sell_weeks", "Trailing exit: close below the low of the past N weeks", 20.0, 4.0, 52.0, 1.0), p("stop_pct", "Protective stop below entry (%)", 2.0, 0.5, 10.0, 0.5)],
        "kdta_may_oct" => vec![p("shorts", "Short from May 15 (1) or cash (0)", 1.0, 0.0, 1.0, 1.0)],
        "kdta_pullback6" => vec![p("pct", "Buy limit below yesterday's close (%)", 6.0, 1.0, 15.0, 0.5)],
        "kdta_adline" => vec![p("shorts", "Reverse to short (1) or cash (0)", 1.0, 0.0, 1.0, 1.0)],
        "kdta_ad1018" => vec![p("level", "One-day advances/declines level", 1.018, 0.5, 2.0, 0.001), p("shorts", "Reverse to short (1) or cash (0)", 1.0, 0.0, 1.0, 1.0)],
        "kdta_adr191" => vec![p("days", "Sum length (days)", 10.0, 5.0, 20.0, 1.0), p("level", "10-day advances / 10-day declines level", 1.91, 1.2, 3.0, 0.01), p("hold", "Bars held (a year = 252)", 252.0, 20.0, 504.0, 1.0)],
        "kdta_zweig_thrust" => vec![p("ma", "SMA of A/(A+D) (book 10; authors' 4-6 option)", 10.0, 3.0, 20.0, 1.0), p("buy", "Buy above", 0.659, 0.5, 0.8, 0.001), p("sell", "Sell below", 0.366, 0.2, 0.5, 0.001)],
        "kdta_haurlan20" => vec![p("ema", "EMA length of A-D", 20.0, 5.0, 60.0, 1.0), p("buy", "Buy below", -200.0, -1000.0, 0.0, 10.0), p("sell", "Sell above", 200.0, 0.0, 1000.0, 10.0), p("time_stop", "Time stop in bars (0 = none)", 0.0, 0.0, 200.0, 1.0)],
        "kdta_haurlan200" => vec![p("ema", "EMA length of A-D", 200.0, 50.0, 400.0, 10.0), p("buy", "Buy below", -550.0, -2000.0, 0.0, 10.0), p("sell", "Sell above", 550.0, 0.0, 2000.0, 10.0), p("time_stop", "Time stop in bars (0 = none; authors' test 50)", 0.0, 0.0, 200.0, 1.0)],
        "kdta_arms265" => vec![p("level", "Arms Index above", 2.65, 1.5, 4.0, 0.05), p("hold", "Bars held (book 252; authors' 40)", 252.0, 10.0, 504.0, 1.0)],
        "kdta_arms_ext" => vec![p("low", "Arms Index below", 0.523, 0.3, 0.8, 0.001), p("high", "Arms Index above", 1.444, 1.1, 2.5, 0.001), p("hold", "Bars held after the last signal (a year = 252)", 252.0, 10.0, 504.0, 1.0)],
        "kdta_arms10ma" => vec![p("ma", "SMA length of the Arms Index", 10.0, 3.0, 30.0, 1.0), p("level", "SMA breaks above", 1.266, 1.0, 2.0, 0.001), p("hold", "Bars held (a year = 252)", 252.0, 10.0, 504.0, 1.0)],
        _ if id.starts_with("kdta_") => vec![],
        _ => return None,
    })
}

// ------------------------------------------------------------------ helpers

/// EMA with alpha = 2/(n+1), seeded with the first finite value; NaN inputs keep the previous value (output NaN).
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut prev = f64::NAN;
    v.iter().map(|&x| {
        if !x.is_finite() { return f64::NAN; }
        prev = if prev.is_nan() { x } else { prev + a * (x - prev) };
        prev
    }).collect()
}

fn new_trade(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}

fn finish(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
}

/// Close-to-close position driver: `f(i, dir, entry_i)` returns Some((new_dir, reason)) to change the position at
/// bar i's close (exit and, if new_dir != 0, enter the new direction at the same close).
fn drive(x: &Ctx, from: usize, to: usize, mut f: impl FnMut(usize, i8, usize) -> Option<(i8, String)>) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        let (d, e) = cur.as_ref().map_or((0i8, 0usize), |t| (t.dir as i8, t.legs[0].i));
        let Some((nd, why)) = f(i, d, e) else { continue };
        if nd == d { continue; }
        if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = why.clone(); out.push(t); }
        if nd != 0 && i < to { let mut t = new_trade(nd as f64, i, x.c[i], "close"); t.reason = why; cur = Some(t); }
    }
    finish(&mut out, cur, x, to);
    out
}

/// Buy on the close of a signal day and hold `hold` bars; `extend` = every signal while long restarts the count.
fn hold_bars(x: &Ctx, from: usize, to: usize, sig: &[bool], hold: usize, extend: bool, why: &str) -> Vec<Trade> {
    let mut end = 0usize;
    drive(x, from, to, |i, d, _| {
        if d == 1 {
            if extend && sig[i] { end = i + hold; }
            if i >= end { return Some((0, format!("held {hold} bars"))); }
            None
        } else if sig[i] { end = i + hold; Some((1, why.to_string())) } else { None }
    })
}

fn dnum(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0));
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// Weekly bars (Monday-Sunday calendar weeks): for each bar, its week number k and, at the last bar of a week, the
/// week's high/low/close. hh(k, n) / ll(k, n) = highest high / lowest low of the n weeks before week k.
struct Weeks { k: Vec<usize>, end: Vec<bool>, wh: Vec<f64>, wl: Vec<f64>, wc: Vec<f64> }
impl Weeks {
    fn new(x: &Ctx) -> Weeks {
        let n = x.c.len();
        let wid: Vec<i64> = x.r.iter().map(|r| (dnum(&r.date) + 3).div_euclid(7)).collect();
        let (mut k, mut end, mut wh, mut wl, mut wc) = (vec![0; n], vec![false; n], vec![], vec![], vec![]);
        let mut cur = 0usize;
        for i in 0..n {
            if i > 0 && wid[i] != wid[i - 1] { cur += 1; }
            k[i] = cur;
            if wh.len() <= cur { wh.push(x.h[i]); wl.push(x.l[i]); wc.push(x.c[i]); }
            else { wh[cur] = wh[cur].max(x.h[i]); wl[cur] = wl[cur].min(x.l[i]); wc[cur] = x.c[i]; }
            end[i] = i + 1 == n || wid[i + 1] != wid[i];
        }
        Weeks { k, end, wh, wl, wc }
    }
    fn hh(&self, k: usize, n: usize) -> Option<f64> { (k >= n && n > 0).then(|| self.wh[k - n..k].iter().cloned().fold(f64::MIN, f64::max)) }
    fn ll(&self, k: usize, n: usize) -> Option<f64> { (k >= n && n > 0).then(|| self.wl[k - n..k].iter().cloned().fold(f64::MAX, f64::min)) }
}

/// first bar on/after (month, day) in each year
fn first_on_after(x: &Ctx, i: usize, m: u32, d: u32) -> bool {
    fn md(s: &str) -> (u32, u32, &str) { (s[5..7].parse::<u32>().unwrap_or(0), s[8..10].parse::<u32>().unwrap_or(0), &s[0..4]) }
    let (mi, di, yi) = md(&x.r[i].date);
    if (mi, di) < (m, d) { return false; }
    if i == 0 { return true; }
    let (mp, dp, yp) = md(&x.r[i - 1].date);
    yp != yi || (mp, dp) < (m, d)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("kdta_") { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || to >= n || from > to { return Some(vec![]); }
    let nse = id.ends_with("_nse");
    let ad = || -> Option<(&[f64], &[f64])> { let s = if nse { NSE_AD } else { NYSE_AD }; Some((x.series(s[0])?, x.series(s[1])?)) };
    let trin = || x.series(if nse { NSE_TR[0] } else { NYSE_TR[0] });
    let base = id.trim_end_matches("_nse");
    let out = match base {
        "kdta_vix_ema" => {
            let Some(v) = x.series("VIX.Close") else { return Some(vec![]) };
            let e = ema(v, g("ema") as usize);
            let mut last = f64::NAN; // previous finite EMA value
            drive(x, from, to, |i, d, _| {
                let (prev, now) = (last, e[i]);
                if now.is_finite() { last = now; }
                if !(now.is_finite() && prev.is_finite()) { return None; }
                if d == 0 && now > prev { Some((1, "VIX 10-day EMA up".into())) }
                else if d == 1 && now < prev { Some((0, "VIX 10-day EMA down".into())) } else { None }
            })
        }
        "kdta_rsi_wrong_long" | "kdta_rsi_wrong_short" => {
            let r = rsi(&x.c, g("rsi") as usize);
            let (hi, lo, days) = (g("hi"), g("lo"), g("days") as usize);
            let long = base.ends_with("long");
            drive(x, from, to, |i, d, e| {
                if i == 0 || !(r[i].is_finite() && r[i - 1].is_finite()) { return None; }
                let up = r[i - 1] <= hi && r[i] > hi;
                let dn = r[i - 1] >= lo && r[i] < lo;
                if d == 0 {
                    if long && up { return Some((1, "RSI crosses above upper level".into())); }
                    if !long && dn { return Some((-1, "RSI crosses below lower level".into())); }
                    return None;
                }
                if (long && dn) || (!long && up) { return Some((0, "RSI crosses the opposite level".into())); }
                if i - e >= days { return Some((0, format!("{days} bars"))); }
                None
            })
        }
        "kdta_apple_sar" => {
            let w = Weeks::new(x);
            let (bn, sn) = (g("buy_weeks") as usize, g("sell_weeks") as usize);
            drive(x, from, to, |i, d, _| {
                if !w.end[i] { return None; }
                let k = w.k[i];
                let (Some(hh), Some(ll)) = (w.hh(k, bn), w.ll(k, sn)) else { return None };
                if d != 1 && w.wc[k] > hh { Some((1, "weekly close above the channel high".into())) }
                else if d != -1 && w.wc[k] < ll { Some((-1, "weekly close below the channel low".into())) } else { None }
            })
        }
        "kdta_apple_v2" => {
            let w = Weeks::new(x);
            let (bn, sn, sp) = (g("buy_weeks") as usize, g("sell_weeks") as usize, g("stop_pct") / 100.0);
            let mut out = vec![];
            let mut cur: Option<Trade> = None;
            for i in from..=to {
                if let Some(mut t) = cur.take() {
                    let stop = t.legs[0].px * (1.0 - sp);
                    if i > t.legs[0].i && x.l[i] <= stop {
                        t.exit_i = i; t.exit_px = x.o[i].min(stop); t.signal_i = i; t.reason = "2% protective stop".into(); out.push(t);
                    } else if w.end[i] && w.ll(w.k[i], sn).is_some_and(|ll| w.wc[w.k[i]] < ll) {
                        t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = "weekly close below the 20-week low".into(); out.push(t);
                    } else { cur = Some(t); }
                }
                if cur.is_none() && i < to && w.end[i] && w.hh(w.k[i], bn).is_some_and(|hh| w.wc[w.k[i]] > hh) {
                    let mut t = new_trade(1.0, i, x.c[i], "close"); t.reason = "weekly close above the 22-week high".into(); cur = Some(t);
                }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "kdta_may_oct" => {
            let shorts = g("shorts") >= 0.5;
            drive(x, from, to, |i, d, _| {
                if first_on_after(x, i, 10, 1) && d != 1 { Some((1, "October 1".into())) }
                else if first_on_after(x, i, 5, 15) && d == 1 { Some((if shorts { -1 } else { 0 }, "May 15".into())) } else { None }
            })
        }
        "kdta_pullback6" => {
            let k = 1.0 - g("pct") / 100.0;
            let mut out = vec![];
            for i in from.max(1)..=to {
                let px = x.c[i - 1] * k;
                if x.l[i] <= px {
                    let fill = x.o[i].min(px);
                    let mut t = new_trade(1.0, i, fill, "limit");
                    if i + 1 <= to { t.exit_i = i + 1; t.exit_px = x.o[i + 1]; t.signal_i = i; t.reason = "next open".into(); }
                    else { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; }
                    out.push(t);
                }
            }
            out
        }
        "kdta_adline" | "kdta_ad1018" => {
            let Some((a, dcl)) = ad() else { return Some(vec![]) };
            let shorts = g("shorts") >= 0.5;
            let lvl = g("level");
            let off = if shorts { -1 } else { 0 };
            drive(x, from, to, |i, d, _| {
                if !(a[i].is_finite() && dcl[i].is_finite()) { return None; }
                let (up, dn) = if base == "kdta_adline" { (a[i] > dcl[i], a[i] < dcl[i]) } else {
                    if dcl[i] <= 0.0 { return None; }
                    let r = a[i] / dcl[i];
                    (r > lvl, r < lvl)
                };
                if up && d != 1 { Some((1, "breadth up".into())) }
                else if dn && ((shorts && d != -1) || (!shorts && d == 1)) { Some((off, "breadth down".into())) } else { None }
            })
        }
        "kdta_adr191" => {
            let Some((a, dcl)) = ad() else { return Some(vec![]) };
            let (days, lvl) = (g("days") as usize, g("level"));
            let r: Vec<f64> = (0..n).map(|i| {
                if i + 1 < days { return f64::NAN; }
                let (sa, sd) = (a[i + 1 - days..=i].iter().sum::<f64>(), dcl[i + 1 - days..=i].iter().sum::<f64>());
                if sd > 0.0 { sa / sd } else { f64::NAN }
            }).collect();
            let sig: Vec<bool> = (0..n).map(|i| i > 0 && r[i] > lvl && r[i - 1].is_finite() && r[i - 1] <= lvl).collect();
            hold_bars(x, from, to, &sig, g("hold") as usize, false, "10-day A/D ratio crosses above 1.91")
        }
        "kdta_zweig_thrust" => {
            let Some((a, dcl)) = ad() else { return Some(vec![]) };
            let ratio: Vec<f64> = (0..n).map(|i| if a[i] + dcl[i] > 0.0 { a[i] / (a[i] + dcl[i]) } else { f64::NAN }).collect();
            let s = sma(&ratio, g("ma") as usize);
            let (b, sl) = (g("buy"), g("sell"));
            drive(x, from, to, |i, d, _| {
                if !s[i].is_finite() { return None; }
                if d == 0 && s[i] > b { Some((1, "thrust above buy level".into())) } else if d == 1 && s[i] < sl { Some((0, "thrust below sell level".into())) } else { None }
            })
        }
        "kdta_haurlan20" | "kdta_haurlan200" => {
            let Some((a, dcl)) = ad() else { return Some(vec![]) };
            let net: Vec<f64> = (0..n).map(|i| a[i] - dcl[i]).collect();
            let e = ema(&net, g("ema") as usize);
            let (b, sl, ts) = (g("buy"), g("sell"), g("time_stop") as usize);
            drive(x, from, to, |i, d, en| {
                if d == 1 && ts > 0 && i - en >= ts { return Some((0, format!("{ts}-bar time stop"))); }
                if !e[i].is_finite() { return None; }
                if d == 0 && e[i] < b { Some((1, "Haurlan EMA below buy level".into())) } else if d == 1 && e[i] > sl { Some((0, "Haurlan EMA above sell level".into())) } else { None }
            })
        }
        "kdta_arms265" => {
            let Some(t) = trin() else { return Some(vec![]) };
            let lvl = g("level");
            let sig: Vec<bool> = t.iter().map(|&v| v.is_finite() && v > lvl).collect();
            hold_bars(x, from, to, &sig, g("hold") as usize, false, "Arms Index above 2.65")
        }
        "kdta_arms_ext" => {
            let Some(t) = trin() else { return Some(vec![]) };
            let (lo, hi) = (g("low"), g("high"));
            let sig: Vec<bool> = t.iter().map(|&v| v.is_finite() && (v < lo || v > hi)).collect();
            hold_bars(x, from, to, &sig, g("hold") as usize, true, "Arms Index below 0.523 or above 1.444")
        }
        "kdta_arms10ma" => {
            let Some(t) = trin() else { return Some(vec![]) };
            let s = sma(t, g("ma") as usize);
            let lvl = g("level");
            let sig: Vec<bool> = (0..n).map(|i| i > 0 && s[i].is_finite() && s[i - 1].is_finite() && s[i - 1] <= lvl && s[i] > lvl).collect();
            hold_bars(x, from, to, &sig, g("hold") as usize, false, "10-day Arms average breaks above 1.266")
        }
        _ => return Some(vec![]),
    };
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn ctx_rows(px: &[(f64, f64, f64, f64)], start: i64) -> Vec<Row> {
        px.iter().enumerate().map(|(k, &(o, h, l, c))| {
            let z = start + k as i64; // consecutive calendar days (weekends included: fine for the tests)
            let (y, m, d) = civil(z);
            Row { ticker: "T".into(), date: format!("{y:04}-{m:02}-{d:02}"), open: o, high: h, low: l, close: c, volume: None }
        }).collect()
    }
    fn civil(z: i64) -> (i64, u32, u32) {
        let z = z + 719468; let era = z.div_euclid(146097); let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153; let d = (doy - (153 * mp + 2) / 5 + 1) as u32; let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
    }
    #[test]
    fn pullback_limit_and_next_open() {
        let r = ctx_rows(&[(100.0, 101.0, 99.0, 100.0), (99.0, 99.5, 93.0, 95.0), (96.0, 97.0, 95.0, 96.0)], dnum("2020-01-06"));
        let x = Ctx::new(&r, &[], &[]);
        let t = run("kdta_pullback6", &x, 0, 2, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1, 94.0, 2, 96.0));
        assert_eq!(dnum("1970-01-01"), 0);
        assert_eq!(civil(dnum("2001-12-31")), (2001, 12, 31));
    }
}
