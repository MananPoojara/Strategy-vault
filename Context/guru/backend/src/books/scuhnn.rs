//! Hans Uhlig, "Nearest Neighbor Prediction", Technical Analysis of Stocks & Commodities V.19:11 (2001) pp.56-61, with John
//! Sweeney's sidebar "Nearest neighbor by spreadsheet" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26
//! (1982 - 2008) (traders.com)/VOLUMES/V19/C11/129NEAR.pdf). Book id `scuhnn`. Lane PA2 (13 Nov 2026); independent check
//! authors/pa2/w/pa2_ref.py. Admission: sclang.rs (PA2_BOOKS).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::scdkdmi::at_close;
use super::sctraub::period_ends;

pub const REVIEWED: &[&str] = &["scuhnn_nnp"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Weekly closes (the last bar of each calendar week; the data's last bar counts only on a Friday). L = ln(weekly close). The pattern of week w is its four moves L(w) - L(w-1), L(w-1) - L(w-2), L(w-3) - L(w-4), L(w-4) - L(w-5) ('the weekly differences of the (natural) log of the closes of the previous first, second, fourth, and fifth weeks'); the distance between two patterns is the sum of the absolute differences of the four moves; the four nearest earlier patterns whose next move is known (weeks 5 .. w-1) are averaged: forecast = mean of their next moves L(k+1) - L(k) ('four nearest neighbors are good to start with'; equal distances keep the more recent week first, as the spreadsheet's date-descending sort does). Every earlier week of the data is a candidate ('only data older than the reference pattern can be used'). Unstated in the article: (1) what a negative forecast does - vault reading: short for the next week (the forecast is a direction, and NNP 'would have made a small gain' in a falling half-year where buy and hold lost; option short = 0: cash); (2) the fill - vault reading: the forecast week's closing price, held to the next weekly close and re-decided there (a zero forecast keeps the position). Simple average only (the distance-weighted refinement is not coded). The article prints a 26-week chart only, no trade table (S&P 500 weekly, 1980-2000). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scuhnn_nnp", name: "Uhlig nearest-neighbour prediction: the four most similar past weekly patterns forecast next week's direction, long or short at the weekly close", group: "S&C: nearest neighbor prediction (Uhlig, V.19:11)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scuhnn", rules: &[
            "F|\"To predict the pattern for any one week, I selected the weekly differences of the (natural) log of the closes of the previous first, second, fourth, and fifth weeks.\" (V.19:11, PDF p.2); sidebar: \"C5=LN(B5) ... D5=C9-C10 E5=C8-C9 F5=C6-C7 G5=C5-C6\"",
            "F|\"By calculating the sum of absolute differences of the logs of the index relative to the reference pattern, I can measure the similarity between two patterns.\" (PDF p.2); sidebar: \"H5=[ABS($D$5-D5)+ABS($E$5-E5)+ABS($F$5-F5)+ABS($G$5-G5)]\"",
            "E|\"I average the next moves of the nearest neighbors, which gives me an estimate of what the movement of my reference pattern (today's pattern) will be. I have found that four nearest neighbors are good to start with\" (PDF p.3); sidebar: \"I6=C5-C6 ... J5=(I6+I7+I8+I9)/4 ... The average is slightly positive (0.0032695), so the prognosis would be an upward move.\"",
            "X|The position is re-decided at every weekly close from the new forecast (PDF p.3, sidebar 'To go to the next week, fill in row 5'): a forecast of the other sign reverses it (vault reading; option short = 0: to cash)",
            "F|\"the database becomes smaller as you go further into the past, because only data older than the reference pattern can be used for nearest neighbor predictions\" (PDF p.4)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scuhnn_nnp", "Uhlig nearest-neighbour prediction (S&C)", "k")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "k", label: "Nearest neighbours averaged (article 4)", def: 4., min: 1., max: 20., step: 1. },
        ParamDef { key: "short", label: "1 = short on a negative forecast (vault reading); 0 = cash", def: 1., min: 0., max: 1., step: 1. }])
}

/// Forecast (mean next log move of the k nearest earlier patterns) at each weekly close; NaN where not enough history.
pub fn forecasts(c: &[f64], k: usize) -> Vec<f64> {
    let l: Vec<f64> = c.iter().map(|v| v.ln()).collect();
    let pat = |w: usize| [l[w] - l[w - 1], l[w - 1] - l[w - 2], l[w - 3] - l[w - 4], l[w - 4] - l[w - 5]];
    (0..l.len()).map(|w| {
        if w < 5 + k { return f64::NAN; }
        let r = pat(w);
        let mut d: Vec<(f64, usize)> = (5..w).map(|j| { let q = pat(j); ((0..4).map(|m| (r[m] - q[m]).abs()).sum::<f64>(), j) }).collect();
        d.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
        d[..k].iter().map(|&(_, j)| l[j + 1] - l[j]).sum::<f64>() / k as f64
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let k = p.get("k").copied().unwrap_or(4.).round().max(1.) as usize;
    let dn = if p.get("short").copied().unwrap_or(1.) > 0.5 { -1. } else { 0. };
    let ends = period_ends(x, false);
    let wc: Vec<f64> = ends.iter().map(|&i| x.c[i]).collect();
    let f = forecasts(&wc, k);
    let mut want = vec![f64::NAN; x.c.len()];
    for (w, &i) in ends.iter().enumerate() { if f[w].is_finite() && f[w] != 0. { want[i] = if f[w] > 0. { 1. } else { dn }; } }
    Some(at_close(x, from, to, "forecast turned", |t, side| if want[t].is_nan() { side } else { want[t] }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nearest_neighbours_and_weekly_positions() {
        // a pattern that repeats exactly: its nearest neighbour is the same phase, so the forecast is that phase's next move
        let c: Vec<f64> = (0..60).map(|i| 100. * (1f64 + 0.01 * [1., -2., 3., 0.5, -1.5][i % 5]).powi(1 + (i / 5) as i32 % 3)).collect();
        let f = forecasts(&c, 1);
        assert!(f[..6].iter().all(|v| v.is_nan()) && f[6..].iter().all(|v| v.is_finite()));
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scuhnn_nnp", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let ends = period_ends(&x, false);
        assert!(t.len() > 10 && t.iter().all(|z| ends.contains(&z.legs[0].i) && (z.open || ends.contains(&z.exit_i))));
        assert!(run("scuhnn_nnp", &x, 0, r.len() - 1, &Params::from([("short".into(), 0.)])).unwrap().iter().all(|z| z.dir > 0.));
    }
}
