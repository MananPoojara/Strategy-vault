//! Active Trader Magazine, 2000 volume (Apr-Nov issues): the monthly "Trading System Lab" columns (Thomas Stridsman).
//! Book id `atm00`. Source copies: Books/Active Trader Magazine/2000/2000 MM.pdf (TSL page of each issue).
//! Transcription, testability table and book-vs-ours checks: library/atm00/README.md.
//!
//! The columns tested portfolios (S&P 500 futures, Dow / Nasdaq-100 stocks, currency futures) with fixed-fractional
//! sizing in one shared account. The vault runs each system on one index at a time with full equity per trade
//! (instrument swap only: SPX/NDX/DJI, NIFTY 50; stocks via basket:<list>:<SYMBOL> tickers). Sizing rules are noted,
//! not modelled.
//!
//! Shared conventions (book silent, neutral, used by atm00/atm01/atm02 through `sim`):
//! - a stop/limit level is filled at the level when touched, or at the open when the bar gaps through it;
//! - price-level exits (stops, targets) work from the bar AFTER the entry bar (TradeStation next-bar orders);
//!   close-of-bar exit conditions may fire on the entry bar when the column says so;
//! - stop before target when both lie inside one bar (the April 2001 column states this convention explicitly);
//! - if a long and a short entry order would both fill on one bar, the one nearer the open is taken;
//! - after an exit on a bar, only an at-the-close entry can open a new trade on that bar.
//! Duplicates already in the vault (not repeated here): Hybrid System No. 1 (Sep 2000) = strmm_hybrid with its
//! "original rules" parameters; RS System No. 1 (Oct 2000) = strmm_rs1 original-rules parameters.
use crate::connors::{sma, true_range, wilder, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "atm00";

// ------------------------------------------------------------------ shared helpers (used by atm01 / atm02)

pub(crate) fn hi_n(x: &Ctx, i: usize, n: usize) -> f64 { x.h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max) }
pub(crate) fn lo_n(x: &Ctx, i: usize, n: usize) -> f64 { x.l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min) }

/// Monday-based week number of a YYYY-MM-DD date.
pub(crate) fn week_of(date: &str) -> i64 {
    let f = |a: usize, b: usize| date.get(a..b).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let (y, m, d) = (f(0, 4), f(5, 7), f(8, 10));
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + d - 1;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    (days + 3).div_euclid(7)
}

/// For each bar: (close of the previous completed week, close of the week before that). Week close = last daily close.
pub(crate) fn prev_week_closes(x: &Ctx) -> Vec<(f64, f64)> {
    let n = x.c.len();
    let mut out = vec![(f64::NAN, f64::NAN); n];
    let (mut p1, mut p2) = (f64::NAN, f64::NAN);
    for i in 0..n {
        if i > 0 && week_of(&x.r[i].date) != week_of(&x.r[i - 1].date) { p2 = p1; p1 = x.c[i - 1]; }
        out[i] = (p1, p2);
    }
    out
}

/// TradeStation AverageTrueRange: simple average of TrueRange.
pub(crate) fn avg_tr(x: &Ctx, n: usize) -> Vec<f64> { sma(&true_range(x.r), n) }

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Ord { Open, Close, Stop(f64), Limit(f64) }

#[derive(Clone, Copy, Debug)]
pub(crate) struct Pos { pub dir: f64, pub ei: usize, pub epx: f64, pub sig: usize, pub kind: &'static str, pub hh: f64, pub ll: f64 }

/// Fill price of an entry order on bar t (None = not filled).
pub(crate) fn fill(x: &Ctx, t: usize, dir: f64, o: Ord) -> Option<f64> {
    let (op, h, l) = (x.o[t], x.h[t], x.l[t]);
    match o {
        Ord::Open => Some(op),
        Ord::Close => Some(x.c[t]),
        Ord::Stop(s) if s.is_finite() => if dir > 0.0 { (h >= s).then(|| op.max(s)) } else { (l <= s).then(|| op.min(s)) },
        Ord::Limit(p) if p.is_finite() => if dir > 0.0 { (l <= p).then(|| op.min(p)) } else { (h >= p).then(|| op.max(p)) },
        _ => None,
    }
}

/// Protective stop / profit target on bar t for a position of direction `dir`: gap through first (at the open), then
/// stop before target.
pub(crate) fn levels(x: &Ctx, t: usize, dir: f64, stop: Option<f64>, tgt: Option<f64>) -> Option<(f64, &'static str)> {
    let (o, h, l) = (x.o[t], x.h[t], x.l[t]);
    let stop = stop.filter(|s| s.is_finite());
    let tgt = tgt.filter(|g| g.is_finite());
    if let Some(s) = stop { if (dir > 0.0 && o <= s) || (dir < 0.0 && o >= s) { return Some((o, "stop (gap at open)")); } }
    if let Some(g) = tgt { if (dir > 0.0 && o >= g) || (dir < 0.0 && o <= g) { return Some((o, "target (gap at open)")); } }
    if let Some(s) = stop { if (dir > 0.0 && l <= s) || (dir < 0.0 && h >= s) { return Some((s, "stop")); } }
    if let Some(g) = tgt { if (dir > 0.0 && h >= g) || (dir < 0.0 && l <= g) { return Some((g, "target")); } }
    None
}

fn trade(p: &Pos, i: usize, px: f64, reason: &str, sig: usize) -> Trade {
    Trade { dir: p.dir, legs: vec![Leg { i: p.ei, px: p.epx, w: 1.0 }], exit_i: i, exit_px: px, reason: reason.into(),
        entry_kind: p.kind, signal_i: sig, open: false, exits: vec![] }
}

/// One-position simulator. `ent(t)` = entry orders working on bar t (direction, order) built from data the trader
/// knows when the order is placed (bars < t, plus o[t] for orders conditioned on the open; bar t itself only for
/// Ord::Close). `ex(t, pos)` = exit on bar t (price, reason); it is also called on the entry bar after the fill, where
/// it must only return close-of-bar exits the column applies on the entry day.
pub(crate) fn sim(x: &Ctx, from: usize, to: usize, ent: &dyn Fn(usize) -> Vec<(f64, Ord)>,
                  ex: &dyn Fn(usize, &Pos) -> Option<(f64, &'static str)>) -> Vec<Trade> {
    let mut out = vec![];
    let mut pos: Option<Pos> = None;
    for t in from.max(1)..=to {
        let mut exited = false;
        if let Some(mut p) = pos {
            if let Some((px, why)) = ex(t, &p) {
                let sig = if why.contains("next open") { t - 1 } else { t };
                out.push(trade(&p, t, px, why, sig));
                pos = None;
                exited = true;
            } else {
                p.hh = p.hh.max(x.h[t]);
                p.ll = p.ll.min(x.l[t]);
                pos = Some(p);
            }
        }
        if pos.is_none() {
            let mut best: Option<(f64, f64, Ord)> = None; // (dir, px, ord)
            for (d, o) in ent(t) {
                if exited && o != Ord::Close { continue; }
                if let Some(px) = fill(x, t, d, o) {
                    let near = |q: f64| (q - x.o[t]).abs();
                    if best.map_or(true, |(_, bp, _)| near(px) < near(bp)) { best = Some((d, px, o)); }
                }
            }
            if let Some((d, px, o)) = best {
                let kind = match o { Ord::Open => "open", Ord::Close => "close", Ord::Stop(_) => "stop", Ord::Limit(_) => "limit" };
                let sig = if o == Ord::Close { t } else { t - 1 };
                let p = Pos { dir: d, ei: t, epx: px, sig, kind, hh: x.h[t], ll: x.l[t] };
                if let Some((epx, why)) = ex(t, &p) {
                    out.push(trade(&p, t, epx, why, t));
                } else {
                    pos = Some(p);
                }
            }
        }
    }
    if let Some(p) = pos {
        let mut tr = trade(&p, to, x.c[to], "open", to);
        tr.open = true;
        out.push(tr);
    }
    out
}

/// Wilder +DI / -DI (TradeStation DMIPlus / DMIMinus) over n bars.
fn di(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let (mut p, mut m, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    let trs = true_range(x.r);
    for i in 1..len {
        let up = x.h[i] - x.h[i - 1];
        let dn = x.l[i - 1] - x.l[i];
        p[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        m[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&p, n), wilder(&m, n), wilder(&tr, n));
    let f = |a: &[f64]| (0..len).map(|i| if st[i] > 0.0 { 100.0 * a[i] / st[i] } else { f64::NAN }).collect::<Vec<_>>();
    (f(&sp), f(&sm))
}

// ------------------------------------------------------------------ catalogue

macro_rules! def {
    ($id:expr, $name:expr, $g:expr, $dir:expr, $st:expr, $vol:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $g, session: 0, dir: $dir, status: $st, needs_volume: $vol, needs_vix: false,
            needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
}
fn cl(t: &'static str, w: &'static str, a: &'static str, d: &'static str, p: &'static str) -> Option<Claim> {
    Some(Claim { trades: t, win: w, avg: a, hold: d, period: p })
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        def!("atm00_gold_digger", "Gold Digger system (Apr 2000 TSL)", "Trading System Lab · Apr 2000", "Long/Short", Live, false,
            &["E|\"Go long if the market closes down two days in a row in the same week (following a down week), except if this coincides with the market setting a new 60-day low (which suggests a possible downside breakout). Reverse the rules for the short side.\" (TSL p.54)",
              "X|\"Stop out immediately if the market moves against you 1.1 percent; lock in a profit after a move of 0.6 percent or more in your favor; exit immediately if market moves 2.8 percent in your favor. Exit any open positions after eight days.\" (TSL p.54); companion article: \"Always lock in a profit of 0.6 percent\" (p.42, 49)",
              "X|Read: lock = stop at entry +/- 0.6% once the trade has moved 0.6% in favour (the companion article's \"lock in a profit of 0.6 percent\"); trigger = an intrabar touch (p.lock_close=1: a close beyond it); no reading reproduces the printed trade count (see README)",
              "E|Read: entry at the close of the second down day, as the companion article's strategy enters \"at the close\" (p.entry_open=1: next open); down week = last close of the previous calendar week below the one before; new 60-day low = low below the lowest low of the prior 60 bars (p.low_basis=1: close)",
              "X|Read: exit at the close of the 8th bar after an at-close entry (8th day counting the entry day for an open entry)"],
            "Book: S&P 500 futures, Oct 9 1990-Oct 27 1999, 3% of equity risked per trade, $50 costs (TSL p.54). The vault runs SPX (cash index) for the comparison; India: NIFTY 50.",
            cl("474", "55.91%", "$610", "5.00 days", "S&P 500 futures 10/1990-10/1999")),
        def!("atm00_lucy", "Lucy stock picking system (May 2000 TSL)", "Trading System Lab · May 2000", "Long",
            NotTestable("\"after a retracement of at least 2.5 percent\" names no reference point or lookback for the retracement, and the \"lock in a profit after a move of 0.6 percent\" exit gives no locked level in this column (TSL p.51)"), false,
            &["E|\"Enter on the close (long side only) if: 1. Today's low is 0.2 percent lower than yesterday's low and 2. today's close is 0.3 percent higher than yesterday's close, but only if the market is above its 350-day moving average and after a retracement of at least 2.5 percent.\" (TSL p.51)",
              "X|\"1. Stop out immediately if the market moves against you 1.6 percent; 2. lock in a profit after a move of 0.6 percent or more in your favor and 3. exit immediately if market moves 3 percent in your favor. 4. Exit any open positions after nine days.\" (TSL p.51)"],
            "Book: all Dow stocks, Jan 1995-Feb 2000: 3,201 trades, 52.92% winners, avg $64, 2.77 days.", None),
        def!("atm00_fourplay", "Fourplay pyramiding strategy (Jun 2000 TSL)", "Trading System Lab · Jun 2000", "Long", Live, false,
            &["E|\"Enter on the close (long side only) if: today's close crosses above its 50-day moving average.\" (TSL p.49)",
              "A|\"Add the same number of shares/contracts as in the initial entry ... as soon as the market moves 1 percent in your favor. Continue to add the same number ... as soon as the market moves twice the distance of the last add-on (i.e., as soon as the market has moved 2 percent, 4 percent, 8 percent, 16 percent and so on from the initial entry).\" (TSL p.49)",
              "X|\"Exit on the close on a cross below the 50-day moving average.\" (TSL p.49)",
              "A|Read: each add is a buy stop at entry x (1 + 1%, 2%, 4%, ...) from the bar after entry (gap = open); every add has the initial share count, so leg weight = add price / entry price (initial leg = 1, i.e. adds lever the position)"],
            "Book: all Dow stocks, Jan 1990-Feb 2000, 100 shares per entry/add, no costs: 7,962 entries incl. adds / 2,904 initial entries (TSL p.49). India: NIFTY 50 / basket stocks.",
            cl("2,904 initial (7,962 incl. adds)", "n/a", "$60 per initial entry", "n/a", "30 Dow stocks 1/1990-2/2000")),
        def!("atm00_bear_claw", "Bear claw (Jul 2000 TSL)", "Trading System Lab · Jul 2000", "Short", Live, false,
            &["F|\"DMI+ and DMI- indicators (lookback periods: 14 and 8 days, respectively)\" (TSL p.52) - Wilder's directional indicators",
              "E|\"Enter on the close (short side only) if: 1. Today's close is below the 200-day moving average. 2. The DMI- line is above the DMI+ line ... 3. The DMI+ value is lower today than yesterday\" (TSL p.52)",
              "X|\"Exit on the close if the DMI+ value is higher today than yesterday, indicating increasing upside volatility.\" (TSL p.52)",
              "X|The \"highest high of the last two days\" is only the sizing distance (0.5% risk), not a stop - no stop is stated."],
            "Book: currency futures (euro, yen, A$, C$), 1981-1999: 1,088 trades, 48.44% winners, avg $34, 3 days. Not an index result; not compared.", None),
        def!("atm00_ma_9_18", "Basic moving average crossover 9/18 (Aug 2000 TSL)", "Trading System Lab · Aug 2000", "Long", Live, false,
            &["E|\"1. Enter on the close when a nine-day moving average crosses above the 18-day moving average.\" (TSL p.53)",
              "X|\"2. Exit on the close when the nine-day moving average crosses below the 18-day moving average.\" \"we traded only long signals with a fixed lot size of 100 shares\" (TSL p.53)"],
            "Book: all Dow stocks, Feb 1990-Feb 2000: 2,365 trades, 38.52% winners, avg $10, 19 days (TSL p.53).",
            cl("2,365", "38.52%", "$10", "19 days", "30 Dow stocks 2/1990-2/2000")),
        def!("atm00_pullback1", "Pullback system No. 1 (Nov 2000 TSL)", "Trading System Lab · Nov 2000", "Long", Live, false,
            &["E|\"1. The market must make two consecutive lower closes. 2. The high of the next bar must be higher than the previous high. 3. Enter long at the next open. Filter: The close of the bar in step 2 must be above the 50-day moving average\" (TSL p.44)",
              "X|\"1. Exit at the open when the position shows a profit at the previous day's close. 2. Stop-loss: Exit with a loss if the market falls below the lowest low of the last two days.\" (TSL p.44) - \"the stop-loss is simply placed at the low of the pullback\": fixed at the lowest low of the signal bar and the bar before it"],
            "Book: 30 Nasdaq-100 stocks, Dec 1985-Aug 2000, 4% risk per trade: win 65%, avg 2.9 days, 3.7 trades/market/year (TSL p.44).",
            cl("n/a (3.7 per market per year)", "65%", "$3,091", "2.9 days", "30 Nasdaq-100 stocks 12/1985-8/2000")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some(("atm00_tsl", "Active Trader · Trading System Lab 2000", d.name))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "atm00_gold_digger" => vec![p("stop_pct", "Stop loss %", 1.1, 0.2, 5., 0.1), p("lock_pct", "Lock-in profit %", 0.6, 0.1, 3., 0.1),
            p("tgt_pct", "Profit target %", 2.8, 0.5, 10., 0.1), p("hold", "Exit after days", 8., 2., 20., 1.), p("low_n", "New-low exception days", 60., 10., 120., 5.),
            p("entry_open", "Entry: 0 at the close, 1 next open", 0., 0., 1., 1.), p("lock_close", "Lock trigger: 0 intrabar, 1 close", 0., 0., 1., 1.),
            p("low_basis", "New 60-day low by: 0 low, 1 close", 0., 0., 1., 1.)],
        "atm00_fourplay" => vec![p("ma", "Moving average days", 50., 10., 200., 5.), p("add_pct", "First add-on %", 1., 0.25, 5., 0.25)],
        "atm00_bear_claw" => vec![p("ma", "Trend MA days", 200., 50., 300., 10.), p("pdi_n", "DMI+ days", 14., 5., 30., 1.), p("mdi_n", "DMI- days", 8., 3., 30., 1.)],
        "atm00_ma_9_18" => vec![p("fast", "Short MA days", 9., 2., 50., 1.), p("slow", "Long MA days", 18., 5., 200., 1.)],
        "atm00_pullback1" => vec![p("ma", "Trend MA days", 50., 10., 200., 5.), p("stop_n", "Stop: lowest low of days", 2., 1., 10., 1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("atm00_") { return None; }
    let schema = params(id)?;
    if from >= to || to >= x.r.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let (c, o, h, l) = (&x.c, &x.o, &x.h, &x.l);
    Some(match id {
        "atm00_gold_digger" => {
            let wc = prev_week_closes(x);
            let (sp, lp, tp, hold, ln) = (g("stop_pct") / 100., g("lock_pct") / 100., g("tgt_pct") / 100., u("hold"), u("low_n"));
            let (eo, lock_close, lowc) = (g("entry_open") >= 0.5, g("lock_close") >= 0.5, g("low_basis") >= 0.5);
            let sig = |s: usize| -> f64 {
                if s < ln + 2 { return 0.0; }
                let same_week = week_of(&x.r[s].date) == week_of(&x.r[s - 1].date);
                let (w1, w2) = wc[s];
                if !same_week || !(w1.is_finite() && w2.is_finite()) { return 0.0; }
                let (lo_ref, hi_ref) = if lowc { (c[s - ln..s].iter().cloned().fold(f64::MAX, f64::min), c[s - ln..s].iter().cloned().fold(f64::MIN, f64::max)) }
                                       else { (lo_n(x, s - 1, ln), hi_n(x, s - 1, ln)) };
                let (new_lo, new_hi) = if lowc { (c[s] < lo_ref, c[s] > hi_ref) } else { (l[s] < lo_ref, h[s] > hi_ref) };
                if c[s] < c[s - 1] && c[s - 1] < c[s - 2] && w1 < w2 && !new_lo { 1.0 }
                else if c[s] > c[s - 1] && c[s - 1] > c[s - 2] && w1 > w2 && !new_hi { -1.0 } else { 0.0 }
            };
            sim(x, from, to, &|t| {
                if eo { let d = sig(t - 1); if d != 0.0 { return vec![(d, Ord::Open)]; } }
                else { let d = sig(t); if d != 0.0 { return vec![(d, Ord::Close)]; } }
                vec![]
            }, &|t, q| {
                let days = t - q.ei + usize::from(q.kind == "open");
                if t > q.ei {
                    let lock_lvl = q.epx * (1.0 + q.dir * lp);
                    let touched = |k: usize| if q.dir > 0.0 { h[k] >= lock_lvl } else { l[k] <= lock_lvl };
                    // lock armed once an earlier bar of the trade reached it intrabar (or closed beyond it, p.lock_close)
                    let k0 = if q.kind == "open" { q.ei } else { q.ei + 1 };
                    let armed = (k0..t).any(|k| if lock_close { q.dir * (c[k] - lock_lvl) >= 0.0 } else { touched(k) });
                    let base_stop = q.epx * (1.0 - q.dir * sp);
                    let stop = if armed { if q.dir > 0.0 { base_stop.max(lock_lvl) } else { base_stop.min(lock_lvl) } } else { base_stop };
                    if let Some((px, why)) = levels(x, t, q.dir, Some(stop), Some(q.epx * (1.0 + q.dir * tp))) {
                        return Some((px, if why.starts_with("stop") && armed { "lock-in stop" } else { why }));
                    }
                    // armed on this bar intrabar and closed back through the lock: the lock was hit after the touch
                    if !armed && !lock_close && touched(t) && q.dir * (c[t] - lock_lvl) < 0.0 {
                        return Some((lock_lvl, "lock-in stop (same bar)"));
                    }
                }
                if days >= hold && t > q.ei { return Some((c[t], "time exit (8 days)")); }
                None
            })
        }
        "atm00_fourplay" => {
            let m = sma(c, u("ma"));
            let a1 = g("add_pct") / 100.;
            let cross_up = |s: usize| s >= 1 && c[s] > m[s] && c[s - 1] <= m[s - 1];
            let cross_dn = |s: usize| s >= 1 && c[s] < m[s] && c[s - 1] >= m[s - 1];
            let mut out = vec![];
            let mut cur: Option<(usize, f64, Vec<Leg>, f64)> = None; // (entry bar, entry px, legs, next add %)
            for t in from.max(1)..=to {
                if let Some((ei, epx, mut legs, mut nxt)) = cur.take() {
                    if t > ei {
                        // add-on buy stops at entry x (1 + nxt), doubling the distance after each add
                        loop {
                            let lvl = epx * (1.0 + nxt);
                            if h[t] < lvl { break; }
                            let px = o[t].max(lvl);
                            legs.push(Leg { i: t, px, w: px / epx });
                            nxt *= 2.0;
                        }
                    }
                    if cross_dn(t) {
                        out.push(Trade { dir: 1.0, legs, exit_i: t, exit_px: c[t], reason: "close crosses below MA".into(), entry_kind: "close",
                            signal_i: t, open: false, exits: vec![] });
                        continue;
                    }
                    cur = Some((ei, epx, legs, nxt));
                } else if cross_up(t) {
                    cur = Some((t, c[t], vec![Leg { i: t, px: c[t], w: 1.0 }], a1));
                }
            }
            if let Some((_, _, legs, _)) = cur {
                out.push(Trade { dir: 1.0, legs, exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
            }
            out
        }
        "atm00_bear_claw" => {
            let m = sma(c, u("ma"));
            let (pd, _) = di(x, u("pdi_n"));
            let (_, md) = di(x, u("mdi_n"));
            sim(x, from, to, &|t| {
                if t >= 1 && c[t] < m[t] && md[t] > pd[t] && pd[t] < pd[t - 1] { vec![(-1.0, Ord::Close)] } else { vec![] }
            }, &|t, q| (t > q.ei && pd[t] > pd[t - 1]).then(|| (c[t], "DMI+ rising (exit on close)")))
        }
        "atm00_ma_9_18" => {
            let (f, s) = (sma(c, u("fast")), sma(c, u("slow")));
            sim(x, from, to, &|t| {
                if t >= 1 && f[t] > s[t] && f[t - 1] <= s[t - 1] { vec![(1.0, Ord::Close)] } else { vec![] }
            }, &|t, q| (t > q.ei && f[t] < s[t] && f[t - 1] >= s[t - 1]).then(|| (c[t], "MA cross below (exit on close)")))
        }
        "atm00_pullback1" => {
            let m = sma(c, u("ma"));
            let sn = u("stop_n");
            sim(x, from, to, &|t| {
                if t < 4 { return vec![]; }
                let s = t - 1; // bar of step 2
                if c[s - 1] < c[s - 2] && c[s - 2] < c[s - 3] && h[s] > h[s - 1] && c[s] > m[s] { vec![(1.0, Ord::Open)] } else { vec![] }
            }, &|t, q| {
                if t == q.ei { return None; }
                if c[t - 1] > q.epx { return Some((o[t], "first profitable close: exit next open")); }
                let stop = lo_n(x, q.sig, sn);
                levels(x, t, 1.0, Some(stop), None)
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i + 1),
            open: o, high: h, low: l, close: c, volume: Some(1.0) }).collect()
    }
    #[test]
    fn pullback1_enters_next_open_and_exits_open_after_first_profitable_close() {
        // 60 flat bars at 100 (MA50 = 100), then two lower closes, a higher high closing above the MA, entry next open.
        let mut v: Vec<(f64, f64, f64, f64)> = (0..60).map(|_| (100., 101., 99., 100.)).collect();
        v.push((100., 101., 99., 99.5)); // lower close 1
        v.push((99.5, 100., 98., 99.0)); // lower close 2
        v.push((99.0, 100.5, 98.5, 100.4)); // high 100.5 > 100, close 100.4 > MA
        v.push((100.2, 101., 100., 100.8)); // entry at open 100.2, closes in profit
        v.push((101., 102., 100.5, 101.5)); // exit at this open 101
        let r = rows(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("atm00_pullback1", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px), (63, 100.2));
        assert_eq!((t[0].exit_i, t[0].exit_px), (64, 101.0));
    }
}
