//! Rotation / ranking rules across several actual ETFs (lane K28, 3 Nov 2026, decision #169 item 3).
//! The engine runs one ticker per request; these rules hold several ETFs at once, so they run on the SPY calendar
//! (SPY supplies the dates and the buy-and-hold benchmark) and read the other ETFs' own daily rows from the price file
//! the engine already loaded (`PRELOADED`, set once by main). Each position is valued every day at its own ETF's close,
//! through the existing multi-position account shape (`options::Account`, with its own model/basis text).
//! The rule definitions live in their books (stv7, fbrs, fgtofd in fontanills.rs, alpha); the rankings are here.
//! Independent check: authors/titles-1030/K28w/k28_ref.py (0 mismatches on every holding and trade).
use crate::connors::{Ctx, Leg, Params, Trade};
use crate::Row;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Every row of the engine's own price file (us_data.csv / idx_data.csv), set once at start-up.
pub static PRELOADED: OnceLock<&'static BTreeMap<String, Vec<Row>>> = OnceLock::new();

pub const IDS: &[&str] = &["alpha_dyn_treasuries", "alpha_rising_assets", "fbrs_combo_top3", "fgtofd_sector_tilt", "stv7_bull_someplace", "stv7_sell_in_may"];
pub fn is_rotation(id: &str) -> bool { IDS.contains(&id) }
pub const ANCHOR: &str = "SPY";
pub const SECTORS: &[&str] = &["XLB", "XLE", "XLF", "XLI", "XLK", "XLP", "XLU", "XLV", "XLY"];
pub const RISING: &[&str] = &["SPY", "IWM", "QQQ", "EFA", "EEM", "VNQ", "LQD", "GLD", "SHY", "IEF", "TLT", "AGG"];
pub const TREASURY: &[&str] = &["IEI", "IEF", "TLH", "TLT"];

pub const MODEL: &str = "rotation_multi_etf_v1";
pub const BASIS: &str = "Rotation account (lane K28). The rule ranks or weights several actual ETFs from the vault's US price file (price-only daily closes and opens, no dividends). SPY supplies only the trading calendar and the buy-and-hold benchmark; the chart's price line is SPY's, while each trade's prices are those of the ETF named in its reason. Positions are valued every day at their own ETF's close; no price is filled in or carried. Rules that keep a holding until it leaves the selection pass a sold holding's proceeds to its replacement; rules that set weights at every rebalance close each position and reopen it at the new weight, so a continuing holding is one trade per rebalance period. Costs: the vault fee and slippage setting is charged per side on each position (0 by default). No rotation is run before every ETF of the universe has the history the ranking needs.";

/// Closes and opens of the universe aligned to the anchor's dates (NaN where the ETF has no bar that day).
struct U { names: Vec<&'static str>, o: Vec<Vec<f64>>, c: Vec<Vec<f64>> }
fn universe(x: &Ctx, data: &BTreeMap<String, Vec<Row>>, names: &[&'static str]) -> Result<U, String> {
    let (mut o, mut c) = (vec![], vec![]);
    for n in names {
        let rows = data.get(*n).ok_or_else(|| format!("needs_data: the rotation universe needs {n}, which is not in the loaded price file"))?;
        let m: BTreeMap<&str, &Row> = rows.iter().map(|r| (r.date.as_str(), r)).collect();
        o.push(x.r.iter().map(|r| m.get(r.date.as_str()).map_or(f64::NAN, |v| v.open)).collect());
        c.push(x.r.iter().map(|r| m.get(r.date.as_str()).map_or(f64::NAN, |v| v.close)).collect());
    }
    Ok(U { names: names.to_vec(), o, c })
}
fn ok(v: f64) -> bool { v.is_finite() && v > 0. }
impl U {
    fn all(&self, ks: &[usize]) -> bool { self.c.iter().all(|c| ks.iter().all(|&k| ok(c[k]))) }
    /// asset indices sorted by score, best first; ties by ticker
    fn rank(&self, score: &[f64]) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.names.len()).collect();
        v.sort_by(|&a, &b| score[b].total_cmp(&score[a]).then(self.names[a].cmp(self.names[b])));
        v
    }
}

/// A rebalance at bar k (at the open or the close): hold these assets (weights used only in reweight mode).
struct Ev { k: usize, open: bool, hold: Vec<(usize, f64)>, why: String }

fn month_end(x: &Ctx, k: usize) -> bool { k + 1 < x.r.len() && x.r[k].date[..7] != x.r[k + 1].date[..7] }
fn week(x: &Ctx, k: usize) -> i64 { (crate::day_number(&x.r[k].date) + 3).div_euclid(7) } // Monday-based weeks
fn week_end(x: &Ctx, k: usize) -> bool { k + 1 < x.r.len() && week(x, k) != week(x, k + 1) }
fn ret(c: &[f64], k: usize, n: usize) -> f64 { c[k] / c[k - n] - 1. }
fn pct(v: f64) -> String { format!("{:+.2}%", v * 100.) }

/// The rule's rebalances for bars from..=to, and whether it keeps holdings until they leave (true) or reweights (false).
fn events(id: &str, x: &Ctx, u: &U, from: usize, to: usize, p: &Params) -> (Vec<Ev>, bool) {
    let n = x.r.len();
    let mut ev = vec![];
    let me: Vec<usize> = (0..n).filter(|&k| month_end(x, k)).collect();
    let name = |a: usize| u.names[a];
    match id {
        "stv7_sell_in_may" => {
            // u = [SPY, XLP or XLV]: buy the sector at April's last close, SPY at October's last close
            for &k in &me {
                let m = &x.r[k].date[5..7];
                if (m == "04" || m == "10") && u.all(&[k]) {
                    let a = if m == "04" { 1 } else { 0 };
                    ev.push(Ev { k, open: false, hold: vec![(a, 1.)], why: format!("{} held {}", name(a), if a == 1 { "May-October (bought at April's last close)" } else { "November-April (bought at October's last close)" }) });
                }
            }
            (ev, true)
        }
        "stv7_bull_someplace" => {
            for j in 12..me.len() {
                let (k, b) = (me[j], me[j - 12]);
                if k + 1 >= n || !u.all(&[k, b]) || !u.o.iter().all(|o| ok(o[k + 1])) { continue; }
                let s: Vec<f64> = u.c.iter().map(|c| c[k] / c[b] - 1.).collect();
                let top = u.rank(&s);
                ev.push(Ev { k: k + 1, open: true, hold: top[..3].iter().map(|&a| (a, 1. / 3.)).collect(),
                    why: format!("top 3 by 12-month price change to {}: {}", x.r[k].date, top[..3].iter().map(|&a| format!("{} {}", name(a), pct(s[a]))).collect::<Vec<_>>().join(", ")) });
            }
            (ev, true)
        }
        "fbrs_combo_top3" => {
            let top_n = p.get("top").copied().unwrap_or(3.).round().clamp(1., 9.) as usize;
            let months = p.get("months").copied().unwrap_or(0.).round().clamp(0., 12.) as usize;
            let ms: Vec<usize> = if months == 0 { vec![1, 3, 6, 9, 12] } else { vec![months] };
            for j in 12..me.len() {
                let k = me[j];
                let ks: Vec<usize> = std::iter::once(k).chain(ms.iter().map(|m| me[j - m])).collect();
                if !u.all(&ks) { continue; }
                let s: Vec<f64> = u.c.iter().map(|c| ms.iter().map(|m| c[k] / c[me[j - m]] - 1.).sum::<f64>() / ms.len() as f64).collect();
                let top = u.rank(&s);
                ev.push(Ev { k, open: false, hold: top[..top_n].iter().map(|&a| (a, 1. / top_n as f64)).collect(),
                    why: format!("top {top_n} by {} at {}: {}", if months == 0 { "the 1/3/6/9/12-month average return".to_string() } else { format!("{months}-month return") }, x.r[k].date, top[..top_n].iter().map(|&a| format!("{} {}", name(a), pct(s[a]))).collect::<Vec<_>>().join(", ")) });
            }
            (ev, false)
        }
        "fgtofd_sector_tilt" => {
            // SPY 50/200-day SMAs on the anchor's own closes
            let sma = |len: usize, k: usize| if k + 1 >= len { x.c[k + 1 - len..=k].iter().sum::<f64>() / len as f64 } else { f64::NAN };
            let (mut held, mut weak) = (None::<usize>, 0usize);
            for k in from.max(199)..=to {
                let (f, s) = (sma(50, k), sma(200, k));
                let ranks = (k >= 63 && u.all(&[k, k - 63])).then(|| { let sc: Vec<f64> = u.c.iter().map(|c| ret(c, k, 63)).collect(); (u.rank(&sc), sc) });
                if held.is_some() && f < s {
                    ev.push(Ev { k, open: false, hold: vec![], why: "SPY 50-day SMA below the 200-day SMA: bearish, sector position exited".into() });
                    held = None;
                } else if held.is_none() && f > s {
                    if let Some((r, sc)) = &ranks {
                        ev.push(Ev { k, open: false, hold: vec![(r[0], 1.)], why: format!("bullish (SPY 50-day SMA above the 200-day): bought the top 3-month ETF {} {}", name(r[0]), pct(sc[r[0]])) });
                        held = Some(r[0]); weak = 0;
                    }
                } else if let (Some(h), true, Some((r, sc))) = (held, week_end(x, k), &ranks) {
                    let place = r.iter().position(|&a| a == h).unwrap() + 1;
                    weak = if place > 3 { weak + 1 } else { 0 };
                    if weak >= 2 {
                        ev.push(Ev { k, open: false, hold: vec![(r[0], 1.)], why: format!("{} ranked {place} of 9 for a second week: switched to the top 3-month ETF {} {}", name(h), name(r[0]), pct(sc[r[0]])) });
                        held = Some(r[0]); weak = 0;
                    }
                }
            }
            (ev, true)
        }
        "alpha_rising_assets" => {
            for &k in &me {
                if k < 252 || !u.all(&(k - 252..=k).collect::<Vec<_>>()) { continue; }
                let s: Vec<f64> = u.c.iter().map(|c| [21, 63, 126, 252].iter().map(|&m| ret(c, k, m)).sum::<f64>() / 4.).collect();
                let vol: Vec<f64> = u.c.iter().map(|c| {
                    let r: Vec<f64> = (k - 62..=k).map(|t| c[t] / c[t - 1] - 1.).collect();
                    let mu = r.iter().sum::<f64>() / 63.;
                    (r.iter().map(|v| (v - mu).powi(2)).sum::<f64>() / 62.).sqrt()
                }).collect();
                let top = u.rank(&s);
                let inv: f64 = top[..5].iter().map(|&a| 1. / vol[a]).sum();
                ev.push(Ev { k, open: false, hold: top[..5].iter().map(|&a| (a, 1. / vol[a] / inv)).collect(),
                    why: format!("top 5 momentum at {}, weights by inverse 63-day volatility: {}", x.r[k].date, top[..5].iter().map(|&a| format!("{} {:.1}%", name(a), 100. / vol[a] / inv)).collect::<Vec<_>>().join(", ")) });
            }
            (ev, false)
        }
        "alpha_dyn_treasuries" => {
            // u = [IEI, IEF, TLH, TLT]
            for k in (105..n).filter(|&k| week_end(x, k)) {
                if !u.all(&[k, k - 21, k - 42, k - 63, k - 84, k - 105]) { continue; }
                let w: Vec<f64> = (1..4).map(|a| 0.05 * [21, 42, 63, 84, 105].iter().filter(|&&m| ret(&u.c[a], k, m) > 0.).count() as f64).collect();
                let mut hold = vec![(0, 1. - w.iter().sum::<f64>())];
                hold.extend(w.iter().enumerate().map(|(i, &v)| (i + 1, v)));
                hold.retain(|h| h.1 > 1e-12);
                ev.push(Ev { k, open: false, why: format!("week-end weights: {}", hold.iter().map(|&(a, v)| format!("{} {:.0}%", name(a), v * 100.)).collect::<Vec<_>>().join(", ")), hold });
            }
            (ev, false)
        }
        _ => (ev, true),
    }
}

struct Pos { a: usize, i: usize, px: f64, alloc: f64, open_fill: bool, why: String }

/// Daily account from the rebalances: (trades, allocation at entry, equity per bar from..=to).
fn simulate(x: &Ctx, u: &U, ev: &[Ev], keep: bool, from: usize, to: usize, cost: f64) -> Result<(Vec<Trade>, Vec<f64>, Vec<f64>), String> {
    let mut ev = ev.iter().filter(|e| e.k >= from && e.k <= to).peekable();
    let (mut cash, mut pos, mut trades, mut alloc, mut eq) = (1.0f64, Vec::<Pos>::new(), vec![], vec![], vec![]);
    let gap = |a: usize, k: usize| format!("needs_data: {} has no price on {} while held; no price is carried or filled in", u.names[a], x.r[k].date);
    // signal_i: the bar whose close decided the exit (the bar before a next-open fill)
    let close = |p: Pos, k: usize, px: f64, at_open: bool, open: bool, why: &str, trades: &mut Vec<Trade>, alloc: &mut Vec<f64>| {
        trades.push(Trade { dir: 1., legs: vec![Leg { i: p.i, px: p.px, w: 1. }], exit_i: k, exit_px: px, reason: format!("{}: {}; {}", u.names[p.a], p.why, why),
            entry_kind: if p.open_fill { "open" } else { "close" }, signal_i: if at_open { k.saturating_sub(1).max(p.i) } else { k }, open, exits: vec![] });
        alloc.push(p.alloc);
        p.alloc * (px / p.px - 2. * cost)
    };
    for k in from..=to {
        if let Some(e) = ev.next_if(|e| e.k == k) {
            let px = |a: usize| if e.open { u.o[a][k] } else { u.c[a][k] };
            let wanted: Vec<usize> = e.hold.iter().map(|h| h.0).collect();
            let (go, stay): (Vec<Pos>, Vec<Pos>) = pos.into_iter().partition(|p| !keep || !wanted.contains(&p.a));
            pos = stay;
            for p in go {
                let a = p.a;
                if !ok(px(a)) { return Err(gap(a, k)); }
                cash += close(p, k, px(a), e.open, false, if keep { "sold: left the selection" } else { "closed at the rebalance" }, &mut trades, &mut alloc);
            }
            let new: Vec<(usize, f64)> = e.hold.iter().copied().filter(|h| !pos.iter().any(|p| p.a == h.0)).collect();
            let (pool, count) = (cash, new.len() as f64);
            for (a, w) in new {
                if !ok(px(a)) { return Err(gap(a, k)); }
                // keep mode: the free cash (a sold holding's proceeds) is split equally among the newcomers
                let amount = if keep { pool / count } else { pool * w };
                cash -= amount;
                pos.push(Pos { a, i: k, px: px(a), alloc: amount, open_fill: e.open, why: e.why.clone() });
            }
            if cash.abs() < 1e-12 { cash = 0.; }
        }
        let mut v = cash;
        for p in &pos {
            if !ok(u.c[p.a][k]) { return Err(gap(p.a, k)); }
            v += p.alloc * (u.c[p.a][k] / p.px - cost);
        }
        eq.push(v);
    }
    for p in pos { let px = u.c[p.a][to]; close(p, to, px, false, true, "still held at the last bar (marked at its close)", &mut trades, &mut alloc); }
    let mut order: Vec<usize> = (0..trades.len()).collect();
    order.sort_by_key(|&j| (trades[j].legs[0].i, trades[j].exit_i, trades[j].reason.clone()));
    Ok((order.iter().map(|&j| trades[j].clone()).collect(), order.iter().map(|&j| alloc[j]).collect(), eq))
}

/// The rotation account (None = not a rotation rule; Err = refused or a data gap).
pub fn account(id: &str, x: &Ctx, from: usize, to: usize, cost: f64, p: &Params, data: Option<&BTreeMap<String, Vec<Row>>>) -> Option<Result<crate::options::Account, String>> {
    if !is_rotation(id) { return None; }
    Some((|| {
        if x.r.first().map(|r| r.ticker.as_str()) != Some(ANCHOR) {
            return Err(format!("not_testable: this rotation holds several ETFs and runs on the {ANCHOR} calendar only (SPY is the calendar and the benchmark; the holdings are the ETFs named in each trade)"));
        }
        let data = data.ok_or("needs_data: the loaded price file is not available to the rotation")?;
        let names: Vec<&'static str> = match id {
            "stv7_sell_in_may" => vec!["SPY", if p.get("sector").copied().unwrap_or(0.) >= 0.5 { "XLV" } else { "XLP" }],
            "alpha_rising_assets" => RISING.to_vec(),
            "alpha_dyn_treasuries" => TREASURY.to_vec(),
            _ => SECTORS.to_vec(),
        };
        let u = universe(x, data, &names)?;
        let (ev, keep) = events(id, x, &u, from, to, p);
        let (trades, alloc, equity) = simulate(x, &u, &ev, keep, from, to, cost)?;
        let (_, _, gross) = simulate(x, &u, &ev, keep, from, to, 0.)?;
        Ok(crate::options::Account { trades, alloc, equity, gross, ruin: None, model: MODEL, basis: BASIS })
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rows(t: &str, closes: &[f64]) -> Vec<Row> {
        // 28 bars a month from January 2001
        closes.iter().enumerate().map(|(d, &c)| { let m = d / 28; Row { ticker: t.into(), date: format!("{}-{:02}-{:02}", 2001 + m / 12, m % 12 + 1, d % 28 + 1), open: c, high: c, low: c, close: c, volume: None } }).collect()
    }
    #[test]
    fn sell_in_may_switches_at_april_and_october_closes_and_values_each_etf_at_its_own_close() {
        // 28 bars a month, Jan-Dec 2001: SPY flat at 100, XLP rises 1 a bar
        let spy = rows("SPY", &vec![100.; 336]);
        let xlp = rows("XLP", &(0..336).map(|i| 50. + i as f64).collect::<Vec<_>>());
        let data = BTreeMap::from([("SPY".to_string(), spy.clone()), ("XLP".to_string(), xlp)]);
        let x = Ctx::new(&spy, &spy, &[]);
        let a = account("stv7_sell_in_may", &x, 0, 335, 0., &Params::new(), Some(&data)).unwrap().unwrap();
        // April's last bar = index 111 (XLP 161), October's = 279 (XLP 329): one XLP trade, then SPY held to the end
        let t: Vec<_> = a.trades.iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.open)).collect();
        assert_eq!(t, vec![(111, 161., 279, 329., false), (279, 100., 335, 100., true)]);
        assert!((a.equity[279] - 329. / 161.).abs() < 1e-12 && (a.equity[200] - 250. / 161.).abs() < 1e-12 && a.equity[50] == 1.);
        assert!(account("stv7_sell_in_may", &Ctx::new(&data["XLP"], &spy, &[]), 0, 335, 0., &Params::new(), Some(&data)).unwrap().is_err());
        assert!(account("f1a", &x, 0, 335, 0., &Params::new(), Some(&data)).is_none());
    }
    #[test]
    fn faber_reweights_every_month_end_and_bull_someplace_keeps_its_three_holders() {
        // 26 months; sector i grows by (i+1)/1000 a bar, so the top 3 are always XLY, XLV, XLU
        let n = 26 * 28;
        let spy = rows("SPY", &vec![100.; n]);
        let mut data = BTreeMap::from([("SPY".to_string(), spy.clone())]);
        for (i, s) in SECTORS.iter().enumerate() { data.insert(s.to_string(), rows(s, &(0..n).map(|k| 10. * (1. + (i + 1) as f64 / 1000.).powi(k as i32)).collect::<Vec<_>>())); }
        let x = Ctx::new(&spy, &spy, &[]);
        // Stovall: first ranking at the 13th month end (bar 363), bought at the next open (bar 364), never sold
        let a = account("stv7_bull_someplace", &x, 0, n - 1, 0., &Params::new(), Some(&data)).unwrap().unwrap();
        let mut held: Vec<_> = a.trades.iter().map(|t| (t.legs[0].i, t.open, t.reason.split(':').next().unwrap().to_string())).collect();
        held.sort();
        assert_eq!(held, vec![(364, true, "XLU".into()), (364, true, "XLV".into()), (364, true, "XLY".into())]);
        assert!(a.alloc.iter().all(|v| (v - 1. / 3.).abs() < 1e-12) && a.equity[363] == 1.);
        // Faber: reweighted at every month end from bar 363 (close fills): 3 trades a month, last 3 still open
        let b = account("fbrs_combo_top3", &x, 0, n - 1, 0., &Params::new(), Some(&data)).unwrap().unwrap();
        let month_ends = (12..25).count(); // month ends 13..25 (the final bar is not a known month end)
        assert_eq!((b.trades.len(), b.trades.iter().filter(|t| t.open).count()), (3 * month_ends, 3));
        assert_eq!(b.trades[0].legs[0].i, 363);
        let k = 363 + 28; // after one month the equity is the mean of the three growth factors
        let g = |i: i32| (1. + (i + 1) as f64 / 1000.).powi(28);
        assert!((b.equity[k] - (g(8) + g(7) + g(6)) / 3.).abs() < 1e-9);
    }
}
