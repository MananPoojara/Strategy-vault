//! Van Tharp: Trade Your Way to Financial Freedom (1st ed., 1999), Super Trader (2009),
//! Developing a Winning Trading System That Fits You (Systems Development Workbook, 2004),
//! Special Report on Money Management. Source books remain distinct. The two internal
//! runnable demonstrations still require instrument and execution review before admission.
//! Sources and bounded content review: library/tharp/.
use crate::connors::{true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "tharp";

/// Admitted 2026-10-24 (authors/tharp/admission/SOURCE-CONTRACT.md): independent ledgers, US and India, 0 mismatches.
pub const REVIEWED: &[&str] = &["tharp_random_entry", "tharp_55_21_breakout"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: match id {
            "tharp_false_breakout" => "tharp_system_workbook",
            "tharp_110day_breakout" => "tharp_super_trader",
            _ => BOOK,
        }, rules, notes, claim: None,
    };
    vec![
        make("tharp_random_entry", "Random entry + 3x volatility trailing stop", "TYW ch. 8 · Trying to beat random entry", "Long/Short", Status::Live,
            &["E|\"he was always in the market, either long or short, based upon a random signal. As soon as he got an exit signal, he'd reenter the market again based upon the random signal.\" (TYW p.200)",
              "E|Coin flip: fixed-seed splitmix64 draw per entry, top bit 1 = long, 0 = short. [Not from the book: a reproducible stand-in for the coin; vary p.seed for other runs]",
              "E|Enter at the close of the first bar of the test window, and again at the close of every stop-out day (always in the market). [Book silent on the fill; daily-bar reading of 'as soon as']",
              "F|TYW p.200 specifies a 10-day exponential average of true range. This implementation uses ordinary EMA alpha = 2/(n+1), default n=10; the first n true ranges seed its mean. Seed choice is a declared implementation convention. Wilder-10 (alpha 0.1), Workbook EMA-19 and Super Trader's 20-day ATR are separate source specifications, not equivalent TYW settings.",
              "X|TYW p.200: the initial stop is three times that source volatility reading. Workbook p.83 has a separate EMA-19 volatility specification and is not this TYW definition.",
              "X|\"the same three-times-volatility stop was trailed from the close. However, the stop could only move in our favor.\" (TYW p.200). Recomputed after each close: long stop = max(stop, close - 3V), short = min(stop, close + 3V).",
              "X|Stop is a resting order: long exits when low <= stop, short when high >= stop; fill at the stop, or at the open if the bar gaps through it. Not checked on the entry bar. [Book silent; vault convention]",
              "A|\"We also used a 1 percent risk model for our position-sizing system\" (TYW p.200): position = 1% of equity / (3V / entry price) as a fraction of equity. p.risk_pct = 0 gives the one-unit (full-equity) run."],
            "TYW p.199-202, 257-258. The supplied source's 10-day exponential average has ordinary EMA alpha 2/11; the former Wilder-10 implementation was not equivalent. Workbook EMA-19 and Super Trader's 20-day ATR are distinct sources and are not silently selected by changing this EMA period. The reported 10-futures, $100-cost, multi-run and 1%-account-risk experiments are not reproduced by one price series and a chosen RNG seed. Admitted 2026-10-24 as a declared application on indices, ETFs and stocks (US and India): one price series per run, the 1% risk weight as printed, no costs; the book's futures portfolio results are not claimed."),
        make("tharp_55_21_breakout", "55/21-day channel breakout", "TYW ch. 12 · The system used for position sizing", "Long/Short", Status::Live,
            &["E|\"it enters the market on a stop order if the market makes a new 55-day high (long) or a new 55-day low (short).\" (TYW p.286; Special Report p.15). Long trigger: high > highest high of the previous 55 bars, fill at that level or the open if gapped above; short symmetric.",
              "X|\"The stop, for both the initial risk and profit taking, is a 21-day trailing stop on the other side of the market. ... if you go long and the market hits a 21-day low, you exit. If you are short and the market makes a new 21-day high, you exit.\" (TYW p.286)",
              "X|\"This stop is recalculated each day, and it is always moved in your favor\" (TYW p.286): long stop = max(stop, lowest low of the previous 21 bars); exit when low <= stop, fill at stop or the open if gapped below. Short symmetric. Not checked on the entry bar.",
              "E|After an exit the system waits for the next 55-day breakout; an opposite breakout on the exit bar enters at once. A bar that breaks both 55-day extremes while flat is skipped (order unknown). [Book silent]",
              "A|Sizing (non-default variants from ch. 12): p.sizing=1 percent risk (Model 3: weight = pct% x entry / |entry - stop|), p.sizing=2 percent volatility (Model 4: weight = pct% x entry / 20-day SMA of true range at the prior close). Default p.sizing=0 = full equity per trade (book reports no single default model)."],
            "TYW p.286-297 (Tables 12-2, 12-4, 12-6); Special Report on Money Management (same system, same tables); Workbook p.111-113. Book results: 10 commodities 1981-1991, $1M start, one table per sizing model; not reproducible on a single index. A one-symbol unit-exposure OHLC diagnostic would be a separate market adaptation. Admitted 2026-10-24 as a declared application on indices, ETFs and stocks (US and India) with one unit per trade (sizing models as options); the book's futures portfolio tables are not claimed."),
        make("tharp_gallacher_10day", "Gallacher 10-day channel breakout (fundamental)", "TYW ch. 8-10 · Common systems", "Long/Short",
            Status::NotTestable("needs fundamental setup: Gallacher only takes 10-day breakouts in the direction the fundamentals predict (TYW p.229, 251, 269); a bare 10-day reversal would drop a stated rule"),
            &["F|Fundamentals must be set up for the market to move in a particular direction (TYW p.229)", "E|Enter on a 10-day channel breakout in that direction (TYW p.251)", "X|Exit on a 10-day channel breakout in the opposite direction (TYW p.269)"], "Not testable: fundamentals are judgement/data the book does not define."),
        make("tharp_foolish_four", "Motley Fool Foolish Four", "TYW ch. 7-10 · Common systems", "Long",
            Status::NeedsData("needs Dow 30 constituents with daily prices and dividend yields (stock universe)"),
            &["F|Ten highest-yielding Dow Jones Industrials; take the 2nd-5th lowest priced (TYW p.226, 267)", "E|Buy at the market on the first business day of the year (TYW p.226)", "X|Rebalance once a year; exit stocks that no longer qualify (TYW p.267-268)"], "Single-index data cannot run a Dow-stock selection."),
        make("tharp_oneil_canslim", "William O'Neil CANSLIM", "TYW ch. 7-10 · Common systems", "Long",
            Status::NotTestable("discretionary chart patterns (cup and handle, bases) plus fundamental CANSLIM setups on a stock universe"),
            &["E|Breakout from a 7-week to 15-month consolidation (cup and handle etc.) on volume 50% above average (TYW p.225)", "X|Never let a stock go against you by more than 7-8% (TYW p.249)", "X|Take a 20% profit, tempered by 36 other selling rules (TYW p.266)"], "Pattern recognition is judgement; rules incomplete in this book."),
        make("tharp_kaufman_ama", "Kaufman adaptive moving average", "TYW ch. 8-10 · Common systems", "Long/Short",
            Status::NotTestable("efficiency-ratio period, filter percentage and exit thresholds are not fixed by the book (\"some predetermined level\", \"perhaps\")"),
            &["E|Buy when the AMA turns up by more than a filter = % x std dev of 1-day AMA change over 20 days (10% futures, 100% equities) (TYW p.227-228)", "X|Exit on an opposite AMA change or when efficiency hits a very high value such as 0.8 (TYW p.268)"], "Entry/exit parameters left open; not implemented."),
        make("tharp_roberts_123", "Ken Roberts 1-2-3 reversal", "TYW ch. 8-10 · Common systems", "Long/Short",
            Status::NotTestable("discretionary: 1-2-3 pattern points and the consolidation trailing stop are chart judgement"),
            &["F|9-month high or low, then a 1-2-3 reversal (TYW p.229)", "E|Enter when price passes point 2 (TYW p.229, 252)", "X|Stop just beyond point 1; trail below new consolidations (TYW p.252, 269)"], "Not mechanical as described."),
        make("tharp_false_breakout", "False breakout entry (Workbook)", "Workbook 2-2 · Entry techniques", "Long/Short",
            Status::NotTestable("no profit exit (\"some trades will last 2 to 3 hours, others will last days\"), tick-based intraday entry; entry idea already in vault as ss_turtle_soup"),
            &["E|Today makes a 20-day low; the previous low was not within the last 5 days; buy stop 8 ticks above the previous 20-day low, good today only (Workbook p.75)", "X|If filled, GTC sell stop just under today's low (Workbook p.75)"], "Exit rule missing."),
        make("tharp_110day_breakout", "110-day breakout, weekly-volatility stop", "Super Trader ch. 13 · Simple system idea", "Long",
            Status::NotTestable("multiple of weekly volatility for the stop is not given (\"something as simple as\")"),
            &["E|Buy on a 110-day channel breakout (Super Trader p.213)", "X|Weekly volatility as worst-case exit, trailed from the close; 1% risk (Super Trader p.213)"], "Stop multiple unspecified (TYW p.258 suggests 0.7-2x weekly volatility as a range, not a rule)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "tharp_random_entry" => ("tharp_random", "Random entry", "3x volatility trailing stop, 1% risk"),
        "tharp_55_21_breakout" => ("tharp_breakout", "Channel breakout", "55-day entry / 21-day trailing stop"),
        "tharp_gallacher_10day" => ("tharp_breakout", "Channel breakout", "Gallacher 10-day (fundamental)"),
        "tharp_110day_breakout" => ("tharp_breakout", "Channel breakout", "110-day (stop multiple missing)"),
        "tharp_foolish_four" | "tharp_oneil_canslim" | "tharp_kaufman_ama" | "tharp_roberts_123" | "tharp_false_breakout" =>
            ("tharp_common", "Common systems reviewed", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "tharp_random_entry" => Some(vec![
            p("atr_n", "True-range EMA days (TYW default 10; other sources are separate)", 10., 5., 30., 1.),
            p("mult", "Stop = multiple of volatility", 3., 2., 4., 0.1),
            p("risk_pct", "Percent risk per trade (0 = full-equity unit)", 1., 0., 3., 0.25),
            p("seed", "Coin-flip seed [Not from the book]", 1., 1., 1000., 1.),
        ]),
        "tharp_55_21_breakout" => Some(vec![
            p("entry_n", "Entry breakout days", 55., 20., 100., 5.),
            p("exit_n", "Trailing stop days", 21., 5., 55., 1.),
            p("sizing", "Sizing: 0 full equity, 1 percent risk, 2 percent volatility", 0., 0., 2., 1.),
            p("pct", "Percent of equity for sizing 1/2", 1., 0.25, 5., 0.25),
        ]),
        _ => None,
    }
}

/// splitmix64; the coin is the top bit. Mirrored in library/tharp/verify/oracle.py.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn open_trade(dir: f64, i: usize, px: f64, w: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}

fn close_trade(mut t: Trade, i: usize, px: f64, reason: &str) -> Trade {
    t.exit_i = i;
    t.exit_px = px;
    t.signal_i = i;
    t.reason = reason.into();
    t
}

// TYW p.200 specifies exponential averaging, not Wilder's alpha=1/n.
// Source seeding is unspecified; this diagnostic seeds with the first n TRs.
fn true_range_ema(r: &[crate::Row], n: usize) -> Vec<f64> {
    let tr = true_range(r);
    let mut out = vec![f64::NAN; tr.len()];
    if n == 0 || tr.len() < n { return out; }
    let mut value = tr[..n].iter().sum::<f64>() / n as f64;
    if !value.is_finite() { return out; }
    out[n - 1] = value;
    let alpha = 2.0 / (n as f64 + 1.0);
    for i in n..tr.len() {
        value += alpha * (tr[i] - value);
        out[i] = value;
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let mut out = vec![];
    if id == "tharp_random_entry" {
        let (n, mult, risk) = (g("atr_n").round().max(1.) as usize, g("mult"), g("risk_pct") / 100.);
        let v = true_range_ema(x.r, n);
        let mut rng = g("seed").round().max(0.) as u64;
        let mut cur: Option<(Trade, f64)> = None;
        for i in from..=to {
            if let Some((t, stop)) = cur.as_mut() {
                let hit = if t.dir > 0. { x.l[i] <= *stop } else { x.h[i] >= *stop };
                if !hit {
                    let s = x.c[i] - t.dir * mult * v[i];
                    *stop = if t.dir > 0. { stop.max(s) } else { stop.min(s) };
                    continue;
                }
                let px = if t.dir > 0. { x.o[i].min(*stop) } else { x.o[i].max(*stop) };
                out.push(close_trade(cur.take().unwrap().0, i, px, "3x volatility trailing stop"));
            }
            if i < to && v[i].is_finite() && v[i] > 0. {
                let dir = if splitmix(&mut rng) >> 63 == 1 { 1. } else { -1. };
                let dist = mult * v[i];
                let w = if risk > 0. { risk * x.c[i] / dist } else { 1. };
                cur = Some((open_trade(dir, i, x.c[i], w, "close"), x.c[i] - dir * dist));
            }
        }
        if let Some((t, _)) = cur {
            let mut t = close_trade(t, to, x.c[to], "open");
            t.open = true;
            out.push(t);
        }
    } else {
        let (en, xn) = (g("entry_n").round().max(1.) as usize, g("exit_n").round().max(1.) as usize);
        let (sizing, pct) = (g("sizing").round() as i32, g("pct") / 100.);
        let tr = true_range(x.r);
        let hh = |i: usize, k: usize| x.h[i - k..i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let ll = |i: usize, k: usize| x.l[i - k..i].iter().copied().fold(f64::INFINITY, f64::min);
        let mut cur: Option<(Trade, f64)> = None;
        for i in from.max(en.max(xn).max(20))..=to {
            let mut exited = 0.;
            if let Some((t, stop)) = cur.as_mut() {
                if i > t.legs[0].i {
                    *stop = if t.dir > 0. { stop.max(ll(i, xn)) } else { stop.min(hh(i, xn)) };
                    let hit = if t.dir > 0. { x.l[i] <= *stop } else { x.h[i] >= *stop };
                    if hit {
                        let px = if t.dir > 0. { x.o[i].min(*stop) } else { x.o[i].max(*stop) };
                        exited = t.dir;
                        out.push(close_trade(cur.take().unwrap().0, i, px, &format!("{xn}-day trailing stop")));
                    }
                }
            }
            if cur.is_some() {
                continue;
            }
            let (up_hit, dn_hit) = (x.h[i] > hh(i, en), x.l[i] < ll(i, en));
            let (up, dn) = if exited > 0. { (false, dn_hit) } else if exited < 0. { (up_hit, false) } else { (up_hit && !dn_hit, dn_hit && !up_hit) };
            if !(up || dn) {
                continue;
            }
            let (dir, px, stop) = if up { (1., x.o[i].max(hh(i, en)), ll(i, xn)) } else { (-1., x.o[i].min(ll(i, en)), hh(i, xn)) };
            let w = match sizing {
                1 => pct * px / (dir * (px - stop)),
                2 => pct * px / (tr[i - 20..i].iter().sum::<f64>() / 20.),
                _ => 1.,
            };
            cur = Some((open_trade(dir, i, px, w, "stop"), stop));
        }
        if let Some((t, _)) = cur {
            let mut t = close_trade(t, to, x.c[to], "open");
            t.open = true;
            out.push(t);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(n: usize) -> Vec<Row> {
        (0..n).map(|i| Row { ticker: "T".into(), date: format!("{:05}", i), open: 100., high: 101., low: 99., close: 100., volume: None }).collect()
    }
    #[test]
    fn splitmix_matches_reference() {
        let mut s = 1u64;
        assert_eq!(splitmix(&mut s), 0x910A_2DEC_8902_5CC1); // splitmix64(seed=1) first output
    }
    #[test]
    fn tyw_ema10_responds_at_two_over_eleven_not_wilder_one_over_ten() {
        let mut r = rows(12);
        r[10].high = 109.; // true range jumps from 2 to 10 while the close stays 100.
        let ema = true_range_ema(&r, 10);
        let wilder = crate::connors::atr(&r, 10);
        assert!(ema[..9].iter().all(|v| v.is_nan()));
        assert_eq!(ema[9], 2.);
        assert!((ema[10] - 38. / 11.).abs() < 1e-12);
        assert!((wilder[10] - 2.8).abs() < 1e-12);
        assert!((ema[10] - wilder[10]).abs() > 0.6);
    }
    #[test]
    fn source_books_are_not_all_attributed_to_trade_your_way() {
        let defs = catalogue();
        assert_eq!(defs.iter().find(|d| d.id == "tharp_random_entry").unwrap().book, "tharp");
        assert_eq!(defs.iter().find(|d| d.id == "tharp_false_breakout").unwrap().book, "tharp_system_workbook");
        assert_eq!(defs.iter().find(|d| d.id == "tharp_110day_breakout").unwrap().book, "tharp_super_trader");
    }
    #[test]
    fn breakout_enters_on_stop_and_exits_on_21_day_low() {
        let mut r = rows(120);
        r[60].high = 105.; r[60].close = 104.; // new 55-day high: fill at prior high 101
        for row in &mut r[61..] { row.low = 99.5; }
        r[70].low = 90.; r[70].open = 98.; // breaks 21-day low 99 -> fill at 99
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("tharp_55_21_breakout", &x, 0, 119, &Params::new()).unwrap();
        assert_eq!(ts[0].legs[0].i, 60); assert_eq!(ts[0].legs[0].px, 101.);
        assert_eq!(ts[0].exit_i, 70); assert_eq!(ts[0].exit_px, 98.); // gapped below 99 -> open
    }
    #[test]
    fn random_entry_always_in_market_and_stop_only_tightens() {
        let mut r = rows(80);
        for (i, row) in r.iter_mut().enumerate() { let c = 100. + (i as f64 * 0.7).sin() * 5.; row.close = c; row.open = c; row.high = c + 1.; row.low = c - 1.; }
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("tharp_random_entry", &x, 20, 79, &Params::new()).unwrap();
        assert!(ts.last().unwrap().open);
        for w in ts.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
        for t in &ts { assert!(t.exit_px >= x.l[t.exit_i] - 1e-9 && t.exit_px <= x.h[t.exit_i] + 1e-9); }
    }
}

// Instrument admission: Tharp 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/tharp/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"tharp") && (crate::books::tharp::reviewed(def.id)),
    label: "Trade Your Way to Financial Freedom tested these systems on a basket of futures; index, ETF and stock runs (US and India) are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.",
    basis: "Van K. Tharp, Trade Your Way to Financial Freedom (1999): random entry with a trailing 3 x volatility stop and 1% risk (p.200-201) and the 55-day breakout with a 21-day trailing stop (p.286), reviewed in authors/tharp/admission/SOURCE-CONTRACT.md with independent chronological ledgers. The coin is a seeded generator (seed option) and fills follow the card's declared readings. No costs; the book's futures results are not reproduced.",
};
