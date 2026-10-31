//! John Forman, "The Essentials of Trading: From the Basics to Building a Winning Strategy" (Wiley, 2006; book id
//! `jfeot`; printed pages, PDF = printed + 21). Reading: claude-continuation-20261020/authors/forman (admission/eot_ref.py).
//! Live: the four Ch.7 test systems the book builds on daily USD/JPY 1999-2004 (Test I SMA entry / SMA exit, Test II
//! Parabolic SAR only, Test III SMA entry / SAR exit, Test IV SAR entry / SMA exit, p.173-198) and the Ch.8 GBP/USD
//! 5-period fast stochastic 40/60 system (p.216-217). Not coded: the 10-period always-in sample (p.165-167) = the same
//! calculation as rwmts_sma26_sar with 10 days; the 350/400-pip MAE stop (p.224, pips on GBP/USD) and the 10-day MA with
//! a 100-pip stop (p.225, a warning example) are fixed FX amounts; position sizing (Ch.8) is money management.
//! Book: USD/JPY and GBP/USD spot FX (not held); indices, ETFs and US/India stocks here are a declared application.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/forman/admission/eot_ref.py (header)).
pub const REVIEWED: &[&str] = &["jfeot_sma_cross", "jfeot_sar", "jfeot_sma_sar", "jfeot_sar_sma", "jfeot_stoch"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
use crate::books::mkits::sar;

const NOTE: &str = "[Declared readings] Signals use the daily close. A cross needs yesterday on or beyond the line and today strictly past it (\"the cross will occur once in a given direction\", p.195); SAR \"moves below the close\" = SAR below today's close after being on or above yesterday's close. Parabolic SAR = Wilder's (the book's formula, p.184): it starts long on the first bar of the history, never inside the two prior bars' range, and on a flip restarts at the extreme point; computed from all chart history. Entries supersede exits (p.195): an opposite entry closes and reverses the position; an exit fires only when no entry in the held direction fires on the same bar; after an exit the position is flat until the next entry signal. Fill: \"executing trades at the close of the day, or the open of the next trading session\" (p.186-187) - default the signal day's close, the next open is an option. The book's tests are points only on spot FX with no costs.";

const G: &str = "Forman Ch.7-8 test systems";
fn def(id: &'static str, name: &'static str, rules: &'static [&'static str]) -> StrategyDef {
    StrategyDef { id, name, group: G, session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: "jfeot", rules, notes: NOTE, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def("jfeot_sma_cross", "Test I: close crosses the 92-day average to enter, the 50-day average to exit", &[
            "E|\"Go Long when Close crosses above the 92-day moving average ... Go Short when Close crosses below the 92-day moving average\" (p.178)",
            "X|\"Exit Long when Close crosses below the 50-day moving average ... Exit Short when Close crosses above the 50-day moving average\" (p.178); an opposite entry also closes the trade (p.195)",
            "F|Daily USD/JPY 1999-2004 in the book (p.173-174); set the exit average equal to the entry average for a plain stop and reverse (option)"]),
        def("jfeot_sar", "Test II: Parabolic SAR 0.02/0.20 stop and reverse on the close", &[
            "E|\"Exit Shorts and Go Long when the 0.02/0.20 setting SAR line moves below the close\" (p.186)",
            "X|\"Exit Longs and Go Short when the 0.02/0.20 setting SAR line moves above the close\" (p.186) - always in"]),
        def("jfeot_sma_sar", "Test III: close crosses the 20-day average to enter, Parabolic SAR 0.09/0.50 to exit", &[
            "E|\"Go Long when Close crosses above the 20-day moving average ... Go Short when Close crosses below the 20-day moving average\" (p.192)",
            "X|\"Exit Long when the 0.9/0.50 setting SAR line moves above Close ... Exit Short when the 0.9/0.50 setting SAR line moves below Close\" (p.192; read as step 0.09 - the tested steps were 0.01-0.10 and the text names \"the 0.09 step and 0.50 cap\", p.191)",
            "F|First combination tried: 92-day average with SAR 0.02/0.20 (p.189, Table 7.7) - set as parameters"]),
        def("jfeot_sar_sma", "Test IV: Parabolic SAR 0.02/0.20 to enter, close crosses the 60-day average to exit", &[
            "E|\"Go Long when 0.02/0.20 setting SAR line moves below Close ... Go Short when 0.02/0.20 setting SAR line moves above Close\" (p.196-197)",
            "X|\"Exit Long when Close crosses below the 60-day moving average ... Exit Short when Close crosses above the 60-day moving average\" (p.196-197); the book's chosen system (p.211, p.234-235)"]),
        def("jfeot_stoch", "Ch.8 GBP/USD system: 5-period fast stochastic %K through 40 / 60", &[
            "F|%K = (close - lowest low of n) / (highest high of n - lowest low of n) x 100, n = 5 (p.217)",
            "E|\"Go Long when 5-period Fast Stochastic %K rises above 40 ... Go Short when 5-period Fast Stochastic %K falls below 60\" (p.217)",
            "X|\"Exit Long when 5-period Fast Stochastic %K falls below 40 ... Exit Short when 5-period Fast Stochastic %K rises above 60\" (p.217)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id {
        "jfeot_sma_cross" => "Test I SMA 92 / 50", "jfeot_sar" => "Test II SAR 0.02/0.20", "jfeot_sma_sar" => "Test III SMA 20 / SAR 0.09/0.50",
        "jfeot_sar_sma" => "Test IV SAR 0.02/0.20 / SMA 60", "jfeot_stoch" => "Stochastic 5 40/60", _ => return None,
    };
    Some(("jfeot_tests", "Forman test systems", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let fill = p("fill", "Fill: 0 = signal day's close (default), 1 = next open (book allows either, p.187)", 0., 0., 1., 1.);
    let step = |d| p("step", "SAR acceleration step", d, 0.01, 0.2, 0.01);
    let cap = |d| p("cap", "SAR acceleration cap", d, 0.05, 1.0, 0.05);
    Some(match id {
        "jfeot_sma_cross" => vec![p("entry", "Entry average days (book 92)", 92., 2., 300., 1.), p("exit", "Exit average days (book 50)", 50., 2., 300., 1.), fill],
        "jfeot_sar" => vec![step(0.02), cap(0.2), fill],
        "jfeot_sma_sar" => vec![p("ma", "Entry average days (book 20)", 20., 2., 300., 1.), step(0.09), cap(0.5), fill],
        "jfeot_sar_sma" => vec![step(0.02), cap(0.2), p("ma", "Exit average days (book 60)", 60., 2., 300., 1.), fill],
        "jfeot_stoch" => vec![p("n", "%K bars (book 5)", 5., 2., 50., 1.), p("lo", "Long level (book 40)", 40., 1., 99., 1.), p("hi", "Short level (book 60)", 60., 1., 99., 1.), fill],
        _ => return None,
    })
}

/// a crosses above b on bar t (strict today, on-or-below yesterday); NaN = no cross.
fn up(a: &[f64], b: &[f64], t: usize) -> bool { t > 0 && a[t] > b[t] && a[t - 1] <= b[t - 1] }
fn dn(a: &[f64], b: &[f64], t: usize) -> bool { t > 0 && a[t] < b[t] && a[t - 1] >= b[t - 1] }

/// Signals per bar: (long entry, short entry, long exit, short exit). Entries supersede exits (p.195).
fn simulate(x: &Ctx, from: usize, to: usize, next_open: bool, sig: impl Fn(usize) -> (bool, bool, bool, bool)) -> Vec<Trade> {
    let mk = |d: f64, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool, si: usize| Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }],
        exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: if next_open { "open" } else { "close" }, signal_i: si, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>);
    for t in from..to {
        let (le, se, lx, sx) = sig(t);
        let (f, px) = if next_open { (t + 1, x.o[t + 1]) } else { (t, x.c[t]) };
        if let Some((d, ei, ep)) = cur {
            let (rev, ex) = if d > 0. { (se, lx && !le) } else { (le, sx && !se) };
            if rev || ex { out.push(mk(d, ei, ep, f, px, if rev { "reverse" } else { "exit" }, false, t)); cur = None; }
        }
        if cur.is_none() {
            if le { cur = Some((1., f, px)); } else if se { cur = Some((-1., f, px)); }
        }
    }
    if let Some((d, ei, ep)) = cur { out.push(mk(d, ei, ep, to, x.c[to], "open", true, to)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.) as usize;
    let nx = g("fill").round() >= 1.;
    let c = &x.c;
    Some(match id {
        "jfeot_sma_cross" => {
            let (me, mx) = (sma(c, u("entry")), sma(c, u("exit")));
            simulate(x, from, to, nx, |t| (up(c, &me, t), dn(c, &me, t), dn(c, &mx, t), up(c, &mx, t)))
        }
        "jfeot_sar" => {
            let s = sar(&x.h, &x.l, g("step"), g("cap"));
            simulate(x, from, to, nx, |t| { let (l, sh) = (up(c, &s, t), dn(c, &s, t)); (l, sh, sh, l) })
        }
        "jfeot_sma_sar" => {
            let (m, s) = (sma(c, u("ma")), sar(&x.h, &x.l, g("step"), g("cap")));
            simulate(x, from, to, nx, |t| (up(c, &m, t), dn(c, &m, t), dn(c, &s, t), up(c, &s, t)))
        }
        "jfeot_sar_sma" => {
            let (m, s) = (sma(c, u("ma")), sar(&x.h, &x.l, g("step"), g("cap")));
            simulate(x, from, to, nx, |t| (up(c, &s, t), dn(c, &s, t), dn(c, &m, t), up(c, &m, t)))
        }
        "jfeot_stoch" => {
            let n = u("n");
            let k: Vec<f64> = (0..c.len()).map(|i| {
                if i + 1 < n { return f64::NAN; }
                let hh = x.h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max);
                let ll = x.l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min);
                if hh > ll { (c[i] - ll) / (hh - ll) * 100. } else { f64::NAN }
            }).collect();
            let (lo, hi) = (vec![g("lo"); c.len()], vec![g("hi"); c.len()]);
            simulate(x, from, to, nx, |t| (up(&k, &lo, t), dn(&k, &hi, t), dn(&k, &lo, t), up(&k, &hi, t)))
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crosses_need_a_side_change() {
        let a = [1., 2., 2., 1., 1.];
        let b = [1.5; 5];
        assert!(!up(&a, &b, 0) && up(&a, &b, 1) && !up(&a, &b, 2) && dn(&a, &b, 3) && !dn(&a, &b, 4));
        let n = [f64::NAN, 2., 1.];
        assert!(!up(&n, &b, 1) && dn(&n, &b, 2));
    }
}

// Instrument admission: John Forman 2026-10-29 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/forman/admission/eot_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="jfeot" && crate::books::jfeot::reviewed(def.id),
    label: "John Forman tests these systems on daily USD/JPY (Ch.7) and GBP/USD (Ch.8) spot currency prices; runs on indices, ETFs and US/India stocks are a declared application, not the author's test. Cash-index shorts are hypothetical.",
    basis: "John Forman, The Essentials of Trading (2006), Ch.7 System Design and Testing p.162-202 and Ch.8 p.203-235",
};
