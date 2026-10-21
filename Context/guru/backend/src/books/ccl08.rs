//! Larry Connors / The Connors Group, "TradingMarkets Chairman's Club 2008" (September 2008 online sessions).
//! Book id "ccl08". Source copy: Books/Larry Connors/Chairman's Club 2008/ ("CC 2008 Session 1.pdf" 24 slides,
//! "CC 2008 Session 2.pdf" 103 slides, "CRDT Equity.pdf" = slides 58-93 of a later deck). Refs: S1 sN / S2 sN /
//! CRDT sN = printed slide number. Notes, duplicates and the oracle: library/ccl08/.
//! Coded here: the CR Equity Day Trader (CRDT) limit-order day trades (I, II, III, IV, V long; I, II, IV, V short)
//! and the TPS "preview" rules of Session 2. Options programs are NotTestable; RSI 25, %b and Cumulative RSI are
//! vault duplicates (README). The day-trade simulator is local (no jkbwts::sim).
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "ccl08";

/// Rules re-read from the slide images and admitted 2026-10-21 (connors/newrules-cc/SOURCE-CONTRACT.md): the nine CRDT
/// limit-order day trades (exact on daily OHLC: limit then a stop beyond it, or the close) and the three TPS rules.
pub const REVIEWED_DAILY: &[&str] = &["ccl08_crdt1_long", "ccl08_crdt1_short", "ccl08_crdt2_long", "ccl08_crdt3_long", "ccl08_crdt4_long",
    "ccl08_crdt5_long", "ccl08_crdt2_short", "ccl08_crdt4_short", "ccl08_crdt5_short", "ccl08_tps_long", "ccl08_tps_short", "ccl08_tps_research"];
pub fn reviewed_daily(id: &str) -> bool { REVIEWED_DAILY.contains(&id) }
/// CRDT day trades: stock methods (index/ETF a declared application). Their limit fills read the open.
pub fn day_trade(id: &str) -> bool { reviewed_daily(id) && id.starts_with("ccl08_crdt") }

const DT_EXIT: &str = "X|\"EXIT - 10% stop or on the close.\" = stop 10% beyond the entry fill (filled at the stop; a day trade cannot gap through it); otherwise exit at that day's close";
const LIMIT_NOTE: &str = "Entry = limit order for the day after the setup day only. [Not from the book] fill convention: a buy fills if that day's low <= limit, at min(open, limit) (a gap below the limit fills at the open); a short fills if high >= limit, at max(open, limit); the decks print only 'Buy X% below previous close' and '10% stop or at the close'. The 10% stop is then certain whenever the low (high) reaches 0.90 (1.10) x fill, because price must pass the limit before it can reach the stop, so daily OHLC orders every fill without ambiguity. Percent limits are not rounded to the price tick [Not from the book]. Stock method (Connors' US stock day trades, Bloomberg/TradeStation examples on US stocks): runs on actual US and NSE stocks, and on indices/ETFs as a declared application where moves of this size are rare. [Not from the book] LF: the decks never define the Liquidity Filter; the vault uses the TradingMarkets Liquidity Filter of Connors' R3/R4 guide (updated 1/19/10): 1,000,000 x SMA20(close x volume) / SMA65(benchmark close x volume), benchmark SPX for US instruments (the guide's S&P 500) and NIFTY 50 for Indian ones; windows where it cannot be computed are refused. [Not from the book] HV100 = sample standard deviation of the last 100 daily log returns x sqrt(252) x 100 (the decks give no formula).";

const CRDT1_L: &[&str] = &[
    "F|\"1. The stock must be above its 200 day moving average.\" (S1 s13) = setup-day close > SMA(200)",
    "F|\"2. LF is greater than 3.\"",
    "F|\"3. The historical volatility of the stock must be above 20.\" [Not from the book: period not printed on S1; the CRDT deck states HV100 for every CRDT rule] = HV100 > 20",
    "E|\"4. Yesterday the stock must have dropped at least 3% (4%, 5%) or more and closed below its open.\" = setup day: close <= prior close x 0.97 and close < open",
    "E|\"5. We also want the 2 period RSI on the close from yesterday to be less than 20 (15, 10) to further assure the stock is oversold.\"",
    "E|\"6. ENTRY - Today the selling must follow through from yesterday and the stock must drop 6% (7%, 8%) from yesterdays close. If it does-buy.\" (S1 s14) = buy limit 6% below the setup close",
    DT_EXIT,
];
const CRDT1_S: &[&str] = &[
    "F|\"1. The stock must be above its 200-day moving average.\" (S1 s16)",
    "F|\"2. LF is greater than 10.\"",
    "F|\"3. The historical volatility of the stock must be above 20.\" = HV100 > 20 [Not from the book: period not printed on S1; HV100 as in the CRDT deck]",
    "E|\"4. Yesterday, the stock must have risen at least 4% (5%, 6%) and closed above its open.\"",
    "E|\"5. We also want the 2-period RSI to be greater than 85 (90,95) to further assure the stock is overbought.\"",
    "E|\"6. ENTRY - Today the buying must follow through from yesterday and the stock must rise 4% (5%, 6%) above yesterday's close. If it does-sell short.\" (S1 s17) = short limit 4% above the setup close",
    DT_EXIT,
];
const CRDT2_L: &[&str] = &[
    "F|\"1. HV 100 is greater than 20\" (S2 s20)",
    "E|\"2. 2 period RSI is less than 20\"",
    "F|\"3. Close is above the 200 day\"",
    "E|\"4. Close is less than the open\"",
    "E|\"5. The stock drops 0-3% for the day\" = setup-day return in [-3%, 0%) [Not from the book: '0-3%' read as a down day of at most 3%]",
    "E|\"6. Buy 10% below previous days close\"",
    "X|\"7. Exit on close or at 10% stop\"",
];
const CRDT3_L: &[&str] = &[
    "F|\"1. HV100 > 30.\" (CRDT s61)",
    "E|\"2. Stock must be have 4 days in a row where there's a lower low, lower high, and lower close.\" = on each of the last 4 days low < prior low, high < prior high, close < prior close",
    "F|\"3. C > 200MA.\"", "F|\"4. LF > 3.\"",
    "E|\"5. Buy 5% below previous close.\"",
    "X|\"Exit Rules 1. 10% stop from entry price. Or 2. At the Close.\" (CRDT s62)",
];
const CRDT4_L: &[&str] = &[
    "F|\"1. HV100 > 20\" (CRDT s66)",
    "E|\"2. Stock closes below the open\"",
    "E|\"3. The stock is down 15% or more over the past 4 days.\" = close <= close 4 days earlier x 0.85",
    "F|\"4. C > 200MA\"", "F|\"5. LF > 3\"",
    "E|\"6. Buy 5% below previous close.\"",
    "X|\"Exit Rules 1. 10% stop from entry price. Or 2. At the Close.\" (CRDT s67)",
];
const CRDT5_L: &[&str] = &[
    "F|\"1. HV100 > 20\" (CRDT s71)",
    "E|\"2. Stock must be have been down versus the previous days close.\"",
    "E|\"3. Stock closes below the open.\"",
    "E|\"4. The past two 2 days return is down 10% or more.\" = close <= close 2 days earlier x 0.90",
    "F|\"5. LF > 3\" (no 200-day MA rule in CRDT V)",
    "E|\"6. Buy 5% below previous close .\"",
    "X|\"Exit Rules 1. 10% stop from entry price. Or 2. At the Close.\" (CRDT s72)",
];
const CRDT2_S: &[&str] = &[
    "F|\"1. HV100 > 20\" (CRDT s76)",
    "E|\"2. RSI(2) > 90\"", "E|\"3. Close > Open\"",
    "E|\"4. 0% < 1 day return < 5%\" [the s78 example caption says 'yesterday was down, and up less than 5%'; the printed rules 3-4 govern]",
    "F|\"5. C > 200MA\"", "F|\"6. LF > 10\"",
    "E|\"7. Sell short 10% above previous close\"",
    "X|\"Exit Rules 1. 10% stop from entry price. Or 2. At the Close.\" (CRDT s77)",
];
const CRDT4_S: &[&str] = &[
    "F|\"1. HV100 > 20\" (CRDT s81)",
    "E|\"2. 4 days return > 20%\" = close > close 4 days earlier x 1.20",
    "F|\"3. C > 200MA\"", "F|\"4. LF > 10\"",
    "E|\"5. Sell short 5% above previous close (Setup's day)\"",
    "X|\"Exit Rules 1. 10% stop from entry price. Or 2. At the Close.\" (CRDT s82)",
];
const CRDT5_S: &[&str] = &[
    "F|\"1. HV100 > 20\" (CRDT s85)",
    "E|\"2. Stock must be 2 up days in a row\" = close > prior close on each of the last 2 days",
    "E|\"3. 2 days return > 10%\" = close > close 2 days earlier x 1.10",
    "F|\"4. C > 200MA\"", "F|\"5. LF > 10\"",
    "E|\"6. Sell short 7% above previous close .\"",
    "X|\"Exit Rules 1. 10% stop from entry price. Or 2. At the Close.\" (CRDT s86)",
];
const TPS_L: &[&str] = &[
    "F|\"Buy 1. Close Above 200MA by at least 1.5\" (S2 s95) [the % sign is printed on the short slide s96] = close >= SMA(200) x 1.015",
    "E|\"2. RSI(2)< 40 3. Buy 1 share\" [timing printed on s96: \"Enter on Close\"] = buy 1 unit (1/10 of the full 1+2+3+4 position) at the close",
    "A|\"Scale In Scale 1: Close is less than previous entry. Buy 2 shares Scale 2: ... Buy 3 shares Scale 3: ... Buy 4 shares\" = on a later close below the last entry price, add 2, then 3, then 4 units at that close",
    "X|\"Sell RSI(2) > 70 or Close Below 200MA by at least 1.5%\" = exit the whole position at the close when RSI(2) > 70 or close <= SMA(200) x 0.985",
];
const TPS_S: &[&str] = &[
    "E|\"Short RSI(2) > 70 for 2 days\" (S2 s96) = RSI(2) > 70 on this and the previous close",
    "F|\"Close Below 200MA by at least 1.5%\" = close <= SMA(200) x 0.985",
    "E|\"Buy 1 share Enter on Close\" = short 1 unit (1/10) at the close",
    "A|\"Scale In Scale 1: Close is greater than previous entry. Buy 2 shares Scale 2: ... 3 shares Scale 3: ... 4 shares\"",
    "X|\"Exit RSI(2) < 25 or Close Above 200MA by at least 1.5%\" = cover at the close",
];
const TPS_R: &[&str] = &[
    "F|\"TPS Where the Research Started. Buy 1. C > 200MA\" (S2 s93)",
    "E|\"2. Yesterday RSI2 < 75 (this is added to make it easy to do optimization runs) 3. Buy 2 shares\" = buy 2 units (of 20) at the close [Not from the book: s93 gives no fill time; the close is taken from the deck's TPS rule \"Enter on Close\", s96]",
    "A|\"Scale In 1. If still in position 1 day later buy 4 shares 2. If still in position 2 days later buy 6 shares 3. If still in position 3 days later buy 8 shares 4. Max 20 shares\" = adds at the 1st, 2nd and 3rd closes after entry",
    "X|\"Sell 1. RSI2 > 75\" = exit at the close on which RSI(2) > 75",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    let v = |s: StrategyDef| StrategyDef { needs_volume: true, ..s };
    vec![
        v(d("ccl08_crdt1_long", "CR Equity Day Trader · long (3% down day, RSI(2) < 20, buy 6% below)", "Session 1 · CR Equity Day Trader", "Long", Live, CRDT1_L, LIMIT_NOTE,
          claim("53 longs", "-", "+4.25%", "1 day", "Simulated returns of all signals 15-25 Sep 2008, US stocks (S2 s14; long and short CRDT Equity signals)"))),
        v(d("ccl08_crdt1_short", "CR Equity Day Trader · short (4% up day, RSI(2) > 85, short 4% above)", "Session 1 · CR Equity Day Trader", "Short", Live, CRDT1_S, LIMIT_NOTE,
          claim("36 shorts", "-", "+3.10%", "1 day", "Simulated returns of all signals 15-25 Sep 2008, US stocks (S2 s14)"))),
        d("ccl08_crdt2_long", "CR Equity Day Trading II · long (0-3% down day, RSI(2) < 20, buy 10% below)", "Session 2 · CR Equity Day Trading II", "Long", Live, CRDT2_L,
          "No LF rule in CRDT II (S2 s20). Same limit/stop conventions as CRDT I. Results were shown from a spreadsheet, not printed (S2 s21).", None),
        v(d("ccl08_crdt3_long", "CRDT III · long (4 lower highs/lows/closes, HV100 > 30, buy 5% below)", "CRDT Equity · III", "Long", Live, CRDT3_L, LIMIT_NOTE, None)),
        v(d("ccl08_crdt4_long", "CRDT IV · long (down 15% in 4 days, buy 5% below)", "CRDT Equity · IV", "Long", Live, CRDT4_L, LIMIT_NOTE, None)),
        v(d("ccl08_crdt5_long", "CRDT V · long (down 10% in 2 days, buy 5% below)", "CRDT Equity · V", "Long", Live, CRDT5_L, LIMIT_NOTE, None)),
        v(d("ccl08_crdt2_short", "CRDT II Short (RSI(2) > 90, up 0-5%, short 10% above)", "CRDT Equity · II Short", "Short", Live, CRDT2_S, LIMIT_NOTE, None)),
        v(d("ccl08_crdt4_short", "CRDT IV Short (up 20% in 4 days, short 5% above)", "CRDT Equity · IV Short", "Short", Live, CRDT4_S, LIMIT_NOTE, None)),
        v(d("ccl08_crdt5_short", "CRDT V Short (2 up days, up 10% in 2 days, short 7% above)", "CRDT Equity · V Short", "Short", Live, CRDT5_S, LIMIT_NOTE, None)),
        d("ccl08_tps_long", "TPS preview · long (1-2-3-4 units, RSI(2) < 40, 1.5% over the 200-day)", "Session 2 · TPS preview", "Long", Live, TPS_L,
          "Session 2 'TPS - Preview' rules (s95), not the stc9 2-3-5 course version (RSI(2) < 30, 20/30/50%) nor the HPETF book version (RSI(2) < 25 two days, 10/20/30/40%). Book's 'SP-500' trade list for the past 12 months (s99) is reproduced on SPX (README). Entry price in that list = weighted average of the 1-2-3-4 legs. Slide 100: 'Exiting on the next days open improves the test results on the SPX' = labelled option p.exit_next=1 (default 0 = close). [Not from the book] Order of checks on each close: the exit rule first (no add on an exit close), then the scale-in; no new entry on the exit close; one position at a time; unit weights = shares / the maximum position (1+2+3+4 = 10; 2+4+6+8 = 20). Index/ETF only: the deck shows TPS on the S&P 500 (SPX/SPY); other indices/ETFs, NIFTY 50 included, are a declared application.",
          claim("3 longs (SPX)", "-", "-", "-", "TPS SPY trades past 12 months, SP-500, 9/2007-8/2008 (S2 s99)")),
        d("ccl08_tps_short", "TPS preview · short (RSI(2) > 70 two days, 1.5% under the 200-day)", "Session 2 · TPS preview", "Short", Live, TPS_S,
          "Session 2 s96. Differs from stc9 tps_short (RSI(2) > 75 two days, 2-3-5). p.exit_next=1 = the s100 next-open exit (labelled option). [Not from the book] Order of checks on each close: the exit rule first (no add on an exit close), then the scale-in; no new entry on the exit close; one position at a time; unit weights = shares / the maximum position (1+2+3+4 = 10; 2+4+6+8 = 20). Index/ETF only: the deck shows TPS on the S&P 500 (SPX/SPY); other indices/ETFs, NIFTY 50 included, are a declared application.",
          claim("9 shorts (SPX)", "-", "-", "-", "TPS SPY trades past 12 months, SP-500, 9/2007-8/2008 (S2 s99)")),
        d("ccl08_tps_research", "TPS 'where the research started' (time scale-in 2-4-6-8)", "Session 2 · TPS preview", "Long", Live, TPS_R,
          "S2 s93, printed as the starting point of the TPS research. The entry filter is only 'C > 200MA' plus 'Yesterday RSI2 < 75', which the slide says was added for optimisation runs; coded literally, so it trades often. Fill timing (close) from the TPS rule slide s96. [Not from the book] Order of checks on each close: the exit rule first (no add on an exit close), then the scale-in; no new entry on the exit close; one position at a time; unit weights = shares / the maximum position (1+2+3+4 = 10; 2+4+6+8 = 20). Index/ETF only: the deck shows TPS on the S&P 500 (SPX/SPY); other indices/ETFs, NIFTY 50 included, are a declared application.", None),
        d("ccl08_crdt_emini", "CRDT E-mini/SPY (breakout day trades)", "Session 1/CRDT · E-mini", "Long/Short",
          NotTestable("the rules are not in these files: Session 1 lists it on the agenda and the CRDT deck (s92) shows only 'observations so far' and its relation to EEV; it also needs intraday e-mini data"),
          &["E|\"Breakout trading (e-minis & SPYs)\" (S1 s9); \"CRDT E-mini/SPY - Observations so far\" (CRDT s92)"], "", None),
        d("ccl08_weekly_rsi_options", "Monthly Options Income I · Weekly RSI Program for ETFs", "Session 2 · Options", "Short",
          NotTestable("the position is short calls / call spreads (P&L needs option chains); the exit menu is also a choice ('under 80, 70, 60 (the latter is preferred)')"),
          &["F|\"1. No 200 day. 2. Use Weekly Bars. 3. The 2-period RSI (weekly RSI) closes above 85.\" (S2 s32)",
            "E|\"4. Aggressive - Sell Naked the ATM or OTM calls. 5. More Conservative-Sell the OTM Ratio Call Spread. 6. Even more conservative. Sell OTM Vertical Call Spread.\"",
            "X|\"7. Exit one week later or when the daily 2-period RSI drops under 80, 70, 60 (the latter is preferred). 8. Possible Stops-2x the premium...\" (s33)"], "", None),
        d("ccl08_quick_scalp", "Monthly Options Income II · ETF Quick Scalp", "Session 2 · Options", "Short",
          NotTestable("sells calls / call spreads; P&L needs option chains"),
          &["E|\"1. No 200 day. 2. The 2 (two) period RSI of the ETF is above 97 3. Sell the higher strike price calls\" (S2 s42)", "X|\"5. Exit on the first down close in price.\""], "", None),
        d("ccl08_adx_quiet", "Monthly Options Income III · ADX Quiet Money (iron condors)", "Session 2 · Options", "Long/Short",
          NotTestable("iron condors on SPX; P&L needs option chains"),
          &["E|\"1. The 10 period ADX of a security drops under 20 ... selling spreads on each side of the stock with Iron Condors.\" (S2 s51)", "X|\"3. The exit is when the ADX rises 10 points from its low or 15 trading days, whichever comes first.\" (s56)"], "", None),
        d("ccl08_busted_options", "Monthly Options Income IV · Busted Options", "Session 2 · Options", "Short",
          NotTestable("sells next-month calls after an earnings gap; needs earnings dates and option chains"),
          &["F|\"1. The stock market (SPX) is under the 200 day MA. 2. A stock misses its earnings and opens at least 15% below yesterday's close.\" (S2 s60)",
            "E|\"3. Sell the first or second strike price out-of-the-money call for next month and if the premium is at least $1.\"", "X|\"4. Exit-Expiration ... 5. Stop-2x the price you paid for the option.\""], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ccl08_crdt1_long" => ("ccl08_crdt", "CR Equity Day Trader (CRDT)", "I long"),
        "ccl08_crdt2_long" => ("ccl08_crdt", "CR Equity Day Trader (CRDT)", "II long"),
        "ccl08_crdt3_long" => ("ccl08_crdt", "CR Equity Day Trader (CRDT)", "III long"),
        "ccl08_crdt4_long" => ("ccl08_crdt", "CR Equity Day Trader (CRDT)", "IV long"),
        "ccl08_crdt5_long" => ("ccl08_crdt", "CR Equity Day Trader (CRDT)", "V long"),
        "ccl08_crdt1_short" => ("ccl08_crdt", "CR Equity Day Trader (CRDT)", "I short"),
        "ccl08_crdt2_short" => ("ccl08_crdt", "CR Equity Day Trader (CRDT)", "II short"),
        "ccl08_crdt4_short" => ("ccl08_crdt", "CR Equity Day Trader (CRDT)", "IV short"),
        "ccl08_crdt5_short" => ("ccl08_crdt", "CR Equity Day Trader (CRDT)", "V short"),
        "ccl08_crdt_emini" => ("ccl08_crdt", "CR Equity Day Trader (CRDT)", "E-mini/SPY (not testable)"),
        "ccl08_tps_long" => ("ccl08_tps", "TPS preview (Chairman's Club 2008)", "Long 1-2-3-4"),
        "ccl08_tps_short" => ("ccl08_tps", "TPS preview (Chairman's Club 2008)", "Short 1-2-3-4"),
        "ccl08_tps_research" => ("ccl08_tps", "TPS preview (Chairman's Club 2008)", "Research start 2-4-6-8"),
        "ccl08_weekly_rsi_options" | "ccl08_quick_scalp" | "ccl08_adx_quiet" | "ccl08_busted_options" =>
            ("ccl08_options", "Chairman's Club 2008 · options income", "Options (not testable)"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let tr = || p("trend", "200-day MA length", 200., 50., 300., 50.);
    let stop = || p("stop_pct", "Stop % beyond the entry fill", 10., 2., 20., 1.);
    let lf = |v: f64| p("lf", "Liquidity Factor above", v, 0., 40., 1.);
    let hv = |v: f64| p("hv", "HV(100) above (%)", v, 0., 80., 5.);
    let ent = |v: f64| p("entry_pct", "Limit % beyond the setup close", v, 1., 15., 1.);
    Some(match id {
        "ccl08_crdt1_long" => vec![tr(), lf(3.), hv(20.), p("move_pct", "Setup day down at least % (book menu 3/4/5)", 3., 1., 10., 1.),
            p("rsi_lt", "RSI(2) below (book menu 20/15/10)", 20., 5., 40., 5.), ent(6.), stop()],
        "ccl08_crdt1_short" => vec![tr(), lf(10.), hv(20.), p("move_pct", "Setup day up at least % (book menu 4/5/6)", 4., 1., 10., 1.),
            p("rsi_gt", "RSI(2) above (book menu 85/90/95)", 85., 60., 99., 5.), ent(4.), stop()],
        "ccl08_crdt2_long" => vec![tr(), hv(20.), p("rsi_lt", "RSI(2) below", 20., 5., 40., 5.), p("move_pct", "Setup day drop at most %", 3., 1., 10., 1.), ent(10.), stop()],
        "ccl08_crdt3_long" => vec![tr(), lf(3.), hv(30.), p("days", "Days in a row of lower high, lower low, lower close", 4., 2., 7., 1.), ent(5.), stop()],
        "ccl08_crdt4_long" => vec![tr(), lf(3.), hv(20.), p("move_pct", "Down at least % over the past 4 days", 15., 5., 30., 1.), ent(5.), stop()],
        "ccl08_crdt5_long" => vec![lf(3.), hv(20.), p("move_pct", "Down at least % over the past 2 days", 10., 3., 20., 1.), ent(5.), stop()],
        "ccl08_crdt2_short" => vec![tr(), lf(10.), hv(20.), p("rsi_gt", "RSI(2) above", 90., 60., 99., 5.), p("move_pct", "Setup day up less than %", 5., 1., 10., 1.), ent(10.), stop()],
        "ccl08_crdt4_short" => vec![tr(), lf(10.), hv(20.), p("move_pct", "Up more than % over the past 4 days", 20., 5., 40., 1.), ent(5.), stop()],
        "ccl08_crdt5_short" => vec![tr(), lf(10.), hv(20.), p("move_pct", "Up more than % over the past 2 days", 10., 3., 20., 1.), ent(7.), stop()],
        "ccl08_tps_long" | "ccl08_tps_short" => vec![tr(),
            p("band_pct", "% beyond the 200-day MA (entry filter and stop)", 1.5, 0., 5., 0.5),
            if id == "ccl08_tps_long" { p("rsi_entry", "Buy when RSI(2) below", 40., 10., 50., 5.) } else { p("rsi_entry", "Short when RSI(2) above (2 days)", 70., 50., 95., 5.) },
            if id == "ccl08_tps_long" { p("rsi_exit", "Sell when RSI(2) above", 70., 50., 95., 5.) } else { p("rsi_exit", "Cover when RSI(2) below", 25., 5., 50., 5.) },
            p("exit_next", "0 = exit at the signal close (rules); 1 = next open (s100 remark) [labelled option]", 0., 0., 1., 1.)],
        "ccl08_tps_research" => vec![tr(), p("rsi_prev_lt", "Yesterday's RSI(2) below", 75., 10., 95., 5.), p("rsi_exit", "Sell when RSI(2) above", 75., 50., 95., 5.)],
        _ => return None,
    })
}

/// Limit-order day trade: setup on bar s (close), order for bar s+1 only; exit at the stop or that day's close.
fn day_trades(x: &Ctx, from: usize, to: usize, dir: f64, setup: &dyn Fn(usize) -> bool, entry_pct: f64, stop_pct: f64) -> Vec<Trade> {
    let to = to.min(x.c.len() - 1);
    let mut out = vec![];
    for s in from..to {
        if !setup(s) { continue; }
        let e = s + 1;
        let lim = x.c[s] * (1. - dir * entry_pct);
        let fill = if dir > 0. { (x.l[e] <= lim).then(|| x.o[e].min(lim)) } else { (x.h[e] >= lim).then(|| x.o[e].max(lim)) };
        let Some(px) = fill else { continue };
        let stop = px * (1. - dir * stop_pct);
        let hit = if dir > 0. { x.l[e] <= stop } else { x.h[e] >= stop };
        let (exit_px, why) = if hit { (stop, "10% stop") } else { (x.c[e], "exit at the close") };
        out.push(Trade { dir, legs: vec![Leg { i: e, px, w: 1. }], exit_i: e, exit_px, reason: why.into(), entry_kind: "limit",
            signal_i: e, open: false, exits: vec![] });
    }
    out
}

/// TPS: entry at a close, adds at later closes (`add(i, legs)` -> Some(weight)), whole exit at a close or next open.
fn scale_sim(x: &Ctx, from: usize, to: usize, dir: f64, entry: &dyn Fn(usize) -> Option<f64>, add: &dyn Fn(usize, &[Leg]) -> Option<f64>,
             exit: &dyn Fn(usize) -> bool, exit_next: bool) -> Vec<Trade> {
    let to = to.min(x.c.len() - 1);
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    let mut pend = false;
    for i in from..=to {
        if pend {
            let mut t = cur.take().unwrap();
            (t.exit_i, t.exit_px, t.signal_i, t.reason) = (i, x.o[i], i - 1, "exit rule, out next open".into());
            out.push(t);
            pend = false;
        }
        if let Some(t) = cur.as_mut() {
            if exit(i) {
                if exit_next {
                    if i < to { pend = true; }
                } else {
                    let mut t = cur.take().unwrap();
                    (t.exit_i, t.exit_px, t.signal_i, t.reason) = (i, x.c[i], i, "exit rule at the close".into());
                    out.push(t);
                }
                continue;
            }
            if let Some(w) = add(i, &t.legs) { t.legs.push(Leg { i, px: x.c[i], w }); }
            continue;
        }
        if let Some(w) = entry(i) {
            cur = Some(Trade { dir, legs: vec![Leg { i, px: x.c[i], w }], exit_i: i, exit_px: f64::NAN, reason: String::new(),
                entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur {
        (t.exit_i, t.exit_px, t.reason, t.signal_i, t.open) = (to, x.c[to], "open".into(), to, true);
        out.push(t);
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (c, o, h, l) = (&x.c, &x.o, &x.h, &x.l);
    let ok = |v: f64| v.is_finite();
    let ma = x.sma_n(if schema.iter().any(|d| d.key == "trend") { g("trend") as usize } else { 200 });
    let r2 = &x.rsi2;
    let above = |i: usize| ok(ma[i]) && c[i] > ma[i];
    let ret = |i: usize, k: usize| if i >= k { c[i] / c[i - k] - 1. } else { f64::NAN };
    let pct = |k: &str| g(k) / 100.;
    let lf_ok = |i: usize| x.lf[i] > g("lf");
    let hv_ok = |i: usize| x.hv100[i] > g("hv");
    let dt = |dir: f64, setup: &dyn Fn(usize) -> bool| day_trades(x, from, to, dir, setup, pct("entry_pct"), pct("stop_pct"));
    Some(match id {
        "ccl08_crdt1_long" => dt(1., &|i| above(i) && lf_ok(i) && hv_ok(i) && ret(i, 1) <= -pct("move_pct") && c[i] < o[i] && r2[i] < g("rsi_lt")),
        "ccl08_crdt1_short" => dt(-1., &|i| above(i) && lf_ok(i) && hv_ok(i) && ret(i, 1) >= pct("move_pct") && c[i] > o[i] && r2[i] > g("rsi_gt")),
        "ccl08_crdt2_long" => dt(1., &|i| hv_ok(i) && r2[i] < g("rsi_lt") && above(i) && c[i] < o[i] && ret(i, 1) < 0. && ret(i, 1) >= -pct("move_pct")),
        "ccl08_crdt3_long" => {
            let k = g("days") as usize;
            dt(1., &|i| hv_ok(i) && i >= k && (i + 1 - k..=i).all(|j| l[j] < l[j - 1] && h[j] < h[j - 1] && c[j] < c[j - 1]) && above(i) && lf_ok(i))
        }
        "ccl08_crdt4_long" => dt(1., &|i| hv_ok(i) && c[i] < o[i] && ret(i, 4) <= -pct("move_pct") && above(i) && lf_ok(i)),
        "ccl08_crdt5_long" => dt(1., &|i| hv_ok(i) && ret(i, 1) < 0. && c[i] < o[i] && ret(i, 2) <= -pct("move_pct") && lf_ok(i)),
        "ccl08_crdt2_short" => dt(-1., &|i| hv_ok(i) && r2[i] > g("rsi_gt") && c[i] > o[i] && ret(i, 1) > 0. && ret(i, 1) < pct("move_pct") && above(i) && lf_ok(i)),
        "ccl08_crdt4_short" => dt(-1., &|i| hv_ok(i) && ret(i, 4) > pct("move_pct") && above(i) && lf_ok(i)),
        "ccl08_crdt5_short" => dt(-1., &|i| hv_ok(i) && i >= 2 && c[i] > c[i - 1] && c[i - 1] > c[i - 2] && ret(i, 2) > pct("move_pct") && above(i) && lf_ok(i)),
        "ccl08_tps_long" | "ccl08_tps_short" => {
            let long = id == "ccl08_tps_long";
            let b = pct("band_pct");
            let (re, rx) = (g("rsi_entry"), g("rsi_exit"));
            let up = |i: usize| ok(ma[i]) && c[i] >= ma[i] * (1. + b);
            let dn = |i: usize| ok(ma[i]) && c[i] <= ma[i] * (1. - b);
            let w = [0.1, 0.2, 0.3, 0.4];
            let entry = |i: usize| (if long { up(i) && r2[i] < re } else { i >= 1 && dn(i) && r2[i] > re && r2[i - 1] > re }).then_some(w[0]);
            let add = |i: usize, legs: &[Leg]| {
                let last = legs.last().unwrap().px;
                (legs.len() < 4 && if long { c[i] < last } else { c[i] > last }).then(|| w[legs.len()])
            };
            let exit = |i: usize| if long { r2[i] > rx || dn(i) } else { r2[i] < rx || up(i) };
            scale_sim(x, from, to, if long { 1. } else { -1. }, &entry, &add, &exit, g("exit_next") >= 0.5)
        }
        "ccl08_tps_research" => {
            let (rp, rx) = (g("rsi_prev_lt"), g("rsi_exit"));
            let w = [0.1, 0.2, 0.3, 0.4];
            let entry = |i: usize| (i >= 1 && above(i) && r2[i - 1] < rp).then_some(w[0]);
            // "If still in position 1 (2, 3) days later": the k-th close after the entry close gets the k-th add.
            let add = |i: usize, legs: &[Leg]| (legs.len() < 4 && i == legs[0].i + legs.len()).then(|| w[legs.len()]);
            scale_sim(x, from, to, 1., &entry, &add, &|i| r2[i] > rx, false)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn crdt1_long_limit_fill_and_stop() {
        // 230 rising bars, then a -4% red day with RSI(2) < 20; next day opens 2% down, trades 7% down (fill at
        // the 6% limit) and closes higher -> exit at the close; a second identical setup whose next day falls
        // 20% from the fill -> 10% stop.
        let mut cl = vec![100.0f64];
        for i in 1..230 { let v = cl[i - 1] * if i % 2 == 0 { 1.03 } else { 0.99 }; cl.push(v); }
        let mut r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("d{:04}", i), open: c, high: c * 1.01, low: c * 0.99, close: c, volume: Some(1e6) }).collect();
        let mut push = |o: f64, h: f64, l: f64, c: f64| { let n = r.len(); r.push(Row { ticker: "T".into(), date: format!("d{:04}", n), open: o, high: h, low: l, close: c, volume: Some(1e6) }); };
        let c0 = cl[229];
        let s = c0 * 0.96;
        push(c0, c0, s, s); // setup 230
        push(s * 0.98, s * 0.99, s * 0.93, s * 0.97); // 231: fill at s*0.94, close s*0.97
        let x = Ctx::new(&r, &r, &[]);
        assert!(x.rsi2[230] < 20. && x.hv100[230] > 20., "rsi {} hv {}", x.rsi2[230], x.hv100[230]);
        let t = run("ccl08_crdt1_long", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let t = t.last().unwrap();
        assert_eq!((t.legs[0].i, t.exit_i), (231, 231));
        assert!((t.legs[0].px - s * 0.94).abs() < 1e-9 && (t.exit_px - s * 0.97).abs() < 1e-9);
        // stop case
        r.pop();
        r.push(Row { ticker: "T".into(), date: "d0231".into(), open: s * 0.98, high: s * 0.99, low: s * 0.80, close: s * 0.82, volume: Some(1e6) });
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ccl08_crdt1_long", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let t = t.last().unwrap();
        assert!((t.exit_px - s * 0.94 * 0.9).abs() < 1e-9 && t.reason == "10% stop");
    }
}
