//! The Best of the Professional Traders Journal: Options Trading and Volatility Trading (Connors, 1999). Book id "otvt".
//! (Same publication as the "ptjopt" scan; this module covers both.)
//!
//! Conventions:
//!   * HV = annualised sample stdev of daily log returns (connors::hv). `p.nm1=1` uses N-1 returns (the
//!     Advanced Trading Strategies appendix definition). Ratios do not depend on the annualisation.
//!   * VIX OHLC is read from the external series VIX_OHLC (Open/High/Low/Close columns), because Ctx only
//!     exposes derived VIX facts. `p.src=1` reads VXO_OHLC (CBOE OEX volatility index, the book's "VIX").
//!   * Option trades are priced with Black-Scholes (r = 0, no dividends, no skew) from a daily IV series and
//!     returned as an unlevered position on one unit of the underlying: Trade px = underlying close at entry,
//!     exit px = entry px +/- option P&L, so `ret` = option P&L / underlying price. dir = sign of the option
//!     position's delta (short puts / short strangles +1, long puts / short calls -1).
#![allow(unused_imports, dead_code)]
use crate::connors::*;

pub const BOOK: &str = "otvt";

/// Daily underlying rules admitted after the 2026-10-21 source review (connors/hfm-admission/SOURCE-CONTRACT.md) and an
/// independent chronological Python ledger check. Declared application: the book traded S&P futures / OEX.
/// otvt_cvr1 (combined) stays gate-withheld; option, event-study and stock-split items stay held.
pub const REVIEWED_DAILY: &[&str] = &["otvt_cvr1_long", "otvt_cvr1_short", "otvt_hv_thrust"];
pub fn reviewed_daily(id: &str) -> bool { REVIEWED_DAILY.contains(&id) }
/// CVR I reads the VIX open (India VIX open = previous close): SPX and SPY with the CBOE VIX only.
pub fn admitted_symbol(id: &str, ticker: &str) -> bool {
    match id {
        "otvt_cvr1_long" | "otvt_cvr1_short" => matches!(ticker, "SPX" | "SPY"),
        "otvt_hv_thrust" => matches!(ticker, "SPX" | "NDX" | "DJI" | "SPY" | "NIFTY 50" | "NIFTYBANK"),
        _ => false,
    }
}
/// Fills read the traded instrument's open only with the labelled ATR trailing stop (exit = 1).
pub fn reads_open(id: &str, p: &Params) -> bool {
    id == "otvt_hv_thrust" && p.get("exit").is_some_and(|v| v.round() == 1.0)
}
/// The served CBOE VIX history has close-only bars (open = high = low = close) for 1990-1991.
pub const REAL_VIX_OHLC_FROM: &str = "1992-01-01";

const VIX_IV: &[&str] = &["VIX_OHLC.Open", "VIX_OHLC.High", "VIX_OHLC.Low", "VIX_OHLC.Close"];
const IV: &[&str] = &["IV30"];

// ---------------------------------------------------------------- rules text

const DIZ: &[&str] = &[
    "F|\"when short-term volatility (measured by the 10-day historical volatility) reaches 1.65 times the 100-day reading or higher, mean-reversion and volatility contraction are near\"",
    "E|\"We wait for two down-tick moves in the 10-day reading for confirmation that mean-reversion and volatility contraction have started\" (counted from the peak, as in the gold example; an up-tick resets the count, silver example)",
    "E|\"on the day after these readings we place the trade\": sell a strangle, an out-of-the-money call and an out-of-the-money put",
    "X|Buy the strangle back when \"reversion to a normal volatility level is complete\" (10-day HV back to the 100-day reading)",
    "X|\"if reversion is not complete in ten to fifteen days after the trade is initiated, one should close any position based on this setup\"",
];
const CVR_BUY: &[&str] = &[
    "E|\"Today the VIX must make a 15-day high.\"",
    "E|\"Today, the close of the VIX must be below the open.\"",
    "E|\"If Rules 1 and 2 are met, buy the S&P's (or the market) on the close of today.\"",
    "X|\"Hold the position at least one to three days.\" (exit on the close of day 3)",
];
const CVR_SELL: &[&str] = &[
    "E|\"Today the VIX must make a 15-day low.\"",
    "E|\"Today the close of the VIX must be above the open.\"",
    "E|\"If Rules 1 and 2 are met, sell the S&P's (or the market) on the close of today.\"",
    "X|\"Hold the position at least one to three days.\" (exit on the close of day 3)",
];
const CVR_BOTH: &[&str] = &[
    "E|Buy: \"Today the VIX must make a 15-day high.\" and \"Today, the close of the VIX must be below the open.\" Buy on the close",
    "E|Sell: \"Today the VIX must make a 15-day low.\" and \"Today the close of the VIX must be above the open.\" Sell on the close",
    "X|\"Hold the position at least one to three days.\" (exit on the close of day 3)",
    "X|An opposite signal while in a trade reverses the position on that close (as in the Advanced Trading Strategies trade list)",
];
const CVR_PUTS_SELL: &[&str] = &[
    "F|CVR I buy signal (VIX 15-day high, VIX close below its open)",
    "E|\"By selling puts!\" \"as near the money as possible (within two to three strike prices)\": sell the closest month's puts on the close",
    "X|\"The exit strategy on each should be consistent with the short-term hold recommended earlier\" (3 days; both examples exit on the close of day 3)",
    "X|\"I recommend closing out the naked puts if prices reach your strike price\"",
    "X|\"If, though, there is a sharp move in your favor earlier than this, I highly recommend taking profits on at least half the position\" (not coded: undefined)",
];
const CVR_PUTS_BUY: &[&str] = &[
    "F|CVR I sell signal (VIX 15-day low, VIX close above its open)",
    "E|\"Because a successful sell signal will be accompanied by a rise in implied volatility, we do not want to sell calls. It is more advisable to buy puts.\"",
    "X|Short-term hold (3 days, exit on the close)",
    "X|\"recommend closing out the long puts if prices move 10 S&P points against you\"",
];
const SPLIT: &[&str] = &[
    "E|After a stock-split announcement: \"sell these beeper subscribers calls as they run up the price of the options\"",
    "X|\"The options, as measured by implied volatility, tend to remain overvalued for a few days\"; no strike, threshold, exit or stop is given",
];
const SECTOR: &[&str] = &[
    "E|\"Identify a sector whose combined implied volatility on the near the money put and call strike price is three times the 50-day historical volatility reading ... sell these options.\"",
    "E|Variant: \"if you have a directional bias, you can sell just one side\"",
    "X|\"Risk 30 percent to 45 percent of the combined price, which means if you sell the combined options for 4, you should stop yourself out at approximately 5 1/4-5 3/4.\"",
    "X|Otherwise hold to expiration (\"priced to expect a 13 percent volatility until expiration\")",
];
const HV50: &[&str] = &[
    "F|\"whenever a 10-day period volatility was 50 percent or less than the 100-day period, a large market move was likely to occur\"; \"most of the examples shown use a six-day lower trading period\"",
    "F|\"Ideally, we would like to see the historical volatility ratio reading stay under 50 percent for more than one day.\"",
    "F|Filter: after \"a very large, extraordinary move\" readings under 50% \"are usually false\": \"wait a full 100 days\"",
    "X|No entry direction, stop or exit: the setup predicts a large move in either direction",
];
const HV_COMB: &[&str] = &[
    "F|\"combining a 10-period reading and a 6-period reading is a superior method to only using a 10-period reading\": 6/100 and 10/100 both under 50%",
    "F|\"When the number is this close, I will not stand on ceremony\" (a 50.2% reading accepted)",
    "X|No entry direction, stop or exit",
];
const HV_40: &[&str] = &[
    "F|\"This holds true both on a daily basis and on an intraday basis\": 10/100 and 6/100 HV under 50% on 40-minute S&P bars",
    "X|No entry direction, stop or exit (\"The S&P's explode 8 points higher within a day\")",
];
const THRUST: &[&str] = &[
    "F|\"Identify a market whose 6-day or 10-day volatility is less than 1/2 its 100-day reading.\"",
    "E|\"On the day of the readings under 50 percent look for a daily price range that is larger than the daily range of the previous nine days.\" (HV reading taken on the range day itself; same_day=0 = [Not from the book] previous reading: HV read on the prior bar)",
    "E|\"If rules 1 and 2 are met, expect a follow-through in the direction of the large-range day\": enter on the close in the direction of the day (close vs prior close)",
    "X|\"for the next one to ten days\": a follow-through expectation, not an exit rule; the book gives no exit or stop",
    STD_EXIT_RULE,
];
const STD_EXIT_RULE: &str = "X|[Not from the book] standard exits (user rule, the book gives none): param exit 0 = close after `hold` bars (default 10; 3 / 5 / 10), 1 = trailing stop 2 x ATR(5) behind the best price since entry, 2 = first up close (shorts: first down close)";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, needs_series, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series, book: BOOK, rules, notes, claim,
    };
    // CVR ids: test window starts at the first bar of the market's volatility index (needs_vix), so CAGR is not diluted by years with no VIX.
    let v = |s: StrategyDef| StrategyDef { needs_vix: true, ..s };
    macro_rules! cvr_note { () => { "Book's VIX is the CBOE OEX index (now VXO). Reads the external series VIX_OHLC (each market's own volatility index: CBOE VIX for US, India VIX for India); p.src=1 reads VXO_OHLC. VIX 15-day high = today's VIX high is the highest high of the last 15 bars, today included (lows mirrored). Same rules as Advanced Trading Strategies ch. 2 (ats CVR I); on SPY 1993-97 with VXO this reproduces 61 of that book's 72 listed trades exactly (same entry and exit). Claim \">60%\" reproduces only with the book's index: p.src=1 (VXO) gives 68.3% (41/60) on SPY 1993-03..1997-02; the default p.src=0 (today's SPX VIX) gives 48.6% (34/70) in the same window. p.src=1 is a data gap where VXO_OHLC has no data in the test window (India; SPY after 2021-09-23, when CBOE stopped VXO). Book traded S&P futures; run here on indices/ETFs." } }
    const CVR_NOTE: &str = cvr_note!();
    // user-approved visible note (2026-10-21): the sides stay admitted, the combined rule failed the gate's book comparison
    const CVR_SIDE_NOTE: &str = concat!("Accuracy-gate note: the combined CVR I (buys and sells together, otvt_cvr1) FAILED the gate's comparison with the book: 48.6% winners (34 of 70, SPY 1993-03..1997-01) vs the book's >60%, because the vault reads today's CBOE VIX, not the book's OEX VXO. The book's >60% is for buys and sells together; this side is shown on its own. The short side alone wins only about 36% (16 of 45) in that window. ", cvr_note!());
    vec![
        d("otvt_diz_short_strangle", "Diz HV10/HV100 peak · short strangle", "Trading Volatility with Options", "Long/Short", NeedsData("real traded prices of the exact option contracts (strike and expiry the rule picks); option P&L is never computed from a pricing model"), IV, DIZ,
          "Prof. Fernando Diz (PTJ July 1996). Signal from daily closes; the strangle is priced with Black-Scholes from the per-ticker IV30 series (no skew), strikes otm% either side (book examples are 2-13% OTM, no rule), dte calendar days to expiry (examples about 2 months). Down-ticks counted on the 10-day HV (p.tick_src=1: the 10/100 ratio, as in the soybean text). Down-ticks count from the peak (the bar that first reaches 1.65); p.rearm_reset=1 is the old fitted reading that restarts the count on every bar still >= 1.65 [Not from the book]. Reversion = HV10 <= HV100. No IV30 series is shipped, so this is a data gap until real per-ticker IV or option data exists; a single ATM IV for both legs would ignore skew, which matters for a 5% OTM strangle. Book traded COMEX gold/silver and soybeans; run here on indices/ETFs. Return = option P&L / underlying price.",
          claim("3 examples", "3/3", "130 pts gold, 360 pts silver, 15c beans", "8-14 days", "1994-95 futures options")),
        v(d("otvt_cvr1", "CVR I · buys and sells (reversing)", "Connors VIX Reversal I", "Long/Short", Live, VIX_IV, CVR_BOTH, CVR_NOTE,
          claim("-", ">60%", "-", "1-3 days", "four years to March 1997, S&P, book's VIX = OEX VXO (reproduces with p.src=1: 68.3%; default SPX VIX: 48.6%)"))),
        v(d("otvt_cvr1_long", "CVR I · buys", "Connors VIX Reversal I", "Long", Live, VIX_IV, CVR_BUY, CVR_SIDE_NOTE, None)),
        v(d("otvt_cvr1_short", "CVR I · sells", "Connors VIX Reversal I", "Short", Live, VIX_IV, CVR_SELL, CVR_SIDE_NOTE, None)),
        v(d("otvt_cvr1_sell_puts", "CVR I buy signal · sell near-the-money puts", "Trading Options with the CVR", "Long", NeedsData("real traded prices of the exact option contracts (strike and expiry the rule picks); option P&L is never computed from a pricing model"), VIX_IV, CVR_PUTS_SELL,
          "Put priced with Black-Scholes using the volatility index close itself as the implied vol (VIX = 30-day SPX IV, India VIX = NIFTY IV, VXO = OEX IV), so it needs no extra series. Strike otm% below the close (book: 770 put with OEX 781.98 and 760 with 768.54, about 1.3%); dte = days to expiry (book: March options, 21-29 days). Stop: the index trades to the strike (intraday low <= strike; priced at the strike or the open if it gaps through). Partial profit-taking is undefined and not coded. Return = put P&L / underlying price. Runs only on SPY and NIFTY 50, the two tickers whose own option IV the volatility index measures; other tickers return a data gap (their numbers would be priced off another index's IV) [Not from the book]. Model biases [Not from the book]: p.r = risk-free rate (default 0; about 5 for 1990s US, 6.5 for India; r = 0 overprices puts about 8-10%); p.vxo_adj=1 scales VXO by sqrt(22/30) to undo its trading-day upward bias (Carr & Wu 2006); no skew (optimistic for sold puts in sell-offs); no bid/ask. Equity inside a trade moves with the underlying (delta 1), so max DD and Sharpe are approximate; closed-trade returns are exact.",
          claim("2 examples", "2/2", "+5 1/2 and +6 1/4 points", "3 days", "Feb 1997, OEX"))),
        v(d("otvt_cvr1_buy_puts", "CVR I sell signal · buy puts", "Trading Options with the CVR", "Short", NeedsData("real traded prices of the exact option contracts (strike and expiry the rule picks); option P&L is never computed from a pricing model"), VIX_IV, CVR_PUTS_BUY,
          "At-the-money put (otm% = 0), priced like the put-selling version. Stop: the index rises 10 S&P points (1.28% of the OEX 781.98 example level) above the entry close, priced at that level or the open if it gaps through. Return = put P&L / underlying price. Runs only on SPY and NIFTY 50, the two tickers whose own option IV the volatility index measures; other tickers return a data gap (their numbers would be priced off another index's IV) [Not from the book]. Model biases [Not from the book]: p.r = risk-free rate (default 0; about 5 for 1990s US, 6.5 for India; r = 0 overprices puts about 8-10%); p.vxo_adj=1 scales VXO by sqrt(22/30) to undo its trading-day upward bias (Carr & Wu 2006); no skew (optimistic for sold puts in sell-offs); no bid/ask. Equity inside a trade moves with the underlying (delta 1), so max DD and Sharpe are approximate; closed-trade returns are exact.", None)),
        d("otvt_split_call_selling", "Sell calls after stock-split announcements", "Options on Stock Splits", "Short", NotTestable("needs option chains and split-announcement data; no threshold, strike or exit given"), &[], SPLIT,
          "Barnes & Noble example (Aug 21, 1997): 100-day HV 27%, call IV 39.3%. No trade rule. Not applicable to index data.", None),
        d("otvt_sector_options_overpriced", "Overpriced sector options · sell IV >= 3x HV50", "Exploiting Overpriced Stock Sector Options", "Long/Short", NeedsData("real traded prices of the exact option contracts (strike and expiry the rule picks); option P&L is never computed from a pricing model"), IV, SECTOR,
          "Per-ticker IV30 stands in for both legs' IV (no IV30 series is shipped, so this is a data gap until real per-ticker IV or option data exists; one ATM IV for both legs reduces the test to index IV vs HV50). \"Combined\" read as the SUM of put and call IV (2 x IV30 >= 3 x HV50); p.comb=0 reads it as each leg's IV >= 3 x HV50 (the ptjopt transcription's reading). HV50 annualised with 260 days (the book's own factor). Strikes otm% either side of the close (book: 195/200 around 197.02), dte 19 (REIT example). Stop checked at the close: combined price >= credit x (1 + stop%). Otherwise held to expiry (intrinsic value). p.side=1 calls only, 2 puts only. Book recommends indices; run on indices/ETFs. Return = option P&L / underlying price.", None),
        d("otvt_hv_ratio_under_50", "HV 6/100 or 10/100 under 50% (multi-day)", "Additional Volatility Research", "Long/Short", NotTestable("event study: no entry or exit"), &[], HV50,
          "Predicts a large move in either direction; the tradeable form is HV Thrusts (otvt_hv_thrust). Same material as Advanced Trading Strategies' HV chapter.", None),
        d("otvt_hv_ratio_combined", "HV 6/100 and 10/100 both under 50%", "Additional Volatility Research", "Long/Short", NotTestable("event study: no entry or exit"), &[], HV_COMB,
          "Use otvt_hv_thrust with p.hv_mode=1 for the tradeable combined form.", None),
        d("otvt_hv_ratio_combined_40min", "HV 6/100 and 10/100 under 50% · 40-minute bars", "Additional Volatility Research", "Long/Short", NotTestable("event study: no entry or exit (also needs 40-minute bars)"), &[], HV_40,
          "Intraday variant of the combined reading.", None),
        d("otvt_hv_thrust", "HV Thrust · HV ratio < 50% + 9-day range expansion", "HV Thrusts", "Long/Short", Live, &[], THRUST,
          "HV ratio < 0.5 on the large-range day itself, \"on the day of the readings\" (p.same_day=0: [Not from the book] previous reading, HV on the bar before the range day). Range = high - low > every range of the previous 9 days. Direction = close vs prior close (p.dir_mode=1: close vs open); unchanged days are skipped. Entry on the range day's close, exit on the close N days later; no stop in the book. p.hv_mode: 0 = 6/100 or 10/100, 1 = both, 2 = 6/100 only, 3 = 10/100 only; p.min_days = consecutive days under 50%. p.filt_k > 0 enables a false-signal filter (not quantified in the book): skip signals for filt_days after a day whose |return| > filt_k x the daily sigma implied by HV100. Book examples are futures (British pound, T-bonds); run on indices/ETFs.",
          claim("2 examples", "-", "-", "1-10 days", "1996 futures")),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let cvr = || vec![
        p("n", "VIX high/low lookback (days, today included)", 15.0, 5.0, 30.0, 1.0),
        p("hold", "Exit on the close after N days", 3.0, 1.0, 10.0, 1.0),
        p("src", "Volatility index: 0 = VIX_OHLC, 1 = VXO_OHLC", 0.0, 0.0, 1.0, 1.0),
    ];
    Some(match id {
        "otvt_diz_short_strangle" => vec![
            p("short_n", "Short HV length", 10.0, 5.0, 20.0, 1.0),
            p("long_n", "Long HV length", 100.0, 50.0, 200.0, 10.0),
            p("ratio_hi", "Arm: HV ratio at or above", 1.65, 1.2, 2.5, 0.05),
            p("ticks", "Consecutive down-ticks to confirm", 2.0, 1.0, 4.0, 1.0),
            p("tick_src", "Down-ticks on: 0 = 10-day HV, 1 = ratio", 0.0, 0.0, 1.0, 1.0),
            p("entry_next", "Enter: 0 = confirmation close, 1 = next day's close", 1.0, 0.0, 1.0, 1.0),
            p("exit_ratio", "Exit: HV ratio at or below", 1.0, 0.7, 1.3, 0.05),
            p("tstop", "Time stop (days)", 15.0, 5.0, 20.0, 1.0),
            p("otm", "Strikes: % out of the money", 5.0, 1.0, 15.0, 1.0),
            p("dte", "Calendar days to expiry", 60.0, 20.0, 120.0, 5.0),
            p("nm1", "HV over N-1 returns (0/1)", 0.0, 0.0, 1.0, 1.0),
            p("rearm_reset", "Restart the down-tick count on every bar still at/above the arm ratio (0 = book, 1 = old fitted reading)", 0.0, 0.0, 1.0, 1.0),
        ],
        "otvt_cvr1" => cvr().into_iter().chain([p("rev", "Opposite signal reverses (0/1)", 1.0, 0.0, 1.0, 1.0)]).collect(),
        "otvt_cvr1_long" | "otvt_cvr1_short" => cvr(),
        "otvt_cvr1_sell_puts" => cvr().into_iter().chain([
            p("otm", "Put strike: % below the close", 1.3, 0.0, 5.0, 0.1),
            p("dte", "Calendar days to expiry", 25.0, 7.0, 60.0, 1.0),
            p("r", "Risk-free rate % [Not from the book]", 0.0, 0.0, 10.0, 0.5),
            p("vxo_adj", "With src=1: scale VXO by sqrt(22/30) (0/1) [Not from the book]", 1.0, 0.0, 1.0, 1.0),
          ]).collect(),
        "otvt_cvr1_buy_puts" => cvr().into_iter().chain([
            p("otm", "Put strike: % below the close", 0.0, 0.0, 5.0, 0.1),
            p("dte", "Calendar days to expiry", 25.0, 7.0, 60.0, 1.0),
            p("stop", "Stop: % move against (10 S&P points)", 1.28, 0.5, 3.0, 0.1),
            p("r", "Risk-free rate % [Not from the book]", 0.0, 0.0, 10.0, 0.5),
            p("vxo_adj", "With src=1: scale VXO by sqrt(22/30) (0/1) [Not from the book]", 1.0, 0.0, 1.0, 1.0),
          ]).collect(),
        "otvt_sector_options_overpriced" => vec![
            p("mult", "IV at least N x HV", 3.0, 1.5, 4.0, 0.25),
            p("hv_n", "HV length", 50.0, 25.0, 200.0, 5.0),
            p("comb", "IV test: 1 = put + call IV, 0 = each leg", 1.0, 0.0, 1.0, 1.0),
            p("otm", "Strikes: % from the close", 1.0, 0.0, 5.0, 0.5),
            p("dte", "Calendar days to expiry", 19.0, 7.0, 60.0, 1.0),
            p("stop", "Stop: % of the credit risked", 37.5, 30.0, 45.0, 2.5),
            p("side", "Sell: 0 = put + call, 1 = calls, 2 = puts", 0.0, 0.0, 2.0, 1.0),
        ],
        "otvt_hv_thrust" => vec![
            p("short_a", "Short HV length A", 6.0, 3.0, 15.0, 1.0),
            p("short_b", "Short HV length B", 10.0, 3.0, 20.0, 1.0),
            p("long_n", "Long HV length", 100.0, 50.0, 200.0, 10.0),
            p("ratio", "HV ratio below", 0.5, 0.3, 0.7, 0.05),
            p("hv_mode", "0 = A or B, 1 = both, 2 = A only, 3 = B only", 0.0, 0.0, 3.0, 1.0),
            p("min_days", "Consecutive days under the ratio", 1.0, 1.0, 10.0, 1.0),
            p("same_day", "HV reading: 1 = range day (book), 0 = [Not from the book] previous reading: prior bar", 1.0, 0.0, 1.0, 1.0),
            p("range_n", "Range larger than the previous N days", 9.0, 3.0, 20.0, 1.0),
            p("dir_mode", "Direction: 0 = close vs prior close, 1 = close vs open", 0.0, 0.0, 1.0, 1.0),
            p("hold", "Exit on the close after N days", 10.0, 1.0, 10.0, 1.0),
            p("nm1", "HV over N-1 returns (0/1)", 0.0, 0.0, 1.0, 1.0),
            p("filt_k", "False-signal filter: |return| > k sigma (0 = off)", 0.0, 0.0, 6.0, 0.5),
            p("filt_days", "False-signal filter: days to wait", 100.0, 20.0, 150.0, 10.0),
            p("exit", "[Not from the book] exit: 0 = close after hold bars, 1 = 2 x ATR(5) trailing stop, 2 = first up close (shorts: first down close)", 0.0, 0.0, 2.0, 1.0),
        ],
        _ => return None,
    })
}

// ---------------------------------------------------------------- helpers

/// HV over n returns, or n-1 returns (`nm1`, the "N minus one most recent logarithmic daily returns" definition).
fn hvx(c: &[f64], n: usize, nm1: bool) -> Vec<f64> {
    hv(c, if nm1 { n.saturating_sub(1).max(2) } else { n })
}

/// (open, high, low, close) of the volatility index aligned to this ticker (NaN where missing).
fn vix<'a>(x: &'a Ctx, src: usize) -> Option<[&'a [f64]; 4]> {
    let b = if src == 1 { "VXO_OHLC" } else { "VIX_OHLC" };
    let f = |k: &str| x.series(&format!("{b}.{k}"));
    Some([f("Open")?, f("High")?, f("Low")?, f("Close")?])
}

/// CVR I: +1 buy (VIX high = highest high of the last n bars incl. today, close < open), -1 sell (mirror).
fn cvr_signals(v: [&[f64]; 4], n: usize) -> Vec<i8> {
    let [o, h, l, c] = v;
    (0..o.len())
        .map(|i| {
            if i + 1 < n || !(o[i].is_finite() && h[i].is_finite() && l[i].is_finite() && c[i].is_finite()) {
                return 0;
            }
            let w = i + 1 - n..=i;
            let hh = w.clone().map(|k| h[k]).filter(|v| v.is_finite()).fold(f64::MIN, f64::max);
            let ll = w.map(|k| l[k]).filter(|v| v.is_finite()).fold(f64::MAX, f64::min);
            if h[i] >= hh && c[i] < o[i] {
                1
            } else if l[i] <= ll && c[i] > o[i] {
                -1
            } else {
                0
            }
        })
        .collect()
}

/// Calendar day number of a YYYY-MM-DD date (days_from_civil).
fn day(date: &str) -> i64 {
    let num = |a: usize, b: usize, d: i64| date.get(a..b).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (y, m, d) = (num(0, 4, 1970), num(5, 7, 1), num(8, 10, 1));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// Standard normal CDF (Abramowitz-Stegun 26.2.17, |error| < 7.5e-8).
fn ncdf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.231_641_9 * x.abs());
    let poly = t * (0.319_381_530 + t * (-0.356_563_782 + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
    let tail = (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt() * poly;
    if x >= 0.0 { 1.0 - tail } else { tail }
}

/// Black-Scholes price, no dividends; `vol` and `r` in percent, `t` in years. Intrinsic value at or after expiry.
fn bs(put: bool, s: f64, k: f64, t: f64, vol: f64, r: f64) -> f64 {
    let intrinsic = if put { (k - s).max(0.0) } else { (s - k).max(0.0) };
    let sig = vol / 100.0;
    if t <= 0.0 || !(sig > 0.0) {
        return intrinsic;
    }
    let (sd, kd) = (sig * t.sqrt(), k * (-r / 100.0 * t).exp());
    let d1 = ((s / kd).ln() + 0.5 * sd * sd) / sd;
    let call = s * ncdf(d1) - kd * ncdf(d1 - sd);
    if put { call - s + kd } else { call }
}

/// An option position: (is_put, strike, qty) legs priced off one IV, expiring on calendar day `exp`.
struct Book {
    legs: Vec<(bool, f64, f64)>,
    exp: i64,
    r: f64,
}
impl Book {
    /// Signed value (short legs negative).
    fn value(&self, s: f64, today: i64, vol: f64) -> f64 {
        let t = (self.exp - today) as f64 / 365.0;
        self.legs.iter().map(|&(put, k, q)| q * bs(put, s, k, t, vol, self.r)).sum()
    }
}

/// Last finite value at or before i.
fn last(v: &[f64], i: usize) -> f64 {
    (0..=i).rev().map(|k| v[k]).find(|x| x.is_finite()).unwrap_or(f64::NAN)
}

fn trade(dir: f64, i: usize, px: f64, exit_i: usize, exit_px: f64, reason: String, open: bool) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i, exit_px, reason, entry_kind: "close", signal_i: exit_i, open, exits: Vec::new() }
}

/// Option P&L as an unlevered position on one unit of the underlying: ret = dir * (exit/entry - 1) = pnl / S0.
fn opt_trade(dir: f64, i: usize, s0: f64, exit_i: usize, pnl: f64, reason: String, open: bool) -> Trade {
    trade(dir, i, s0, exit_i, s0 + dir * pnl, reason, open)
}

fn f2(v: f64) -> String {
    format!("{v:.2}")
}

// ---------------------------------------------------------------- run

/// Per-ticker / per-parameter data gaps that `needs_series` cannot express. `run` returns no trades when this is Some;
/// the message is for the API to show (it needs a call to this hook, shared code).
pub fn gap(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<String> {
    let puts = matches!(id, "otvt_cvr1_sell_puts" | "otvt_cvr1_buy_puts");
    if !puts && !matches!(id, "otvt_cvr1" | "otvt_cvr1_long" | "otvt_cvr1_short") {
        return None;
    }
    let ticker = x.r.first().map_or("", |r| r.ticker.as_str());
    if puts && !matches!(ticker, "SPY" | "SPX" | "NIFTY 50") {
        return Some("data gap: needs this ticker's own option IV (the volatility index prices SPY / NIFTY 50 options only)".into());
    }
    let vxo = p.get("src").map_or(false, |s| s.round() == 1.0);
    let has = |v: &[f64]| v.get(from..=to.min(v.len().saturating_sub(1))).map_or(false, |w| w.iter().any(|v| v.is_finite()));
    (vxo && !x.series("VXO_OHLC.Close").map_or(false, has)).then(|| "data gap: needs VXO_OHLC in the test window (CBOE VXO ends 2021-09-23; none for India)".into())
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if gap(id, x, from, to, p).is_some() {
        return Some(vec![]);
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let b = |k: &str| g(k).round() >= 1.0;
    let (c, o, h, l) = (&x.c, &x.o, &x.h, &x.l);
    let days: Vec<i64> = x.r.iter().map(|r| day(&r.date)).collect();
    Some(match id {
        "otvt_cvr1" | "otvt_cvr1_long" | "otvt_cvr1_short" => {
            let Some(v) = vix(x, g("src").round() as usize) else { return Some(vec![]) };
            let sig = cvr_signals(v, u("n"));
            let side: i8 = match id { "otvt_cvr1_long" => 1, "otvt_cvr1_short" => -1, _ => 0 };
            let rev = side == 0 && b("rev");
            cvr_sim(x, &sig, side, u("hold"), rev, from, to)
        }
        "otvt_cvr1_sell_puts" | "otvt_cvr1_buy_puts" => {
            let Some(v) = vix(x, g("src").round() as usize) else { return Some(vec![]) };
            let sig = cvr_signals(v, u("n"));
            // VXO's trading-day convention inflates it by about sqrt(30/22) (Carr & Wu 2006): undo it for pricing
            let adj = if g("src").round() == 1.0 && b("vxo_adj") { (22f64 / 30.0).sqrt() } else { 1.0 };
            let ivs: Vec<f64> = v[3].iter().map(|v| v * adj).collect();
            let ivs = &ivs[..];
            let long_puts = id == "otvt_cvr1_buy_puts";
            let (want, hold, otm, dte) = (if long_puts { -1 } else { 1 }, u("hold"), g("otm") / 100.0, g("dte").round() as i64);
            let stop = if long_puts { g("stop") / 100.0 } else { 0.0 };
            let q = if long_puts { 1.0 } else { -1.0 };
            let mut out = Vec::new();
            let mut i = from;
            while i < to {
                if sig[i] != want || !ivs[i].is_finite() {
                    i += 1;
                    continue;
                }
                let (s0, k) = (c[i], c[i] * (1.0 - otm));
                let bk = Book { legs: vec![(true, k, q)], exp: days[i] + dte, r: g("r") };
                let v0 = bk.value(s0, days[i], ivs[i]);
                let mut done = None;
                for j in i + 1..=to {
                    let iv = last(ivs, j);
                    // stop: naked put -> the index trades to the strike; long put -> the index rises stop% above entry
                    let hit = if long_puts {
                        let lv = s0 * (1.0 + stop);
                        (h[j] >= lv).then(|| o[j].max(lv))
                    } else {
                        (l[j] <= k).then(|| o[j].min(k))
                    };
                    done = if let Some(s) = hit {
                        Some((j, bk.value(s, days[j], iv), "stop".to_string(), false))
                    } else if j >= i + hold || days[j] >= bk.exp {
                        Some((j, bk.value(c[j], days[j], iv), format!("{hold} days held"), false))
                    } else if j == to {
                        Some((j, bk.value(c[j], days[j], iv), "open".into(), true))
                    } else {
                        continue;
                    };
                    break;
                }
                let Some((j, v1, why, open)) = done else { break };
                let lab = if long_puts { "bought" } else { "sold" };
                out.push(opt_trade(-q, i, s0, j, v1 - v0, format!("{why} · {lab} {} put {} -> {}", f2(k), f2(v0.abs()), f2(v1.abs())), open));
                i = j + 1;
            }
            out
        }
        "otvt_diz_short_strangle" => {
            let Some(iv) = x.series("IV30") else { return Some(vec![]) };
            let nm1 = b("nm1");
            let (hs, hl) = (hvx(c, u("short_n"), nm1), hvx(c, u("long_n"), nm1));
            let ratio: Vec<f64> = hs.iter().zip(&hl).map(|(a, b)| a / b).collect();
            let tick: &[f64] = if b("tick_src") { &ratio } else { &hs };
            let (hi, need, next, ex, ts, otm, dte, rearm) = (g("ratio_hi"), u("ticks"), b("entry_next"), g("exit_ratio"), u("tstop"), g("otm") / 100.0, g("dte").round() as i64, b("rearm_reset"));
            // confirmation bars: armed when the ratio reaches hi, then `need` consecutive down-ticks counted from the peak
            // (an up-tick resets the count). rearm_reset=1: the old fitted reading, where every bar still >= hi restarts the count.
            let mut conf = vec![false; c.len()];
            let (mut armed, mut cnt) = (false, 0);
            for i in 1..c.len() {
                if armed {
                    cnt = if tick[i] < tick[i - 1] { cnt + 1 } else { 0 };
                    if cnt >= need {
                        conf[i] = true;
                        armed = false;
                        cnt = 0;
                        continue;
                    }
                }
                if ratio[i] >= hi && (rearm || !armed) {
                    armed = true;
                    cnt = 0;
                }
            }
            let mut out = Vec::new();
            let mut e = from;
            while e < to {
                if !(if next { e > 0 && conf[e - 1] } else { conf[e] }) || !iv[e].is_finite() {
                    e += 1;
                    continue;
                }
                let s0 = c[e];
                let (pk, ck) = (s0 * (1.0 - otm), s0 * (1.0 + otm));
                let bk = Book { legs: vec![(true, pk, -1.0), (false, ck, -1.0)], exp: days[e] + dte, r: 0.0 };
                let credit = -bk.value(s0, days[e], iv[e]);
                let mut done = None;
                for j in e + 1..=to {
                    let why = if ratio[j] <= ex {
                        "reversion complete"
                    } else if j >= e + ts {
                        "time stop"
                    } else if days[j] >= bk.exp {
                        "expiry"
                    } else if j == to {
                        "open"
                    } else {
                        continue;
                    };
                    done = Some((j, -bk.value(c[j], days[j], last(iv, j)), why));
                    break;
                }
                let Some((j, debit, why)) = done else { break };
                out.push(opt_trade(1.0, e, s0, j, credit - debit, format!("{why} · sold {}P/{}C for {} -> {}", f2(pk), f2(ck), f2(credit), f2(debit)), why == "open"));
                e = j + 1;
            }
            out
        }
        "otvt_sector_options_overpriced" => {
            let Some(iv) = x.series("IV30") else { return Some(vec![]) };
            let hvn: Vec<f64> = hv(c, u("hv_n")).iter().map(|v| v * (260f64 / 252.0).sqrt()).collect();
            let (mult, comb, otm, dte, stop, side) = (g("mult"), b("comb"), g("otm") / 100.0, g("dte").round() as i64, g("stop") / 100.0, g("side").round() as i64);
            let mut out = Vec::new();
            let mut i = from;
            while i < to {
                let ivsum = if comb { 2.0 * iv[i] } else { iv[i] };
                if !(ivsum >= mult * hvn[i]) {
                    i += 1;
                    continue;
                }
                let s0 = c[i];
                let mut legs = Vec::new();
                if side != 1 {
                    legs.push((true, s0 * (1.0 - otm), -1.0));
                }
                if side != 2 {
                    legs.push((false, s0 * (1.0 + otm), -1.0));
                }
                let bk = Book { legs, exp: days[i] + dte, r: 0.0 };
                let credit = -bk.value(s0, days[i], iv[i]);
                let mut done = None;
                for j in i + 1..=to {
                    let debit = -bk.value(c[j], days[j], last(iv, j));
                    let why = if days[j] >= bk.exp {
                        "expiry"
                    } else if debit >= credit * (1.0 + stop) {
                        "stop"
                    } else if j == to {
                        "open"
                    } else {
                        continue;
                    };
                    done = Some((j, debit, why));
                    break;
                }
                let Some((j, debit, why)) = done else { break };
                let dir = if side == 1 { -1.0 } else { 1.0 };
                out.push(opt_trade(dir, i, s0, j, credit - debit, format!("{why} · credit {} -> {} (IV {} vs HV{} {})", f2(credit), f2(debit), f2(iv[i]), u("hv_n"), f2(hvn[i])), why == "open"));
                i = j + 1;
            }
            out
        }
        "otvt_hv_thrust" => {
            let nm1 = b("nm1");
            let hl = hvx(c, u("long_n"), nm1);
            let ra: Vec<f64> = hvx(c, u("short_a"), nm1).iter().zip(&hl).map(|(a, b)| a / b).collect();
            let rb: Vec<f64> = hvx(c, u("short_b"), nm1).iter().zip(&hl).map(|(a, b)| a / b).collect();
            let (thr, mode, md, same, rn, dm, hold) = (g("ratio"), g("hv_mode").round() as i64, u("min_days"), b("same_day"), u("range_n"), b("dir_mode"), u("hold"));
            let ok = |j: usize| {
                let (a, b) = (ra[j] < thr, rb[j] < thr);
                match mode { 1 => a && b, 2 => a, 3 => b, _ => a || b }
            };
            // optional false-signal filter: block filt_days bars after |log return| > k x the HV100 daily sigma (prior bar)
            let (fk, fd) = (g("filt_k"), u("filt_days"));
            let mut blocked = vec![false; c.len()];
            if fk > 0.0 {
                let mut until = None;
                for j in 1..c.len() {
                    let sd = x.hv100[j - 1] / 100.0 / 252f64.sqrt();
                    if (c[j] / c[j - 1]).ln().abs() > fk * sd {
                        until = Some(j + fd);
                    }
                    blocked[j] = until.map_or(false, |u| j <= u);
                }
            }
            let atr5 = if g("exit").round() == 1.0 { atr(x.r, 5) } else { Vec::new() };
            let mut out = Vec::new();
            let mut i = from.max(rn + 1);
            while i < to {
                let k = if same { i } else { i - 1 };
                let rng = h[i] - l[i];
                let wide = (i - rn..i).all(|j| rng > h[j] - l[j]);
                let d = if dm { c[i] - o[i] } else { c[i] - c[i - 1] };
                if !(wide && d != 0.0 && x.all(k, md, &ok) && !blocked[i]) {
                    i += 1;
                    continue;
                }
                let t = match g("exit").round() as i64 {
                    1 | 2 => std_exit(x, &atr5, g("exit").round() as i64, d.signum(), i, to),
                    _ => {
                        let j = (i + hold).min(to);
                        let open = i + hold > to;
                        trade(d.signum(), i, c[i], j, c[j], if open { "open".into() } else { format!("{hold} days held") }, open)
                    }
                };
                i = t.exit_i + 1;
                out.push(t);
            }
            out
        }
        _ => return None,
    })
}

/// [Not from the book] the user's standard exits for a close entry at bar `i` (VERIFICATION-PIPELINE section 4, same
/// contract as ats): mode 1 = trailing stop 2 x Wilder ATR(5) of the previous bar behind the best price since entry
/// (entry price, then the highs / lows of the bars after the entry bar), from the bar after entry, ratcheting, filled at
/// the level or at a gapped open; mode 2 = first close above the previous close after the entry bar (shorts: below).
/// A position still held at `to` is reported open at the last close.
fn std_exit(x: &Ctx, atr5: &[f64], mode: i64, dir: f64, i: usize, to: usize) -> Trade {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (mut best, mut stop) = (c[i], f64::NAN);
    for k in i + 1..=to {
        if mode == 1 {
            if k > i + 1 {
                best = if dir > 0.0 { best.max(h[k - 1]) } else { best.min(l[k - 1]) };
            }
            let lv = best - dir * 2.0 * atr5[k - 1];
            if lv.is_finite() {
                stop = if dir > 0.0 { stop.max(lv) } else { stop.min(lv) }; // NaN stop -> lv
            }
            if stop.is_finite() && (if dir > 0.0 { l[k] <= stop } else { h[k] >= stop }) {
                let px = if dir > 0.0 { o[k].min(stop) } else { o[k].max(stop) };
                return trade(dir, i, c[i], k, px, "2 x ATR(5) trailing stop".into(), false);
            }
        } else if if dir > 0.0 { c[k] > c[k - 1] } else { c[k] < c[k - 1] } {
            return trade(dir, i, c[i], k, c[k], if dir > 0.0 { "first up close" } else { "first down close" }.into(), false);
        }
    }
    trade(dir, i, c[i], to, c[to], "open".into(), true)
}

/// CVR I underlying trades: enter on the signal close, exit on the close `hold` bars later;
/// same-direction signals in a trade are ignored; with `rev`, an opposite signal reverses on that close.
fn cvr_sim(x: &Ctx, sig: &[i8], side: i8, hold: usize, rev: bool, from: usize, to: usize) -> Vec<Trade> {
    let c = &x.c;
    let mut out = Vec::new();
    let mut cur: Option<(i8, usize)> = None;
    for i in from..=to {
        if let Some((pos, e)) = cur {
            if rev && sig[i] == -pos {
                out.push(trade(pos as f64, e, c[e], i, c[i], "reversed".into(), false));
                cur = (i < to).then_some((-pos, i));
            } else if i >= e + hold {
                out.push(trade(pos as f64, e, c[e], i, c[i], format!("{hold} days held"), false));
                cur = None;
            } else if i == to {
                out.push(trade(pos as f64, e, c[e], i, c[i], "open".into(), true));
            }
            continue;
        }
        if i < to && sig[i] != 0 && (side == 0 || sig[i] == side) {
            cur = Some((sig[i], i));
        }
    }
    out
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "otvt_diz_short_strangle" => ("otvt_diz", "Diz volatility mean reversion", "Short strangle"),
        "otvt_cvr1" => ("otvt_cvr1", "Connors VIX Reversal I (OTVT)", "Buys and sells"),
        "otvt_cvr1_long" => ("otvt_cvr1", "Connors VIX Reversal I (OTVT)", "Buys"),
        "otvt_cvr1_short" => ("otvt_cvr1", "Connors VIX Reversal I (OTVT)", "Sells"),
        "otvt_cvr1_sell_puts" => ("otvt_cvr1", "Connors VIX Reversal I (OTVT)", "Options: sell puts"),
        "otvt_cvr1_buy_puts" => ("otvt_cvr1", "Connors VIX Reversal I (OTVT)", "Options: buy puts"),
        "otvt_split_call_selling" => ("otvt_split", "Options on stock splits", "Sell calls"),
        "otvt_sector_options_overpriced" => ("otvt_sector", "Overpriced sector options", "Sell IV >= 3x HV50"),
        "otvt_hv_ratio_under_50" => ("otvt_hv", "HV ratio under 50%", "6/100 or 10/100"),
        "otvt_hv_ratio_combined" => ("otvt_hv", "HV ratio under 50%", "6/100 and 10/100"),
        "otvt_hv_ratio_combined_40min" => ("otvt_hv", "HV ratio under 50%", "40-minute bars"),
        "otvt_hv_thrust" => ("otvt_hv", "HV ratio under 50%", "HV Thrust"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    #[test]
    fn black_scholes_and_cvr_by_hand() {
        // ATM, 1 year, 20% vol, r = 0: call = put = S * (2N(0.1) - 1) = 7.9656
        assert!((bs(true, 100.0, 100.0, 1.0, 20.0, 0.0) - 7.9656).abs() < 1e-3);
        assert!((bs(false, 100.0, 100.0, 1.0, 20.0, 0.0) - 7.9656).abs() < 1e-3);
        assert_eq!(bs(true, 90.0, 100.0, 0.0, 20.0, 0.0), 10.0);
        // r = 5%: call 10.4506, put 5.5735 (Hull's standard S=K=100, T=1, vol 20% example)
        assert!((bs(false, 100.0, 100.0, 1.0, 20.0, 5.0) - 10.4506).abs() < 1e-3);
        assert!((bs(true, 100.0, 100.0, 1.0, 20.0, 5.0) - 5.5735).abs() < 1e-3);
        // VIX bars, n = 3: bar 3 makes a 3-bar high (15) and closes below its open -> buy;
        // bar 4 makes a 3-bar low (9) and closes above its open -> sell; bar 2 is a high but closes above its open.
        let o = [10.0, 11.0, 11.0, 14.0, 10.0];
        let hh = [11.0, 12.0, 13.0, 15.0, 12.0];
        let ll = [10.0, 10.0, 10.5, 12.0, 9.0];
        let cc = [10.5, 11.5, 12.0, 13.0, 11.0];
        assert_eq!(cvr_signals([&o, &hh, &ll, &cc], 3), vec![0, 0, 0, 1, -1]);
        // buy on bar 3, the sell on bar 4 reverses, the short is held 3 bars
        let px = [100.0, 101.0, 102.0, 103.0, 104.0, 103.0, 102.0, 101.0, 100.0];
        let rows: Vec<Row> = px.iter().enumerate().map(|(i, &p)| Row { ticker: "T".into(), date: format!("2020-01-{:02}", i + 1), open: p, high: p, low: p, close: p, volume: None }).collect();
        let ctx = Ctx::new(&rows, &rows, &[]);
        let t = cvr_sim(&ctx, &[0, 0, 0, 1, -1, 0, 0, 0, 0], 0, 3, true, 0, 8);
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].dir), (3, 4, 1.0));
        assert_eq!((t[1].legs[0].i, t[1].exit_i, t[1].dir), (4, 7, -1.0));
        assert!((t[1].ret(0.0) - (1.0 - 101.0 / 104.0)).abs() < 1e-12);
    }

    #[test]
    fn gaps_for_wrong_vol_index_and_missing_vxo() {
        let rows = |t: &str| -> Vec<Row> { (1..=5).map(|i| Row { ticker: t.into(), date: format!("2020-01-{i:02}"), open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: None }).collect() };
        let (spy, qqq) = (rows("SPY"), rows("QQQ"));
        let (spy, qqq) = (Ctx::new(&spy, &spy, &[]), Ctx::new(&qqq, &qqq, &[]));
        let (none, vxo) = (Params::new(), Params::from([("src".to_string(), 1.0)]));
        assert!(gap("otvt_cvr1_sell_puts", &qqq, 0, 4, &none).is_some()); // F5: QQQ options are not priced by the SPX VIX
        assert!(gap("otvt_cvr1", &qqq, 0, 4, &none).is_none()); // underlying CVR runs on any ticker
        assert!(gap("otvt_cvr1_buy_puts", &spy, 0, 4, &none).is_none());
        assert!(gap("otvt_cvr1", &spy, 0, 4, &vxo).is_some()); // F3: src=1 with no VXO_OHLC
        assert_eq!(run("otvt_cvr1", &spy, 0, 4, &vxo).map(|t| t.len()), Some(0));
    }

    #[test]
    fn hv_thrust_standard_exits() {
        // long close entry at bar 0 (100): bar 1 closes lower, bar 2 higher -> first up close = bar 2
        let b = [(100.0, 101.0, 99.0, 100.0), (100.0, 100.5, 98.0, 99.0), (99.0, 101.0, 98.5, 100.5), (100.0, 100.0, 90.0, 91.0)];
        let rows: Vec<Row> = b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2020-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let ctx = Ctx::new(&rows, &rows, &[]);
        let t = std_exit(&ctx, &[], 2, 1.0, 0, 3);
        assert_eq!((t.exit_i, t.exit_px, t.reason.as_str(), t.open), (2, 100.5, "first up close", false));
        let t = std_exit(&ctx, &[], 2, -1.0, 0, 3);
        assert_eq!((t.exit_i, t.reason.as_str()), (1, "first down close"));
        // ATR trail: level = best since entry - 2 x ATR[k-1]; ATR 1.0 -> bar 1 level 98 (low 98 hits, fill 98);
        // a NaN ATR leaves the trade open at the last bar
        let t = std_exit(&ctx, &[1.0; 4], 1, 1.0, 0, 3);
        assert_eq!((t.exit_i, t.exit_px, t.reason.as_str()), (1, 98.0, "2 x ATR(5) trailing stop"));
        let t = std_exit(&ctx, &[f64::NAN; 4], 1, 1.0, 0, 3);
        assert!(t.open && t.exit_i == 3);
        assert_eq!(params("otvt_hv_thrust").unwrap().iter().find(|q| q.key == "exit").map(|q| q.def), Some(0.0));
    }
}
