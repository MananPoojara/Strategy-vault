//! Options Trader / Futures & Options Trader magazine (Active Trader), the "Options Strategy Lab" (2005-06) and "Options
//! Trading System Lab" (2009) columns by Steve Lentz and Jim Graham of OptionVue. Research share Trading Knowledge I/@Newsletters/
//! Futures & Options Trader Magazine 2005 - 2010 (futuresandoptionstrader.com); text authors/batch-i/i1w/full/fot0NN.txt
//! (pdftotext -layout of each issue). Book id `fotol`. Lane K29 (3 Nov 2026): the held option labs whose rules are complete and
//! need no fixed point amount or model value (authors/titles-1030/K29.tsv); independent check authors/titles-1030/K29w/k29_ref.py.
//! Every rule trades real option contracts from the vault's chains (crate::options): fills only on a traded row, no price is
//! modelled or filled in, a missing strike or day is no trade. The labs priced S&P 500 futures / QQQQ options with OptionVue
//! (bid/ask, else theoretical prices) in 2001-09; the runs here are a declared application on SPY/QQQ/IWM and NIFTY/BANKNIFTY options.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::options::{self as opt, Contract, Exit};

pub const BOOK: &str = "fotol";

macro_rules! common { () => { "Options phase 2: real option prices, no pricing model. A fill needs a row that traded that day (US SPY/QQQ/IWM: volume > 0 and a two-sided quote, at the 16:00 mid (bid+ask)/2 [labelled: no bid/ask edge]; India NIFTY 50 / NIFTY BANK: Contracts > 0, at the traded close or open). A contract that is not listed, or did not trade, on the entry day means no trade that day. An untraded exit day exits on the next traded close; a contract held to its last trading day settles at intrinsic value on the underlying's close. Open positions are valued daily at their own price (traded close / mid; India settlement price on untraded days, labelled). Each position is one option per unit of the underlying, sized at the account equity; return = option points / underlying close at entry. Costs: the vault cost setting per side (0 by default)." } }

const BB: &[&str] = &[
    "F|\"1. Price closes above the upper Bollinger Band. 2. The Bollinger Bands are narrow (i.e., BandWidth is in the lowest 10 percent of its 200-day range). 3. Daily volume is above its 10-day average.\" (Options Trader Nov 2005, Options Strategy Lab p.34); \"the previous day's BandWidth must be in the lowest 10 percent of the 200-day BandWidth range\" (p.34)",
    "E|\"Go long (buy 10 ATM calls)\" on an upper-band close; \"Sell short (buy 10 ATM puts)\" when price closes below the lower band with the same two filters; \"go long at-the-money (ATM) calls or puts, depending upon the breakout's direction, before the market close\" (p.34): bought at the signal day's close",
    "X|\"Exit the position two days later at the close.\" (p.34; Table 1: 5/21/01 -> 5/23/01)",
];
const BB_NOTE: &str = concat!("[Declared readings] Bollinger Bands = 20-day SMA +/- 2 population standard deviations of closes (the article: 'two standard deviations above and below a simple moving average', default settings); BandWidth = (upper - lower) / middle (printed). 'Lowest 10 percent of its 200-day range' = yesterday's BandWidth <= the lowest + 10% of (highest - lowest) of the 200 BandWidths ending yesterday. 10-day average volume includes today. ATM = the listed strike nearest the close (tie: lower); the article names no expiry, so the nearest monthly expiry whose last trading day is after the exit day is bought. One position at a time (the article's table never overlaps). Needs the underlying's volume: SPY, QQQ and IWM (the NSE index files carry no volume). Book: QQQQ options 5/2001-9/2005, 19 trades, 6 winners, -$2,400 (10 contracts, no commissions); our QQQ options begin in 2011, so the book's trades cannot be replayed. ", common!());
const CC: &[&str] = &[
    "E|\"The traditional covered call purchased 100 shares of QQQQ on Jan. 2, 2001 and held those shares until the end of the test period.\" (Options Trader Jun 2006, Options Strategy Lab p.24): long the underlying from the first day of the run",
    "E|\"Sell the closest ATM call option available in the front month. ATM is defined as the call with the highest time premium.\" (p.24, Trade rules 2)",
    "X|\"Hold the short call until expiration day. On expiration day, buy it back and sell the ATM call in the next month.\" (p.24): bought back at the close of its last trading day and the next front-month ATM call sold at that close; each monthly cycle is one trade",
];
const CC_NOTE: &str = concat!("[Declared readings] Front month = the nearest monthly expiry whose last trading day is after today. Time premium = the call's fill price minus its intrinsic value on the underlying close; only calls of that expiry that traded today are compared (an untraded strike has no price), ties go to the strike nearest the close, then the lower. No call traded: the next day is tried. A call that did not trade on its expiration day settles at intrinsic value. Return per cycle = (underlying points + call premium kept) / underlying close at entry; no dividends (price-only closes). Book: QQQQ Jan 2, 2001 - Apr 20, 2006, +7.58% (1.44% a year); our QQQ options begin in 2011. The LEAPS version of the same lab is not coded: the vault's chains hold the three nearest monthly expiries only. ", common!());
const AC: &[&str] = &[
    "E|\"1. Buy 500 shares of QQQQ on Jan. 3, 2007 and sell 5 ATM February 43 call options (both ATM and adjusted strategies start with same position).\" (Futures & Options Trader Feb 2009, Options Trading System Lab p.23): long the underlying from the first day of the run, the first call sold ATM",
    "X|\"2. Rolling: a. Buy back calls when time premium declines to $0.20 or less at close. b. Sell calls in the first month with more than 21 days to expiration.\" (p.23)",
    "E|\"Adjusted covered calls: 1. If the underlying's price is above the 50- and 200-day SMAs, sell calls 5 percent OTM. 2. If the underlying's price is below the 50- and 200-day SMAs, sell calls 5 percent ITM. 3. If the underlying's price is between the 50- and 200-day SMAs, sell ATM calls.\" (p.23-24)",
    "F|\"ATM is defined as the call with the highest time premium. Each call is sold for at least $0.40 in time premium. If this is not available at the selected strike, the system moves back toward the underlying's price and sells the first strike to meet this requirement.\" (p.24, Test details)",
];
const AC_NOTE: &str = concat!("[Declared readings] Time premium = the call's fill price minus its intrinsic value on the underlying close; a roll is decided only on a day the held call traded. 5 percent OTM / ITM = the listed strike nearest 1.05 / 0.95 x the close; 'moves back toward the underlying's price' = the next listed strikes toward the strike nearest the close, up to and including it; a strike that did not trade is skipped (no price). No call qualifies: the next day is tried. A call held to its last trading day settles at intrinsic value and the next call is sold at that close. SMAs of closes; equal to an SMA counts as between. Each call cycle is one trade: (underlying points + call premium kept) / underlying close at entry; no dividends. The $0.20 / $0.40 amounts are QQQ option prices, the book's own underlying, so the rule runs on QQQ only. Book: QQQQ Jan 2, 2007 - Dec 31, 2008: adjusted -$41 (-0.2%), ATM covered calls -14.1%, buy and hold -31.3% on $22,000; our QQQ options begin in 2011. ", common!());
const ME: &[&str] = &[
    "E|\"1. Buy one ATM, front-month call at the close on the last day of each month.\" (Futures & Options Trader Apr 2009, Options Trading System Lab p.22)",
    "X|\"2. Sell the call at the close on the fourth trading day of the next month.\" (p.22)",
];
const ME_NOTE: &str = concat!("[Declared readings] Last day of the month = the last bar of a calendar month that has a later bar. Front month = the nearest monthly expiry whose last trading day is after today (the lab's Jan 31, 2001 trade bought a call with 16 days left); ATM = the listed strike nearest the close (tie: lower). Book: S&P 500 futures options (CME) Jan 31, 2001 - Mar 5, 2009: 98 trades, 47 winners (48%), -$676 (-2.3%) on $30,000; S&P futures options are not held (declared application on index ETF / index options; the rule has no fixed amount). ", common!());
const PB: &[&str] = &[
    "F|\"1. Today's low is below yesterday's low and yesterday's low is below the previous day's low. 2. Today's high is below yesterday's high and yesterday's high is below the previous day's high. 3. Today's close is below yesterday's close, yesterday's close is below the previous day's close, and the previous day's close is below the close the day before that. 4. Today's low is at least 2 percent below the low two days ago.\" (Futures & Options Trader Jun 2009, Options Trading System Lab p.30)",
    "E|\"Buy one ATM call in the first expiration month with at least 10 days remaining on the next day's open\"; \"ATM is defined as the call option with the most time premium.\" (p.30)",
    "A|\"5. Take up to five consecutive signals.\" (p.30)",
    "X|\"6. Exit when the seven-day momentum calculation is 0.75 or higher. 7. Exit any remaining open positions after 10 days on the close.\" (p.30); \"Momentum signals appear when the current close is above the close seven days ago and is in the upper 25 percent of the high-low range of the past seven days: Seven-day momentum = (close today-lowest(low,7))/(highest(high,7)-lowest(low,7))\" (p.29)",
];
const PB_NOTE: &str = concat!("[Declared readings] The contract is chosen at the signal day's close: the first monthly expiry with at least 10 calendar days from the entry day to its last trading day, and among its calls that traded on the signal day the one with the most time premium (price minus intrinsic value on the close; ties: strike nearest the close, then lower). It is bought at its own traded opening price on the next day; no traded open means no trade (the opening price of the underlying is not used). Each signal while fewer than five calls are open adds one call (one per signal day). Exits at the close: momentum >= 0.75 with the close above the close 7 days ago (as fot08_nerves), checked from the entry day, or 10 trading days after the entry day. India only: the US option snapshot has no opening price. Book: S&P 500 futures options Mar 23, 2001 - Mar 11, 2009: 53 trades, 29 winners (55%), +$36,608 (38.5%) on $95,000. Same signal as fot08_nerves (the futures version). ", common!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, needs_volume, rules, notes: &'static str, claim| StrategyDef {
        id, name, group: "Options labs (OptionVue)", session: 0, dir, status: Status::Live, needs_volume, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim,
    };
    vec![
        d("fotol_adjusted_cc", "QQQQ covered call adjusted to the trend: 5% OTM above both the 50- and 200-day SMAs, 5% ITM below both, ATM between; rolled at $0.20 time premium", "Long", false, AC,
          AC_NOTE, claim("-", "-", "-0.2% total (ATM covered calls -14.1%, buy and hold -31.3%)", "-", "QQQQ Jan 2007 - Dec 2008")),
        d("fotol_bb_breakout", "Bollinger Band breakout: buy ATM calls (puts) after a narrow-band close outside the band, out 2 days later", "Long/Short", true, BB,
          BB_NOTE, claim("19", "31.5% (6 of 19)", "-$126 (10 contracts)", "2 days", "QQQQ options May 2001 - Sep 2005")),
        d("fotol_covered_call", "QQQQ covered call: long the underlying, short the front-month call with the most time premium, rolled on expiration day", "Long", false, CC,
          CC_NOTE, claim("-", "-", "+7.58% total (1.44%/yr)", "1 month", "QQQQ Jan 2001 - Apr 2006")),
        d("fotol_monthend_call", "Month-end call: buy an ATM front-month call at the month's last close, sell on the 4th trading day", "Long", false, ME,
          ME_NOTE, claim("98", "48% (47 of 98)", "-$6.90", "4 days", "S&P 500 futures options Jan 2001 - Mar 2009")),
        d("fotol_pullback_calls", "Pullback calls: three falling bars with a 2% drop, buy the ATM call next open, out on 7-day momentum or 10 days", "Long", false, PB,
          PB_NOTE, claim("53", "55% (29 of 53)", "+$690.72", "5 days (winners)", "S&P 500 futures options Mar 2001 - Mar 2009")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(("fotol", "F&OT options labs", match id {
        "fotol_adjusted_cc" => "Trend-adjusted covered call",
        "fotol_bb_breakout" => "Bollinger breakout options",
        "fotol_covered_call" => "Covered call",
        "fotol_monthend_call" => "Month-end call",
        "fotol_pullback_calls" => "Pullback calls",
        _ => return None,
    }))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    family_of(id).map(|_| Vec::new())
}

pub fn is_rule(id: &str) -> bool { family_of(id).is_some() }

/// Option positions of a fotol rule (None = not a fotol rule; Err = no chain for this instrument).
pub fn option_positions(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Result<Vec<opt::Pos>, String>> {
    if !is_rule(id) { return None; }
    let Some(chain) = x.options else { return Some(Err(crate::books::oiss::NO_CHAIN.into())) };
    let mut w = opt::Window::new(chain, x, 16);
    Some(Ok(match id {
        "fotol_bb_breakout" => bb_breakout(&mut w, x, from, to),
        "fotol_covered_call" => covered_call(&mut w, x, from, to),
        "fotol_adjusted_cc" => adjusted_cc(&mut w, chain, x, from, to),
        "fotol_monthend_call" => monthend_call(&mut w, x, from, to),
        _ => pullback_calls(&mut w, x, from, to),
    }))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    option_positions(id, x, from, to, p).map(|r| opt::account(r.unwrap_or_default(), from, to, 0.0).trades)
}

/// Front month: the nearest listed expiry whose last trading day is after YYYYMMDD `after`.
fn front(day: &[opt::Q], x: &Ctx, after: u32) -> Option<u32> { opt::expiries(day).into_iter().find(|&e| opt::ltd(x, e) > after) }

/// The call of `expiry` with the most time premium among the calls that traded that day (ties: nearest the close, then lower).
fn most_time_premium(day: &[opt::Q], expiry: u32, s: f64) -> Option<(Contract, f64)> {
    let mut best: Option<(f64, f64, f64, f64)> = None; // (time premium, |K - S|, K, price)
    for q in day.iter().filter(|q| q.expiry == expiry && !q.put) {
        let Some(p) = q.fill_close() else { continue };
        let tv = p - (s - q.strike).max(0.0);
        let better = match best { None => true, Some((b, d, k, _)) => tv > b || (tv == b && ((q.strike - s).abs() < d || ((q.strike - s).abs() == d && q.strike < k))) };
        if better { best = Some((tv, (q.strike - s).abs(), q.strike, p)); }
    }
    best.map(|(_, _, k, p)| (Contract { expiry, put: false, strike: k }, p))
}

fn label(c: Contract) -> String { format!("{} {} exp {}", c.strike, if c.put { "put" } else { "call" }, c.expiry) }

fn monthend_call(w: &mut opt::Window, x: &Ctx, from: usize, to: usize) -> Vec<opt::Pos> {
    let n = x.r.len();
    let mut out = Vec::new();
    let mut i = from;
    while i <= to {
        if !(i + 1 < n && x.r[i + 1].date[..7] != x.r[i].date[..7]) { i += 1; continue; }
        let day = w.at(i);
        let pick = front(day, x, opt::ymd(&x.r[i].date))
            .and_then(|e| opt::nearest(&opt::strikes(day, e, false), x.c[i]).map(|strike| Contract { expiry: e, put: false, strike }));
        let Some((c, px0)) = pick.and_then(|c| w.quote(i, c).and_then(|q| q.fill_close()).map(|v| (c, v))) else { i += 1; continue };
        let exit = i + 4;
        let (j, pnl, why, open, marks) = opt::follow(w, x, c, 1.0, i, px0, to, &mut |k| (k == exit).then(|| (Exit::Close, "4th trading day of the month: sold at the close".to_string())));
        out.push(opt::Pos { dir: 1.0, s0: x.c[i], entry_i: i, exit_i: j, pnl, open,
            reason: format!("{why} · bought {} on {} at {px0:.2} · {}", label(c), x.r[i].date, marks.text()) });
        if open { break; }
        i = j + 1;
    }
    out
}

fn bb_breakout(w: &mut opt::Window, x: &Ctx, from: usize, to: usize) -> Vec<opt::Pos> {
    let (n, c) = (x.c.len(), &x.c);
    let mut mid = vec![f64::NAN; n];
    let mut sd = vec![f64::NAN; n];
    let mut bw = vec![f64::NAN; n];
    for i in 19..n {
        let win = &c[i - 19..=i];
        let m = win.iter().sum::<f64>() / 20.0;
        let s = (win.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / 20.0).sqrt();
        (mid[i], sd[i], bw[i]) = (m, s, 4.0 * s / m);
    }
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let narrow = |i: usize| {
        if i < 200 { return false; }
        let win = &bw[i - 200..i];
        if !win.iter().all(|v| v.is_finite()) { return false; }
        let (lo, hi) = win.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| (a.min(v), b.max(v)));
        bw[i - 1] <= lo + 0.1 * (hi - lo)
    };
    let loud = |i: usize| i >= 9 && vol[i - 9..=i].iter().all(|v| v.is_finite()) && vol[i] > vol[i - 9..=i].iter().sum::<f64>() / 10.0;
    let mut out = Vec::new();
    let mut i = from;
    while i <= to {
        let put = if c[i] > mid[i] + 2.0 * sd[i] { false } else if c[i] < mid[i] - 2.0 * sd[i] { true } else { i += 1; continue };
        if !(narrow(i) && loud(i)) { i += 1; continue; }
        let after = opt::ymd(&x.r[(i + 2).min(n - 1)].date);
        let day = w.at(i);
        let pick = front(day, x, after).and_then(|e| opt::nearest(&opt::strikes(day, e, put), c[i]).map(|strike| Contract { expiry: e, put, strike }));
        let Some((k, px0)) = pick.and_then(|k| w.quote(i, k).and_then(|q| q.fill_close()).map(|v| (k, v))) else { i += 1; continue };
        let exit = i + 2;
        let (j, pnl, why, open, marks) = opt::follow(w, x, k, 1.0, i, px0, to, &mut |b| (b == exit).then(|| (Exit::Close, "two days later at the close".to_string())));
        out.push(opt::Pos { dir: if put { -1.0 } else { 1.0 }, s0: c[i], entry_i: i, exit_i: j, pnl, open,
            reason: format!("{why} · bought {} on {} at {px0:.2} ({} band close) · {}", label(k), x.r[i].date, if put { "lower" } else { "upper" }, marks.text()) });
        if open { break; }
        i = j + 1;
    }
    out
}

fn covered_call(w: &mut opt::Window, x: &Ctx, from: usize, to: usize) -> Vec<opt::Pos> {
    let mut out = Vec::new();
    let mut i = from;
    while i <= to {
        let day = w.at(i);
        let Some((c, px0)) = front(day, x, opt::ymd(&x.r[i].date)).and_then(|e| most_time_premium(day, e, x.c[i])) else { i += 1; continue };
        let last = opt::last_day(x, c.expiry);
        let (j, mut pnl, why, open, marks) = opt::follow(w, x, c, -1.0, i, px0, to, &mut |k| (Some(k) == last).then(|| (Exit::Close, "expiration day: bought back at the close".to_string())));
        for (k, v) in pnl.iter_mut().enumerate() { *v += x.c[i + k] - x.c[i]; }
        out.push(opt::Pos { dir: 1.0, s0: x.c[i], entry_i: i, exit_i: j, pnl, open,
            reason: format!("{why} · long the underlying at {:.2}, sold {} at {px0:.2} on {} · {}", x.c[i], label(c), x.r[i].date, marks.text()) });
        if open || j == i { break; }
        i = j;
    }
    out
}

/// The trend-adjusted call (Feb 2009): first strike from the target toward the strike nearest the close whose time premium
/// is at least 0.40 (trend 0: the call with the most time premium, if it has 0.40).
fn adjusted_pick(day: &[opt::Q], e: u32, s: f64, trend: i32) -> Option<(Contract, f64)> {
    let tv = |k: f64, p: f64| p - (s - k).max(0.0);
    if trend == 0 { return most_time_premium(day, e, s).filter(|(k, p)| tv(k.strike, *p) >= 0.40); }
    let ks = opt::strikes(day, e, false);
    let (target, atm) = (opt::nearest(&ks, s * (1.0 + 0.05 * trend as f64))?, opt::nearest(&ks, s)?);
    let mut path: Vec<f64> = ks.into_iter().filter(|&k| k >= target.min(atm) && k <= target.max(atm)).collect();
    if trend > 0 { path.reverse(); }
    path.into_iter().find_map(|k| {
        let c = Contract { expiry: e, put: false, strike: k };
        opt::find(day, c).and_then(|q| q.fill_close()).filter(|&p| tv(k, p) >= 0.40).map(|p| (c, p))
    })
}

fn adjusted_cc(w: &mut opt::Window, chain: &opt::Chain, x: &Ctx, from: usize, to: usize) -> Vec<opt::Pos> {
    let (ma50, ma200) = (x.sma_n(50), x.sma_n(200));
    let mut w2 = opt::Window::new(chain, x, 4);
    let mut out = Vec::new();
    let mut i = from;
    while i <= to {
        let today = opt::day_no(opt::ymd(&x.r[i].date));
        let s = x.c[i];
        let trend = if out.is_empty() { 0 } else if s > ma50[i] && s > ma200[i] { 1 } else if s < ma50[i] && s < ma200[i] { -1 } else { 0 };
        let day = w.at(i);
        let pick = opt::expiries(day).into_iter().find(|&e| opt::day_no(opt::ltd(x, e)) - today > 21).and_then(|e| adjusted_pick(day, e, s, trend));
        let Some((c, px0)) = pick else { i += 1; continue };
        let (j, mut pnl, why, open, marks) = opt::follow(w, x, c, -1.0, i, px0, to, &mut |k| {
            w2.quote(k, c).and_then(|q| q.fill_close()).filter(|&p| p - (x.c[k] - c.strike).max(0.0) <= 0.20)
                .map(|_| (Exit::Close, "time premium <= 0.20: bought back at the close".to_string()))
        });
        for (k, v) in pnl.iter_mut().enumerate() { *v += x.c[i + k] - x.c[i]; }
        out.push(opt::Pos { dir: 1.0, s0: s, entry_i: i, exit_i: j, pnl, open,
            reason: format!("{why} · long the underlying at {s:.2}, sold {} at {px0:.2} on {} ({}) · {}", label(c), x.r[i].date,
                ["5% ITM, below both SMAs", "ATM", "5% OTM, above both SMAs"][(trend + 1) as usize], marks.text()) });
        if open || j == i { break; }
        i = j;
    }
    out
}

struct Held { c: Contract, e: usize, px0: f64, pnl: Vec<f64>, last: f64, marks: opt::Marks, pending: Option<String> }

fn pullback_calls(w: &mut opt::Window, x: &Ctx, from: usize, to: usize) -> Vec<opt::Pos> {
    let (c, h, l) = (&x.c, &x.h, &x.l);
    let signal = |s: usize| s >= 3 && l[s] < l[s - 1] && l[s - 1] < l[s - 2] && h[s] < h[s - 1] && h[s - 1] < h[s - 2]
        && c[s] < c[s - 1] && c[s - 1] < c[s - 2] && c[s - 2] < c[s - 3] && l[s] <= 0.98 * l[s - 2];
    let momentum = |t: usize| {
        if t < 7 || c[t] <= c[t - 7] { return false; }
        let (hh, ll) = (t - 6..=t).fold((f64::NEG_INFINITY, f64::INFINITY), |(a, b), k| (a.max(h[k]), b.min(l[k])));
        hh > ll && (c[t] - ll) / (hh - ll) >= 0.75
    };
    let mut held: Vec<Held> = Vec::new();
    let mut out = Vec::new();
    let mut order: Option<Contract> = None;
    for t in from..=to {
        if let Some(k) = order.take() {
            if let Some(px0) = w.quote(t, k).and_then(|q| q.fill_open()) {
                held.push(Held { c: k, e: t, px0, pnl: Vec::new(), last: px0, marks: opt::Marks::default(), pending: None });
            }
        }
        let mut keep = Vec::new();
        for mut p in held.drain(..) {
            let q = w.quote(t, p.c);
            let m = q.and_then(|q| q.mark());
            p.marks.add(m.map(|m| m.1));
            if let Some((v, _)) = m { p.last = v; }
            let close = q.and_then(|q| q.fill_close());
            let mut done: Option<(f64, String)> = None;
            if let Some(why) = p.pending.take() {
                match close { Some(v) => done = Some((v, format!("{why} (filled on the next traded close)"))), None => p.pending = Some(why) }
            } else {
                let why = if momentum(t) { Some("7-day momentum >= 0.75: sold at the close") } else if t - p.e >= 10 { Some("10 days: sold at the close") } else { None };
                if let Some(why) = why { match close { Some(v) => done = Some((v, why.to_string())), None => p.pending = Some(why.to_string()) } }
            }
            if done.is_none() && opt::last_day(x, p.c.expiry) == Some(t) {
                let v = opt::intrinsic(p.c, c[t]);
                done = Some((v, format!("final settlement {v:.2} on the last trading day{}", p.pending.take().map_or(String::new(), |w| format!(" ({w}: no traded price before it)")))));
            }
            let fin = |p: Held, v: f64, why: String, open: bool, out: &mut Vec<opt::Pos>| {
                let mut pnl = p.pnl;
                pnl.push(v - p.px0);
                out.push(opt::Pos { dir: 1.0, s0: c[p.e], entry_i: p.e, exit_i: t, pnl, open,
                    reason: format!("{why} · bought {} on {} at the open {:.2} · {}", label(p.c), x.r[p.e].date, p.px0, p.marks.text()) });
            };
            match done {
                Some((v, why)) => fin(p, v, why, false, &mut out),
                None if t == to => { let v = p.last; fin(p, v, "open".into(), true, &mut out) }
                None => { p.pnl.push(p.last - p.px0); keep.push(p) }
            }
        }
        held = keep;
        if t < to && held.len() < 5 && signal(t) {
            let day = w.at(t);
            let entry = opt::day_no(opt::ymd(&x.r[t + 1].date));
            order = opt::expiries(day).into_iter().find(|&e| opt::day_no(opt::ltd(x, e)) - entry >= 10)
                .and_then(|e| most_time_premium(day, e, c[t])).map(|(k, _)| k);
        }
    }
    out.sort_by_key(|p| (p.entry_i, p.exit_i));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn fixture(name: &str, rows: &[(&str, f64)], csv_rows: &[&str]) -> (Vec<Row>, &'static opt::Chain, std::path::PathBuf) {
        let rows: Vec<Row> = rows.iter().map(|&(d, c)| Row { ticker: "NIFTY 50".into(), date: d.into(), open: c, high: c, low: c, close: c, volume: None }).collect();
        let mut csv = String::from("Date,Expiry,Type,Strike,Open,High,Low,Close,Settle,Contracts,OI\n");
        let mut idx = String::from("Date,offset,bytes\n");
        for r in &rows {
            let start = csv.len();
            for l in csv_rows.iter().filter(|l| l.starts_with(&r.date)) { csv += l; csv.push('\n'); }
            if csv.len() > start { idx += &format!("{},{start},{}\n", r.date, csv.len() - start); }
        }
        let dir = std::env::temp_dir().join(format!("fotol-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("NIFTY_50.opt.csv"), csv).unwrap();
        std::fs::write(dir.join("NIFTY_50.optidx.csv"), idx).unwrap();
        let ch: &'static opt::Chain = Box::leak(Box::new(opt::Chain::load(dir.to_str().unwrap(), "NIFTY 50").unwrap()));
        (rows, ch, dir)
    }

    /// Month-end call on a fixture chain (test arithmetic, not market data): bought 100 call at 5.0 on Jan 31, sold on the
    /// 4th trading day at 7.5 (+2.5 points); a missing strike on Feb 28 means no trade that month.
    #[test]
    fn monthend_call_buys_the_last_close_and_sells_the_fourth_day() {
        let days = [("2024-01-30", 99.0), ("2024-01-31", 101.0), ("2024-02-01", 102.0), ("2024-02-02", 102.0), ("2024-02-05", 103.0),
                    ("2024-02-06", 104.0), ("2024-02-07", 104.0), ("2024-02-28", 104.0), ("2024-02-29", 104.0), ("2024-03-01", 104.0)];
        let q = |d: &str, k: u32, px: f64| format!("{d},20240229,C,{k},{px},{px},{px},{px},{px},10,10");
        let lines: Vec<String> = vec![q("2024-01-31", 100, 5.0), q("2024-01-31", 105, 2.0), q("2024-02-01", 100, 5.5), q("2024-02-02", 100, 6.0),
            q("2024-02-05", 100, 7.0), q("2024-02-06", 100, 7.5), "2024-02-29,20240328,C,90,1,1,1,1,1,0,0".into()];
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        let (rows, ch, dir) = fixture("me", &days, &refs);
        let mut x = Ctx::new(&rows, &rows, &[]);
        x.options = Some(ch);
        let pos = option_positions("fotol_monthend_call", &x, 0, rows.len() - 1, &Params::new()).unwrap().unwrap();
        assert_eq!(pos.len(), 1, "{:?}", pos.iter().map(|p| &p.reason).collect::<Vec<_>>());
        assert_eq!((pos[0].entry_i, pos[0].exit_i), (1, 5));
        assert!((pos[0].pnl.last().unwrap() - 2.5).abs() < 1e-12 && pos[0].reason.contains("100 call exp 20240229"), "{}", pos[0].reason);
        std::fs::remove_dir_all(dir).ok();
    }

    /// Most time premium: a call whose price is all intrinsic loses to the strike nearest the close.
    #[test]
    fn time_premium_picks_the_richest_traded_call() {
        let q = |k: f64, p: f64, v: f64| opt::Q { expiry: 1, put: false, strike: k, us: false, open: p, high: p, close: p, settle: p, bid: f64::NAN, ask: f64::NAN, volume: v };
        let day = [q(90.0, 10.5, 5.0), q(100.0, 3.0, 5.0), q(105.0, 1.0, 5.0), q(110.0, 9.0, 0.0)];
        assert_eq!(most_time_premium(&day, 1, 100.0).map(|(c, p)| (c.strike, p)), Some((100.0, 3.0)), "the untraded 110 call is never compared");
    }
}
