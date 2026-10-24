//! Alpesh B. Patel, "The Internet Trading Course" (scan, OCR; printed page = pdf page - 33). Book id `apitc`.
//! Reading: claude-continuation-20261020/authors/patel/agents/itcourse/. Live: the Module 2 example of a mechanical
//! system, long above the 50-day moving average and out otherwise (p.23). The indicator chapters give no settings.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::mpts::sma;

pub const BOOK: &str = "apitc";
pub const REVIEWED: &[&str] = &["apitc_ma50"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let nt = |id, name, group, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long", status: NotTestable(why), needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes: "Held after the 2026-10-24 reading (authors/patel/agents/itcourse).", claim: None };
    vec![
        StrategyDef { id: "apitc_ma50", name: "Long above the 50-day moving average, out otherwise", group: "Module 2 · Mechanical or discretionary? (p.23)",
            session: 0, dir: "Long", status: Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"a very simple system may involve being long (i.e. being in the market) if the price is above the 50-day moving average\" (p.23)",
                     "X|\"and being out (i.e. out of the market) if it is not\" (p.23)"],
            notes: "Given as an example of a mechanical system, not as Patel's recommended system. [Declared readings, not from the book] Price = the daily close; simple 50-day average of closes; the position is taken or closed at the close that crosses; a close equal to the average is 'not above' (out). A run starts flat. No stop, no costs. The course is about US and UK shares; index, ETF and stock runs (US and India) are a declared application.",
            claim: None },
        nt("apitc_stoch", "Stochastic %K/%D crossover, momentum turns, MACD filter, trailing stop", "Modules 4-6 (p.69-82)",
           "no periods, levels, stop percentage or exits are given", &["E|Buy when %K crosses above %D (p.80)"]),
        nt("apitc_patterns", "Trendlines, support/resistance and chart patterns", "Module 5", "drawn by eye", &["E|Chart reading"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "apitc_ma50" => ("apitc_ma50", "Patel 50-day average", "50"),
        _ if id.starts_with("apitc_") => ("apitc_nt", "Patel Internet Trading Course (not testable)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "apitc_ma50").then(|| vec![ParamDef { key: "n", label: "Average days (book: 50)", def: 50., min: 2., max: 250., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "apitc_ma50" || from > to || to >= x.r.len() { return None; }
    let ma = sma(&x.c, p.get("n").copied().unwrap_or(50.).round().max(1.) as usize);
    let state = |i: usize| ma[i].is_finite().then(|| x.c[i] > ma[i]);
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for i in from.max(1)..=to {
        let (s, prev) = (state(i), state(i - 1));
        if !matches!((s, prev), (Some(a), Some(b)) if a != b) { continue; }
        if s == Some(true) {
            cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        } else if let Some(mut t) = cur.take() {
            t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = "close not above the average".into(); out.push(t);
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}
