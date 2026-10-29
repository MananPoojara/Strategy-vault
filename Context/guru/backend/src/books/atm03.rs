//! Active Trader Magazine, 2003 issues (research drive Trading Knowledge II/@NewsLetters-Mags/Active Trader Magazine/2003/,
//! copies in Books/Active Trader Magazine/2003/). Book id "atm03". Trading System Lab (stocks + futures) columns and the
//! fully specified strategy columns of 2003. Notes, page refs, oracle: library/atm03/README.md.
//! Own simulator (no jkbwts::sim). The shared helpers below are also used by atm04.
use crate::connors::{atr, dmi, rsi, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use Status::*;

pub const BOOK: &str = "atm03";
/// Admitted ids (authors/activetrader/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["atm03_bb60", "atm03_pullback6", "atm03_pullback_fut", "atm03_tpe", "atm03_tpe_ls", "atm03_ch100_20", "atm03_triple2", "atm03_triple5", "atm03_blood", "atm03_mfi", "atm03_twobar", "atm03_cmo_srsi", "atm03_ob_cont", "atm03_memorial", "atm03_july4"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ------------------------------------------------------------------ helpers (shared with atm04)

pub fn new_trade(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}
pub fn done(mut t: Trade, j: usize, px: f64, sig: usize, why: &str) -> Trade {
    t.exit_i = j; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); t
}
pub fn finish(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
}
/// Population standard deviation over n bars (TradeStation StdDev / Wealth-Lab StdDev).
pub fn stdev(v: &[f64], n: usize) -> Vec<f64> {
    let m = sma(v, n);
    (0..v.len()).map(|i| if m[i].is_nan() { f64::NAN } else {
        (v[i + 1 - n..=i].iter().map(|x| (x - m[i]).powi(2)).sum::<f64>() / n as f64).sqrt() }).collect()
}
/// Exponential average 2/(n+1) seeded with the first value (TradeStation XAverage).
pub fn xavg(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() { out[i] = if i == 0 { v[0] } else { out[i - 1] + a * (v[i] - out[i - 1]) }; }
    out
}
/// Exponential average 2/(n+1) seeded with the SMA of the first n valid values (Wealth-Lab EMA); leading NaNs skipped.
pub fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; v.len()];
    let Some(s) = v.iter().position(|x| !x.is_nan()) else { return out };
    if v.len() < s + n { return out; }
    out[s + n - 1] = v[s..s + n].iter().sum::<f64>() / n as f64;
    for i in s + n..v.len() { out[i] = out[i - 1] + a * (v[i] - out[i - 1]); }
    out
}
/// Linearly weighted moving average, weights 1..n (newest = n).
pub fn wma(v: &[f64], n: usize) -> Vec<f64> {
    let d = (n * (n + 1) / 2) as f64;
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { (0..n).map(|k| (k + 1) as f64 * v[i + 1 - n + k]).sum::<f64>() / d }).collect()
}
/// Highest value of v over the n bars ending at i (inclusive); NaN if not enough history.
pub fn hh(v: &[f64], i: usize, n: usize) -> f64 { if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max) } }
pub fn ll(v: &[f64], i: usize, n: usize) -> f64 { if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min) } }
/// Chande Momentum Oscillator: 100 * (sum up - sum down) / (sum up + sum down) over n close-to-close changes.
pub fn cmo(c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|i| if i < n { f64::NAN } else {
        let (mut u, mut d) = (0., 0.);
        for k in i + 1 - n..=i { let x = c[k] - c[k - 1]; if x > 0. { u += x } else { d -= x } }
        if u + d > 0. { 100. * (u - d) / (u + d) } else { 0. } }).collect()
}
/// Chande StochRSI(n) on a 0-100 scale: (RSI(n) - lowest RSI(n), n bars) / (highest - lowest) * 100.
pub fn stoch_rsi(c: &[f64], n: usize) -> Vec<f64> {
    let r = rsi(c, n);
    (0..c.len()).map(|i| { let (lo, hi) = (ll(&r, i, n), hh(&r, i, n)); if r[i].is_nan() || lo.is_nan() || !(hi > lo) { f64::NAN } else { 100. * (r[i] - lo) / (hi - lo) } }).collect()
}
/// Money Flow Index (typical price x volume), n bars.
pub fn mfi(x: &Ctx, n: usize) -> Vec<f64> {
    let tp: Vec<f64> = (0..x.c.len()).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    let v: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    (0..tp.len()).map(|i| if i < n { f64::NAN } else {
        let (mut p, mut m) = (0., 0.);
        for k in i + 1 - n..=i { let f = tp[k] * v[k]; if tp[k] > tp[k - 1] { p += f } else if tp[k] < tp[k - 1] { m += f } }
        if p.is_nan() || m.is_nan() { f64::NAN } else if m == 0. { 100. } else { 100. - 100. / (1. + p / m) } }).collect()
}
/// Commodity Channel Index (typical price, SMA, mean absolute deviation, 0.015).
pub fn cci_of(h: &[f64], l: &[f64], c: &[f64], n: usize) -> Vec<f64> {
    let tp: Vec<f64> = (0..c.len()).map(|i| (h[i] + l[i] + c[i]) / 3.).collect();
    let m = sma(&tp, n);
    (0..tp.len()).map(|i| if m[i].is_nan() { f64::NAN } else {
        let md = tp[i + 1 - n..=i].iter().map(|x| (x - m[i]).abs()).sum::<f64>() / n as f64;
        if md > 0. { (tp[i] - m[i]) / (0.015 * md) } else { 0. } }).collect()
}
/// Long-side stop fill (sell stop s / buy stop for shorts mirrored by caller): gap through = open.
pub fn stop_dn(x: &Ctx, j: usize, s: f64) -> Option<f64> { if !s.is_nan() && x.l[j] <= s { Some(x.o[j].min(s)) } else { None } }
pub fn stop_up(x: &Ctx, j: usize, s: f64) -> Option<f64> { if !s.is_nan() && x.h[j] >= s { Some(x.o[j].max(s)) } else { None } }
pub fn lim_up(x: &Ctx, j: usize, t: f64) -> Option<f64> { stop_up(x, j, t) } // sell limit above / buy-stop: same fill rule
pub fn lim_dn(x: &Ctx, j: usize, t: f64) -> Option<f64> { stop_dn(x, j, t) }

/// Market-on-open state machine: decide(i, pos) at the close of bar i returns the wanted position
/// (Some(1/-1/0)) or None (hold); a change fills at the open of bar i+1 (a reversal closes and opens there).
pub fn by_open(x: &Ctx, from: usize, to: usize, decide: impl Fn(usize, i8, Option<&Trade>) -> Option<i8>) -> Vec<Trade> {
    let (mut out, mut cur, mut pos) = (vec![], None::<Trade>, 0i8);
    for i in from..to {
        let Some(want) = decide(i, pos, cur.as_ref()) else { continue };
        if want == pos { continue; }
        let j = i + 1;
        if let Some(t) = cur.take() { out.push(done(t, j, x.o[j], i, match want { 1 => "buy at open (reverse)", -1 => "sell at open (reverse)", _ => "exit at open" })); }
        if want != 0 { cur = Some(new_trade(want as f64, j, x.o[j], "open")); }
        pos = want;
    }
    finish(&mut out, cur, x, to);
    out
}

/// Independent positions (each with its own exit) folded into non-overlapping trades: positions that overlap in time
/// form one trade whose legs are the entries (weight w each) and whose scale-out pieces are the individual exits; exit_px
/// is set so that Trade::ret() equals the exact sum of the positions' returns.
pub struct Pos { pub dir: f64, pub i: usize, pub px: f64, pub w: f64, pub xi: usize, pub xpx: f64, pub why: String, pub open: bool }
pub fn fold(mut ps: Vec<Pos>) -> Vec<Trade> {
    ps.sort_by_key(|p| (p.i, p.xi));
    let mut out = vec![];
    let mut k = 0;
    while k < ps.len() {
        let mut end = ps[k].xi;
        let mut e = k + 1;
        while e < ps.len() && ps[e].i <= end { end = end.max(ps[e].xi); e += 1; }
        let g = &ps[k..e];
        let tw: f64 = g.iter().map(|p| p.w).sum();
        let num: f64 = g.iter().map(|p| p.w * p.xpx / p.px).sum();
        let den: f64 = g.iter().map(|p| p.w / p.px).sum();
        let mut ex: Vec<(usize, f64, f64)> = g.iter().map(|p| (p.xi, p.xpx, p.w / tw)).collect();
        ex.sort_by_key(|e| e.0);
        let last = g.iter().max_by_key(|p| p.xi).unwrap();
        out.push(Trade { dir: g[0].dir, legs: g.iter().map(|p| Leg { i: p.i, px: p.px, w: p.w }).collect(), exit_i: end, exit_px: num / den,
            reason: format!("{} position(s); last: {}", g.len(), last.why), entry_kind: "open", signal_i: end, open: g.iter().any(|p| p.open),
            exits: if g.len() > 1 { ex } else { vec![] } });
        k = e;
    }
    out
}

/// US market holidays used by the Market History columns (observed dates). Returns day numbers.
pub fn day_num(date: &str) -> i64 {
    let (y, m, d): (i64, i64, i64) = (date[0..4].parse().unwrap(), date[5..7].parse().unwrap(), date[8..10].parse().unwrap());
    civil(y, m, d)
}
pub fn civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468
}
pub fn weekday(dn: i64) -> i64 { (dn + 4).rem_euclid(7) } // 0 = Sunday
pub fn memorial_day(y: i64) -> i64 {
    if y < 1971 { return civil(y, 5, 30); }
    let mut d = civil(y, 5, 31);
    while weekday(d) != 1 { d -= 1; }
    d
}
pub fn july4_observed(y: i64) -> i64 {
    let d = civil(y, 7, 4);
    match weekday(d) { 6 => d - 1, 0 => d + 1, _ => d }
}
/// Holiday trade on bars: entry at the close of the `before`-th bar dated before the holiday, exit at the close of the
/// `after`-th bar dated after it (after < 0: |after|-th bar before the holiday). One trade per year.
pub fn holiday_trades(x: &Ctx, from: usize, to: usize, hol: impl Fn(i64) -> i64, before: usize, after: i64) -> Vec<Trade> {
    let dn: Vec<i64> = x.r.iter().map(|r| day_num(&r.date)).collect();
    let (y0, y1): (i64, i64) = (x.r[from].date[0..4].parse().unwrap(), x.r[to].date[0..4].parse().unwrap());
    let mut out = vec![];
    for y in y0..=y1 {
        let h = hol(y);
        let first_after = dn.partition_point(|&d| d <= h); // first bar dated after the holiday
        let first_on_or_after = dn.partition_point(|&d| d < h);
        if first_on_or_after < before { continue; }
        let e = first_on_or_after - before;
        let xi = if after > 0 { first_after + after as usize - 1 } else { match first_on_or_after.checked_sub((-after) as usize) { Some(v) => v, None => continue } };
        if e < from || e >= to || xi <= e { continue; }
        let t = new_trade(1., e, x.c[e], "close");
        if xi > to { let mut t = t; t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); continue; }
        out.push(done(t, xi, x.c[xi], xi, "holiday exit at the close"));
    }
    out
}

fn checked(p: &Params, s: &[ParamDef]) -> bool { p.keys().all(|k| s.iter().any(|d| d.key == k)) }

// ------------------------------------------------------------------ catalogue


pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, vol: bool, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: vol, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    let c = crate::connors::claim;
    vec![
        d("atm03_bb60", "Long-term volatility breakout · 60-day Bollinger Bands stop-and-reverse", "TSL Jan 2003 (p.50; futures p.62)", "Long/Short", Live, false,
          &["E|\"1. Go long on the open if yesterday's close is above the upper 60-day Bollinger Band.\" (TSL Jan 2003 p.50)",
            "X|\"2. Exit and enter a short trade if yesterday's close is below the lower 60-day Bollinger Band. Reverse the rules for short trades.\"",
            "F|Bands \"set two standard deviations above and below the moving average\""],
          "Omega ProSuite test. Bands = SMA(60) +/- 2 population standard deviations of the close [B: TradeStation StdDev]. Book: 30 Nasdaq-100 stocks Mar 1993-Sep 2002 (stocks) and 15 commodity futures (futures column, same rules); run on the indices (instrument swap). Sizing (1-2% risk / farthest band) not modelled: full equity per trade.",
          c("n/a", "n/a", "PF 1.33 (stocks), 1.32 (futures)", "", "30 Nasdaq-100 stocks 3/1993-9/2002: +174%, 11.11%/yr, max DD 38.1%; 15 futures: +232%, 13.35%/yr")),
        d("atm03_dyn_breakout", "Dynamic breakout system (20-60 day volatility-adjusted lookback)", "TSL Feb 2003 (p.60; futures p.70)", "Long/Short",
          NotTestable("the column says the lookback 'can range from 20 to 60 days and will change daily depending on the level of volatility (30-day standard deviation of the close)' but never gives the formula mapping volatility to the lookback"), false,
          &["E|\"1. Go long on the open if yesterday's close is higher than the highest high of the lookback period. 2. Exit by reversing the position.\" (TSL Feb 2003)"],
          "Volatility-to-lookback rule missing in both the stock and the futures columns; not completed (Chande's DBS formula is in the vault as strtw DBS from another book).", None),
        d("atm03_pullback6", "Basic pullback buyer · limit 6% below the close, exit next open", "TSL Mar 2003 (p.50)", "Long", Live, false,
          &["E|\"1. Go long on the next bar at a limit price 6 percent below the current bar's close.\" (TSL Mar 2003)",
            "X|\"2. Exit the next bar on the open (at the market).\"",
            "F|\"3. The system can open a new long position on a bar for which a previous trade was closed.\""],
          "Wealth-Lab test (Richard Tymerski idea). Limit fills at min(open, limit). Book: 18-stock AT portfolio Nov 1992-Nov 2002, 33% of equity per position; run on the indices.",
          c("n/a", "n/a", "winners +2.81%", "1 day", "AT 18-stock portfolio 11/1992-11/2002: +411.63%, PF 1.40, exposure 17.09%, max DD 20.83%")),
        d("atm03_pullback_fut", "Basic pullback buyer (futures) · close 4% down, 5% target/stop", "TSL Futures Mar 2003", "Long", Live, false,
          &["E|\"1. Go long on the next bar at the market open if the close is lower than the previous day's close minus 4 percent.\"",
            "X|\"1. Exit the next bar at the market open if the position has a 5-percent or greater profit. 2. Exit on a 5-percent stop-loss. 3. The stop-loss is in effect starting on the bar that establishes a long position.\""],
          "Profit measured on the close [B]. Stop = entry x 0.95, gap through = open. Book: 20 futures Jan 1993-Nov 2002; run on the indices.",
          c("n/a", "n/a", "winners +7.08%", "7.75 days winners / 5.12 losers", "AT futures portfolio 1/1993-11/2002: +136%, PF 1.18, max DD 28.79%")),
        d("atm03_tpe", "Trend with Pattern Entry (TPE) · stocks, long only", "TSL Apr 2003 (p.44)", "Long", Live, false,
          &["F|\"1. Trend-detection rule: The closing price must be above the 100-bar moving average.\"",
            "E|\"2. Pullback entry rule: When two consecutive bars close below the previous bar's close, enter long on the next bar's open.\"",
            "X|\"3. Stop rule: The stop-loss level is equal to the closing price minus four times the 10-bar average true range (ATR). Keep this stop-loss order active for the duration of the trade.\"",
            "X|\"4. Exit rule: Exit at the market open when the trade closes with a 5-percent profit.\""],
          "Wealth-Lab. Stop = signal-bar close - 4 x Wilder ATR(10), fixed for the trade, working from the bar after entry [B: Wealth-Lab loop], gap = open. 5% profit on the close -> exit next open. One position at a time.",
          c("n/a", "n/a", "winners hold 15.13 days, losers 24.91", "", "AT 18-stock portfolio 12/1992-12/2002: +212.93%, PF 1.22, max DD 30.52%")),
        d("atm03_tpe_ls", "Trend with Pattern Entry (TPE) · futures, long and short", "TSL Futures Apr 2003 (p.54)", "Long/Short", Live, false,
          &["F|Same rules as the stock version; \"Invert the rules to execute short trades.\" (TSL Futures Apr 2003)",
            "E|\"2. Pullback entry rule: When two consecutive bars close below the previous bar's close, enter long on the next bar's open.\" (inverted for shorts)",
            "X|\"3. Stop rule: ... closing price minus four times the 10-bar ATR\"; \"4. Exit rule: Exit at the market open when the trade closes with a 5-percent profit.\""],
          "Short: close < SMA(100), two consecutive higher closes, stop = close + 4 x ATR(10), exit at the open after a close with 5% profit. Book: 20 futures Aug 1993-Nov 2002 (Table 1: 834 trades, 64.75% winners).",
          c("834", "64.75%", "winners +6.00%", "27.2 days winners / 33.4 losers", "AT futures portfolio 8/1993-11/2002: +572.63%, PF 1.34, max DD 16.57%")),
        d("atm03_gap_closer", "Gap closer · buy after a down gap > ATR(20), sell when the gap fills", "TSL May 2003", "Long", Live, false,
          &["E|\"1. Enter long on the open the day after a down gap greater than the 20-bar average true range (ATR) is completed.\"",
            "X|\"2. Place a limit order to sell the position at the low price of the bar that immediately preceded the gap.\"",
            "F|\"3. The system will maintain multiple open long positions. 4. Hold the position indefinitely until the gap is closed and the limit sell is triggered.\" No protective stop (\"no protective stops are included\")"],
          "Gap = low of the prior bar - high of the gap bar (\"a price bar's high is lower than the previous low\"); compared with Wilder ATR(20) on the gap bar [B]. Each gap is its own position (weight p.w = 9% of equity, the book's size); overlapping positions are folded into one trade (legs = entries, scale-out pieces = individual exits; return exact). Limit works from the bar after entry [B].",
          c("62 gaps", "50 closed (80%)", "closed +~11%, open -32%", "~100 days to close", "AT 18-stock portfolio 1/1993-1/2003: +11.94%")),
        d("atm03_ch100_20", "100-20 channel breakout", "TSL Jun 2003 (stocks; futures)", "Long/Short", Live, false,
          &["E|\"1. Enter long on the next bar at the highest 100-day high. 3. Enter short on the next bar at the lowest 100-day low.\"",
            "X|\"2. Exit long on the next bar at the lowest 20-day low. 4. Exit short on the next bar at the highest 20-day high.\" \"(All trades are executed as stop orders.)\""],
          "Wealth-Lab Highest/Lowest include the current bar; orders for the next bar, gap = open. Flat: both entry stops work (open nearer the high = high first). In a position only its exit works. Book: AT 18 stocks Jan 1993-Feb 2003 (+12.61%) and 20 futures Aug 1993-Nov 2002.",
          c("n/a", "n/a", "", "", "AT 18-stock portfolio 1/1993-2/2003: +12.61% (buy & hold +253%)")),
        d("atm03_triple2", "Triple screen SMA system · stocks (2% stop, 5% target)", "TSL Aug 2003 (p.44)", "Long/Short", Live, false,
          &["F|\"Screen 1: The 200-bar SMA must be greater than its value 10 bars ago.\"",
            "F|\"Screen 2: The low price must cross below the 15-bar SMA, the 100-bar SMA must be greater than its value 10 bars ago and 14-bar average directional movement index (ADX) must be greater than its value 10 bars ago.\"",
            "E|\"Screen 3: Buy on a stop order at the previous bar's high plus one cent.\"",
            "X|\"Stop-loss: 2 percent below the basis price\" (= close of the signal bar); \"Profit target: Place a limit order 5 percent above the basis price.\"; \"Trend reversal: Exit the position if the 100-bar SMA is less than its value 10 bars ago.\" Reverse for shorts."],
          "Wealth-Lab, submitted by Richard Ludowise. 'one cent' = p.tick (price units). Buy stop valid for the next bar only [B]. Exits work from the bar after entry; trend-reversal exit at the next open [B: Wealth-Lab at market]; stop before target when both trade in one bar [B]. ADX = Wilder ADX(14).",
          c("n/a", "n/a", "winners hold 3.74 days, losers 2.62", "", "AT 18-stock portfolio 1/1993-3/2003: +101.2%, PF 1.23, max DD 13.95%")),
        d("atm03_triple5", "Triple screen SMA system · futures (5% stop, 5% target)", "TSL Futures Aug 2003 (p.54)", "Long/Short", Live, false,
          &["F|Same screens as the stock version (TSL Futures Aug 2003)", "E|\"Screen 3: Buy on a stop at the previous bar's high value plus one tick.\"",
            "X|\"Stop-loss: Place a stop-loss order 5 percent below the basis price. Profit target: 5 percent above the basis price. Trend reversal: exit if the 100-bar SMA is less than its value 10 bars ago.\""],
          "Same engine as atm03_triple2 with p.stop = 5. Book: 19 futures Jan 1994-Nov 2002 (+207.3%, PF 1.42, max DD 13.67%).",
          c("n/a", "n/a", "winners +4.77%", "22.1 days winners / 19.6 losers", "AT futures portfolio 1/1994-11/2002: +207.30%")),
        d("atm03_blood", "\"Blood in the Street\" · RSI(30) < 40 + TD buy setup 9", "TSL Sep 2003", "Long", Live, false,
          &["E|\"Go long the next bar at the market when both: 1. The 30-bar RSI of the DJIA is below 40. 2. There must be at least nine consecutive bars with closing prices lower than the closes four bars earlier.\"",
            "X|\"Exit the long position on the next bar at the market when either: 1. The open profit is at least 13 percent. 2. There have been nine consecutive bars that have closed higher than the close four bars earlier.\""],
          "RSI = Wilder RSI(30) of the market index; the stock pattern is taken on the same instrument (index = market, instrument swap). Open profit on the close [B]. One position at a time.",
          c("n/a", "n/a", "winners +11.26%", "37.38 days winners / 70.08 losers", "AT 18-stock portfolio 1/1993-3/2003: +101.95%, PF 10.65, max DD 16.23%")),
        d("atm03_mfi", "MFI timing system · buy MFI(14) < 25, add on each new cross, exit MFI > 75", "TSL Oct 2003", "Long", Live, true,
          &["E|\"Enter a new long position on the next bar at the market when the 14-day MFI crosses below 25\" (averaging down: multiple positions)",
            "X|\"Exit all long positions on the next bar at the market when the MFI crosses above 75\"",
            "F|\"Each trade is sized at 5 percent of the current account equity.\" No stop-loss"],
          "MFI with typical price (H+L+C)/3 x volume [B: the column says 'Average Price']. Each cross below 25 adds a leg of weight p.w = 0.05 (book 5%). Needs volume.",
          c("524", "n/a", "winners +12.58%", "47.47 days winners / 100.9 losers", "AT 18-stock portfolio 1/1993-6/2003: +323.73%, PF 2.36, max DD 32.87%")),
        d("atm03_twobar", "Two-bar breakout (Vetri Vel)", "TSL Nov 2003 (stocks p.44; futures p.58)", "Long/Short", Live, false,
          &["E|Long: \"1. Low is less than previous bar's low. 2. High is less than previous bar's high. 3. Close is less than open. 4. If 1 through 3 are true, enter long on a stop on the next bar at this bar's high plus one cent.\"",
            "X|\"5. Exit using a trailing stop at the current bar's low.\"",
            "E|Short: low > previous low, high > previous high, close > open; sell stop next bar at this bar's low minus one cent; trailing stop at the current bar's high"],
          "Entry stop valid for one bar; trailing stop = low (high) of the latest completed bar, from the bar after entry; gap = open. p.tick = one cent / one tick.",
          c("n/a", "n/a", "winners +3.42% (stocks)", "3.71 days winners", "AT 18 stocks 7/1993-7/2003: +19.70%; futures 9/1993-12/2002: +45.39% (1 contract)")),
        d("atm03_cmo_srsi", "CMO / StochRSI system · scale in at StochRSI(14) = 0 and CMO(20) < -28", "TSL Dec 2003 (p.48)", "Long", Live, false,
          &["E|\"1. The 14-bar StochRSI must be zero. 2. The 20-bar CMO must be below -28. 3. When these first two conditions are met, buy the next bar at the market. 4. Continue to scale into the position if rules 1 and 2 are both true on subsequent bars, and the closing price is lower than the last entry price.\"",
            "X|\"1. The two-bar StochRSI must be above 90. 2. The 20-bar CMO must be above 20. 3. When rules 1 and 2 are true, exit all positions on the next bar at the market.\"",
            "F|\"Each trade was sized to equal 10 percent of the current account equity.\" Modified version: 3.33% each, \"No more than three trades\" (p.max_legs = 3, p.w = 0.0333)"],
          "StochRSI(n) = (RSI(n) - lowest RSI(n) of n bars)/(range) x 100 (Chande); CMO = 100 x (up - down)/(up + down) of n close changes. Leg weight p.w = 0.10 (book). p.max_legs = 0 = unlimited (book default).",
          c("n/a", "75.71%", "winners +9.43%", "26.29 days winners / 59.61 losers", "AT 18-stock portfolio 9/1993-9/2003: +789.89%, PF 2.54, max DD 26.01%")),
        d("atm03_or_rev", "Outside reversal days · long exit after 3 bars, short after 47", "Trading Strategies Apr-May 2003 (Andrews)", "Long/Short", Live, false,
          &["F|Bullish: \"high > high[1] and low < low[1] and (high - close)/(high - low) < .50 and close > open and close > close[2] and 10-day EMA < 10-day EMA[1] and low < 10-day EMA\"; bearish = inverted (Apr 2003 p.42; May 2003 p.42)",
            "E|\"Trades were entered on the open the day after the outside day.\"",
            "X|Time exits BarToExitOn(L) = 3, BarToExitOn(S) = 47: \"a three-day exit for long trades and a 47-day exit for short trades\" (Fig. 2, SPY 3/10/2000-3/11/2003)"],
          "TradeStation (inputs BarToExitOn = TradeStation's built-in 'TimeExit (Bars)': exit next bar at market when BarsSinceEntry = N, i.e. at the open of bar entry + N + 1). EMA = XAverage(10). Opposite signal reverses (TradeStation). The 3/47 lengths are the article's optimised values (it also reports 3/3-4 day variants).",
          c("13", "69% (9 of 13)", "$355.46/100 sh", "winners 21 bars", "SPY 3/10/2000-3/11/2003, 100 shares, no costs: net $4,621 (long 8 trades $862, short 5 trades $3,759)")),
        d("atm03_ob_cont", "Outside bar continuation · stop-and-reverse", "Trading Strategies Sep 2003 (Andrews & Etzkorn)", "Long/Short", Live, false,
          &["F|\"high > high[1] and low < low[1] and (high - close)/(high - low) < .50 and close > open and close > close[2] and close > 20-day moving average and (high[1] > 20-day moving average[1] or high[2] > 20-day moving average[2])\"; invert for bearish",
            "E|\"Buy the next day on the open.\" (sell short the next day on the open after a bearish bar)",
            "X|Positions held \"until an offsetting bearish outside bar produced a sell signal\" (Table 1 test, no stop-loss or other exit rules)"],
          "Table 1 version (signals only). 20-day MA = SMA(20) [B]. The article's second version (stop-loss on the close and a 3% profit exit) is atm03_ob_cont_stops (not testable). SPY 3 years 2 months to 7/9/2002.",
          c("n/a", "n/a", "net $34,164 (long -$13,577, short +$47,741)", "53.07 bars", "SPY ~5/2000-7/9/2002")),
        d("atm03_ob_cont_stops", "Outside bar continuation with stop and 3% profit exit", "Trading Strategies Sep 2003", "Long/Short",
          NotTestable("the exit rules leave open whether 'dropped below' means the low or the close, whether the 3% profit is taken during the first five bars, and what exits a trade that never reaches 3% after the stop period"), false,
          &["X|\"exits (win or lose) long trades if the first two bars after the entry bar dropped below both the previous bar's low and the outside bar's low by more than .05 percent. For the next three bars, long trades are exited simply if price moves below the previous day's low by more than .05 percent ... All these trades are executed on the close. To close trades that are still open (and thus, profitable) after this initial period, we will exit when the position has built a 3-percent profit\""],
          "Not completed (Table 2: net $12,649, long -$301, short +$12,950).", None),
        d("atm03_spike", "Spike-reversal bars · S&P E-Mini · long 3 days, short 5 days", "Futures Trading Strategies Nov 2003", "Long/Short", Live, false,
          &["F|High bar: high at least 1.5% above the previous high and close in the lower half of the bar; low bar: low at least 1.5% below the previous low and close in the upper half (text p.53; the printed formulas carry sign typos)",
            "E|\"buying or selling the S&P 500 E-Mini on the close of a spike-reversal bar\"",
            "X|\"exiting long trades three days later and short trades five days later\" (on the close)"],
          "Prose definitions win over the mistyped formulas. One position at a time: a signal during an open trade is ignored [B]. Book: continuous E-Mini Aug 1998-Aug 2003, Table 3.",
          c("43", "67.44%", "+1.51% (17.52 pts)", "3 / 5 days", "S&P 500 E-Mini 8/1998-8/2003")),
        d("atm03_memorial", "Memorial Day trade · buy the day before, sell 4 days after", "Market History Jun 2003 (Bukey)", "Long", Live, false,
          &["E|\"go long one day before\" Memorial Day (close of the last session before the holiday)", "X|\"sell four days after the holiday\" (close of the 4th session after)"],
          "Memorial Day = last Monday of May (May 30 before 1971). India: the same US date, sessions counted before/after it (instrument swap). Book: S&P 500 1980-2002.",
          c("23", "69.57%", "+1.08%", "4 days", "S&P 500 1980-2002 (wins avg +2.04%, losses -1.10%)")),
        d("atm03_july4", "Fourth of July trade · buy 6 days before, sell 1 day before", "Market History Jul 2003 (Bukey)", "Long", Live, false,
          &["E|\"Go long six days before the Fourth of July\" (close of the 6th session before)", "X|\"exit one day before the Fourth of July\" (close of the last session before)"],
          "Observed holiday (Sat -> Fri, Sun -> Mon). Book: S&P 500 1978-2002, the 'Exit -5' column.",
          c("25", "68%", "+0.53%", "5 days", "S&P 500 1978-2002")),
        d("atm03_harris_delay", "Delay pattern (APS, SPY) · 7% target / 7% stop, one-bar delay", "Trading Strategies Feb 2003 (Harris)", "Long", Live, false,
          &["F|MetaStock code (Fig. 4): \"(Ref(h,0) > Ref(c,0)) and (Ref(c,0) > Ref(h,-1)) and (Ref(h,-1) > Ref(h,-2)) and (Ref(h,-2) > Ref(l,0)) and (Ref(l,0) > Ref(l,-1)) and (Ref(l,-1) > Ref(l,-2))\"",
            "E|\"LONG, %, TARGET: 7, STOP: 7, ENTRY PRICE: OPEN, DELAY: 2\" (MetaStock delay 2 = one bar after tomorrow): enter at the open two bars after the pattern bar",
            "X|\"the trade was exited as soon as price had moved up or down 7 percent from the entry price\""],
          "Target and stop work from the entry bar; stop first if both in one bar; gap = open [B]. One position at a time. Book: SPY Jan 1990 (SPY starts 1993)-Jun 2002.",
          c("28", "67.86%", "", "", "SPY to June 2002, delay = 1 bar")),
        d("atm03_meyers", "Dennis Meyers intraday systems (Next-Bar Forecast, Jul/Nov 2003)", "Advanced Strategies May/Jul/Nov 2003", "Long/Short",
          NeedsData("five-minute bars (QQQ, IBM, E-mini) and walk-forward re-optimised parameters"), false,
          &["E|Next-Bar Forecast: Pf(N) = [2/N(N-1)] sum (3t-N-2) p(t); buy when Pf rises pctup from its low, sell when it falls pctdn from its high; no trades before 10:00, flat 5 minutes before the close (May 2003)"],
          "Five-minute data and per-window optimised N/pctup/pctdn; not coded.", None),
        d("atm03_intraday_misc", "Intraday columns (Morning Gap Reversal, McNutt breakout, up/down volume slope)", "May/Jun 2003", "Long/Short",
          NeedsData("one/five/ten-minute bars and intraday NYSE up/down volume"), false,
          &["E|Babcock MGR (May 2003): fade the opening gap 9:30-9:42 a.m.; Shirley (Jun 2003): 10-minute SPY breakout; Hartle (May 2003): five-minute up/down volume slope difference"],
          "Intraday data not in the vault.", None),
        d("atm03_chart_patterns", "Chart-pattern columns (rectangles, H&S, broadening tops, Gartley 222, horn/pipe/3RV, W-bottom, RSI trendlines)", "Bierovic, Bulkowski, Conway, Hartle 2003", "Long/Short",
          NotTestable("pattern boundaries, pivots and trendlines are drawn by eye; no objective definition is given"), false,
          &["E|e.g. Bierovic Jan 2003: \"buy on the first day that reaches the previous day's high\" after a rectangle chosen on the chart"],
          "Discretionary.", None),
        d("atm03_bodri", "Short-term channel breakout with filters (Bodri)", "Futures Strategies Feb 2003", "Long/Short",
          NotTestable("the stop is a $1,500 futures-dollar stop and the channel length is only given as a tested range; a dollar stop cannot be mapped to an index without rescaling"), false,
          &["E|\"If C<C[1] then Buy at Highest(H, ChLen) stop; If C>C[1] then Sell at Lowest(L, ChLen) stop\"; exit on the first profitable open; $1,500 initial stop-loss"],
          "Not completed.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v: &'static str = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some(match id {
        "atm03_bb60" | "atm03_dyn_breakout" | "atm03_ch100_20" | "atm03_bodri" => ("atm03_trend", "Active Trader 2003 · trend breakouts", v),
        "atm03_pullback6" | "atm03_pullback_fut" | "atm03_gap_closer" | "atm03_blood" | "atm03_spike" => ("atm03_counter", "Active Trader 2003 · pullbacks & reversals", v),
        "atm03_tpe" | "atm03_tpe_ls" => ("atm03_tpe", "Trend with Pattern Entry", if id == "atm03_tpe" { "Stocks, long" } else { "Futures, long/short" }),
        "atm03_triple2" | "atm03_triple5" => ("atm03_triple", "Triple screen SMA", if id == "atm03_triple2" { "2% stop" } else { "5% stop" }),
        "atm03_mfi" | "atm03_cmo_srsi" => ("atm03_osc", "Active Trader 2003 · oscillator scale-ins", v),
        "atm03_twobar" | "atm03_harris_delay" => ("atm03_patterns", "Active Trader 2003 · bar patterns", v),
        "atm03_or_rev" | "atm03_ob_cont" | "atm03_ob_cont_stops" => ("atm03_outside", "Outside bars (Andrews)", v),
        "atm03_memorial" | "atm03_july4" => ("atm03_seasonal", "Market History holidays", v),
        "atm03_meyers" | "atm03_intraday_misc" | "atm03_chart_patterns" => ("atm03_other", "Active Trader 2003 · not testable", v),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let tick = || p("tick", "'One cent' / one tick added to the stop (price units)", 0.01, 0., 1., 0.01);
    Some(match id {
        "atm03_bb60" => vec![p("n", "Bollinger lookback", 60., 10., 200., 5.), p("k", "Standard deviations", 2., 0.5, 4., 0.25)],
        "atm03_pullback6" => vec![p("pct", "Limit % below the close", 6., 1., 15., 0.5)],
        "atm03_pullback_fut" => vec![p("drop", "Close-to-close drop %", 4., 1., 10., 0.5), p("target", "Profit % (on the close)", 5., 1., 15., 0.5), p("stop", "Stop-loss %", 5., 1., 15., 0.5)],
        "atm03_tpe" | "atm03_tpe_ls" => vec![p("ma", "Trend SMA", 100., 25., 200., 25.), p("atr_n", "ATR length", 10., 5., 30., 1.), p("atr_k", "ATR multiple", 4., 1., 8., 0.5), p("target", "Profit % (on the close)", 5., 1., 15., 0.5)],
        "atm03_gap_closer" => vec![p("atr_n", "ATR length", 20., 5., 50., 1.), p("w", "Position weight (fraction of equity)", 0.09, 0.01, 1., 0.01)],
        "atm03_ch100_20" => vec![p("entry", "Entry channel (days)", 100., 20., 250., 5.), p("exit", "Exit channel (days)", 20., 5., 100., 5.)],
        "atm03_triple2" | "atm03_triple5" => vec![p("stop", "Stop-loss % from basis", if id == "atm03_triple2" { 2. } else { 5. }, 1., 10., 0.5), p("target", "Profit target % from basis", 5., 1., 15., 0.5), tick()],
        "atm03_blood" => vec![p("rsi_n", "RSI length", 30., 5., 50., 1.), p("rsi_lvl", "RSI below", 40., 20., 50., 1.), p("count", "Consecutive closes vs 4 bars earlier", 9., 5., 13., 1.), p("profit", "Exit open profit %", 13., 5., 30., 1.)],
        "atm03_mfi" => vec![p("n", "MFI length", 14., 5., 30., 1.), p("lo", "Oversold level", 25., 10., 40., 1.), p("hi", "Overbought level", 75., 60., 90., 1.), p("w", "Leg weight (fraction of equity)", 0.05, 0.01, 1., 0.01)],
        "atm03_twobar" => vec![tick()],
        "atm03_cmo_srsi" => vec![p("srsi_n", "Entry StochRSI length", 14., 5., 30., 1.), p("cmo_n", "CMO length", 20., 5., 40., 1.), p("cmo_in", "Entry CMO below", -28., -60., 0., 1.), p("srsi_x", "Exit StochRSI length", 2., 2., 14., 1.), p("srsi_hi", "Exit StochRSI above", 90., 50., 100., 1.), p("cmo_out", "Exit CMO above", 20., 0., 60., 1.), p("w", "Leg weight (fraction of equity)", 0.10, 0.01, 1., 0.01), p("max_legs", "Maximum legs (0 = unlimited)", 0., 0., 20., 1.)],
        "atm03_or_rev" => vec![p("ema", "EMA length", 10., 3., 30., 1.), p("xl", "BarToExitOn long", 3., 1., 60., 1.), p("xs", "BarToExitOn short", 47., 1., 60., 1.)],
        "atm03_ob_cont" => vec![p("ma", "Moving average length", 20., 5., 50., 1.)],
        "atm03_spike" => vec![p("pct", "Spike size %", 1.5, 0.5, 5., 0.25), p("xl", "Long exit (days)", 3., 1., 10., 1.), p("xs", "Short exit (days)", 5., 1., 10., 1.)],
        "atm03_memorial" => vec![p("after", "Exit: sessions after the holiday", 4., 1., 10., 1.)],
        "atm03_july4" => vec![p("before", "Entry: sessions before the holiday", 6., 2., 10., 1.)],
        "atm03_harris_delay" => vec![p("target", "Profit target %", 7., 1., 20., 0.5), p("stop", "Stop-loss %", 7., 1., 20., 0.5), p("delay", "Delay (bars after the pattern bar before the entry bar)", 1., 0., 3., 1.)],
        "atm03_dyn_breakout" | "atm03_ob_cont_stops" | "atm03_meyers" | "atm03_intraday_misc" | "atm03_chart_patterns" | "atm03_bodri" => vec![],
        _ => return None,
    })
}

// ------------------------------------------------------------------ run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if !checked(p, &schema) || from > to || to >= n { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).max(1.) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    Some(match id {
        "atm03_bb60" => {
            let (m, sd, k) = (sma(c, u("n")), stdev(c, u("n")), g("k"));
            by_open(x, from, to, |i, _, _| {
                if m[i].is_nan() { None } else if c[i] > m[i] + k * sd[i] { Some(1) } else if c[i] < m[i] - k * sd[i] { Some(-1) } else { None }
            })
        }
        "atm03_pullback6" => {
            let f = 1. - g("pct") / 100.;
            let mut out = vec![];
            for i in from..to {
                let j = i + 1;
                if let Some(px) = lim_dn(x, j, c[i] * f) {
                    let t = new_trade(1., j, px, "limit");
                    if j + 1 <= to { out.push(done(t, j + 1, o[j + 1], j, "exit next open")); } else { finish(&mut out, Some(t), x, to); }
                }
            }
            out
        }
        "atm03_pullback_fut" => {
            let (dr, tg, st) = (1. - g("drop") / 100., 1. + g("target") / 100., 1. - g("stop") / 100.);
            let (mut out, mut cur, mut pend) = (vec![], None::<Trade>, false);
            for j in from.max(2)..=to {
                if pend { if let Some(t) = cur.take() { out.push(done(t, j, o[j], j - 1, "profit >= 5% on the close: exit at the open")); } pend = false; }
                if cur.is_none() && c[j - 1] < c[j - 2] * dr { cur = Some(new_trade(1., j, o[j], "open")); }
                if let Some(t) = cur.take() {
                    let e = t.legs[0].px;
                    if let Some(px) = stop_dn(x, j, e * st) { out.push(done(t, j, px, j, "5% stop")); continue; }
                    if c[j] >= e * tg && j < to { pend = true; }
                    cur = Some(t);
                }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atm03_tpe" | "atm03_tpe_ls" => {
            let shorts = id == "atm03_tpe_ls";
            let (ma, a, k, tg) = (sma(c, u("ma")), atr(x.r, u("atr_n")), g("atr_k"), g("target") / 100.);
            let (mut out, mut cur, mut stop) = (vec![], None::<Trade>, 0.);
            let mut i = from.max(2);
            while i < to {
                if let Some(t) = cur.take() {
                    let (d, e) = (t.dir, t.legs[0].px);
                    // stop works from the bar after the entry bar
                    let hit = if i > t.legs[0].i { if d > 0. { stop_dn(x, i, stop) } else { stop_up(x, i, stop) } } else { None };
                    if let Some(px) = hit { out.push(done(t, i, px, i, "ATR stop")); }
                    else if d * (c[i] / e - 1.) >= tg { out.push(done(t, i + 1, o[i + 1], i, "closed with 5% profit: exit next open")); i += 1; continue; }
                    else { cur = Some(t); i += 1; continue; }
                }
                if !ma[i].is_nan() && !a[i].is_nan() {
                    if c[i] > ma[i] && c[i] < c[i - 1] && c[i - 1] < c[i - 2] { cur = Some(new_trade(1., i + 1, o[i + 1], "open")); stop = c[i] - k * a[i]; }
                    else if shorts && c[i] < ma[i] && c[i] > c[i - 1] && c[i - 1] > c[i - 2] { cur = Some(new_trade(-1., i + 1, o[i + 1], "open")); stop = c[i] + k * a[i]; }
                }
                i += 1;
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atm03_gap_closer" => {
            let (a, w) = (atr(x.r, u("atr_n")), g("w"));
            let mut ps = vec![];
            for i in from.max(1)..to {
                if !(h[i] < l[i - 1] && l[i - 1] - h[i] > a[i]) { continue; }
                let (e, tgt) = (i + 1, l[i - 1]);
                let mut pos = Pos { dir: 1., i: e, px: o[e], w, xi: to, xpx: c[to], why: "open".into(), open: true };
                for j in e + 1..=to { if let Some(px) = lim_up(x, j, tgt) { pos.xi = j; pos.xpx = px; pos.why = "gap closed (limit)".into(); pos.open = false; break; } }
                ps.push(pos);
            }
            fold(ps)
        }
        "atm03_ch100_20" => {
            let (ne, nx) = (u("entry"), u("exit"));
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for j in from.max(1)..=to {
                let i = j - 1;
                match cur.take() {
                    Some(t) => {
                        let hit = if t.dir > 0. { stop_dn(x, j, ll(l, i, nx)) } else { stop_up(x, j, hh(h, i, nx)) };
                        match hit { Some(px) => out.push(done(t, j, px, j, "exit channel stop")), None => cur = Some(t) }
                    }
                    None => {
                        let (b, s) = (stop_up(x, j, hh(h, i, ne)), stop_dn(x, j, ll(l, i, ne)));
                        let go = match (b, s) { (Some(_), Some(_)) => if h[j] - o[j] <= o[j] - l[j] { 1. } else { -1. }, (Some(_), None) => 1., (None, Some(_)) => -1., _ => 0. };
                        if go > 0. { cur = Some(new_trade(1., j, b.unwrap(), "stop")); } else if go < 0. { cur = Some(new_trade(-1., j, s.unwrap(), "stop")); }
                    }
                }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atm03_triple2" | "atm03_triple5" => {
            let (s200, s100, s15) = (sma(c, 200), sma(c, 100), sma(c, 15));
            let adx = dmi(x.r, 14).1;
            let (sp, tp, tick) = (g("stop") / 100., g("target") / 100., g("tick"));
            let (mut out, mut cur, mut basis) = (vec![], None::<Trade>, 0.);
            for j in from.max(11)..=to {
                let i = j - 1;
                if let Some(t) = cur.take() {
                    let d = t.dir;
                    let ex = if i >= 10 && (d > 0. && s100[i] < s100[i - 10] || d < 0. && s100[i] > s100[i - 10]) { Some((o[j], "trend reversal: next open")) }
                        else if d > 0. { stop_dn(x, j, basis * (1. - sp)).map(|v| (v, "stop-loss")).or(lim_up(x, j, basis * (1. + tp)).map(|v| (v, "profit target"))) }
                        else { stop_up(x, j, basis * (1. + sp)).map(|v| (v, "stop-loss")).or(lim_dn(x, j, basis * (1. - tp)).map(|v| (v, "profit target"))) };
                    match ex { Some((px, why)) => { out.push(done(t, j, px, if why.starts_with("trend") { i } else { j }, why)); }, None => { cur = Some(t); } }
                    continue;
                }
                if i < 11 || s200[i - 10].is_nan() || s100[i - 10].is_nan() || adx[i - 10].is_nan() || s15[i - 1].is_nan() { continue; }
                let adx_up = adx[i] > adx[i - 10];
                if s200[i] > s200[i - 10] && l[i] < s15[i] && l[i - 1] >= s15[i - 1] && s100[i] > s100[i - 10] && adx_up {
                    if let Some(px) = stop_up(x, j, h[i] + tick) { cur = Some(new_trade(1., j, px, "stop")); basis = c[i]; }
                } else if s200[i] < s200[i - 10] && h[i] > s15[i] && h[i - 1] <= s15[i - 1] && s100[i] < s100[i - 10] && adx_up {
                    if let Some(px) = stop_dn(x, j, l[i] - tick) { cur = Some(new_trade(-1., j, px, "stop")); basis = c[i]; }
                }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atm03_blood" => {
            let (r, lvl, cnt, pr) = (rsi(c, u("rsi_n")), g("rsi_lvl"), u("count"), g("profit") / 100.);
            let mut dn = vec![0usize; n];
            let mut up = vec![0usize; n];
            for i in 4..n { dn[i] = if c[i] < c[i - 4] { dn[i - 1] + 1 } else { 0 }; up[i] = if c[i] > c[i - 4] { up[i - 1] + 1 } else { 0 }; }
            by_open(x, from, to, |i, pos, t| {
                if pos == 0 { if r[i] < lvl && dn[i] >= cnt { Some(1) } else { None } }
                else { let e = t.unwrap().legs[0].px; if c[i] / e - 1. >= pr || up[i] >= cnt { Some(0) } else { None } }
            })
        }
        "atm03_mfi" | "atm03_cmo_srsi" => {
            let is_mfi = id == "atm03_mfi";
            let w = g("w");
            let (ent, ext): (Vec<bool>, Vec<bool>) = if is_mfi {
                let (m, lo, hi) = (mfi(x, u("n")), g("lo"), g("hi"));
                ((0..n).map(|i| i > 0 && m[i] < lo && m[i - 1] >= lo).collect(), (0..n).map(|i| i > 0 && m[i] > hi && m[i - 1] <= hi).collect())
            } else {
                let (s1, s2, cm) = (stoch_rsi(c, u("srsi_n")), stoch_rsi(c, u("srsi_x")), cmo(c, u("cmo_n")));
                let (ci, co, sh) = (g("cmo_in"), g("cmo_out"), g("srsi_hi"));
                ((0..n).map(|i| s1[i] == 0. && cm[i] < ci).collect(), (0..n).map(|i| s2[i] > sh && cm[i] > co).collect())
            };
            let maxl = g("max_legs") as usize;
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for i in from..to {
                let j = i + 1;
                if let Some(t) = cur.take() {
                    if ext[i] { out.push(done(t, j, o[j], i, "exit all at next open")); continue; }
                    let mut t = t;
                    let last = t.legs.last().unwrap().px;
                    if ent[i] && (is_mfi || c[i] < last) && (maxl == 0 || t.legs.len() < maxl) { t.legs.push(Leg { i: j, px: o[j], w }); }
                    cur = Some(t);
                } else if ent[i] {
                    let mut t = new_trade(1., j, o[j], "open");
                    t.legs[0].w = w;
                    cur = Some(t);
                }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atm03_twobar" => {
            let tick = g("tick");
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for j in from.max(2)..=to {
                let i = j - 1;
                if let Some(t) = cur.take() {
                    let hit = if j > t.legs[0].i { if t.dir > 0. { stop_dn(x, j, l[i]) } else { stop_up(x, j, h[i]) } } else { None };
                    match hit { Some(px) => out.push(done(t, j, px, j, "trailing stop")), None => cur = Some(t) }
                    continue;
                }
                if l[i] < l[i - 1] && h[i] < h[i - 1] && c[i] < o[i] { if let Some(px) = stop_up(x, j, h[i] + tick) { cur = Some(new_trade(1., j, px, "stop")); } }
                else if l[i] > l[i - 1] && h[i] > h[i - 1] && c[i] > o[i] { if let Some(px) = stop_dn(x, j, l[i] - tick) { cur = Some(new_trade(-1., j, px, "stop")); } }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atm03_or_rev" => {
            let e = xavg(c, u("ema"));
            let (xl, xs) = (u("xl"), u("xs"));
            let bull = |i: usize| i >= 2 && h[i] > h[i - 1] && l[i] < l[i - 1] && h[i] > l[i] && (h[i] - c[i]) / (h[i] - l[i]) < 0.5 && c[i] > o[i] && c[i] > c[i - 2] && e[i] < e[i - 1] && l[i] < e[i];
            let bear = |i: usize| i >= 2 && h[i] > h[i - 1] && l[i] < l[i - 1] && h[i] > l[i] && (c[i] - l[i]) / (h[i] - l[i]) < 0.5 && c[i] < o[i] && c[i] < c[i - 2] && e[i] > e[i - 1] && h[i] > e[i];
            by_open(x, from, to, |i, pos, t| {
                if bull(i) && pos != 1 { return Some(1); }
                if bear(i) && pos != -1 { return Some(-1); }
                let t = t?;
                let bse = i - t.legs[0].i;
                if pos == 1 && bse == xl || pos == -1 && bse == xs { Some(0) } else { None }
            })
        }
        "atm03_ob_cont" => {
            let m = sma(c, u("ma"));
            by_open(x, from, to, |i, _, _| {
                if i < 2 || m[i - 2].is_nan() || !(h[i] > l[i]) || !(h[i] > h[i - 1] && l[i] < l[i - 1]) { return None; }
                let q = (h[i] - c[i]) / (h[i] - l[i]);
                if q < 0.5 && c[i] > o[i] && c[i] > c[i - 2] && c[i] > m[i] && (h[i - 1] > m[i - 1] || h[i - 2] > m[i - 2]) { Some(1) }
                else if (c[i] - l[i]) / (h[i] - l[i]) < 0.5 && c[i] < o[i] && c[i] < c[i - 2] && c[i] < m[i] && (l[i - 1] < m[i - 1] || l[i - 2] < m[i - 2]) { Some(-1) }
                else { None }
            })
        }
        "atm03_spike" => {
            let (pc, xl, xs) = (g("pct") / 100., u("xl"), u("xs"));
            let mut out = vec![];
            let mut i = from.max(1);
            while i <= to {
                let rng = h[i] - l[i];
                let dir = if rng > 0. && (h[i] - h[i - 1]) / h[i - 1] >= pc && (h[i] - c[i]) / rng > 0.5 { -1. }
                    else if rng > 0. && (l[i - 1] - l[i]) / l[i - 1] >= pc && (h[i] - c[i]) / rng < 0.5 { 1. } else { 0. };
                if dir == 0. { i += 1; continue; }
                let xi = i + if dir > 0. { xl } else { xs };
                let t = new_trade(dir, i, c[i], "close");
                if xi > to { finish(&mut out, Some(t), x, to); break; }
                out.push(done(t, xi, c[xi], xi, "time exit at the close"));
                i = xi; // a new spike on the exit day is taken at that close
            }
            out
        }
        "atm03_memorial" => holiday_trades(x, from, to, memorial_day, 1, g("after") as i64),
        "atm03_july4" => holiday_trades(x, from, to, july4_observed, u("before"), -1),
        "atm03_harris_delay" => {
            let (tg, st, dl) = (g("target") / 100., g("stop") / 100., g("delay") as usize);
            let pat = |i: usize| i >= 2 && h[i] > c[i] && c[i] > h[i - 1] && h[i - 1] > h[i - 2] && h[i - 2] > l[i] && l[i] > l[i - 1] && l[i - 1] > l[i - 2];
            let mut out = vec![];
            let mut i = from;
            while i + dl + 1 <= to {
                if !pat(i) { i += 1; continue; }
                let e = i + dl + 1;
                let ep = o[e];
                let mut t = Some(new_trade(1., e, ep, "open"));
                let mut j = e;
                while j <= to {
                    let hit = stop_dn(x, j, ep * (1. - st)).map(|v| (v, "7% stop")).or(lim_up(x, j, ep * (1. + tg)).map(|v| (v, "7% target")));
                    if let Some((px, why)) = hit { out.push(done(t.take().unwrap(), j, px, j, why)); break; }
                    j += 1;
                }
                if t.is_some() { finish(&mut out, t, x, to); break; }
                i = (i + 1).max(j.saturating_sub(dl)); // next entry bar is after the exit bar
            }
            out
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn holidays_and_wma() {
        assert_eq!(weekday(civil(2003, 5, 26)), 1);
        assert_eq!(memorial_day(2003), civil(2003, 5, 26));
        assert_eq!(july4_observed(2004), civil(2004, 7, 5));
        let w = wma(&[1., 2., 3.], 3);
        assert!((w[2] - 14. / 6.).abs() < 1e-12);
    }
    #[test]
    fn fold_exact_return() {
        let ps = vec![Pos { dir: 1., i: 1, px: 100., w: 0.1, xi: 5, xpx: 110., why: "a".into(), open: false },
                      Pos { dir: 1., i: 3, px: 90., w: 0.1, xi: 8, xpx: 99., why: "b".into(), open: false }];
        let t = fold(ps);
        assert_eq!(t.len(), 1);
        assert!((t[0].ret(0.) - (0.1 * 0.1 + 0.1 * 0.1)).abs() < 1e-12);
    }
    #[test]
    fn pullback_limit_hand_checked() {
        let b = [(100., 101., 99., 100.), (99., 99., 93., 95.), (96., 97., 95., 96.)];
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("atm03_pullback6", &x, 0, 2, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1, 94., 2, 96.));
    }
}
