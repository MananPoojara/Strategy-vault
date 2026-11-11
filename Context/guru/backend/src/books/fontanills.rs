//! George A. Fontanills (with Tom Gentile) - four works with complete daily rules. Book ids:
//!   `fgitc`  The Index Trading Course (Wiley 2006; printed page = pdf page - 21): Volex bands (stop and reverse), 20-day
//!            ROC / ROC-average with ADX filter, QQQQ rising-volume system, 10/20 EMA crossover with two-close exit.
//!   `fgitsm` Interactive Technical Trading Seminar manual (Optionetics; chapter-page cites): Reverse VolEx, volume spike.
//!   `fgsmc`  The Stock Market Course (Wiley 2001): 50/200-day moving-average crossover.
//!   `fgtofd` Trading Options For Dummies (Wiley 2008): 20/50-day EMA crossover (XLF example).
//! Reading: claude-continuation-20261020/authors/fontanills/agents/. Seasonal rules repeat kpss / hsta; rotation, VIX,
//! expiration and option rules need data we do not hold (authors/fontanills/fontanills-REPORT.md).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::mpts::sma;

pub const REVIEWED: &[&str] = &["fgitc_volex", "fgitc_rocadx", "fgitc_qqqvol", "fgitc_ema1020", "fgitsm_revvolex", "fgitsm_volspike", "fgsmc_ma50_200", "fgtofd_ema20_50", "fgsmc_vix" /* lane K75, R4 #279 (3) */];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! note { ($s:expr) => { concat!($s, " [Declared conventions, not from the book] A run starts flat; history before the window builds the indicators; an open trade is marked at the window's last close; one unit, no commissions. Index, ETF and stock runs (US and India) are a declared application where the book used another market; cash-index shorts are hypothetical.") }; }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let live = |id, name, group, book, dir, vol, rules, notes| StrategyDef { id, name, group, session: 0, dir, status: Live, needs_volume: vol,
        needs_vix: false, needs_series: &[], book, rules, notes, claim: None };
    let held = |id, name, group, book, st, rules| StrategyDef { id, name, group, session: 0, dir: "Long", status: st, needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/fontanills/agents).", claim: None };
    vec![
        live("fgitc_volex", "Volex bands: buy a break above the upper band, sell short below the lower, stop and reverse", "Ch.9 · Volex (p.162-169)", "fgitc", "Long/Short", false,
            &["F|\"Upper band value = Current day high + (ATR x ATR multiple); Lower band value = Current day low - (ATR x ATR multiple)\" (p.165); \"Volex indicator settings of nine days with an ATR multiple of 1.3\" (p.166); ATR = \"the sum of 14 TR values divided by 14\" style simple average (p.164)",
              "E|\"A break to the upside initiates a long position ... A break of the lower band initiates a short sale\" (p.166)",
              "X|\"In the traditional Volex system, trades are exited either long or short when the Volex System triggers another signal in the opposite direction\" (p.167)"],
            note!("Today's bands are built from yesterday's high, low and 9-day average true range (a band from today's own high can never be broken today); a break fills at the band, or at the open if the day opens beyond it. A day that breaks both bands is skipped. The book takes signals from the S&P 500 index and trades SPY; here signal and trade are the same instrument. Gentile's point-and-figure exit (p.168) is not run: the page gives two different exit rules and no box size.")),
        live("fgitc_rocadx", "20-day ROC crossing its 5-day average with ADX above 18, long only", "Ch.9 · Momentum (p.171-176, Table 9.3)", "fgitc", "Long", false,
            &["F|\"The ROC is calculated by dividing the latest price by the price from n periods ago\" (p.385)",
              "E|\"Buy when 20-day ROC crosses up above the 5-day SMA on the ROC. ADX must be above 18\" (Table 9.3, p.172)",
              "X|\"Sell when 20-day ROC crosses down below the 5-day SMA on the ROC. No ADX requirement\" (Table 9.3, p.172)"],
            note!("The base rule of Table 9.3. ADX length is not given: Wilder's 14-day ADX [declared]. Signals and fills at the close [book silent]. Equal ROC and average is no cross. The book tested S&P 100 stocks 1980-1998 with $50,000 accounts; the Table 9.7 and SPY versions (13/20, rising ADX, 13-15% stops) are not run.")),
        live("fgitc_qqqvol", "QQQQ volume system: buy an up close on two days of rising volume, exit at +3.5% or a rise on falling volume", "Ch.10 · Volume (p.187-188)", "fgitc", "Long", true,
            &["E|\"enter the QQQQ when it has moved higher in price and volume has been rising for two consecutive days - in other words, the volume is greater than yesterday, and yesterday's volume is higher than the day before. In addition, the close of the current day is greater than the close of the day before\" (p.187)",
              "X|\"We exit the trade when either (1) the QQQQ has moved 3.5 percent higher or (2) the QQQQ rises on falling volume. If the QQQQ moves higher but volume has declined for two consecutive days, the position is closed\" (p.187)"],
            note!("Entry at the signal close. From the next day a sell limit 3.5% above the entry fills when the high reaches it (at the open if the day opens above it); otherwise the trade closes at the close of a day that rises on two days of falling volume. No stop ('losses are allowed to run', p.187). One trade at a time; no entry on an exit day. Needs actual volume. Book: QQQQ, 119 trades, 64.7% winners (not claimed here).")),
        live("fgitc_ema1020", "10/20-day EMA crossover: buy next open, exit next open after two closes below the 10-day or a cross down, 15% stop", "Ch.14 · ETF trading (p.321-323)", "fgitc", "Long", false,
            &["E|\"When the 10-day exponential moving average (EMA) crosses up above the 20-day EMA, the trader buys the underlying ETF at the open on the next trading day\" (p.321)",
              "X|\"When the ETF price closes below the 10-day EMA for two consecutive days or the 10-day EMA crosses down below the 20-day EMA, the trader exits the position at the open on the following day\" (p.321)",
              "X|GTC stop 15% below the entry price: \"$30.60 [$36 - ($36 x 0.15)]\" (p.322)"],
            note!("Standard exponential averages, 2/(n+1), started at the first close of the data file [declared: the book's own EMA description on p.44 is a weighted average]. The stop is live from the entry day after the open fill and fills at the stop or a gapping open. A signal on the window's last bar is not filled. Book example: QQQQ bought 13 May 2005 at $36, sold 9 Jun 2005 at $37.66.")),
        live("fgitsm_revvolex", "Reverse VolEx: buy a close after the low breaks the lower band, sell short after the high breaks the upper band, 10% stop", "Ch.12 · Reverse VolEx (p.12-1 to 12-18)", "fgitsm", "Long/Short", false,
            &["F|\"Top Line Value = current intervals High + (ATR x ATR Multiple); Bottom Line Value = current interval's Low - (ATR x ATR Multiple) ... we will be using a 1.3 multiple\"; Volex (9, 1.300) (p.12-2, 12-3)",
              "E|\"Bullish VolEx Entry: Daily Low < VolEx Low; Bearish VolEx Entry: Daily High > VolEx High\"; \"Enter at Close of the signal day\" (p.12-4, 12-9)",
              "X|\"The Reverse VolEx System will use these very entry signals as exit signals ... always have a trade in play\"; \"If the trade moves against us by more than 10%, we will exit the trade\": \"Current Price <= Entry Price - (Entry Price * 10%)\" (p.12-4, 12-5)"],
            note!("The band is yesterday's (today's low can never be below a band built from today's own low). The 10% stop is checked on the close and exits at that close; the book prints the long formula and applies the stop to shorts too (p.12-12). A day that breaks both bands is skipped; after a stop exit there is no new entry the same day. Repeat signals in the same direction add positions in the book (p.12-4); this one-unit ledger ignores them. Book: DIA 2003-2005, 16 trades, 56.25% winners (not claimed).")),
        live("fgitsm_volspike", "Volume spike: buy a close above the spike day's high, exit on a break of its low (mirror short)", "Ch.10 · Volume spikes (p.10-1 to 10-24)", "fgitsm", "Long/Short", true,
            &["F|\"300%+ volume spike\" (p.10-1); scan: \"Volume of the current day Is Greater Than Moving Average of the current day * 3\", \"MA Periods ... (30 days)\" (p.10-18 to 10-20)",
              "E|\"Bullish entry: Close above high of day\"; \"Bearish entry: Close below low of day\" (p.10-1)",
              "X|\"Exit at break below low of day\" (long); \"Exit at break above high of day\" (short) (p.10-1)"],
            note!("'Day' = the spike day. The spike is volume above 3 x the 30-day average volume including that day. Entry at the close of the first later day that closes beyond the spike day's range; the exit is a stop at the spike day's low (high), filled at the stop or a gapping open, from the day after entry. A later spike replaces the levels while flat; spikes during a trade are ignored; levels are used up by an entry. The $9 price and 300,000-share filters of the scan are not applied. No profit exit in the book ('volume spikes really don't provide clear-cut targets', p.10-9). Needs actual volume.")),
        live("fgsmc_ma50_200", "50-day / 200-day moving-average crossover, long or cash", "Technical analysis (p.94, 256-257)", "fgsmc", "Long", false,
            &["E|\"My favorites are the 50-day and 200-day moving averages ... if it moves above, it's a buy signal\" (p.257)",
              "X|\"If the 50-day drops below the 200-day, it's a sell signal\" (p.257)"],
            note!("Simple averages ('most technicians seem to favor the simple moving average', p.256); signal and fill at the close [book silent]; long or cash (a sell signal is not said to open a short). Equal averages keep the position.")),
        live("fgtofd_ema20_50", "20/50-day EMA crossover, entry and exit at the next open (XLF example)", "Ch.5 · Technical analysis (p.100-101)", "fgtofd", "Long", false,
            &["E|\"enter a long position in XLF (financials ETF) the day after the 20-day EMA crosses up over the 50-day EMA\" (p.100)",
              "X|\"One exit signal includes a cross of the 20-day EMA down below the 50-day EMA\" (p.100); example exit \"at the next open\" (p.101)"],
            note!("Exponential averages 2/(n+1) started at the first close of the file. The book's loss exit is a support line drawn by eye ($36.58) and is not run. A signal on the window's last bar is not filled.")),
        live("fgitc_rotation", "Pure Momentum country ETF rotation: each month hold the five Table 11.8 country ETFs with the largest 240-day gain", "Ch.11 · Sector trading, Go international (p.249-252)", "fgitc", "Long", false,
            &["F|Universe: the 20 iShares country ETFs of Table 11.8 (EWA EWC EWD EWG EWH EWI EWJ EWK EWL EWM EWN EWO EWP EWQ EWS EWT EWU EWW EWY EWZ) (p.251)",
              "E|\"After the last trading day of each month, take the five country ETFs that have advanced the most over the last 240 trading days. Buy and hold an equal dollar amount of shares of those five funds until the end of the next month. Then repeat this process.\" (p.251-252)",
              "F|\"If fewer than five funds posted an advance over the past 240 trading days, then hold that portion of the portfolio in cash.\" (p.252)",
              "X|\"... until the end of the next month. Then repeat this process.\" (p.252): at the next month end every position is closed and the new five are bought"],
            "Jay Kaeppel's Pure Momentum system (S&C, July 1999) applied to country ETFs, as presented in The Index Trading Course. Book: Table 11.9 'Top 5 Monthly' yearly returns 4/1/97-5/12/04. [Declared readings, book silent] 240-day gain = month-end close / close 240 trading days earlier - 1; the ranking and the equal-fifth rebalance both use the month's last close (p.fill=1: buy at the next session's open), each fifth reset to equal dollars every month; ties by ticker; price-only closes (no dividends). The rotation runs once all 20 ETFs have 240 trading days of history (EWZ from July 2000, so from mid-2001): Kaeppel's 1997-2000 test necessarily had fewer funds, and a rotation with members missing would be a different rule (decision #229 (10)). Runs on the SPY calendar (rotation account, books::rotation). Lane K54."),
        held("fgitc_seasonal", "Presidential cycle, Kaeppel seasonal windows, Santa Claus rally", "Ch.10 (p.213-216)", "fgitc",
            NotTestable("Duplicate of kpss_monthly10 and the Stock Trader's Almanac (hsta) rules already on the site (Kaeppel's Seasonal Stock Market Trends); the book's turn-of-month window is the last 1 + first 4 days (kpss: last 2 + first 4)"), &["E|Buy the year before the election"]),
        held("fgitcwb_dia_expiry", "DIA option-expiration Friday short", "Workbook (p.118-119)", "fgitc",
            NotTestable("has no exit: a classroom exercise 'created for this exercise (but not implemented)' from one week of data; DIA prices are held (us_data.csv), so the block is the missing exit"), &["E|Short DIA at the close before expiration Friday after a down close"]),
        StrategyDef { needs_series: &["VXO_OHLC.Close"], // lane K75 (R4 #279 (3))
            ..live("fgsmc_vix", "VIX above 30 buy, below 20 out (the book's OEX VIX = VXO)", "Ch.10 · Sentiment analysis, CBOE Volatility Index (p.292-294)", "fgsmc", "Long", false,
            &["F|\"The CBOE Volatility Index (ticker = VIX) computes implied volatility of four S&P 100 (OEX) option contracts in the two front months\" (p.293): the index the CBOE renamed VXO in 2003 when the new S&P 500 VIX took the name; the vault holds VXO 1986-01-02..2021-09-23 (extra_data VXO_OHLC, CBOE), so p.src = 0 (default) reads the VXO close",
              "E|\"When the VIX is high (>30), it's time to buy.\" (p.293); \"Remember, 30+ is generally considered a good entry point!\" (p.294): flat and the VXO closes above 30 -> buy",
              "X|\"When the VIX is low (<20), it's time to go.\" (p.293): long and the VXO closes below 20 -> sell",
              "F|p.src = 1 (labelled option): the market's own volatility index instead of the book's VXO: today's CBOE VIX (S&P 500 options, 1990-) on US instruments, the India VIX on NSE instruments (declared application)"],
            note!("Long only (the book names no short). The VXO close (4:15 pm ET) comes after the stock market's 4:00 close, so a signal fills at the next session's open [declared]; while flat every close above 30 is a buy signal (a level, not a cross); 30 and 20 exactly give no signal. Runs where the VXO exists (to 2021-09-23, when the CBOE stopped it): US indices, ETFs and US stocks; India has no VXO (the India VIX is another index).")) },
        StrategyDef { claim: crate::connors::claim("20", "70%", "n/a", "weeks to months", "Select Sector SPDRs, 3 Jan 2000 - 21 Sep 2007, two bullish periods: 4 trades (3 gains) $5,000 -> $5,135, and 16 trades (11 gains) $5,000 -> $14,080 (Table 13-5, p.233-234)"),
            ..live("fgtofd_sector_tilt", "Select Sector SPDR tilt: hold the top 3-month ETF while SPY's 50-day SMA is above its 200-day", "Ch.13 · Tilting your portfolio with sector ETFs (p.230-234)", "fgtofd", "Long", false,
            &["F|\"Using SPY and the nine major Select Sector SPDR ETFs\" (Table 13-4: XLB XLE XLF XLI XLK XLP XLU XLV XLY) (p.231-232)",
              "E|\"1. On a weekly basis rank the ETFs using three-month returns. The top ranked ETF is the one with the best returns over a three-month period 2. Invest in the top ranked ETF.\" (p.232)",
              "X|\"3. Maintain investment in ETF until it drops below the third rank (ranks 4 - 9) for two consecutive weeks. 4. Identify new ETF for investment by repeating steps 1 through 3.\" (p.232)",
              "F|\"bullish periods are identified as those periods when the 50-day SMA is above the 200-day SMA ... as the 50-day SMA crosses below the 200-day SMA, a bearish period is identified ... the sector ETF position is exited\" (SPY daily chart, p.233); \"no sector investment is made during bearish periods\" (p.232)"],
            "Readings [book silent]: three-month return = close / close 63 trading days earlier - 1; the weekly ranking is taken at each week's last close and a switch fills there; the two weeks are counted from the purchase (two weekly rankings in a row at rank 4-9 while held); SMAs are simple averages of SPY's daily closes, checked every day: a close with the 50-day below the 200-day sells at that close, and a close with the 50-day above the 200-day while flat buys the top-ranked ETF at that close (the book's tilt starts on its first data day, 3 Jan 2000, already bullish); ties by ticker. Price-only ETF closes; the book's $5,000 tilt sits next to a protected SPY core that is not part of this card. Runs on the SPY calendar (SPY is also the bullish/bearish filter); the holdings are the ETFs named in each trade. US only (no NIFTY sector index series, decision #11).") },
        held("fgitsm_elliott", "ProfitSource Elliott-wave trades", "Seminar manual", "fgitsm", NotTestable("depends on proprietary Elliott-wave counts"), &["E|Proprietary"]),
        held("fg_options", "Option strategies (straddles, spreads, covered calls ...)", "Options Course, Trading Options For Dummies, seminar manuals", "fgtofd",
            NotTestable("rules not fully defined: the positions (calls, puts, spreads, straddles, covered calls) are chosen to a market view; entry and exit rules are not specified (The Index Trading Course Ch.4-7 and 13, p.47-132 and 287-308; the For Dummies trading guide; The Stock Market Course Ch.5 p.117-157); real SPY/QQQ/IWM and NIFTY chains are now held, but there is no rule to run"), &["E|Option positions"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "fgitc_volex" => ("fgitc_volex", "Fontanills Volex", "9, 1.3"),
        "fgitc_rocadx" => ("fgitc_rocadx", "Fontanills ROC/ADX", "20/5, ADX 18"),
        "fgitc_qqqvol" => ("fgitc_qqqvol", "Fontanills volume system", "3.5%"),
        "fgitc_ema1020" => ("fgitc_ema1020", "Fontanills 10/20 EMA", "10/20, 15%"),
        "fgitsm_revvolex" => ("fgitsm_revvolex", "Reverse VolEx", "9, 1.3, 10%"),
        "fgitsm_volspike" => ("fgitsm_volspike", "Volume spike", "3x 30-day"),
        "fgsmc_ma50_200" => ("fgsmc_ma50_200", "Fontanills 50/200", "50/200"),
        "fgtofd_ema20_50" => ("fgtofd_ema20_50", "Fontanills 20/50 EMA", "20/50"),
        "fgtofd_sector_tilt" => ("fgtofd_sector_tilt", "Fontanills sector SPDR tilt", "3-month rank, SPY 50/200"),
        "fgitc_rotation" => ("fgitc_rotation", "Fontanills Pure Momentum country rotation", "top 5 of 20, 240 days"),
        _ if id.starts_with("fgitc_") || id.starts_with("fgitcwb_") || id.starts_with("fgitsm_") || id.starts_with("fgsmc_") || id.starts_with("fgtofd_") || id.starts_with("fg_") => ("fg_nt", "Fontanills (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "fgitc_volex" | "fgitsm_revvolex" => vec![p("n", "ATR days (book: 9)", 9., 2., 50., 1.), p("m", "ATR multiple (book: 1.3)", 1.3, 0.1, 5., 0.1)],
        "fgitc_rocadx" => vec![p("roc", "ROC days (book: 20)", 20., 2., 100., 1.), p("ma", "ROC average days (book: 5)", 5., 1., 50., 1.), p("adx", "ADX above (book: 18)", 18., 0., 60., 1.)],
        "fgitc_qqqvol" => vec![p("target", "Profit target % (book: 3.5)", 3.5, 0.5, 20., 0.5)],
        "fgitc_ema1020" => vec![p("s", "Short EMA (book: 10)", 10., 2., 100., 1.), p("l", "Long EMA (book: 20)", 20., 3., 250., 1.), p("stop", "Stop % (book: 15)", 15., 1., 50., 1.)],
        "fgitsm_volspike" => vec![p("mult", "Spike = volume above x times average (book: 3)", 3., 1.5, 10., 0.5), p("n", "Average volume days (book: 30)", 30., 5., 200., 1.)],
        "fgsmc_vix" => vec![p("buy", "Buy above (book: 30)", 30., 10., 80., 1.), p("sell", "Sell below (book: 20)", 20., 5., 60., 1.), p("src", "0 = VXO, the book's OEX index (default); 1 = the market's volatility index (US VIX, India VIX)", 0., 0., 1., 1.)],
        "fgsmc_ma50_200" => vec![p("s", "Short average (book: 50)", 50., 2., 200., 1.), p("l", "Long average (book: 200)", 200., 3., 400., 1.)],
        "fgtofd_ema20_50" => vec![p("s", "Short EMA (book: 20)", 20., 2., 100., 1.), p("l", "Long EMA (book: 50)", 50., 3., 250., 1.)],
        "fgtofd_sector_tilt" => vec![], // lane K28: runs in books::rotation
        "fgitc_rotation" => vec![p("fill", "Rebalance fill: 0 = the month's last close (default), 1 = the next session's open", 0., 0., 1., 1.)], // lane K54: books::rotation
        _ => return None,
    })
}

pub fn ema(c: &[f64], n: usize) -> Vec<f64> {
    let k = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; c.len()];
    for i in 0..c.len() { out[i] = if i == 0 { c[0] } else { k * c[i] + (1. - k) * out[i - 1] }; }
    out
}

/// Simple n-day average of the daily true range (bar 0: high - low).
fn atr(x: &Ctx, n: usize) -> Vec<f64> {
    let tr: Vec<f64> = (0..x.c.len()).map(|i| if i == 0 { x.h[0] - x.l[0] } else {
        (x.h[i] - x.l[i]).max((x.h[i] - x.c[i - 1]).abs()).max((x.l[i] - x.c[i - 1]).abs()) }).collect();
    sma(&tr, n)
}

/// +1 / -1 by a > b / a < b, kept on ties; None until the first difference.
fn state(a: &[f64], b: &[f64]) -> Vec<Option<i8>> {
    let mut cur = None;
    a.iter().zip(b).map(|(&u, &v)| { if u.is_finite() && v.is_finite() && u != v { cur = Some(if u > v { 1 } else { -1 }); } cur }).collect()
}

fn trade(d: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir: d, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn close(mut t: Trade, i: usize, px: f64, sig: usize, why: &str, out: &mut Vec<Trade>) { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); out.push(t); }

fn vol(x: &Ctx) -> Vec<f64> { x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect() }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let ni = |k: &str, d: f64| g(k, d).round().max(1.) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let start = from.max(1);
    match id {
        "fgitc_volex" | "fgitsm_revvolex" => {
            let (a, m) = (atr(x, ni("n", 9.)), g("m", 1.3));
            let rev = id == "fgitsm_revvolex";
            for i in start..=to {
                if !a[i - 1].is_finite() { continue; }
                let (ub, lb) = (h[i - 1] + m * a[i - 1], l[i - 1] - m * a[i - 1]);
                let (up, dn) = (h[i] > ub, l[i] < lb);
                if !rev {
                    if up == dn { continue; }
                    let (d, px) = if up { (1., o[i].max(ub)) } else { (-1., o[i].min(lb)) };
                    if cur.as_ref().is_some_and(|t| t.dir == d) { continue; }
                    if let Some(t) = cur.take() { close(t, i, px, i, "opposite band broken", &mut out); }
                    cur = Some(trade(d, i, px, "stop", i));
                } else {
                    if let Some(t) = cur.take() {
                        let e = t.legs[0].px;
                        if (t.dir > 0. && c[i] <= e * 0.9) || (t.dir < 0. && c[i] >= e * 1.1) { close(t, i, c[i], i, "10% stop", &mut out); continue; }
                        cur = Some(t);
                    }
                    if up == dn { continue; }
                    let d = if dn { 1. } else { -1. };
                    if cur.as_ref().is_some_and(|t| t.dir == d) { continue; }
                    if let Some(t) = cur.take() { close(t, i, c[i], i, "opposite VolEx signal", &mut out); }
                    cur = Some(trade(d, i, c[i], "close", i));
                }
            }
        }
        "fgitc_rocadx" => {
            let n = ni("roc", 20.);
            let roc: Vec<f64> = (0..c.len()).map(|i| if i >= n { c[i] / c[i - n] } else { f64::NAN }).collect();
            let st = state(&roc, &sma(&roc, ni("ma", 5.)));
            let (_, adx) = crate::connors::dmi(x.r, 14);
            let lvl = g("adx", 18.);
            for i in start..=to {
                if !matches!((st[i], st[i - 1]), (Some(a), Some(b)) if a != b) { continue; }
                if st[i] == Some(1) { if cur.is_none() && adx[i] > lvl { cur = Some(trade(1., i, c[i], "close", i)); } }
                else if let Some(t) = cur.take() { close(t, i, c[i], i, "ROC crossed below its average", &mut out); }
            }
        }
        "fgitc_qqqvol" => {
            let v = vol(x);
            let tg = g("target", 3.5) / 100.;
            for i in start.max(2)..=to {
                if let Some(t) = cur.take() {
                    let tp = t.legs[0].px * (1. + tg);
                    if h[i] >= tp { close(t, i, o[i].max(tp), i, "3.5% target", &mut out); continue; }
                    if c[i] > c[i - 1] && v[i] < v[i - 1] && v[i - 1] < v[i - 2] { close(t, i, c[i], i, "rise on falling volume", &mut out); continue; }
                    cur = Some(t); continue;
                }
                if c[i] > c[i - 1] && v[i] > v[i - 1] && v[i - 1] > v[i - 2] { cur = Some(trade(1., i, c[i], "close", i)); }
            }
        }
        "fgitc_ema1020" => {
            let (es, el) = (ema(c, ni("s", 10.)), ema(c, ni("l", 20.)));
            let st = state(&es, &el);
            let stop_f = 1. - g("stop", 15.) / 100.;
            let mut pend_entry: Option<usize> = None; // signal bar
            let mut pend_exit: Option<usize> = None;
            for i in start..=to {
                if let (Some(s), true) = (pend_exit.take(), cur.is_some()) { close(cur.take().unwrap(), i, o[i], s, "next open after the exit signal", &mut out); }
                if let Some(s) = pend_entry.take() { cur = Some(trade(1., i, o[i], "open", s)); }
                if let Some(t) = cur.take() {
                    let sp = t.legs[0].px * stop_f;
                    if l[i] <= sp { close(t, i, o[i].min(sp), i, "15% stop", &mut out); }
                    else {
                        let down = matches!((st[i], st[i - 1]), (Some(-1), Some(1)));
                        if (c[i] < es[i] && c[i - 1] < es[i - 1]) || down { if i < to { pend_exit = Some(i); } }
                        cur = Some(t);
                    }
                    continue;
                }
                if matches!((st[i], st[i - 1]), (Some(1), Some(-1))) && i < to { pend_entry = Some(i); }
            }
        }
        "fgitsm_volspike" => {
            let v = vol(x);
            let (mult, av) = (g("mult", 3.), sma(&v, ni("n", 30.)));
            let mut lv: Option<(f64, f64)> = None;
            for i in start..=to {
                if let Some(t) = cur.take() {
                    let (sh, sl) = lv.unwrap();
                    if t.dir > 0. && l[i] < sl { close(t, i, o[i].min(sl), i, "spike-day low broken", &mut out); lv = None; }
                    else if t.dir < 0. && h[i] > sh { close(t, i, o[i].max(sh), i, "spike-day high broken", &mut out); lv = None; }
                    else { cur = Some(t); }
                    continue;
                }
                if let Some((sh, sl)) = lv {
                    if c[i] > sh { cur = Some(trade(1., i, c[i], "close", i)); continue; }
                    if c[i] < sl { cur = Some(trade(-1., i, c[i], "close", i)); continue; }
                }
                if av[i].is_finite() && v[i] > mult * av[i] { lv = Some((h[i], l[i])); }
            }
        }
        "fgsmc_ma50_200" | "fgtofd_ema20_50" => {
            let st = if id == "fgsmc_ma50_200" { state(&sma(c, ni("s", 50.)), &sma(c, ni("l", 200.))) } else { state(&ema(c, ni("s", 20.)), &ema(c, ni("l", 50.))) };
            let nx = id == "fgtofd_ema20_50";
            for i in start..=to {
                if !matches!((st[i], st[i - 1]), (Some(a), Some(b)) if a != b) { continue; }
                if nx && i == to { continue; }
                let (fi, px, kind) = if nx { (i + 1, o[i + 1], "open") } else { (i, c[i], "close") };
                if st[i] == Some(1) { cur = Some(trade(1., fi, px, kind, i)); }
                else if let Some(t) = cur.take() { close(t, fi, px, i, "short average crossed below the long", &mut out); }
            }
        }
        "fgsmc_vix" => { // lane K75 (R4 #279 (3))
            let Some(v) = x.series(if g("src", 0.) >= 0.5 { "VIX.Close" } else { "VXO_OHLC.Close" }) else { return Some(vec![]) };
            let (buy, sell) = (g("buy", 30.), g("sell", 20.));
            for i in from..to {
                if cur.is_none() && v[i] > buy { cur = Some(trade(1., i + 1, o[i + 1], "open", i)); }
                else if cur.is_some() && v[i] < sell { close(cur.take().unwrap(), i + 1, o[i + 1], i, "VIX closed below 20 · next open", &mut out); }
            }
        }
        _ => return None,
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogue_admits_exactly_the_reviewed_ids() {
        let live: Vec<_> = catalogue().into_iter().filter(|d| d.status == Status::Live && !crate::books::rotation::is_rotation(d.id)).map(|d| d.id).collect();
        assert_eq!(live, REVIEWED);
    }
    #[test]
    fn vix_30_buy_20_out_next_open() {
        // lane K75: VXO closes 25, 31 (buy day 2 open), 35 (held), 19 (sell day 4 open), 32 on the window's last bar (no fill)
        let r: Vec<crate::Row> = (0..6).map(|i| crate::Row { ticker: "SPY".into(), date: format!("2020-01-{:02}", i + 1), open: 100. + i as f64, high: 110., low: 90., close: 101. + i as f64, volume: None }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert("VXO_OHLC.Close".into(), vec![25., 31., 35., 19., 22., 32.]);
        let t = run("fgsmc_vix", &x, 0, 5, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px, z.open)).collect::<Vec<_>>(), vec![(2, 102., 4, 104., false)]);
        x.extra.insert("VIX.Close".into(), vec![31., 25., 25., 25., 25., 25.]);
        let v = run("fgsmc_vix", &x, 0, 5, &[("src".to_string(), 1.)].into_iter().collect()).unwrap();
        assert_eq!((v.len(), v[0].legs[0].i, v[0].open), (1, 1, true));
    }
}

// Instrument admission: Fontanills 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/fontanills/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"fgitc"|"fgitsm"|"fgsmc"|"fgtofd") && (crate::books::fontanills::reviewed(def.id)),
    label: "Fontanills and Gentile show these rules on SPY, the S&P 500, QQQQ, DIA, XLF and US stocks; other index, ETF and stock runs (US and India) are a declared application, not the authors' test.",
    basis: "George A. Fontanills and Tom Gentile: The Index Trading Course (2006) Volex bands, ROC/ADX momentum, QQQQ volume and 10/20 EMA systems (p.162-188, 321-323); Interactive Technical Trading Seminar manual Reverse VolEx and volume-spike systems (Ch.10, 12); The Stock Market Course 50/200 crossover (p.257); Trading Options For Dummies 20/50 EMA crossover (p.100-101); reviewed in authors/fontanills/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Band-day, fill, ADX length and EMA start are declared readings; one unit, no pyramiding. Cash-index shorts are hypothetical exposure.",
};
