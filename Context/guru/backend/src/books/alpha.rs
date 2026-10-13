//! The Alpha Formula (Connors & Cain, Connors Research, 2019/2020). Book id "alpha".
//! The book's four strategies trade multi-ETF / stock portfolios on Quantopian; the vault holds one position per ticker,
//! so each full strategy is catalogued as NeedsData and its rules are also coded as a single-instrument version.
//! Live: Ch 4 SPY RSI(4) study, single-instrument Weekly Mean Reversion, single-ETF Dynamic Treasuries sleeve,
//! single-instrument ETF Avalanches, Appendix A1 six-month trend rule.
#![allow(unused_imports, dead_code)]
use crate::connors::*;

pub const BOOK: &str = "alpha";

const RSI4: &str = "alpha_spy_rsi4_mr";
const RA: &str = "alpha_rising_assets";
const WMR: &str = "alpha_weekly_mr";
const WMR1: &str = "alpha_weekly_mr_single";
const DT: &str = "alpha_dyn_treasuries";
const DT1: &str = "alpha_dyn_treasuries_sleeve";
const AV: &str = "alpha_etf_avalanches";
const AV1: &str = "alpha_etf_avalanches_single";
const PF: &str = "alpha_formula_portfolio";
const TF: &str = "alpha_trend_6m";

macro_rules! tr_note { () => { " 'Total return' = close / close N trading days ago - 1 on the vault's prices (the book used Quantopian dividend-adjusted data; vault US data is split-adjusted price only, Indian data are price indices) [data deviation]." } }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d(RSI4, "SPY Mean Reversion (RSI 4: buy < 25, add < 20, sell > 55)", "Ch 4 · Why Mean Reversion Works", "Long", Live,
          &[
              "F|SPY is above its 200-day moving average",
              "E|SPY has had a short-term pullback to the downside. We quantify this by buying the ETF on the close when the 4-period RSI (see appendix for the calculation of RSI) is below 25.",
              "A|If the 4-period RSI closes below 20 anytime we are in the trade, we add another unit to the trade. This effectively doubles our position if SPY continues to decline.",
              "X|Sell SPY on the close after it has reverted to the mean. In this case we will measure that by selling when the same 4-period RSI indicator is above 55.",
          ],
          "RSI = Wilder RSI(4) on closes (Appendix A2-1, p-129: Wilder smoothing). 200-day MA = SMA of closes, Close > SMA strict, entry bar only. \
           Add-on: one extra unit (\"add another unit\") at the close of a LATER bar of the trade with RSI4 < 20 [Book silent on the entry close; verified: on dividend-adjusted SPY, later-only reproduces 2009-2017 exactly (62 trades, 96.77% wins, avg 1.32%, 4.53 days) while adding on the entry close gives 1.22%. On the vault's price-only SPY: 142 trades, avg 1.40% for 1993-2017 vs the book's 147 / 1.43% - the gap is dividends (ex-dividend drops move RSI4 and the 200-day test)]; the text puts no 200-day condition on the add. \
           Units are weight 0.5 each (a 1-unit trade uses half the allocation); the book's 'trade' = one position (85/62/147 fit positions, not units) and its 'avg gain' is on the position's cost (verify_alpha.py prints that measure). \
           [Not from the book] p.add_entry_bar=1 lets the add fire on the entry close; p.add_trend=1 requires Close > SMA200 for the add. \
           India: NIFTY 50 for SPY (instrument swap). Source tables: The Connors Research Traders Journal (study first published in Short Term Trading Strategies That Work, 2009).",
          claim("85", "88.24%", "1.50%", "5.07 days", "SPY 1993-2008 (2009-2017: 62 / 96.77% / 1.32% / 4.53 d; 1993-2017: 147 / 91.84% / 1.43% / 4.85 d)")),
        d(RA, "Rising Assets Strategy", "Ch 5 · Rising Assets", "Long",
          NeedsData("a 12-ETF universe (SPY IWM QQQ EFA EEM VNQ LQD GLD SHY IEF TLT AGG; vault has only SPY/IWM/QQQ) and a multi-asset inverse-volatility portfolio engine (the vault holds one position per ticker)"),
          &[
              "E|On the last business day of every month, invest in the top 5 assets with the highest momentum score.",
              "X|Sell any asset that was previously in the top 5 but has since dropped out.",
              "F|Weight each asset by the inverse of its 63-day historical volatility.",
              "F|Our momentum score is the average of the last 1mo, 3mo, 6mo and 12mo trailing total returns.",
              "F|Volatility here is measured by the standard deviation of each ETF's daily returns.",
          ],
          "Cross-sectional rotation; there is no meaningful single-instrument version (ranking needs the universe), so none is coded. Months would be 21/63/126/252 trading days [Book silent]. India: would need an Indian multi-asset ETF set (equity, gold, G-sec) - not in the vault.",
          claim("n/a", "n/a", "ann 9.60%", "monthly rebalance", "12 ETFs 11/2004-12/2018: vol 11.60%, maxDD -14.80%, Sharpe 0.83, alpha 6.40% (2007-2018: 9.20% / 11.50% / -14.80% / 0.80)")),
        d(WMR, "Connors Research Weekly Mean Reversion", "Ch 6 · CR Weekly Mean Reversion", "Long",
          NeedsData("the 500 most liquid US stocks (monthly 200-day dollar-volume ranking), 10 equal-weight slots ranked by 100-day volatility, and SHY for idle cash; see alpha_weekly_mr_single for the single-instrument version"),
          WMR_RULES,
          "Full strategy: portfolio of up to 10 stocks. India: NIFTY 500 constituents would be the swap (not in the vault).",
          claim("n/a", "n/a", "ann 12.30%", "40-60 trading days", "US stocks 2003-2018: vol 10.10%, maxDD -12.80%, Sharpe 1.22, alpha 7.30%, beta 0.27 (2007-2018: 12.00% / 10.20% / -12.80% / 1.18)")),
        d(WMR1, "CR Weekly Mean Reversion · single instrument", "Ch 6 · CR Weekly Mean Reversion", "Long", Live,
          WMR_RULES,
          concat!("Single-instrument version of the stock strategy (vault convention): the traded index/ETF is also the regime index (SPY for SPY; NIFTY 50 for NIFTY 50 - instrument swap), one slot at full weight, no liquidity filter / volatility ranking (one candidate), idle cash earns 0 instead of SHY. \
           Week end = last trading day of each Monday-Sunday week. Weekly RSI(2) = Wilder RSI on those weekly closes. Regime: close / close 126 trading days ago - 1 > 0 on the week-end bar (entry filter only). \
           Entry: week-end close with regime on and weekly RSI2 < 20 -> buy the close. Exits: week-end close with weekly RSI2 > 80; any daily close more than 10% below the entry price (close < entry x 0.90) -> sell that close. \
           After an exit the next entry can only come at a later week end [Book silent]. p.exec=1 enters and exits at the next open instead (the book's tested alternative, p-065). No published single-instrument result.", tr_note!()),
          None),
        d(DT, "Connors Research Dynamic Treasuries", "Ch 7 · CR Dynamic Treasuries", "Long",
          NeedsData("IEI, IEF, TLH and TLT daily data and a 4-ETF weighted portfolio (vault has no Treasury ETFs); see alpha_dyn_treasuries_sleeve for the per-ETF rule"),
          DT_RULES,
          "Full strategy: IEF/TLH/TLT each 0-25% (5% per positive lookback), remainder IEI, weekly. India: an Indian G-sec ladder would be the swap (not in the vault).",
          claim("n/a", "n/a", "ann 4.00%", "weekly rebalance", "IEI/IEF/TLH/TLT 01/2007-12/2018: vol 6.60%, maxDD -10.80%, Sharpe 0.61; 2008 +14.03%, 2011 +18.25%")),
        d(DT1, "CR Dynamic Treasuries · single-ETF sleeve", "Ch 7 · CR Dynamic Treasuries", "Long",
          NeedsData("Treasury ETF data (IEF, TLH, TLT): the rule is written for bonds; running it on an equity index would swap the asset class, not just the instrument"),
          DT_RULES,
          concat!("Single-ETF version of the allocation rule (the book applies it to IEF, TLH, TLT): at every week-end close the instrument's weight is reset to 5% x (number of the 21/42/63/84/105-day total returns that are > 0), i.e. 0-25% of capital; the rest (the book's IEI) is idle cash here. \
           Each weekly holding is one trade (weight = that week's allocation; weeks with 0 positive lookbacks hold nothing). Needs 105 bars of history before the first allocation. \
           The vault has no Treasury ETFs, so on vault data this runs on equity indices (NIFTY 50, SPY...) - the book's instruments are bonds; no published single-ETF result.", tr_note!()),
          None),
        d(AV, "ETF Avalanches", "Ch 8 · ETF Avalanches", "Short",
          NeedsData("the 40-ETF universe of table 8.1 (vault has SPY, MDY, QQQ, IWM only), 5 short slots ranked by 100-day volatility and SHY for idle cash; see alpha_etf_avalanches_single for the per-ETF rules"),
          AV_RULES,
          "Full strategy: portfolio of up to 5 shorts across 40 country/sector/regional ETFs. Borrow costs not included in the book. India: NSE sector/broad indices would be the universe.",
          claim("n/a", "n/a", "ann 2.40%", "a few days", "40 ETFs 2007-2018: vol 10.20%, maxDD -20.00%, Sharpe 0.24; 2008 +19.49%, 2009 -11.05%")),
        d(AV1, "ETF Avalanches · single instrument", "Ch 8 · ETF Avalanches", "Short", Live,
          AV_RULES,
          concat!("Single-instrument version (one slot at full weight; SPY, MDY, QQQ, IWM are in the book's universe; NIFTY 50 / NIFTY BANK / midcap index on Indian data). \
           'At the beginning of the day' = on yesterday's close: close/close[-252] - 1 < 0, close/close[-21] - 1 < 0 and Wilder RSI(2) > 70. Then a sell limit at 1.03 x yesterday's close ('the current price' at the start of the day [Book silent: prior close]) is good for that day only: \
           fills if the high reaches it, at the limit or at the open if the open gaps above it. Exits at any close (including the fill day's) with RSI(2) < 15 or close/close[-21] - 1 > 0. \
           [Not from the book] p.limit_base=1 puts the limit 3% above today's open instead. Idle cash earns 0 instead of SHY. No published single-ETF result.", tr_note!()),
          None),
        d(PF, "The Alpha Formula Portfolio", "Ch 9 · The Alpha Formula Portfolio", "Long/Short",
          NeedsData("all four sleeves (Rising Assets 12 ETFs, Weekly Mean Reversion 500 stocks, Dynamic Treasuries 4 Treasury ETFs, ETF Avalanches 40 ETFs) combined in one multi-strategy portfolio"),
          &[
              "F|We combined the strategies for our non-leveraged portfolio in weights of: 30% - Rising Assets, 30% - CR Weekly Mean Reversion, 20% - CR Dynamic Treasuries, 20% - ETF Avalanches",
              "F|For our leveraged portfolio, we increased the weights to: 45% - Rising Assets, 45% - CR Weekly Mean Reversion, 30% - CR Dynamic Treasuries, 30% - ETF Avalanches. Resulting in total leverage coming in at 1.5:1.",
          ],
          "Sleeve rebalancing frequency not stated [Book silent]. Alpha vs SPY via CAPM, rf 1%; leveraged version borrows at Fed Funds + 50 bps; no slippage or commission.",
          claim("n/a", "n/a", "ann 8.00%", "n/a", "2007-2018 no leverage: vol 6.20%, maxDD -7.10%, Sharpe 1.29, alpha 6.50%, beta 0.073; 1.5x: 11.50% / 9.30% / -10.60% / 1.24 / alpha 9.50%")),
        d(TF, "Six-month trend-following rule (Appendix A1)", "Appendix A1 · Trend Following Reduces Risk", "Long", Live,
          &[
              "E|If the trailing 6-month total return is greater than 0, buy and hold that ETF.",
              "X|If the trailing 6-month total return is less than 0, buy and hold SHY (1-3 year US Treasuries).",
              "F|Rules are checked once a month, at the end of the month.",
              "F|For our unoptimized trend following measure, we will look at total returns over the past 6 months (126 trading days).",
          ],
          concat!("Checked on the last trading day of each month at the close: close/close[-126] - 1 > 0 -> hold (buy that close if flat); otherwise -> out (sell that close). Exactly 0 counts as 'not > 0' [Book silent]. \
           The out-of-market leg (SHY in the book) is idle cash here. The book tested US sector ETFs XLB..XLY (not in the vault) and calls it a risk-reduction illustration, 'not intended to be a stand-alone trading strategy'. India: NIFTY 50 etc. (instrument swap).", tr_note!()),
          claim("n/a", "n/a", "ann 7.67% (avg of 9 sectors)", "monthly check", "XLB..XLY 01/2003-12/2018, average vol 13.82%, Sharpe 0.56, maxDD -25.97% (buy & hold 8.62% / 20.54% / 0.46 / -54.96%)")),
    ]
}

const WMR_RULES: &[&str] = &[
    "F|All rules are checked at the end of the business week, with the exception of our percentage stop rule which is checked at the end of every business day.",
    "F|Long-term Trend Following Regime Filter. SPY's total return over the last six months (126 trading days) is positive.",
    "F|Liquidity Filter. The stock must be one of the 500 most liquid US stocks. Our 500 most liquid US stock universe is determined every month by taking the 500 US stocks with the highest 200-day average dollar volume.",
    "E|The Weekly 2-period RSI of the stock is below 20.",
    "E|All stocks that meet these criteria are then ranked by their trailing 100-day historical volatility. We then BUY on the close, in equal weight, the 10 stocks with the LOWEST historical volatility.",
    "X|SELL the stock on the close if its weekly 2-period RSI is above 80. This is checked at the end of every business Week.",
    "X|SELL the stock on the close if the current price is more than 10% below the entry price. This is checked at the end of every business Day.",
    "F|Any capital that is not allocated is put into SHY (1-3-year US Treasuries ETF).",
];

const DT_RULES: &[&str] = &[
    "E|Allocate anywhere from 0-25% of the portfolio to each of our longer duration ETFs (IEF, TLH and TLT). This allocation is based on trailing 1,2,3,4,5-month total returns (21, 42, 63, 84 and 105 trading days), with each positive lookback contributing 5% to the total allocation for that ETF.",
    "E|Any part of the portfolio not allocated to our duration ETFs is allocated to IEI.",
    "X|We rebalance this strategy once a week, at the end of the business week.",
];

const AV_RULES: &[&str] = &[
    "F|Long-Term Trend Following Regime Filter. At the beginning of the day, the security is showing a negative total return over the past year (252 trading days).",
    "F|Intermediate-Term Trend Following Confirmation. At the beginning of the day, the security is showing a negative total return over the past month (21 trading days).",
    "F|Check for Short-Term Strength. At the beginning of the day, the 2-period RSI is above 70.",
    "E|Limit Order to Enter Short. At the beginning of the day, if all of the above are true, put a sell limit order 3.0% above the current price. If the order gets filled during that day, then we are short the ETF. If the order doesn't get filled by the end of the day, it is canceled.",
    "X|RSI Exit. Exit if the 2-period RSI is below 15. This is checked every business day at the close.",
    "X|Intermediate-Term Momentum Exit. Exit if the trailing one-month (21 trading days) total return turns positive. This is checked every business day at the close.",
    "F|Maximum Positions. Our strategy holds a maximum of 5 short positions.",
    "F|Signal Ranking. Positions are ranked by their 100-day historical volatility (standard deviation), from high volatility to low volatility.",
    "F|How to Allocate Cash. The percentage of capital that is not allocated to the short positions is placed into SHY (iShares 1-3-year Treasury Bond ETF)",
];

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        RSI4 => vec![
            p("trend", "Trend filter: SMA period", 200.0, 50.0, 300.0, 10.0),
            p("rsi_n", "RSI period", 4.0, 2.0, 14.0, 1.0),
            p("entry", "Buy when RSI below", 25.0, 5.0, 40.0, 5.0),
            p("add", "Add a unit when RSI closes below", 20.0, 5.0, 25.0, 5.0),
            p("exit", "Sell when RSI above", 55.0, 40.0, 80.0, 5.0),
            p("add_entry_bar", "[Not from the book] Add-on may fire on the entry close: 1 = yes, 0 = later closes only (default)", 0.0, 0.0, 1.0, 1.0),
            p("add_trend", "[Not from the book] Add-on also needs Close > SMA: 1 = yes, 0 = no (default, as written)", 0.0, 0.0, 1.0, 1.0),
        ],
        WMR1 => vec![
            p("regime_n", "Regime: total-return lookback (days)", 126.0, 63.0, 252.0, 21.0),
            p("rsi_n", "Weekly RSI period", 2.0, 2.0, 4.0, 1.0),
            p("entry", "Buy when weekly RSI below", 20.0, 10.0, 30.0, 5.0),
            p("exit", "Sell when weekly RSI above", 80.0, 60.0, 90.0, 5.0),
            p("stop_pct", "Stop: close more than % below entry", 10.0, 5.0, 15.0, 1.0),
            p("exec", "Fills: 0 = signal close (book default), 1 = next open (book's tested alternative)", 0.0, 0.0, 1.0, 1.0),
        ],
        DT1 => vec![
            p("lb_step", "Lookback step (days); lookbacks = step x 1..n", 21.0, 10.0, 42.0, 1.0),
            p("n_lb", "Number of lookbacks", 5.0, 1.0, 8.0, 1.0),
            p("w_per", "Allocation per positive lookback (% of capital)", 5.0, 1.0, 20.0, 1.0),
        ],
        AV1 => vec![
            p("yr_n", "Long-term total-return lookback (days)", 252.0, 126.0, 504.0, 21.0),
            p("mo_n", "Intermediate total-return lookback (days)", 21.0, 10.0, 63.0, 1.0),
            p("rsi_n", "RSI period", 2.0, 2.0, 4.0, 1.0),
            p("rsi_hi", "Short only if RSI above", 70.0, 50.0, 90.0, 5.0),
            p("limit_pct", "Sell limit % above the current price", 3.0, 1.5, 3.5, 0.5),
            p("rsi_exit", "Exit when RSI below", 15.0, 5.0, 30.0, 5.0),
            p("limit_base", "[Not from the book] Limit base: 0 = yesterday's close (default), 1 = today's open", 0.0, 0.0, 1.0, 1.0),
        ],
        TF => vec![p("lookback", "Total-return lookback (days)", 126.0, 63.0, 252.0, 21.0)],
        RA | WMR | DT | AV | PF => vec![],
        _ => return None,
    })
}

/// Calendar days since 1970-01-01 from "YYYY-MM-DD" (Hinnant's days_from_civil).
fn day_number(date: &str) -> i64 {
    let (y, m, d): (i64, i64, i64) = (date[0..4].parse().unwrap(), date[5..7].parse().unwrap(), date[8..10].parse().unwrap());
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468
}

/// Monday-based week id (1970-01-05 was a Monday).
fn week_id(date: &str) -> i64 {
    (day_number(date) + 3).div_euclid(7)
}

/// Last trading day of its week: the next bar is in another week; the last bar of the data only if it is a Friday.
fn week_end(x: &Ctx, i: usize) -> bool {
    match x.r.get(i + 1) {
        Some(n) => week_id(&n.date) != week_id(&x.r[i].date),
        None => (day_number(&x.r[i].date) + 3).rem_euclid(7) == 4,
    }
}

/// Last trading day of its month (the last bar of the data is not treated as a month end).
fn month_end(x: &Ctx, i: usize) -> bool {
    x.r.get(i + 1).map_or(false, |n| n.date[..7] != x.r[i].date[..7])
}

/// close / close n bars ago - 1 (NaN before n bars).
fn tret(c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|i| if i >= n { c[i] / c[i - n] - 1.0 } else { f64::NAN }).collect()
}

/// Wilder RSI(n) of the week-end closes, written on each week-end bar (NaN on other bars).
fn weekly_rsi(x: &Ctx, n: usize) -> Vec<f64> {
    let ends: Vec<usize> = (0..x.c.len()).filter(|&i| week_end(x, i)).collect();
    let wc: Vec<f64> = ends.iter().map(|&i| x.c[i]).collect();
    let wr = rsi(&wc, n);
    let mut out = vec![f64::NAN; x.c.len()];
    for (k, &i) in ends.iter().enumerate() {
        out[i] = wr[k];
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if schema.is_empty() {
        return Some(Vec::new()); // NeedsData: catalogued only
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let c = &x.c;
    Some(match id {
        RSI4 => {
            let rn = u("rsi_n").max(2);
            let rs = x.rsi_n(rn);
            let ma = x.sma_n(u("trend"));
            let (lo, hi, add_lt, add_tr) = (g("entry"), g("exit"), g("add"), g("add_trend") >= 0.5);
            let entry = |i: usize| (c[i] > ma[i] && rs[i] < lo).then_some(Entry::Close);
            let add = |i: usize, _: &Trade| rs[i] < add_lt && (!add_tr || c[i] > ma[i]);
            let exit = |i: usize, _: &Trade| (rs[i] > hi).then(|| Exit::Close(format!("RSI({rn}) {:.1} > {hi}", rs[i])));
            let mut t = simulate(x, &Spec { dir: 1.0, weights: &[0.5, 0.5], entry: &entry, add: &add, exit: &exit }, from, to);
            if g("add_entry_bar") >= 0.5 {
                // [Not from the book] the add also fires on the entry close; the shared simulator only adds on later bars.
                for tr in t.iter_mut() {
                    let i = tr.legs[0].i;
                    if add(i, &*tr) {
                        tr.legs.insert(1, Leg { i, px: c[i], w: 0.5 });
                        tr.legs.truncate(2);
                    }
                }
            }
            t
        }
        WMR1 => {
            let wr = weekly_rsi(x, u("rsi_n").max(2));
            let reg = tret(c, u("regime_n"));
            let (lo, hi, stop, nxt) = (g("entry"), g("exit"), g("stop_pct") / 100.0, g("exec") >= 0.5);
            // next-open entry = a limit at +inf for the next bar (fills at that open)
            let entry = |i: usize| (week_end(x, i) && reg[i] > 0.0 && wr[i] < lo).then(|| if nxt { Entry::Limit(f64::INFINITY) } else { Entry::Close });
            let exit = |i: usize, t: &Trade| {
                let why = if c[i] < t.legs[0].px * (1.0 - stop) {
                    format!("close {:.2} more than {}% below entry {:.2}", c[i], g("stop_pct"), t.legs[0].px)
                } else if week_end(x, i) && wr[i] > hi {
                    format!("weekly RSI {:.1} > {hi}", wr[i])
                } else {
                    return None;
                };
                Some(if nxt { Exit::NextOpen(why) } else { Exit::Close(why) })
            };
            simulate(x, &Spec { dir: 1.0, weights: &[1.0], entry: &entry, add: &no_add, exit: &exit }, from, to)
        }
        DT1 => {
            let (step, n) = (u("lb_step"), u("n_lb"));
            let rets: Vec<Vec<f64>> = (1..=n).map(|k| tret(c, k * step)).collect();
            let wper = g("w_per") / 100.0;
            let mut out: Vec<Trade> = Vec::new();
            let mut cur: Option<Trade> = None;
            for i in from..=to {
                if !week_end(x, i) || i < n * step {
                    continue;
                }
                if let Some(mut t) = cur.take() {
                    t.exit_i = i;
                    t.exit_px = c[i];
                    t.signal_i = i;
                    t.reason = "weekly rebalance".into();
                    out.push(t);
                }
                if i == to {
                    break;
                }
                let k = rets.iter().filter(|r| r[i] > 0.0).count();
                if k > 0 {
                    let w = (k as f64 * wper).min(1.0);
                    cur = Some(Trade { dir: 1.0, legs: vec![Leg { i, px: c[i], w }], exit_i: i, exit_px: c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: Vec::new() });
                }
            }
            if let Some(mut t) = cur {
                t.exit_i = to;
                t.exit_px = c[to];
                t.signal_i = to;
                t.reason = "open".into();
                t.open = true;
                out.push(t);
            }
            out
        }
        AV1 => {
            let rs = x.rsi_n(u("rsi_n").max(2));
            let (yr, mo) = (tret(c, u("yr_n")), tret(c, u("mo_n")));
            let (hi, lp, ex, on_open) = (g("rsi_hi"), g("limit_pct") / 100.0, g("rsi_exit"), g("limit_base") >= 0.5);
            let last = c.len() - 1;
            // conditions on yesterday's close (i); the limit order works during bar i+1
            let entry = |i: usize| {
                (yr[i] < 0.0 && mo[i] < 0.0 && rs[i] > hi).then(|| Entry::Limit(if on_open && i < last { x.o[i + 1] } else { c[i] } * (1.0 + lp)))
            };
            let exit = |i: usize, _: &Trade| {
                if rs[i] < ex {
                    Some(Exit::Close(format!("RSI {:.1} < {ex}", rs[i])))
                } else if mo[i] > 0.0 {
                    Some(Exit::Close(format!("{}-day return {:+.2}% > 0", u("mo_n"), mo[i] * 100.0)))
                } else {
                    None
                }
            };
            simulate(x, &Spec { dir: -1.0, weights: &[1.0], entry: &entry, add: &no_add, exit: &exit }, from, to)
        }
        TF => {
            let r = tret(c, u("lookback"));
            let entry = |i: usize| (month_end(x, i) && r[i] > 0.0).then_some(Entry::Close);
            let exit = |i: usize, _: &Trade| {
                (month_end(x, i) && !(r[i] > 0.0)).then(|| Exit::Close(format!("month-end {}-day return {:+.2}% not > 0", u("lookback"), r[i] * 100.0)))
            };
            simulate(x, &Spec { dir: 1.0, weights: &[1.0], entry: &entry, add: &no_add, exit: &exit }, from, to)
        }
        _ => return None,
    })
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        RSI4 => ("alpha_spy_rsi4", "SPY Mean Reversion RSI(4) (Alpha Formula Ch 4)", "Book rules"),
        RA => ("alpha_rising_assets", "Rising Assets (Alpha Formula)", "Full 12-ETF rotation"),
        WMR => ("alpha_weekly_mr", "CR Weekly Mean Reversion (Alpha Formula)", "Full (500 stocks)"),
        WMR1 => ("alpha_weekly_mr", "CR Weekly Mean Reversion (Alpha Formula)", "Single instrument"),
        DT => ("alpha_dyn_treasuries", "CR Dynamic Treasuries (Alpha Formula)", "Full (IEI/IEF/TLH/TLT)"),
        DT1 => ("alpha_dyn_treasuries", "CR Dynamic Treasuries (Alpha Formula)", "Single-ETF sleeve"),
        AV => ("alpha_etf_avalanches", "ETF Avalanches (Alpha Formula)", "Full (40 ETFs)"),
        AV1 => ("alpha_etf_avalanches", "ETF Avalanches (Alpha Formula)", "Single instrument"),
        PF => ("alpha_formula_portfolio", "The Alpha Formula Portfolio", "30/30/20/20 (and 1.5x)"),
        TF => ("alpha_trend_6m", "Six-month trend rule (Alpha Formula A1)", "Book rules"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    /// One bar per weekday from Monday 2024-01-01, O=H=L=C.
    fn rows(c: &[f64]) -> Vec<Row> {
        let mut out = Vec::new();
        let mut dn = day_number("2024-01-01");
        for &v in c {
            while (dn + 3).rem_euclid(7) >= 5 {
                dn += 1;
            }
            let z = dn + 719_468; // civil_from_days
            let era = z.div_euclid(146_097);
            let doe = z - era * 146_097;
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let d = doy - (153 * mp + 2) / 5 + 1;
            let m = if mp < 10 { mp + 3 } else { mp - 9 };
            let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
            out.push(Row { ticker: "T".into(), date: format!("{y:04}-{m:02}-{d:02}"), open: v, high: v, low: v, close: v, volume: None });
            dn += 1;
        }
        out
    }

    #[test]
    fn calendar_helpers() {
        let r = rows(&[1.0; 22]); // Mon 2024-01-01 .. Tue 2024-01-30
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!((r[4].date.as_str(), r[21].date.as_str()), ("2024-01-05", "2024-01-30"));
        assert!(week_end(&x, 4) && !week_end(&x, 3) && week_end(&x, 9));
        assert!(!week_end(&x, 21)); // last bar is a Tuesday
        assert!(!month_end(&x, 20) && !month_end(&x, 21));
    }

    /// Dynamic Treasuries sleeve: weight = 5% x positive lookbacks, one trade per week, back to back.
    #[test]
    fn treasury_sleeve_weights() {
        let c: Vec<f64> = (0..140).map(|i| 100.0 + i as f64).collect(); // rising: all 5 lookbacks positive
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(DT1, &x, 0, c.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty());
        assert!(t.iter().all(|t| (t.legs[0].w - 0.25).abs() < 1e-12 && week_end(&x, t.legs[0].i) && t.legs[0].i >= 105));
        for w in t.windows(2) {
            assert_eq!(w[0].exit_i, w[1].legs[0].i);
        }
    }

    /// ETF Avalanches single: downtrend, 2-day pop (RSI2 > 70), sell limit 3% above that close fills next day, RSI2 < 15 exit.
    #[test]
    fn avalanche_limit_fill_and_exit() {
        let mut c: Vec<f64> = (0..300).map(|i| 400.0 - i as f64).collect(); // 252- and 21-day returns < 0
        c.extend([103.0, 106.0, 104.0, 90.0]);
        let mut r = rows(&c);
        let sig = 301; // close 106: the pop
        (r[sig + 1].open, r[sig + 1].high, r[sig + 1].low) = (107.0, 110.0, 104.0);
        (r[sig + 2].open, r[sig + 2].high) = (95.0, 96.0);
        let x = Ctx::new(&r, &r, &[]);
        assert!(x.rsi2[sig] > 70.0 && x.rsi2[sig - 1] <= 70.0);
        let t = run(AV1, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (-1.0, sig + 1, sig + 2));
        assert!((t[0].legs[0].px - 106.0 * 1.03).abs() < 1e-9); // open 107 below the 109.18 limit, high 110 reaches it
        assert!(x.rsi2[sig + 2] < 15.0 && t[0].exit_px == 90.0);
    }
}
