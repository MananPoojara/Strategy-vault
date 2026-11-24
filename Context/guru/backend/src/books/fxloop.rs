//! "The Forex Loophole" (forexloophole.com, thefxloop.pdf, "Forex Loop Hole" folder, Trading Knowledge 3). Book id `fxloop`.
//! Rule (pp.11-19): each night at the close look for a short set-up (today's close above last Friday's close, above yesterday's
//! close, and yesterday's close above the day before's; the long set-up is the mirror); next day a stop order 7 pips beyond today's
//! close (valid that day only; no Monday entry if the market gaps beyond the entry); stop and target are fixed pips per pair
//! (USD/CAD 60 pip target, 40 pip stop); open trades are closed before the Friday close.
//! Lane I2 (25 Nov 2026, R4 #401): the book trades the USD/CAD spot pair; the vault holds Norgate Canadian dollar futures (&6C,
//! USD per CAD), so the rule runs on the EXACTLY INVERTED series (close' = 1/close, high' = 1/low, low' = 1/high, open' = 1/open;
//! a long USD/CAD trade is a short &6C trade). Independent check authors/conv-1123/I2w/fxloop_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fxloop";
const EPS: f64 = 1e-9;
/// 1 pip of the USD/CAD quote (#380: the book's own pip scale; the inverted series is a USD/CAD quote).
const PIP: f64 = 0.0001;

const NOTE: &str = "[Declared adaptation, R4 #401/#380] The book trades spot USD/CAD; no spot forex is held, so the rule runs on Norgate Canadian dollar continuous futures (&6C, US$ per CAD, ends 2020-09-11, back-adjusted series for signals and levels, % per trade on the unadjusted entry price) turned into a USD/CAD quote by exact inversion (open' = 1/open, high' = 1/low, low' = 1/high, close' = 1/close); a long USD/CAD trade is a short &6C trade and conversely. One pip = 0.0001 of the inverted quote; the book's days are 00:00 GMT days, here Norgate daily sessions. Unstated choices, one each (R4 #400): (1) 'last Friday's close' = the close of the last session of the previous Monday-Friday week, also when today is a Friday or that Friday is a holiday; the Friday exit = the last session of the week; (2) a stop order that the open gaps through fills at the open (Tuesday-Friday; on Mondays the book skips such a trade); (3) when the stop, the target and the entry can all be touched inside one bar, the bar path is open, the nearer of high/low, the other extreme, close (no minute data exists, #240; the share of trades that change with the other order is stated in the gate row); (4) a new set-up while a trade is open is not traded (one position at a time). The stop and target stay at the prices of the stop order (entry price -/+ pips), as the book's pending order carries them, also when the fill is at a gap open. Same near-twins: fxfs_weekly (FirstStrike weekly OCO), jkbwts_one_night_stand (Friday stop entry). No results are printed in the book (one worked EUR/USD example, 15 May 2008); UNVERIFIED.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "fxloop_loophole", name: "Forex Loophole: three-close exhaustion fade, 7-pip stop entry, 60/40-pip target/stop (USD/CAD via inverted Canadian dollar futures)",
        group: "Forex Loophole", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "fxloop",
        rules: &[
            "E|Short set-up (p.11): today's close > last Friday's close, > yesterday's close, and yesterday's close > the day before's; long set-up = the mirror (p.13)",
            "E|Next day only: buy stop (long) / sell stop (short) 7 pips beyond today's close (p.16); no Monday entry if the market has gapped beyond the entry price (p.15)",
            "X|Target 60 pips and stop 40 pips from the entry price (USD/CAD, p.19)",
            "X|Flat before the Friday close (p.15)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "fxloop_loophole" => Some(("fxloop", "Forex Loophole", "entry 7 pips, target 60, stop 40")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "fxloop_loophole" => Some(vec![
            p("entry", "Stop-order distance beyond the close, pips (p.16: 7)", 7.0, 1.0, 50.0, 1.0),
            p("tp", "Profit target, pips (USD/CAD, p.19: 60)", 60.0, 5.0, 300.0, 5.0),
            p("sl", "Stop loss, pips (USD/CAD, p.19: 40)", 40.0, 5.0, 200.0, 5.0)]),
        _ => None,
    }
}

fn days(d: &str) -> Option<i64> {
    let v: Vec<i64> = d.split('-').map(|s| s.parse().ok()).collect::<Option<_>>()?;
    if v.len() != 3 { return None; }
    let (mut y, m, dd) = (v[0], v[1], v[2]);
    if m <= 2 { y -= 1; }
    let era = y.div_euclid(400); let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + dd - 1;
    Some(era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468)
}

/// First (distance along the bar path, fill price) at which `px` is reached at or after `t0`.
/// Path: open -> nearer of high/low -> the other -> close. `rising`: fills when price trades up to px.
fn touch(o: f64, h: f64, l: f64, c: f64, px: f64, rising: bool, t0: f64) -> Option<(f64, f64)> {
    if t0 <= 0. && ((rising && o >= px) || (!rising && o <= px)) { return Some((0., o)); }
    let pts = if h - o <= o - l { [o, h, l, c] } else { [o, l, h, c] };
    let mut t = 0.;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a.min(b) - EPS <= px && px <= a.max(b) + EPS {
            let at = t + (px - a).abs();
            if at >= t0 - EPS { return Some((at, px)); }
        }
        t += (b - a).abs();
    }
    None
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "fxloop_loophole" { return None; }
    if crate::futures::ctx_market(x).map(|m| m.sym) != Some("&6C") { return Some(vec![]); }
    if from >= to || to >= x.r.len() { return Some(vec![]); }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (ent, tp, sl) = (g("entry") * PIP, g("tp") * PIP, g("sl") * PIP);
    let n = x.r.len();
    if x.o.iter().chain(&x.h).chain(&x.l).chain(&x.c).any(|v| !(*v > 0.)) { return Some(vec![]); }
    let (o, h, l, c): (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) = (
        x.o.iter().map(|v| 1.0 / v).collect(), x.l.iter().map(|v| 1.0 / v).collect(),
        x.h.iter().map(|v| 1.0 / v).collect(), x.c.iter().map(|v| 1.0 / v).collect());
    let day: Vec<i64> = x.r.iter().map(|r| days(&r.date).unwrap_or(-1)).collect();
    let wk: Vec<i64> = day.iter().map(|d| (d + 3).div_euclid(7)).collect(); // Monday-based week number
    let wd: Vec<i64> = day.iter().map(|d| (d + 3).rem_euclid(7)).collect(); // Monday = 0
    // last session of each calendar week
    let last_of_week = |k: usize| k + 1 >= n || wk[k + 1] != wk[k];
    let mut out: Vec<Trade> = vec![];
    let mut busy_until: Option<usize> = None; // bar index at which the open trade closes
    let mut i = from.max(2);
    while i < to {
        if busy_until.is_some_and(|b| b > i) { i += 1; continue; }
        // last session of the previous week
        let mut j = i; while j > 0 && wk[j] == wk[i] { j -= 1; }
        if wk[j] != wk[i] - 1 { i += 1; continue; }
        let wc = c[j];
        let dir_inv: f64 = if c[i] > wc && c[i] > c[i - 1] && c[i - 1] > c[i - 2] { -1.0 } else if c[i] < wc && c[i] < c[i - 1] && c[i - 1] < c[i - 2] { 1.0 } else { i += 1; continue };
        let m = i + 1; // entry day
        let entry = if dir_inv > 0.0 { c[i] + ent } else { c[i] - ent };
        if wd[m] == 0 && ((dir_inv > 0.0 && o[m] > entry) || (dir_inv < 0.0 && o[m] < entry)) { i += 1; continue; }
        let rising = dir_inv > 0.0;
        let Some((te, fill)) = touch(o[m], h[m], l[m], c[m], entry, rising, 0.) else { i += 1; continue };
        let (stop, tgt) = (entry - dir_inv * sl, entry + dir_inv * tp);
        // entry day, then later days: stop (against) and target (with) by bar path; flat at the last session of the week
        let mut res: Option<(usize, f64, &'static str)> = None;
        let mut k = m;
        while k <= to {
            let t0 = if k == m { te } else { 0. };
            let ts = touch(o[k], h[k], l[k], c[k], stop, dir_inv < 0.0, t0);
            let tt = touch(o[k], h[k], l[k], c[k], tgt, dir_inv > 0.0, t0);
            match (ts, tt) {
                (Some(a), Some(b)) => res = Some(if a.0 <= b.0 { (k, a.1, "Stop") } else { (k, b.1, "Target") }),
                (Some(a), None) => res = Some((k, a.1, "Stop")),
                (None, Some(b)) => res = Some((k, b.1, "Target")),
                _ => {}
            }
            if res.is_some() { break; }
            if last_of_week(k) { res = Some((k, c[k], "Friday close")); break; }
            k += 1;
        }
        let dir = -dir_inv; // contract direction (inverted series)
        match res {
            Some((xi, xp, why)) => {
                out.push(Trade { dir, legs: vec![Leg { i: m, px: 1.0 / fill, w: 1. }], exit_i: xi, exit_px: 1.0 / xp, reason: why.into(),
                    entry_kind: if te == 0. { "open" } else { "stop" }, signal_i: i, open: false, exits: vec![] });
                busy_until = Some(xi);
            }
            None => {
                out.push(Trade { dir, legs: vec![Leg { i: m, px: 1.0 / fill, w: 1. }], exit_i: to, exit_px: x.c[to], reason: "open".into(),
                    entry_kind: if te == 0. { "open" } else { "stop" }, signal_i: i, open: true, exits: vec![] });
                break;
            }
        }
        i += 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn norgate() -> Option<Vec<Row>> {
        let dir = std::env::var("NORGATE_DIR").unwrap_or("/home/server/.cache/strategy-vault-webapp/data/futures/norgate-20200908".into());
        let txt = std::fs::read_to_string(format!("{dir}/&6C_CCB.csv")).ok()?;
        let m = crate::futures::MARKETS.iter().find(|m| m.sym == "&6C")?;
        Some(txt.lines().skip(1).filter_map(|l| {
            let f: Vec<&str> = l.split(',').map(|s| s.trim_matches('"')).collect();
            let n = |k: usize| f[k].parse::<f64>().unwrap();
            Some(Row { ticker: crate::futures::ticker(m), date: f[0].into(), open: n(3), high: n(4), low: n(5), close: n(6), volume: None })
        }).collect())
    }

    /// authors/conv-1123/I2w/fxloop_ref.py compares its trades with this dump (FXLOOP_DUMP = output directory).
    #[test]
    fn dump_trades() {
        let Ok(dir) = std::env::var("FXLOOP_DUMP") else { return };
        let Some(r) = norgate() else { return };
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("fxloop_loophole", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let s: String = ts.iter().map(|t| format!("{},{},{:.6},{},{:.6},{},{}\n", t.dir, r[t.legs[0].i].date, t.legs[0].px, r[t.exit_i].date, t.exit_px, t.reason, t.open)).collect();
        std::fs::write(format!("{dir}/fxloop_trades.csv"), s).unwrap();
    }

    #[test]
    fn runs_only_on_the_canadian_dollar_contract() {
        let r: Vec<Row> = (0..30).map(|i| Row { ticker: "SPY".into(), date: format!("2020-01-{:02}", i + 1), open: 100., high: 101., low: 99., close: 100., volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("fxloop_loophole", &x, 0, r.len() - 1, &Params::new()).unwrap().is_empty());
    }
}
