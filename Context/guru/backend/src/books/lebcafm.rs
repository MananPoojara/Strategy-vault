//! Charles LeBeau & David W. Lucas, "Technical Traders Guide to Computer Analysis of the Futures Market"
//! (Business One Irwin, 1992). Chapter 4 was reprinted as "Day Trading Systems & Methods" (Marketplace
//! Books "Trade Secrets", (c)1992) - same text, so the day-trading methods are listed here once.
//! Live: the fully specified systems and the Chapter 3 entry/exit test protocols. Futures dollar stops,
//! intraday methods and judgement rules are NeedsData / NotTestable. Sources and verification: library/lebcafm/.
// Exact window-sum SMA (apwms::sma): the running-sum SMA drifted at exact ties (a touch of a level or a close equal to the
// average was misread); 2026-10-24 LeBeau admission, book rule wins (see authors/lebeau/lebeau-REPORT.md).
use super::apwms::sma;
use crate::connors::{rsi, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
// Admitted 2026-10-24 (authors/lebeau/admission/SOURCE-CONTRACT.md): complete rules with independent US/India ledgers.
pub const REVIEWED: &[&str] = &["lebcafm_adx_parabolic", "lebcafm_vol_breakout", "lebcafm_babcock_dmi28", "lebcafm_cci_lambert", "lebcafm_et_ma918", "lebcafm_et_channel10", "lebcafm_et_rsi14", "lebcafm_et_cci10", "lebcafm_et_mom10", "lebcafm_et_vol", "lebcafm_xt_benchmark", "lebcafm_xt_rsi_trail", "lebcafm_xt_key_reversal", "lebcafm_xt_hl", "lebcafm_xt_volatility", "lebcafm_st_noprofit"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const BOOK: &str = "lebcafm";
const FUT: &str = "futures dollar amounts (per-contract $ stops, $ targets and $ trailing stops) have no index equivalent";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    let et_note = "Ch. 3 'Testing Entries' p.170-178: \"Set up your trading system and then delete your normal exits. Replace them with a method that automatically exits the market a given number of days after each trade has been entered\" (p.171); \"January 1986 to December 1990 ... Entries were intraday in the case of channel breakout and volatility entries, the next day's open in the case of the others. No stops were used. ... Exits were on the close of days 5, 10, 15, and 20 after entry.\" (p.173). Book results are % winners on D-mark, gold, soybeans, T-bonds and crude oil futures (Tables 1-10), not reproducible on an index. One position at a time; signals during a trade are ignored [book silent]. p.hold = 5 (first of the book's 5/10/15/20).";
    let xt_note = "Ch. 3 'Testing Exits' p.179-185: benchmark = dual moving average reversal (5/10, 5/20, 5/30, 5/40); \"On the opening of the day after the moving averages cross, we will close out one trade and immediately initiate another in the opposite direction ... we will trade only at the crossovers\" (p.179). \"If an exit isn't triggered, the trade will be closed out by the moving average crossover and a new trade initiated on the same date and at the same opening price\" (p.180). Jan 1986-Dec 1990, no stops, no costs. Results (Exhibit 3-14) are futures win% and $ P/L per market. p.slow = 10 (first pair); 20/30/40 are the book's other pairs. Crossover = fast SMA closes above (below) the slow SMA after being at or below (above) it the day before.";
    let mut v = vec![
        make("lebcafm_adx_parabolic", "ADX + Parabolic system ('Another Parabolic Trading System')", "Ch. 2 · Parabolic p.113-114", "Long/Short", Status::Live,
            &["F|\"Use an 18-day ADX and DMI. Do not initiate any trades when the direction of the ADX is downward.\" (rule 1, p.113) ADX rising = ADX today > ADX yesterday.",
              "E|\"When the ADX begins moving upward, enter in the direction of the DMI only if the Parabolic direction agrees.\" (rule 2, p.114) Read: at a close where ADX(18) is rising, +DI vs -DI gives the direction and the Parabolic is on the same side -> enter at the next open. [Book silent on the fill]",
              "E|\"If the Parabolic does not agree, wait and enter when it reverses.\" (rule 2) / \"After an exit re-enter when the prices cross back above the Parabolic, assuming the ADX is still rising and the DMI shows the trend is still intact.\" (rule 4) Read: with ADX rising and the DMI direction set at the prior close, a buy (sell) stop at today's SAR; fill at the SAR or the open if it gaps through.",
              "X|\"Once a trade is entered, use the Parabolic stops to exit.\" (rule 3) Exit when the bar trades through the SAR; fill at the SAR or the open.",
              "F|Parabolic = Wilder's: \"SAR (tomorrow) = SAR (today) + AF x (EP - SAR (today))\", AF starts at 0.02, +0.02 per new extreme, max 0.20 (p.109-110); \"We recommend using the standard default values\" (p.111). Started on the first bars of the data (\"if the first few days are up, the formula will assume an uptrend\", p.109)."],
            "LeBeau & Lucas's own simplification of Kaufman's Directional Parabolic (p.112-114, Exhibit 2-68). No results printed. Futures system in the book; run on the index (instrument swap). Both sides."),
        make("lebcafm_vol_breakout", "Basic volatility breakout (reversal)", "Ch. 2 · Volatility p.139-146", "Long/Short", Status::Live,
            &["F|ATR: \"take a number of past days and find the mean\" of the true range (p.141).",
              "E|\"the basic volatility breakout system is a reversal system always in the market. Each day after the close, calculate the ATR, and then multiply it by the percent move necessary to trigger a trade. Add the result to the close, and you will get the point at which a buy will be triggered the next day. Subtract the result from the close, and you will get the point at which a sell will be triggered. Enter both orders the next day\" (p.142)",
              "E|Values: the book's worked example \"the ATR for the last five days ... 150 percent\" (p.141); the two variables are otherwise left to the user (p.142).",
              "X|Stop-and-reverse only. A bar that hits both levels while flat is skipped; at most one reversal per bar. [Book silent: \"If both are triggered ... a whipsaw is the obvious result\", p.142]"],
            "Defaults = the book's example (5 days, 150%). The book's suggestions (neutral-zone risk stop, 18-day ADX filter, CCI filter, quick exits; p.144-146) are not given as fixed rules and are not coded. Futures in the book; index here."),
        make("lebcafm_babcock_dmi28", "Babcock 28-day DMI reversal", "Ch. 2 · DMI/ADX p.39", "Long/Short", Status::Live,
            &["E|\"The DMI method that Babcock tested goes long on the close, when net directional movement is positive. When net directional movement is negative, the system reverses to go short.\" (p.39) Net directional movement positive = +DI(28) > -DI(28).",
              "X|\"no stops were employed\"; \"profit taking was strictly on crossovers\" (p.39). Always in the market after the first signal."],
            "Cited test (Babcock, Dow Jones-Irwin Guide to Trading Systems). The authors add: \"Although we don't recommend trading DMI this way\" (p.39). No figures printed. Enters on the first close of the window where the sign is defined."),
        make("lebcafm_cci_lambert", "Lambert CCI +/-100", "Ch. 2 · CCI p.61-63", "Long/Short", Status::Live,
            &["F|CCI: \"a simple moving average of average daily prices [(high + low + close) / 3], and then calculates the mean deviation ... multiplied by a constant (0.015 ...) and divided into the difference between the simple moving average and today's average price\" (p.62); \"Twenty is the CCI default value\" / \"ending up with 20 as a standard\" (p.63)",
              "E|\"He suggested that long positions be established only when the CCI exceeds +100.\" \"The rules on the short side are the same: sell below -100, buy back above -100.\" (p.63)",
              "X|\"A subsequent drop below +100 becomes a signal to close out the long positions\"; short: \"buy back above -100\" (p.63)",
              "E|Signals on the close, fills at the next open [book silent on fills]."],
            "Lambert's original rules as described by the authors; they prefer zero-line or confirmed entries (p.64-67), which are not complete rule sets. Index here."),
        make("lebcafm_et_ma918", "Entry test · 9/18 MA crossover", "Ch. 3 · Testing Entries p.173", "Long/Short", Status::Live,
            &["E|\"the 9-18 crossover ... a long entry is generated when the 9 crosses above the 18. A short entry occurs when the 9 crosses below the 18.\" (p.173) Entry at the next open (p.173).",
              "X|\"Exits were on the close of days 5, 10, 15, and 20 after entry.\" (p.173)"], et_note),
        make("lebcafm_et_channel10", "Entry test · 10-day channel breakout", "Ch. 3 · Testing Entries p.174", "Long/Short", Status::Live,
            &["E|\"enter the market on a new intraday high or low of the last n days. We've chosen 10 days\" (p.174): buy stop just above the highest high of the previous 10 bars (fill at that high or the open if gapped), sell stop below the lowest low. A bar breaking both is skipped [book silent].",
              "X|Close of day N after entry (p.173)."], et_note),
        make("lebcafm_et_rsi14", "Entry test · RSI 14 overbought/oversold", "Ch. 3 · Testing Entries p.175", "Long/Short", Status::Live,
            &["E|\"the common 14-bar RSI, selling when it is overbought (above 75) and buying when it is oversold (below 25)\" (p.175). Entry at the next open.",
              "X|Close of day N after entry (p.173)."], et_note),
        make("lebcafm_et_cci10", "Entry test · CCI 10 zero cross", "Ch. 3 · Testing Entries p.175", "Long/Short", Status::Live,
            &["E|\"buying or selling when it crosses the zero line, the direction being determined by the CCI's direction. We used a 10-bar CCI.\" (p.175) Entry at the next open.",
              "X|Close of day N after entry (p.173)."], et_note),
        make("lebcafm_et_mom10", "Entry test · 10-day momentum zero cross", "Ch. 3 · Testing Entries p.176", "Long/Short", Status::Live,
            &["E|\"The momentum study uses a simple calculation that plots the difference between today's market close and the close n days ago. We used 10 days and entered when the zero line was crossed.\" (p.176) Entry at the next open.",
              "X|Close of day N after entry (p.173)."], et_note),
        make("lebcafm_et_vol", "Entry test · volatility breakout", "Ch. 3 · Testing Entries p.176", "Long/Short", Status::Live,
            &["E|\"A market is entered intraday on a breakout in either direction of p% of n day's average true range (ATR)\" (p.176), measured from the prior close (p.141-142). Tables: 150%-10 days (default), 100%-10 days, 100%-5 days.",
              "X|Close of day N after entry (p.173)."], et_note),
        make("lebcafm_xt_benchmark", "Exit test · MA reversal benchmark", "Ch. 3 · Testing Exits p.179-180", "Long/Short", Status::Live,
            &["E|Dual SMA crossover: \"A long entry will be signaled when the shorter-term moving average crosses over the longer-term moving average, and a short entry will be generated by the opposite crossover.\" Close out and reverse at the next open (p.179).",
              "X|Exit only at the opposite crossover: \"The exits for the initial benchmark test will be the moving average reversals\" (p.179)."], xt_note),
        make("lebcafm_xt_rsi_trail", "Exit test · RSI trail", "Ch. 3 · Testing Exits p.181", "Long/Short", Status::Live,
            &["E|Benchmark dual SMA crossover reversal at the next open (p.179).",
              "X|\"When a 9-day RSI gets above 75 or below 25 and the market closes against the direction of the trade, we exit on the opening the next day. The idea is to exit the market on strength\" (p.181). Read: long exits when RSI(9) > 75 and close < prior close on the same day; short when RSI(9) < 25 and close > prior close."], xt_note),
        make("lebcafm_xt_key_reversal", "Exit test · key reversal", "Ch. 3 · Testing Exits p.181", "Long/Short", Status::Live,
            &["E|Benchmark dual SMA crossover reversal at the next open (p.179).",
              "X|\"For our test we've defined our key reversal as a new high for the last 10 days, followed by a close lower than the previous day. A key reversal to exit a short position is a new low for the last 10 days, followed by a close higher than the previous day.\" (p.181) Read: on one bar (high above the prior 10 bars' highs and close below the prior close). Exit at the next open [book silent on the fill; same as the RSI trail]."], xt_note),
        make("lebcafm_xt_hl", "Exit test · n-day high/low trailing stop", "Ch. 3 · Testing Exits p.181", "Long/Short", Status::Live,
            &["E|Benchmark dual SMA crossover reversal at the next open (p.179).",
              "X|\"a stop that trails at the high or low of the last n days. The stop is triggered when the market corrects to the stop point. We did two tests, one using a high or low of the last 3 days, the other a high or low of the last 10 days.\" (p.181) Long stop = lowest low of the previous n bars; fill at the stop or the open. p.n = 3 (default) or 10."], xt_note),
        make("lebcafm_xt_volatility", "Exit test · volatility exit", "Ch. 3 · Testing Exits p.184", "Long/Short", Status::Live,
            &["E|Benchmark dual SMA crossover reversal at the next open (p.179).",
              "X|\"The market is exited on a volatility breakout against the direction of the trend. We used 100 percent of five day's average true range to define our volatility exit signal.\" (p.184) Long stop = prior close - 1.0 x mean 5-day true range; fill at the stop or the open."], xt_note),
        make("lebcafm_st_noprofit", "Stop test · no-profit exit", "Ch. 3 · Testing Stops p.187", "Long/Short", Status::Live,
            &["E|Benchmark dual SMA crossover reversal at the next open (p.186).",
              "X|\"test each trade a given number of days after its entry and see if the trade is profitable. We used 1, 5, and 10 days in our testing. If there is no profit after the specified number of days, the trade will be closed out on the opening the following day. If the trade is profitable, it will be allowed to continue to its normal conclusion.\" (p.187) Profit measured on the close of day N after entry (entry day = day 0)."],
            "Ch. 3 'Testing Stops' p.185-188, Exhibit 3-15 (futures win% and $ P/L, Jan 1986-Dec 1990). p.days = 1 (first of 1/5/10); p.slow = 10 (first MA pair)."),
    ];
    let nt = |id, name, group, why, rules| make(id, name, group, "Long/Short", Status::NotTestable(why), rules, "");
    let nd = |id, name, group, why, rules| make(id, name, group, "Long/Short", Status::NeedsData(why), rules, "");
    v.extend([
        nt("lebcafm_et_stoch_cross", "Entry test · stochastic crossover with boundaries", "Ch. 3 · Testing Entries p.174", "the stochastic length and smoothing are not stated",
            &["E|\"Buy when %K crosses above %D after the oscillator has been below 25. Sell short when %K crosses below %D after having been above 75. Crossovers between 26 and 74 are ignored.\" (p.174)"]),
        nt("lebcafm_et_stoch_pop", "Entry test · stochastic pop", "Ch. 3 · Testing Entries p.174-175", "which stochastic line and the slowing period are not stated",
            &["E|\"trend in the direction of a 14-bar slow stochastic that is advancing through the 75 level or declining through the 25 level\" (p.174-175)"]),
        nt("lebcafm_et_random", "Entry test · random entries", "Ch. 3 · Testing Entries p.178", "random generator not specified",
            &["E|Random entries on the open the day after the previous exit (p.178, Table 11)"]),
        nt("lebcafm_xt_parabolic", "Exit test · 5/10-day Parabolic (System Writer)", "Ch. 3 · Testing Exits p.181", "the System Writer Plus Parabolic variant (n-day starting point) is not given as a formula",
            &["X|\"The Parabolic we tested is not calculated exactly according to Wilder's original formula ... allows the user to select the number of days back to scan for the extreme high or low starting point\" (p.181)"]),
        nt("lebcafm_xt_stochastic", "Exit test · 14-day slow stochastic cross", "Ch. 3 · Testing Exits p.184", "slow-stochastic smoothing periods are not stated",
            &["X|\"We exit a long when %K crosses below %D, and exit a short when %K crosses above %D. We used a 14-day slow stochastic.\" (p.184)"]),
        nt("lebcafm_xt_random", "Exit test · random exits", "Ch. 3 · Testing Exits p.184-185", "random number generator not specified",
            &["X|\"a number between 5 and 20 is randomly selected to determine the minimum number of days ... the first close against the direction of the trend will result in the trade being exited on the next day's open\" (p.184-185)"]),
        nd("lebcafm_xt_dollar", "Exit/stop tests · dollar trailing stops, targets, initial and break-even stops", "Ch. 3 · Testing Exits/Stops p.181-188", FUT,
            &["X|$500 / $1,000 trailing stop from the best close (p.181-184); $1,000 / $2,000 profit targets (p.184); $500-$2,000 initial risk stops (p.186); break-even after $500-$2,000 (p.187); dollar trail from close or from high/low (p.187-188)"]),
        nt("lebcafm_st_support", "Stop test · support/resistance (20-day swing)", "Ch. 3 · Testing Stops p.187", "\"recent swing highs and lows\" are not defined mechanically",
            &["X|\"We used recent swing highs and lows that occurred within the 20 days prior to our entries.\" (p.187)"]),
        nd("lebcafm_simple_system", "Creating a Simple Trading System (3/12 MA + ADX filter)", "Ch. 3 · Creating a Simple Trading System p.188-198",
            "futures: $1,500 initial and trailing dollar stops; the exit is System Writer's 10-day Parabolic variant (formula not given)",
            &["E|\"We'll try a dual 3/12 moving average, for entries only.\" (p.194)",
              "F|Test 2: \"We'll use an 18-day ADX ... we only buy or sell when an ADX uptick and a moving average crossover occur on the same day.\" (p.196)",
              "X|\"a 10-day Parabolic for exits, and $1,500 initial risk and trailing stops\" (p.194)"]),
    ]);
    // Chapter 2 rules that are guidance, cited from others, or duplicates elsewhere in the vault
    v.extend([
        nt("lebcafm_colby_dmi", "Colby & Meyers DI crossover with rising ADX", "Ch. 2 · DMI/ADX p.39-40", "only a range of periods (1-50 weeks) is reported; weekly NYSE",
            &["E|\"entered on +DI and -DI crossovers only when the ADX was rising. They exited when the ADX ticked down or when a reverse crossover occurred.\" (p.39)"]),
        nt("lebcafm_williams_r", "Williams %R rules", "Ch. 2 · Percent R p.115-116", "no exit rule is given",
            &["E|Buy when \"Percent R has reached 100. Five trading days have elapsed since the last 100 was hit. Percent R subsequently falls below 95.\" (p.115)"]),
        nt("lebcafm_pnf", "Point-and-figure column reversal", "Ch. 2 · Point and Figure p.119-120", "box size is set once by hand ('a little rounding'), plotting method (close vs high/low) not stated",
            &["E|\"buy whenever you start a new column of Xs and to reverse and go short whenever you start a new column of Os\"; box = half the difference of 10-day averages of highs and lows, three-box reversal (p.120)"]),
        nt("lebcafm_envelope", "Envelope breakout", "Ch. 2 · Envelopes p.55-56", "no moving-average length or band width is given for any market we hold",
            &["E|\"Enter the market in the direction of the breakout when a band is penetrated\"; exit at the opposite band or at the moving average (p.55)"]),
    ]);
    // Chapter 4 day-trading methods (also "Day Trading Systems & Methods", same text)
    v.extend([
        make("lebcafm_dt_5_25", "5-25 Envelope Method (30-minute bars, fade the envelope)", "Ch. 4 · Day trading p.204-205 (DTSM ch. 2)", "Long/Short", Status::NotTestable("coded on real SPX 1-minute bars in lane K57 (decision 236: 30-minute bars, 5-bar envelope at +/-0.25%, the 150/5-point filters as the % at the book's ~380, the 75-point break-even trigger as 0.75 literal S&P index points, SPX only), but 1,853 of 3,593 SPX trades (51.6%, 2014-01-02..2026-08) depend on the order inside one minute: the minute that reaches the 0.75-point trigger also comes back to the entry price, and which came first decides between a break-even exit and a held trade (decision 175 limit 5%). #177b on tradable SPY with the points scaled passes (SPX avg +0.0104% vs SPY +0.0039%, compounded +40.9% vs +13.6%, 2014-01-02..2025-10-21). No result is published. authors/titles-1030/K57w/"),
            &["F|\"Use 30-minute bars on the S&P futures. Set up an envelope study for five periods, normal (no smoothing), and at a distance of 25/100 of 1 percent from the closes.\" (rules 1-2, p.204) = 5-bar simple average of closes, bands x (1 +/- 0.0025)",
              "F|\"Look for trades only when the boundaries of the envelope are at least 150 points apart.\" (rule 3) 150 points = 1.50 index points (Exhibit 4-1 axis 34550-35100 = 345.50-351.00). Default: the % equivalent at the book's ~380 (1.50/380 of the average, decision 236 / 207); p.points = 1 uses the literal 1.50 points. +/-0.25% bands are always 0.50% apart, so on either reading this filter never binds on SPX [Declared reading]",
              "E|\"When one of the 30-minute bars closes at least 5 points outside the envelope, look to initiate a trade in the opposite direction as soon as the next bar closes back inside the envelope.\" (rule 3) 5 points = 0.05 index points; default the % equivalent (0.05/380 of the average), p.points = 1 the literal 0.05. Read: a close at least that far above the upper band arms a short (below the lower band: a long); the first later 30-minute close back inside the bands (no time limit; the stricter reading, only the very next bar, is not coded) is the signal, filled at the next bar's open; a close beyond the opposite band disarms; arming and the excursion are tracked while a position is open, but a signal is taken only if no position is open at its close (Codex review) [Declared reading]",
              "X|\"Use an initial stop loss at the extreme high or low point just before your entry. After the market has moved 75 points in your favor, the stop loss should be changed to at least your break-even point.\" (rule 4) Read: the stop = the highest high (short) / lowest low (long) of the excursion: the run of 30-minute closes beyond the band that includes the arming bar, through the signal bar; 75 points = 0.75 S&P index points, literal (decision 236, 196(2)), then the stop moves to the entry price [Declared reading]",
              "X|\"Take profits when the market reaches the opposite side of the envelope. If you want to simplify the profit taking, use the boundary of the envelope at the time you enter the trade as the target, otherwise you might have to adjust your exit point every half hour.\" (rule 5) Default: the opposite band at the signal bar's close, a fixed limit; p.band = 1 re-sets it to the opposite band at each 30-minute close [labelled option]. No close-of-day exit is printed, so none is coded: positions carry overnight",
              "A|Read: session-aligned 30-minute bars built from the vault's SPX 1-minute file (09:30-16:00 ET), one continuous series across days. Stops and the target fill at their level, or at the minute's open when it opens through; inside a minute the price runs open, the nearer extreme, the farther extreme, close (a tie: the extreme against the position first) [convention]; a minute whose fills differ between the two orders is counted as ambiguous (decision 175)"],
            "Lane K57 (decision 236): LeBeau & Lucas Ch.4 p.204-205 (PDF 212-213, OCR batch-h3/ocr2/cafm_pdf212*.txt); the chapter's methods are 1992 S&P futures day-trading methods. The 75-point break-even trigger is S&P points, so the card runs on SPX only, from 2014 where the SPX opens are real. Exhibit 4-1 is a chart, not a results table: no book result to compare. No costs; cash-index shorts are hypothetical."),
        nt("lebcafm_dt_hi_mom", "Hi MOM system", "Ch. 4 · p.205-206 (DTSM ch. 3)", "divergence spotting and peak/valley trailing stops are chart judgement",
            &["E|9-minute S&P bars, six-bar momentum, divergence whose first spike penetrates +/-150; enter after the hook (p.205)"]),
        nt("lebcafm_dt_ohama", "Ohama 3-D intermarket divergence", "Ch. 4 · p.207 (DTSM ch. 4)", "\"This method is not a complete system, because of the lack of specific stops and the lack of a more specific exit strategy\" (p.207)",
            &["E|Five-minute charts of related markets; trade the divergence in the most liquid one (p.207)"]),
        make("lebcafm_dt_kane", "Kane's %K hooks (5-minute 12-bar slow stochastic, hourly trend filter)", "Ch. 4 · Day trading p.209-210 (DTSM ch. 5)", "Long/Short", Status::NotTestable("coded on real SPX 1-minute bars in lane K57 (decision 236: 5-minute 12-bar slow stochastic with the book's own Ch.2 3-bar smoothing, hourly trend filter, 1.00-point stop literal, SPX only; 0 of 7,423 SPX trades depend on the order inside one minute), but on tradable SPY with the stop scaled by the trailing 250-day SPX/SPY ratio (decision 177b, 2014-01-02..2025-10-21) the average trade has the opposite sign (scaled SPY -0.0004% over 6,315 trades, compounded -2.9%; SPX +0.0113% over 6,972, +117.8%): test a fires, so the SPX minute result is withheld. No result is published. authors/titles-1030/K57w/"),
            &["F|\"Determine the trend using one-hour charts. Trade only when the hourly chart has made a higher high or lower low within the last two hours. When the trend is up, look for buy signals only. When the trend is down, look for sell signals only.\" (rule 1, p.209) Read: session-aligned 1-hour bars (09:30 ET; the last one 15:30-16:00); at a 5-minute close, up = one of the last two completed hourly bars made a high above the hourly bar before it, down = a low below it; both or neither = no trend, no entry [Declared reading]",
              "F|\"Use a five-minute chart with a 12-period slow stochastics.\" (rule 2) The book's own definition (Ch.2 'Slow Stochastics' p.131, PDF 139, OCR K57w/ocr_pdf139.txt): \"%K = today's close minus the low of the last n days, divided by the high of the last n days minus the low of the last n days. %D is a three-day moving average of %K ... Fast %K and %D smoothed by three again produce the slow stochastic\": slow %K = the 3-bar simple average of fast %K(12) [book definition; simple average = Declared reading, 50 when the 12-bar range is 0]",
              "E|\"Buy when the %K (the faster moving line) goes below 20 and turns up. Sell short when the %K goes above 80 and turns down.\" (rule 2) Read: a buy hook = slow %K one bar ago below 20, at or under the bar before it, and higher now; a sell hook mirrors above 80; with the hourly trend agreeing it fills at the next 5-minute bar's open [Declared reading]",
              "X|\"Use an initial stop of 100 points\" (rule 3) = 1.00 S&P index point from the fill, literal (Exhibit 4-4 axis 37600-38200 = 376.00-382.00; decision 236, 196(2)); \"or put a closer stop just beyond a recent trading range\" is a judgement and is not modelled",
              "X|\"Take profits when the %K hooks in the opposite direction from +80 or -20.\" (rule 4) Read: a long exits on a sell hook, a short on a buy hook, at the next bar's open, whatever the trend; if the trend agrees the opposite entry is taken there too (reverse). No close-of-day exit is printed, so positions carry overnight [Declared reading]",
              "A|Not modelled (advice, decision 236): the break-even move ('a good idea'), the 'very sudden' 100-point profit take, the range stop, the one-minute stochastic divergence exit and the stop-after-two-losses rule. Minute path: the stop fills at its level, or at the minute's open when it opens through [convention]"],
            "Lane K57 (decision 236): LeBeau & Lucas Ch.4 p.209-210 (PDF 217-218, OCR batch-h3/ocr2/cafm_pdf217*.txt, 218). 5-minute and 1-hour bars built from the vault's SPX 1-minute file, one continuous series across days. The 100-point stop is S&P points, so the card runs on SPX only, from 2014 where the SPX opens are real. Exhibit 4-4 is a chart, not a results table: no book result to compare. No costs; cash-index shorts are hypothetical."),
        nt("lebcafm_dt_chang", "One-minute charts with stochastics", "Ch. 4 · p.211 (DTSM ch. 6)", "trailing stops \"using swing patterns\" are chart judgement",
            &["E|1-minute S&P bars, 21-period stochastic, both lines beyond 80/20 then cross, first hour only (p.211)"]),
        nt("lebcafm_dt_pivots", "Pivot points", "Ch. 4 · p.212-213 (DTSM ch. 7)", "support/resistance levels only; no entry or exit rules",
            &["F|Average = (H + L + C)/3; pivot high = 2 x average - low; pivot low = 2 x average - high (p.212)"]),
        nt("lebcafm_dt_gaps", "Babcock price gaps on openings", "Ch. 4 · p.214-215 (DTSM ch. 8)", "\"Bruce was probably intentionally vague about defining some of the instructions\" (p.215): gap size, opening range and follow-through are left open",
            &["E|Gap \"noticeably\"; \"Tack on a few points above or below the opening price range and enter ... on a buy stop or sell stop\"", "X|\"Use a stop loss of about $500 and exit the trade on the close\""]),
        nt("lebcafm_dt_rsi_div", "RSI divergences (30-minute)", "Ch. 4 · p.216-217 (DTSM ch. 9)", "divergence patterns are chart judgement",
            &["E|30-minute S&P, six-period RSI, divergence whose first spike penetrated 80/20 (p.216-217)"]),
        nt("lebcafm_dt_knife", "Sibbet's Knife system", "Ch. 4 · p.218-219 (DTSM ch. 10)", "\"Identify a fairly recent significant high or low point on the chart. (Just eyeball it for now.)\" (p.218)",
            &["E|Sell 0.70 below a significant high / buy 0.70 above a significant low (NYFE)"]),
        nt("lebcafm_dt_stoch_div", "Stochastic divergences (3-minute)", "Ch. 4 · p.220 (DTSM ch. 11)", "divergence patterns are chart judgement",
            &["F|18-day ADX/DMI trend filter; half-hour stochastics", "E|3-minute 21-period slow stochastic divergence from beyond 80/20 (p.220)"]),
        nt("lebcafm_dt_swing_rev", "Swing reversals plus stochastics", "Ch. 4 · p.221-222 (DTSM ch. 12)", "\"a new short-term low\" has no stated lookback, and the stop alternative is a 'tighter dollar stop' of unstated size; also intraday (30-minute bars)",
            &["E|30-minute S&P bars, nine-period slow stochastic; key reversal with %D below 30 (buy) or above 70 (sell); buy the open of bar 4 (Exhibit 4-10)", "X|Stop a tick or two outside the swing; exit on an opposite key reversal or at the close"]),
    ]);
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("lebcafm_") { return None; }
    Some(match id {
        "lebcafm_adx_parabolic" | "lebcafm_babcock_dmi28" | "lebcafm_colby_dmi" | "lebcafm_simple_system" => ("lebcafm_adx", "LeBeau & Lucas ADX/DMI systems", ""),
        "lebcafm_vol_breakout" => ("lebcafm_vol", "Volatility breakout", "Reversal system"),
        "lebcafm_cci_lambert" => ("lebcafm_cci", "CCI", "Lambert +/-100"),
        _ if id.starts_with("lebcafm_et_") => ("lebcafm_entry_tests", "Ch. 3 entry tests (exit after N days)", ""),
        _ if id.starts_with("lebcafm_xt_") || id.starts_with("lebcafm_st_") => ("lebcafm_exit_tests", "Ch. 3 exit/stop tests on the MA benchmark", ""),
        _ if id.starts_with("lebcafm_dt_") => ("lebcafm_daytrading", "Ch. 4 day-trading methods", ""),
        _ => ("lebcafm_other", "Ch. 2 indicator rules", ""),
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let hold = p("hold", "Exit on the close of day N after entry (book: 5, 10, 15, 20)", 5., 1., 40., 1.);
    let ma = |v: &mut Vec<ParamDef>| { v.push(p("fast", "Fast SMA", 5., 2., 20., 1.)); v.push(p("slow", "Slow SMA (book: 10, 20, 30, 40)", 10., 5., 60., 1.)); };
    Some(match id {
        "lebcafm_adx_parabolic" => vec![p("adx_n", "ADX/DMI days", 18., 5., 40., 1.), p("af0", "Parabolic starting AF", 0.02, 0.005, 0.1, 0.005), p("afstep", "AF step", 0.02, 0.005, 0.1, 0.005), p("afmax", "AF maximum", 0.2, 0.05, 0.5, 0.01)],
        "lebcafm_vol_breakout" => vec![p("n", "ATR days (mean true range)", 5., 2., 30., 1.), p("pct", "Percent of ATR", 150., 50., 300., 5.)],
        "lebcafm_babcock_dmi28" => vec![p("n", "DMI days", 28., 5., 60., 1.)],
        "lebcafm_cci_lambert" => vec![p("n", "CCI days", 20., 5., 60., 1.), p("level", "Entry/exit level", 100., 50., 200., 5.)],
        "lebcafm_et_ma918" => vec![p("fast", "Fast SMA", 9., 2., 30., 1.), p("slow", "Slow SMA", 18., 5., 60., 1.), hold],
        "lebcafm_et_channel10" => vec![p("n", "Channel days", 10., 2., 60., 1.), hold],
        "lebcafm_et_rsi14" => vec![p("n", "RSI days", 14., 2., 30., 1.), p("ob", "Overbought (sell above)", 75., 50., 95., 1.), p("os", "Oversold (buy below)", 25., 5., 50., 1.), hold],
        "lebcafm_et_cci10" => vec![p("n", "CCI days", 10., 3., 40., 1.), hold],
        "lebcafm_et_mom10" => vec![p("n", "Momentum days", 10., 2., 40., 1.), hold],
        "lebcafm_et_vol" => vec![p("pct", "Percent of ATR (book: 150, 100)", 150., 25., 300., 5.), p("n", "ATR days (book: 10, 5)", 10., 2., 30., 1.), hold],
        "lebcafm_xt_benchmark" | "lebcafm_xt_key_reversal" | "lebcafm_xt_volatility" => { let mut v = vec![]; ma(&mut v); v }
        "lebcafm_xt_rsi_trail" => { let mut v = vec![]; ma(&mut v); v.push(p("rsi_n", "RSI days", 9., 2., 30., 1.)); v }
        "lebcafm_xt_hl" => { let mut v = vec![]; ma(&mut v); v.push(p("n", "Trailing stop days (book: 3, 10)", 3., 1., 30., 1.)); v }
        "lebcafm_st_noprofit" => { let mut v = vec![]; ma(&mut v); v.push(p("days", "Days after entry (book: 1, 5, 10)", 1., 1., 20., 1.)); v }
        // intraday EP block (lane K57): S&P index points, SPX only
        "lebcafm_dt_5_25" => vec![p("n", "Envelope moving average (30-minute bars)", 5., 2., 50., 1.), p("pct", "Envelope distance (% of the average; book 0.25)", 0.25, 0.05, 2., 0.05),
            p("be", "Break-even trigger (S&P points in favour; book 75 points = 0.75)", 0.75, 0.05, 20., 0.05),
            p("points", "Filters: 0 = % equivalent at the book's ~380 (default), 1 = literal 1.50 / 0.05 points", 0., 0., 1., 1.),
            p("band", "Target: 0 = opposite band at entry (default), 1 = opposite band re-set every 30 minutes", 0., 0., 1., 1.)],
        "lebcafm_dt_kane" => vec![p("n", "Stochastic length (5-minute bars)", 12., 3., 50., 1.), p("smooth", "Slow %K smoothing (book Ch.2: 3)", 3., 1., 10., 1.),
            p("lower", "Buy hook below", 20., 1., 50., 1.), p("upper", "Sell hook above", 80., 50., 99., 1.),
            p("stop", "Initial stop (S&P points; book 100 points = 1.00)", 1., 0.1, 20., 0.05)],
        _ => return None,
    })
}

// ---------------------------------------------------------------- indicators (local, book definitions)

fn mean_tr(x: &Ctx, n: usize) -> Vec<f64> { sma(&true_range(x.r), n) }

/// Lambert CCI: (TP - SMA(TP)) / (0.015 x mean |TP - SMA(TP)|) over the same n bars.
fn cci(x: &Ctx, n: usize) -> Vec<f64> {
    let tp: Vec<f64> = (0..x.c.len()).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    let m = sma(&tp, n);
    let mut out = vec![f64::NAN; tp.len()];
    for i in n.saturating_sub(1)..tp.len() {
        let md = tp[i + 1 - n..=i].iter().map(|v| (v - m[i]).abs()).sum::<f64>() / n as f64;
        if md > 0. { out[i] = (tp[i] - m[i]) / (0.015 * md); }
    }
    out
}

/// Wilder Parabolic over the whole series: (SAR in force during bar i, direction after bar i).
fn parabolic(x: &Ctx, a0: f64, step: f64, amax: f64) -> (Vec<f64>, Vec<i8>) {
    let n = x.c.len();
    let (mut sar, mut dir) = (vec![f64::NAN; n], vec![0i8; n]);
    if n < 3 { return (sar, dir); }
    let mut d: i8 = if x.c[1] >= x.c[0] { 1 } else { -1 };
    let mut ep = if d == 1 { x.h[1] } else { x.l[1] };
    let mut next = if d == 1 { x.l[0].min(x.l[1]) } else { x.h[0].max(x.h[1]) };
    let mut af = a0;
    dir[0] = d;
    dir[1] = d;
    for i in 2..n {
        let s = next;
        sar[i] = s;
        if (d == 1 && x.l[i] <= s) || (d == -1 && x.h[i] >= s) {
            let old = ep;
            d = -d;
            af = a0;
            if d == -1 { ep = x.l[i]; next = old.max(x.h[i]).max(x.h[i - 1]); } else { ep = x.h[i]; next = old.min(x.l[i]).min(x.l[i - 1]); }
        } else {
            if d == 1 && x.h[i] > ep { ep = x.h[i]; af = (af + step).min(amax); }
            if d == -1 && x.l[i] < ep { ep = x.l[i]; af = (af + step).min(amax); }
            next = s + af * (ep - s);
            next = if d == 1 { next.min(x.l[i]).min(x.l[i - 1]) } else { next.max(x.h[i]).max(x.h[i - 1]) };
        }
        dir[i] = d;
    }
    (sar, dir)
}

/// +1 when a crosses above b on bar k (a > b today, a <= b yesterday), -1 for the opposite, else 0.
fn cross(a: &[f64], b: &[f64], k: usize) -> i8 {
    if k == 0 { return 0; }
    let (a0, b0, a1, b1) = (a[k], b[k], a[k - 1], b[k - 1]);
    if [a0, b0, a1, b1].iter().any(|v| v.is_nan()) { return 0; }
    // running-sum SMAs carry float noise: values within 1e-9 (relative) are a tie, and a tie is not a cross
    let cmp = |a: f64, b: f64| if (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.) { 0 } else if a > b { 1 } else { -1 };
    let (now, before) = (cmp(a0, b0), cmp(a1, b1));
    if now == 1 && before <= 0 { 1 } else if now == -1 && before >= 0 { -1 } else { 0 }
}

fn open_trade(dir: f64, i: usize, px: f64, signal_i: usize, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i, open: false, exits: vec![] }
}
fn close(mut t: Trade, i: usize, px: f64, signal_i: usize, reason: &str) -> Trade {
    t.exit_i = i;
    t.exit_px = px;
    t.signal_i = signal_i;
    t.reason = reason.into();
    t
}
/// Stop fill for an exit (dir = side of the open trade) or a stop entry (dir = side entered).
fn stop_exit(x: &Ctx, i: usize, dir: f64, stop: f64) -> f64 { if dir > 0. { x.o[i].min(stop) } else { x.o[i].max(stop) } }
fn stop_entry(x: &Ctx, i: usize, dir: f64, lvl: f64) -> f64 { if dir > 0. { x.o[i].max(lvl) } else { x.o[i].min(lvl) } }
fn hit_exit(x: &Ctx, i: usize, dir: f64, stop: f64) -> bool { if dir > 0. { x.l[i] <= stop } else { x.h[i] >= stop } }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    match id { // intraday EP block (lane K57)
        "lebcafm_dt_5_25" => return Some(env_5_25(x, from, to, u("n"), g("pct") / 100., g("be"), g("points") >= 0.5, g("band") >= 0.5)),
        "lebcafm_dt_kane" => return Some(kane(x, from, to, u("n"), u("smooth"), g("lower"), g("upper"), g("stop"))),
        _ => {}
    }
    let mut out: Vec<Trade> = vec![];
    let mut cur: Option<Trade> = None;
    match id {
        "lebcafm_adx_parabolic" => {
            let (pdi, mdi, adx) = super::lebtc::dmi3(x, u("adx_n"));
            let (sar, pd) = parabolic(x, g("af0"), g("afstep"), g("afmax"));
            for i in from.max(3)..=to {
                let k = i - 1;
                let mut exited = false;
                if let Some(t) = cur.as_ref() {
                    if hit_exit(x, i, t.dir, sar[i]) {
                        let px = stop_exit(x, i, t.dir, sar[i]);
                        out.push(close(cur.take().unwrap(), i, px, i, "Parabolic stop"));
                        exited = true;
                    } else { continue; }
                }
                if !(adx[k] > adx[k - 1]) { continue; }
                let dd: i8 = if pdi[k] > mdi[k] { 1 } else if mdi[k] > pdi[k] { -1 } else { 0 };
                if dd == 0 { continue; }
                let d = dd as f64;
                if !exited && pd[k] == dd {
                    let t = open_trade(d, i, x.o[i], k, "open");
                    if hit_exit(x, i, d, sar[i]) {
                        let px = stop_exit(x, i, d, sar[i]);
                        out.push(close(t, i, px, i, "Parabolic stop"));
                    } else { cur = Some(t); }
                } else if pd[k] == -dd && pd[i] == dd {
                    cur = Some(open_trade(d, i, stop_entry(x, i, d, sar[i]), k, "stop"));
                }
            }
        }
        "lebcafm_vol_breakout" => {
            let (a, f) = (mean_tr(x, u("n")), g("pct") / 100.);
            for i in from.max(1)..=to {
                let k = i - 1;
                if a[k].is_nan() { continue; }
                let (buy, sell) = (x.c[k] + f * a[k], x.c[k] - f * a[k]);
                match cur.as_ref().map(|t| t.dir) {
                    None => {
                        let (hb, hs) = (x.h[i] >= buy, x.l[i] <= sell);
                        if hb != hs {
                            let d = if hb { 1. } else { -1. };
                            cur = Some(open_trade(d, i, stop_entry(x, i, d, if hb { buy } else { sell }), k, "stop"));
                        }
                    }
                    Some(d) => {
                        let lvl = if d > 0. { sell } else { buy };
                        if hit_exit(x, i, d, lvl) {
                            let px = stop_exit(x, i, d, lvl);
                            out.push(close(cur.take().unwrap(), i, px, i, "Volatility breakout reversal"));
                            cur = Some(open_trade(-d, i, px, k, "stop"));
                        }
                    }
                }
            }
        }
        "lebcafm_babcock_dmi28" => {
            let (pdi, mdi, _) = super::lebtc::dmi3(x, u("n"));
            for i in from..=to {
                let d = if pdi[i] > mdi[i] { 1. } else if mdi[i] > pdi[i] { -1. } else { 0. };
                if d == 0. || cur.as_ref().is_some_and(|t| t.dir == d) { continue; }
                if let Some(t) = cur.take() { out.push(close(t, i, x.c[i], i, "Net DM reversed · close")); }
                cur = Some(open_trade(d, i, x.c[i], i, "close"));
            }
        }
        "lebcafm_cci_lambert" => {
            let (c, lv) = (cci(x, u("n")), g("level"));
            for i in from.max(1)..=to {
                let k = i - 1;
                if let Some(t) = cur.as_ref() {
                    if (t.dir > 0. && c[k] < lv) || (t.dir < 0. && c[k] > -lv) {
                        out.push(close(cur.take().unwrap(), i, x.o[i], k, "CCI back inside +/-100 · next open"));
                    }
                }
                if cur.is_none() {
                    if c[k] > lv { cur = Some(open_trade(1., i, x.o[i], k, "open")); } else if c[k] < -lv { cur = Some(open_trade(-1., i, x.o[i], k, "open")); }
                }
            }
        }
        _ if id.starts_with("lebcafm_et_") => {
            let hold = g("hold").round().max(1.) as usize;
            let zero = vec![0.; x.c.len()];
            let (f, s) = (sma(&x.c, if id == "lebcafm_et_ma918" { u("fast") } else { 1 }), sma(&x.c, if id == "lebcafm_et_ma918" { u("slow") } else { 1 }));
            let r = rsi(&x.c, if id == "lebcafm_et_rsi14" { u("n") } else { 2 });
            let cc = if id == "lebcafm_et_cci10" { cci(x, u("n")) } else { zero.clone() };
            let mn = if id == "lebcafm_et_mom10" { u("n") } else { 1 };
            let mom: Vec<f64> = (0..x.c.len()).map(|i| if i >= mn { x.c[i] - x.c[i - mn] } else { f64::NAN }).collect();
            let a = if id == "lebcafm_et_vol" { mean_tr(x, u("n")) } else { zero.clone() };
            for i in from.max(1)..=to {
                let k = i - 1;
                if let Some(t) = cur.as_ref() {
                    if i == t.legs[0].i + hold { out.push(close(cur.take().unwrap(), i, x.c[i], i, &format!("Close of day {hold} after entry"))); }
                    continue;
                }
                match id {
                    "lebcafm_et_channel10" | "lebcafm_et_vol" => {
                        let (up, dn, hb, hs) = if id == "lebcafm_et_channel10" {
                            let n = u("n");
                            if i < n { continue; }
                            let up = x.h[i - n..i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                            let dn = x.l[i - n..i].iter().copied().fold(f64::INFINITY, f64::min);
                            (up, dn, x.h[i] > up, x.l[i] < dn)
                        } else {
                            if a[k].is_nan() { continue; }
                            let f = g("pct") / 100.;
                            let (up, dn) = (x.c[k] + f * a[k], x.c[k] - f * a[k]);
                            (up, dn, x.h[i] >= up, x.l[i] <= dn)
                        };
                        if hb != hs {
                            let d = if hb { 1. } else { -1. };
                            cur = Some(open_trade(d, i, stop_entry(x, i, d, if hb { up } else { dn }), k, "stop"));
                        }
                    }
                    _ => {
                        let d = match id {
                            "lebcafm_et_ma918" => cross(&f, &s, k),
                            "lebcafm_et_rsi14" => if r[k] < g("os") { 1 } else if r[k] > g("ob") { -1 } else { 0 },
                            "lebcafm_et_cci10" => cross(&cc, &zero, k),
                            _ => cross(&mom, &zero, k),
                        };
                        if d != 0 { cur = Some(open_trade(d as f64, i, x.o[i], k, "open")); }
                    }
                }
            }
        }
        _ => {
            // Ch. 3 exit/stop tests on the dual-MA reversal benchmark
            let (f, s) = (sma(&x.c, u("fast")), sma(&x.c, u("slow")));
            let r9 = rsi(&x.c, if id == "lebcafm_xt_rsi_trail" { u("rsi_n") } else { 9 });
            let a5 = mean_tr(x, 5);
            let mut armed = false;
            for i in from.max(1)..=to {
                let k = i - 1;
                let xo = cross(&f, &s, k);
                if xo != 0 {
                    if let Some(t) = cur.take() { out.push(close(t, i, x.o[i], k, "MA crossover · next open")); }
                    cur = Some(open_trade(xo as f64, i, x.o[i], k, "open"));
                    armed = false;
                } else if armed {
                    let why = match id { "lebcafm_xt_rsi_trail" => "RSI trail · next open", "lebcafm_xt_key_reversal" => "Key reversal · next open", _ => "No profit after N days · next open" };
                    if let Some(t) = cur.take() { out.push(close(t, i, x.o[i], k, why)); }
                    armed = false;
                }
                let Some(d) = cur.as_ref().map(|t| t.dir) else { continue };
                let stop = match id {
                    "lebcafm_xt_hl" => { let n = u("n"); (i >= n).then(|| if d > 0. { x.l[i - n..i].iter().copied().fold(f64::INFINITY, f64::min) } else { x.h[i - n..i].iter().copied().fold(f64::NEG_INFINITY, f64::max) }) }
                    "lebcafm_xt_volatility" => (!a5[k].is_nan()).then(|| x.c[k] - d * a5[k]),
                    _ => None,
                };
                if let Some(st) = stop {
                    if hit_exit(x, i, d, st) {
                        let px = stop_exit(x, i, d, st);
                        let why = if id == "lebcafm_xt_hl" { "n-day high/low trailing stop" } else { "Volatility exit (100% of 5-day ATR)" };
                        out.push(close(cur.take().unwrap(), i, px, i, why));
                        continue;
                    }
                }
                let (ei, epx) = { let l = &cur.as_ref().unwrap().legs[0]; (l.i, l.px) };
                armed = match id {
                    "lebcafm_xt_rsi_trail" => (d > 0. && r9[i] > 75. && x.c[i] < x.c[i - 1]) || (d < 0. && r9[i] < 25. && x.c[i] > x.c[i - 1]),
                    "lebcafm_xt_key_reversal" => i >= 10 && ((d > 0. && x.h[i] > x.h[i - 10..i].iter().copied().fold(f64::NEG_INFINITY, f64::max) && x.c[i] < x.c[i - 1])
                        || (d < 0. && x.l[i] < x.l[i - 10..i].iter().copied().fold(f64::INFINITY, f64::min) && x.c[i] > x.c[i - 1])),
                    "lebcafm_st_noprofit" => i == ei + u("days") && d * (x.c[i] - epx) <= 0.,
                    _ => false,
                };
            }
        }
    }
    if let Some(t) = cur {
        let mut t = close(t, to, x.c[to], to, "open");
        t.open = true;
        out.push(t);
    }
    Some(out)
}

// ---- intraday EP block (lane K57, 5 Nov 2026, decision #236; authors/titles-1030/K57.tsv)
// Ch.4 day-trading methods on N-minute bars built from the vault's 1-minute file (n1intra::series over intraday_ep::day_bars,
// session-aligned), one continuous series across days (EP2); orders inside a bar walked by the K46 minute walker
// (n1intra::minute). Amounts in S&P index points: SPX only. Independent check: authors/titles-1030/K57w/k57_ref.py.
use super::n1intra::{minute, series, Ord};
use crate::intraday::day_trade;

/// A position: direction, entry price, entry day row, stop, target, break-even reached.
#[derive(Clone, Copy)]
struct Dp { d: f64, ep: f64, ei: usize, stop: f64, tgt: f64, be: bool }
/// Walker state: the position and the exits filled in the current bar (position, price, reason).
#[derive(Clone, Default)]
struct Ds { pos: Option<Dp>, done: Vec<(Dp, f64, &'static str)> }
impl Ds {
    fn walk(&mut self, z: &super::n1intra::Bn, orders: &dyn Fn(&Ds) -> Vec<Ord>, fill: &dyn Fn(&mut Ds, u8, f64), out: &mut Vec<Trade>) {
        for q in &z.m { let adv = self.pos.map_or(false, |p| p.d < 0.0); minute(q, self, orders, fill, adv); } // ambiguity: counted by the reference
        for (p, px, why) in self.done.drain(..) { out.push(day_trade(p.d, p.ei, p.ep, z.i, px, why, "open", false)); }
    }
    fn finish(self, b: &[super::n1intra::Bn], mut out: Vec<Trade>) -> Vec<Trade> {
        if let (Some(p), Some(z)) = (self.pos, b.last()) { out.push(day_trade(p.d, p.ei, p.ep, z.i, z.c, "open", "open", true)); }
        out
    }
}

/// 5-25 Envelope (p.204-205): 30-minute bars, n-bar SMA of closes +/- pct. A close at least `thr` beyond a band arms the
/// opposite trade; the first later close back inside (bands at least `wide` apart) enters at the next bar's open; stop at the
/// excursion extreme, moved to the entry once the price has gone `be` points in favour; target the opposite band.
fn env_5_25(x: &Ctx, from: usize, to: usize, n: usize, pct: f64, be: f64, points: bool, update: bool) -> Vec<Trade> {
    let b = series(x, from, to, 30);
    let ma = sma(&b.iter().map(|z| z.c).collect::<Vec<_>>(), n);
    let orders = |s: &Ds| -> Vec<Ord> {
        let Some(p) = s.pos else { return vec![] };
        let mut v = vec![Ord { up: p.d < 0.0, lv: p.stop, tag: 10 }, Ord { up: p.d > 0.0, lv: p.tgt, tag: 11 }];
        if !p.be { v.push(Ord { up: p.d > 0.0, lv: p.ep + p.d * be, tag: 12 }); }
        v
    };
    let fill = |s: &mut Ds, t: u8, px: f64| {
        let Some(mut p) = s.pos else { return };
        if t == 12 { p.be = true; p.stop = if p.d > 0.0 { p.stop.max(p.ep) } else { p.stop.min(p.ep) }; s.pos = Some(p); return; }
        s.pos = None;
        s.done.push((p, px, if t == 11 { "target" } else if p.be { "break-even stop" } else { "initial stop" }));
    };
    let (mut st, mut out) = (Ds::default(), vec![]);
    let (mut pend, mut armed, mut ext) = (None::<(f64, f64, f64)>, 0.0, 0.0);
    let (mut run_hi, mut run_lo) = (f64::NEG_INFINITY, f64::INFINITY); // extremes of the current run of closes beyond a band
    for (k, z) in b.iter().enumerate() {
        if let Some((d, stop, tgt)) = pend.take() { st.pos = Some(Dp { d, ep: z.m[0].o, ei: z.i, stop, tgt, be: false }); }
        st.walk(z, &orders, &fill, &mut out);
        if !ma[k].is_finite() { continue; }
        let (u, l) = (ma[k] * (1.0 + pct), ma[k] * (1.0 - pct));
        if update { if let Some(p) = st.pos.as_mut() { p.tgt = if p.d > 0.0 { u } else { l }; } }
        let (thr, wide) = if points { (0.05, 1.5) } else { (ma[k] * 0.05 / 380.0, ma[k] * 1.5 / 380.0) };
        let mut sig = 0.0;
        if z.c > u {
            run_hi = run_hi.max(z.h);
            if armed > 0.0 { armed = 0.0; }
            if armed < 0.0 { ext = f64::max(ext, z.h); } else if z.c - u >= thr { armed = -1.0; ext = run_hi; }
        } else if z.c < l {
            run_lo = run_lo.min(z.l);
            if armed < 0.0 { armed = 0.0; }
            if armed > 0.0 { ext = f64::min(ext, z.l); } else if l - z.c >= thr { armed = 1.0; ext = run_lo; }
        } else if armed != 0.0 {
            ext = if armed < 0.0 { ext.max(z.h) } else { ext.min(z.l) };
            sig = armed;
            armed = 0.0;
        }
        if z.c <= u { run_hi = f64::NEG_INFINITY; }
        if z.c >= l { run_lo = f64::INFINITY; }
        if sig != 0.0 && u - l >= wide && st.pos.is_none() { pend = Some((sig, ext, if sig > 0.0 { u } else { l })); }
    }
    st.finish(&b, out)
}

/// Kane's %K hooks (p.209-210): slow %K = `sm`-bar simple average of fast %K(n) on 5-minute bars (Ch.2 p.131); a hook under
/// `lo` buys / over `hi` sells at the next bar's open when the hourly trend agrees; the opposite hook exits; stop `stop` points.
fn kane(x: &Ctx, from: usize, to: usize, n: usize, sm: usize, lo: f64, hi: f64, stop: f64) -> Vec<Trade> {
    let (b, hb) = (series(x, from, to, 5), series(x, from, to, 60));
    let fast: Vec<f64> = (0..b.len()).map(|k| if k + 1 < n { f64::NAN } else {
        let w = &b[k + 1 - n..=k];
        let hh = w.iter().map(|z| z.h).fold(f64::NEG_INFINITY, f64::max);
        let ll = w.iter().map(|z| z.l).fold(f64::INFINITY, f64::min);
        if hh > ll { 100.0 * (b[k].c - ll) / (hh - ll) } else { 50.0 } }).collect();
    let sk = sma(&fast, sm);
    let orders = |s: &Ds| -> Vec<Ord> { s.pos.map_or(vec![], |p| vec![Ord { up: p.d < 0.0, lv: p.stop, tag: 10 }]) };
    let fill = |s: &mut Ds, _: u8, px: f64| { if let Some(p) = s.pos.take() { s.done.push((p, px, "initial stop")); } };
    let (mut st, mut out) = (Ds::default(), vec![]);
    let (mut pend, mut pend_exit, mut j) = (0.0, false, 0usize); // j = hourly bars completed so far
    for (k, z) in b.iter().enumerate() {
        if pend_exit || pend != 0.0 {
            let o = z.m[0].o;
            if let Some(p) = st.pos.filter(|p| pend_exit || p.d != pend) { out.push(day_trade(p.d, p.ei, p.ep, z.i, o, "%K hook", "open", false)); st.pos = None; }
            if pend != 0.0 && st.pos.is_none() { st.pos = Some(Dp { d: pend, ep: o, ei: z.i, stop: o - pend * stop, tgt: f64::NAN, be: false }); }
            (pend, pend_exit) = (0.0, false);
        }
        st.walk(z, &orders, &fill, &mut out);
        while j < hb.len() && (hb[j].i < z.i || (hb[j].i == z.i && hb[j].s <= z.s)) { j += 1; }
        if k < 2 || !(sk[k].is_finite() && sk[k - 1].is_finite() && sk[k - 2].is_finite()) { continue; }
        let buy = sk[k - 1] < lo && sk[k] > sk[k - 1] && sk[k - 1] <= sk[k - 2];
        let sell = sk[k - 1] > hi && sk[k] < sk[k - 1] && sk[k - 1] >= sk[k - 2];
        let trend = if j < 3 { 0.0 } else {
            let h = &hb[j - 3..j];
            let (up, dn) = (h[2].h > h[1].h || h[1].h > h[0].h, h[2].l < h[1].l || h[1].l < h[0].l);
            if up && !dn { 1.0 } else if dn && !up { -1.0 } else { 0.0 } };
        let d = st.pos.map_or(0.0, |p| p.d);
        if (d > 0.0 && sell) || (d < 0.0 && buy) { pend_exit = true; }
        if buy && trend > 0.0 && d <= 0.0 { pend = 1.0; } else if sell && trend < 0.0 && d >= 0.0 { pend = -1.0; }
    }
    st.finish(&b, out)
}
// ---- end intraday EP block

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{i:05}"), open: c, high: c + 1., low: c - 1., close: c, volume: None }).collect()
    }
    #[test]
    fn entry_test_exits_on_close_of_day_n() {
        // 9/18 cross up then hold 5 days
        let mut c: Vec<f64> = (0..40).map(|_| 100.).collect();
        c.extend((0..20).map(|i| 101. + i as f64));
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("lebcafm_et_ma918", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let t = &ts[0];
        assert_eq!(t.dir, 1.);
        assert_eq!(t.legs[0].i, 41); // cross on the close of bar 40, open of 41
        assert_eq!(t.exit_i, 46);
        assert_eq!(t.exit_px, c[46]);
    }
    // ---- lane K57: the Ch.4 minute rules on hand-built minutes (one minute per 5- or 30-minute bucket = that bar)
    fn minutes(name: &str, days: &[(&str, Vec<String>)]) -> &'static crate::intraday::Minutes {
        let dir = std::env::temp_dir().join(format!("k57{name}{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (mut body, mut idx) = (String::from("Date,Time,Open,High,Low,Close\n"), String::from("Date,offset,bytes\n"));
        for (d, lines) in days {
            let start = body.len();
            for l in lines { body.push_str(&format!("{d},{l}\n")); }
            idx.push_str(&format!("{d},{start},{}\n", body.len() - start));
        }
        std::fs::write(dir.join("T.m1.csv"), &body).unwrap();
        std::fs::write(dir.join("T.m1idx.csv"), &idx).unwrap();
        Box::leak(Box::new(crate::intraday::Minutes::load(dir.to_str().unwrap(), "T").unwrap()))
    }
    fn line(m: u16, o: f64, h: f64, l: f64, c: f64) -> String { format!("{:02}:{:02},{o},{h},{l},{c}", m / 60, m % 60) }
    fn day_rows(d: &[&str]) -> Vec<Row> { d.iter().map(|d| Row { ticker: "T".into(), date: (*d).into(), open: 1., high: 1., low: 1., close: 1., volume: None }).collect() }
    fn t4(t: &[Trade]) -> Vec<(f64, f64, f64, String)> { t.iter().map(|z| (z.dir, (z.legs[0].px * 1e6).round() / 1e6, (z.exit_px * 1e6).round() / 1e6, z.reason.clone())).collect() }
    #[test]
    fn k57_envelope_fade_target_and_break_even() {
        // 30-minute bars: five closes at 100 (MA 100, bands 99.75 / 100.25), bar 6 closes 100.6 (0.23 above its upper band
        // 100.37 >= 0.05/380 of the MA): short armed, excursion high 100.7; bar 7 closes 100.2, inside (bands 99.9096 /
        // 100.4104): short at bar 8's open 100.1, stop 100.7, target the lower band 99.9096, reached on bar 8's low.
        let flat = |m: u16| line(m, 100., 100.05, 99.95, 100.);
        let mut v: Vec<String> = (0..5).map(|k| flat(570 + 30 * k)).collect();
        v.push(line(720, 100.1, 100.7, 100.05, 100.6));
        v.push(line(750, 100.5, 100.5, 100.15, 100.2));
        let mut v1 = v.clone();
        v1.push(line(780, 100.1, 100.15, 99.8, 99.85));
        let r = day_rows(&["2020-01-02"]);
        let mut x = Ctx::new(&r, &r, &[]);
        x.minutes = Some(minutes("env1", &[("2020-01-02", v1)]));
        assert_eq!(t4(&run("lebcafm_dt_5_25", &x, 0, 0, &Params::new()).unwrap()), vec![(-1., 100.1, 99.909600, "target".into())]);
        // break-even trigger 0.1 (option): bar 8 reaches 100.0 (not the target), bar 9 comes back to the entry 100.1
        let mut v2 = v.clone();
        v2.push(line(780, 100.1, 100.12, 99.95, 100.05));
        v2.push(line(810, 100.05, 100.2, 100.0, 100.15));
        x.minutes = Some(minutes("env2", &[("2020-01-02", v2.clone())]));
        let mut p = Params::new();
        p.insert("be".into(), 0.1);
        assert_eq!(t4(&run("lebcafm_dt_5_25", &x, 0, 0, &p).unwrap()), vec![(-1., 100.1, 100.1, "break-even stop".into())]);
        // default 0.75 trigger is not reached: the position is still open at the end, marked open at the last close
        let t = run("lebcafm_dt_5_25", &x, 0, 0, &Params::new()).unwrap();
        assert_eq!((t4(&t), t[0].open), (vec![(-1., 100.1, 100.15, "open".into())], true));
        // literal-point filters (option): at a price of 100 the bands are 0.50 apart, under the literal 1.50: no trade
        p.insert("points".into(), 1.0);
        x.minutes = Some(minutes("env3", &[("2020-01-02", v2)]));
        assert!(run("lebcafm_dt_5_25", &x, 0, 0, &p).unwrap().is_empty());
    }
    #[test]
    fn k57_kane_hook_with_hourly_trend_and_stop() {
        // day 1 rises 0.1 every 5 minutes (hourly highs and lows rising: trend up); day 2: 107.0, 106.0, 105.0 (%K 0, 0, 0 with
        // n = 3, no smoothing), 105.5 (%K 50): a buy hook from under 20 at the 09:50 close; long at the next bar's open 105.8,
        // whose low 104.7 reaches the 1.00 stop 104.8 after the high (the nearer extreme).
        let d1: Vec<String> = (0..78u16).map(|k| { let p = 100. + 0.1 * k as f64; line(570 + 5 * k, p, p, p, p) }).collect();
        let d2: Vec<String> = [107.0, 106.0, 105.0, 105.5].iter().enumerate().map(|(k, &p)| line(570 + 5 * k as u16, p, p, p, p))
            .chain([line(590, 105.8, 105.9, 104.7, 104.9)]).collect();
        let r = day_rows(&["2020-01-02", "2020-01-03"]);
        let mut x = Ctx::new(&r, &r, &[]);
        x.minutes = Some(minutes("kane1", &[("2020-01-02", d1.clone()), ("2020-01-03", d2)]));
        let mut p = Params::new();
        p.insert("n".into(), 3.0);
        p.insert("smooth".into(), 1.0);
        assert_eq!(t4(&run("lebcafm_dt_kane", &x, 0, 1, &p).unwrap()), vec![(1., 105.8, 104.8, "initial stop".into())]);
        // the same day 2 after a falling day 1 (hourly trend down): the buy hook is not taken
        let d0: Vec<String> = (0..78u16).map(|k| { let p = 115. - 0.1 * k as f64; line(570 + 5 * k, p, p, p, p) }).collect();
        let d2b: Vec<String> = [107.0, 106.0, 105.0, 105.5].iter().enumerate().map(|(k, &p)| line(570 + 5 * k as u16, p, p, p, p)).collect();
        x.minutes = Some(minutes("kane2", &[("2020-01-02", d0), ("2020-01-03", d2b)]));
        assert!(run("lebcafm_dt_kane", &x, 0, 1, &p).unwrap().is_empty());
    }
    /// Lane K57 engine-vs-reference dump (run by hand; K57w/cmp_dump.py compares with k57_ref.py):
    /// VAULT_US=<data/us_data.csv> VAULT_M1_DIR=<data/intraday> K57_OUT=<file> cargo test --release k57_dump -- --ignored
    #[test]
    #[ignore]
    fn k57_dump() {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("K57_OUT").unwrap()).unwrap());
        for tk in ["SPX", "SPY"] {
            let rows = crate::load_csv(&std::env::var("VAULT_US").unwrap()).unwrap().remove(tk).unwrap();
            let mut x = Ctx::new(&rows, &rows, &[]);
            x.minutes = Some(Box::leak(Box::new(crate::intraday::Minutes::load(&std::env::var("VAULT_M1_DIR").unwrap(), tk).unwrap())));
            let from = rows.partition_point(|r| r.date.as_str() < x.minutes.unwrap().first());
            let sets: [(&str, &str, &[(&str, f64)]); 5] = [("lebcafm_dt_5_25", "default", &[]), ("lebcafm_dt_5_25", "points_band", &[("points", 1.), ("band", 1.)]),
                ("lebcafm_dt_kane", "default", &[]), ("lebcafm_dt_kane", "n14_s1_stop2", &[("n", 14.), ("smooth", 1.), ("stop", 2.)]), ("lebcafm_dt_5_25", "be0.3_n8", &[("be", 0.3), ("n", 8.)])];
            for (id, tag, kv) in sets {
                let mut p = Params::new();
                for (k, v) in kv { p.insert((*k).into(), *v); }
                for a in [from, rows.partition_point(|r| r.date.as_str() < "2014-01-02")] {
                    for z in run(id, &x, a, rows.len() - 1, &p).unwrap() {
                        writeln!(f, "{tk},{id},{tag},{},{},{},{},{},{},{},{}", rows[a].date, z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.reason, z.open).unwrap();
                    }
                }
            }
        }
    }
    #[test]
    fn parabolic_flips_and_trails() {
        let c: Vec<f64> = (0..30).map(|i| 100. + i as f64).chain((0..30).map(|i| 129. - 2. * i as f64)).collect();
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let (sar, d) = parabolic(&x, 0.02, 0.02, 0.2);
        assert_eq!(d[20], 1);
        assert!(sar[20] < x.l[20]);
        assert_eq!(d[59], -1);
        assert!(sar[59] > x.h[58].min(x.h[59]) - 1e-9);
    }
}

// Instrument admission: LeBeau 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lebeau/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"lebcafm"|"lebtc"|"lebtcb"|"lebws") && (crate::books::lebcafm::reviewed(def.id) || crate::books::lebtc::reviewed(def.id) || crate::books::lebtcb::reviewed(def.id) || crate::books::lebws::reviewed(def.id)),
    label: "Charles LeBeau and his System Traders Club tested these rules on futures (Computer Analysis of the Futures Markets: D-mark, gold, soybeans, T-bonds and crude; the club bulletins and forum: interest-rate and other futures) or on US stocks (High Beta workbook, ABC handout); index, ETF and stock runs (US and India) are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.",
    basis: "Charles LeBeau (& David W. Lucas): Computer Analysis of the Futures Markets Ch.2 ADX + Parabolic, volatility breakout, Babcock DMI and Lambert CCI rules (p.39, 61-63, 113-114, 139-146) and the Ch.3 entry, exit and stop test protocols (p.170-188); System Traders Club High Beta Stock system, Ronco adaptive channel and volatility breakouts, TBond Attack, Johnson's Pretty Good Oscillator (Bulletin #4, Traderclub Forum) and the ABC ATR handout. Reviewed in authors/lebeau/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Orders follow the book or printed code; no costs; the futures results are not reproduced.",
};
