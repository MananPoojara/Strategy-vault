//! Jeremy G. Konstenius, "Combining Trend And Oscillator Signals", Technical Analysis of Stocks & Commodities V.14:12 (1996)
//! p.520-525 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C12/
//! COMBINI.pdf). Book id `konwv`, author Jeremy G. Konstenius. Lane PB (13 Nov 2026, pool B row 408 of authors/a1/A1.tsv);
//! independent check authors/pb/w/pb_ref2.py. The "wave 1" model: one method, printed with yen and T-bond parameter sets.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::mk;

pub const REVIEWED: &[&str] = &["konwv_yen", "konwv_bond"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Literal rules] Book check (#168): yen passes on both printed windows (verified); T-bond fails (28 trades vs 20, 35.7% vs 60%), so the T-bond card stays unverified. Weekly bars: Monday-Friday weeks of the Norgate continuous contract, back-adjusted (the article: 'nearest continuous back-adjusted' weekly data), signals on the week's close, orders at the next week's first open ('tomorrow's opening' on a weekly chart). avg = simple average of the last Length1 weekly closes; slope = LinearRegSlope(Close, Length2), the least-squares slope of the last Length2 weekly closes; trigger = LinearRegValue(slope, Length3, 0), the least-squares line of the last Length3 slopes evaluated at the current week. Long entry: close > avg and slope > trigger; long exit: close crosses below avg or slope crosses below trigger; short entry: close < avg and slope < trigger; short exit: close crosses above avg or slope crosses above trigger (p.521-522, EasyLanguage for SuperCharts). A week that exits one side and qualifies for the other reverses at the same open. Optimised values printed: yen 30 / 5 / 50 weeks, T-bonds 20 / 10 / 30 weeks (in-sample 4/11/1986-4/5/1991, out-of-sample to 4/5/1996, $50 commission a round turn). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor.";

pub fn catalogue() -> Vec<StrategyDef> {
    let rules: &'static [&'static str] = &[
        "F|\"Close>Average(Close,Length1) And LinearRegSlope(Close,Length2) > LinearRegValue(LinearRegSlope(Close,Length2),Length3,0)\" (buy rule, p.521)",
        "E|\"If the close is greater than the average and the linear regression slope of the closing price is greater than the trigger line ... then buy at tomorrow's opening\"; short entry: Close<Average(Close,Length1) And LinearRegSlope(Close,Length2) < LinearRegValue(...) (p.521-522)",
        "X|\"If the close crosses below the average or the linear regression slope line crosses below the trigger line, then exit the long position at tomorrow's opening\"; the short exit mirrors it with 'crosses above' (p.521-522)"];
    let d = |id, name| StrategyDef { id, name, group: "S&C: Combining Trend And Oscillator Signals (Konstenius, V.14:12)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "konwv", rules, notes: NOTE,
        claim: if id == "konwv_yen" { crate::connors::claim("21", "57%", "$1,803.57 per contract", "7 weeks winners / 3 losers", "Yen futures weekly 04/11/86-04/05/91 (30/5/50)") }
            else { crate::connors::claim("20", "60%", "$2,987.50 per contract", "8 weeks winners / 5 losers", "T-bond futures weekly 04/11/86-04/05/91 (20/10/30)") } };
    vec![d("konwv_yen", "Konstenius wave 1 on yen futures: 30-week average + 5-week regression slope vs its 50-week trigger"),
         d("konwv_bond", "Konstenius wave 1 on T-bond futures: 20-week average + 10-week regression slope vs its 30-week trigger")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("konwv_yen", "Konstenius wave 1 (trend + regression-slope oscillator)", if id == "konwv_yen" { "Yen 30/5/50" } else { "T-bond 20/10/30" }))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let (a, b, c) = match id { "konwv_yen" => (30., 5., 50.), "konwv_bond" => (20., 10., 30.), _ => return None };
    Some(vec![p("len1", "Length1: weeks in the average", a, 2., 80., 1.), p("len2", "Length2: weeks in the regression slope", b, 2., 40., 1.), p("len3", "Length3: weeks in the trigger regression", c, 2., 80., 1.)])
}

/// Least-squares line through v[i+1-n..=i] (x = 0..n-1): (slope, fitted value at x = n-1). NaN when a value is missing.
pub fn linreg(v: &[f64], i: usize, n: usize) -> (f64, f64) {
    if n < 2 || i + 1 < n { return (f64::NAN, f64::NAN); }
    let w = &v[i + 1 - n..=i];
    let nf = n as f64;
    let (sx, sy) = (nf * (nf - 1.) / 2., w.iter().sum::<f64>());
    let sxy: f64 = w.iter().enumerate().map(|(k, y)| k as f64 * y).sum();
    let sxx = (nf - 1.) * nf * (2. * nf - 1.) / 6.;
    let b = (nf * sxy - sx * sy) / (nf * sxx - sx * sx);
    let a = (sy - b * sx) / nf;
    (b, a + b * (nf - 1.))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let schema = params(id)?;
    let u = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def).round().max(2.) as usize;
    let (l1, l2, l3) = (u("len1"), u("len2"), u("len3"));
    let wk: Vec<usize> = super::apta::bars(x, 1).into_iter().filter(|&i| i <= to).collect();
    let c: Vec<f64> = wk.iter().map(|&i| x.c[i]).collect();
    let n = c.len();
    let avg: Vec<f64> = (0..n).map(|k| if k + 1 >= l1 { c[k + 1 - l1..=k].iter().sum::<f64>() / l1 as f64 } else { f64::NAN }).collect();
    let slope: Vec<f64> = (0..n).map(|k| linreg(&c, k, l2).0).collect();
    let trig: Vec<f64> = (0..n).map(|k| linreg(&slope, k, l3).1).collect();
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>);
    for k in 1..n {
        let i = wk[k];
        if i < from || i + 1 > to { continue; } // the order fills at the next bar's open, inside the window
        if !(avg[k].is_finite() && avg[k - 1].is_finite() && trig[k].is_finite() && trig[k - 1].is_finite()) { continue; }
        let below = |a: f64, b: f64, a1: f64, b1: f64| a < b && a1 >= b1; // a crosses below b
        let exit_long = below(c[k], avg[k], c[k - 1], avg[k - 1]) || below(slope[k], trig[k], slope[k - 1], trig[k - 1]);
        let exit_short = below(avg[k], c[k], avg[k - 1], c[k - 1]) || below(trig[k], slope[k], trig[k - 1], slope[k - 1]);
        let long_in = c[k] > avg[k] && slope[k] > trig[k];
        let short_in = c[k] < avg[k] && slope[k] < trig[k];
        let (f, px) = (i + 1, x.o[i + 1]);
        let mut d = cur.map_or(0., |c| c.0);
        if (d > 0. && exit_long) || (d < 0. && exit_short) {
            let (dd, e, ep) = cur.take().unwrap();
            out.push(mk(dd, e, ep, "open", f, px, if dd > 0. { "close or slope crossed below" } else { "close or slope crossed above" }, false, i));
            d = 0.;
        }
        if d == 0. && (long_in || short_in) { cur = Some((if long_in { 1. } else { -1. }, f, px)); }
    }
    if let Some((d, e, ep)) = cur { out.push(mk(d, e, ep, "open", to, x.c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn regression_line_and_weekly_reversal() {
        let (b, v) = linreg(&[1., 3., 5., 7.], 3, 4);
        assert!((b - 2.).abs() < 1e-12 && (v - 7.).abs() < 1e-12);
        assert!(linreg(&[1., 2.], 0, 2).0.is_nan());
        // two bars a week (Thursday, Friday): a rise then a fall; Length 2/2/2
        let px: Vec<f64> = vec![10., 10., 11., 11., 13., 13., 16., 16., 14., 14., 11., 11., 9., 9., 8., 8.];
        let r: Vec<Row> = px.iter().enumerate().map(|(k, &c)| { let f = crate::books::apta::tests_friday(k / 2); let d: u32 = f[8..].parse().unwrap();
            Row { ticker: "Japanese Yen (Norgate &6J)".into(), date: if k % 2 == 0 { format!("{}{:02}", &f[..8], d - 1) } else { f }, open: c, high: c, low: c, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::from([("len1".to_string(), 2.), ("len2".to_string(), 2.), ("len3".to_string(), 3.)]);
        // weekly closes 10 11 13 16 14 11 9 8: week 4 closes below its 2-week average with the slope (-2) under its trigger (-1)
        // -> short at week 5's first open (bar 10); week 5's slope (-3) crosses above its trigger (-3.67) -> cover at bar 12
        let t = run("konwv_yen", &x, 0, r.len() - 1, &p).unwrap();
        assert_eq!(t.iter().map(|z| (z.dir, z.legs[0].i, z.exit_i, z.open)).collect::<Vec<_>>(), vec![(-1., 10, 12, false)]);
        assert!(t.iter().all(|z| z.entry_kind == "open" && z.legs[0].px == x.o[z.legs[0].i] && z.exit_px == x.o[z.exit_i]));
        assert!(run("konwv_x", &x, 0, 5, &p).is_none());
    }
}

// Admission: a futures card (futures::CARDS_PB, policy requirement "futures" via futures::card_basis), like lane F3's cards.
