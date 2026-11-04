//! Joe Luisi, "Monitoring Equity For Market Analysis", Technical Analysis of Stocks & Commodities V.11:4 (1993) pp.166-168, with the
//! sidebar "Least-squares cycle and the time series moving average" (research share Trading Knowledge I/Stocks & Commodities DVD
//! 11.26 (1982 - 2008) (traders.com)/VOLUMES/V11/C04/MONITOR.pdf, SIDELEA.pdf). Book id `scjlme`: a least-squares oscillator system
//! and a time-series MA crossover, each traded only while its own equity is above the equity's 25-day average.
//! Lane K41 (batch T7a, 4 Nov 2026); independent check authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::scdkdmi::{at_close, close_equity, gt, ols_end};

pub const REVIEWED: &[&str] = &["scjlme_osc", "scjlme_trend"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Least-squares value = Excel TREND over the last n closes evaluated at the last point (=TREND(D2:D14,,13)); comparisons ignore floating-point noise (an exact tie is no cross and no turn). Oscillator system: osc = 3-day simple average of (close - 13-day least-squares value); 'Sell signals are taken when the oscillator rises above the zero line and then turns down' = the previous value is above zero and the oscillator falls after rising (osc[t-1] > 0, osc[t] < osc[t-1], osc[t-1] >= osc[t-2]); buys mirrored. Trend system: 10-day time-series MA (least-squares value) of the lows crossing the 10-day simple average of the lows. Each system is a stop-and-reverse shadow system from the first bar of the data, positions at the signal close ('Signals are taken on the close of the day of the signal'); its equity = the sum of its price-point changes marked at each close (the $10,000 start does not move the crossings; the $16 round-turn commission is left out: a dollar amount on currency contracts has no meaning on the vault's instruments (decision 166), and a per-trade cost would step the equity down at each trade, which can move some crossings). The published position = the shadow position while the equity closes above its 25-day simple average, else flat ('A trader can enter when the equity rises above the moving average in the direction of the latest signal ... and exited when the equity fell below'); changes at the close. Option filter = 0 publishes the shadow system. The article runs both systems together and takes no signals when both equities are below their averages; each card here is one system with its own filter. Article tests: Canadian dollar and Swiss franc, March-August 1992 (charts only). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "S&C: monitoring equity (Luisi, V.11:4)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scjlme", rules, notes: NOTE, claim: None };
    const F: &str = "F|\"By tracking the equity, a trader can invest more capital into the system whose equity is above the moving average and ignore signals from the system whose equity is below the moving average.\" (V.11:4, PDF p.1); \"A trader can enter when the equity rises above the moving average in the direction of the latest signal, which was short at 1.4276 and exited when the equity fell below at 1.3151.\" (PDF p.2)";
    const X: &str = "X|The opposite shadow signal reverses the position at the signal close; the published position goes flat at the close on which the equity is not above its 25-day average (\"exited when the equity fell below\", MONITOR.pdf p.2)";
    vec![
        d("scjlme_osc", "13-day least-squares oscillator (3-day average) turns, traded only while its equity is above its 25-day average", &[
            "F|\"the difference between the current closing price and the current value of the least-squares regression line is determined. This value is smoothed with an n period moving average\"; \"=TREND(D2:D14,,13)\"; \"=AVERAGE(F15:F17)\" (SIDELEA.pdf p.1-2)",
            "E|\"Sell signals are taken when the oscillator rises above the zero line and then turns down. Buy signals occur when the indicator goes below the zero line and turns up. Signals are taken on the close of the day of the signal.\" (MONITOR.pdf p.2)", F, X]),
        d("scjlme_trend", "10-day time-series MA of the lows crossing their 10-day SMA, traded only while its equity is above its 25-day average", &[
            "E|\"The first is a 10-day simple moving average and the second is a 10-day time series moving average (see sidebar). Both moving averages are calculated using the daily lows ... Buys occur when the time series moving average crosses above the simple moving average and sells occur when the time series moving average crosses below\" (MONITOR.pdf p.2); \"=TREND(D5:D14,,10)\" (SIDELEA.pdf p.2)", F, X]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "scjlme_osc" => "Least-squares oscillator", "scjlme_trend" => "Time-series MA of lows", _ => return None };
    Some(("scjlme_equity", "Equity monitoring (Luisi, S&C)", v))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "ma", label: "Equity moving average days", def: 25., min: 2., max: 60., step: 1. },
        ParamDef { key: "filter", label: "1 = trade only while the equity is above its average; 0 = the shadow system", def: 1., min: 0., max: 1., step: 1. }])
}

/// Shadow position (+1/-1/0) after each close.
fn shadow(id: &str, x: &Ctx) -> Vec<f64> {
    let n = x.c.len();
    let mut pos = vec![0.0; n];
    let mut side = 0.0;
    if id == "scjlme_osc" {
        let ls = ols_end(&x.c, 13);
        let d: Vec<f64> = (0..n).map(|i| x.c[i] - ls[i]).collect();
        let o = sma(&d, 3);
        for t in 2..n {
            if o[t - 2].is_finite() {
                let z = 1e-9 * x.c[t - 1]; // price-level noise: the zero line and the turns ignore changes within it
                if o[t - 1] > z && o[t - 1] - o[t] > z && !(o[t - 2] - o[t - 1] > z) { side = -1.0; }
                else if o[t - 1] < -z && o[t] - o[t - 1] > z && !(o[t - 1] - o[t - 2] > z) { side = 1.0; }
            }
            pos[t] = side;
        }
    } else {
        let (ts, s) = (ols_end(&x.l, 10), sma(&x.l, 10));
        for t in 1..n {
            if s[t - 1].is_finite() {
                if gt(ts[t], s[t]) && !gt(ts[t - 1], s[t - 1]) { side = 1.0; } else if gt(s[t], ts[t]) && !gt(s[t - 1], ts[t - 1]) { side = -1.0; }
            }
            pos[t] = side;
        }
    }
    pos
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let nm = p.get("ma").copied().unwrap_or(25.).round().max(1.) as usize;
    let filt = p.get("filter").copied().unwrap_or(1.) > 0.5;
    let sh = shadow(id, x);
    let eq = close_equity(&x.c, &sh);
    let ma = sma(&eq, nm);
    Some(at_close(x, from, to, "Equity below its average", |t, _| if !filt || eq[t] > ma[t] { sh[t] } else { 0.0 }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shadow_and_filter() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let mut p = Params::new(); p.insert("filter".into(), 0.);
            let all = run(id, &x, 0, r.len() - 1, &p).unwrap();
            let f = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(all.len() > 5 && !f.is_empty() && all.windows(2).all(|w| w[0].dir != w[1].dir), "{id}");
        }
    }
}
