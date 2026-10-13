//! Larry Connors, "Option Income Stream System" (OISS, 2004). Book id "oiss".
//!
//! Every tradeable rule acts on an OPTION's own daily chart (3-day RSI of the at-the-money put/call),
//! filtered by the underlying's 200-day SMA. The vault has no option data yet, so the strategies read
//! one external per-ticker series per side and run the moment it is dropped into extra_data/:
//!
//!   `<TICKER>__OPT_ATM_PUT.csv` / `<TICKER>__OPT_ATM_CALL.csv`, one row per trading day d of the underlying:
//!   * scan contract S(d): the monthly contract of that type whose strike is the listed strike nearest the
//!     underlying's close on d (tie -> lower strike), front month unless d is in the front month's
//!     expiration week (the Mon..Fri calendar week holding its last trading day), then the next month.
//!   * `Strike`, `Expiry` (YYYYMMDD of S(d)'s last trading day), `Open`, `High`, `Close` = S(d)'s bar on d.
//!   * `B1..B30`: S(d)'s close on the k-th previous trading day (its own history, for the RSI).
//!   * `O1..O30`, `H1..H30`, `C1..C30`: on day d, the open/high/close of S(d-k) (the contract that was the
//!     scan contract k trading days ago), so a short contract can be followed after it stops being ATM.
//!   Empty cell = no price that day (not listed, expired, no trade). Prices in index points / $ per share.
//!
//! P&L convention: a short option leg earns (sale premium - buy-back premium) / underlying close at the
//! first entry. In the engine's Trade the position is a synthetic price that starts at the underlying close
//! (puts: dir +1, px = S0 + e1 - premium; calls: dir -1, px = S0 - e1 + premium), so Trade::ret() is that
//! fraction exactly for the first leg and the equity curve marks the trade with the underlying (delta 1)
//! between entry and exit. The book's own "% gained" (points / sale price) is printed in each trade's reason.
#![allow(unused_imports, dead_code)]
use crate::connors::*;

pub const BOOK: &str = "oiss";

const PUT: &str = "OPT_ATM_PUT";
const CALL: &str = "OPT_ATM_CALL";

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

const NOTE: &str = "Needs option data: per-ticker series OPT_ATM_PUT / OPT_ATM_CALL (scan contract = monthly strike nearest the underlying close, next month during expiration week; columns Strike, Expiry, Open, High, Close, B1..B30 own past closes, O/H/C1..30 = today's bar of the contract that was ATM k days ago). India: build from NSE F&O bhavcopy (NIFTY/BANKNIFTY monthly options). US: OEX/SPX/QQQ option EOD history. \
RSI = Wilder RSI(3) of the contract's own daily closes (TradeStation RSI), computed over the 30 back closes. Trend filter = underlying close vs its 200-day SMA, checked at entry only. Enter at the option close; stop = buy-stop at 2x the sale price on the option's intraday high (a gap above it fills at the open), checked before the close exit, from the day after entry. Exit on the close when the held contract's RSI(3) < 25. Expiration day of the held contract with no exit: buy back at its close, sell the then-ATM next-month contract at its close (no RSI/MA check, stop reset to 2x the new sale). A new signal on a different contract opens an independent, overlapping position (book OEX Trades 8/9); a repeat signal on a held contract adds nothing. \
P&L = premium points / underlying close at entry (Trade return); the book's % (points / sale price; for a rolled trade points / last sale, which reproduces Trade 12's 23.60%) is in each trade's reason. Book tested OEX, QQQ, GS, EBAY options Nov 2002-Apr 2004; published results are both sides combined.";

const NOTE_P3: &str = "Same data and mechanics as base OISS, plus one add-on: Day 1 = the entry day (p.57: 3/17 Day 1, 3/19 Day 3); when the held contract's RSI(3) has closed above 80 on add_after_days consecutive days counting the entry day, sell the same contract again (same size) at the close, at most once per trade (every example adds exactly once). The add requires the underlying still on the entry side of its 200-day SMA; the call rule's printed 'still above' is read as BELOW (matches call rule 1; flagged). One stop for all legs at 2x the FIRST sale (the book does not say; the add-on's own 2x stop would be wider). Both legs are bought back together on RSI < 25 / stop / roll; trade return = sum of the legs (position doubles); the book likewise adds each leg's % with the trade count unchanged.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, dir, status, needs_series, rules, notes, claim| StrategyDef {
        id, name, group: "OISS · Option Income Stream System", session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series, book: BOOK, rules, notes, claim,
    };
    let base = || claim("12 OEX (both sides); QQQ 14; GS 15; EBAY 8", "100% OEX; 92.86% QQQ; 86% GS; 100% EBAY", "57.85% of premium OEX; 39.59% QQQ; 35.86% GS; 62.10% EBAY", "11 days OEX; 8 QQQ; 9 GS; 8 EBAY", "Nov 2002 - Apr 2004, ATM options");
    let p3 = || claim("12 OEX (both sides); QQQ 14; GS 15", "100% OEX; 92.86% QQQ; 86% GS", "67.04% OEX; 39.49% QQQ; 46.37% GS (legs summed)", "", "Nov 2002 - Apr 2004, ATM options");
    vec![
        d("oiss_short_put", "OISS · Short the ATM put", "Long", Live, &["OPT_ATM_PUT.Close"], RULES_PUT, NOTE, base()),
        d("oiss_short_call", "OISS · Short the ATM call", "Short", Live, &["OPT_ATM_CALL.Close"], RULES_CALL, NOTE, base()),
        d("oiss_plus_three_put", "OISS Plus Three · Short put + add-on", "Long", Live, &["OPT_ATM_PUT.Close"], RULES_P3_PUT, NOTE_P3, p3()),
        d("oiss_plus_three_call", "OISS Plus Three · Short call + add-on", "Short", Live, &["OPT_ATM_CALL.Close"], RULES_P3_CALL, NOTE_P3, p3()),
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

/// Option columns of one side, aligned to the underlying's bars.
struct Opt<'a> {
    strike: &'a [f64],
    expiry: &'a [f64],
    open: &'a [f64],
    high: &'a [f64],
    close: &'a [f64],
    back: Vec<&'a [f64]>, // back[k-1] = Bk
    lo: Vec<&'a [f64]>,   // lo[k-1] = Ok
    lh: Vec<&'a [f64]>,
    lc: Vec<&'a [f64]>,
}

impl<'a> Opt<'a> {
    fn load(x: &'a Ctx, side: &str) -> Option<Self> {
        let s = |c: &str| x.extra.get(&format!("{side}.{c}")).map(|v| v.as_slice());
        let many = |p: &str| (1..).map_while(|k| s(&format!("{p}{k}"))).collect::<Vec<_>>();
        Some(Opt { strike: s("Strike")?, expiry: s("Expiry")?, open: s("Open")?, high: s("High")?, close: x.series(&format!("{side}.Close"))?,
                   back: many("B"), lo: many("O"), lh: many("H"), lc: many("C") })
    }
    /// (open, high, close) on bar i of the contract that was the scan contract on bar o.
    fn held(&self, o: usize, i: usize) -> (f64, f64, f64) {
        let k = i - o;
        let at = |v: &[&[f64]]| v.get(k - 1).map_or(f64::NAN, |c| c[i]);
        if k == 0 { (self.open[i], self.high[i], self.close[i]) } else { (at(&self.lo), at(&self.lh), at(&self.lc)) }
    }
    /// Close history of that contract up to bar i (its back closes at bar o, then o..=i), gaps dropped.
    fn hist(&self, o: usize, i: usize) -> Vec<f64> {
        let mut h: Vec<f64> = self.back.iter().rev().map(|b| b[o]).collect();
        h.extend((o..=i).map(|j| self.held(o, j).2));
        h.retain(|v| v.is_finite());
        h
    }
}

/// Wilder RSI(n) at the last close of a history (connors::rsi, TradeStation-style).
fn last_rsi(h: &[f64], n: usize) -> f64 {
    rsi(h, n).last().copied().unwrap_or(f64::NAN)
}

fn ymd(date: &str) -> f64 {
    date.replace('-', "").parse().unwrap_or(f64::NAN)
}
/// Calendar day number of a YYYYMMDD value (days_from_civil).
fn day_no(v: f64) -> i64 {
    let v = v as i64;
    let (y, m, d) = (v / 10000, v / 100 % 100, v % 100);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy
}

/// One open short-option position (all legs in the same contract).
struct Pos {
    origin: usize,        // bar on which the held contract was the scan contract
    contract: (f64, f64), // (strike, expiry)
    s0: f64,              // underlying close at first entry
    e1: f64,              // first sale premium (anchors the synthetic price)
    legs: Vec<Leg>,
    sales: Vec<f64>,      // effective sale premium of each leg
    roll_adj: f64,        // sum over rolls of (buy-back - new sale); effective exit = final buy-back + roll_adj
    last_sale: f64,       // latest sale in the held contract (stop base after a roll; book % of a rolled trade)
    stop: f64,
    streak: usize,        // consecutive RSI closes above the entry level, entry day = 1 (Plus Three)
    added: bool,
    last_px: f64,         // last known close of the held contract
    log: String,
}

fn simulate_oiss(x: &Ctx, o: &Opt, put: bool, plus3: bool, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let sg = if put { 1.0 } else { -1.0 };
    let ma = x.sma_n(g("ma_len").round().max(1.0) as usize);
    let n = g("rsi_len").round().max(1.0) as usize;
    let (hi, lo, sm, dte) = (g("rsi_entry"), g("rsi_exit"), g("stop_mult"), g("min_dte"));
    let addn = if plus3 { g("add_after_days").round().max(1.0) as usize } else { 0 };
    let trend_ok = |i: usize| if put { x.c[i] > ma[i] } else { x.c[i] < ma[i] };
    let synth = |p: &Pos, prem: f64| p.s0 + sg * p.e1 - sg * prem;
    // last trading day on or before the contract's expiry date
    let is_expiry_day = |i: usize, exp: f64| {
        let d = ymd(&x.r[i].date);
        d == exp || (d < exp && i + 1 < x.r.len() && ymd(&x.r[i + 1].date) > exp)
    };
    let close_out = |p: Pos, i: usize, buy: f64, reason: &str, open: bool| {
        let x_eff = buy + p.roll_adj;
        let den = |e: f64| if p.roll_adj != 0.0 { p.last_sale } else { e };
        let pts: Vec<String> = p.sales.iter().map(|&e| format!("{:.2}pts book {:.2}%", e - x_eff, (e - x_eff) / den(e) * 100.0)).collect();
        Trade {
            dir: sg, legs: p.legs.clone(), exit_i: i, exit_px: synth(&p, x_eff), entry_kind: "close", signal_i: i, open,
            reason: format!("{reason} · {} · buy back {buy:.2} · {}", p.log, pts.join(" / ")), exits: Vec::new()
        }
    };
    let mut open: Vec<Pos> = Vec::new();
    let mut out = Vec::new();
    for i in from..=to {
        let mut keep = Vec::new();
        for mut p in open.drain(..) {
            let (op, h, c) = o.held(p.origin, i);
            if c.is_finite() {
                p.last_px = c;
            }
            // 1. intraday buy-stop at stop_mult x the sale price
            if h.is_finite() && h >= p.stop {
                let fill = if op.is_finite() && op >= p.stop { op } else { p.stop };
                out.push(close_out(p, i, fill, "stop", false));
                continue;
            }
            // 2. RSI of the held option closes below the exit level
            let r = last_rsi(&o.hist(p.origin, i), n);
            if c.is_finite() && r < lo {
                out.push(close_out(p, i, c, &format!("RSI{n}<{lo}"), false));
                continue;
            }
            // 3. expiration day, no exit: buy back, sell the next month's ATM option at the close
            if is_expiry_day(i, p.contract.1) {
                let new = o.close[i];
                if new.is_finite() && o.expiry[i] > p.contract.1 {
                    p.roll_adj += p.last_px - new;
                    p.log += &format!(" → roll {} buy {:.2}, sell {} exp {} @{new:.2}", x.r[i].date, p.last_px, o.strike[i], o.expiry[i]);
                    p.origin = i;
                    p.contract = (o.strike[i], o.expiry[i]);
                    p.last_sale = new;
                    p.stop = sm * new;
                    p.streak = 0;
                    p.last_px = new;
                    keep.push(p);
                } else {
                    let buy = p.last_px;
                    out.push(close_out(p, i, buy, "expiry (no next-month price)", false));
                }
                continue;
            }
            // 4. Plus Three: one add-on after N consecutive closes above the entry level (entry day = Day 1)
            if plus3 {
                p.streak = if r > hi { p.streak + 1 } else { 0 };
                if !p.added && p.streak >= addn && c.is_finite() && trend_ok(i) {
                    p.added = true;
                    p.log += &format!(" + add {} @{c:.2}", x.r[i].date);
                    let e = c + p.roll_adj; // sold after any roll: its points are c - final buy-back
                    p.legs.push(Leg { i, px: synth(&p, e), w: 1.0 });
                    p.sales.push(e);
                }
            }
            keep.push(p);
        }
        open = keep;
        // new short on today's scan contract
        let (c, k, e) = (o.close[i], o.strike[i], o.expiry[i]);
        if c.is_finite() && c > 0.0 && trend_ok(i) && !open.iter().any(|p| p.contract == (k, e))
            && (day_no(e) - day_no(ymd(&x.r[i].date))) as f64 >= dte
            && last_rsi(&o.hist(i, i), n) > hi
        {
            let mut p = Pos { origin: i, contract: (k, e), s0: x.c[i], e1: c, legs: vec![], sales: vec![c], roll_adj: 0.0, last_sale: c,
                              stop: sm * c, streak: 1, added: false, last_px: c,
                              log: format!("{} {k} exp {e} sold {c:.2}", if put { "P" } else { "C" }) };
            p.legs.push(Leg { i, px: synth(&p, c), w: 1.0 });
            open.push(p);
        }
    }
    for p in open {
        let buy = p.last_px;
        out.push(close_out(p, to, buy, "open", true));
    }
    out.sort_by_key(|t| (t.legs[0].i, t.exit_i));
    out
}

/// None = not a strategy of this book.
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let (put, plus3) = match id {
        "oiss_short_put" => (true, false),
        "oiss_short_call" => (false, false),
        "oiss_plus_three_put" => (true, true),
        "oiss_plus_three_call" => (false, true),
        "oiss_intraday_exit_variant" => return Some(vec![]),
        _ => return None,
    };
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def);
    let Some(o) = Opt::load(x, if put { PUT } else { CALL }) else { return Some(vec![]) };
    Some(simulate_oiss(x, &o, put, plus3, from, to, &g))
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

    /// OEX Trade 12 replayed on synthetic option rows: sell Mar 550 put 6.00 (3/10/04), expiry roll 3/19/04
    /// (buy 6.30, sell Apr 540 put 8.90), RSI exit 3/26/04 at 6.50: 2.10 points, book 23.60%.
    #[test]
    fn trade12_roll() {
        let dates = ["2004-03-03", "2004-03-04", "2004-03-05", "2004-03-08", "2004-03-09", "2004-03-10", "2004-03-11", "2004-03-12", "2004-03-15",
                     "2004-03-16", "2004-03-17", "2004-03-18", "2004-03-19", "2004-03-22", "2004-03-23", "2004-03-24", "2004-03-25", "2004-03-26"];
        let n = dates.len();
        let bar = |d: String, c: f64| Row { ticker: "OEX".into(), date: d, open: c, high: c, low: c, close: c, volume: None };
        let mut rows: Vec<Row> = (0..250).map(|k| bar(format!("2003-{:02}-{:02}", 1 + k / 28, 1 + k % 28), 500.0)).collect();
        let base = rows.len();
        rows.extend(dates.iter().map(|d| bar(d.to_string(), 552.8)));
        let len = rows.len();
        // Mar 550 put: quiet, then RSI(3) > 80 on 3/10 at 6.00; 6.30 on expiry 3/19
        let mar = [5.0, 4.95, 4.9, 4.85, 4.8, 6.0, 5.6, 5.9, 6.4, 6.3, 6.1, 6.2, 6.30];
        // Apr 540 put from 3/19 (sold 8.90) to 3/26 (RSI(3) < 25, bought back 6.50)
        let apr = [8.90, 9.2, 9.0, 8.8, 8.9, 6.50];
        let scan_mar = |j: usize| j <= 11;
        let price = |j: usize| if scan_mar(j) { mar[j] } else { apr[j - 12] };
        let held = |j: usize, k: usize| if j < k { f64::NAN } else if scan_mar(j - k) { mar.get(j).copied().unwrap_or(f64::NAN) } else { apr[j - 12] };
        let mut extra = Vec::new();
        let mut col = |name: String, f: &dyn Fn(usize) -> f64| {
            let mut v = vec![f64::NAN; len];
            (0..n).for_each(|j| v[base + j] = f(j));
            extra.push((format!("OPT_ATM_PUT.{name}"), v));
        };
        col("Strike".into(), &|j| if scan_mar(j) { 550.0 } else { 540.0 });
        col("Expiry".into(), &|j| if scan_mar(j) { 20040319.0 } else { 20040416.0 });
        col("Open".into(), &price);
        col("High".into(), &price);
        col("Close".into(), &price);
        for k in 1..=10 {
            col(format!("B{k}"), &|j| if j >= k && scan_mar(j) == scan_mar(j - k) { price(j - k) } else { f64::NAN });
            for p in ["O", "H", "C"] {
                col(format!("{p}{k}"), &|j| held(j, k));
            }
        }
        let mut x = Ctx::new(&rows, &rows, &[]);
        x.extra.extend(extra);
        let t = run("oiss_short_put", &x, base, len - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1, "{:?}", t.iter().map(|t| &t.reason).collect::<Vec<_>>());
        let t = &t[0];
        assert_eq!(x.r[t.legs[0].i].date, "2004-03-10");
        assert_eq!(x.r[t.exit_i].date, "2004-03-26");
        assert!(t.reason.contains("roll 2004-03-19") && t.reason.contains("2.10pts book 23.60%"), "{}", t.reason);
        assert!((t.ret(0.0) - 2.10 / 552.8).abs() < 1e-12, "P&L = points / underlying close at entry");
    }
}
