//! Connors Research, "The Bollinger Bands %b Swing Trading System" (May 2004). Book id "pctb".
//! Two systems: the standard %b swing system (Part I) and %b Plus One (Part II, adds a 2nd unit on day 4).
//! Both trade long AND short from one position, so they use a small local simulator (one pass, both
//! directions, close or next-open fills, same-bar add) that returns the engine's `Trade` structure.
#![allow(unused_imports, dead_code)]
use crate::connors::*;

pub const BOOK: &str = "pctb";

const STD: &str = "pbs_percent_b_swing";
const PLUS: &str = "pbs_percent_b_plus_one";

const NOTE: &str = "Bollinger Bands = SMA(Close,5) +/- 1 x POPULATION standard deviation of the last 5 closes (TradeStation BollingerBand/StandardDev type 1, Bollinger's own definition; the book does not say). \
'Closes below 0.2 for 3 days in a row' = %b < 0.2 on today and the 2 prior closes (strict); the signal stays live on day 4, 5... of the streak. \
Trend filter (strict > / < the 200-day SMA) is checked on the signal bar only; the exit ignores it. One position at a time; long and short never overlap (a long exits on the first close with %b > 0.8, which is day 1 of any short streak). \
Default fills at the signal-day close (the book's first option); p.next_open=1 fills at the next open (the book's alternative) and the exit can then fire on the entry day's close. \
No stop, no target, no time exit (none in the book). The book tested the S&P 500 (SPX), Nasdaq-100 (NDX) and Semiconductor (SOX) cash indices in points, 1 unit, no costs, and suggests SPY/QQQ/SMH or their component stocks; the vault runs it on the indices directly (India: NIFTY 50 as the SPX analogue, NIFTY BANK / NIFTY IT / midcap indices as the sector analogues).";

const NOTE_PLUS: &str = "Same rules and conventions as the standard system, plus one extra unit when the streak reaches 4 days. Read as TradeStation pyramiding: the add is its own entry order, so it also needs the 200-day filter on that bar, fires at most once per trade, and fills on the same bar as the first unit if the first signal only came on day 4+. Both units exit together. \
The book's trade counts are per UNIT (TradeStation counts each entry separately): here a trade has 2 legs of weight 0.5 when the add filled, so the book's counts = number of legs.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, rules, notes, claim| StrategyDef {
        id,
        name,
        group,
        session: 0,
        dir: "Long/Short",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules,
        notes,
        claim,
    };
    vec![
        d(STD, "Bollinger Bands %b Swing (standard)", "Part I · Strategy Rules",
          &[
              "F|Bollinger Bands Settings: 5-period Bollinger Bands with a standard deviation of 1",
              "F|%b = (Close - Lower Bollinger Band) / (Upper Bollinger Band - Lower Bollinger Band)",
              "F|Long: the market closes above its 200-day Moving Average (MA)",
              "E|Long: the %b closes below 0.2 for 3 days in a row",
              "E|Long: buy the market at the close (or at the next morning's open)",
              "X|Long: exit at the close when the %b closes above 0.8",
              "F|Short: the market closes below its 200-day MA",
              "E|Short: the %b closes above 0.8 for 3 days in a row",
              "E|Short: sell the market at the close (or at the next morning's open)",
              "X|Short: exit at the close when the %b closes below 0.2",
          ],
          NOTE,
          claim("109", "81.65%", "9.56 pts", "3 / 8 bars (win / loss)", "SPX 1/3/94-4/13/04, index points, PF 3.66")),
        d(PLUS, "%b Plus One Swing", "Part II · %b Plus One",
          &[
              "F|5-period Bollinger Bands with a standard deviation of 1",
              "F|Long: the market closes above its 200-day MA",
              "E|Long: the %b closes below 0.2 for 3 days in a row",
              "E|Long: buy the market at the close (or at the next morning's open)",
              "A|Long: buy an additional unit when the %b closes below 0.2 for 4 days in a row",
              "X|Long: exit at the close when the %b closes above 0.8",
              "F|Short: the market closes below its 200-day MA",
              "E|Short: the %b closes above 0.8 for 3 days in a row",
              "E|Short: sell the market at the close (or at the next morning's open)",
              "A|Short: sell an additional unit when the %b closes above 0.8 for 4 days in a row",
              "X|Short: exit at the close when the %b closes below 0.2",
          ],
          NOTE_PLUS,
          claim("171 (per unit)", "81.29%", "9.81 pts", "3 / 8 bars (win / loss)", "SPX 1/3/94-4/13/04, index points, PF 3.84")),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![
        p("trend", "Trend filter: SMA period", 200.0, 50.0, 300.0, 10.0),
        p("bb_n", "Bollinger Band period", 5.0, 3.0, 20.0, 1.0),
        p("bb_k", "Bollinger Band standard deviations", 1.0, 0.5, 2.5, 0.25),
        p("os", "%b oversold (long entry / short exit)", 0.2, 0.0, 0.4, 0.05),
        p("ob", "%b overbought (short entry / long exit)", 0.8, 0.6, 1.0, 0.05),
        p("days", "Days in a row beyond the threshold", 3.0, 1.0, 6.0, 1.0),
        p("next_open", "Fill: 0 = signal close, 1 = next open", 0.0, 0.0, 1.0, 1.0),
        p("sample_sd", "Std dev: 0 = population, 1 = sample", 0.0, 0.0, 1.0, 1.0),
    ];
    match id {
        STD => {}
        PLUS => v.push(p("add_days", "Add 2nd unit at N days in a row", 4.0, 2.0, 8.0, 1.0)),
        _ => return None,
    }
    Some(v)
}

/// %b with bands = SMA(n) +/- k * stdev of the last n closes (ddof 0 = population, 1 = sample).
fn pct_b_dd(c: &[f64], n: usize, k: f64, ddof: f64) -> Vec<f64> {
    let mut out = vec![f64::NAN; c.len()];
    for i in n.saturating_sub(1)..c.len() {
        let w = &c[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        let sd = (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n as f64 - ddof)).sqrt();
        if sd > 0.0 {
            out[i] = (c[i] - (m - k * sd)) / (2.0 * k * sd);
        }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let pb = pct_b_dd(&x.c, u("bb_n").max(2), g("bb_k"), if g("sample_sd") >= 0.5 { 1.0 } else { 0.0 });
    let ma = x.sma_n(u("trend"));
    let (os, ob) = (g("os"), g("ob"));
    let r = Rules {
        ma: &ma,
        pb: &pb,
        lo: streak(x.c.len(), |i| pb[i] < os),
        hi: streak(x.c.len(), |i| pb[i] > ob),
        os,
        ob,
        days: u("days"),
        add_days: if id == PLUS { u("add_days") } else { usize::MAX },
        next_open: g("next_open") >= 0.5,
    };
    Some(sim(x, from, to, &r))
}

struct Rules<'a> {
    ma: &'a [f64],
    pb: &'a [f64],
    lo: Vec<usize>, // consecutive closes with %b < os
    hi: Vec<usize>, // consecutive closes with %b > ob
    os: f64,
    ob: f64,
    days: usize,
    add_days: usize, // usize::MAX = no add (standard system)
    next_open: bool,
}

impl Rules<'_> {
    /// Streak length in direction dir on bar i if the trend filter holds there, else 0.
    fn run_len(&self, x: &Ctx, i: usize, dir: f64) -> usize {
        if dir > 0.0 && x.c[i] > self.ma[i] {
            self.lo[i]
        } else if dir < 0.0 && x.c[i] < self.ma[i] {
            self.hi[i]
        } else {
            0
        }
    }
}

/// One position at a time, long or short. Orders decided on bar i's close fill at close[i]
/// (default) or open[i+1]; exits at the close when %b crosses the opposite threshold.
fn sim(x: &Ctx, from: usize, to: usize, r: &Rules) -> Vec<Trade> {
    let w = if r.add_days == usize::MAX { 1.0 } else { 0.5 };
    let mut trades = Vec::new();
    let mut cur: Option<Trade> = None;
    let mut pend_entry: Option<(f64, usize)> = None; // (dir, signal bar) for next-open fills
    let mut pend_add = false;
    for i in from..=to {
        if let Some((dir, sig)) = pend_entry.take() {
            cur = Some(Trade { dir, legs: vec![Leg { i, px: x.o[i], w }], exit_i: i, exit_px: x.o[i], reason: String::new(), entry_kind: "open", signal_i: sig, open: false, exits: Vec::new() });
        }
        if std::mem::take(&mut pend_add) {
            if let Some(t) = cur.as_mut() {
                t.legs.push(Leg { i, px: x.o[i], w: 0.5 });
            }
        }
        // exit at the close (never on the bar whose close opened the trade)
        if let Some(t) = cur.as_mut() {
            let out = if t.dir > 0.0 { r.pb[i] > r.ob } else { r.pb[i] < r.os };
            if out && !(t.legs[0].i == i && !r.next_open) {
                t.exit_i = i;
                t.exit_px = x.c[i];
                t.signal_i = i;
                t.reason = if t.dir > 0.0 { format!("%b {:.2} > {}", r.pb[i], r.ob) } else { format!("%b {:.2} < {}", r.pb[i], r.os) };
                trades.push(cur.take().unwrap());
            }
        }
        // entry from flat
        if cur.is_none() && i < to {
            if let Some(dir) = [1.0, -1.0].into_iter().find(|&d| r.run_len(x, i, d) >= r.days) {
                if r.next_open {
                    pend_entry = Some((dir, i));
                } else {
                    cur = Some(Trade { dir, legs: vec![Leg { i, px: x.c[i], w }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: Vec::new() });
                }
            }
        }
        // Plus One: 2nd unit once the streak reaches add_days (its own order: trend filter applies; once per trade)
        let pos = cur.as_ref().map(|t| (t.dir, t.legs.len())).or(pend_entry.map(|(d, _)| (d, 1)));
        if let Some((dir, 1)) = pos {
            if (i < to || !r.next_open) && r.run_len(x, i, dir) >= r.add_days {
                if r.next_open {
                    pend_add = true;
                } else if let Some(t) = cur.as_mut() {
                    t.legs.push(Leg { i, px: x.c[i], w: 0.5 });
                }
            }
        }
    }
    if let Some(mut t) = cur {
        t.exit_i = to;
        t.exit_px = x.c[to];
        t.reason = "open".into();
        t.signal_i = to;
        t.open = true;
        trades.push(t);
    }
    trades
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let fam = ("pbs", "Bollinger Bands %b Swing (Connors Research 2004)");
    match id {
        STD => Some((fam.0, fam.1, "Standard")),
        PLUS => Some((fam.0, fam.1, "Plus One (add a unit on day 4)")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    /// Deterministic random walk with a drift, long enough for the 200-day MA.
    fn rows() -> Vec<Row> {
        let (mut c, mut s) = (100.0f64, 7u64);
        (0..1500)
            .map(|k| {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                c *= 1.0 + 0.0004 + ((s >> 33) as f64 / (1u64 << 31) as f64 - 0.5) * 0.03;
                Row { ticker: "T".into(), date: format!("{:04}-01-01", 1000 + k), open: c, high: c, low: c, close: c, volume: None }
            })
            .collect()
    }

    #[test]
    fn pctb_hand_calc_and_rules_match_a_direct_scan() {
        // %b of 1,2,3,4,5 (mean 3, population sd sqrt(2)) at the last close: 0.5 + 2 / (2 sqrt 2)
        let pb = pct_b_dd(&[1.0, 2.0, 3.0, 4.0, 5.0], 5, 1.0, 0.0);
        assert!((pb[4] - (0.5 + 1.0 / 2f64.sqrt())).abs() < 1e-12);
        let r = rows();
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(pct_b_dd(&x.c, 5, 1.0, 0.0)[300..], x.pctb5[300..]); // same as the engine's population %b
        let (pb, ma, to) = (&x.pctb5, &x.ma200, r.len() - 1);
        let t = run(STD, &x, 0, to, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().any(|t| t.dir > 0.0) && t.iter().any(|t| t.dir < 0.0));
        // direct scan of the book's rules: flat -> 3 closes beyond the band + trend; exit on the opposite threshold
        let mut i = 0;
        for tr in &t {
            let e = (i..to).find(|&k| k >= 2 && ((x.c[k] > ma[k] && (k - 2..=k).all(|j| pb[j] < 0.2)) || (x.c[k] < ma[k] && (k - 2..=k).all(|j| pb[j] > 0.8)))).unwrap();
            assert_eq!(tr.legs[0].i, e);
            assert_eq!(tr.dir, if pb[e] < 0.2 { 1.0 } else { -1.0 });
            let ex = (e + 1..=to).find(|&k| if tr.dir > 0.0 { pb[k] > 0.8 } else { pb[k] < 0.2 }).unwrap_or(to);
            assert_eq!(tr.exit_i, ex);
            i = ex;
        }
        // Plus One: same trades; a 2nd leg exactly when the streak reaches 4 inside the trade with the trend filter on
        let p = run(PLUS, &x, 0, to, &Params::new()).unwrap();
        assert_eq!(p.len(), t.len());
        for (a, b) in p.iter().zip(&t) {
            assert_eq!((a.legs[0].i, a.exit_i), (b.legs[0].i, b.exit_i));
            let want = (a.legs[0].i..a.exit_i).find(|&k| {
                let n = if a.dir > 0.0 { (0..4).all(|j| pb[k - j] < 0.2) && x.c[k] > ma[k] } else { (0..4).all(|j| pb[k - j] > 0.8) && x.c[k] < ma[k] };
                n
            });
            assert_eq!(a.legs.get(1).map(|l| l.i), want);
        }
    }
}
