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
/// Vivanti & Kaufman ch.7: the United States, Japan and Germany in US dollars; their US-listed funds DIA, EWJ, EWG (labelled, #229 (5)).
/// The chapter's US market is the Dow (p.220), so the Dow fund DIA, not SPY (R4 #248 (4), lane K61); SPY stays the calendar.
pub const GEI: &[&str] = &["DIA", "EWJ", "EWG"];

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
            // funds), set next year's weights; u = [DIA, EWJ, EWG]
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

// ================================================================ EP5: basket ranking portfolio (lane K56, 5 Nov 2026; #20, #229)
// Ranks the members of a vault stock basket on a schedule (daily, weekly, monthly) and holds the chosen stocks on one account:
// top/bottom N or a fraction, long or long/short, overlapping equal-weight vintages (one cash sleeve per vintage, as Daragan's
// A/B/C parts and Jegadeesh-Titman's K portfolios), or slots sized at a fraction of equity. Every decision reads data up to its
// signal bar only; fills are at the member's own open or close of the fill bar. A member with no bar on a day is not ranked or
// traded that day; nothing is filled in. Rules: alpha, atm04, cjlmom, dgstt, gh52w, ggth (gregoriou), hmrw, jtwl modules.
// Independent check: authors/titles-1030/K56w/k56_ref.py.
pub const BASKET_IDS: &[&str] = &["alpha_weekly_mr", "atm04_mtpg", "cjlmom_r6", "dgstt_low_risk", "ggth_momentum", "gh52w_52wh_6x6", "hmrw_sp500_rotation", "jowws_rs50", "jtwl_jk", "jtwl_jk_skip"];
pub fn is_basket(id: &str) -> bool { BASKET_IDS.contains(&id) }
pub const BASKET_MODEL: &str = "basket_ranking_v1";
pub const BASKET_BASIS: &str = "Basket ranking account (EP5, lane K56). The rule ranks the stocks of a vault US basket on the SPY calendar and holds the chosen stocks on one account. Universe (#239): every stock in the vault that was ever in the index (us_sp500_past / us_dow30_past: the current members and the former members with their own price files, e.g. old GM and Sears); membership dates are not known, so a stock is ranked whenever it has prices, also before it joined or after it left the index. p.members = 1 ranks today's list only (us_sp500 / us_dow30, the Norgate snapshot of 8 Sep 2020), a labelled option with survivorship bias. India: withheld, the NSE basket is the 2026 member list (#239 (2)). Prices are the vault's split-adjusted daily files, price only (no dividends). SPY supplies only the calendar and the buy-and-hold benchmark; each trade's prices are those of the stock named in its reason. A stock is ranked or traded only on days it has its own bar (a chosen stock without a bar on the fill day is not bought and its share of the vintage stays in cash; a slot entry the account cannot pay for is not bought and its slot stays open); a held stock with no bar on a day keeps its last real close in the equity line (a mark, never a fill), and an exit that falls on such a day happens at the stock's next real bar. A held stock whose file ends (a delisting, merger or rename) is sold at its last real close (a data-end convention, as the vault's basket portfolio). Idle cash that a rule puts in an ETF grows only between two consecutive days that both have the ETF's close. Shorts reserve their own principal (no leverage): a long/short vintage puts equal money on every leg. Costs: the vault fee and slippage setting per side on each position (0 by default). The run ends where the basket's price files end (11 Sep 2020).";
/// #239 (2): the India results are withheld (no dated or former-member NSE list exists)
pub const INDIA_HELD: &str = "Withheld (decision #239): the NSE basket is the 2026 member list; a ranking over 20 years of today's members is dominated by survivorship bias. No India result is published until dated members are supplied.";

pub const ANCHOR_IN: &str = "NIFTY 50";
/// Active Trader's 18-stock Standard Stock Portfolio (TSL Jul 2004 p.57, Aug 2004 p.38): the 2004 symbol and the vault file
/// that holds that company's history (us_sp500_past): GM = old General Motors, S = Sears Roebuck, T = old AT&T.
pub const ATM04_18: &[(&str, &str)] = &[("AAPL", "AAPL"), ("BA", "BA"), ("C", "C"), ("CAT", "CAT"), ("CSCO", "CSCO"), ("DIS", "DIS"), ("GM", "MTLQQ-201103"), ("HPQ", "HPQ"),
    ("IBM", "IBM"), ("INTC", "INTC"), ("IP", "IP"), ("JPM", "JPM"), ("KO", "KO"), ("MSFT", "MSFT"), ("S", "S-200503"), ("SBUX", "SBUX"), ("T", "T-200511"), ("WMT", "WMT")];

/// The basket a rule ranks in the engine's anchor context (None: no home there; India is withheld, #239 (2)).
/// US default: every stock ever in the index (`_past`); p.members = 1: today's members only (labelled option).
pub fn basket_for(id: &str, anchor: &str, p: &Params) -> Option<&'static str> {
    let today = p.get("members").copied().unwrap_or(0.) >= 0.5;
    match (id, anchor) {
        ("atm04_mtpg", ANCHOR) => Some("us_sp500_past"),
        ("ggth_momentum", ANCHOR) => Some(if today { "us_dow30" } else { "us_dow30_past" }),
        (_, ANCHOR) => Some(if today { "us_sp500" } else { "us_sp500_past" }),
        _ => None,
    }
}

/// Members aligned to the anchor's dates (NaN where a member has no bar), with each member's first and last bar.
pub struct Members { u: U, dv: Vec<Vec<f64>>, last: Vec<usize> }
impl Members {
    /// Members from (name, own rows); rows off the anchor calendar are not used.
    pub fn new(x: &Ctx, rows: Vec<(String, Vec<Row>)>) -> Self {
        let mut m = Members::empty();
        let pos = Members::dates(x);
        for (name, rs) in rows { m.add(&pos, name, &rs); }
        m
    }
    fn empty() -> Self { Members { u: U { names: vec![], o: vec![], c: vec![] }, dv: vec![], last: vec![] } }
    fn dates<'a>(x: &'a Ctx) -> BTreeMap<&'a str, usize> { x.r.iter().enumerate().map(|(k, r)| (r.date.as_str(), k)).collect() }
    /// one stock's own rows, aligned to the anchor's dates (rows off the calendar are not used); a stock with no bar there is left out
    fn add(&mut self, pos: &BTreeMap<&str, usize>, name: String, rs: &[Row]) {
        let n = pos.len();
        let (mut oo, mut cc, mut vv) = (vec![f64::NAN; n], vec![f64::NAN; n], vec![f64::NAN; n]);
        for r in rs { if let Some(&k) = pos.get(r.date.as_str()) { oo[k] = r.open; cc[k] = r.close; vv[k] = r.volume.map_or(f64::NAN, |v| v * r.close); } }
        let Some(l) = cc.iter().rposition(|v| ok(*v)) else { return };
        self.last.push(l);
        self.u.names.push(&*Box::leak(name.into_boxed_str())); self.u.o.push(oo); self.u.c.push(cc); self.dv.push(vv);
    }
    fn len(&self) -> usize { self.u.names.len() }
    fn has(&self, a: usize, k: usize) -> bool { ok(self.u.c[a][k]) }
    /// f over the member's own bars, written back on the anchor's dates (NaN on days without a bar)
    fn own(&self, a: usize, v: &[f64], f: impl Fn(&[f64]) -> Vec<f64>) -> Vec<f64> {
        let ix = self.ix(a);
        let w = f(&ix.iter().map(|&k| v[k]).collect::<Vec<_>>());
        let mut out = vec![f64::NAN; v.len()];
        for (j, &k) in ix.iter().enumerate() { out[k] = w[j]; }
        out
    }
    /// where an exit planned for bar k happens: (bar, at its own fill, data ended). k or the stock's next real bar; when the stock's
    /// file ends first, its last real close; usize::MAX when it is still held where the basket's files end
    fn exit_at(&self, a: usize, k: usize) -> (usize, bool) {
        if let Some(j) = (k..=self.last[a]).find(|&j| self.has(a, j)) { return (j, false); }
        if self.last[a] < self.end(false).unwrap_or(0) { (self.last[a], true) } else { (usize::MAX, false) }
    }
    /// a stock is bought only on its own bar
    fn can_buy(&self, a: usize, k: usize) -> bool { self.has(a, k) }
    fn ix(&self, a: usize) -> Vec<usize> { (0..self.u.c[a].len()).filter(|&k| self.has(a, k)).collect() }
    /// last real close of member a at or before bar k (a mark only)
    fn mark(&self, a: usize, k: usize) -> f64 { (0..=k).rev().map(|j| self.u.c[a][j]).find(|v| ok(*v)).unwrap_or(f64::NAN) }
    /// last bar on which the rule's stocks have prices: the latest member end, or for a fixed named list the earliest one
    pub fn end(&self, all: bool) -> Option<usize> { if all { self.last.iter().min().copied() } else { self.last.iter().max().copied() } }
}

/// The run's members, loaded once per basket from VAULT_BASKET_DATA_DIR (the engine's own basket files, checksums verified).
fn members(id: &str, x: &Ctx, p: &Params) -> Result<std::sync::Arc<Members>, String> {
    // one entry per basket of this engine (at most three: us_sp500, us_dow30 and the named list; India nse500 and nifty50)
    static CACHE: std::sync::Mutex<BTreeMap<String, std::sync::Arc<Members>>> = std::sync::Mutex::new(BTreeMap::new());
    let anchor = x.r[0].ticker.as_str();
    let basket = basket_for(id, anchor, p).ok_or_else(|| format!("not_testable: this basket rule has no stock basket in the {anchor} context"))?;
    let named = id == "atm04_mtpg";
    let dir = std::env::var("VAULT_BASKET_DATA_DIR").map_err(|_| format!("needs_data: the {basket} stock basket is not loaded in this engine"))?;
    // the calendar's identity: every date (FNV-1a), not only its length and ends
    let cal = x.r.iter().flat_map(|r| r.date.bytes()).fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
    let key = format!("{dir}|{basket}|{named}|{anchor}|{}|{cal:016x}", x.r.len());
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(m) = cache.get(&key) { return Ok(m.clone()); }
    let cat = crate::baskets::Catalogue::load(std::path::Path::new(&dir)).map_err(|e| format!("needs_data: the {basket} stock basket is unavailable ({e})"))?;
    let list = cat.members(basket);
    if list.is_empty() { return Err(format!("needs_data: the {basket} stock basket is not in this engine's basket data")); }
    // stock by stock: each file's rows are aligned and dropped before the next is read (bounded memory)
    let (mut mm, pos) = (Members::empty(), Members::dates(x));
    if named {
        for (sym, file) in ATM04_18 {
            let m = list.iter().find(|m| m.symbol == *file).ok_or_else(|| format!("needs_data: {sym} ({file}) is not in the {basket} files; the rule's 18-stock portfolio is incomplete"))?;
            let rs = cat.rows(&m.key).map_err(|e| format!("needs_data: {file}: {e}"))?;
            mm.add(&pos, if sym == file { sym.to_string() } else { format!("{sym} ({file})") }, &rs);
        }
    } else {
        for m in &list { let rs = cat.rows(&m.key).map_err(|e| format!("needs_data: {}: {e}", m.symbol))?; mm.add(&pos, m.symbol.clone(), &rs); }
    }
    let m = std::sync::Arc::new(mm);
    if named && m.len() != ATM04_18.len() { return Err("needs_data: a stock of the 18-stock portfolio has no bar on this calendar".into()); }
    cache.insert(key, m.clone());
    Ok(m)
}

/// Last bar of the basket's prices in this context (readiness::auto_end); None when the rule is not a basket rule here.
pub fn basket_end(id: &str, x: &Ctx, p: &Params) -> Option<usize> {
    if !is_basket(id) || x.r.is_empty() || basket_for(id, &x.r[0].ticker, p).is_none() { return None; }
    members(id, x, p).ok()?.end(id == "atm04_mtpg")
}

/// One planned position: entry at bar i (open or close), exit at bar x (usize::MAX = held to the end), its cash sleeve and size
/// (0 = 1/legs of the sleeve's free cash, legs = the vintage's selected stocks, so a leg without a bar leaves its share in cash;
/// f > 0 = f x equity, now or at the prior close).
struct P5 { a: usize, dir: f64, i: usize, io: bool, x: usize, xo: bool, sleeve: usize, size: f64, legs: usize, why: String, xwhy: String }
struct Book5 { sleeves: usize, prev_equity: bool, cash_px: Option<Vec<f64>> }

/// (trades, allocation at entry, equity per bar, plans that could not be funded)
fn simulate5(m: &Members, plans: &[P5], b: &Book5, from: usize, to: usize, cost: f64) -> Result<(Vec<Trade>, Vec<f64>, Vec<f64>, Vec<usize>), String> {
    let u = &m.u;
    let mut cash = vec![1.0 / b.sleeves as f64; b.sleeves];
    let (mut open, mut trades, mut alloc, mut eq) = (Vec::<(usize, f64, f64)>::new(), vec![], vec![], vec![]); // (plan, amount, entry px)
    let mut prev = 1.0f64;
    let mut skipped = vec![];
    let mut by_entry: BTreeMap<(usize, bool), Vec<usize>> = BTreeMap::new();
    for (j, p) in plans.iter().enumerate() { if p.i >= from && p.i <= to { by_entry.entry((p.i, !p.io)).or_default().push(j); } }
    let px_of = |a: usize, k: usize, at_open: bool| if at_open { u.o[a][k] } else { u.c[a][k] };
    let value = |p: &P5, amt: f64, p0: f64, px: f64, sides: f64| amt * (1. + p.dir * (px / p0 - 1.) - sides * cost);
    let close = |j: usize, amt: f64, p0: f64, k: usize, px: f64, at_open: bool, still: bool, trades: &mut Vec<Trade>, alloc: &mut Vec<f64>| {
        let p = &plans[j];
        trades.push(Trade { dir: p.dir, legs: vec![Leg { i: p.i, px: p0, w: 1. }], exit_i: k, exit_px: px, reason: format!("{}: {}; {}", u.names[p.a], p.why, if still { "still held at the last bar (marked at its last real close)" } else { &p.xwhy }),
            entry_kind: if p.io { "open" } else { "close" }, signal_i: if at_open { k.saturating_sub(1).max(p.i) } else { k }, open: still, exits: vec![] });
        alloc.push(amt);
    };
    for k in from..=to {
        // a long/short account is also checked at the open (equity at the opens, last real closes where a stock has no bar)
        if open.iter().any(|&(j, _, _)| plans[j].dir < 0.) {
            let v = cash.iter().sum::<f64>() + open.iter().map(|&(j, amt, p0)| { let p = &plans[j]; let px = px_of(p.a, k, true); value(p, amt, p0, if ok(px) { px } else { m.mark(p.a, k) }, 1.) }).sum::<f64>();
            if v <= 0. { return Err(format!("not_testable: the account's equity reached zero at the open of bar {k} (short losses beyond the reserved principal); there is no margin model, so no result is published")); }
        }
        for at_open in [true, false] {
            if !at_open {
                if let Some(v) = b.cash_px.as_ref().filter(|v| k > from && ok(v[k]) && ok(v[k - 1])) { for c in cash.iter_mut() { *c *= v[k] / v[k - 1]; } }
            }
            let mut keep = vec![];
            for (j, amt, p0) in std::mem::take(&mut open) {
                let p = &plans[j];
                if p.x == k && p.xo == at_open {
                    let px = px_of(p.a, k, at_open);
                    if !ok(px) { return Err(format!("needs_data: {} has no {} on {}; no price is filled in", u.names[p.a], if at_open { "open" } else { "close" }, k)); }
                    cash[p.sleeve] += value(p, amt, p0, px, 2.);
                    close(j, amt, p0, k, px, at_open, false, &mut trades, &mut alloc);
                } else { keep.push((j, amt, p0)); }
            }
            open = keep;
            let Some(js) = by_entry.get(&(k, !at_open)) else { continue };
            let now = cash.iter().sum::<f64>() + open.iter().map(|&(j, amt, p0)| { let p = &plans[j]; let px = px_of(p.a, k, at_open); value(p, amt, p0, if ok(px) { px } else { m.mark(p.a, k) }, 1.) }).sum::<f64>();
            let free = cash.clone();
            for &j in js {
                let p = &plans[j];
                let px = px_of(p.a, k, at_open);
                if !ok(px) { return Err(format!("needs_data: {} has no fill price on bar {k}; no price is filled in", u.names[p.a])); }
                let amt = if p.size > 0. { (p.size * if b.prev_equity { prev } else { now }).min(cash[p.sleeve]) } else { free[p.sleeve] / p.legs as f64 };
                if amt <= 1e-12 { skipped.push(j); continue; }
                cash[p.sleeve] -= amt;
                if cash[p.sleeve].abs() < 1e-12 { cash[p.sleeve] = 0.; }
                open.push((j, amt, px));
            }
            if !at_open {
                // a stock whose file ends on its entry day is sold at that same close (the data-end convention)
                for (j, amt, p0) in std::mem::take(&mut open) {
                    let p = &plans[j];
                    if p.x == k && !p.xo && p.i == k { let px = u.c[p.a][k]; cash[p.sleeve] += value(p, amt, p0, px, 2.); close(j, amt, p0, k, px, false, false, &mut trades, &mut alloc); } else { open.push((j, amt, p0)); }
                }
            }
        }
        let v = cash.iter().sum::<f64>() + open.iter().map(|&(j, amt, p0)| { let p = &plans[j]; value(p, amt, p0, m.mark(p.a, k), 1.) }).sum::<f64>();
        if v <= 0. { return Err(format!("not_testable: the account's equity reached zero on bar {k} (short losses beyond the reserved principal); there is no margin model, so no result is published")); }
        eq.push(v);
        prev = v;
    }
    for (j, amt, p0) in open { let px = m.mark(plans[j].a, to); close(j, amt, p0, to, px, false, true, &mut trades, &mut alloc); }
    let mut order: Vec<usize> = (0..trades.len()).collect();
    order.sort_by(|&a, &b| (trades[a].legs[0].i, trades[a].exit_i, &trades[a].reason).cmp(&(trades[b].legs[0].i, trades[b].exit_i, &trades[b].reason)));
    Ok((order.iter().map(|&j| trades[j].clone()).collect(), order.iter().map(|&j| alloc[j]).collect(), eq, skipped))
}

fn pint(p: &Params, k: &str, d: f64, lo: f64, hi: f64) -> usize { p.get(k).copied().unwrap_or(d).round().clamp(lo, hi) as usize }

/// Monthly cross-sectional momentum with overlapping vintages (Jegadeesh-Titman 1993, Chan-Jegadeesh-Lakonishok 1996,
/// George-Hwang 2004, Molchanov-Stork in Gregoriou 2010). Ranked at a month's last close on data up to that close.
struct Mom { j: usize, k: usize, high52: bool, frac: f64, count: usize, entry_days: usize, exit_days: usize, skip_month: bool, long_only: bool }
fn momentum(x: &Ctx, m: &Members, from: usize, to: usize, c: &Mom) -> Vec<P5> {
    let (n, u) = (x.r.len(), &m.u);
    let me: Vec<usize> = (0..n).filter(|&k| month_end(x, k)).collect();
    let lb = if c.high52 { 12 } else { c.j };
    let what = if c.high52 { "close / 12-month highest close".to_string() } else { format!("{}-month return", c.j) };
    let mut plans = vec![];
    for q in lb..me.len() {
        let (s, b) = (me[q], me[q - lb]);
        let e = if c.skip_month { match me.get(q + 1) { Some(&v) => v, None => continue } } else { s + c.entry_days };
        if e >= n || e < from || e > to { continue; }
        let xb = me.get(q + c.k + c.skip_month as usize).map(|&v| v + c.exit_days).filter(|&v| v < n);
        let sc: Vec<f64> = (0..m.len()).map(|a| {
            if !(m.has(a, s) && m.has(a, b)) { return f64::NAN; }
            if c.high52 { u.c[a][s] / u.c[a][b + 1..=s].iter().copied().filter(|v| ok(*v)).fold(f64::NAN, f64::max) } else { u.c[a][s] / u.c[a][b] - 1. }
        }).collect();
        let elig: Vec<usize> = u.rank(&sc).into_iter().filter(|&a| sc[a].is_finite()).collect();
        let nsel = if c.count > 0 { c.count } else { (elig.len() as f64 * c.frac).floor() as usize };
        if nsel == 0 || 2 * nsel > elig.len() { continue; }
        for (dir, set, tag) in [(1., &elig[..nsel], "winners"), (-1., &elig[elig.len() - nsel..], "losers")] {
            if dir < 0. && c.long_only { continue; }
            for &a in set {
                if !m.can_buy(a, e) { continue; } // no bar at the fill: that leg's share stays in cash
                let r = elig.iter().position(|&v| v == a).unwrap() + 1;
                let (xi, gone) = m.exit_at(a, xb.unwrap_or(n)); // exit month beyond the data: held, or sold where its file ends
                plans.push(P5 { a, dir, i: e, io: false, x: xi, xo: false, sleeve: q % c.k, size: 0., legs: if c.long_only { nsel } else { 2 * nsel },
                    why: format!("{} {tag}: rank {r} of {} by {what} {} at {}", if dir > 0. { "bought" } else { "sold short" }, elig.len(), if c.high52 { format!("{:.3}", sc[a]) } else { pct(sc[a]) }, x.r[s].date),
                    xwhy: if gone { "the stock's price file ends".into() } else { format!("{}-month holding period ended", c.k) } });
            }
        }
    }
    plans
}

/// lane K81: O'Shaughnessy, What Works on Wall Street (1997) p.28, 192: at each year's last close buy the 50 stocks with the
/// largest price change over the year (this close / the previous year-end close), equal weight, sold at the next year-end
/// close; p.worst = 1: the 50 worst. One portfolio a year (one sleeve).
fn jowws(x: &Ctx, m: &Members, from: usize, to: usize, p: &Params) -> Vec<P5> {
    const N: usize = 50;
    let (n, u) = (x.r.len(), &m.u);
    let worst = p.get("worst").copied().unwrap_or(0.) >= 0.5;
    let ye: Vec<usize> = (0..n).filter(|&k| year_end(x, k)).collect();
    let mut plans = vec![];
    for q in 1..ye.len() {
        let (s, b) = (ye[q], ye[q - 1]);
        if s < from || s > to { continue; }
        let sc: Vec<f64> = (0..m.len()).map(|a| if m.has(a, s) && m.has(a, b) { u.c[a][s] / u.c[a][b] - 1. } else { f64::NAN }).collect();
        let elig: Vec<usize> = u.rank(&sc).into_iter().filter(|&a| sc[a].is_finite()).collect();
        if elig.len() < N { continue; }
        let set = if worst { &elig[elig.len() - N..] } else { &elig[..N] };
        for &a in set {
            let r = elig.iter().position(|&v| v == a).unwrap() + 1;
            let (xi, gone) = m.exit_at(a, ye.get(q + 1).copied().unwrap_or(n));
            plans.push(P5 { a, dir: 1., i: s, io: false, x: xi, xo: false, sleeve: 0, size: 0., legs: N,
                why: format!("bought: rank {r} of {} by the 1-year price change {} at {}", elig.len(), pct(sc[a]), x.r[s].date),
                xwhy: if gone { "the stock's price file ends".into() } else { "1-year holding period ended".into() } });
        }
    }
    plans
}

/// Daragan, Short-Term Trading Analysis ch.2-6: least-squares trend T and deviation D over the last N closes, i = 1..N.
fn daragan_td(c: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    let (mut t, mut d) = (vec![f64::NAN; c.len()], vec![f64::NAN; c.len()]);
    let nf = n as f64;
    let (si, sii) = (nf * (nf + 1.) / 2., nf * (nf + 1.) * (2. * nf + 1.) / 6.);
    for j in 32..c.len() { // at least 32 trading days of history before the day of analysis (p.25)
        let w = &c[j + 1 - n..=j];
        let sp: f64 = w.iter().sum();
        let sip: f64 = w.iter().enumerate().map(|(i, p)| (i + 1) as f64 * p).sum();
        let bb = (nf * sip - si * sp) / (nf * sii - si * si);
        let aa = (sp - bb * si) / nf;
        let sig = (w.iter().enumerate().map(|(i, p)| (p - aa - bb * (i + 1) as f64).powi(2)).sum::<f64>() / nf).sqrt();
        let fit = aa + bb * nf;
        t[j] = bb / fit * 100.;
        if sig > 0. { d[j] = (w[n - 1] - fit) / sig; }
    }
    (t, d)
}
fn zscores(v: &[f64]) -> Vec<f64> {
    let f: Vec<f64> = v.iter().copied().filter(|x| x.is_finite()).collect();
    if f.len() < 2 { return vec![f64::NAN; v.len()]; }
    let mu = f.iter().sum::<f64>() / f.len() as f64;
    let sd = (f.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / f.len() as f64).sqrt();
    v.iter().map(|x| if sd > 0. { (x - mu) / sd } else { f64::NAN }).collect()
}
fn dgstt(x: &Ctx, m: &Members, from: usize, to: usize, p: &Params) -> Vec<P5> {
    let (n, u, na) = (x.r.len(), &m.u, m.len());
    let level = 1. - p.get("drop").copied().unwrap_or(10.).clamp(0., 50.) / 100.;
    let td: Vec<[Vec<f64>; 4]> = (0..na).map(|a| {
        let ix = m.ix(a);
        let c: Vec<f64> = ix.iter().map(|&k| u.c[a][k]).collect();
        let mut out: [Vec<f64>; 4] = std::array::from_fn(|_| vec![f64::NAN; n]);
        for (f, len) in [16usize, 32].into_iter().enumerate() {
            let (t, d) = daragan_td(&c, len);
            for (j, &k) in ix.iter().enumerate() { out[2 * f][k] = t[j]; out[2 * f + 1][k] = d[j]; }
        }
        out
    }).collect();
    let mut plans = vec![];
    for t in from.max(1)..=to.min(n - 1) {
        let t0 = t - 1; // day #0, the day of analysis
        let z: Vec<Vec<f64>> = (0..4).map(|f| zscores(&td.iter().map(|v| v[f][t0]).collect::<Vec<_>>())).collect();
        let mut cand: Vec<(f64, usize)> = (0..na).filter(|&a| ((z[0][a] < -1. && z[1][a] < -1.) || (z[2][a] < -1. && z[3][a] < -1.)) && m.can_buy(a, t) && m.has(a, t0))
            .map(|a| (u.c[a][t] / u.c[a][t0], a)).collect();
        cand.sort_by(|p, q| p.0.total_cmp(&q.0).then(u.names[p.1].cmp(u.names[q.1])));
        if cand.len() < 2 || !(cand[0].0 < level && cand[1].0 < level) { continue; }
        for &(r, a) in &cand[..2] {
            let (xi, gone) = m.exit_at(a, (t + 3).min(n));
            plans.push(P5 { a, dir: 1., i: t, io: false, x: xi, xo: !gone, sleeve: t % 3, size: 0., legs: 2,
                why: format!("potentially bullish on {} (T-D cell 13 in the 16 or 32 day frame), fell {} on day 1, one of the two largest drops of the {} listed", x.r[t0].date, pct(r - 1.), cand.len()),
                xwhy: if gone { "the stock's price file ends".into() } else { "sold at the open of day 4".into() } });
        }
    }
    plans
}

/// The indicators are computed once; the returned planner takes the entries the account could not pay for.
fn hmrw<'a>(x: &'a Ctx<'a>, m: &'a Members, from: usize, to: usize, p: &Params) -> impl Fn(&std::collections::BTreeSet<(usize, usize)>) -> Vec<P5> + 'a {
    let (n, u, na) = (x.r.len(), &m.u, m.len());
    let rn = pint(p, "rsi_n", 2., 2., 14.);
    let (lim, hvmax) = (p.get("rsi").copied().unwrap_or(15.), p.get("hv").copied().unwrap_or(35.));
    let spy = crate::connors::sma(&x.c, 200);
    let ind: Vec<[Vec<f64>; 3]> = (0..na).map(|a| [m.own(a, &u.c[a], |c| crate::connors::sma(c, 200)), m.own(a, &u.c[a], |c| crate::connors::rsi(c, rn)), m.own(a, &u.c[a], |c| crate::connors::hv(c, 100))]).collect();
    let down2: Vec<Vec<f64>> = (0..na).map(|a| m.own(a, &u.c[a], |c| (0..c.len()).map(|j| if j >= 2 && c[j] < c[j - 1] && c[j - 1] < c[j - 2] { 1. } else { 0. }).collect())).collect();
    let we: Vec<usize> = (0..n).filter(|&k| week_end(x, k)).collect();
    move |unfunded| {
    let mut plans: Vec<P5> = vec![];
    for (wi, &w) in we.iter().enumerate() {
        if w < from || w > to || !(x.c[w] > spy[w]) { continue; }
        let mut q: Vec<usize> = (0..na).filter(|&a| m.can_buy(a, w) && u.c[a][w] > ind[a][0][w] && down2[a][w] == 1. && ind[a][1][w] < lim && ind[a][2][w] < hvmax).collect();
        q.sort_by(|&a, &b| ind[b][2][w].total_cmp(&ind[a][2][w]).then(u.names[a].cmp(u.names[b])));
        let nq = q.len();
        let busy = plans.iter().filter(|p| p.i < w && p.x > w).count(); // positions whose exit was delayed by a missing bar
        q.retain(|&a| !plans.iter().any(|p| p.a == a && p.i < w && p.x > w) && !unfunded.contains(&(a, w)));
        for (r, &a) in q.iter().take(10usize.saturating_sub(busy)).enumerate() {
            let (xi, gone) = m.exit_at(a, we.get(wi + 1).copied().unwrap_or(n)); // no later week end: held, or sold where its file ends
            plans.push(P5 { a, dir: 1., i: w, io: false, x: xi, xo: false, sleeve: 0, size: 0.1, legs: 0,
                why: format!("week-end {}: index above its 200-day SMA; above its 200-day SMA, down 2 days, RSI({rn}) {:.1} < {lim}, HV100 {:.1} < {hvmax}; {} of {nq} by highest HV100", x.r[w].date, ind[a][1][w], ind[a][2][w], r + 1),
                xwhy: if gone { "the stock's price file ends".into() } else { "sold at the next week-end close".into() } });
        }
    }
    plans
    }
}

/// `unfunded`: (stock, bar) entries the account could not pay for (no free cash); they are not bought and keep their slot open.
fn alpha_wmr<'a>(x: &'a Ctx<'a>, m: &'a Members, from: usize, to: usize, p: &Params) -> impl Fn(&std::collections::BTreeSet<(usize, usize)>) -> Vec<P5> + 'a {
    let (n, u, na) = (x.r.len(), &m.u, m.len());
    let rn = pint(p, "rsi_n", 2., 2., 4.);
    let (lo, hi, stop) = (p.get("entry").copied().unwrap_or(20.), p.get("exit").copied().unwrap_or(80.), p.get("stop_pct").copied().unwrap_or(10.) / 100.);
    let slots = pint(p, "slots", 10., 1., 50.);
    let hv: Vec<Vec<f64>> = (0..na).map(|a| m.own(a, &u.c[a], |c| crate::connors::hv(c, 100))).collect();
    let adv: Vec<Vec<f64>> = (0..na).map(|a| m.own(a, &m.dv[a], |v| crate::connors::sma(v, 200))).collect();
    // weekly RSI: the stock's last real close of each Monday-Sunday week, written on the anchor's last bar of that week
    let wk: Vec<usize> = (0..n).filter(|&k| k + 1 == n || week(x, k) != week(x, k + 1)).collect();
    let wrsi: Vec<Vec<f64>> = (0..na).map(|a| {
        let mut ends = vec![]; let mut start = 0;
        for &e in &wk { if let Some(j) = (start..=e).rev().find(|&j| m.has(a, j)) { ends.push((e, u.c[a][j])); } start = e + 1; }
        let r = crate::connors::rsi(&ends.iter().map(|v| v.1).collect::<Vec<_>>(), rn);
        let mut out = vec![f64::NAN; n]; for (q, &(e, _)) in ends.iter().enumerate() { out[e] = r[q]; } out
    }).collect();
    let me: Vec<usize> = (0..n).filter(|&k| month_end(x, k)).collect();
    move |unfunded| {
    let (mut liquid, mut mi) = (vec![false; na], 0usize);
    let mut held: Vec<(usize, usize)> = vec![]; // (plan, member)
    let mut plans: Vec<P5> = vec![];
    for k in 0..=to.min(n - 1) {
        while mi < me.len() && me[mi] <= k {
            // the 500 most liquid by 200-day average dollar volume, set at each month's last close
            let s = me[mi];
            let mut r: Vec<usize> = (0..na).filter(|&a| adv[a][s].is_finite()).collect();
            r.sort_by(|&a, &b| adv[b][s].total_cmp(&adv[a][s]).then(u.names[a].cmp(u.names[b])));
            liquid = vec![false; na]; for &a in r.iter().take(500) { liquid[a] = true; }
            mi += 1;
        }
        if k < from { continue; }
        let is_we = week_end(x, k);
        let (mut still, mut sold) = (vec![], vec![]);
        for (j, a) in std::mem::take(&mut held) {
            let pl = &mut plans[j];
            if m.has(a, k) && k > pl.i {
                let entry = u.c[a][pl.i];
                if u.c[a][k] < entry * (1. - stop) { pl.x = k; pl.xwhy = format!("close {:.4} more than {}% below the entry {:.4}", u.c[a][k], stop * 100., entry); sold.push(a); continue; }
                if is_we && wrsi[a][k] > hi { pl.x = k; pl.xwhy = format!("weekly RSI({rn}) {:.1} > {hi}", wrsi[a][k]); sold.push(a); continue; }
            }
            if k == m.last[a] && m.exit_at(a, k + 1).1 { pl.x = k; pl.xwhy = "the stock's price file ends".into(); continue; }
            still.push((j, a));
        }
        held = still;
        if !is_we || k < 126 || !(x.c[k] / x.c[k - 126] - 1. > 0.) || held.len() >= slots { continue; }
        let mut q: Vec<usize> = (0..na).filter(|&a| liquid[a] && m.can_buy(a, k) && wrsi[a][k] < lo && hv[a][k].is_finite() && !held.iter().any(|h| h.1 == a) && !sold.contains(&a) && !unfunded.contains(&(a, k))).collect();
        q.sort_by(|&a, &b| hv[a][k].total_cmp(&hv[b][k]).then(u.names[a].cmp(u.names[b])));
        let free = slots - held.len();
        for &a in q.iter().take(free) {
            plans.push(P5 { a, dir: 1., i: k, io: false, x: usize::MAX, xo: false, sleeve: 0, size: 1. / slots as f64, legs: 0,
                why: format!("week-end {}: index 126-day return {} > 0; weekly RSI({rn}) {:.1} < {lo}; HV100 {:.1}, lowest first of {} candidates for {free} open slot(s)", x.r[k].date, pct(x.c[k] / x.c[k - 126] - 1.), wrsi[a][k], hv[a][k], q.len()),
                xwhy: String::new() });
            if k == m.last[a] && m.exit_at(a, k + 1).1 { let pl = plans.last_mut().unwrap(); pl.x = k; pl.xwhy = "the stock's price file ends".into(); continue; } // sold at this close
            held.push((plans.len() - 1, a));
        }
    }
    plans
    }
}

fn atm04(x: &Ctx, m: &Members, from: usize, to: usize, p: &Params, unfunded: &std::collections::BTreeSet<(usize, usize)>) -> Vec<P5> {
    let (n, u, na) = (x.r.len(), &m.u, m.len());
    let losers = p.get("losers").copied().unwrap_or(0.) >= 0.5;
    let top = pint(p, "top", 5., 1., 9.);
    let me: Vec<usize> = (0..n).filter(|&k| month_end(x, k)).collect();
    let mut held: Vec<(usize, usize)> = vec![];
    let mut plans: Vec<P5> = vec![];
    for q in 1..me.len() {
        let (s, b) = (me[q], me[q - 1]);
        let f2 = s + 2; // the second trading day of the next month
        if f2 >= n || f2 > to || f2 < from || x.r[f2].date[..7] != x.r[s + 1].date[..7] { continue; }
        if !(0..na).all(|a| m.has(a, s) && m.has(a, b)) { continue; } // the rule ranks all 18; a month with one missing is not ranked
        let mut sc: Vec<f64> = (0..na).map(|a| u.c[a][s] / u.c[a][b] - 1.).collect();
        if losers { for v in sc.iter_mut() { *v = -*v; } }
        let pick: Vec<usize> = u.rank(&sc).into_iter().take(top).collect();
        let mut still = vec![];
        for (j, a) in std::mem::take(&mut held) {
            if pick.contains(&a) { still.push((j, a)); continue; }
            if !m.has(a, f2) { // no bar on the rotation day: sold at its next real open (or its file's last close)
                let (xi, gone) = m.exit_at(a, f2);
                plans[j].x = xi; plans[j].xo = !gone; plans[j].xwhy = format!("no longer a top-{top} {} of {} (sold at its next real bar)", if losers { "loser" } else { "gainer" }, &x.r[s].date[..7]);
                continue;
            }
            plans[j].x = f2; plans[j].xo = true; plans[j].xwhy = format!("no longer a top-{top} {} of {}", if losers { "loser" } else { "gainer" }, &x.r[s].date[..7]);
        }
        held = still;
        for &a in &pick {
            if held.iter().any(|h| h.1 == a) || !m.has(a, f2) || unfunded.contains(&(a, f2)) { continue; } // an unpaid buy is not a holding
            let r = pick.iter().position(|&v| v == a).unwrap() + 1;
            plans.push(P5 { a, dir: 1., i: f2, io: true, x: usize::MAX, xo: false, sleeve: 0, size: 0.18, legs: 0,
                why: format!("top-{top} {} of {} ({} {}), bought at the open of the second trading day", if losers { "loser" } else { "gainer" }, &x.r[s].date[..7], r, pct(u.c[a][s] / u.c[a][b] - 1.)), xwhy: String::new() });
            held.push((plans.len() - 1, a));
        }
    }
    plans
}

/// Slot rules whose later choices depend on what is held: an entry the account cannot pay for (no free cash) is not a holding
/// and keeps its slot open, so the rule is planned again without it until every planned entry is funded.
fn funded(m: &Members, book: Book5, from: usize, to: usize, cost: f64, plan: impl Fn(&std::collections::BTreeSet<(usize, usize)>) -> Vec<P5>) -> Result<(Vec<P5>, Book5), String> {
    let mut unfunded = std::collections::BTreeSet::new();
    loop {
        let pl = plan(&unfunded);
        let skipped = simulate5(m, &pl, &book, from, to, cost)?.3;
        // only the earliest bar's skips are certain (everything before it is unchanged; cash cannot grow inside one bar's entries)
        let Some(first) = skipped.iter().map(|&j| pl[j].i).min() else { return Ok((pl, book)) };
        for j in skipped.into_iter().filter(|&j| pl[j].i == first) { unfunded.insert((pl[j].a, pl[j].i)); }
    }
}

/// The basket ranking account (None = not a basket rule; Err = refused or a data gap).
pub fn basket_account(id: &str, x: &Ctx, from: usize, to: usize, cost: f64, p: &Params) -> Option<Result<crate::options::Account, String>> {
    if !is_basket(id) { return None; }
    Some((|| {
        let anchor = x.r.first().map(|r| r.ticker.as_str()).unwrap_or("");
        if anchor == ANCHOR_IN { return Err(format!("not_testable: {INDIA_HELD}")); }
        if basket_for(id, anchor, p).is_none() {
            return Err(format!("not_testable: this rule ranks a stock basket and runs only on the {ANCHOR} calendar (SPY is the calendar and the benchmark; the trades are the US stocks named in each trade)"));
        }
        let m = members(id, x, p)?;
        let end = m.end(id == "atm04_mtpg").unwrap_or(0);
        if from >= end { return Err(format!("needs_data: the basket's stock prices end on {}; the selected period starts later, and no price is filled in", x.r[end].date)); }
        let side = p.get("long_only").copied().unwrap_or(0.) >= 0.5;
        let mom = |j: usize, k: usize, high52: bool, frac: f64, count: usize, entry_days: usize, exit_days: usize, skip_month: bool| Mom { j, k, high52, frac, count, entry_days, exit_days, skip_month, long_only: side };
        let (plans, book) = match id {
            "jtwl_jk" | "jtwl_jk_skip" => {
                let (j, k, d) = (pint(p, "j", 6., 1., 12.), pint(p, "k", 6., 1., 12.), if id == "jtwl_jk_skip" { 5 } else { 0 });
                (momentum(x, &m, from, to, &mom(j, k, false, 0.1, 0, d, d, false)), Book5 { sleeves: k, prev_equity: false, cash_px: None })
            }
            "cjlmom_r6" => { let k = pint(p, "k", 6., 6., 12.); (momentum(x, &m, from, to, &mom(6, k, false, 0.1, 0, 5, 0, false)), Book5 { sleeves: k, prev_equity: false, cash_px: None }) }
            "gh52w_52wh_6x6" => (momentum(x, &m, from, to, &mom(12, 6, true, 0.3, 0, 0, 0, false)), Book5 { sleeves: 6, prev_equity: false, cash_px: None }),
            "ggth_momentum" => {
                let (j, k) = (pint(p, "j", 12., 1., 12.), pint(p, "k", 6., 1., 12.));
                (momentum(x, &m, from, to, &mom(j, k, false, 0., pint(p, "n", 5., 1., 10.), 0, 0, true)), Book5 { sleeves: k, prev_equity: false, cash_px: None })
            }
            "dgstt_low_risk" => (dgstt(x, &m, from, to, p), Book5 { sleeves: 3, prev_equity: false, cash_px: None }),
            "jowws_rs50" => (jowws(x, &m, from, to, p), Book5 { sleeves: 1, prev_equity: false, cash_px: None }),
            "hmrw_sp500_rotation" => funded(&m, Book5 { sleeves: 1, prev_equity: false, cash_px: None }, from, to, cost, hmrw(x, &m, from, to, p))?,
            "alpha_weekly_mr" => {
                // idle cash in SHY where the price file holds SHY (US); otherwise cash earns nothing
                let shy = if anchor == ANCHOR { PRELOADED.get().and_then(|d| universe(x, d, &["SHY"]).ok()).map(|v| v.c[0].clone()) } else { None };
                funded(&m, Book5 { sleeves: 1, prev_equity: false, cash_px: shy }, from, to, cost, alpha_wmr(x, &m, from, to, p))?
            }
            "atm04_mtpg" => funded(&m, Book5 { sleeves: 1, prev_equity: true, cash_px: None }, from, to, cost, |u| atm04(x, &m, from, to, p, u))?,
            _ => return Err("not_testable: unknown basket rule".into()),
        };
        let (trades, alloc, equity, _) = simulate5(&m, &plans, &book, from, to, cost)?;
        let (_, _, gross, _) = simulate5(&m, &plans, &book, from, to, 0.)?;
        Ok(crate::options::Account { trades, alloc, equity, gross, ruin: None, model: BASKET_MODEL, basis: BASKET_BASIS })
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
    fn jowws_buys_the_50_best_one_year_performers_at_each_year_end() {
        // lane K81: 3 years of 28-bar months; stock i grows by (i + 1) / 10000 a bar, so the 50 best are i = 10..59
        let n = 36 * 28;
        let spy = rows("SPY", &vec![100.; n]);
        let x = Ctx::new(&spy, &spy, &[]);
        let mem: Vec<(String, Vec<Row>)> = (0..60).map(|i| { let t = format!("S{i:02}"); let r = rows(&t, &(0..n).map(|k| 10. * (1. + (i as f64 + 1.) / 10000.).powi(k as i32)).collect::<Vec<_>>()); (t, r) }).collect();
        let m = Members::new(&x, mem);
        let pl = jowws(&x, &m, 0, n - 1, &Params::new());
        // year ends: bars 335 (Dec 2001) and 671 (Dec 2002); bar 1007 is the last bar (not a year end): one portfolio, at 671, held to the end (usize::MAX)
        assert_eq!(pl.len(), 50);
        assert!(pl.iter().all(|p| p.i == 671 && p.x == usize::MAX && p.legs == 50 && m.u.names[p.a] >= "S10"));
        let w = jowws(&x, &m, 0, n - 1, &[("worst".to_string(), 1.)].into_iter().collect());
        assert!(w.iter().all(|p| m.u.names[p.a] < "S50") && w.len() == 50);
        assert!(crate::gate::get("jowws_rs50").is_some_and(|g| g.allowed == Some(&["SPY"][..])));
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
        // lane K54 / K61: prices constant inside each year; yearly returns DIA 10/10/5%, EWJ +30/-20/+30%, EWG -20/+30/-20%;
        // SPY is only the calendar (flat), so a US weight on SPY instead of DIA would fail here
        let year = 12 * 28; let n = 5 * year;
        let px = |p: [f64; 5]| (0..n).map(|k| p[k / year]).collect::<Vec<f64>>();
        let spy = rows("SPY", &vec![50.; n]);
        let data = BTreeMap::from([("SPY".to_string(), spy.clone()), ("DIA".to_string(), rows("DIA", &px([100., 110., 121., 127.05, 127.05]))),
            ("EWJ".to_string(), rows("EWJ", &px([100., 130., 104., 135.2, 135.2]))), ("EWG".to_string(), rows("EWG", &px([100., 80., 104., 83.2, 83.2])))]);
        let x = Ctx::new(&spy, &spy, &[]);
        let w = |id: &str| { let a = account(id, &x, 0, n - 1, 0., &Params::new(), Some(&data)).unwrap().unwrap();
            let k = 4 * year - 1; // the 4th year-end: the first with three yearly returns behind it
            let mut v: Vec<(String, f64)> = a.trades.iter().zip(&a.alloc).filter(|(t, _)| t.legs[0].i == k).map(|(t, &al)| (t.reason.split(':').next().unwrap().to_string(), al)).collect();
            v.sort_by(|p, q| p.0.cmp(&q.0)); v };
        let near = |v: Vec<(String, f64)>, want: &[(&str, f64)]| v.len() == want.len() && v.iter().zip(want).all(|(a, b)| a.0 == b.0 && (a.1 - b.1).abs() < 1e-9);
        assert!(near(w("kfgei_alloc_vol"), &[("DIA", 2. / 3.), ("EWG", 1. / 6.), ("EWJ", 1. / 6.)])); // DIA: the lowest SD
        assert!(near(w("kfgei_alloc_rar"), &[("DIA", 2. / 3.), ("EWG", 1. / 6.), ("EWJ", 1. / 6.)])); // DIA: the best return / SD
        assert!(near(w("kfgei_alloc_corr"), &[("DIA", 0.7), ("EWJ", 0.3)])); // EWJ moves against DIA, EWG with it
        assert!(near(w("kfgei_alloc_combined"), &[("DIA", (4. / 3. + 0.7) / 3.), ("EWG", 1. / 9.), ("EWJ", 0.6333333333333333 / 3.)]));
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
    // ---------------------------------------------------------------- EP5 (lane K56)
    fn basket(n: usize, px: impl Fn(usize, usize) -> f64, k: usize) -> (Vec<Row>, Vec<(String, Vec<Row>)>) {
        // anchor rising 0.1 a bar (index above its averages, positive 126-day return); members with volume
        let spy = rows("SPY", &(0..n).map(|d| 100. + d as f64 / 10.).collect::<Vec<_>>());
        let mem = (0..k).map(|a| { let nm = format!("S{a:02}"); (nm.clone(), rows(&nm, &(0..n).map(|d| px(a, d)).collect::<Vec<_>>()).into_iter().map(|mut r| { r.volume = Some(1000.); r }).collect()) }).collect();
        (spy, mem)
    }
    fn every_fill_is_a_real_bar(m: &Members, t: &[Trade]) {
        for tr in t {
            let a = m.u.names.iter().position(|nm| tr.reason.starts_with(&format!("{nm}:"))).unwrap();
            let e = if tr.entry_kind == "open" { m.u.o[a][tr.legs[0].i] } else { m.u.c[a][tr.legs[0].i] };
            assert_eq!(e, tr.legs[0].px, "{}", tr.reason);
            assert!(m.has(a, tr.exit_i) && (tr.exit_px == m.u.c[a][tr.exit_i] || tr.exit_px == m.u.o[a][tr.exit_i]), "{}", tr.reason);
        }
    }
    #[test]
    fn ep5_ties_rank_by_ticker_and_deciles_take_the_extremes() {
        // 20 stocks, 8 months: S00-S09 rise 1%/bar, S10-S19 flat; equal scores inside each group
        let n = 8 * 28;
        let (spy, mem) = basket(n, |a, d| if a < 10 { 10. * 1.01f64.powi(d as i32) } else { 10. }, 20);
        let x = Ctx::new(&spy, &spy, &[]);
        let m = Members::new(&x, mem);
        let pl = momentum(&x, &m, 0, n - 1, &Mom { j: 1, k: 1, high52: false, frac: 0.1, count: 0, entry_days: 0, exit_days: 0, skip_month: false, long_only: false });
        // decile of 20 = 2: winners S00, S01 (ties by ticker), losers the last two by ticker S18, S19
        let first: Vec<(&str, f64)> = pl.iter().filter(|p| p.i == 55).map(|p| (m.u.names[p.a], p.dir)).collect();
        assert_eq!(first, vec![("S00", 1.), ("S01", 1.), ("S18", -1.), ("S19", -1.)]);
    }
    #[test]
    fn ep5_overlapping_vintages_roll_their_own_sleeve_and_shorts_reserve_principal() {
        // 4 stocks; S00 rises 1%/bar, S03 falls 1%/bar, the others flat; J = 1, K = 3, winner and loser = 1 each (count)
        let n = 10 * 28;
        let (spy, mem) = basket(n, |a, d| match a { 0 => 10. * 1.01f64.powi(d as i32), 3 => 10. * 0.99f64.powi(d as i32), _ => 10. }, 4);
        let x = Ctx::new(&spy, &spy, &[]);
        let m = Members::new(&x, mem);
        let c = Mom { j: 1, k: 3, high52: false, frac: 0., count: 1, entry_days: 0, exit_days: 0, skip_month: false, long_only: false };
        let pl = momentum(&x, &m, 0, n - 1, &c);
        let (t, al, eq, _) = simulate5(&m, &pl, &Book5 { sleeves: 3, prev_equity: false, cash_px: None }, 0, n - 1, 0.).unwrap();
        every_fill_is_a_real_bar(&m, &t);
        // first formation at month end 2 (bar 55): sleeve 1 of 3 = 1/3, half long S00, half short S03; exits 3 month ends later (bar 139)
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (55, 139));
        assert!((al[0] - 1. / 6.).abs() < 1e-12 && (al[1] - 1. / 6.).abs() < 1e-12);
        // the vintage formed at bar 139 reuses the cash of the one it replaces (same sleeve)
        let v = 1. / 12. * (1.01f64.powi(84) + 2. - 0.99f64.powi(84)); // the sleeve's proceeds, split over the 2 new legs
        let re: Vec<f64> = t.iter().zip(&al).filter(|(tr, _)| tr.legs[0].i == 139).map(|(_, &a)| a).collect();
        assert!(re.iter().all(|a| (a - v).abs() < 1e-9), "{re:?} {v}");
        // with three vintages running, no cash is idle: equity at a month end = cash 0 + marks
        assert!(eq.iter().all(|v| v.is_finite() && *v > 0.));
    }
    #[test]
    fn ep5_skip_month_buys_a_month_after_the_ranking_and_exits_k_months_later() {
        let n = 10 * 28;
        let (spy, mem) = basket(n, |a, d| 10. + (a * d) as f64 / 100., 12);
        let x = Ctx::new(&spy, &spy, &[]);
        let m = Members::new(&x, mem);
        let pl = momentum(&x, &m, 0, n - 1, &Mom { j: 1, k: 2, high52: false, frac: 0., count: 5, entry_days: 0, exit_days: 0, skip_month: true, long_only: false });
        // ranked at month end 1 (bar 27 vs bar ... ) : first ranking needs J = 1 month: me[1] = 55 vs me[0] = 27; bought at me[2] = 83
        assert_eq!(pl[0].i, 83);
        assert!(pl[0].why.contains(&x.r[55].date));
        assert!(pl.iter().all(|p| p.x == p.i + 2 * 28 || p.x == usize::MAX));
        assert_eq!(pl.iter().filter(|p| p.i == 83 && p.dir > 0.).map(|p| m.u.names[p.a]).collect::<Vec<_>>(), vec!["S11", "S10", "S09", "S08", "S07"]);
    }
    #[test]
    fn ep5_no_look_ahead_truncated_data_keeps_every_earlier_decision() {
        // pseudo-random prices; plans that enter before the cut are identical when the data stop at the cut
        let n = 30 * 28;
        let f = |a: usize, d: usize| { let h = ((a as f64 * 12.9898 + d as f64 * 78.233).sin() * 43758.5453).fract().abs(); 20. * (1. + (a as f64 - 10.) / 2000.).powi(d as i32) * (0.8 + 0.4 * h) };
        let (spy, mem) = basket(n, f, 25);
        let cut = 20 * 28 + 5;
        let (spy2, mem2) = (spy[..cut].to_vec(), mem.iter().map(|(nm, r)| (nm.clone(), r[..cut].to_vec())).collect::<Vec<_>>());
        let (x, x2) = (Ctx::new(&spy, &spy, &[]), Ctx::new(&spy2, &spy2, &[]));
        let (m, m2) = (Members::new(&x, mem), Members::new(&x2, mem2));
        let key = |v: Vec<P5>, lim: usize| v.into_iter().filter(|p| p.i < lim).map(|p| (p.a, p.i, p.dir, if p.x < lim { p.x } else { 0 })).collect::<Vec<_>>();
        let mom = Mom { j: 3, k: 3, high52: true, frac: 0.3, count: 0, entry_days: 0, exit_days: 0, skip_month: false, long_only: false };
        assert_eq!(key(momentum(&x, &m, 0, n - 1, &mom), cut - 1), key(momentum(&x2, &m2, 0, cut - 1, &mom), cut - 1));
        let p = Params::new();
        let pd = Params::from([("drop".to_string(), 0.)]); // any decline on day 1, so this small random basket has picks
        assert_eq!(key(dgstt(&x, &m, 0, n - 1, &pd), cut - 4), key(dgstt(&x2, &m2, 0, cut - 1, &pd), cut - 4));
        assert_eq!(key(alpha_wmr(&x, &m, 0, n - 1, &p)(&Default::default()), cut - 1), key(alpha_wmr(&x2, &m2, 0, cut - 1, &p)(&Default::default()), cut - 1));
        assert_eq!(key(hmrw(&x, &m, 0, n - 1, &p)(&Default::default()), cut - 6), key(hmrw(&x2, &m2, 0, cut - 1, &p)(&Default::default()), cut - 6));
        for (nm, v) in [("momentum", momentum(&x, &m, 0, n - 1, &mom)), ("dgstt", dgstt(&x, &m, 0, n - 1, &pd)), ("alpha", alpha_wmr(&x, &m, 0, n - 1, &p)(&Default::default()))] { assert!(v.iter().filter(|q| q.i < cut - 4).count() > 0, "{nm} made no decision"); }
    }
    #[test]
    fn ep5_a_chosen_stock_without_a_fill_bar_leaves_its_share_in_cash() {
        // skip-month vintage: S03 (the loser) has no bar on the fill day (83); the winner gets half the sleeve, not all of it
        let n = 6 * 28;
        let (spy, mut mem) = basket(n, |a, d| match a { 0 => 10. * 1.01f64.powi(d as i32), 3 => 10. * 0.99f64.powi(d as i32), _ => 10. }, 4);
        mem[3].1.retain(|r| r.date != spy[83].date);
        let x = Ctx::new(&spy, &spy, &[]);
        let m = Members::new(&x, mem);
        let pl = momentum(&x, &m, 0, n - 1, &Mom { j: 1, k: 1, high52: false, frac: 0., count: 1, entry_days: 0, exit_days: 0, skip_month: true, long_only: false });
        let (t, al, eq, skipped) = simulate5(&m, &pl, &Book5 { sleeves: 1, prev_equity: false, cash_px: None }, 0, n - 1, 0.).unwrap();
        assert!(skipped.is_empty() && !pl.iter().any(|p| p.a == 3 && p.i == 83));
        let (j, tr) = t.iter().enumerate().find(|(_, tr)| tr.legs[0].i == 83).unwrap();
        assert!(tr.reason.starts_with("S00:") && (al[j] - 0.5).abs() < 1e-12);
        assert!((eq[83] - 1.).abs() < 1e-12);
    }
    #[test]
    fn ep5_a_missing_member_bar_is_never_filled() {
        // S00 rises (the winner) but has no bar on bars 83-84: the planned month-end exit (83) and the next formation
        let n = 8 * 28;
        let (spy, mut mem) = basket(n, |a, d| if a == 0 { 10. * 1.01f64.powi(d as i32) } else { 10. + a as f64 }, 10);
        mem[0].1.retain(|r| r.date != spy[83].date && r.date != spy[84].date);
        let x = Ctx::new(&spy, &spy, &[]);
        let m = Members::new(&x, mem);
        assert!(!m.has(0, 83) && m.mark(0, 84) == m.u.c[0][82]);
        let pl = momentum(&x, &m, 0, n - 1, &Mom { j: 1, k: 1, high52: false, frac: 0., count: 1, entry_days: 0, exit_days: 0, skip_month: false, long_only: true });
        assert!(!pl.iter().any(|p| p.a == 0 && p.i == 83)); // not ranked or bought on a day without its own bar
        assert_eq!(pl.iter().find(|p| p.a == 0 && p.i == 55).unwrap().x, 85); // its exit falls on its next real bar
        let (t, _, eq, _) = simulate5(&m, &pl, &Book5 { sleeves: 1, prev_equity: false, cash_px: None }, 0, n - 1, 0.).unwrap();
        every_fill_is_a_real_bar(&m, &t);
        assert!(eq.iter().all(|v| v.is_finite()));
        assert!(eq[82] == eq[83] && eq[83] == eq[84]); // marked at its last real close; the stock due at 83 got no cash
        assert!(t.iter().all(|tr| tr.legs[0].i != 83));
    }
}
