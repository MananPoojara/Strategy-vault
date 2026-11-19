//! Phil Abel, "Diversification And Your Portfolio", Technical Analysis of Stocks & Commodities V21:10 (October 2003) p.20-31
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V21/C10/208ABEL.pdf).
//! Book id `scabeldiv`, author Phil Abel (his other article, V21:4 triple SMA, is book `scabel`). Lane PF (13 Nov 2026, A1 row 429;
//! R4 #293, #297); independent check authors/pf/w/pf_ref2.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::mk;

pub const REVIEWED: &[&str] = &["scabeldiv_sma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; declared adaptation: one contract per trade] Norgate continuous futures (back-adjusted daily bars), each of the article's Figure 3 contracts held: yen, Swiss franc, gold, silver, S&P 500, Nasdaq-100 (the regular contract is not held; the E-mini on the same index, which the article names, stands in), natural gas, corn, wheat, cocoa, sugar, cotton, coffee, 10-year note, Eurodollar (crude oil is not run: a crude trade held through 2020-04-20 (the short of 2020-02-27 at 47.09) is priced below zero in the contract held at entry (its mark is -38.32 on 2020-04-20, exit -15.84), so the engine's trade validation refuses the whole run). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. Unstated in the article: (1) 'four standard deviations (10 days)': vault reading, 4 x the standard deviation of the last 10 closes at the entry bar (TradeStation StdDev, population), fixed for the trade; (2) fills: vault reading, entries and the moving-average exits at the close of the signal day, the hard stop as a stop order from the next day (at the stop, or at the open when the market opens through it). The entry is a state, so a flat system re-enters at any close where the averages are stacked (also on the day of a stop). The article sizes each position to lose 1% of equity at the stop and reports a 16-market portfolio; the vault runs one contract per market and reports each market alone (declared adaptation), so the portfolio figures are not comparable.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scabeldiv_sma", name: "Abel futures trend system: close above a rising stack of 25/50/100-day averages, exit when the 25 crosses the 100 or at a 4-standard-deviation stop (long and short)", group: "S&C: Diversification And Your Portfolio (Abel, V21:10)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "scabeldiv", rules: &[
            "E|\"Enter long when close > SMA(25) and SMA(25) > SMA(50) and SMA(50) > SMA(100) [SMA = simple moving average]\" (p.25)",
            "X|\"Exit long (trailing stop) when SMA(25) < SMA(100)\"; \"Exit long (hard stop) four standard deviations (10 days) below the entry price.\" (p.25)",
            "E|\"The system will trade both long and short positions.\" (p.25): short when close < SMA(25) < SMA(50) < SMA(100); cover when SMA(25) > SMA(100) or four standard deviations above the entry",
            "F|\"Positions are sized so that 1% of total equity is lost if the market retraces to the hard stop price\" (p.25); simulated over continuous futures prices from 1995 to 2002 on 16 markets (Figure 3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scabeldiv_sma", "Abel 25/50/100 futures trend system", "Printed rules"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("a", "Short average", 25., 5., 60., 1.), p("b", "Middle average", 50., 10., 120., 1.), p("cl", "Long average", 100., 20., 250., 5.),
        p("k", "Hard stop: standard deviations", 4., 1., 8., 0.5), p("sd", "Standard deviation days", 10., 5., 40., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let (a, b, cl, sdn, k) = (u("a"), u("b"), u("cl"), u("sd"), g("k"));
    let c = &x.c;
    let sma = |n: usize, i: usize| if i + 1 >= n { c[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN };
    let sd = |i: usize| {
        if i + 1 < sdn { return f64::NAN; }
        let w = &c[i + 1 - sdn..=i];
        let m = w.iter().sum::<f64>() / sdn as f64;
        (w.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / sdn as f64).sqrt()
    };
    let mut out = Vec::new();
    let mut pos: Option<(f64, usize, f64)> = None; // (dir, entry bar, stop)
    for i in 0..=to {
        let (s1, s2, s3) = (sma(a, i), sma(b, i), sma(cl, i));
        if let Some((d, e, stop)) = pos {
            // hard stop from the day after entry (stop order: at the stop, or at the open when the market opens through it)
            let hit = if d > 0. { x.l[i] <= stop } else { x.h[i] >= stop };
            if i > e && hit {
                let px = if d > 0. { x.o[i].min(stop) } else { x.o[i].max(stop) };
                if e >= from { out.push(mk(d, e, c[e], "close", i, px, "Hard stop (4 standard deviations)", false, i)); }
                pos = None;
            } else if if d > 0. { s1 < s3 } else { s1 > s3 } {
                if e >= from { out.push(mk(d, e, c[e], "close", i, c[i], "SMA(25) crossed SMA(100)", false, i)); }
                pos = None;
                continue; // the exit bar's close is the exit; the stack cannot hold on the same close
            }
        }
        if pos.is_none() {
            let d = if c[i] > s1 && s1 > s2 && s2 > s3 { 1. } else if c[i] < s1 && s1 < s2 && s2 < s3 { -1. } else { 0. };
            let v = sd(i);
            if d != 0. && v.is_finite() { pos = Some((d, i, c[i] - d * k * v)); }
        }
    }
    if let Some((d, e, _)) = pos { if e >= from { out.push(mk(d, e, c[e], "close", to, c[to], "open", true, to)); } }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn ctx(px: &[f64]) -> Vec<Row> {
        px.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{}-{:02}-{:02}", 2000 + i / 300, 1 + (i % 300) / 25, 1 + i % 25), open: c, high: c + 0.5, low: c - 0.5, close: c, volume: None }).collect()
    }
    #[test]
    fn stacked_entry_and_hard_stop() {
        // Unit-test fixture only (no market data): a steady rise stacks the averages (long from bar 99, the first bar with SMA(100));
        // a 40-point collapse on bar 130 opens below the 4-standard-deviation stop -> filled at that open.
        let px: Vec<f64> = (0..140).map(|i| if i < 130 { 100. + i as f64 } else { 189. - 40. }).collect();
        let r = ctx(&px);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scabeldiv_sma", &x, 0, 139, &Params::new()).unwrap();
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (1.0, 99, 130));
        assert_eq!(t[0].exit_px, 149.0); // opened through the stop (199 - 4 x 2.87 = 187.5): the open fills
        assert!(t[0].reason.starts_with("Hard stop"));
    }
}
