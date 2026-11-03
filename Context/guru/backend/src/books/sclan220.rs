//! David S. Landry, "A Volatility Trade In Gold", Technical Analysis of Stocks & Commodities V.16:7 (July 1998) pp.344-348,
//! sidebar "2/20-day EMA breakout system defined" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008)
//! (traders.com)/VOLUMES/V16/C07/055VOLA.pdf). Book id `sclan220`. Lane K16 (titles loop, 2 Nov 2026), to-code row 247,
//! decision 57; independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{ema0, ticks};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sclan220_bo"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The sidebar's rules: buy alert when today's and yesterday's lows are both above the 20-day EMA; it stays valid until a low touches or falls below the EMA; buy on a stop 10 ticks above the two-day high (today's and yesterday's highs), kept while the alert is valid, so each day the alert holds the stop for the next day is 10 ticks above the latest two days' highs. Long exit: a stop at the 20-day EMA, updated daily (the EMA of the day before, from the day after entry). Sell alert, sell entry and short exit mirrored. 'Ticks' = the instrument's minimum tick (NSE 0.05, US 0.01, decision 57); 0 ticks is an option. Stops fill at their level or at the open when the open is beyond it; entries only when flat at the start of a day; EMA alpha 2/21 seeded with the first close. The article applies the setup to COMEX gold as a confirming indicator and prints no system results; the ATR, volatility and hammer remarks are trade commentary, not part of the system. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sclan220_bo", name: "Landry 2/20-day EMA breakout: two lows above the 20-day EMA, buy 10 ticks over the 2-day high, trail at the EMA (shorts mirrored)", group: "S&C: A Volatility Trade In Gold (Landry, V.16:7)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sclan220", rules: &[
            "E|\"Buy alert: If today's low and yesterday's low is greater than the 20-day EMA. This signal remains valid until the low touches or falls below the 20-day EMA. Buy entry: Place a stop order 10 ticks above the two-day high ... Keep order until filled or as long as the buy alert is still valid.\" (V.16:7, PDF p.4 sidebar)",
            "X|\"Long exit: Place a stop equal to the 20-day EMA. Continue to update this stop daily to form a trailing stop.\" (PDF p.4)",
            "E|\"Sell alert: If today's high and yesterday's high is less than the 20-day EMA ... Sell entry: Place a stop order 10 ticks below the two-day low.\" (PDF p.4)",
            "X|\"Short exit: Place a stop equal to the 20-day EMA. Continue to update this stop daily to form a trailing stop.\" (PDF p.4)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("sclan220_bo", "Landry 2/20-day EMA breakout (S&C)", "20-day EMA, 10 ticks"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("tick", "Entry offset in ticks (sidebar 10; 0 = none)", 10., 0., 20., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let mut q = p.clone();
    q.entry("tick".into()).or_insert(10.0);
    let k = ticks(x, &q);
    let e = ema0(&x.c, 20);
    let mut pos = Book::new();
    for i in from.max(2)..=to {
        let j = i - 1; // stops are set at the close of day j for day i
        match pos.side() {
            s if s == 0.0 => {
                if x.l[j] > e[j] && x.l[j - 1] > e[j - 1] {
                    let lv = x.h[j].max(x.h[j - 1]) + k;
                    if x.h[i] >= lv { pos.enter(1.0, i, x.o[i].max(lv), "stop"); }
                } else if x.h[j] < e[j] && x.h[j - 1] < e[j - 1] {
                    let lv = x.l[j].min(x.l[j - 1]) - k;
                    if x.l[i] <= lv { pos.enter(-1.0, i, x.o[i].min(lv), "stop"); }
                }
            }
            s if s > 0.0 => if x.l[i] <= e[j] { pos.exit(i, x.o[i].min(e[j]), "EMA stop") },
            _ => if x.h[i] >= e[j] { pos.exit(i, x.o[i].max(e[j]), "EMA stop") },
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn breakout_trails_at_the_ema() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sclan220_bo", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.0) && t.iter().any(|z| z.dir < 0.0));
        assert!(t.iter().all(|z| z.exit_i > z.legs[0].i || z.open));
    }
}

// Instrument admission: S&C Landry 2/20 EMA breakout 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sclan220" && reviewed(def.id),
    label: "Dave Landry showed the setup on COMEX gold (1997, no system results). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.16:7 pp.344-348 (A Volatility Trade In Gold), sidebar '2/20-day EMA breakout system defined'; independent check authors/titles-1030/K16w/k16_ref.py",
};
