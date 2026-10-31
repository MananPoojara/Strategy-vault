//! Ed Ponsi: "Forex Patterns & Probabilities" (Wiley 2007; book id `epfpp`) and "The Ed Ponsi Forex Playbook" (Wiley 2010).
//! Reading: claude-continuation-20261020/authors/ponsi (text + admission/ponsi_ref.py). Live: the FX-Ed Trend Technique (Ch.9,
//! p.99-130): 10/20/50/200 EMAs in order, price held on one side of the 10-day EMA for 10 candles, entry when price comes back to
//! the 10-day EMA, stop 50% of the 14-day ATR beyond the EMA, raised after every close. The other chapters are judgement
//! (Fibonacci/trend-line plans), intraday pip rules, carry/interest-rate or central-bank trades and are held.
//! Book: currency pairs; indices, ETFs and stocks are a declared application.
use crate::connors::{atr, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/ponsi/admission/ponsi_ref.py (header)).
pub const REVIEWED: &[&str] = &["epfpp_fxed_trend"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const FXED_NOTE: &str = "[Declared readings] All four averages are exponential averages of closes (the book's figures say 'Proper order of EMAs'), seeded with the simple average of the first n closes. Trend and the 10-candle filter are judged at the previous close: the last 10 closes all above (short: below) the 10-day EMA and 10 > 20 > 50 > 200 (short: reversed). The touch level is the previous close's 10-day EMA: buy at that level when the day's low reaches it, or at the open when the day opens below it. Initial stop = that EMA - 0.5 x the 14-day Wilder ATR of the previous close (no trade when the entry is at or beyond it); the stop is live on the entry day. From the second day after entry the stop is raised to the latest close's EMA - 0.5 x ATR and never lowered (p.112: no raise on day 2). Stops fill at the level or a worse open. Optional partial exits (p.125-127) are not applied. One position at a time; a new entry can come from the bar after an exit.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, book, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes, claim: None };
    let nt = |id, name, group, book, why, rules| d(id, name, group, book, NotTestable(why), rules, "Held after the 2026-10-27 reading (authors/ponsi).");
    vec![
        d("epfpp_fxed_trend", "FX-Ed Trend Technique (10-day EMA pullback in 10/20/50/200 order)", "Ch.9 FX-Ed Trend Technique (p.99-130)", "epfpp", Live,
          &["F|\"1. The 10-day moving average is above the 20-day moving average. 2. The 20-day moving average is above the 50-day moving average. 3. The 50-day moving average is above the 200-day moving average.\" (p.103)",
            "F|\"the exchange rate must be above the 10-day exponential moving average for a period of at least 10 or more candles in the case of an uptrend\" (p.105)",
            "E|\"As the exchange rate falls to the 10-day EMA, we'll enter a long position ... there is no need to wait for the candle to close\" (p.110)",
            "X|\"we will place the stop under the 10-day exponential moving average by an amount of pips equivalent to 50 percent of the daily ATR\" (ATR 14); \"We will raise the stop every time a daily candle closes ... we will not lower the stop under any circumstances\" (p.110-112)"], FXED_NOTE),
        nt("epfpp_mtf_fib", "Forex multiple time frame strategy (Fibonacci + RSI)", "Ch.8 (p.69-97)", "epfpp", "the Fibonacci levels come from hand-picked swings, the stop goes 'above the recent high' and the Fibonacci level, and exits are in thirds at chosen levels",
           &["E|\"we enter short in the vicinity of 1.1225, the point at which the RSI is no longer giving an overbought reading\""]),
        nt("epfpp_patterns", "Intraday triangle breakouts, flags and pennants, the squeeze play", "Ch.11-13 (p.145-180)", "epfpp", "patterns and trend lines are drawn by hand, targets use support/resistance, round numbers and Fibonacci levels, and some setups are intraday",
           &["E|\"Trend lines are then added to create entry points, and to determine the location of stops\" (p.178)"]),
        nt("epfpp_round_trip", "The round trip (round-number bounce with a moving average)", "Ch.14 (p.181-191)", "epfpp", "round numbers ('00' levels) are currency-quote levels in pips; no equivalent for indices, ETFs or stocks", &[]),
        nt("epfpp_boomerang", "The boomerang (EUR/USD 17:00 fade)", "Ch.16 (p.201-205)", "epfpp", "5-minute EUR/USD orders 15 pips either side of the 17:00 New York price with 15-pip stops", &[]),
        nt("epfpp_carry", "Interest-rate edge / carry, central-bank intervention trades", "Ch.15; Playbook Ch.9-12", "epfpp", "needs interest-rate differentials and central-bank announcements; the Playbook's FXEd carry trade repeats the Ch.9 technique", &[]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "epfpp_fxed_trend" => ("epfpp_fxed_trend", "Ponsi FX-Ed trend", "10 EMA / 0.5 ATR"),
        _ if id.starts_with("epfpp_") => ("epfpp_nt", "Ponsi (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "epfpp_fxed_trend" => vec![p("mult", "Stop distance (x 14-day ATR)", 0.5, 0.1, 3., 0.1)],
        _ if id.starts_with("epfpp_") => vec![],
        _ => return None,
    })
}

fn ema_sma_seed(c: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; c.len()];
    if c.len() < n { return out; }
    let a = 2. / (n as f64 + 1.);
    let mut v = c[..n].iter().sum::<f64>() / n as f64;
    out[n - 1] = v;
    for i in n..c.len() { v = a * c[i] + (1. - a) * v; out[i] = v; }
    out
}

fn run_fxed(x: &Ctx, from: usize, to: usize, mult: f64) -> Vec<Trade> {
    let e: Vec<Vec<f64>> = [10, 20, 50, 200].iter().map(|&n| ema_sma_seed(&x.c, n)).collect();
    let a = atr(x.r, 14);
    let trend = |j: usize| -> f64 {
        if !(0..4).all(|k| e[k][j].is_finite()) { return 0.; }
        if (0..3).all(|k| e[k][j] > e[k + 1][j]) { 1. } else if (0..3).all(|k| e[k][j] < e[k + 1][j]) { -1. } else { 0. }
    };
    let mk = |d: f64, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "limit", signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64, f64)>); // dir, entry bar, entry price, stop
    for i in from.max(11)..=to {
        let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
        if let Some((d, ei, ep, mut s)) = cur {
            if i > ei {
                if i >= ei + 2 {
                    let ns = e[0][i - 1] - d * mult * a[i - 1];
                    s = if d > 0. { s.max(ns) } else { s.min(ns) };
                }
                cur = Some((d, ei, ep, s));
                if (d > 0. && o <= s) || (d < 0. && o >= s) { out.push(mk(d, ei, ep, i, o, "trailing stop", false)); cur = None; }
                else if (d > 0. && l <= s) || (d < 0. && h >= s) { out.push(mk(d, ei, ep, i, s, "trailing stop", false)); cur = None; }
                continue;
            }
        }
        if cur.is_some() || i >= to || !a[i - 1].is_finite() { continue; }
        let d = trend(i - 1);
        if d == 0. { continue; }
        if !(i - 10..i).all(|j| e[0][j].is_finite() && (if d > 0. { x.c[j] > e[0][j] } else { x.c[j] < e[0][j] })) { continue; }
        let lvl = e[0][i - 1];
        if !(if d > 0. { l <= lvl } else { h >= lvl }) { continue; }
        let ep = if (d > 0. && o < lvl) || (d < 0. && o > lvl) { o } else { lvl };
        let s = lvl - d * mult * a[i - 1];
        if (d > 0. && ep <= s) || (d < 0. && ep >= s) { continue; }
        if (d > 0. && l <= s) || (d < 0. && h >= s) { out.push(mk(d, i, ep, i, s, "trailing stop", false)); } else { cur = Some((d, i, ep, s)); }
    }
    if let Some((d, ei, ep, _)) = cur { out.push(mk(d, ei, ep, to, x.c[to], "open", true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    Some(match id {
        "epfpp_fxed_trend" => run_fxed(x, from, to, g("mult")),
        _ => return None,
    })
}

// Instrument admission: Ed Ponsi 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/ponsi/admission/ponsi_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="epfpp" && crate::books::epfpp::reviewed(def.id),
    label: "Ed Ponsi wrote the FX-Ed Trend Technique for currency pairs; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical.",
    basis: "Ed Ponsi, Forex Patterns & Probabilities (Wiley 2007), Ch.9 The FX-Ed Trend Technique (p.99-130)",
};
