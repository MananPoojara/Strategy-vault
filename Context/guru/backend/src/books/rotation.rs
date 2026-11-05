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

pub const IDS: &[&str] = &["alpha_dyn_treasuries", "alpha_rising_assets", "fbrs_combo_top3", "fgitc_rotation", "fgtofd_sector_tilt", "kfgei_alloc_combined",
    "kfgei_alloc_corr", "kfgei_alloc_rar", "kfgei_alloc_vol", "stv7_bull_someplace", "stv7_sell_in_may"]; // + lane K54 (decision #229 (5), (10))
pub fn is_rotation(id: &str) -> bool { IDS.contains(&id) }
pub const ANCHOR: &str = "SPY";
pub const SECTORS: &[&str] = &["XLB", "XLE", "XLF", "XLI", "XLK", "XLP", "XLU", "XLV", "XLY"];
pub const RISING: &[&str] = &["SPY", "IWM", "QQQ", "EFA", "EEM", "VNQ", "LQD", "GLD", "SHY", "IEF", "TLT", "AGG"];
pub const TREASURY: &[&str] = &["IEI", "IEF", "TLH", "TLT"];
/// Fontanills, The Index Trading Course p.251 Table 11.8 "ETFs by Country": all 20, every one in the US price file (lane K54).
pub const COUNTRIES: &[&str] = &["EWA", "EWC", "EWD", "EWG", "EWH", "EWI", "EWJ", "EWK", "EWL", "EWM", "EWN", "EWO", "EWP", "EWQ", "EWS", "EWT", "EWU", "EWW", "EWY", "EWZ"];
/// Vivanti & Kaufman ch.7: the United States, Japan and Germany in US dollars; their US-listed funds SPY, EWJ, EWG (labelled, #229 (5)).
pub const GEI: &[&str] = &["SPY", "EWJ", "EWG"];

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
fn year_end(x: &Ctx, k: usize) -> bool { k + 1 < x.r.len() && x.r[k].date[..4] != x.r[k + 1].date[..4] }
fn sd(v: &[f64]) -> f64 { let m = v.iter().sum::<f64>() / v.len() as f64; (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64).sqrt() }
fn corr(a: &[f64], b: &[f64]) -> f64 {
    let (ma, mb) = (a.iter().sum::<f64>() / a.len() as f64, b.iter().sum::<f64>() / b.len() as f64);
    let cov: f64 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    cov / (a.iter().map(|x| (x - ma).powi(2)).sum::<f64>() * b.iter().map(|y| (y - mb).powi(2)).sum::<f64>()).sqrt()
}
/// First index of the smallest (largest when `max`) finite value; NaN never wins; ties keep the earlier market (US, Japan, Germany).
fn pick(v: &[f64], max: bool) -> Option<usize> {
    (0..v.len()).filter(|&a| v[a].is_finite()).fold(None, |b: Option<usize>, a| match b { Some(b) if (if max { v[a] <= v[b] } else { v[a] >= v[b] }) => Some(b), _ => Some(a) })
}
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
        "fgitc_rotation" => {
            // Table 11.8 country ETFs (lane K54): after the month's last close rank the 20 by their 240-trading-day gain; equal
            // fifths in the five with the largest gain, a fifth in cash for each of those five without a positive gain (p.251-252)
            let next = p.get("fill").copied().unwrap_or(0.) >= 0.5;
            for &k in &me {
                if k < 240 || !u.all(&[k, k - 240]) || (next && !u.o.iter().all(|o| ok(o[k + 1]))) { continue; }
                let s: Vec<f64> = u.c.iter().map(|c| ret(c, k, 240)).collect();
                let top: Vec<usize> = u.rank(&s).into_iter().take(5).collect();
                let hold: Vec<(usize, f64)> = top.iter().filter(|&&a| s[a] > 0.).map(|&a| (a, 0.2)).collect();
                let cash = 5 - hold.len();
                ev.push(Ev { k: if next { k + 1 } else { k }, open: next, hold, why: format!("top 5 by 240-day gain to {}: {}{}", x.r[k].date,
                    top.iter().map(|&a| format!("{} {}", name(a), pct(s[a]))).collect::<Vec<_>>().join(", "), if cash > 0 { format!("; {cash} fifth(s) in cash (no positive gain)") } else { String::new() }) });
            }
            (ev, false)
        }
        s if s.starts_with("kfgei_alloc_") => {
            // Vivanti & Kaufman ch.7 (lane K54): at each year's last close, from the last three calendar-year returns (price only, USD
            // funds), set next year's weights; u = [SPY, EWJ, EWG]
            let monthly = p.get("corr").copied().unwrap_or(0.) >= 0.5;
            let ye: Vec<usize> = (0..n).filter(|&k| year_end(x, k)).collect();
            for j in 3..ye.len() {
                let (k, ks) = (ye[j], &ye[j - 3..=j]);
                if !u.all(ks) { continue; }
                let r: Vec<Vec<f64>> = u.c.iter().map(|c| (1..4).map(|q| c[ks[q]] / c[ks[q - 1]] - 1.).collect()).collect();
                let w_one = |best: Option<usize>| -> Option<Vec<f64>> { let b = best?; Some((0..3).map(|a| if a == b { 2. / 3. } else { 1. / 6. }).collect()) };
                let vol = w_one(pick(&r.iter().map(|v| sd(v)).collect::<Vec<_>>(), false));
                let rar = w_one(pick(&r.iter().map(|v| (v.iter().map(|x| 1. + x).product::<f64>().powf(1. / 3.) - 1.) / sd(v)).collect::<Vec<_>>(), true));
                let cr = if monthly {
                    let me36: Vec<usize> = me.iter().copied().filter(|&m| m <= k).collect();
                    (me36.len() >= 37 && u.all(&me36[me36.len() - 37..])).then(|| {
                        let m = &me36[me36.len() - 37..];
                        let mr: Vec<Vec<f64>> = u.c.iter().map(|c| (1..37).map(|q| c[m[q]] / c[m[q - 1]] - 1.).collect()).collect();
                        [f64::NAN, corr(&mr[0], &mr[1]), corr(&mr[0], &mr[2])]
                    })
                } else { Some([f64::NAN, corr(&r[0], &r[1]), corr(&r[0], &r[2])]) };
                let corr_w = cr.and_then(|cv| pick(&cv, false)).map(|b| (0..3).map(|a| if a == 0 { 0.7 } else if a == b { 0.3 } else { 0. }).collect::<Vec<f64>>());
                let w = match id {
                    "kfgei_alloc_vol" => vol,
                    "kfgei_alloc_rar" => rar,
                    "kfgei_alloc_corr" => corr_w,
                    _ => match (vol, rar, corr_w) { (Some(a), Some(b), Some(c)) => Some((0..3).map(|q| (a[q] + b[q] + c[q]) / 3.).collect()), _ => None },
                };
                let Some(w) = w else { continue };
                let yr = |q: usize| &x.r[ks[q]].date[..4];
                ev.push(Ev { k, open: false, hold: (0..3).filter(|&a| w[a] > 0.).map(|a| (a, w[a])).collect(),
                    why: format!("weights for {} from {}-{} yearly returns: {}", yr(3).parse::<i32>().map_or(String::new(), |y| (y + 1).to_string()), yr(1), yr(3),
                        (0..3).map(|a| format!("{} {:.1}%", name(a), 100. * w[a])).collect::<Vec<_>>().join(", ")) });
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
            "fgitc_rotation" => COUNTRIES.to_vec(),
            s if s.starts_with("kfgei_alloc_") => GEI.to_vec(),
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
    fn country_rotation_holds_fifths_of_the_top_five_and_cash_for_those_without_a_gain() {
        // lane K54: 12 months of 28 bars; country i grows (i - 16)/1000 a bar, so only EWW, EWY, EWZ (i = 17..19) gained over 240 days
        let n = 12 * 28;
        let spy = rows("SPY", &vec![100.; n]);
        let mut data = BTreeMap::from([("SPY".to_string(), spy.clone())]);
        for (i, s) in COUNTRIES.iter().enumerate() { data.insert(s.to_string(), rows(s, &(0..n).map(|k| 10. * (1. + (i as f64 - 16.) / 1000.).powi(k as i32)).collect::<Vec<_>>())); }
        let x = Ctx::new(&spy, &spy, &[]);
        let a = account("fgitc_rotation", &x, 0, n - 1, 0., &Params::new(), Some(&data)).unwrap().unwrap();
        // first month end with 240 days behind it: bar 9 x 28 - 1 = 251; three fifths invested at that close, two fifths cash
        let first: Vec<_> = a.trades.iter().filter(|t| t.legs[0].i == 251).map(|t| t.reason.split(':').next().unwrap().to_string()).collect();
        assert_eq!(first, vec!["EWW", "EWY", "EWZ"]);
        assert!(a.alloc[..3].iter().all(|v| (v - 0.2).abs() < 1e-12));
        let g = |i: f64| (1. + (i - 16.) / 1000.).powi(28);
        assert!((a.equity[251 + 28] - (0.4 + 0.2 * (g(17.) + g(18.) + g(19.)))).abs() < 1e-9);
    }
    #[test]
    fn gei_allocations_follow_the_three_yearly_returns() {
        // lane K54: prices constant inside each year; yearly returns SPY 10/10/5%, EWJ +30/-20/+30%, EWG -20/+30/-20%
        let year = 12 * 28; let n = 5 * year;
        let px = |p: [f64; 5]| (0..n).map(|k| p[k / year]).collect::<Vec<f64>>();
        let spy = rows("SPY", &px([100., 110., 121., 127.05, 127.05]));
        let data = BTreeMap::from([("SPY".to_string(), spy.clone()), ("EWJ".to_string(), rows("EWJ", &px([100., 130., 104., 135.2, 135.2]))),
            ("EWG".to_string(), rows("EWG", &px([100., 80., 104., 83.2, 83.2])))]);
        let x = Ctx::new(&spy, &spy, &[]);
        let w = |id: &str| { let a = account(id, &x, 0, n - 1, 0., &Params::new(), Some(&data)).unwrap().unwrap();
            let k = 4 * year - 1; // the 4th year-end: the first with three yearly returns behind it
            let mut v: Vec<(String, f64)> = a.trades.iter().zip(&a.alloc).filter(|(t, _)| t.legs[0].i == k).map(|(t, &al)| (t.reason.split(':').next().unwrap().to_string(), al)).collect();
            v.sort_by(|p, q| p.0.cmp(&q.0)); v };
        let near = |v: Vec<(String, f64)>, want: &[(&str, f64)]| v.len() == want.len() && v.iter().zip(want).all(|(a, b)| a.0 == b.0 && (a.1 - b.1).abs() < 1e-9);
        assert!(near(w("kfgei_alloc_vol"), &[("EWG", 1. / 6.), ("EWJ", 1. / 6.), ("SPY", 2. / 3.)])); // SPY: the lowest SD
        assert!(near(w("kfgei_alloc_rar"), &[("EWG", 1. / 6.), ("EWJ", 1. / 6.), ("SPY", 2. / 3.)])); // SPY: the best return / SD
        assert!(near(w("kfgei_alloc_corr"), &[("EWJ", 0.3), ("SPY", 0.7)])); // EWJ moves against SPY, EWG with it
        assert!(near(w("kfgei_alloc_combined"), &[("EWG", 1. / 9.), ("EWJ", 0.6333333333333333 / 3.), ("SPY", (4. / 3. + 0.7) / 3.)]));
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
