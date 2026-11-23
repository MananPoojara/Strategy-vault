//! Bill Poulos, "ETF Profit Driver" course (Profits Run, ~2008; Articulate Presenter flash modules 1-6 and Bonus). Book id `pfpd`.
//! Research share: Trading Courses,Seminars,Videos/Bill Poulos - ETF Profit Driver Course, $1947 (etfprofitdriver.com)/Module N/data/
//! presentation.xml (the SlideText of modules 2-5 is the rules text; the narration videos are not readable). Lane K3 (23 Nov 2026).
//! Four long-only swing methods on daily bars: Breakaway (M2), Momentum Maximizer (M3), Trend Rebound (M4), Trend Recovery (M5).
//! Module 6 (1-2% risk per trade, at most 10 positions and 12% total risk) is the portfolio layer: not coded.
//! Independent check authors/conv-1123/k3w/pfpd_ref.py.
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "pfpd";
pub const REVIEWED: &[&str] = &["pfpd_breakaway", "pfpd_momentum", "pfpd_rebound", "pfpd_recovery"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { "[Printed rules; unverified] Readings (labelled): (1) the initial stop and the 10% profit-target limit are live from the session after the fill, not on the fill day; (2) when the stop and the target are both reached in one bar the stop is taken first (the target is then cancelled, as the course says); (3) one position per ticker, and a new setup day while an order is pending replaces that order. Conventions (not readings): Wilder ADX(14), Slow Stochastics (8,3,3) with slow %K = 3-bar SMA of the 8-bar fast %K, simple moving averages, 'Average volume of past 50 days >= 300,000 shares' read on the ticker's own share volume, a stop fills at its level or at the open when the day opens through it, a buy limit fills at min(open, limit), a buy stop at max(open, stop), the sell-half-at-10% limit at max(open, target). " }; }

const R_BREAK: &[&str] = &[
    "F|\"Uptrend Setup Conditions: 20 day MA is going up; 20 day MA of 2 days ago < 50 day MA of 2 days ago & 20 day MA of 9 days ago < 50 day MA of 9 days ago; Setup day close > 50 day MA; Setup day high > last Swing High(5); ADX(14) < 25 and ADX of setup day > 1 day ago & ADX of 1 day ago > ADX of 2 days ago & ADX of 2 days ago < ADX of 3 days ago & ADX of 3 days ago < ADX of 4 days ago; Highest high of the past 3 days = highest high of the past 30 days\" (Module 2)",
    "F|\"Volatility Filter 1: (Highest high of the past 3 days - Lowest low of the past 30 days)/Close < 0.15. Volatility Filter 2: (Highest close of the past 5 days - Lowest low of the past 5 days)/Close < 0.08. Average volume of past 50 days >= 300,000 shares\"",
    "E|\"Once a setup day occurs ... buy as follows: Buy at Market\" (next session's open)",
    "X|\"Sell next bar at the last Swing Low(5) - 1/2% of the Swing Low(5) Stop\" (the last swing low(5) that occurred below the 50 day MA; a swing low(5) has 5 higher lows and 5 higher highs on both sides)",
    "X|\"Sell 1/2 position at Entry Price x 1.10 Limit. Once the lowest low of the past 5 days (LL5) > Entry Price ... Sell the next day at the LL5 - 1/2% of the LL5 Stop\"",
];
const R_MOM: &[&str] = &[
    "F|\"20 day MA is going up & 50 MA is going up; 20 day MA > 50 day MA & close > 50 day MA; Highest high of last 15 days = highest high of last 20 days; Lows of 1 to 5 days ago > 20 day MAs of 1 to 5 days ago; Low of setup day < 20 day MA; At least one Slow Stochastics (8,3,3) %K of 2 to 9 days ago > 91\" (Module 3; Volatility Filter: (Highest high of the past 30 days - Lowest low of the past 10 days)/Close < 0.15; average volume of past 50 days >= 300,000 shares)",
    "E|\"Buy at the setup day 20 MA Limit ... If not filled the day after setup day, keep trying for up to 5 days following setup day\"",
    "X|\"Wait until the market closes below the 50 day MA. We will call this closing day. Sell at the closing day Low - 1/2% of the closing day Low Stop\"",
    "X|\"Sell 1/2 position at Entry Price x 1.10 Limit. Once the LL5 > Entry Price ... Sell the next day at the LL5 - 1/2% of the LL5 Stop\"",
];
const R_REB: &[&str] = &[
    "F|\"50 day MA is going up & 50 MA of 9 days ago > 50 MA of 10 days ago; 20 day MA > 50 day MA; Lows of 1 to 5 days ago > 50 day MAs of 1 to 5 days ago and low of today < 50 day MA of today OR Lows of 2 to 5 days ago > 50 day MAs ... and low of 1 day ago < 50 day MA of 1 day ago & low of today < low of 1 day ago & high of today > 50 day MA of today; High of today < high of 1 day ago\" (Module 4; same Volatility Filter and volume filter as Module 3)",
    "E|\"Buy at the setup day high + 1/2% of setup day high Stop ... keep trying for up to 5 days following setup day\"",
    "X|\"Sell at lowest low of the last 5 days (LL5) from and prior to entry day - 1/2% of that lowest low Stop\"",
    "X|\"Sell 1/2 position at Entry Price x 1.10 Limit ... begin moving up the trailing stop even if the trailing stop < entry price ... Sell the next day at the LL6 - 1/2% of the LL6 Stop\"",
];
const R_REC: &[&str] = &[
    "F|\"High < 50 day MA; 50 day MA > 50 day MA of 1 day ago AND 50 day MA > 50 day MA of 10 days ago + 1/2% of that; 50 day MA of 10 days ago > 20 days ago > 30 days ago > 40 days ago; High of today < Highest high of the past 3 days from 1 day ago; Lowest low of the past 4 days = lowest low of the past 10 days\" (Module 5; same Volatility Filter and volume filter as Module 3)",
    "E|\"Buy at the highest high of the last 3 days (HH3) Stop\"",
    "X|\"Sell at lowest low of the last 5 days (LL5) from and prior to entry day Stop\"",
    "X|\"Sell 1/2 position at Entry Price x 1.10 Limit. Once the Lowest low of the past 5 days (LL5) > Entry Price ... Sell the next day at the LL5 - 1/2% of the LL5 Stop\"",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "ETF Profit Driver (Poulos) · ETFs", session: 0, dir: "Long", status: Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("pfpd_breakaway", "ETF Profit Driver - Breakaway Method (upside breakout, swing-low stop, 10% half scale-out, LL5 trail)", R_BREAK,
          concat!(common!(), "(4) Breakaway only: Swing High(5) / Swing Low(5) = a bar whose high and low are both above (below) those of each of the 5 bars on both sides, the swing being known 5 bars later; 'the last swing low below the 50 day MA' = the latest confirmed swing low whose low is under the 50-day MA of its own bar; without one there is no trade. The swing stop is replaced by the LL5 stop once LL5 > entry price.")),
        d("pfpd_momentum", "ETF Profit Driver - Momentum Maximizer (buy the pullback to the 20-day MA, closing-day stop, 10% half scale-out, LL5 trail)", R_MOM,
          concat!(common!(), "(4) Momentum Maximizer only: the 'closing day' is the first close under the 50-day MA from the entry day on; its stop (low - 1/2%) is live from the next session and is replaced by the LL5 stop once LL5 > entry price; no stop exists before a closing day.")),
        d("pfpd_rebound", "ETF Profit Driver - Trend Rebound (bounce off the rising 50-day MA, buy stop above the setup high, LL5 initial / LL6 trailing stop)", R_REB,
          concat!(common!(), "(4) Trend Rebound only: the LL6 trailing stop only moves up ('begin moving up the trailing stop'), starting from the LL5 initial stop.")),
        d("pfpd_recovery", "ETF Profit Driver - Trend Recovery (50-day MA resumes, buy stop at the 3-day high, LL5 initial and trailing stop)", R_REC,
          concat!(common!(), "(4) Trend Recovery only: the entry order is for the next session only (the module gives no 5-day retry); the LL5 stop (no 1/2% offset) is replaced by LL5 - 1/2% once LL5 > entry price.")),
        StrategyDef { id: "pfpd_portfolio", name: "ETF Profit Driver portfolio (risk 1-2% per trade, at most 10 positions, 12% total risk)", group: "ETF Profit Driver (Poulos) · ETFs", session: 0, dir: "Long",
            status: NeedsData("an ETF universe on one equity curve: Module 6 sizes each trade by risk (1-2% of the account over the stop distance), at most 10 positions and 12% total open risk, with the ETF search lists of Modules 2-5; the engine backtests one ticker per run"),
            needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules: &["F|\"Maximum 10 positions; maximum 12% risk\" (Module 6)"], notes: "Portfolio layer of the four methods.", claim: None },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |v| Some(("pfpd", "ETF Profit Driver", v));
    match id { "pfpd_breakaway" => f("Breakaway"), "pfpd_momentum" => f("Momentum Maximizer"), "pfpd_rebound" => f("Trend Rebound"), "pfpd_recovery" => f("Trend Recovery"), "pfpd_portfolio" => f("Portfolio · needs a universe"), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if !reviewed(id) { return None; }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![p("target", "Sell half at entry price x (1 + this %)", 10., 5., 30., 1.), p("vol", "Average 50-day volume at least (shares)", 300_000., 0., 2_000_000., 50_000.),
        p("vfilter", "Volatility filter below", 0.15, 0.05, 0.30, 0.01)])
}

fn swings(h: &[f64], l: &[f64], high: bool) -> Vec<bool> {
    let n = h.len();
    (0..n).map(|j| j >= 5 && j + 5 < n && (j - 5..=j + 5).filter(|&k| k != j).all(|k| if high { h[j] > h[k] && l[j] > l[k] } else { l[j] < l[k] && h[j] < h[k] })).collect()
}
fn mx(v: &[f64], a: usize, b: usize) -> f64 { v[a..=b].iter().copied().fold(f64::MIN, f64::max) }
fn mn(v: &[f64], a: usize, b: usize) -> f64 { v[a..=b].iter().copied().fold(f64::MAX, f64::min) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (o, h, l, c, n) = (&x.o, &x.h, &x.l, &x.c, x.c.len());
    let to = to.min(n - 1);
    let (s20, s50) = (x.sma_n(20), x.sma_n(50));
    let adx = dmi(&x.r, 14).1;
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let v50 = sma(&vol, 50);
    let fk: Vec<f64> = (0..n).map(|i| if i >= 7 { let (a, b) = (mn(l, i - 7, i), mx(h, i - 7, i)); if b > a { 100. * (c[i] - a) / (b - a) } else { f64::NAN } } else { f64::NAN }).collect();
    let sk = sma(&fk, 3);
    let (shs, sls) = (swings(h, l, true), swings(h, l, false));
    let (target, vmin, vf) = (g("target") / 100., g("vol"), g("vfilter"));
    let ok = |v: f64| v == v;
    let vol_ok = |i: usize| ok(v50[i]) && v50[i] >= vmin;
    let vf3010 = |i: usize| (mx(h, i - 29, i) - mn(l, i - 9, i)) / c[i] < vf;
    let last_swing = |sw: &Vec<bool>, i: usize, below50: bool| -> Option<usize> { (5..=i).rev().map(|k| k - 5).find(|&j| sw[j] && (!below50 || l[j] < s50[j])) };
    let setup = |i: usize| -> bool {
        if i < 60 || !vol_ok(i) { return false; }
        match id {
            "pfpd_breakaway" => {
                let Some(sh) = last_swing(&shs, i, false) else { return false };
                s20[i] > s20[i - 1] && s20[i - 2] < s50[i - 2] && s20[i - 9] < s50[i - 9] && c[i] > s50[i] && h[i] > h[sh]
                    && adx[i] < 25. && adx[i] > adx[i - 1] && adx[i - 1] > adx[i - 2] && adx[i - 2] < adx[i - 3] && adx[i - 3] < adx[i - 4]
                    && mx(h, i - 2, i) == mx(h, i - 29, i)
                    && (mx(h, i - 2, i) - mn(l, i - 29, i)) / c[i] < vf && (mx(c, i - 4, i) - mn(l, i - 4, i)) / c[i] < 0.08
            }
            "pfpd_momentum" => {
                s20[i] > s20[i - 1] && s50[i] > s50[i - 1] && s20[i] > s50[i] && c[i] > s50[i] && mx(h, i - 14, i) == mx(h, i - 19, i)
                    && (1..=5).all(|k| l[i - k] > s20[i - k]) && l[i] < s20[i] && (2..=9).any(|k| sk[i - k] > 91.) && vf3010(i)
            }
            "pfpd_rebound" => {
                let a = (1..=5).all(|k| l[i - k] > s50[i - k]) && l[i] < s50[i];
                let b = (2..=5).all(|k| l[i - k] > s50[i - k]) && l[i - 1] < s50[i - 1] && l[i] < l[i - 1] && h[i] > s50[i];
                s50[i] > s50[i - 1] && s50[i - 9] > s50[i - 10] && s20[i] > s50[i] && (a || b) && vf3010(i) && h[i] < h[i - 1]
            }
            _ => {
                h[i] < s50[i] && s50[i] > s50[i - 1] && s50[i] > s50[i - 10] * 1.005 && s50[i - 10] > s50[i - 20] && s50[i - 20] > s50[i - 30] && s50[i - 30] > s50[i - 40]
                    && h[i] < mx(h, i - 3, i - 1) && mn(l, i - 3, i) == mn(l, i - 9, i) && vf3010(i)
            }
        }
    };
    let ll = |i: usize, k: usize| mn(l, i + 1 - k, i);
    let mut out: Vec<Trade> = vec![];
    // pending order: (level, last day it may fill, setup bar); position: (entry bar, entry px, fraction left, stop, closing-day stop, target taken, exits)
    let mut pend: Option<(f64, usize)> = None;
    struct P { e: usize, px: f64, left: f64, stop: f64, cstop: f64, half: bool, ex: Vec<(usize, f64, f64)> }
    let mut pos: Option<P> = None;
    for i in from.max(60)..=to {
        if let Some(q) = pos.as_mut() {
            if i > q.e {
                let tgt = q.px * (1. + target);
                let stop = if ok(q.stop) { q.stop } else { q.cstop };
                if ok(stop) && l[i] <= stop {
                    let fill = o[i].min(stop);
                    q.ex.push((i, fill, q.left)); q.left = 0.;
                } else if !q.half && h[i] >= tgt {
                    q.ex.push((i, o[i].max(tgt), 0.5)); q.left -= 0.5; q.half = true;
                }
            }
            if q.left > 0. {
                // orders for the next session, set at the close of bar i
                let t = i;
                let trail = |k: usize| ll(t, k) * 0.995;
                match id {
                    "pfpd_breakaway" => { q.stop = if ll(t, 5) > q.px { trail(5) } else { last_swing(&sls, t, true).map_or(f64::NAN, |j| l[j] * 0.995) }; }
                    "pfpd_momentum" => {
                        if !ok(q.cstop) && c[t] < s50[t] { q.cstop = l[t] * 0.995; }
                        q.stop = if ll(t, 5) > q.px { trail(5) } else { f64::NAN };
                    }
                    "pfpd_rebound" => { let v = trail(6); q.stop = if ok(q.stop) { q.stop.max(v) } else { v }; }
                    _ => { q.stop = if ll(t, 5) > q.px { trail(5) } else { ll(q.e, 5) }; }
                }
            }
            if q.left <= 0. || i == to && false {
                let q = pos.take().unwrap();
                let fin = *q.ex.last().unwrap();
                let w: f64 = q.ex.iter().map(|e| e.1 * e.2).sum();
                out.push(Trade { dir: 1., legs: vec![Leg { i: q.e, px: q.px, w: 1. }], exit_i: fin.0, exit_px: w, reason: "stop".into(), entry_kind: "limit", signal_i: fin.0, open: false, exits: q.ex });
            }
            if pos.is_some() { continue; }
        }
        if pos.is_none() {
            if let Some((lvl, last)) = pend {
                let (fill, hit) = match id {
                    "pfpd_momentum" => (o[i].min(lvl), l[i] <= lvl),
                    _ if id == "pfpd_breakaway" => (o[i], true),
                    _ => (o[i].max(lvl), h[i] >= lvl),
                };
                if hit {
                    pend = None;
                    let mut q = P { e: i, px: fill, left: 1., stop: f64::NAN, cstop: f64::NAN, half: false, ex: vec![] };
                    // the initial stop is placed at the fill and is live from the next session
                    match id {
                        "pfpd_rebound" | "pfpd_recovery" => q.stop = if id == "pfpd_rebound" { ll(i, 5) * 0.995 } else { ll(i, 5) },
                        "pfpd_breakaway" => q.stop = last_swing(&sls, i, true).map_or(f64::NAN, |j| l[j] * 0.995),
                        _ => {}
                    }
                    if id == "pfpd_momentum" && c[i] < s50[i] { q.cstop = l[i] * 0.995; }
                    if id == "pfpd_breakaway" && !ok(q.stop) { /* no swing low below the 50-day MA: no stop (labelled) */ }
                    // fill-day close may already move the trailing stops
                    let t = i;
                    match id {
                        "pfpd_breakaway" => { q.stop = if ll(t, 5) > q.px { ll(t, 5) * 0.995 } else { q.stop }; }
                        "pfpd_momentum" => { q.stop = if ll(t, 5) > q.px { ll(t, 5) * 0.995 } else { f64::NAN }; }
                        "pfpd_rebound" => { q.stop = q.stop.max(ll(t, 6) * 0.995); }
                        _ => { q.stop = if ll(t, 5) > q.px { ll(t, 5) * 0.995 } else { q.stop }; }
                    }
                    pos = Some(q);
                    continue;
                } else if i >= last { pend = None; }
            }
            if i < to && setup(i) {
                pend = match id {
                    "pfpd_breakaway" => Some((f64::NAN, i + 1)),
                    "pfpd_momentum" => Some((s20[i], i + 5)),
                    "pfpd_rebound" => Some((h[i] * 1.005, i + 5)),
                    _ => Some((mx(h, i - 2, i), i + 1)),
                };
            }
        }
    }
    if let Some(q) = pos {
        let mut ex = q.ex.clone();
        ex.push((to, c[to], q.left));
        let w: f64 = ex.iter().map(|e| e.1 * e.2).sum();
        out.push(Trade { dir: 1., legs: vec![Leg { i: q.e, px: q.px, w: 1. }], exit_i: to, exit_px: w, reason: "open".into(), entry_kind: "limit", signal_i: to, open: true, exits: ex });
    }
    Some(out)
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "The course teaches these methods on exchange-traded funds; they run here only on the six US ETFs the independent reference covers (SPY, QQQ, XLE, EWJ, IWM, XLK), a declared application of the course's ETF universe, with the course's 300,000-share average-volume filter read on the ticker's own volume. Other ETFs (including Indian ETFs), indices and stocks are not admitted.",
    basis: "Bill Poulos, ETF Profit Driver, Modules 2-5 (presentation.xml slide text); independent check authors/conv-1123/k3w/pfpd_ref.py on SPY, QQQ, XLE, EWJ, IWM, XLK",
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn needs_ohlc_and_runs() {
        let r: Vec<crate::Row> = (0..400).map(|i| { let p = 50. + (i as f64 * 0.37).sin() * 5. + i as f64 * 0.05; crate::Row { ticker: "T".into(), date: format!("d{:04}", i), open: p, high: p * 1.01, low: p * 0.99, close: p * 1.002, volume: Some(1e6) } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED { assert!(run(id, &x, 0, 399, &Params::new()).is_some()); }
        assert!(run("pfpd_portfolio", &x, 0, 399, &Params::new()).is_none());
    }
    /// hand-run dump for the pandas reference: PFPD_CSV=<us_data.csv> PFPD_OUT=<file> cargo test --release pfpd_dump -- --ignored
    #[test]
    #[ignore]
    fn pfpd_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("PFPD_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("PFPD_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,entry_date,entry_px,exits,open").unwrap();
        for tk in ["SPY", "QQQ", "XLE", "EWJ", "IWM", "XLK"] {
            let rows = &data[tk];
            let x = Ctx::new(rows, &[], &[]);
            for id in REVIEWED {
                for z in run(id, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                    let ex: Vec<String> = z.exits.iter().map(|e| format!("{}@{:.10}x{}", rows[e.0].date, e.1, e.2)).collect();
                    writeln!(f, "{id},{tk},{},{:.10},{},{}", rows[z.legs[0].i].date, z.legs[0].px, ex.join(";"), z.open).unwrap();
                }
            }
        }
    }
}
