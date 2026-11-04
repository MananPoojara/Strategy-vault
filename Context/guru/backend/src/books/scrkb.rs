//! Rene Koch, "Creating Your Own Trading System: A Tutorial On Bands", Technical Analysis of Stocks & Commodities V.22:7 (2004)
//! pp.24-35 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V22/C07/
//! 142KOCH.pdf): the constant-distance band system of Figure 3. Book id `scrkb`. Lane K41 (batch T7a, 4 Nov 2026); independent check
//! authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::ema0;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scrkb_band6"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Centre = 6-bar EMA (alpha 2/7, seeded with the first value) of the average price (High + Low)/2 (Wealth-Lab #Average); lower band = centre x (1 - 0.06), the code's dist := 0.06. The trading routine ('BandTrader') is not printed: the entry is a limit buy at the previous bar's lower band ('The band limits are shifted by one bar to the right to reflect the daily limits of the limit orders used to enter a position'), filled at the limit or at a lower open; the 1% profit target is a limit sell at the fill x 1.01 from the next bar (at the open if it opens above); after 'Timeout was two days' without the target the position is sold at the close of the second bar after entry ('One or two bars later, the position is closed'); one position at a time, the next entry from the bar after an exit. Long only. Article tests: 1,000 daily bars of eight stocks, $10,000 a trade, optimal bandwidths 5-25% (table; 6% is the code value). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scrkb_band6", name: "Limit buy at a 6% band below the 6-bar EMA of (H+L)/2, 1% profit target, 2-day time-out", group: "S&C: tutorial on bands (Koch, V.22:7)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scrkb", rules: &[
            "F|\"center_line := EMASeries( #Average, 6 ); ... dist := 0.06; upper_s := MultiplySeriesValue( center_line, 1 + dist ); lower_s := MultiplySeriesValue( center_line, 1 - dist );\" (V.22:7, PDF p.2, Figure 3)",
            "E|\"Whenever prices go below the red line, a buy signal (blue circle) is generated. One or two bars later, the position is closed\" (PDF p.1); \"The band limits are shifted by one bar to the right to reflect the daily limits of the limit orders used to enter a position.\" (PDF p.3, Figure 6)",
            "X|\"Profit target was 1%; Timeout was two days\" (PDF p.2, sidebar 'Details')"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scrkb_band6", "Constant band limit buy (Koch, S&C)", "6% band")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "band", label: "Band distance % (6 = the code; table optima 5-25)", def: 6., min: 1., max: 30., step: 0.5 },
        ParamDef { key: "target", label: "Profit target %", def: 1., min: 0.25, max: 10., step: 0.25 },
        ParamDef { key: "days", label: "Time-out bars", def: 2., min: 1., max: 10., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (dist, tgt, days) = (g("band", 6.) / 100.0, g("target", 1.) / 100.0, g("days", 2.).round().max(1.) as usize);
    let avg: Vec<f64> = (0..x.c.len()).map(|i| (x.h[i] + x.l[i]) / 2.0).collect();
    let ctr = ema0(&avg, 6);
    let mut pos = Book::new();
    let mut free_from = from.max(1);
    for t in from.max(1)..=to {
        if let Some(e) = pos.entry_bar() {
            let lv = pos.entry_px() * (1.0 + tgt);
            if x.h[t] >= lv { pos.exit(t, x.o[t].max(lv), "Profit target"); free_from = t + 1; }
            else if t - e >= days { pos.exit(t, x.c[t], "Time-out at the close"); free_from = t + 1; }
            continue;
        }
        if t < free_from { continue; }
        let lim = ctr[t - 1] * (1.0 - dist);
        if x.l[t] <= lim { pos.enter(1.0, t, x.o[t].min(lim), "limit"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn band_buys_exit_within_two_bars() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new(); p.insert("band".into(), 1.5);
        let t = run("scrkb_band6", &x, 0, r.len() - 1, &p).unwrap();
        assert!(!t.is_empty() && t.iter().filter(|z| !z.open).all(|z| z.exit_i - z.legs[0].i <= 2 && z.exit_i > z.legs[0].i));
    }
}
