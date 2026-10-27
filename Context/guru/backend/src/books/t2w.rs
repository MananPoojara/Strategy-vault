//! Trade-to-Win / TechnicalTradingIndicators.com (tradingsystemsthatwork.com) system manuals, 2007-2009:
//! MRX Swing Trading System, R90 Swing Trading System v2, The Crouching Tiger System (2008),
//! The Oops 1-2-3 Trading Systems, Swing Traders Edge (STE), The ReVersal Twelve (RVT), The PR1 Portfolio Trading System.
//! Source, page refs, verification: library/t2w/README.md. Own simulators below (no jkbwts::sim).
//! Conventions: every rule is evaluated on the bar's close; "at the open of the next trading day" fills at open[i+1];
//! a TradeStation trade = one unit, so a two-unit position is one Trade with two legs of weight 0.5.
use crate::connors::{claim, rsi, sma, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "t2w";
/// Admitted ids (authors/vendor/laneb/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["t2w_cts", "t2w_cts_plus", "t2w_cts_stock", "t2w_mrx", "t2w_mrx_plus", "t2w_mrx_stock", "t2w_oops_11", "t2w_oops_12", "t2w_oops_13", "t2w_oops_14", "t2w_oops_21", "t2w_oops_22", "t2w_oops_23", "t2w_oops_31", "t2w_oops_32", "t2w_oops_33", "t2w_oops_34", "t2w_oops_i1", "t2w_oops_i2", "t2w_oops_i3"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ------------------------------------------------------------------ indicators
/// TradeStation XAverage: alpha = 2/(n+1), seeded with the first price.
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = Vec::with_capacity(v.len());
    for (i, &x) in v.iter().enumerate() {
        out.push(if i == 0 { x } else { out[i - 1] + a * (x - out[i - 1]) });
    }
    out
}
/// Wilder average up / down moves behind RSI(n) (same seeding as connors::rsi).
fn rsi_avgs(c: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut up = vec![f64::NAN; c.len()];
    let mut dn = vec![f64::NAN; c.len()];
    for i in 1..c.len() {
        up[i] = (c[i] - c[i - 1]).max(0.0);
        dn[i] = (c[i - 1] - c[i]).max(0.0);
    }
    (wilder(&up, n), wilder(&dn, n))
}
fn adx(x: &Ctx, n: usize) -> Vec<f64> {
    if n == 10 { x.adx10.clone() } else { crate::connors::dmi(x.r, n).1 }
}
/// SMA(n) of close x volume converted to US dollars with the USDINR series (INR per USD; 1.0 for USD-listed data).
fn money_flow_usd(x: &Ctx, n: usize) -> Option<Vec<f64>> {
    let fx = x.series("USDINR")?;
    let v: Vec<f64> = x.r.iter().enumerate().map(|(i, r)| r.volume.map_or(f64::NAN, |v| v * r.close / fx[i])).collect();
    Some(sma(&v, n))
}
/// Price at which tomorrow's RSI(n) equals `target` (close below today's), by inverting Wilder's RSI.
fn rsi_price_below(c: f64, au: f64, ad: f64, n: usize, target: f64) -> f64 {
    c - (n as f64 - 1.0) * (au * (100.0 - target) / target - ad)
}
/// Predictive RSI 50 band for bar i: the price where RSI(n) on bar i would read exactly 50 (uses bar i-1's averages).
fn rsi50_band(c_prev: f64, au: f64, ad: f64, n: usize) -> f64 {
    c_prev + (n as f64 - 1.0) * (ad - au)
}

// ------------------------------------------------------------------ simulators
fn new_trade(dir: f64, i: usize, px: f64, w: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn close_open(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(mut t) = cur {
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t);
    }
}

/// Market-order system: entry(i) = +1/-1 on bar i's close -> next open; add(i, dir, signal_bar) -> second unit next open;
/// exit(i, dir) -> all units out next open. Exit is checked before the add; no new entry is evaluated while a position or an
/// exit order is pending. `units` = 1 or 2.
fn next_open_sim(x: &Ctx, from: usize, to: usize, units: usize, entry: &dyn Fn(usize) -> i8, add: &dyn Fn(usize, f64, usize) -> bool,
                 exit: &dyn Fn(usize, f64) -> bool, why: &str) -> Vec<Trade> {
    let w = 1.0 / units as f64;
    let (mut out, mut cur, mut sig) = (vec![], None::<Trade>, 0usize);
    let mut pend: Option<(u8, f64)> = None; // 0 enter(dir), 1 add, 2 exit
    for i in from..=to {
        if let Some((k, d)) = pend.take() {
            match k {
                0 => cur = Some(new_trade(d, i, x.o[i], w, "open", sig)),
                1 => cur.as_mut().unwrap().legs.push(Leg { i, px: x.o[i], w }),
                _ => { let mut t = cur.take().unwrap(); t.exit_i = i; t.exit_px = x.o[i]; out.push(t); }
            }
        }
        if i == to { break; }
        if let Some(t) = cur.as_mut() {
            if exit(i, t.dir) {
                t.reason = why.into(); t.signal_i = i; pend = Some((2, 0.0));
            } else if t.legs.len() < units && add(i, t.dir, sig) {
                pend = Some((1, 0.0));
            }
        } else {
            let d = entry(i);
            if d != 0 { sig = i; pend = Some((0, d as f64)); }
        }
    }
    close_open(&mut out, cur, x, to);
    out
}

// ------------------------------------------------------------------ catalogue
const MRXN: &str = "Manual: 'MRX Swing Trading System' (technicaltradingindicators.com). TradeStation counts each unit as one trade; here a two-unit position is one trade with two half-weight legs. 5XMA/200XMA = TradeStation XAverage (seeded with the first close). Rule 2 is read on the bar 1 (2, 4) bars ago with that bar's XMA; rule 4 'at least 1 bar has passed' counts from the signal bar, as the p.4 example does (second entry signal on the close of the first entry bar) - this reading reproduces the published counts (SPX 386 vs 391 units); counting from the fill bar gives 330. While a position is open, opposite signals are ignored (book silent). Indices: run on NIFTY 50 / SPX instead of the S&P 500 / NASDAQ 100 cash index; stocks: NSE basket tickers (basket:<list>:<SYMBOL>).";
const R90N: &str = "Manual: 'R90 Swing Trading System, Version 2'. Conflict in the manual: the rule text says 'RSI200 must be greater than 50' while the indicator list ('2. 100 period RSI'), the trade example ('the 100 period [RSI] is above 50') and the strategy input default 'TrendPrice = rsi(close,100)' say 100; the input table also lists RSIlength = 3 while every rule says RSI2. Default = RSI(100) and RSI(2) - this reproduces the published SPX report (134 vs 126 units, 87.3% vs 88.1%); RSI(200) gives 157 units, RSI(3) gives 9. p.trend_len=200 is the rule-text reading (labelled, non-default). % change = ((R3-R)/R3)x100 for longs, on 100-RSI for shorts (manual formula). Wilder RSI.";
const CTSN: &str = "Manual: 'The Crouching Tiger System' (tradingsystemsthatwork.com, 2008). 'Less than 40 for the last 3 days' = RSI(2) < 40 on today's close and the 2 closes before. Second position: 'at least one day has passed and the first two rules are still valid' - counted from the signal bar (the S&P example takes the second unit on the close of the 4th day below 40, i.e. the first entry day); reproduces SPX 235 vs 232 units. At most 2 units (example: no third). Wilder RSI(2), simple 200-day MA.";
const OOPSN: &str = "Manual: 'The Oops 1-2-3 Trading Systems' (scanned, read visually). Rules 1-3 are evaluated on the prior close, the gap on today's open; the buy (sell short) stop at the prior low (high) fills at that price when the bar trades back to it (the open is beyond it by the gap rule); valid that day only. 'Gaps more than 1%' = open below prior low x (1 - 1%) (input PcntGap p.27: 'more than 1% below the previous low'). Exit: close >= 5MA (<= for shorts) -> next open; can fire on the entry bar's close. Index systems were tested on the S&P 500 E-mini (@ES.D, 09/11/97-05/01/09, 1 contract): here the cash index (instrument swap; cash-index gaps differ from ES day-session gaps). Equity systems: S&P 500 members 01/01/95-05/01/09, $10,000 per trade, compiled over all 500 stocks: here per NSE basket ticker.";
const STEN: &str = "Manual: 'Swing Traders Edge - System Manual'. Limit 3% below the close for the next day only (ATI example: 60.79 -> 58.96). Exit can trigger on the fill day's close. HV(100) = sample stdev of ln returns x sqrt(252) x 100 (book gives no formula; the examples' table header says '20 Day HV' while rule 6 says 100 - rule text kept). Rule 7 is in US dollars: on NSE tickers the money flow is converted with series USDINR (INR per USD, needed; not in the vault yet -> data gap). Book results are a 10-slot portfolio over a US stock universe ranked by 4-day % drop (t2w_ste_portfolio) - not reproducible per ticker.";
const RVTN: &str = "Manual: 'The ReVersal Twelve (RVT) Trading System'. Limit 8% below the close, placed before the next open (next day only). Rule 5 threshold 5,000,000 is dollars ('average daily money flow of at least $5,000,000', p.3) -> USDINR conversion on NSE (series needed). Wilder RSI(3). No stop (FAQ: 'stops harm the overall performance'). Book prints no statistics.";
const PR1N: &str = "Manual: 'The PR1 Portfolio Trading System' (scanned). Predictive RSI 50 band = the price at which today's RSI(2) would read 50, from yesterday's Wilder averages: C[1] + (n-1)(AvgDown[1]-AvgUp[1]); limit = the price at which tomorrow's RSI(2) is half today's: C - (n-1)(AvgUp(100-R)/R - AvgDown) [the manual names the indicator but prints no formula; inversion of Wilder RSI, TradeStation's RSI]. 'Drop by half (50%)' read as RSI(2) <= 0.5 x yesterday (input RSIDrop 0.5 = 'minimum amount RSI must fall'). Liquidity $4,000,000 -> USDINR series on NSE. Book results are a 10-slot portfolio (t2w_pr1_portfolio).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    let usd = |def: StrategyDef| StrategyDef { needs_volume: true, needs_series: &["USDINR"], ..def };
    let mut v = vec![
        d("t2w_mrx", "MRX Standard (S&P 500 rules)", "MRX Swing Trading System", "Long/Short",
          &["F|\"1. The close must be greater than the 200XMA\" (p.3; short: less than)",
            "E|\"2. The close 1 bar ago must be below the 5XMA\" (short: above)",
            "E|\"3. The close must close lower for at least 1 consecutive bar\" (short: higher) -> \"enter our first long position at the open on the next trading day\"",
            "A|\"4. At least 1 bar has passed since the first long position was entered. 5. The close must close lower for at least 2 consecutive bars\" -> second position at the next open",
            "X|\"6. The close is greater than or equal to the 5XMA\" (short: less than or equal) -> \"all long positions should be exited at the open of the next trading day\""],
          MRXN, claim("391 (256 L / 135 S)", "80.05%", "7.90 index pts", "3.59 / 8.00 bars (win/loss)", "S&P 500 index 01/01/95-05/01/09 (p.5)")),
        d("t2w_mrx_plus", "MRX+ (extended pullback)", "MRX Swing Trading System", "Long/Short",
          &["F|\"1. The close must be greater than the 200XMA\" (p.7)", "E|\"2. The close 2 bars ago must be below the 5XMA\"",
            "E|\"3. The close must close lower for at least 2 consecutive bars\" -> first position at the next open",
            "A|\"4. At least 1 bar has passed ... 5. The close must close lower for at least 3 consecutive bars\" -> second position at the next open",
            "X|\"6. The close is greater than or equal to the 5XMA\" -> all positions out at the next open (shorts mirrored)"],
          MRXN, claim("178 (118 L / 60 S)", "87.08%", "12.31 index pts", "3.61 / 8.43 bars", "S&P 500 index 01/01/95-05/01/09 (p.8)")),
        d("t2w_mrx_stock", "MRX for stocks", "MRX Swing Trading System", "Long/Short",
          &["F|\"1. The close must be greater than the 200XMA\" (p.10)", "E|\"2. The close 4 bars ago must be below the 10XMA\"",
            "E|\"3. The close must close lower for at least 4 consecutive bars\" -> first position at the next open",
            "A|\"4. At least 2 bars have passed since the first long position was entered. 5. ... lower for at least 6 consecutive bars\" -> second position at the next open",
            "X|\"6. The close is greater than or equal to the 5XMA\" -> all positions out at the next open (shorts mirrored)"],
          MRXN, None),
        d("t2w_r90", "R90 Standard (index rules)", "R90 Swing Trading System", "Long/Short",
          &["F|\"1. The RSI200 must be greater than 50\" (p.3; manual's indicator list, example and input default: RSI(100) - default)",
            "E|\"2. The RSI2 must be at least 90% lower than it was 3 days ago\" -> first long at the next open (short: 90% higher on 100-RSI)",
            "A|\"3. At least 1 bar has passed since the first long position was entered. 4. The RSI2 must be at least 50% lower than the previous day\" -> second long at the next open",
            "X|\"5. The RSI2 must be greater than or equal to 70\" -> all longs out at the next open (short: <= 30)"],
          R90N, claim("126 (101 L / 25 S)", "88.10%", "12.98 index pts", "4.63 / 8.87 bars", "S&P 500 index 01/01/95-05/01/09 (p.6)")),
        d("t2w_r90_stock", "R90 for stocks", "R90 Swing Trading System", "Long/Short",
          &["F|\"1. The RSI200 must be greater than 50\" (p.8; default RSI(100) as in the index rules)",
            "E|\"2. The RSI2 must be at least 95% lower than it was 3 days ago\" -> first long at the next open",
            "A|\"3. At least 2 bars have passed ... 4. The RSI2 must be at least 60% lower than the previous day\" -> second long at the next open",
            "X|\"5. The RSI2 must be greater than or equal to 70\" -> next open (short: 95% / 60% higher, exit <= 30)"],
          R90N, None),
        d("t2w_cts", "Crouching Tiger Standard", "Crouching Tiger System", "Long/Short",
          &["F|\"1. The closing price must be greater than the 200 day simple moving average\" (p.4; short: less than)",
            "E|\"2. The 2 period RSI of the closing price must be less than 40 for the last 3 days\" -> long at the market open the next day (short: greater than 60)",
            "A|\"3. If at least one day has passed and the first two rules are still valid then enter a second long position at the market open the next day\"",
            "X|\"4. The 2 period RSI ... must be greater than 70\" -> exit at the market open the next day (short: less than 30)"],
          CTSN, claim("232 (161 L / 71 S)", "79.31%", "8.63 index pts", "4.09 / 8.98 bars", "S&P 500 index 01/01/95-01/01/08 (p.7)")),
        d("t2w_cts_plus", "Crouching Tiger +", "Crouching Tiger System", "Long/Short",
          &["F|Close > 200-day SMA (p.9; short: <)", "E|\"less than 40 for the last 4 days\" -> next open (short: > 60)",
            "A|Second position: at least one day passed and rules 1-2 still valid -> next open", "X|RSI(2) > 70 -> next open (short: < 30)"],
          CTSN, claim("124 (78 L / 46 S)", "76.61%", "8.99 index pts", "3.75 / 8.97 bars", "S&P 500 index 01/01/95-01/01/08 (p.10)")),
        d("t2w_cts_stock", "Crouching Tiger for stocks", "Crouching Tiger System", "Long/Short",
          &["F|Close > 200-day SMA (p.12; short: <)", "E|\"less than 40 for the last 5 days\" -> next open (short: > 60)",
            "A|Second position: at least one day passed and rules 1-2 still valid -> next open", "X|RSI(2) > 70 -> next open (short: < 30)"],
          CTSN, None),
        d("t2w_oops_i1", "Oops Index 1 (gap + stop at prior extreme)", "Oops 1-2-3 · Index", "Long/Short",
          &["F|\"1. The close must be greater than the 200MA. 2. The close must be below the 5MA. 3. The RSI2 must be below 30\" (p.4; short: < 200MA, > 5MA, RSI2 > 70)",
            "E|\"4. The market gaps lower below the low of the previous day\" -> \"buy stop order to enter a long position at the low of the previous bar\" (short: gap above the prior high, sell short stop at the prior high)",
            "X|\"5. The close is greater than or equal to the 5MA\" -> exit at the open of the next trading day (short: <=)"],
          OOPSN, claim("88 (37 L / 51 S)", "72.73%", "$427.56 (ES, 1 contract)", "3.89 / 7.48 bars", "S&P 500 E-mini 09/11/97-05/01/09 (p.6)")),
        d("t2w_oops_i2", "Oops Index 2 (extreme day, market order)", "Oops 1-2-3 · Index", "Long/Short",
          &["F|\"1. The close must be greater than the 200MA. 2. The close must be below the 5MA\" (p.7)",
            "E|\"3. The RSI2 must be below 30 one bar before the extreme price move. 4. The market must close more than 1% lower than the close of the previous day\" -> \"market order to buy at the open of the next day\" (short: RSI2 > 70, close > 1% higher)",
            "X|\"5. The close is greater than or equal to the 5MA\" -> next open (short: <=)"],
          OOPSN, claim("68 (29 L / 39 S)", "72.06%", "$570.77 (ES, 1 contract)", "4.00 / 6.42 bars", "S&P 500 E-mini 09/11/97-05/01/09 (p.9)")),
        d("t2w_oops_i3", "Oops Index 3 (extreme day then gap)", "Oops 1-2-3 · Index", "Long/Short",
          &["F|\"1. ... greater than the 200MA. 2. ... below the 5MA. 3. The RSI2 must be below 30 one bar before the extreme price move. 4. The market must close more than 1% lower than the close of the previous day\" (p.10)",
            "E|\"5. The market gaps lower below the low of the previous day\" -> buy stop at the low of the previous bar (short mirrored)",
            "X|\"6. The close is greater than or equal to the 5MA\" -> next open"],
          OOPSN, claim("20 (9 L / 11 S)", "80.00%", "$726.25 (ES, 1 contract)", "4.00 / 7.25 bars", "S&P 500 E-mini 09/11/97-05/01/09 (p.12)")),
    ];
    let stock: [(&'static str, &'static str, &'static str, &'static [&'static str], Option<crate::connors::Claim>); 11] = [
        ("t2w_oops_11", "Oops 1.1 (equity)", "Long/Short", &["F|Close > 200MA, close < 5MA, \"3. The RSI2 must be below 20\" (p.13; short: < 200MA, > 5MA, RSI2 > 80)", "E|\"4. The market gaps lower below the low of the previous day\" -> buy stop at the prior low (short mirrored)", "X|\"5. The close is greater than or equal to the 5MA\" -> next open"],
         claim("29,577", "66.96%", "0.64%", "-", "S&P 500 members 01/01/95-05/01/09 (p.14)")),
        ("t2w_oops_12", "Oops 1.2 (gap > 1%)", "Long/Short", &["F|As Oops 1.1 (p.15)", "E|\"4. The market gaps more than 1% lower below the low of the previous day\" -> buy stop at the prior low (short: > 1% above the prior high)", "X|Close >= 5MA -> next open"],
         claim("6,303", "68.79%", "1.22%", "-", "S&P 500 members 01/01/95-05/01/09 (p.15)")),
        ("t2w_oops_13", "Oops 1.3 (RSI2 < 1)", "Long/Short", &["F|As Oops 1.1 but \"3. The RSI2 must be below 1\" (short: above 99) (p.16)", "E|Gap below the prior low -> buy stop at the prior low", "X|Close >= 5MA -> next open"],
         claim("1,111", "71.11%", "1.46%", "-", "S&P 500 members 01/01/95-05/01/09 (p.16)")),
        ("t2w_oops_14", "Oops 1.4 (RSI2 < 1 and gap > 1%)", "Long/Short", &["F|As Oops 1.1 but RSI2 below 1 (short: above 99) (p.17)", "E|\"gaps more than 1% lower below the low of the previous day\" -> buy stop at the prior low", "X|Close >= 5MA -> next open"],
         claim("258", "75.19%", "2.33%", "-", "S&P 500 members 01/01/95-05/01/09 (p.17)")),
        ("t2w_oops_21", "Oops 2.1 (5% sell-off, long only)", "Long", &["F|\"1. ... greater than the 200MA. 2. ... below the 5MA. 3. The RSI2 must be below 20 the day before the extreme price move\" (p.18)", "E|\"4. The market must close more than 5% lower than the close of the previous day\" -> market order at the open of the next day", "X|\"5. The close is greater than or equal to the 5MA\" -> next open"],
         claim("2,195", "72.62%", "3.32%", "-", "S&P 500 members 01/01/95-05/01/09 (p.19)")),
        ("t2w_oops_22", "Oops 2.2 (10% sell-off)", "Long", &["F|As Oops 2.1 (p.20)", "E|\"4. The market must close more than 10% lower than the close of the previous day\" -> next open", "X|Close >= 5MA -> next open"],
         claim("219", "79.45%", "8.16%", "-", "S&P 500 members 01/01/95-05/01/09 (p.20)")),
        ("t2w_oops_23", "Oops 2.3 (RSI2 < 2 before the sell-off)", "Long", &["F|As Oops 2.1 but \"3. The RSI2 must be below 2 one day before the extreme price move\" (p.21)", "E|Close more than 5% lower than the previous close -> next open", "X|Close >= 5MA -> next open"],
         claim("180", "73.33%", "4.89%", "-", "S&P 500 members 01/01/95-05/01/09 (p.21)")),
        ("t2w_oops_31", "Oops 3.1 (5% sell-off then gap)", "Long", &["F|\"1. ... greater than the 200MA. 2. ... below the 5MA. 3. The RSI2 must be below 20 one bar before the extreme price move. 4. The market must close more than 5% lower than the close of the previous day\" (p.22)", "E|\"5. The market gaps lower below the low of the previous day\" -> buy stop at the low of the previous bar", "X|\"6. The close is greater than or equal to the 5MA\" -> next open"],
         claim("309", "84.14%", "6.16%", "-", "S&P 500 members 01/01/95-05/01/09 (p.22)")),
        ("t2w_oops_32", "Oops 3.2 (3% sell-off then gap)", "Long", &["F|As Oops 3.1 but \"4. The market must close more than 3% lower\" (p.23)", "E|Gap below the prior low -> buy stop at the prior low", "X|Close >= 5MA -> next open"],
         claim("972", "77.98%", "3.39%", "-", "S&P 500 members 01/01/95-05/01/09 (p.24)")),
        ("t2w_oops_33", "Oops 3.3 (3.2 + gap > 1%)", "Long", &["F|As Oops 3.2 (p.25)", "E|\"5. The market gaps more than 1% lower below the low of the previous day\" -> buy stop at the prior low", "X|Close >= 5MA -> next open"],
         claim("395", "79.49%", "4.12%", "-", "S&P 500 members 01/01/95-05/01/09 (p.25)")),
        ("t2w_oops_34", "Oops 3.4 (3.2 + RSI2 < 10)", "Long", &["F|As Oops 3.2 but \"3. The RSI2 must be below 10 one bar before the extreme price move\" (p.26)", "E|Gap below the prior low -> buy stop at the prior low", "X|Close >= 5MA -> next open"],
         claim("471", "81.74%", "3.82%", "-", "S&P 500 members 01/01/95-05/01/09 (p.26)")),
    ];
    for (id, name, dir, rules, cl) in stock { v.push(d(id, name, "Oops 1-2-3 · Equity", dir, rules, OOPSN, cl)); }
    v.push(usd(d("t2w_ste", "Swing Traders Edge (per stock)", "Swing Traders Edge", "Long",
        &["F|\"3. The closing price must be less than the 5 day simple moving average. 4. ... greater than the 200 day simple moving average. 5. The 10 day ADX must be greater than 20. 6. The 100 day historical volatility must be greater than 20. 7. The 20 day simple money flow must be greater than $4,000,000\" (p.4)",
          "E|\"1. The closing price must have dropped consistently for the last 4 days. 2. The closing price must be below the opening price for that day\"",
          "E|\"a limit order is placed to purchase the stock the next day at 3% below the closing price. This order is valid for that day only.\"",
          "X|\"8. The closing price is greater than or equal to the 5 day simple moving average\" -> \"market order to exit the entire position at the next market open\"",
          "X|[Labelled, non-default] p.stop = 10 / 25 / 50: the manual's stop-loss variants (% below the entry; fill: stop price, or the open if it gaps through)"],
        STEN, None)));
    v.push(usd(d("t2w_ste_aggr", "Aggressive STE (up to two positions per stock)", "Swing Traders Edge", "Long",
        &["F|Rules 1-7 as STE (p.4)", "E|Limit 3% below the close, next day only",
          "A|\"we will allow a stock to occupy up to two portfolio positions\" (p.9): while one unit is held, a new STE signal places another 3% limit",
          "X|Close >= 5-day SMA -> the entire position at the next open"],
        STEN, None)));
    v.push(usd(d("t2w_rvt", "ReVersal Twelve (RVT)", "RVT Trading System", "Long",
        &["F|\"4. The closing price must be above the 200 day moving average. 5. The 20 day moving average of the closing price x volume must be greater than 5,000,000\" (p.3)",
          "E|\"1. The 3 day RSI is below 15. 2. The closing price must be 4% or more below yesterdays closing price. 3. The close must be below the open.\"",
          "E|\"enter a limit order 8% below the close before the market opens the next morning\"",
          "X|\"6. The close of the 3 day RSI is above 55\" -> \"market order to exit the entire position at the next open\""],
        RVTN, None)));
    v.push(StrategyDef { status: Status::NotTestable("the entry needs the \"2 day predictive RSI 50 band\", which the manual names but never defines; solving RSI backwards for a price would be our formula, not the vendor's"), ..usd(d("t2w_pr1", "PR1 setup (per stock)", "PR1 Portfolio Trading System", "Long",
        &["F|\"1. The 20 day price/volume average liquidity must be greater than $4,000,000. 2. The close must be greater than the 200 day simple moving average. 3. The 14 day ADX must be greater than 25.\" (p.3)",
          "F|\"4. The high of the day must be less than the 2 day predictive RSI 50 band\"",
          "E|\"5. The 2 day RSI must drop by half (50%) since yesterday\"",
          "E|\"a limit order for the next day at that price\" = the price where RSI(2) would fall another 50% (\"Limit orders are placed for the next day only\")",
          "X|\"The 2 period RSI closes above 60\" -> \"market order to exit the position at the open of the next trading day\""],
        PR1N, None)) });
    let port = |id, name, group, why: &'static str, rules| StrategyDef { id, name, group, session: 0, dir: "Long", status: Status::NeedsData(why),
        needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: why, claim: None };
    v.push(port("t2w_ste_portfolio", "STE 10-slot portfolio", "Swing Traders Edge",
        "needs a full US stock universe (the book scans ~10,000 stocks) and a multi-stock 10-slot portfolio simulator ranked by % drop over 4 days; the engine runs one ticker",
        &["F|\"We use a portfolio size factor of 10\" (p.6)", "E|Rank candidates by \"% price dropped = (C[4] - C) / C[4] x 100\", highest first; one slot per stock",
          "X|Published: 2000 +433.73% ... 2006 +108.32% (p.7-8, 1 cent/share commission)"]));
    v.push(port("t2w_pr1_portfolio", "PR1 10-slot portfolio", "PR1 Portfolio Trading System",
        "needs a full US stock universe and a multi-stock 10-slot portfolio simulator ranked by (PRSI - Close)/PRSI; the engine runs one ticker",
        &["F|\"The portfolio is split equally into 10 separate slots\" (p.5)", "E|Rank by \"(PRSI-Close) / PRSI x 100\", most oversold first",
          "X|Published monthly returns 01/2001-10/2009 (p.7), e.g. 2001 +122.29%, 2008 +44.23%"]));
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("t2w_") { return None; }
    let v = match id {
        "t2w_mrx" => ("t2w_mrx", "MRX swing system", "Standard"),
        "t2w_mrx_plus" => ("t2w_mrx", "MRX swing system", "MRX+"),
        "t2w_mrx_stock" => ("t2w_mrx", "MRX swing system", "Stocks"),
        "t2w_r90" => ("t2w_r90", "R90 swing system", "Standard"),
        "t2w_r90_stock" => ("t2w_r90", "R90 swing system", "Stocks"),
        "t2w_cts" => ("t2w_cts", "Crouching Tiger", "Standard (3 days)"),
        "t2w_cts_plus" => ("t2w_cts", "Crouching Tiger", "+ (4 days)"),
        "t2w_cts_stock" => ("t2w_cts", "Crouching Tiger", "Stocks (5 days)"),
        "t2w_ste" => ("t2w_ste", "Swing Traders Edge", "Original"),
        "t2w_ste_aggr" => ("t2w_ste", "Swing Traders Edge", "Aggressive"),
        "t2w_ste_portfolio" => ("t2w_ste", "Swing Traders Edge", "10-slot portfolio"),
        "t2w_rvt" => ("t2w_rvt", "ReVersal Twelve", "Standard"),
        "t2w_pr1" => ("t2w_pr1", "PR1 portfolio system", "Per-stock setup"),
        "t2w_pr1_portfolio" => ("t2w_pr1", "PR1 portfolio system", "10-slot portfolio"),
        "t2w_oops_i1" => ("t2w_oops_index", "Oops 1-2-3 (index)", "Oops 1"),
        "t2w_oops_i2" => ("t2w_oops_index", "Oops 1-2-3 (index)", "Oops 2"),
        "t2w_oops_i3" => ("t2w_oops_index", "Oops 1-2-3 (index)", "Oops 3"),
        "t2w_oops_11" => ("t2w_oops_equity", "Oops 1-2-3 (equity)", "1.1"),
        "t2w_oops_12" => ("t2w_oops_equity", "Oops 1-2-3 (equity)", "1.2"),
        "t2w_oops_13" => ("t2w_oops_equity", "Oops 1-2-3 (equity)", "1.3"),
        "t2w_oops_14" => ("t2w_oops_equity", "Oops 1-2-3 (equity)", "1.4"),
        "t2w_oops_21" => ("t2w_oops_equity", "Oops 1-2-3 (equity)", "2.1"),
        "t2w_oops_22" => ("t2w_oops_equity", "Oops 1-2-3 (equity)", "2.2"),
        "t2w_oops_23" => ("t2w_oops_equity", "Oops 1-2-3 (equity)", "2.3"),
        "t2w_oops_31" => ("t2w_oops_equity", "Oops 1-2-3 (equity)", "3.1"),
        "t2w_oops_32" => ("t2w_oops_equity", "Oops 1-2-3 (equity)", "3.2"),
        "t2w_oops_33" => ("t2w_oops_equity", "Oops 1-2-3 (equity)", "3.3"),
        "t2w_oops_34" => ("t2w_oops_equity", "Oops 1-2-3 (equity)", "3.4"),
        _ => return None,
    };
    Some(v)
}

/// Oops settings: (kind 1 gap / 2 extreme / 3 both, rsi_lo, rsi_hi, gap %, extreme %, shorts)
fn oops_cfg(id: &str) -> Option<(u8, f64, f64, f64, f64, bool)> {
    Some(match id {
        "t2w_oops_i1" => (1, 30., 70., 0., 0., true), "t2w_oops_i2" => (2, 30., 70., 0., 1., true), "t2w_oops_i3" => (3, 30., 70., 0., 1., true),
        "t2w_oops_11" => (1, 20., 80., 0., 0., true), "t2w_oops_12" => (1, 20., 80., 1., 0., true),
        "t2w_oops_13" => (1, 1., 99., 0., 0., true), "t2w_oops_14" => (1, 1., 99., 1., 0., true),
        "t2w_oops_21" => (2, 20., 80., 0., 5., false), "t2w_oops_22" => (2, 20., 80., 0., 10., false), "t2w_oops_23" => (2, 2., 98., 0., 5., false),
        "t2w_oops_31" => (3, 20., 80., 0., 5., false), "t2w_oops_32" => (3, 20., 80., 0., 3., false),
        "t2w_oops_33" => (3, 20., 80., 1., 3., false), "t2w_oops_34" => (3, 10., 90., 0., 3., false),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if let Some((kind, lo, hi, gap, ext, shorts)) = oops_cfg(id) {
        let mut v = vec![p("trend", "Trend SMA length", 200., 50., 300., 10.), p("exit_ma", "Exit SMA length", 5., 3., 20., 1.),
                         p("rsi_lo", "RSI(2) oversold below", lo, 0., 50., 1.)];
        if shorts { v.push(p("rsi_hi", "RSI(2) overbought above", hi, 50., 100., 1.)); }
        if kind != 2 { v.push(p("gap", "Minimum gap beyond prior low/high, %", gap, 0., 5., 0.5)); }
        if kind != 1 { v.push(p("extreme", "Extreme close-to-close move, % (more than)", ext, 0., 15., 0.5)); }
        return Some(v);
    }
    Some(match id {
        "t2w_mrx" | "t2w_mrx_plus" | "t2w_mrx_stock" => {
            let (back, e1, e2, delay, ma) = match id { "t2w_mrx" => (1., 1., 2., 1., 5.), "t2w_mrx_plus" => (2., 2., 3., 1., 5.), _ => (4., 4., 6., 2., 10.) };
            vec![p("trend", "Trend XMA length", 200., 50., 300., 10.), p("entry_ma", "Entry XMA length", ma, 3., 20., 1.),
                 p("back", "Close N bars ago beyond the entry XMA", back, 1., 6., 1.), p("entry1", "Consecutive lower closes, unit 1", e1, 1., 8., 1.),
                 p("entry2", "Consecutive lower closes, unit 2", e2, 1., 10., 1.), p("delay", "Bars between first and second entry", delay, 1., 5., 1.),
                 p("exit_ma", "Exit XMA length", 5., 3., 20., 1.)]
        }
        "t2w_r90" | "t2w_r90_stock" => {
            let (p1, p2, delay) = if id == "t2w_r90" { (90., 50., 1.) } else { (95., 60., 2.) };
            vec![p("trend_len", "Trend RSI length (manual: 100 in inputs/example, 200 in rule text)", 100., 50., 250., 50.),
                 p("rsi_len", "Short RSI length", 2., 2., 5., 1.), p("nobars", "Bars for the % change", 3., 1., 5., 1.),
                 p("pct1", "First entry: RSI change at least, %", p1, 50., 99., 1.), p("pct2", "Second entry: 1-bar RSI change at least, %", p2, 20., 90., 5.),
                 p("delay", "Bars between first and second entry", delay, 1., 5., 1.), p("exit_long", "Long exit: RSI >=", 70., 50., 90., 5.),
                 p("exit_short", "Short exit: RSI <=", 30., 10., 50., 5.)]
        }
        "t2w_cts" | "t2w_cts_plus" | "t2w_cts_stock" => {
            let days = match id { "t2w_cts" => 3., "t2w_cts_plus" => 4., _ => 5. };
            vec![p("trend", "Trend SMA length", 200., 50., 300., 10.), p("days", "Days RSI(2) beyond the threshold", days, 1., 8., 1.),
                 p("lo", "Long: RSI(2) below", 40., 10., 50., 5.), p("hi", "Short: RSI(2) above", 60., 50., 90., 5.),
                 p("exit_long", "Long exit: RSI(2) >", 70., 50., 90., 5.), p("exit_short", "Short exit: RSI(2) <", 30., 10., 50., 5.)]
        }
        "t2w_ste" | "t2w_ste_aggr" => {
            let mut v = vec![p("down", "Consecutive lower closes", 4., 2., 8., 1.), p("adx", "ADX(10) above", 20., 0., 40., 5.), p("hv", "HV(100) above", 20., 0., 60., 5.),
                 p("mf", "20-day money flow above, US$", 4_000_000., 0., 50_000_000., 1_000_000.), p("limit", "Limit below the close, %", 3., 0., 10., 0.5)];
            if id == "t2w_ste" { v.push(p("stop", "Stop loss % below entry (0 = none, book original)", 0., 0., 50., 5.)); }
            v
        }
        "t2w_rvt" => vec![p("rsi_lt", "RSI(3) below", 15., 5., 30., 1.), p("drop", "Close down at least, %", 4., 1., 10., 0.5),
                          p("mf", "20-day money flow above, US$", 5_000_000., 0., 50_000_000., 1_000_000.), p("limit", "Limit below the close, %", 8., 1., 15., 1.),
                          p("exit", "Exit: RSI(3) above", 55., 40., 80., 5.)],
        "t2w_pr1" => vec![p("mf", "20-day price x volume above, US$", 4_000_000., 0., 50_000_000., 1_000_000.), p("adx", "ADX(14) above", 25., 10., 40., 5.),
                          p("drop", "RSI(2) fall since yesterday at least (fraction)", 0.5, 0.1, 0.9, 0.1), p("mid", "Predictive RSI midline", 50., 30., 70., 5.),
                          p("entry", "Limit: RSI(2) falls another (fraction)", 0.5, 0.1, 0.9, 0.1), p("exit", "Exit: RSI(2) above", 60., 40., 80., 5.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from >= to || to >= x.r.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let c = &x.c;
    if let Some((kind, _, _, _, _, shorts)) = oops_cfg(id) {
        return Some(oops(x, from, to, kind, shorts, u("trend"), u("exit_ma"), g("rsi_lo"), if shorts { g("rsi_hi") } else { 100.0 },
                         if kind != 2 { g("gap") } else { 0.0 }, if kind != 1 { g("extreme") } else { 0.0 }));
    }
    match id {
        "t2w_mrx" | "t2w_mrx_plus" | "t2w_mrx_stock" => {
            let (tr, ein, eout) = (ema(c, u("trend")), ema(c, u("entry_ma")), ema(c, u("exit_ma")));
            let (back, e1, e2, delay, warm) = (u("back"), u("entry1"), u("entry2"), u("delay"), u("trend") - 1);
            let (lo, hi) = (&x.down_closes, &x.up_closes);
            let entry = |i: usize| -> i8 {
                if i < warm.max(back) { return 0; }
                if c[i] > tr[i] && c[i - back] < ein[i - back] && lo[i] >= e1 { 1 }
                else if c[i] < tr[i] && c[i - back] > ein[i - back] && hi[i] >= e1 { -1 } else { 0 }
            };
            let add = |i: usize, d: f64, sig: usize| i - sig >= delay && (if d > 0.0 { lo[i] } else { hi[i] }) >= e2;
            let exit = |i: usize, d: f64| if d > 0.0 { c[i] >= eout[i] } else { c[i] <= eout[i] };
            Some(next_open_sim(x, from, to, 2, &entry, &add, &exit, "close crossed the exit XMA · next open"))
        }
        "t2w_r90" | "t2w_r90_stock" => {
            let (rs, rt) = (x.rsi_n(u("rsi_len")), x.rsi_n(u("trend_len")));
            let (nb, p1, p2, delay, xl, xs) = (u("nobars"), g("pct1"), g("pct2"), u("delay"), g("exit_long"), g("exit_short"));
            let fall = |i: usize, k: usize| { let b = rs[i - k]; if b > 0.0 { (b - rs[i]) / b * 100.0 } else { f64::NAN } };
            let rise = |i: usize, k: usize| { let b = 100.0 - rs[i - k]; if b > 0.0 { (b - (100.0 - rs[i])) / b * 100.0 } else { f64::NAN } };
            let entry = |i: usize| -> i8 {
                if i < nb || !rt[i].is_finite() || !rs[i - nb].is_finite() { return 0; }
                if rt[i] > 50.0 && fall(i, nb) >= p1 { 1 } else if rt[i] < 50.0 && rise(i, nb) >= p1 { -1 } else { 0 }
            };
            let add = |i: usize, d: f64, sig: usize| i - sig >= delay && (if d > 0.0 { fall(i, 1) } else { rise(i, 1) }) >= p2;
            let exit = |i: usize, d: f64| if d > 0.0 { rs[i] >= xl } else { rs[i] <= xs };
            Some(next_open_sim(x, from, to, 2, &entry, &add, &exit, "RSI exit level · next open"))
        }
        "t2w_cts" | "t2w_cts_plus" | "t2w_cts_stock" => {
            let (tr, r2) = (x.sma_n(u("trend")), &x.rsi2);
            let (days, lo, hi, xl, xs) = (u("days"), g("lo"), g("hi"), g("exit_long"), g("exit_short"));
            let cond = |i: usize| -> i8 {
                if i + 1 < days || !tr[i].is_finite() || (i + 1 - days..=i).any(|k| !r2[k].is_finite()) { return 0; }
                if c[i] > tr[i] && (i + 1 - days..=i).all(|k| r2[k] < lo) { 1 }
                else if c[i] < tr[i] && (i + 1 - days..=i).all(|k| r2[k] > hi) { -1 } else { 0 }
            };
            let add = |i: usize, d: f64, sig: usize| i > sig && cond(i) as f64 == d;
            let exit = |i: usize, d: f64| if d > 0.0 { r2[i] > xl } else { r2[i] < xs };
            Some(next_open_sim(x, from, to, 2, &cond, &add, &exit, "RSI(2) exit level · next open"))
        }
        "t2w_ste" | "t2w_ste_aggr" => {
            let Some(mf) = money_flow_usd(x, 20) else { return Some(vec![]) };
            let (ma5, ma200, hvv, ad) = (&x.ma5, &x.ma200, &x.hv100, &x.adx10);
            let (down, adxm, hvm, mfm, lim) = (u("down"), g("adx"), g("hv"), g("mf"), g("limit"));
            let sig = |i: usize| x.down_closes[i] >= down && c[i] < x.o[i] && c[i] < ma5[i] && c[i] > ma200[i] && ad[i] > adxm && hvv[i] > hvm && mf[i] > mfm;
            let stop = if id == "t2w_ste" { g("stop") } else { 0.0 };
            Some(limit_sim(x, from, to, if id == "t2w_ste" { 1 } else { 2 }, &|i| sig(i).then(|| c[i] * (1.0 - lim / 100.0)),
                           &|i| c[i] >= ma5[i], stop, "close >= 5-day SMA · next open"))
        }
        "t2w_rvt" => {
            let Some(mf) = money_flow_usd(x, 20) else { return Some(vec![]) };
            let (r3, ma200) = (&x.rsi3, &x.ma200);
            let (rl, drop, mfm, lim, ex) = (g("rsi_lt"), g("drop"), g("mf"), g("limit"), g("exit"));
            let sig = |i: usize| i > 0 && r3[i] < rl && c[i] <= c[i - 1] * (1.0 - drop / 100.0) && c[i] < x.o[i] && c[i] > ma200[i] && mf[i] > mfm;
            Some(limit_sim(x, from, to, 1, &|i| sig(i).then(|| c[i] * (1.0 - lim / 100.0)), &|i| r3[i] > ex, 0.0, "RSI(3) > exit level · next open"))
        }
        "t2w_pr1" => {
            let Some(mf) = money_flow_usd(x, 20) else { return Some(vec![]) };
            let n = 2;
            let (au, ad) = rsi_avgs(c, n);
            let r2 = rsi(c, n);
            let (adx14, ma200) = (adx(x, 14), &x.ma200);
            let (mfm, adxm, drop, mid, ent, ex) = (g("mf"), g("adx"), g("drop"), g("mid"), g("entry"), g("exit"));
            let band = |i: usize| -> f64 {
                // RSI = mid  <=>  AvgUp/AvgDown = mid/(100-mid); general inversion around yesterday's averages
                if mid == 50.0 { rsi50_band(c[i - 1], au[i - 1], ad[i - 1], n) } else {
                    let k = mid / (100.0 - mid); let m = n as f64 - 1.0;
                    let up = c[i - 1] + m * (k * ad[i - 1] - au[i - 1]);
                    if up >= c[i - 1] { up } else { c[i - 1] - m * (au[i - 1] / k - ad[i - 1]) }
                }
            };
            let sig = |i: usize| i > 1 && mf[i] > mfm && c[i] > ma200[i] && adx14[i] > adxm && x.h[i] < band(i)
                && r2[i - 1] > 0.0 && r2[i] <= r2[i - 1] * (1.0 - drop) && r2[i] > 0.0;
            Some(limit_sim(x, from, to, 1, &|i| sig(i).then(|| rsi_price_below(c[i], au[i], ad[i], n, r2[i] * (1.0 - ent))),
                           &|i| r2[i] > ex, 0.0, "RSI(2) > exit level · next open"))
        }
        _ => None,
    }
}

/// Oops: kind 1 = setup on yesterday's close + gap beyond yesterday's extreme -> stop at that extreme today;
/// kind 2 = setup + extreme move on today's close -> next open; kind 3 = kind-2 setup yesterday + kind-1 gap today.
#[allow(clippy::too_many_arguments)]
fn oops(x: &Ctx, from: usize, to: usize, kind: u8, shorts: bool, trend: usize, exit_ma: usize, rlo: f64, rhi: f64, gap: f64, ext: f64) -> Vec<Trade> {
    let (c, o, h, l) = (&x.c, &x.o, &x.h, &x.l);
    let (tr, m5, r2) = (x.sma_n(trend), x.sma_n(exit_ma), &x.rsi2);
    let setup = |j: usize, d: f64| -> bool {
        if j < 1 || !tr[j].is_finite() || !m5[j].is_finite() { return false; }
        let base = if d > 0.0 { c[j] > tr[j] && c[j] < m5[j] } else { c[j] < tr[j] && c[j] > m5[j] };
        base && if kind == 1 {
            if d > 0.0 { r2[j] < rlo } else { r2[j] > rhi }
        } else if d > 0.0 { r2[j - 1] < rlo && c[j] < c[j - 1] * (1.0 - ext / 100.0) } else { r2[j - 1] > rhi && c[j] > c[j - 1] * (1.0 + ext / 100.0) }
    };
    let dirs: &[f64] = if shorts { &[1.0, -1.0] } else { &[1.0] };
    let exit = |i: usize, d: f64| if d > 0.0 { c[i] >= m5[i] } else { c[i] <= m5[i] };
    if kind == 2 {
        let entry = |i: usize| -> i8 { dirs.iter().find(|&&d| setup(i, d)).map_or(0, |&d| d as i8) };
        return next_open_sim(x, from, to, 1, &entry, &|_, _, _| false, &exit, "close vs 5MA · next open");
    }
    let (mut out, mut cur, mut pend) = (vec![], None::<Trade>, false);
    for i in from..=to {
        if pend {
            let mut t = cur.take().unwrap(); t.exit_i = i; t.exit_px = o[i]; out.push(t); pend = false;
        }
        if cur.is_none() && i >= 1 {
            for &d in dirs {
                if !setup(i - 1, d) { continue; }
                if d > 0.0 && o[i] < l[i - 1] * (1.0 - gap / 100.0) && h[i] >= l[i - 1] {
                    cur = Some(new_trade(1.0, i, l[i - 1], 1.0, "stop", i - 1));
                } else if d < 0.0 && o[i] > h[i - 1] * (1.0 + gap / 100.0) && l[i] <= h[i - 1] {
                    cur = Some(new_trade(-1.0, i, h[i - 1], 1.0, "stop", i - 1));
                }
                if cur.is_some() { break; }
            }
        }
        if i == to { break; }
        if let Some(t) = cur.as_mut() {
            if exit(i, t.dir) { t.reason = "close vs 5MA · next open".into(); t.signal_i = i; pend = true; }
        }
    }
    close_open(&mut out, cur, x, to);
    out
}

/// Long limit-entry system: order(i) = limit price for bar i+1 (day only), filled when low <= limit at min(open, limit).
/// Up to `units` units (each new signal while holding places another order). exit(i) on a close -> all out next open.
/// stop > 0: stop `stop`% below the first entry, checked from the fill bar (gap through -> open).
fn limit_sim(x: &Ctx, from: usize, to: usize, units: usize, order: &dyn Fn(usize) -> Option<f64>, exit: &dyn Fn(usize) -> bool, stop: f64, why: &str) -> Vec<Trade> {
    let w = 1.0 / units as f64;
    let (mut out, mut cur, mut lim, mut pend_exit) = (vec![], None::<Trade>, None::<(f64, usize)>, false);
    for i in from..=to {
        if pend_exit {
            let mut t = cur.take().unwrap(); t.exit_i = i; t.exit_px = x.o[i]; out.push(t); pend_exit = false;
        }
        if let Some((px, s)) = lim.take() {
            if x.l[i] <= px {
                let f = x.o[i].min(px);
                match cur.as_mut() {
                    Some(t) => t.legs.push(Leg { i, px: f, w }),
                    None => cur = Some(new_trade(1.0, i, f, w, "limit", s)),
                }
            }
        }
        if stop > 0.0 {
            if let Some(t) = cur.as_ref() {
                let sp = t.legs[0].px * (1.0 - stop / 100.0);
                if x.l[i] <= sp {
                    let mut t = cur.take().unwrap();
                    t.exit_i = i; t.exit_px = if t.legs[0].i < i { x.o[i].min(sp) } else { sp };
                    t.reason = format!("{stop}% stop loss"); t.signal_i = i; out.push(t); continue;
                }
            }
        }
        if i == to { break; }
        if cur.as_ref().is_some_and(|_| exit(i)) {
            let t = cur.as_mut().unwrap(); t.reason = why.into(); t.signal_i = i; pend_exit = true; continue;
        }
        if cur.as_ref().map_or(0, |t| t.legs.len()) < units {
            if let Some(px) = order(i) { lim = Some((px, i)); }
        }
    }
    close_open(&mut out, cur, x, to);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(cl: &[f64]) -> Vec<Row> {
        cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2000-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: c, high: c + 1.0, low: c - 1.0, close: c, volume: None }).collect()
    }
    #[test]
    fn predictive_rsi_inversions_hit_their_targets() {
        let c: Vec<f64> = (0..60).map(|i| 100.0 + ((i * 7) % 11) as f64 - 5.0).collect();
        let (au, ad) = rsi_avgs(&c, 2);
        let i = 50;
        let target = rsi(&c, 2)[i] * 0.5;
        let p = rsi_price_below(c[i], au[i], ad[i], 2, target);
        let mut c2 = c[..=i].to_vec(); c2.push(p);
        assert!((rsi(&c2, 2)[i + 1] - target).abs() < 1e-9);
        let b = rsi50_band(c[i], au[i], ad[i], 2);
        let mut c3 = c[..=i].to_vec(); c3.push(b);
        assert!((rsi(&c3, 2)[i + 1] - 50.0).abs() < 1e-9);
    }
    #[test]
    fn mrx_second_unit_can_come_on_the_first_entry_bar_close() {
        // 250 rising closes, then two lower closes after a close below the 5XMA, then a recovery
        let mut cl: Vec<f64> = (0..250).map(|i| 100.0 + i as f64).collect();
        cl.extend([340.0, 335.0, 330.0, 360.0, 370.0]);
        let r = rows(&cl);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("t2w_mrx", &x, 200, r.len() - 1, &Params::new()).unwrap();
        let t = t.iter().find(|t| t.legs.len() == 2).expect("two-unit trade");
        assert_eq!(t.legs[1].i, t.legs[0].i + 1); // signal on bar 251's close, fill 252, add signal on 252's close, fill 253
        assert!(t.legs.iter().all(|l| (x.l[l.i]..=x.h[l.i]).contains(&l.px)));
    }
    #[test]
    fn rvt_limit_fills_and_money_flow_needs_usdinr() {
        // fixture rows for a hand check only (never served as data)
        let mut cl: Vec<f64> = (0..230).map(|i| 50.0 + i as f64 * 0.2).collect();
        cl.extend([100.0, 98.0, 80.0, 75.0, 110.0]); // bar 232: -18.4% close-to-close
        let mut r = rows(&cl);
        for x in r.iter_mut() { x.volume = Some(1_000_000.0); }
        r[232].open = 85.0; r[232].high = 86.0;
        r[233].open = 82.0; r[233].high = 83.0; r[233].low = 70.0; // limit 80 x 0.92 = 73.6 is traded through
        let mut x = Ctx::new(&r, &r, &[]);
        assert!(run("t2w_rvt", &x, 200, 234, &Params::new()).unwrap().is_empty()); // no USDINR -> no trades
        x.extra.insert("USDINR".into(), vec![1.0; r.len()]);
        let t = run("t2w_rvt", &x, 200, 234, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, 233);
        assert!((t[0].legs[0].px - 73.6).abs() < 1e-9);
        x.extra.insert("USDINR".into(), vec![1000.0; r.len()]); // US$ flow below 5M -> filtered
        assert!(run("t2w_rvt", &x, 200, 234, &Params::new()).unwrap().is_empty());
    }
    #[test]
    fn catalogue_ids_unique_and_prefixed() {
        let v = catalogue();
        let mut ids: Vec<_> = v.iter().map(|d| d.id).collect();
        ids.sort(); ids.dedup();
        assert_eq!(ids.len(), v.len());
        assert!(v.iter().all(|d| d.id.starts_with("t2w_") && family_of(d.id).is_some()));
        for d in &v { if d.status == Status::Live { assert!(params(d.id).is_some(), "{}", d.id); } }
    }
}
