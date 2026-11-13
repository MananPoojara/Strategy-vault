//! Benjamin Van Vliet - programming and process books. Book ids `vvbats` (Building Automated Trading Systems), `vvmfm` (Modeling
//! Financial Markets, with Hendry), `vvqmm` (Quality Money Management, with Kumiega). Reading: claude-continuation-20261020/authors/vanvliet/agents/.
//! The author says of his examples that 'in the real world this trading system will not make money' (BATS p.267, p.291).
//! Lane P1 (13 Nov 2026, R4 #293, #297; authors/titles-1030/P1.tsv): the two two-leg examples run on the spread engine (crate::spread):
//! vvbats_spread (ES/NQ ratio spread, BATS ch.46) on the Norgate E-mini continuous contracts, vvqmm_pairs (QMM §5.4) on pairs of actual
//! stocks chosen by the vault. Independent references: authors/titles-1030/P1-1113w/p1_ref.py (written from the pages, not from this file).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::spread::{self, Leg};

/// The rules that run on the two-leg spread account.
pub const SPREAD_IDS: &[&str] = &["vvbats_spread", "vvqmm_pairs"];
pub fn is_spread(id: &str) -> bool { SPREAD_IDS.contains(&id) }

/// vvqmm_pairs: the book names no pair ('pairs of stocks that are in the same sector and have similar fundamentals', p.63), so the
/// vault declares two same-sector pairs per market (lead brief P1): (calendar, [(stock A, stock B); 2]). Spread = price B / price A.
pub const PAIRS: &[(&str, &str, [(&str, &str); 2])] = &[("SPY", "us_sp500", [("KO", "PEP"), ("XOM", "CVX")]), ("NIFTY 50", "nifty50", [("HDFCBANK", "ICICIBANK"), ("TCS", "INFY")])];
fn pair_key(sym: &str) -> String { format!("PAIR.{sym}.Close") }
/// The declared pairs' daily closes from the vault's stock basket files, attached to the calendar context (extra series: exact date,
/// NaN where the stock has no bar). Only stocks of the engine's own basket data load (US engine: us_sp500; India engine: nifty50).
pub fn pair_series(cat: &crate::baskets::Catalogue) -> Vec<((String, String), std::collections::BTreeMap<String, f64>)> {
    let mut out = Vec::new();
    for (anchor, basket, pairs) in PAIRS {
        for sym in pairs.iter().flat_map(|(a, b)| [*a, *b]) {
            let key = format!("basket:{basket}:{sym}");
            if !cat.contains(&key) { continue; }
            match cat.rows(&key) {
                Ok(rows) => out.push(((anchor.to_string(), pair_key(sym)), rows.into_iter().map(|r| (r.date, r.close)).collect())),
                Err(e) => eprintln!("warning: vvqmm_pairs stock {key} not loaded: {e}"),
            }
        }
    }
    out
}

const BATS_NOTE: &str = "Lane P1 (13 Nov 2026, R4 #293, #297). Book rule (BATS ch.46 p.290-291 and the printed Spreader_Example code p.297-301): spread = -1 x ES + 2 x NQ; z = (spread - MA30(spread)) / sd30(spread) over the last 30 spread prices (the code's NormCalc divides by 30: population standard deviation, the newest price included); z > 2: short the spread (long 1 ES, short 2 NQ), z < -2: long the spread (short 1 ES, long 2 NQ); exit when z crosses 0 (long: z > 0; short: z < 0) or the spread reaches the stop, 4 ticks of the ES contract (4 x 0.25 = 1.00 spread point) from the entry spread price; one spread position at a time. Data (labelled): Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. Both contracts back-adjusted, joined by exact date; daily closing prices (last trade) instead of the bid and ask quote changes the book listens to. Adapted: 2 unstated details. Unstated in the book: the fill on daily bars (the book works a passive ES limit order at the bid/offer and then hits the NQ market); vault reading: both legs at the signal day's close. Unstated in the book: when the stop and target are checked on daily bars (the book checks every quote change); vault reading: on each daily close, exit at that close (no intraday path of the spread exists). Money: ES US$50 and NQ US$20 per index point; trade return = the two legs' dollar P&L / (US$50 x ES + 2 x US$20 x NQ at the unadjusted entry closes). A new signal is taken from the day after an exit. The author says this example 'will not make money' (p.291); no performance is printed, so nothing is compared. India: held (NIFTY / BANKNIFTY futures need a ratio choice and a roll rule for per-contract minute files; questions in authors/verifier-questions.tsv).";
const QMM_NOTE: &str = "Lane P1 (13 Nov 2026, R4 #297). Book rule (QMM §5.4 p.63-64): the spread is the price ratio of two similar stocks; norm = (spread - MA30(spread)) / sd30(spread); norm > 2: short the spread (long stock A, short stock B), norm < -2: long the spread (short A, long B); 'all opening trades are executed at the closing price on the day the signal is received and closing trades executed at the closing price n days hence'. Holding period: the book prints 20 trading days (assumptions) and five trading days (strategy text); default 20, option 5 (R4 #297). The pair is not named: pair chosen by the vault (US: KO/PEP, option XOM/CVX; India: HDFCBANK/ICICIBANK, option TCS/INFY), spread = price B / price A, on the vault's split-adjusted daily stock files (price only, no dividends), joined with the index calendar by exact date (SPY / NIFTY 50 are the calendar and the benchmark, never a stand-in for the stocks). The standard deviation is over the 30 values (population), as the same author's z-transform code divides by 30 (BATS p.301). Adapted: 2 unstated details. Unstated in the book: the size of each leg; vault reading: equal money in each leg at the entry close; return on the gross exposure (long + short notional). On the book's own basis ('long positions are paid for in full and a margin of 50% is assessed on short trades') the capital is 1.5 legs, so a return on that capital = 4/3 x the vault's figure. Unstated in the book: signals while a position is open; vault reading: one position at a time, the next signal is taken from the day after the exit. No borrow fee is modelled. No performance is printed, so nothing is compared.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, book, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes, claim: None };
    let held = "Read 2026-10-24 (authors/vanvliet/agents).";
    vec![
        d("vvbats_tickma", "Tick moving-average crossover with target and stop (NQ e-mini)", "Building Automated Trading Systems · Ch.44", "vvbats",
          Status::NotTestable("needs tick-by-tick Nasdaq-100 e-mini trades; the target and stop tick counts are user inputs with no values; the author says it will not make money"),
          &["E|Short MA (say 5 ticks) crosses above the long MA (say 20 ticks): buy and bracket with a target and a stop a set number of ticks away (p.267)", "X|Cross down: close the long and go short (code p.272-274)"], held),
        d("vvbats_spread", "ES/NQ ratio-spread z-score arbitrage", "Building Automated Trading Systems · Ch.46", "vvbats", Status::Live,
          &["E|\"Pricespread = -1 x ES Bid + 2 x NQ Ask\"; \"(spread - MA30(spread)) / sd30(spread)\"; \"When norm exceeds 2 ... attempt to enter a short position\" (long 1 ES, short 2 NQ); \"when norm is less than -2 ... a long position in the spread\" (short 1 ES, long 2 NQ) (p.290-291)",
            "X|\"I have hard coded the stop price at 4 ticks. The target price will be a norm of 0, at which time the positions in each contract will be flattened.\" (p.291; code p.298, p.300: stop = entry spread +/- 4 x the ES tick)"],
          BATS_NOTE),
        d("vvmfm_optquote", "Option quoting around theoretical value (programming example)", "Modeling Financial Markets · order-entry example", "vvmfm",
          Status::NotTestable("not a trading rule, no entry or exit: a programming example that quotes bids and asks around a model value (Modeling Financial Markets, order-entry chapter), with no quote history to test it on"),
          &["E|Buy when the ask is 10 cents below the model value; sell when the bid is 10 cents above"], held),
        d("vvqmm_pairs", "Pairs statistical arbitrage (z-score of a two-stock spread)", "Quality Money Management · Ch.5 §5.4", "vvqmm", Status::Live,
          &["E|\"When norm exceeds 2, we consider this a bearish signal ... take a hypothetical short position in the spread\" (long A, short B); \"when norm is less than -2 ... simulates a long position in the spread\" (short A, long B); \"all opening trades are executed at the closing price on the day the signal is received\" (p.63-64)",
            "X|\"closing trades executed at the closing price n days hence\": \"Same holding horizons of 20 trading days\" (p.64 assumptions; the strategy text says five trading days, an option)"],
          QMM_NOTE),
        d("vvqmm_factor", "Buy-write filter and multifactor decile long/short", "Quality Money Management · Ch.5 §5.5-5.6", "vvqmm",
          Status::NotTestable("fundamental ranking portfolios (P/B, P/E, P/S, growth) with optimised weights not given, plus index-call overlay; needs fundamentals and option prices"),
          &["E|Top 50 S&P 500 stocks on P/B, size and momentum held a month, index calls sold; or buy the top and sell the bottom decile of a multifactor score (p.64-65)"], held),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "vvbats_spread" => Some(("vvbats_spread", "Van Vliet ES/NQ ratio-spread z-score", "-1 ES : 2 NQ")),
        "vvqmm_pairs" => Some(("vvqmm_pairs", "Kumiega & Van Vliet pairs z-score", "z 2 / 20 days")),
        _ => (id.starts_with("vvbats_") || id.starts_with("vvmfm_") || id.starts_with("vvqmm_")).then_some(("vv_nt", "Van Vliet example systems (held)", "-")),
    }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "vvbats_spread" => Some(vec![p("n", "Spread prices in the z window (book 30)", 30., 5., 200., 1.), p("z", "Entry |z| (book 2)", 2., 0.5, 5., 0.1),
            p("stop", "Stop, ES ticks from the entry spread (book 4)", 4., 1., 400., 1.)]),
        "vvqmm_pairs" => Some(vec![p("hold", "Holding period, trading days (book 20; also printed: 5)", 20., 1., 60., 1.),
            p("pair", "Pair (0: KO/PEP or HDFCBANK/ICICIBANK; 1: XOM/CVX or TCS/INFY)", 0., 0., 1., 1.),
            p("n", "Days in the z window (book 30)", 30., 5., 200., 1.), p("z", "Entry |z| (book 2)", 2., 0.5, 5., 0.1)]),
        _ => None,
    }
}
/// The spread rules return no single-instrument trades; they run on `account`.
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

fn num(p: &Params, k: &str, d: f64) -> f64 { p.get(k).copied().filter(|v| v.is_finite()).unwrap_or(d) }
fn last_joint(j: &[usize], to: usize) -> Option<usize> { j.partition_point(|&i| i <= to).checked_sub(1) }

/// Spread positions of a two-leg rule (None = not a spread rule; Err = not on this context or data missing).
pub fn positions(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Result<Vec<crate::options::Pos>, String>> {
    match id {
        "vvbats_spread" => Some(bats(x, from, to, p)),
        "vvqmm_pairs" => Some(pairs(x, from, to, p)),
        _ => None,
    }
}

fn bats(x: &Ctx, from: usize, to: usize, p: &Params) -> Result<Vec<crate::options::Pos>, String> {
    let m = crate::futures::ctx_market(x).filter(|m| m.sym == "&ES")
        .ok_or("not_testable: this spread runs only in the E-mini S&P 500 (Norgate &ES) context, with E-mini Nasdaq-100 (Norgate &NQ) as the second leg")?;
    let nqm = crate::futures::MARKETS.iter().find(|m| m.sym == "&NQ").ok_or("needs_data: the NQ contract is not defined")?;
    let miss = || "needs_data: the E-mini Nasdaq-100 (Norgate &NQ) leg or an unadjusted series is not loaded".to_string();
    let (es, es_raw) = (&x.c[..], x.series("RAW.Close").ok_or_else(miss)?);
    let (nq, nq_raw) = (x.series("&NQ.Close").ok_or_else(miss)?, x.series("&NQ.RAW.Close").ok_or_else(miss)?);
    let (n, th, stop_ticks) = (num(p, "n", 30.).round().max(2.) as usize, num(p, "z", 2.), num(p, "stop", 4.));
    let j = spread::joint(&[es, es_raw, nq, nq_raw]);
    let s: Vec<f64> = j.iter().map(|&i| -es[i] + 2. * nq[i]).collect();
    let mut out = Vec::new();
    let mut pos: Option<(usize, f64, f64, f64)> = None; // (joint index of entry, dir, entry spread, stop)
    let mk = |e: usize, xi: usize, dir: f64, why: String, open: bool| {
        let (ea, eb) = (crate::futures::usd_per_point(m, &x.r[e].date), crate::futures::usd_per_point(nqm, &x.r[e].date));
        let (Some(ea), Some(eb)) = (ea, eb) else { return Err(format!("needs_data: no contract multiplier on {}", x.r[e].date)) };
        let legs = [Leg { qty: -1., px: es, base: es_raw, mult: ea }, Leg { qty: 2., px: nq, base: nq_raw, mult: eb }];
        let side = if dir > 0. { "long the spread: short 1 ES, long 2 NQ" } else { "short the spread: long 1 ES, short 2 NQ" };
        let reason = format!("{side} at the {} close (ES {:.2}, NQ {:.2} unadjusted); {why}", x.r[e].date, es_raw[e], nq_raw[e]);
        spread::position(&legs, dir, e, xi, reason, open).ok_or_else(|| format!("calculation_unavailable: no positive gross exposure on {}", x.r[e].date))
    };
    for (k, &i) in j.iter().enumerate() {
        if i > to { break; }
        let z = spread::zscore(&s, k, n);
        if let Some((ke, dir, se, stop)) = pos {
            let zv = z.unwrap_or(0.);
            let hit_stop = if dir > 0. { s[k] <= stop } else { s[k] >= stop };
            let hit_target = if dir > 0. { zv > 0. } else { zv < 0. };
            if hit_stop || hit_target {
                let why = if hit_stop { format!("stop: spread {:.2} reached {stop:.2} ({stop_ticks} ES ticks from the entry spread {se:.2})", s[k]) }
                    else { format!("target: z = {zv:.2} crossed 0") };
                out.push(mk(j[ke], i, dir, format!("{why} at the {} close", x.r[i].date), false)?);
                pos = None;
            }
            continue;
        }
        if i < from { continue; }
        let Some(zv) = z else { continue };
        let dir = if zv > th { -1. } else if zv < -th { 1. } else { continue };
        pos = Some((k, dir, s[k], s[k] - dir * stop_ticks * m.tick));
    }
    if let Some((ke, dir, _, _)) = pos {
        let xi = last_joint(&j, to).unwrap_or(j[ke]);
        out.push(mk(j[ke], xi, dir, "still open at the last bar (marked at its close)".into(), true)?);
    }
    Ok(out)
}

fn pairs(x: &Ctx, from: usize, to: usize, p: &Params) -> Result<Vec<crate::options::Pos>, String> {
    let anchor = x.r.first().map(|r| r.ticker.as_str()).unwrap_or("");
    let (_, _, list) = PAIRS.iter().find(|t| t.0 == anchor)
        .ok_or("not_testable: this pairs rule trades two actual stocks and runs only on the SPY calendar (US pairs) or the NIFTY 50 calendar (India pairs); the index is the calendar and the benchmark, never a stand-in for the stocks")?;
    let (a, b) = list[(num(p, "pair", 0.).round().clamp(0., 1.)) as usize];
    let miss = |s: &str| format!("needs_data: the {s} stock prices are not loaded in this engine (pair {a}/{b}); no price is filled in");
    let (pa, pb) = (x.series(&pair_key(a)).ok_or_else(|| miss(a))?, x.series(&pair_key(b)).ok_or_else(|| miss(b))?);
    let (hold, n, th) = (num(p, "hold", 20.).round().max(1.) as usize, num(p, "n", 30.).round().max(2.) as usize, num(p, "z", 2.));
    let j = spread::joint(&[pa, pb, &x.c]);
    let s: Vec<f64> = j.iter().map(|&i| pb[i] / pa[i]).collect();
    let mut out = Vec::new();
    let mut next = 0;
    for (k, &i) in j.iter().enumerate() {
        if i > to { break; }
        if i < from || k < next { continue; }
        let Some(zv) = spread::zscore(&s, k, n) else { continue };
        let dir = if zv > th { -1. } else if zv < -th { 1. } else { continue };
        let (xi, open) = match j.get(k + hold) { Some(&xi) if xi <= to => (xi, false), _ => (last_joint(&j, to).unwrap_or(i), true) };
        // long the spread = short A, long B: equal money (100) in each leg at the entry close
        let legs = [Leg { qty: -100. / pa[i], px: pa, base: pa, mult: 1. }, Leg { qty: 100. / pb[i], px: pb, base: pb, mult: 1. }];
        let side = if dir > 0. { format!("long the spread: short {a} {:.2}, long {b} {:.2}", pa[i], pb[i]) } else { format!("short the spread: long {a} {:.2}, short {b} {:.2}", pa[i], pb[i]) };
        let why = if open { "still open at the last bar (marked at its close)".to_string() } else { format!("closed at the {} close, {hold} trading days later", x.r[xi].date) };
        let reason = format!("{side} at the {} close (z of {b}/{a} = {zv:.2}); {why}", x.r[i].date);
        out.push(spread::position(&legs, dir, i, xi, reason, open).ok_or_else(|| format!("calculation_unavailable: no positive exposure on {}", x.r[i].date))?);
        next = k + hold + 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(t: &str, c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &v)| Row { ticker: t.into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: v, high: v, low: v, close: v, volume: None }).collect()
    }
    #[test]
    fn pairs_enter_on_z_beyond_2_and_close_n_days_later() {
        // A flat at 10; B flat at 20 then a jump on day 35 (z >> 2: short the spread = long A, short B); B missing on day 40
        let n = 80;
        let a: Vec<f64> = (0..n).map(|i| 10. + 0.01 * (i % 3) as f64).collect();
        let mut b: Vec<f64> = (0..n).map(|i| 20. + 0.01 * (i % 2) as f64).collect();
        b[35] = 23.;
        b[40] = f64::NAN;
        let r = rows("SPY", &vec![100.; n]);
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert(pair_key("KO"), a.clone());
        x.extra.insert(pair_key("PEP"), b.clone());
        let pos = pairs(&x, 0, n - 1, &Params::new()).unwrap();
        let first = &pos[0];
        assert_eq!((first.entry_i, first.dir), (35, -1.));
        assert_eq!(first.exit_i, 35 + 20 + 1); // 20 joint days later: day 40 has no B bar, so it is not counted
        let want = -100. / a[35] * (a[56] - a[35]) * -1. + -1. * 100. / b[35] * (b[56] - b[35]);
        assert!((first.pnl.last().unwrap() - want).abs() < 1e-9 && first.s0 == 200.);
        assert!(pos.windows(2).all(|w| w[1].entry_i > w[0].exit_i));
        let five = pairs(&x, 0, n - 1, &[("hold".to_string(), 5.)].into_iter().collect()).unwrap();
        assert_eq!(five[0].exit_i, 41);
        let q = rows("QQQ", &vec![100.; n]);
        assert!(pairs(&Ctx::new(&q, &q, &[]), 0, n - 1, &Params::new()).err().unwrap().starts_with("not_testable"));
        let mut y = Ctx::new(&r, &r, &[]);
        y.extra.insert(pair_key("KO"), a);
        assert!(pairs(&y, 0, n - 1, &Params::new()).err().unwrap().starts_with("needs_data"));
    }
    #[test]
    fn bats_spread_stop_and_target_on_daily_closes() {
        let n = 60;
        let es: Vec<f64> = (0..n).map(|i| 1000. + (i % 2) as f64).collect();
        let mut nq: Vec<f64> = (0..n).map(|i| 1500. + 0.5 * (i % 3) as f64).collect();
        nq[40] = 1510.; // spread jumps up: z > 2, short the spread
        nq[41] = 1511.; // ES +1, NQ +1: spread +1.00 above the entry = the stop (4 x 0.25), hit at the close
        let r = rows("E-mini S&P 500 (Norgate &ES)", &es);
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert("RAW.Close".into(), es.iter().map(|v| v + 100.).collect());
        x.extra.insert("&NQ.Close".into(), nq.clone());
        x.extra.insert("&NQ.RAW.Close".into(), nq.iter().map(|v| v + 200.).collect());
        let pos = bats(&x, 0, n - 1, &Params::new()).unwrap();
        let t = &pos[0];
        assert_eq!((t.entry_i, t.exit_i, t.dir), (40, 41, -1.));
        assert!(t.reason.contains("stop"));
        // short the spread = long 1 ES (x50), short 2 NQ (x20): dollar P&L on the gross exposure at the unadjusted closes
        let want = 50. * (es[41] - es[40]) - 2. * 20. * (nq[41] - nq[40]);
        assert!((t.pnl[1] - want).abs() < 1e-9);
        assert!((t.s0 - (50. * (es[40] + 100.) + 40. * (nq[40] + 200.))).abs() < 1e-9);
        let q = rows("SPY", &es);
        assert!(bats(&Ctx::new(&q, &q, &[]), 0, n - 1, &Params::new()).err().unwrap().starts_with("not_testable"));
    }
    #[test]
    fn catalogue_live_ids_are_the_spread_ids() {
        let live: Vec<_> = catalogue().into_iter().filter(|d| d.status == Status::Live).map(|d| d.id).collect();
        assert_eq!(live, SPREAD_IDS);
        for id in SPREAD_IDS { assert!(params(id).is_some() && family_of(id).is_some_and(|f| f.0 == *id)); }
    }
}
