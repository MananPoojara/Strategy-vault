//! Larry Connors, "Option Income Stream System" (OISS, 2004). Book id "oiss".
//!
//! Every tradeable rule acts on an OPTION's own daily chart (3-day RSI of the at-the-money put/call),
//! filtered by the underlying's 200-day SMA. Options phase 2 (2026-10-24): the rules read the real EOD
//! monthly-option chain of the underlying (crate::options, built from the NSE F&O bhavcopy; the US chain has no
//! option high, so the 2x stop cannot be checked there and US runs stay held in instrument_policy).
//!   * scan contract S(d): the monthly contract of that type whose strike is the listed strike nearest the
//!     underlying's close on d (tie -> lower strike), front month unless d is in the front month's
//!     expiration week (the Mon..Sun calendar week holding its last trading day), then the next month.
//!   * RSI history of a contract = its own traded closes over the 30 bars before the day it was the scan contract,
//!     then up to today (untraded days have no close and are skipped).
//!   * fills only on days the contract traded (Contracts > 0), at its close; the stop on its traded high.
//! P&L: a short option leg earns (sale premium - buy-back premium) per unit of the underlying; position notional =
//! underlying close at the first entry. Open positions are valued every day at the held contract's own price
//! (traded close, else the exchange settlement price, else the last value), so overlapping positions and the
//! equity curve are real option values (crate::options::account). The book's own "% gained" (points / sale price)
//! is printed in each trade's reason.
#![allow(unused_imports, dead_code)]
use crate::connors::*;
use crate::options as opt;

pub const BOOK: &str = "oiss";


const RULES_PUT: &[&str] = &[
    "F|The index (or stock) is above its 200-day MA.",
    "F|The 3-day RSI of the at-the-money put, that is more than one week out from expiration, closes above 80 (use the next month's option if the current option is in expiration week).",
    "E|Sell the put at the close",
    "X|put on a stop-loss at double your entry price.",
    "X|Exit the put when the 3-day RSI of the put closes below 25.",
    "X|[Trade 12, p.38-39; p.44] Options expiration day and a sell signal has not yet been triggered: \"We just buy back the March 550 Puts on expiration day ... Next we sell the ATM [puts] for the next expiration month\".",
];
const RULES_CALL: &[&str] = &[
    "F|The index (or stock) is below its 200-day MA.",
    "F|The 3-day RSI of the at-the-money call, that is more than one week out from expiration, closes above 80 (use the next months option if the current option is in expiration week).",
    "E|Sell the call at the close",
    "X|put on a stop-loss at double your entry price.",
    "X|Exit the call when the 3-day RSI of the call closes below 25.",
    "X|[Trade 12, p.38-39; p.44] On options expiration day with no exit signal: buy back the expiring call and sell the ATM call of the next expiration month at the close (roll).",
];
const RULES_P3_PUT: &[&str] = &[
    "F|The market is above its 200-day MA.",
    "F|The 3-day RSI of the at-the-money put that is more than one week out from expiration closes above 80 (use the next month's option if the current option is in expiration week).",
    "E|Sell the put at the close",
    "X|put on a stop-loss at double your entry price.",
    "A|Take an additional position if the 3-day RSI of the at-the-money put closes above 80, three days in a row (only take the position if the market is still above its 200-day MA).",
    "X|Exit all the puts when the 3-day RSI of the put closes below 25.",
    "X|[By example, base OISS Trade 12] expiration-day roll to the next month's ATM put.",
];
const RULES_P3_CALL: &[&str] = &[
    "F|The market is below its 200-day MA.",
    "F|The 3-day RSI of the at-the-money call that is more than one week out from expiration closes above 80 (use the next month's option if the current option is in expiration week).",
    "E|Sell the call at the close",
    "X|put on a stop-loss at double your entry price.",
    "A|Take an additional position if the 3-day RSI of the at the-money call closes above 80, three days in a row (only take the position if the market is still above its 200-day MA). [coded as BELOW, see notes]",
    "X|Exit all the calls when the 3-day RSI of the call closes below 25.",
    "X|[By example, base OISS Trade 12] expiration-day roll to the next month's ATM call.",
];

const NOTE: &str = "Real NSE option prices (options phase 2): scan contract = the monthly contract whose listed strike is nearest the index close (tie: lower), next month during the front month's expiration week. Fills only on days the contract traded (Contracts > 0): an untraded row is never a fill. Open positions are valued daily at the held contract's own price (traded close; on untraded days the exchange settlement price, labelled; else the last value); the equity curve, drawdown and overlapping positions use these values. Each position = one option per unit of the index, notional = account equity at entry. Book traded OEX/QQQ/stock options 2002-04; NIFTY 50 / NIFTY BANK index options are a declared application. US option data (SPY/QQQ/IWM) has no option high, so the 2x stop cannot be checked: US runs are held. Costs: none in the book; the vault cost setting is charged per side on the notional (0 by default). \
RSI = Wilder RSI(3) of the contract's own traded daily closes (TradeStation RSI), computed over the 30 back bars (untraded days skipped). Trend filter = underlying close vs its 200-day SMA, checked at entry only. Enter at the option close; stop = buy-stop at 2x the sale price on the option's intraday high (a gap above it fills at the open), checked before the close exit, from the day after entry. Exit on the close when the held contract's RSI(3) < 25. Last trading day of the held contract with no exit: buy back at its traded close (no trade: final settlement = intrinsic value on the index close), sell the then-ATM next-month contract at its close (no RSI/MA check, stop reset to 2x the new sale; if that contract did not trade the position is closed). A new signal on a different contract opens an independent, overlapping position (book OEX Trades 8/9); a repeat signal on a held contract adds nothing. \
P&L = premium points / underlying close at entry (Trade return); the book's % (points / sale price; for a rolled trade points / last sale, which reproduces Trade 12's 23.60%) is in each trade's reason. Book tested OEX, QQQ, GS, EBAY options Nov 2002-Apr 2004; published results are both sides combined.";

const NOTE_P3: &str = "Same data and mechanics as base OISS, plus one add-on: Day 1 = the entry day (p.57: 3/17 Day 1, 3/19 Day 3); when the held contract's RSI(3) has closed above 80 on add_after_days consecutive traded days counting the entry day (an untraded day neither counts nor resets), sell the same contract again (same size) at the close, at most once per trade (every example adds exactly once). The add requires the underlying still on the entry side of its 200-day SMA; the call rule's printed 'still above' is read as BELOW (matches call rule 1; flagged). One stop for all legs at 2x the FIRST sale (the book does not say; the add-on's own 2x stop would be wider). Both legs are bought back together on RSI < 25 / stop / roll; trade return = sum of the legs (position doubles); the book likewise adds each leg's % with the trade count unchanged.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, dir, status, needs_series, rules, notes, claim| StrategyDef {
        id, name, group: "OISS · Option Income Stream System", session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series, book: BOOK, rules, notes, claim,
    };
    let base = || claim("12 OEX (both sides); QQQ 14; GS 15; EBAY 8", "100% OEX; 92.86% QQQ; 86% GS; 100% EBAY", "57.85% of premium OEX; 39.59% QQQ; 35.86% GS; 62.10% EBAY", "11 days OEX; 8 QQQ; 9 GS; 8 EBAY", "Nov 2002 - Apr 2004, ATM options");
    let p3 = || claim("12 OEX (both sides); QQQ 14; GS 15", "100% OEX; 92.86% QQQ; 86% GS", "67.04% OEX; 39.49% QQQ; 46.37% GS (legs summed)", "", "Nov 2002 - Apr 2004, ATM options");
    vec![
        d("oiss_short_put", "OISS · Short the ATM put", "Long", Live, &[], RULES_PUT, NOTE, base()),
        d("oiss_short_call", "OISS · Short the ATM call", "Short", Live, &[], RULES_CALL, NOTE, base()),
        d("oiss_plus_three_put", "OISS Plus Three · Short put + add-on", "Long", Live, &[], RULES_P3_PUT, NOTE_P3, p3()),
        d("oiss_plus_three_call", "OISS Plus Three · Short call + add-on", "Short", Live, &[], RULES_P3_CALL, NOTE_P3, p3()),
        d("oiss_intraday_exit_variant", "OISS · Intraday RSI exit (untested)", "Long/Short", NotTestable("discretionary / untested in the book"), &[],
          &["X|If you're in a position that turns profitable and the RSI drops below 25 several hours before the close and the option price drops to an attractive level, you might consider an exit at that time.",
            "X|When this happens and the trade is well into substantial gains, you may opt to exit before the \"official\" exit is reached at the end of the day. We have not tested these situations as exits."],
          "Discretionary ('attractive level', 'substantial gains') and explicitly untested by the book (p.42); would need intraday option bars. Example only: May 2004 OEX 540 put 8.80 -> 2.75 (68.75%) vs the official path 50.91% (p.42-44).",
          claim("1 example", "", "68.75% (modified) vs 50.91% (official)", "8 days", "May 2004, OEX 540 put")),
    ]
}

/// Tunable numbers of each strategy (book value = default). None = not a strategy of this book.
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![
        p("ma_len", "Underlying trend filter: SMA length", 200.0, 100.0, 250.0, 25.0),
        p("rsi_len", "RSI length on the option's daily closes", 3.0, 2.0, 5.0, 1.0),
        p("rsi_entry", "Option RSI closes above this to sell", 80.0, 70.0, 90.0, 5.0),
        p("rsi_exit", "Option RSI closes below this to buy back", 25.0, 15.0, 35.0, 5.0),
        p("stop_mult", "Stop = this multiple of the sale price", 2.0, 1.5, 3.0, 0.25),
        p("min_dte", "Entry contract at least this many calendar days from expiry (one week)", 7.0, 7.0, 21.0, 1.0),
    ];
    match id {
        "oiss_short_put" | "oiss_short_call" => Some(v),
        "oiss_plus_three_put" | "oiss_plus_three_call" => {
            v.push(p("add_after_days", "Add after N consecutive RSI closes above the entry level (entry day = Day 1)", 3.0, 2.0, 5.0, 1.0));
            Some(v)
        }
        "oiss_intraday_exit_variant" => Some(vec![]),
        _ => None,
    }
}

/// (family id, family name, variant label) for the strategy list.
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "oiss_short_put" => ("oiss", "OISS (Option Income Stream System)", "Short put"),
        "oiss_short_call" => ("oiss", "OISS (Option Income Stream System)", "Short call"),
        "oiss_intraday_exit_variant" => ("oiss", "OISS (Option Income Stream System)", "Intraday exit (untested)"),
        "oiss_plus_three_put" => ("oiss_plus3", "OISS Plus Three", "Short put + add"),
        "oiss_plus_three_call" => ("oiss_plus3", "OISS Plus Three", "Short call + add"),
        _ => return None,
    })
}

/// Wilder RSI(n) at the last close of a history (connors::rsi, TradeStation-style).
fn last_rsi(h: &[f64], n: usize) -> f64 {
    rsi(h, n).last().copied().unwrap_or(f64::NAN)
}

/// Back closes in the RSI history (the scan contract's own last 30 trading days before the signal day).
const BACK: usize = 30;

/// The scan contract S(i): monthly contract of the side whose strike is the listed strike nearest the underlying close
/// (tie -> lower), front month unless bar i lies in the Mon..Fri week of the front month's last trading day.
fn scan(w: &mut opt::Window, x: &Ctx, i: usize, put: bool) -> Option<opt::Contract> {
    let day = w.at(i);
    let ex = opt::expiries(day);
    let front = *ex.first()?;
    let expiry = if opt::week_of(opt::ltd(x, front)) == opt::week_of(opt::ymd(&x.r[i].date)) { *ex.get(1)? } else { front };
    let strike = opt::nearest(&opt::strikes(day, expiry, put), x.c[i])?;
    Some(opt::Contract { expiry, put, strike })
}

/// Traded closes of one contract over bars a..=b (untraded days skipped: they have no close).
fn hist(w: &mut opt::Window, c: opt::Contract, a: usize, b: usize) -> Vec<f64> {
    (a..=b).filter_map(|k| w.quote(k, c).and_then(|q| q.fill_close())).collect()
}

/// One open short-option position (all legs in the same contract).
struct Pos {
    origin: usize,        // bar on which the held contract was the scan contract (start of its RSI history)
    contract: opt::Contract,
    entry: usize,
    s0: f64,              // underlying close at first entry (notional)
    sales: Vec<f64>,      // effective sale premium of each leg
    roll_adj: f64,        // sum over rolls of (buy-back - new sale); effective exit = final buy-back + roll_adj
    last_sale: f64,       // latest sale in the held contract (stop base after a roll; book % of a rolled trade)
    stop: f64,
    streak: usize,        // consecutive RSI closes above the entry level, entry day = 1 (Plus Three)
    added: bool,
    mark: f64,            // last mark of the held contract
    pnl: Vec<f64>,        // points per bar since entry
    marks: opt::Marks,
    log: String,
}

impl Pos {
    fn value(&self, price: f64) -> f64 { self.sales.iter().map(|e| e - (price + self.roll_adj)).sum() }
    fn close(mut self, i: usize, buy: f64, reason: &str, open: bool, put: bool) -> opt::Pos {
        let v = self.value(buy);
        self.pnl.push(v);
        let x_eff = buy + self.roll_adj;
        let den = |e: f64| if self.roll_adj != 0.0 { self.last_sale } else { e };
        let pts: Vec<String> = self.sales.iter().map(|&e| format!("{:.2}pts book {:.2}%", e - x_eff, (e - x_eff) / den(e) * 100.0)).collect();
        opt::Pos { dir: if put { 1.0 } else { -1.0 }, s0: self.s0, entry_i: self.entry, exit_i: i, pnl: self.pnl, open,
                   reason: format!("{reason} · {} · buy back {buy:.2} · {} · {}", self.log, pts.join(" / "), self.marks.text()) }
    }
}

fn simulate_oiss(x: &Ctx, chain: &opt::Chain, put: bool, plus3: bool, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<opt::Pos> {
    let ma = x.sma_n(g("ma_len").round().max(1.0) as usize);
    let n = g("rsi_len").round().max(1.0) as usize;
    let (hi, lo, sm, dte) = (g("rsi_entry"), g("rsi_exit"), g("stop_mult"), g("min_dte"));
    let addn = if plus3 { g("add_after_days").round().max(1.0) as usize } else { 0 };
    let trend_ok = |i: usize| if put { x.c[i] > ma[i] } else { x.c[i] < ma[i] };
    let mut w = opt::Window::new(chain, x, 3 * BACK + 40);
    let mut open: Vec<Pos> = Vec::new();
    let mut out = Vec::new();
    for i in from.saturating_sub(BACK)..=to {
        w.at(i);
        if i < from { continue; }
        let mut keep = Vec::new();
        for mut p in open.drain(..) {
            let q = w.quote(i, p.contract);
            let m = q.and_then(|q| q.mark());
            p.marks.add(m.map(|m| m.1));
            if let Some((v, _)) = m { p.mark = v; }
            let c = q.and_then(|q| q.fill_close());
            // 1. buy-stop at stop_mult x the sale price on the option's traded high (a gap above it fills at the open)
            if let Some(h) = q.and_then(|q| q.day_high()).filter(|h| *h >= p.stop) {
                let _ = h;
                let fill = q.and_then(|q| q.fill_open()).filter(|o| *o >= p.stop).unwrap_or(p.stop);
                out.push(p.close(i, fill, "stop", false, put));
                continue;
            }
            // 2. RSI of the held option (its traded closes) closes below the exit level
            if let Some(c) = c {
                if last_rsi(&hist(&mut w, p.contract, p.origin.saturating_sub(BACK), i), n) < lo {
                    out.push(p.close(i, c, &format!("RSI{n}<{lo}"), false, put));
                    continue;
                }
            }
            // 3. last trading day of the held contract, no exit: buy back, sell the next month's ATM option at the close
            if opt::last_day(x, p.contract.expiry) == Some(i) {
                let (buy, how) = match c { Some(c) => (c, "close"), None => (opt::intrinsic(p.contract, x.c[i]), "final settlement") };
                let new = scan(&mut w, x, i, put).filter(|s| s.expiry > p.contract.expiry)
                    .and_then(|s| w.quote(i, s).and_then(|q| q.fill_close()).map(|v| (s, v)));
                match new {
                    Some((s, v)) => {
                        p.roll_adj += buy - v;
                        p.log += &format!(" → roll {} buy {buy:.2} ({how}), sell {} exp {} @{v:.2}", x.r[i].date, s.strike, s.expiry);
                        p.origin = i;
                        p.contract = s;
                        p.last_sale = v;
                        p.stop = sm * v;
                        p.streak = 0;
                        p.mark = v;
                        let val = p.value(v);
                        p.pnl.push(val);
                        keep.push(p);
                    }
                    None => out.push(p.close(i, buy, &format!("expiry ({how}; the next month's ATM option did not trade)"), false, put)),
                }
                continue;
            }
            // 4. Plus Three: one add-on after N consecutive closes above the entry level (entry day = Day 1)
            if plus3 {
                if let Some(c) = c {
                    let r = last_rsi(&hist(&mut w, p.contract, p.origin.saturating_sub(BACK), i), n);
                    p.streak = if r > hi { p.streak + 1 } else { 0 };
                    if !p.added && p.streak >= addn && trend_ok(i) {
                        p.added = true;
                        p.log += &format!(" + add {} @{c:.2}", x.r[i].date);
                        p.sales.push(c + p.roll_adj); // sold after any roll: its points are c - final buy-back
                    }
                }
            }
            let val = p.value(p.mark);
            p.pnl.push(val);
            keep.push(p);
        }
        open = keep;
        // new short on today's scan contract (it must trade today)
        let Some(s) = scan(&mut w, x, i, put) else { continue };
        let Some(c) = w.quote(i, s).and_then(|q| q.fill_close()) else { continue };
        if trend_ok(i) && !open.iter().any(|p| p.contract == s)
            && (opt::day_no(opt::ltd(x, s.expiry)) - opt::day_no(opt::ymd(&x.r[i].date))) as f64 >= dte
            && last_rsi(&hist(&mut w, s, i.saturating_sub(BACK), i), n) > hi
        {
            open.push(Pos { origin: i, contract: s, entry: i, s0: x.c[i], sales: vec![c], roll_adj: 0.0, last_sale: c, stop: sm * c, streak: 1,
                            added: false, mark: c, pnl: vec![0.0], marks: opt::Marks::default(),
                            log: format!("{} {} exp {} sold {} {c:.2}", if put { "P" } else { "C" }, s.strike, s.expiry, x.r[i].date) });
        }
    }
    for p in open {
        let buy = p.mark;
        out.push(p.close(to, buy, "open", true, put));
    }
    out.sort_by_key(|t| (t.entry_i, t.exit_i));
    out
}

/// Option positions of an OISS rule (None = not an OISS option rule). Err = no option chain for this instrument.
pub fn option_positions(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Result<Vec<opt::Pos>, String>> {
    let (put, plus3) = match id {
        "oiss_short_put" => (true, false),
        "oiss_short_call" => (false, false),
        "oiss_plus_three_put" => (true, true),
        "oiss_plus_three_call" => (false, true),
        _ => return None,
    };
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def);
    let Some(chain) = x.options else { return Some(Err(NO_CHAIN.into())) };
    Some(Ok(simulate_oiss(x, chain, put, plus3, from, to, &g)))
}
pub const NO_CHAIN: &str = "needs_data: no traded option prices are loaded for this instrument";

/// None = not a strategy of this book.
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id == "oiss_intraday_exit_variant" { return Some(vec![]); }
    let pos = option_positions(id, x, from, to, p)?.unwrap_or_default();
    Some(opt::account(pos, from, to, 0.0).trades)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    #[test]
    fn book_percent_arithmetic() {
        // book % = (sell - buy) / sell, on the printed option prices (p.15-39, p.57, p.59, p.46)
        let pc = |s: f64, b: f64| ((s - b) / s * 10000.0).round() / 100.0;
        assert_eq!(pc(12.60, 4.80), 61.90); // OEX #6
        assert_eq!(pc(7.70, 1.20), 84.42); // OEX #9
        assert_eq!(pc(15.50, 11.00), 29.03); // OEX #5 first leg
        assert_eq!(pc(18.80, 11.00), 41.49); // OEX #5 Plus Three add
        assert_eq!(pc(10.01, 2.50), 75.02); // OEX #10 Plus Three add
        assert_eq!(pc(1.35, 2.25), -66.67); // QQQ #13
        // rolled OEX #12: (6.00 - 6.30) + (8.90 - 6.50) = 2.10 points, / last sale 8.90 = 23.60%
        assert_eq!(((6.00 - 6.30 + 8.90 - 6.50) / 8.90 * 10000.0f64).round() / 100.0, 23.60);
    }

    #[test]
    fn rsi3_on_option_closes() {
        // Wilder RSI(3), SMA seed: changes +1,+1,-1 -> avg up 2/3, avg down 1/3 -> 66.67;
        // next change +2 -> up (2/3*2+2)/3 = 10/9, down (1/3*2)/3 = 2/9 -> RS 5 -> 83.33 (> 80: sell)
        assert!((last_rsi(&[10.0, 11.0, 12.0, 11.0], 3) - 66.6667).abs() < 1e-3);
        assert!((last_rsi(&[10.0, 11.0, 12.0, 11.0, 13.0], 3) - 83.3333).abs() < 1e-3);
        // OEX #2 (p.17): sold 8.80, closed 10.80 next day, bought back 2.35: the collapse puts RSI(3) under 25
        assert!(last_rsi(&[6.0, 6.5, 7.6, 8.80, 10.80, 6.0, 2.35], 3) < 25.0);
    }

    /// OEX Trade 12 replayed on a fixture option chain (test arithmetic, not market data): sell Mar 550 put 6.00
    /// (3/10/04), last-trading-day roll 3/19/04 (buy 6.30, sell Apr 540 put 8.90), RSI exit 3/26/04 at 6.50: 2.10 points, book 23.60%.
    #[test]
    fn trade12_roll() {
        let dates = ["2004-03-03", "2004-03-04", "2004-03-05", "2004-03-08", "2004-03-09", "2004-03-10", "2004-03-11", "2004-03-12", "2004-03-15",
                     "2004-03-16", "2004-03-17", "2004-03-18", "2004-03-19", "2004-03-22", "2004-03-23", "2004-03-24", "2004-03-25", "2004-03-26"];
        let bar = |d: String, c: f64| Row { ticker: "OEX".into(), date: d, open: c, high: c, low: c, close: c, volume: None };
        let mut rows: Vec<Row> = (0..250).map(|k| bar(format!("2003-{:02}-{:02}", 1 + k / 28, 1 + k % 28), 500.0)).collect();
        let base = rows.len();
        rows.extend(dates.iter().map(|d| bar(d.to_string(), 552.8)));
        let mar = [5.0, 4.95, 4.9, 4.85, 4.8, 6.0, 5.6, 5.9, 6.4, 6.3, 6.1, 6.2, 6.30];
        let apr = [8.90, 9.2, 9.0, 8.8, 8.9, 6.50];
        let mut csv = String::from("Date,Expiry,Type,Strike,Open,High,Low,Close,Settle,Contracts,OI\n");
        let mut idx = String::from("Date,offset,bytes\n");
        for (j, d) in dates.iter().enumerate() {
            let start = csv.len();
            if j < mar.len() { csv += &format!("{d},20040319,P,550,{0},{0},{0},{0},{0},10,100\n", mar[j]); }
            if j >= 12 { csv += &format!("{d},20040416,P,540,{0},{0},{0},{0},{0},10,100\n", apr[j - 12]); }
            idx += &format!("{d},{start},{}\n", csv.len() - start);
        }
        let dir = std::env::temp_dir().join(format!("oiss-fixture-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("OEX.opt.csv"), csv).unwrap();
        std::fs::write(dir.join("OEX.optidx.csv"), idx).unwrap();
        let chain: &'static opt::Chain = Box::leak(Box::new(opt::Chain::load(dir.to_str().unwrap(), "OEX").unwrap()));
        let mut x = Ctx::new(&rows, &rows, &[]);
        x.options = Some(chain);
        let t = run("oiss_short_put", &x, base, rows.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1, "{:?}", t.iter().map(|t| &t.reason).collect::<Vec<_>>());
        let t = &t[0];
        assert_eq!(x.r[t.legs[0].i].date, "2004-03-10");
        assert_eq!(x.r[t.exit_i].date, "2004-03-26");
        assert!(t.reason.contains("roll 2004-03-19") && t.reason.contains("2.10pts book 23.60%"), "{}", t.reason);
        assert!((t.ret(0.0) - 2.10 / 552.8).abs() < 1e-12, "P&L = points / underlying close at entry");
        // the account marks the open put at its own price: on 3/11 the put closed at 5.60 (sold 6.00) -> +0.40 points
        let pos = option_positions("oiss_short_put", &x, base, rows.len() - 1, &Params::new()).unwrap().unwrap();
        let a = opt::account(pos, base, rows.len() - 1, 0.0);
        assert!((a.equity[6] - (1.0 + 0.40 / 552.8)).abs() < 1e-12, "{:?}", &a.equity[..8]);
        std::fs::remove_dir_all(dir).ok();
    }
}
