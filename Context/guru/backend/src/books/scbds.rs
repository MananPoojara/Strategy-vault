//! William Blau, "Double Smoothed-Stochastics", Technical Analysis of Stocks & Commodities V.9:1 (1991) pp.14-16 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V09/C01/DOUBLES.PDF). Book id `scbds`:
//! the DS-stochastic crossover system of the Compaq example. Lane K41 (batch T7a, 4 Nov 2026); independent check
//! authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::ema0;
use super::scdkdmi::at_close;

pub const REVIEWED: &[&str] = &["scbds_cross"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] DS(a,y,z) = 100 x EMA_z(EMA_y(C - LL_a)) / EMA_z(EMA_y(HH_a - LL_a)), LL_a / HH_a = lowest low / highest high of the last a bars including today, EMA alpha 2/(n+1) seeded with its first value (the sidebar's EMA definition). Default DS(2,3,15) and its 3-day simple average, the Compaq example (March-May 1990). Signals and fills at the close ('The first buy signal is given on February 28 at a close of 84 1/4'). Buy when DS crosses above its average; the sell signal closes the long (the example is a stock and counts the rallies from each buy to the next sell); option short = 1 also sells short on the cross down (always in). The article prints a chart and signal prices only. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scbds_cross", name: "Double smoothed stochastic DS(2,3,15) crosses its 3-day average: buy on the cross up, sell on the cross down, at the close", group: "S&C: double smoothed stochastics (Blau, V.9:1)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scbds", rules: &[
            "F|\"DS-stochastic(a,y,z) = 100 Ez(Ey(C - La)) / Ez(Ey(Ha - La)) where: C = Current close, La = Lowest low in a days, Ha = Highest high in a days, Ey = y-day exponential moving average, Ez = z-day exponential moving average\" (V.9:1, PDF p.2); \"EMAtoday = EMAyesterday + a(Ptoday - EMAyesterday)\", a = 2/(y + 1) (PDF p.2, sidebar p.4)",
            "E|\"A DS-stochastic (2, 3, 15) is shown ... The arrows indicate buy and sell signals using a simple crossover system of the DS-stochastic over its three-day arithmetic average as the trading system.\" (PDF p.2)",
            "X|\"The first buy signal is given on February 28 at a close of 84 1/4. The rally that ensues is capped off with a sell signal at 93 3/4 on March 12.\" (PDF p.3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scbds_cross", "Double smoothed stochastic (S&C)", "Crossover of its 3-day average")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "a", label: "a: high/low lookback days", def: 2., min: 1., max: 20., step: 1. },
        ParamDef { key: "y", label: "y: first EMA days", def: 3., min: 1., max: 30., step: 1. },
        ParamDef { key: "z", label: "z: second EMA days", def: 15., min: 1., max: 40., step: 1. },
        ParamDef { key: "short", label: "1 = also sell short on the cross down (always in); 0 = long only", def: 0., min: 0., max: 1., step: 1. }])
}

pub fn ds(x: &Ctx, a: usize, y: usize, z: usize) -> Vec<f64> {
    let n = x.c.len();
    let (mut num, mut den) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    for i in a - 1..n {
        let hh = x.h[i + 1 - a..=i].iter().copied().fold(f64::MIN, f64::max);
        let ll = x.l[i + 1 - a..=i].iter().copied().fold(f64::MAX, f64::min);
        num[i] = x.c[i] - ll; den[i] = hh - ll;
    }
    let (nu, de) = (ema0(&ema0(&num, y), z), ema0(&ema0(&den, y), z));
    (0..n).map(|i| if de[i] > 0.0 { 100.0 * nu[i] / de[i] } else { f64::NAN }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let d = ds(x, g("a", 2.), g("y", 3.), g("z", 15.));
    let s = sma(&d, 3);
    let dn = if p.get("short").copied().unwrap_or(0.) > 0.5 { -1.0 } else { 0.0 };
    Some(at_close(x, from, to, "DS crossed below its average", |t, side| {
        if t == 0 || !s[t - 1].is_finite() || !d[t].is_finite() { return side; }
        if d[t] > s[t] && d[t - 1] <= s[t - 1] { 1.0 } else if d[t] < s[t] && d[t - 1] >= s[t - 1] { dn } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ds_stays_in_range_and_trades() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        assert!(ds(&x, 2, 3, 15).iter().skip(1).all(|v| (0.0..=100.0).contains(v)));
        let t = run("scbds_cross", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().all(|z| z.dir > 0.0));
        let mut p = Params::new(); p.insert("short".into(), 1.);
        assert!(run("scbds_cross", &x, 0, r.len() - 1, &p).unwrap().iter().any(|z| z.dir < 0.0));
    }
}
