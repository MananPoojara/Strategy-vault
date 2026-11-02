//! Peter Aan, "Key reversal days", Technical Analysis of Stocks & Commodities V.7:10 (October 1989) pp.347-349 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V07/C10/KEYREV.pdf). Book id `scaankr`.
//! Lane K13 (titles loop, 2 Nov 2026), to-code row 200, lead decision 73; independent check authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::sctraub::ticks;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scaankr_a"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Bearish reversal day: a higher high than the day before and a lower close; Model A (default, the article's best on coffee) takes any of patterns A, B, C; Model B (option) needs a lower low or a close under the prior low; Model C (option) a close under the prior low. The day's high must be the highest high of the last 30 days, this day included (bullish mirror at a 30-day low). The trade is entered at the next open; a reversal day the other way reverses the position at the next open. The entry day's stop is 1 tick above the reversal day's high ('one point', read as one minimum tick, decision 57; 0 is an option); from the next day the stop is the highest high of the last 10 days (long: lowest low), filled at the stop or at the open if the open is beyond it. The article's tests (five years of futures, setups 5-40, stops 5-30 days, $100 a trade) are printed as a chart table; the 30/10 days of its rule text are the defaults (decision 73). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scaankr_a", name: "Aan key reversal days: fade a reversal day at a 30-day high or low at the next open, 10-day high/low trailing stop", group: "S&C: Key reversal days (Aan, V.7:10)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scaankr", rules: &[
            "F|\"three reversal patterns ... have two things in common: a high that is higher than the previous day's high and a close that is below yesterday's close ... Model A entered the market if either patterns A, B or C were present. Model B entered on patterns B or C. Model C ... required a close below the previous low\" (V.7:10 pp.347-349)",
            "E|\"If the market has made a new high for the most recent 30 trading days and a reversal pattern occurs, sell tomorrow on the opening.\" Long trades are handled in an inverse manner; \"If a bullish reversal day occurs while in a short position, the position is simply reversed.\" (p.349)",
            "X|\"Use an initial protective stop one point above the high of the day prior to entry (the new 30-day high). Liquidate the trade on a stop at the high of the most recent 10 days.\" (p.349)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Aan key reversal days (S&C)", "30-day setup, 10-day stop"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("model", "Pattern model (1 = A: any pattern; 2 = B; 3 = C: close under the prior low)", 1., 1., 3., 1.),
        p("setup", "Setup: reversal day at the n-day high/low (30)", 30., 5., 40., 1.), p("trail", "Trailing stop: n-day high/low (10)", 10., 5., 30., 1.),
        p("tick", "Initial stop offset in ticks ('one point'; 0 = none)", 1., 0., 10., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.0) as usize;
    let (m, n, tr, k) = (g("model", 1.), g("setup", 30.), g("trail", 10.), ticks(x, p));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut pos = Book::new();
    let mut order: Option<(f64, f64)> = None;
    let mut stop = f64::NAN;
    for i in from..=to {
        if let Some((d, s0)) = order.take() { if pos.side() != d { pos.enter(d, i, o[i], "open"); stop = s0; } }
        let d = pos.side();
        if d < 0.0 && h[i] >= stop { pos.exit(i, o[i].max(stop), "stop"); }
        else if d > 0.0 && l[i] <= stop { pos.exit(i, o[i].min(stop), "stop"); }
        if i == to || i < 1usize.max(n - 1) { continue; }
        let mut bear = h[i] > h[i - 1] && c[i] < c[i - 1] && h[i] == hi(h, i + 1 - n, i);
        let mut bull = l[i] < l[i - 1] && c[i] > c[i - 1] && l[i] == lo(l, i + 1 - n, i);
        if m == 2 { bear = bear && (l[i] < l[i - 1] || c[i] < l[i - 1]); bull = bull && (h[i] > h[i - 1] || c[i] > h[i - 1]); }
        else if m == 3 { bear = bear && c[i] < l[i - 1]; bull = bull && c[i] > h[i - 1]; }
        let d = pos.side();
        if d != 0.0 { let a = (i + 1).saturating_sub(tr); stop = if d < 0.0 { hi(h, a, i) } else { lo(l, a, i) }; }
        if bear && d != -1.0 { order = Some((-1.0, h[i] + k)); } else if bull && d != 1.0 { order = Some((1.0, l[i] - k)); }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_reversal_trades_both_ways() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let n: Vec<usize> = [1.0, 3.0].iter().map(|&m| {
            let p: Params = [("model".to_string(), m)].into_iter().collect();
            let t = run("scaankr_a", &x, 0, r.len() - 1, &p).unwrap();
            assert!(t.iter().all(|z| z.legs[0].px == x.o[z.legs[0].i]), "model {m}");
            t.len()
        }).collect();
        assert!(n[0] > 3 && n[1] <= n[0], "{n:?}");
    }
}

// Instrument admission: S&C Aan key reversal 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scaankr" && reviewed(def.id),
    label: "Peter Aan tested key reversal days on five years of futures (coffee best). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.7:10 pp.347-349 (Key reversal days); independent check authors/titles-1030/K13w/k13_ref.py",
};
