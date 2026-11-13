//! Patrick G. Mulloy, "Smoothing Data With Faster Moving Averages", Technical Analysis of Stocks & Commodities V.12:1 (1994)
//! pp.11-19 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V12/C01/
//! SMOOTHI.pdf). Book id `scpmdm`: the MACD built from one-parameter double exponential moving averages (MACD-DEMA1 12/26/9) on
//! weekly closes. Lane PG (14 Nov 2026, pool A row i1773, A1 score <= 1); independent check authors/pg/w/pg_ref.py.
//! Admitted by books::scjrd::ADMISSION (PG_BOOKS).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{ema0, period_ends};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scpmdm_macd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; signal line decided by Figure 2 (REV-PG B1, lead ruling); unverified] Weekly closes = the close of each calendar week's last trading day (the data's last week counts only when it ends on a Friday). DEMA1(w) = 2 x EMA1 - EMA2 with alpha = 2/(w+1), EMA1 the EMA of the series and EMA2 the EMA of EMA1, both started from the article's regression initialisation: over the first w values u = intercept and v = slope of the least-squares line (x = 0..w-1), EMA1(0) = u - ((1 - alpha)/alpha) v, EMA2(0) = u - 2((1 - alpha)/alpha) v, then the recursions from the second value (PDF p.5); DEMA1 is used only from the w-th value on (no look-ahead). MACD-DEMA1 = DEMA1(12) - DEMA1(26) of the weekly closes. Buy when it crosses above its signal line (above this week, at or below last week); out when it crosses back below; long only (the article counts 'buy signals', their 'losses' and 'round-trips'). Unstated: (1) the signal line - the article says 'the filters are not EMAs' for all three lengths, but only an EMA(9) signal line reproduces Figure 2, so EMA(9) of the MACD-DEMA1 (alpha 2/10, started from its first value) is the default and DEMA1(9) is option sig_dema = 1. Calibration table, Norgate NASDAQ Composite weekly closes, Oct 1988 - Dec 1993 (the article's index and window; REV-PG and lane PG): standard EMA MACD 12/26/9 -> 12 buy signals, 6 losses (Figure 1: 12 buy signals, six whipsaws - the data, window and exit reading match); MACD-DEMA1 with an EMA(9) signal -> 9 trades, 2 losses, +429 points (Figure 2: 9 trades, 3 losses, 'fewer round-trips', 'almost twice the profit'); MACD-DEMA1 with a DEMA1(9) signal -> 18 trades, 8 losses, +298 points (more round-trips than the standard MACD: contradicts the caption); EMA lines with a DEMA1(9) signal -> 15 trades, 7 losses. (2) the fill - the signal week's close. The Figure 8 Parabolic SAR qualifier is an illustration without stated use and is not coded. The vault runs the rule on indices, ETFs and stocks, US and India (declared application; the NASDAQ Composite is not a vault ticker).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scpmdm_macd", name: "Mulloy MACD-DEMA1 (12, 26) with a 9-week signal line on weekly closes: buy when it crosses above, sell when it crosses back below", group: "S&C: smoothing data with faster moving averages (Mulloy, V.12:1)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scpmdm", rules: &[
            "F|\"DEMA1 = 2EMA1 - EMA2. Here, we are multiplying the single EMA by two and then subtracting the double EMA.\" with \"S1m = aCm + (1 - a)S1m-1, S2m = aS1m + (1 - a)S2m-1\" (V.12:1, PDF p.5)",
            "F|\"Calculate regression coefficients ... u = Intercept(x,y), v = Slope(x,y) ... Initialization equations: EMA1(0) = u - ((1-a)/a)(v), EMA2(0) = u - 2((1-a)/a)(v)\" (PDF p.5-6)",
            "E|\"In Figure 1, the MACD indicator is applied to the weekly closing price of the NASDAQ composite index ... Figure 2 uses the same filter lengths of 12, 26 and nine, but the filters are not EMAs but are derivations of one-parameter double exponential moving averages (DEMA1). This time, the indicator generated nine trades, with only three losses\" (PDF p.1)",
            "X|Out when the MACD-DEMA1 crosses back below its signal line, at that week's close (the article counts round-trips of buy signals)",
            "F|[Reading decided by Figure 2] signal line = EMA(9) of the MACD-DEMA1: on Norgate NASDAQ Composite weekly closes Oct 1988 - Dec 1993 it gives 9 trades / 2 losses (Figure 2: 9 / 3); a DEMA1(9) signal gives 18 / 8 (option sig_dema = 1)"],
        notes: NOTE, claim: crate::connors::claim("9", "67%", "", "", "Weekly NASDAQ Composite, Oct 1988 - 1993 (Figure 2, chart only)") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scpmdm_macd", "Mulloy MACD-DEMA1 (S&C)", "12/26/9 weekly")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "fast", label: "Fast DEMA1 weeks (article 12)", def: 12., min: 2., max: 100., step: 1. },
        ParamDef { key: "slow", label: "Slow DEMA1 weeks (article 26)", def: 26., min: 3., max: 200., step: 1. },
        ParamDef { key: "sig", label: "Signal line weeks (article 9)", def: 9., min: 2., max: 100., step: 1. },
        ParamDef { key: "sig_dema", label: "Signal line: 0 = EMA (reproduces Figure 2), 1 = DEMA1 (the text's 'not EMAs')", def: 0., min: 0., max: 1., step: 1. }])
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
    let sg = if p.get("sig_dema").copied().unwrap_or(0.) != 0. { dema1(&m, g("sig", 9.)) } else { ema0(&m, g("sig", 9.)) };
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
