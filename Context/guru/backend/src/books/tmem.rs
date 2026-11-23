//! "The 3-Minute E-Mini Trading System" 12th Edition (April 2007, 3-minute-e-mini.com), Sections 7-12, pp. 15-23. Book id `tmem`.
//! Five daily patterns on the S&P e-mini futures regular-hours day (yesterday / today / tomorrow), entered by a stop at the
//! stated level, exited at the close ("market on close", Section 12 B). Lane K2 (23 Nov 2026, R4 #340): run on the NIFTY 50 /
//! NIFTYBANK FUTURES daily bars built from the lane-DI 1-minute series (a cash index cannot stand in for futures prices).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tmem";
const IDS: [&str; 5] = ["tmem_bounce", "tmem_escalator", "tmem_opening", "tmem_wider", "tmem_modesc"];

const NOTE: &str = "A|Adapted (R4 #340, lane K2 23 Nov 2026), UNVERIFIED: the manual trades the S&P 500 e-mini; the rule runs here on the NIFTY 50 / NIFTYBANK FUTURES series (lane DI 1-minute, nearest-expiry continuous, NOT back-adjusted; days with a blank volume are absent, so 'yesterday' is the previous day present) as regular-session daily bars (first minute's open, session high/low, the 15:29 minute's close; 09:15-15:30 IST). The manual prints no result. Readings, 4, all labelled: (1) a stop order triggers when the day's high (sell: low) reaches the level and fills at the level, or at the open when the day opens beyond it; (2) the manual's recommended stop-loss ($250-$1,000, to taste, Section 12 A) is a currency amount with no home on NIFTY and is not applied: the only exits are the close (MOC) and nothing else, so the result carries the pattern's full loss; (3) 'Wider Range' and 'Modified Escalator' use strict inequalities and the manual's 'lower than the open or in the lower third of the range' as an OR; (4) one position at most per pattern per day, each pattern a separate card (the manual does not say how simultaneous signals of different patterns combine). No costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id: &'static str, name: &'static str, rules: &'static [&'static str]| StrategyDef { id, name, group: "The 3-Minute E-Mini Trading System (2007)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: "Manual Sections 7-12 (pp. 15-23): five daily patterns; the stop-loss is optional and left to the trader ($250-$1,000), exit is the close. No printed results.", claim: None };
    vec![
        mk(IDS[0], "Bounce Pattern (daily, futures)", &["E|Higher high, higher low and lower close than yesterday: BUY tomorrow if price goes above yesterday's high; lower high, lower low and higher close: SELL tomorrow if price goes below yesterday's low (Strategy #1, p. 17)", "X|Exit at the close of the trade day (MOC, Section 12 B)", NOTE]),
        mk(IDS[1], "Escalator Pattern (daily, futures)", &["E|Today closed below yesterday's low: BUY tomorrow if price goes above today's high; closed above yesterday's high: SELL tomorrow if price goes below today's low (Strategy #2, p. 18)", "X|Exit at the close of the trade day", NOTE]),
        mk(IDS[2], "Opening Pattern (daily, futures)", &["E|Today opens below yesterday's low: BUY today if price goes above yesterday's low; opens above yesterday's high: SELL today if price goes below yesterday's high (Strategy #3, p. 19)", "X|Exit at the close of the day", NOTE]),
        mk(IDS[3], "Wider Range Pattern (daily, futures)", &["E|Outside day (higher high and lower low) that closed below yesterday's close: BUY at tomorrow's open plus 10% of today's range; closed above yesterday's close: SELL at tomorrow's open minus 10% of today's range (Strategy #4, p. 20)", "X|Exit at the close of the trade day", NOTE]),
        mk(IDS[4], "Modified Escalator Pattern (daily, futures)", &["E|Higher high, low and close than yesterday and today's close below its open or in the lower third of its range: BUY tomorrow if price goes above today's high; mirror (lower high, low, close; close above the open or in the upper third): SELL tomorrow if price goes below today's low (Strategy #5, p. 21)", "X|Exit at the close of the trade day", NOTE]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { IDS.contains(&id).then_some(("tmem", "The 3-Minute E-Mini daily patterns", "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == IDS[3]).then(|| vec![ParamDef { key: "pct", label: "Range fraction added to the open (%, 10 per the manual)", def: 10., min: 1., max: 50., step: 1. }])
        .or_else(|| IDS.contains(&id).then(Vec::new))
}
#[derive(Clone, Copy)]
struct D { i: usize, o: f64, h: f64, l: f64, c: f64 }
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !IDS.contains(&id) || to >= x.c.len() || from > to { return None; }
    let pct = *p.get("pct").unwrap_or(&10.0) / 100.0;
    let mut d: Vec<D> = vec![];
    if let Some(f) = x.fut {
        for i in from..=to {
            let Some((m, _)) = f.fut_day(&x.r[i].date) else { continue };
            if m.is_empty() { continue; }
            d.push(D { i, o: m[0].o, h: m.iter().map(|b| b.h).fold(f64::MIN, f64::max), l: m.iter().map(|b| b.l).fold(f64::MAX, f64::min), c: m[m.len() - 1].c });
        }
    }
    let mut out = vec![];
    let trade = |out: &mut Vec<Trade>, dir: f64, z: &D, lv: f64| {
        let hit = if dir > 0.0 { z.h >= lv } else { z.l <= lv };
        if !hit { return; }
        let px = if dir > 0.0 { z.o.max(lv) } else { z.o.min(lv) };
        out.push(crate::intraday::day_trade(dir, z.i, px, z.i, z.c, "exit on the close (MOC)", "limit", false));
    };
    for k in 2..d.len() {
        let (y, t, z) = (&d[k - 2], &d[k - 1], &d[k]); // yesterday, today, tomorrow (the trade day)
        match id {
            "tmem_bounce" => {
                if t.h > y.h && t.l > y.l && t.c < y.c { trade(&mut out, 1.0, z, y.h); }
                else if t.h < y.h && t.l < y.l && t.c > y.c { trade(&mut out, -1.0, z, y.l); }
            }
            "tmem_escalator" => {
                if t.c < y.l { trade(&mut out, 1.0, z, t.h); } else if t.c > y.h { trade(&mut out, -1.0, z, t.l); }
            }
            "tmem_opening" => {}
            "tmem_wider" => {
                if t.h > y.h && t.l < y.l {
                    let r = t.h - t.l;
                    if t.c < y.c { trade(&mut out, 1.0, z, z.o + pct * r); } else if t.c > y.c { trade(&mut out, -1.0, z, z.o - pct * r); }
                }
            }
            _ => {
                let r = t.h - t.l;
                if t.h > y.h && t.l > y.l && t.c > y.c && (t.c < t.o || t.c < t.l + r / 3.0) { trade(&mut out, 1.0, z, t.h); }
                else if t.h < y.h && t.l < y.l && t.c < y.c && (t.c > t.o || t.c > t.h - r / 3.0) { trade(&mut out, -1.0, z, t.l); }
            }
        }
    }
    if id == "tmem_opening" {
        for k in 1..d.len() { let (y, t) = (&d[k - 1], &d[k]); if t.o < y.l { trade(&mut out, 1.0, t, y.l); } else if t.o > y.h { trade(&mut out, -1.0, t, y.h); } }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_and_params() {
        assert_eq!(params("tmem_wider").unwrap()[0].def, 10.0);
        assert!(params("tmem_bounce").unwrap().is_empty());
        assert!(params("x").is_none());
    }
}
