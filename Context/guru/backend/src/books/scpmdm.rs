//! Patrick G. Mulloy, "Smoothing Data With Faster Moving Averages", Technical Analysis of Stocks & Commodities V.12:1 (1994)
//! pp.11-19 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V12/C01/
//! SMOOTHI.pdf). Book id `scpmdm`: the MACD built from one-parameter double exponential moving averages (MACD-DEMA1 12/26/9) on
//! weekly closes. Lane PG (14 Nov 2026, pool A row i1773, A1 score <= 1); independent check authors/pg/w/pg_ref.py.
//! Admitted by books::scjrd::ADMISSION (PG_BOOKS).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::period_ends;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scpmdm_macd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Weekly closes = the close of each calendar week's last trading day (the data's last week counts only when it ends on a Friday). DEMA1(w) = 2 x EMA1 - EMA2 with alpha = 2/(w+1), EMA1 the EMA of the series and EMA2 the EMA of EMA1, both started from the article's regression initialisation: over the first w values u = intercept and v = slope of the least-squares line (x = 0..w-1), EMA1(0) = u - ((1 - alpha)/alpha) v, EMA2(0) = u - 2((1 - alpha)/alpha) v, then the recursions from the second value (the article's worked example, PDF p.5). Because those starting values use the first w values, DEMA1 is used only from the w-th value on (no look-ahead). MACD-DEMA1 = DEMA1(12) - DEMA1(26) of the weekly closes; its signal line = DEMA1(9) of that MACD. A buy signal = the MACD crossing above its signal line (above this week, at or below last week). Unstated: (1) the exit - vault reading: the MACD crossing back below its signal line, long only (the article counts 'buy signals' and their 'losses', no shorts); (2) the fill - vault reading: the signal week's close. The article's Figure 2 (weekly NASDAQ Composite, from October 1988 into 1993: 9 trades, 3 losses) is a chart without dates or prices; the vault runs the rule on indices, ETFs and stocks, US and India (declared application), so it stays UNVERIFIED. The Figure 8 Parabolic SAR qualifier is an illustration without stated use and is not coded.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scpmdm_macd", name: "Mulloy MACD-DEMA1 (12, 26, 9) on weekly closes: buy when it crosses above its signal line, sell when it crosses back below", group: "S&C: smoothing data with faster moving averages (Mulloy, V.12:1)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scpmdm", rules: &[
            "F|\"DEMA1 = 2EMA1 - EMA2. Here, we are multiplying the single EMA by two and then subtracting the double EMA.\" with \"S1m = aCm + (1 - a)S1m-1, S2m = aS1m + (1 - a)S2m-1\" (V.12:1, PDF p.5)",
            "F|\"Calculate regression coefficients ... u = Intercept(x,y), v = Slope(x,y) ... Initialization equations: EMA1(0) = u - ((1-a)/a)(v), EMA2(0) = u - 2((1-a)/a)(v)\" (PDF p.5-6)",
            "E|\"In Figure 1, the MACD indicator is applied to the weekly closing price of the NASDAQ composite index ... Figure 2 uses the same filter lengths of 12, 26 and nine, but the filters are not EMAs but are derivations of one-parameter double exponential moving averages (DEMA1). This time, the indicator generated nine trades, with only three losses\" (PDF p.1)",
            "X|[Unstated] out when the MACD-DEMA1 crosses back below its signal line, at that week's close (vault reading, R4 #297)"],
        notes: NOTE, claim: crate::connors::claim("9", "67%", "", "", "Weekly NASDAQ Composite, Oct 1988 - 1993 (Figure 2, chart only)") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scpmdm_macd", "Mulloy MACD-DEMA1 (S&C)", "12/26/9 weekly")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "fast", label: "Fast DEMA1 weeks (article 12)", def: 12., min: 2., max: 100., step: 1. },
        ParamDef { key: "slow", label: "Slow DEMA1 weeks (article 26)", def: 26., min: 3., max: 200., step: 1. },
        ParamDef { key: "sig", label: "Signal DEMA1 weeks (article 9)", def: 9., min: 2., max: 100., step: 1. }])
}

/// DEMA1(w) of v with the article's regression start; NaN before the w-th value of v's first finite run.
pub fn dema1(v: &[f64], w: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    let Some(s) = v.iter().position(|z| z.is_finite()) else { return out };
    if w < 2 || s + w > v.len() || v[s..s + w].iter().any(|z| !z.is_finite()) { return out; }
    let n = w as f64;
    let (mx, my) = ((n - 1.) / 2., v[s..s + w].iter().sum::<f64>() / n);
    let (mut sxy, mut sxx) = (0., 0.);
    for k in 0..w { let dx = k as f64 - mx; sxy += dx * (v[s + k] - my); sxx += dx * dx; }
    let slope = sxy / sxx;
    let icpt = my - slope * mx;
    let a = 2. / (n + 1.);
    let (mut e1, mut e2) = (icpt - (1. - a) / a * slope, icpt - 2. * (1. - a) / a * slope);
    for i in s + 1..v.len() {
        if !v[i].is_finite() { break; }
        e1 = a * v[i] + (1. - a) * e1;
        e2 = a * e1 + (1. - a) * e2;
        if i >= s + w - 1 { out[i] = 2. * e1 - e2; }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(2.) as usize;
    let we = period_ends(x, false);
    let wc: Vec<f64> = we.iter().map(|&i| x.c[i]).collect();
    let (f, s) = (dema1(&wc, g("fast", 12.)), dema1(&wc, g("slow", 26.)));
    let m: Vec<f64> = f.iter().zip(&s).map(|(a, b)| a - b).collect();
    let sg = dema1(&m, g("sig", 9.));
    let mut pos = Book::new();
    for k in 1..we.len() {
        let t = we[k];
        if t > to { break; }
        if t < from { continue; }
        let up = m[k] > sg[k] && m[k - 1] <= sg[k - 1];
        let dn = m[k] < sg[k] && m[k - 1] >= sg[k - 1];
        if pos.side() > 0. && dn { pos.exit(t, x.c[t], "MACD-DEMA1 crosses below its signal line"); }
        else if pos.side() == 0. && up && t < to { pos.enter(1., t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dema_matches_the_worked_start_and_trades_cross_over() {
        // the article's initialisation: u = 454.513, v = -6.162, a = 0.074 -> EMA1(0) = 531.62, EMA2(0) = 608.72
        let a: f64 = 2. / 27.;
        assert!((454.513 - (1. - a) / a * -6.162 - 531.62).abs() < 0.2 && (454.513 - 2. * (1. - a) / a * -6.162 - 608.72).abs() < 0.3);
        let v: Vec<f64> = (0..60).map(|i| 100. + i as f64).collect();
        let d = dema1(&v, 26);
        assert!(d[24].is_nan() && (d[59] - 159.).abs() < 1e-6, "a straight line is tracked exactly");
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scpmdm_macd", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let we = period_ends(&x, false);
        assert!(t.len() > 5 && t.iter().all(|z| z.dir > 0. && we.contains(&z.legs[0].i)));
    }
}
