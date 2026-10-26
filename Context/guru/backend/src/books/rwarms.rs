//! Richard W. Arms Jr.: "Volume Cycles in the Stock Market" (1983/1994; book id `ravcs`), "Stop and Make Money" (Wiley 2008; `rasmm`),
//! "Trading with Equivolume" (2001; `rateq`). Reading: claude-continuation-20261020/authors/arms/agents/. Live: the Ease of Movement
//! rules of Volume Cycles Part Three - the 13-day EMV moving average crossing zero on daily bars (Ch.16) and the "four-week reversal of
//! the 13" on weekly bars (Ch.17). Equivolume patterns, power boxes, EMV trend lines and Arms Index extremes are judgement (held).
use crate::books::swsp::week_ends;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ravcs";
pub const REVIEWED: &[&str] = &["ravcs_emv13_daily", "ravcs_emv13_weekly4"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const EMV_F: &str = "F|Ease of movement value = midpoint move / box ratio; midpoint = (high + low) / 2, midpoint move = today's midpoint - yesterday's, box ratio = volume / range (Ch.15 'Calculating Ease of Movement')";
const D_NOTE: &str = "[Declared readings] Exact box ratio from raw volume and range (the book also gives a chart-counting shortcut; the volume unit does not change the sign of the average). A day with no range has an EMV of 0. Simple 13-day average of EMV. Buy at the close when the average turns from minus to plus, sell short when it turns from plus to minus (the book: 'the signal to cover shorts and go long'); always in after the first signal; zero is neither. Needs daily volume. Book: NYSE stocks (IBM 1982).";
const W_NOTE: &str = "[Declared readings] Weekly bars = Monday-Sunday weeks (week high, low and summed volume; the last data bar closes a week only on a Friday). Weekly EMV as on daily bars, simple 13-week average. Each week is 'Up' or 'Down' against the prior week (equal = neither, which breaks the string). Sell (go short) at the weekly close when four consecutive Downs follow a long; buy (cover) after four consecutive Ups; the first string of four starts the first position; a string in the direction already held only confirms. Book: 'we are short (or out of the stock)' - short is the default here. Needs volume.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, book, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: true,
        needs_vix: false, needs_series: &[], book, rules, notes, claim: None };
    let nt = |id, name, group, book, why, rules| StrategyDef { needs_volume: false, ..d(id, name, group, book, NotTestable(why), rules, "Held after the 2026-10-24 reading (authors/arms/agents).") };
    vec![
        d("ravcs_emv13_daily", "13-day Ease of Movement crossing zero (daily)", "Part Three · Ch.16 Ease of Movement on Daily Charts", "ravcs", Live,
          &[EMV_F,
            "E|\"the rally to day 51 generates a buy signal\"; \"At day 52, the signal to cover shorts and go long appears and stays in effect ... until the sell signal\" (Ch.16): the moving average of EMV crossing into plus (buy) or minus (sell) territory",
            "X|\"Experimentation with many values for the EMV has convinced us, however, that the 13-day value is the best we have worked with\" (Ch.16); the opposite crossing reverses the position"], D_NOTE),
        d("ravcs_emv13_weekly4", "Four-week reversal of the 13-week Ease of Movement (weekly)", "Part Three · Ch.17 Ease of Movement on a Weekly Basis", "ravcs", Live,
          &[EMV_F,
            "E|\"waiting until the 13-week moving average of the EMV has changed direction and progressed in that direction for four consecutive weeks, and then acting\"; \"four up notations in a row to tell us ... it is time to cover shorts and go long\" (Ch.17)",
            "X|\"we are now watching for a string of four consecutive down notations, which will be our signal to sell\" (Ch.17)"], W_NOTE),
        nt("ravcs_short_emv", "3-, 5-, 8- and 10-day EMV signals", "Part Three · Ch.16", "ravcs", "the author rejects them as too sensitive and says they 'should not be used mechanically, but judgmentally'", &[]),
        nt("ravcs_cycles", "Volume cycles and Equivolume chart analysis", "Volume Cycles in the Stock Market · Parts One, Two, Four", "ravcs", "cycle lengths and Equivolume shapes are read from the chart", &[]),
        nt("rasmm_power_box", "Power-box breakouts with stop orders", "Stop and Make Money · Ch.8-11", "rasmm", "a power box is 'the first heavy-volume box' judged on the chart, and stop-order levels sit at support and resistance chosen by eye", &["E|buy-stop entries above resistance on a power box (Ch.10)"]),
        nt("rasmm_emv_lines", "Ease of Movement trend lines", "Stop and Make Money · Ch.19 (p.151-155)", "rasmm", "signals are crossings of hand-drawn trend lines on the indicator", &["F|\"Each time the indicator crossed the trend line it was a valid signal to buy or sell\" (p.153)"]),
        nt("rasmm_vama", "13/34 volume-adjusted moving-average crossovers", "Stop and Make Money · Ch.19 (p.155-158)", "rasmm", "the author: 'an alert to a change of direction, but not as a reason to establish a long or short position' (p.156)", &[]),
        nt("rasmm_arms_index", "Arms Index 5/10/21-day averages", "Stop and Make Money · Ch.21 (p.165-171)", "rasmm", "extremes are 'a judgment call' against recent history with no fixed levels or exits; needs NYSE advancing/declining volume", &["F|Arms Index = (advances / declines) / (advancing volume / declining volume) (p.166)"]),
        nt("rateq_patterns", "Equivolume breakouts, continuation patterns and selling", "Trading with Equivolume", "rateq", "chart patterns and box shapes identified by eye", &[]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ravcs_emv13_daily" => ("ravcs_emv", "Arms Ease of Movement", "13-day zero cross"),
        "ravcs_emv13_weekly4" => ("ravcs_emv", "Arms Ease of Movement", "13-week four-week reversal"),
        _ if id.starts_with("ravcs_") || id.starts_with("rasmm_") || id.starts_with("rateq_") => ("rwarms_nt", "Arms volume methods (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "ravcs_emv13_daily" => vec![p("len", "EMV moving average (days)", 13., 3., 30., 1.)],
        "ravcs_emv13_weekly4" => vec![p("len", "EMV moving average (weeks)", 13., 3., 30., 1.), p("weeks", "Consecutive weeks for a reversal", 4., 2., 8., 1.)],
        _ if id.starts_with("ravcs_") || id.starts_with("rasmm_") || id.starts_with("rateq_") => vec![],
        _ => return None,
    })
}

fn sma_exact(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if n > 0 && i + 1 >= n { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect()
}
/// EMV from (high, low, volume) bars; NaN where the volume is missing or there is no previous bar.
fn emv(h: &[f64], l: &[f64], v: &[f64]) -> Vec<f64> {
    (0..h.len()).map(|i| {
        if i == 0 || !(v[i].is_finite() && v[i] > 0.) { return f64::NAN; }
        let mpm = (h[i] + l[i]) / 2. - (h[i - 1] + l[i - 1]) / 2.;
        let range = h[i] - l[i];
        if range > 0. { mpm / (v[i] / range) } else { 0. }
    }).collect()
}
fn vol(x: &Ctx, i: usize) -> f64 { x.r[i].volume.filter(|v| v.is_finite() && *v > 0.).unwrap_or(f64::NAN) }

fn tr(d: f64, i: usize, px: f64, sig: usize) -> Trade {
    Trade { dir: d, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: sig, open: false, exits: vec![] }
}

fn run_daily(x: &Ctx, from: usize, to: usize, n: usize) -> Vec<Trade> {
    let v: Vec<f64> = (0..x.r.len()).map(|i| vol(x, i)).collect();
    let m = sma_exact(&emv(&x.h, &x.l, &v), n);
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for i in from.max(1)..=to {
        if !(m[i].is_finite() && m[i - 1].is_finite()) { continue; }
        let d = if m[i - 1] <= 0. && m[i] > 0. { 1. } else if m[i - 1] >= 0. && m[i] < 0. { -1. } else { 0. };
        if d == 0. || cur.as_ref().map_or(false, |t| t.dir == d) { continue; }
        if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = if d > 0. { "EMV average turned plus" } else { "EMV average turned minus" }.into(); out.push(t); }
        cur = Some(tr(d, i, x.c[i], i));
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

fn run_weekly(x: &Ctx, from: usize, to: usize, n: usize, k: usize) -> Vec<Trade> {
    let we: Vec<usize> = week_ends(x).into_iter().filter(|&i| i <= to).collect();
    let (mut wh, mut wl, mut wv) = (vec![], vec![], vec![]);
    let mut start = 0usize;
    for &e in &we {
        let (mut h, mut l, mut v) = (f64::MIN, f64::MAX, 0.);
        for i in start..=e { h = h.max(x.h[i]); l = l.min(x.l[i]); v += vol(x, i); }
        wh.push(h); wl.push(l); wv.push(v); start = e + 1;
    }
    let m = sma_exact(&emv(&wh, &wl, &wv), n);
    let (mut out, mut cur, mut ups, mut downs) = (vec![], None::<Trade>, 0usize, 0usize);
    for j in 1..we.len() {
        if !(m[j].is_finite() && m[j - 1].is_finite()) { ups = 0; downs = 0; continue; }
        if m[j] > m[j - 1] { ups += 1; downs = 0; } else if m[j] < m[j - 1] { downs += 1; ups = 0; } else { ups = 0; downs = 0; }
        let i = we[j];
        if i < from { continue; }
        let d = if ups >= k { 1. } else if downs >= k { -1. } else { 0. };
        if d == 0. || cur.as_ref().map_or(false, |t| t.dir == d) { continue; }
        if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = if d > 0. { "four weekly rises of the EMV average" } else { "four weekly declines of the EMV average" }.into(); out.push(t); }
        cur = Some(tr(d, i, x.c[i], i));
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def)).round().max(1.) as usize;
    Some(match id {
        "ravcs_emv13_daily" => run_daily(x, from, to, g("len")),
        "ravcs_emv13_weekly4" => run_weekly(x, from, to, g("len"), g("weeks")),
        _ => return None,
    })
}
