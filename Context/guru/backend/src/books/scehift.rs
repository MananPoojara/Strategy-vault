//! John F. Ehlers, "The Inverse Fisher Transform", Technical Analysis of Stocks & Commodities V.22:5 (2004) pp.38-42 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V22/C05/095EHLR.pdf): the inverse Fisher
//! transform of RSI and its trading rules (QQQ example). Book id `scehift`. Lane K41 (batch T7a, 4 Nov 2026); independent check
//! authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::{rsi, wma};
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scehift_rsi"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] RSI = TradeStation's Wilder RSI (average gain/loss seeded with the mean of the first 5 changes); WAverage = 9-bar linearly weighted average. 'Buy when the indicator crosses over -0.5, or crosses over +0.5 (if it has not previously crossed over -0.5)' read as: a cross over -0.5 or over +0.5 is a buy unless the position is already long (a cross over +0.5 after a cross over -0.5 adds nothing); shorts mirrored; a cross = above the level now and at or below it on the previous bar. Always in after the first signal (long and short, 'ETFs ... can be bought long or sold short with equal facility'). The article prints the indicator code and a QQQ chart, no strategy code or results; fills at the next open (TradeStation's default for a strategy). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scehift_rsi", name: "Inverse Fisher transform of RSI(5): buy on crosses over -0.5 / +0.5, short on crosses under +0.5 / -0.5, next open", group: "S&C: inverse Fisher transform (Ehlers, V.22:5)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scehift", rules: &[
            "F|\"Value1 = .1*(RSI(Close, 5) - 50); Value2 = WAverage(Value1, 9); IFish = (ExpValue(2*Value2) - 1) / (ExpValue(2*Value2) + 1);\" (V.22:5, PDF p.2, sidebar)",
            "E|\"Buy when the indicator crosses over -0.5, or crosses over +0.5 (if it has not previously crossed over -0.5). Sell short when the indicator crosses under +0.5, or crosses under -0.5 (if it has not previously crossed under +0.5).\" (PDF p.2)",
            "X|The opposite signal closes the position and opens the other side (always in) [reading: no separate exit is printed]; fills at the next open"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scehift_rsi", "Inverse Fisher RSI (Ehlers, S&C)", "Level crosses")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "rsi", label: "RSI bars", def: 5., min: 2., max: 30., step: 1. },
        ParamDef { key: "wma", label: "Weighted average bars", def: 9., min: 1., max: 30., step: 1. }])
}

pub fn ifish(c: &[f64], n: usize, w: usize) -> Vec<f64> {
    let v1: Vec<f64> = rsi(c, n).iter().map(|r| 0.1 * (r - 50.0)).collect();
    wma(&v1, w).iter().map(|v| ((2.0 * v).exp() - 1.0) / ((2.0 * v).exp() + 1.0)).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let f = ifish(&x.c, g("rsi", 5.), g("wma", 9.));
    Some(next_open(x, from, to, "Inverse Fisher reversal", |t, side| {
        if t == 0 || !f[t - 1].is_finite() { return side; }
        let over = |lv: f64| f[t] > lv && f[t - 1] <= lv;
        let under = |lv: f64| f[t] < lv && f[t - 1] >= lv;
        if over(-0.5) || over(0.5) { 1.0 } else if under(0.5) || under(-0.5) { -1.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transform_is_bounded_and_trades() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        assert!(ifish(&x.c, 5, 9).iter().filter(|v| v.is_finite()).all(|v| v.abs() < 1.0));
        let t = run("scehift_rsi", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.windows(2).all(|w| w[0].dir != w[1].dir));
    }
}
