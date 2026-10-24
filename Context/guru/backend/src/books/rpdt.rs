//! Robert Pardo, Design, Testing and Optimization of Trading Systems (Wiley, 1992).
//! Mostly method (optimization, walk-forward, evaluation); only the fully specified example systems are coded.
//! Source + verification: ~/.local/share/strategy-vault/library/rpdt/. The SAR helpers are reused by rpeo.rs (2008 edition).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rpdt";
/// Admitted ids (authors/pardo/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["rpdt_ma_xover", "rpdt_range_bo", "rpdt_ma_timefilter"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

fn def(id: &'static str, name: &'static str, group: &'static str, status: Status, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        def("rpdt_ma_xover", "Two moving-average crossover (5/20, always in)", "Ch 2 · Step 2: Write the rules in a definitive form", Live, &[
            "F|\"Formula 1: MA1 = [C(t) + C(t+1) + ... + C(t+x-1)]/x\"; \"Formula 2: MA2 = [C(t) + ... + C(t+y-1)]/y\"; \"Rule 1: y is never less than 2 times x.\" (p.13-14)",
            "E|\"Rule 2: If MA1(t) > MA2(t) and MA1(t-1) < MA2(t-1), then go long.\" (p.14)",
            "E|\"Rule 4: If MA1(t) < MA2(t) and MA1(t-1) > MA2(t-1), then go short.\" (p.14)",
            "E|Script: Period_1 = 5, Period_2 = 20; \"buy net_position + 1 at MOO\" / \"sell net_position + 1 at MOO\" - \"an order is set to buy and reverse the current short position with a market-on-open order the next day (MOO)\" (p.15-16)",
            "X|\"Once long, stay long until a sell entry occurs ... Once short, stay short until a buy entry occurs.\" (p.13) - stop-and-reverse, always in the market",
        ], "Book defaults are the Advanced Trader script values 5/20 (p.15). The book's multimarket test (Figure 2-3, p.16-18) used 5/40: run with p.slow=40 to compare (S&P 500 futures 1983-84 23 trades 13% wins, 1985-86 14/42%, 1987-88 17/29%, 1989-90 18/33%; see README). The book traded futures; here run on the index (SPX, NIFTY 50) with percentage returns; shorts are hypothetical index shorts. Strict < and > exactly as printed: a day with MA1 = MA2 gives no cross."),
        def("rpdt_range_bo", "125% of yesterday's range breakout (always in)", "Ch 2 · Step 1: Formulate the trading strategy", Live, &[
            "E|\"go long when the market rallies more than 125% of yesterday's daily range from the close.\" (p.9) - buy stop at yesterday's close + 1.25 x (yesterday's high - low)",
            "E|\"Go short when the market falls the same amount.\" (p.9) - sell stop at yesterday's close - 1.25 x yesterday's range",
            "X|\"Exit positions on opposite entry signals.\" (p.9) - the opposite stop exits and enters the other side",
        ], "Stops evaluated on daily bars: a gap beyond the level fills at the open, otherwise at the level ('more than': the level must be exceeded, high > level). If both levels are crossed on one bar the order is the TradeStation intrabar convention (open nearer the high: O-H-L-C, else O-L-H-C), so the bar can reverse twice - the book is silent (class B assumption). Book traded futures; run on the index."),
        def("rpdt_ma_timefilter", "3-day MA vs 12-day midpoint MA, 2-day time filter (always in)", "Ch 6 · Preliminary testing", Live, &[
            "F|\"MA1, the 3-day moving average of closes\"; MA2 \"the 12-day moving average of midpoints\" (p.85-86) - SMA of (high + low)/2",
            "E|\"It buys at the open of the third day when MA1 ... crosses over and remains over MA2 ... for 2 days.\" (p.85-86) - MA1 > MA2 on two consecutive closes, buy next open",
            "E|\"The model sells at the open when MA1 has been lower than MA2 for 2 days.\" (p.86)",
            "X|Buy and sell reverse each other: the printed Swiss franc trade list alternates Buy/Sell with the position always on (Figure 6-3, p.87)",
        ], "Same rule appears in the 2008 edition (p.159-161, 'a 12-day midpoint moving average'); the 1992 text defines it as the moving average of midpoints. Strict > and <. The book's example market (Swiss franc futures 6/1989-12/1991) is not in the vault, so no book comparison. Run on the index."),
        def("rpdt_3day_hl", "3-day high / 3-day low symmetrical example", "Ch 2 · Step 1", NotTestable("not testable: only the two signal conditions are given ('a buy signal occurs when the price rises through a 3-day high and a sell signal occurs when the price breaks through a 3-day low'); the book does not say whether the sell signal exits or reverses, the order type, or whether today's bar is in the 3-day window"), &[
            "E|\"a buy signal occurs when the price rises through a 3-day high and a sell signal occurs when the price breaks through a 3-day low. This is a symmetrical trading system.\" (p.9)",
        ], "Illustration of a symmetrical system; no exit rule stated."),
        def("rpdt_5dh_ma_asym", "5-day high buy / 5x20 MA sell asymmetrical example", "Ch 2 · Step 1", NotTestable("not testable: an illustration of an asymmetrical system with one buy and one sell condition; exits, reversal and order types are not stated"), &[
            "E|\"a buy signal occurs when a 5-day high is broken, and a sell signal occurs when a 5-day moving average falls below a 20-day moving average. This is an asymmetrical trading system.\" (p.9)",
        ], "Illustration only."),
        def("rpdt_entry_examples", "Example entry rules (RSI < 20, close > yesterday's close + 50% range, ...)", "Ch 3 · Entries and exits", NotTestable("not testable: a list of example entry rules with no exits (only the 5/20 crossover gets an exit example, which is rpdt_ma_xover)"), &[
            "E|\"1. The 5-day moving average crosses from below to above the 20-day moving average. 2. The relative strength index is below 20. 3. The daily close rises by 1%, and the weekly close rises by 1%. 4. Today's close is higher than yesterday's close plus 50% of the daily range. 5. Today's close is higher than the previous 4 closes.\" (p.32-33)",
            "X|\"Close out a long position when the 5-day moving average crosses from above to below the 20-day moving average.\" (p.33)",
        ], "Examples of the definition of an entry rule; rules 2-5 have no exit in the book."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "rpdt_ma_xover" => ("rp_ma", "Pardo two moving-average crossover", "1992 · 5/20 MOO"),
        "rpdt_range_bo" => ("rp_range", "Pardo yesterday's-range breakout", "1992 · 125% both sides"),
        "rpdt_ma_timefilter" => ("rp_ma_tf", "Pardo MA crossover with 2-day time filter", "3/12 midpoint"),
        "rpdt_3day_hl" | "rpdt_5dh_ma_asym" | "rpdt_entry_examples" => ("rp_examples", "Pardo illustrative rules", "not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "rpdt_ma_xover" => Some(vec![p("fast", "MA1 length x (closes)", 5., 1., 20., 1.), p("slow", "MA2 length y (closes, y >= 2x; book test used 40)", 20., 2., 120., 5.)]),
        "rpdt_range_bo" => Some(vec![p("mult", "Breakout as a multiple of yesterday's range (both sides)", 1.25, 0.25, 3., 0.05)]),
        "rpdt_ma_timefilter" => Some(vec![p("fast", "MA1 length (closes)", 3., 1., 10., 1.), p("slow", "MA2 length (midpoints)", 12., 4., 40., 1.), p("days", "Days the cross must hold", 2., 1., 5., 1.)]),
        _ => None,
    }
}

/// Every key known, every value finite and inside its range; integer windows integral; Pardo's Rule 1 (y >= 2x).
pub fn check(schema: &[ParamDef], p: &Params, int_keys: &[&str], two_x: bool) -> Option<String> {
    for (k, &v) in p {
        let Some(d) = schema.iter().find(|d| d.key == k) else { return Some(format!("unknown parameter {k}")) };
        if !v.is_finite() || v < d.min || v > d.max || (int_keys.contains(&k.as_str()) && v.fract() != 0.) {
            return Some(format!("parameter {k} must be within {}..{}", d.min, d.max));
        }
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    if two_x && g("slow") < 2. * g("fast") { return Some("Rule 1: slow (y) is never less than 2 times fast (x)".into()) }
    None
}

pub fn validate(id: &str, p: &Params) -> Option<String> {
    let s = params(id)?;
    match id {
        "rpdt_ma_xover" => check(&s, p, &["fast", "slow"], true),
        "rpdt_ma_timefilter" => check(&s, p, &["fast", "slow", "days"], false),
        _ => check(&s, p, &[], false),
    }
}

fn open_trade(dir: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}

fn finish(mut out: Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) -> Vec<Trade> {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t) }
    out
}

/// Always-in stop-and-reverse at the next open. `want(i)` = side wanted after bar i's close (+1/-1, 0 = no signal).
/// Signals on bars from..to-1 act at the next bar's open; the position still open at `to` is marked at its close.
pub fn sar_next_open(x: &Ctx, from: usize, to: usize, want: impl Fn(usize) -> i8, why: &str) -> Vec<Trade> {
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for i in from..to {
        let w = want(i);
        if w == 0 || cur.as_ref().is_some_and(|t| t.dir == w as f64) { continue }
        if let Some(mut t) = cur.take() { t.exit_i = i + 1; t.exit_px = x.o[i + 1]; t.signal_i = i; t.reason = format!("{why} · reverse next open"); out.push(t) }
        cur = Some(open_trade(w as f64, i + 1, x.o[i + 1], "open", i));
    }
    finish(out, cur, x, to)
}

/// Always-in stop-and-reverse on stops from yesterday's close +/- a multiple of yesterday's range.
/// Gap beyond a level fills at the open; same-bar order follows TradeStation's intrabar path
/// (open nearer the high: O-H-L-C, else O-L-H-C). `strict`: the level must be exceeded (1992 'more than'),
/// otherwise touching is enough (2008 'equal to or more than').
pub fn range_sar(x: &Ctx, from: usize, to: usize, up_m: f64, dn_m: f64, strict: bool) -> Vec<Trade> {
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    let hit = |a: f64, b: f64, lvl: f64| if b >= a { if strict { a <= lvl && b > lvl } else { a < lvl && b >= lvl } } else if strict { a >= lvl && b < lvl } else { a > lvl && b <= lvl };
    for i in from.max(1)..=to {
        let rng = x.h[i - 1] - x.l[i - 1];
        let (up, dn) = (x.c[i - 1] + up_m * rng, x.c[i - 1] - dn_m * rng);
        let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        let path = if h - o <= o - l { [o, h, l, c] } else { [o, l, h, c] };
        let mut flip = |dir: f64, px: f64, cur: &mut Option<Trade>| {
            if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = "opposite stop · reverse".into(); out.push(t) }
            *cur = Some(open_trade(dir, i, px, "stop", i));
        };
        let side = |cur: &Option<Trade>| cur.as_ref().map_or(0., |t| t.dir);
        // at the open: a gap through a level fills at the open
        if side(&cur) != 1. && (if strict { o > up } else { o >= up }) { flip(1., o, &mut cur) }
        else if side(&cur) != -1. && (if strict { o < dn } else { o <= dn }) { flip(-1., o, &mut cur) }
        for k in 0..3 {
            let (a, b) = (path[k], path[k + 1]);
            // a rising leg can only trigger the buy stop, a falling leg only the sell stop
            if b >= a {
                if side(&cur) != 1. && hit(a, b, up) { flip(1., up, &mut cur) }
            } else {
                if side(&cur) != -1. && hit(a, b, dn) { flip(-1., dn, &mut cur) }
            }
        }
    }
    finish(out, cur, x, to)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let s = params(id)?;
    if validate(id, p).is_some() || from >= to || to >= x.r.len() { return None }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    Some(match id {
        "rpdt_ma_xover" => ma_cross(x, from, to, g("fast") as usize, g("slow") as usize),
        "rpdt_range_bo" => range_sar(x, from, to, g("mult"), g("mult"), true),
        "rpdt_ma_timefilter" => {
            let (f, sl, d) = (g("fast") as usize, g("slow") as usize, g("days") as usize);
            let m1 = avg(&x.c, f);
            let mid: Vec<f64> = (0..x.r.len()).map(|i| (x.h[i] + x.l[i]) / 2.).collect();
            let m2 = avg(&mid, sl);
            let held = |i: usize, s: f64| i + 1 >= d && (i + 1 - d..=i).all(|j| m1[j].is_finite() && m2[j].is_finite() && (if s > 0. { above(m1[j], m2[j]) } else { above(m2[j], m1[j]) }));
            sar_next_open(x, from, to, |i| if held(i, 1.) { 1 } else if held(i, -1.) { -1 } else { 0 }, "MA1 vs midpoint MA2 held 2 days")
        }
        _ => return None,
    })
}

/// Strict a > b as the book states it: two averages that are equal in exact arithmetic (float sums can differ
/// by ~1e-15) count as equal, not as a cross (review 25 Oct 2026; before, a rounding difference could trigger a signal).
pub fn above(a: f64, b: f64) -> bool { a - b > 1e-12 * a.abs().max(b.abs()) }
/// Moving average summed afresh over each window (no rolling-sum drift, so `above` can use a 1e-12 tolerance).
pub fn avg(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if n == 0 || i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}

/// Pardo's crossover: MA1(t) > MA2(t) and MA1(t-1) < MA2(t-1) (strict), reverse at the next open.
pub fn ma_cross(x: &Ctx, from: usize, to: usize, fast: usize, slow: usize) -> Vec<Trade> {
    let (a, b) = (avg(&x.c, fast), avg(&x.c, slow));
    sar_next_open(x, from, to, |i| {
        if i == 0 || !(a[i].is_finite() && b[i].is_finite() && a[i - 1].is_finite() && b[i - 1].is_finite()) { return 0 }
        if above(a[i], b[i]) && above(b[i - 1], a[i - 1]) { 1 } else if above(b[i], a[i]) && above(a[i - 1], b[i - 1]) { -1 } else { 0 }
    }, "MA crossover")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 2000 + i), open: v, high: v + 1., low: v - 1., close: v, volume: None }).collect()
    }
    #[test]
    fn crossover_reverses_next_open_and_equality_is_no_cross() {
        // fast=1, slow=2: MA1 = close, MA2 = mean of last 2 closes
        let r = rows(&[10., 9., 8., 9., 10., 9., 9., 8.]);
        let x = Ctx::new(&r, &r, &[]);
        let t = ma_cross(&x, 1, 7, 1, 2);
        // bar3: 9 > 8.5 and bar2 8 < 8.5 -> long at open 4; bar5: 9 < 9.5, bar4 10 > 9.5 -> short at open 6
        assert_eq!(t[0].dir, 1.); assert_eq!(t[0].legs[0].i, 4); assert_eq!(t[0].exit_i, 6);
        assert_eq!(t[1].dir, -1.); assert!(t[1].open);
        assert!(validate("rpdt_ma_xover", &Params::from([("slow".into(), 9.)])).is_some()); // y >= 2x
    }
    #[test]
    fn range_breakout_gap_fills_at_open_and_level_otherwise() {
        let mut r = rows(&[100., 100., 100., 100.]);
        r[2].open = 104.; r[2].high = 105.; // yesterday range 2 -> up = 102.5; gap open 104 > up
        r[3].open = 104.; r[3].low = 98.; r[3].high = 104.5; // dn from bar2: c 100 - 1.25*7 = 91.25, not hit
        let x = Ctx::new(&r, &r, &[]);
        let t = range_sar(&x, 1, 3, 1.25, 1.25, true);
        assert_eq!(t.len(), 1); assert_eq!(t[0].legs[0].px, 104.); assert_eq!(t[0].legs[0].i, 2);
    }
}
