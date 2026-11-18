//! Dennis Meyers, "4th Order Polynomial System on S&P500 Futures 5min Bars" (Meyers Analytics, 1999, 11 pp.; research share Trading
//! Knowledge III/Dennis Meyers Publications/Dennis Meyers - 4th Order Polynomial System on SP500 5min Bar Futures.pdf). Book id `dmpoly`.
//! Lane CV4 (18-19 Nov 2026; R4 #339/#340). Rules p.1-2 (article pages): a least-squares 4th-order polynomial through the last T 5-minute
//! closes, forecast one bar ahead (Pf); buy when Pf rises more than pctup from its lowest low recorded while short, sell when it falls more
//! than pctdn from its highest high recorded while long; all trades closed at the day's close. Defaults T 60, pctup 0.90, pctdn 0.75 =
//! Figure 1 (p.4). 5-minute bars are built from the vault's 1-minute files (intraday_ep::day_bars), SPX and SPY only.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::day_trade;

pub const INTRA: &[&str] = &["dmpoly4_spx5"];
pub fn reviewed(id: &str) -> bool { INTRA.contains(&id) }

const NOTE: &str = "[Declared adaptation, 4 unstated details (R4 #339), UNVERIFIED] The article trades the S&P Sep/99 futures on 5-minute bars; here the real SPX (or SPY) 1-minute file is cut into 5-minute bars from the session open. (1) 'buy ... at the market' on the bar whose Pf trips the rule is filled at the NEXT 5-minute bar's open (TradeStation convention); a trip on the day's last bar has no fill. (2) The least-squares window is one continuous series of closes across days (the article's chart is one continuous 5-minute series): the first Pf of a day uses yesterday's last closes. (3) Every day starts flat with no remembered high or low of Pf: from the first bar with T closes the lowest and highest Pf of the day are tracked, and whichever of 'up pctup from the low' or 'down pctdn from the high' comes first opens the position; after that the rule is stop-and-reverse ('lowest low recorded in Pf while short', 'highest high while long'), the tracked extreme restarting at the Pf of the reversing bar. (4) 'moves up by more than pctup' is read as Pf > low x (1 + pctup/100), and down as Pf < high x (1 - pctdn/100). The fit is solved on a centred, scaled time axis (same polynomial, better conditioned). The defaults are the article's walk-forward parameters for 7 Jul-5 Aug 1999, not re-optimised; the article's results are on the futures, the card runs on the cash index and ETF.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dmpoly4_spx5", name: "Meyers 4th-order polynomial: Pf up pctup from its low buys, down pctdn from its high sells, out at the close (5-minute SPX/SPY)", group: "Dennis Meyers: 4th Order Polynomial System (S&P 5-minute)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "dmpoly", rules: &[
            "F|\"fit a 4th order polynomial curve of the form a0+b0*t+c0*t2+d0*t3+e0*t4 ... the last T bars of SP 5minute close prices. Pf is then constructed ... Pf = a0 + b0*(T+1) + c0*(T+1)2 + d0*(T+1)3 + e0*(T+1)4\" (p.1-2)",
            "E|\"Buy Rule: IF Pf has moved up by more than the percentage amount of pctup from the lowest low recorded in Pf while short then buy one S&P Sep/99 contract at the market. Sell Rule: IF Pf has moved down by more than the percentage amount pctdn from the highest high recorded in Pf while long then sell\" (p.2)",
            "X|\"Exit Rule: Exit all trades at close of each day with a market on close order.\" (p.2); parameters Figure 1: T 60 bars, pctup 0.90%, pctdn 0.75% (p.4)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("dmpoly4_spx5", "Meyers 4th-order polynomial (5-minute)", "T 60, pctup 0.90, pctdn 0.75"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("t", "T, 5-minute closes in the least-squares fit (article Figure 1: 60)", 60., 10., 200., 5.),
        p("up", "pctup, % rise of Pf from its low (Figure 1: 0.90)", 0.90, 0.05, 3., 0.05),
        p("dn", "pctdn, % fall of Pf from its high (Figure 1: 0.75)", 0.75, 0.05, 3., 0.05)])
}

/// Value at one step past the last point of the least-squares polynomial of degree 4 through y (equal steps), as the article's Pf.
/// Normal equations on u = (k - m) / m (m = (n-1)/2) with y centred on its mean, Gaussian elimination with partial pivoting.
pub fn forecast(y: &[f64]) -> f64 {
    let n = y.len();
    let m = (n as f64 - 1.) / 2.;
    let mean = y.iter().sum::<f64>() / n as f64;
    let (mut s, mut b) = ([0f64; 9], [0f64; 5]);
    for (k, &v) in y.iter().enumerate() {
        let u = (k as f64 - m) / m;
        let mut pw = 1.;
        for q in 0..9 { s[q] += pw; if q < 5 { b[q] += pw * (v - mean); } pw *= u; }
    }
    let mut a = [[0f64; 6]; 5];
    for r in 0..5 { for c in 0..5 { a[r][c] = s[r + c]; } a[r][5] = b[r]; }
    for c in 0..5 {
        let piv = (c..5).max_by(|&i, &j| a[i][c].abs().partial_cmp(&a[j][c].abs()).unwrap()).unwrap();
        a.swap(c, piv);
        for r in c + 1..5 { let f = a[r][c] / a[c][c]; for k in c..6 { a[r][k] -= f * a[c][k]; } }
    }
    let mut co = [0f64; 5];
    for r in (0..5).rev() { let t: f64 = (r + 1..5).map(|c| a[r][c] * co[c]).sum(); co[r] = (a[r][5] - t) / a[r][r]; }
    let u = (n as f64 - m) / m;
    mean + (0..5).map(|q| co[q] * u.powi(q as i32)).sum::<f64>()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    if x.minutes.is_none() { return Some(vec![]); }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (tn, up, dn) = (g("t", 60.).round().max(10.) as usize, g("up", 0.90) / 100., g("dn", 0.75) / 100.);
    let mut buf: Vec<f64> = vec![]; // one continuous series of 5-minute closes, from a few days before the window
    let mut out = vec![];
    for i in from.saturating_sub(10)..=to {
        let Some((_, _, _, b)) = crate::intraday_ep::day_bars(x, i, 5) else { continue };
        let (mut pos, mut cur, mut pend): (Option<(f64, usize, f64)>, f64, f64) = (None, 0., 0.);
        let (mut lo, mut hi) = (f64::NAN, f64::NAN);
        for (j, z) in b.iter().enumerate() {
            if pend != 0. {
                if let Some((d, ei, ep)) = pos.take() { out.push(day_trade(d, ei, ep, i, z.o, "reversed", "open", false)); }
                pos = Some((pend, i, z.o));
                pend = 0.;
            }
            buf.push(z.c);
            let last = j + 1 == b.len();
            if i >= from && buf.len() >= tn && !last {
                let pf = forecast(&buf[buf.len() - tn..]);
                if cur >= 0. { hi = if hi.is_nan() { pf } else { hi.max(pf) }; }
                if cur <= 0. { lo = if lo.is_nan() { pf } else { lo.min(pf) }; }
                let want = if cur <= 0. && pf > lo * (1. + up) { 1. } else if cur >= 0. && pf < hi * (1. - dn) { -1. } else { 0. };
                if want != 0. {
                    cur = want; pend = want;
                    if want > 0. { hi = pf; lo = f64::NAN; } else { lo = pf; hi = f64::NAN; }
                }
            }
            if last { if let Some((d, ei, ep)) = pos.take() { out.push(day_trade(d, ei, ep, i, z.c, "end of day", "open", false)); } }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forecast_extends_a_quartic_exactly() {
        let f = |t: f64| 100. + 0.5 * t - 0.02 * t * t + 0.001 * t.powi(3) - 0.00001 * t.powi(4);
        let y: Vec<f64> = (1..=60).map(|t| f(t as f64)).collect();
        assert!((forecast(&y) - f(61.)).abs() < 1e-6);
        let lin: Vec<f64> = (1..=60).map(|t| 1300. + 2. * t as f64).collect();
        assert!((forecast(&lin) - (1300. + 2. * 61.)).abs() < 1e-6);
    }
    #[test]
    fn no_minutes_no_trades() {
        let r = super::super::sctraub::tests::bars(30);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("dmpoly4_spx5", &x, 0, r.len() - 1, &Params::new()).unwrap().is_empty());
        assert!(run("dmpoly_x", &x, 0, 1, &Params::new()).is_none());
    }
}
// Admission: the intraday block (intraday_ep::CV4, SPX and SPY only): the article trades the S&P 500 futures.
