//! Robert A. Levy, "The Predictive Significance of Five-Point Chart Patterns", Journal of Business 44(3), 1971,
//! p.316-323 (research share USB HDD 2/tRADING KNO-2/R.A.Levy - The predictive significance of Five-Point chart
//! patterns/Levy71.pdf, OCR). Book id `ralfp`. One rule: breakouts of five-point reversal patterns, held a fixed time.
//! The author's own conclusion is that no pattern beats the market after costs (a study, not a recommended system).
//! Lane K6 (titles loop, 31 Oct 2026); independent check authors/titles-1030/K6w/k6_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K6w/k6_ref.py (header)).
pub const REVIEWED: &[&str] = &["ralfp_five_point"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Author's warning] Levy's own result: after 2% round-trip costs no five-point pattern did better than the market (548 NYSE stocks, July 1964 - July 1969, 9,383 breakouts); this is his study, not a recommended system, and no claim is shown. [Declared readings] Daily closes. V = the average of the absolute day-to-day percentage changes over the latest 131 days (a signed average would be near zero); a reversal point is the highest (lowest) close before a fall (rise) of at least (a + b x V)% with a = 0, b = 6, V taken on the day the move is completed; a point counts only from the day it is confirmed, so nothing looks ahead. Each newly confirmed point with the four before it forms a five-point pattern; a breakout is the first close beyond the fourth point (above it when it is a high: buy; below it when it is a low: sell short) before the next point is confirmed, so all 32 patterns are traded (the pattern number is not filtered). Entry at the breakout day's close, exit at the close 20 trading days later (4 weeks; the paper also reports 1, 13 and 26 weeks: options). One position at a time; breakouts during a trade or on its exit day are ignored. Option: long only. The paper used adjusted closes of NYSE stocks; indices, ETFs and US/India stocks are a declared application; vault US closes are not dividend-adjusted.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "ralfp_five_point", name: "Five-point pattern breakout (Levy 1971 study): beyond the fourth reversal point, held 4 weeks", group: "Levy Five-Point Chart Patterns",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "ralfp", rules: &[
            "F|\"a reversal point is defined as the highest (lowest) price preceding a cumulative price decline (advance) of some percentage increment\"; increment = a + bV, a = 0, b = 6, V = average of the day-to-day percentage price changes over the most recent 131 days (p.317)",
            "F|Every five consecutive reversal points are ranked and classified into one of 32 patterns (Fig. 1, p.318)",
            "E|A signal requires a 'chart breakout' before the next reversal point: the price penetrates the fourth point, upside if it was a high (buy), downside if it was a low (sell) (p.318-319)",
            "X|Hold for 1, 4, 13 or 26 weeks after the breakout (Table 1, p.320-321)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "ralfp_five_point").then_some(("ralfp_five_point", "Levy five-point patterns", "b = 6, 131 days, 4 weeks"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "ralfp_five_point").then(|| vec![p("hold", "Holding period, trading days (4 weeks = 20; paper also 5, 65, 130)", 20., 1., 260., 1.),
        p("b", "b, increment = b x V (paper 6)", 6., 1., 20., 0.5), p("side", "0 = both directions, 1 = long only", 0., 0., 1., 1.)])
}

/// Breakout signals per bar: +1 upside, -1 downside, 0 none (closes only, no look-ahead).
pub fn breakouts(c: &[f64], b: f64) -> Vec<i8> {
    let n = c.len();
    let mut sig = vec![0i8; n];
    if n <= 132 { return sig; }
    let ch: Vec<f64> = (0..n).map(|i| if i > 0 { (c[i] / c[i - 1] - 1.).abs() * 100. } else { 0. }).collect();
    let mut win: f64 = ch[1..=131].iter().sum();
    let (mut dir, mut hi, mut lo) = (0i8, c[131], c[131]);
    let mut pts: Vec<(f64, bool)> = vec![]; // (close, is a high)
    let mut armed: Option<(f64, i8)> = None; // (fourth point, breakout direction)
    for i in 132..n {
        win += ch[i] - ch[i - 131];
        let th = b * (win / 131.) / 100.;
        let mut new_pt = None;
        match dir {
            1 => if c[i] > hi { hi = c[i]; } else if c[i] <= hi * (1. - th) { new_pt = Some((hi, true)); dir = -1; lo = c[i]; },
            -1 => if c[i] < lo { lo = c[i]; } else if c[i] >= lo * (1. + th) { new_pt = Some((lo, false)); dir = 1; hi = c[i]; },
            _ => {
                hi = hi.max(c[i]); lo = lo.min(c[i]);
                if c[i] <= hi * (1. - th) { new_pt = Some((hi, true)); dir = -1; lo = c[i]; }
                else if c[i] >= lo * (1. + th) { new_pt = Some((lo, false)); dir = 1; hi = c[i]; }
            }
        }
        if let Some(pt) = new_pt {
            pts.push(pt);
            let k = pts.len();
            armed = (k >= 5).then(|| (pts[k - 2].0, if pts[k - 2].1 { 1 } else { -1 }));
        }
        if let Some((p4, d)) = armed {
            if (d > 0 && c[i] > p4) || (d < 0 && c[i] < p4) { sig[i] = d; armed = None; }
        }
    }
    sig
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (hold, long_only) = (g("hold").round().max(1.) as usize, g("side").round() >= 1.);
    let c = &x.c;
    let sig = breakouts(c, g("b"));
    let (mut out, mut free) = (vec![], from);
    for i in from..=to {
        if i < free || sig[i] == 0 || (long_only && sig[i] < 0) { continue; }
        let xi = i + hold;
        out.push(if xi <= to { mk(sig[i] as f64, i, c[i], "close", xi, c[xi], "holding period", false, xi) } else { mk(sig[i] as f64, i, c[i], "close", to, c[to], "open", true, to) });
        free = xi + 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn reversal_points_and_a_breakout_of_the_fourth() {
        // 132 bars alternating +-1% (V = 1%, increment 6%), then swings: 100 -> 110 -> 102 -> 112 -> 104 -> above 112
        let mut c: Vec<f64> = (0..132).map(|i| if i % 2 == 0 { 100. } else { 101. }).collect();
        for &t in &[110., 102., 112., 104., 113.] { let s = *c.last().unwrap(); for k in 1..=4 { c.push(s + (t - s) * k as f64 / 4.); } }
        let sig = breakouts(&c, 6.);
        let ups: Vec<usize> = (0..c.len()).filter(|&i| sig[i] != 0).collect();
        // points: low 101 (bar 131), high 110, low 102, high 112, low 104 (confirmed on the way up to 113);
        // the fourth point is the high 112 -> buy on the first close above 112
        assert_eq!(ups.len(), 1);
        assert!(sig[ups[0]] == 1 && c[ups[0]] > 112. && c[ups[0] - 1] <= 112.);
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{i}"), open: v, high: v, low: v, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ralfp_five_point", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert!(t[0].open && t[0].legs[0].i == ups[0]);
        assert!(run("ralfp_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Robert A. Levy (five-point patterns) 2026-10-31 (titles lane K6).
// Admission only for the module REVIEWED lists (authors/titles-1030/K6w/k6_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "ralfp" && crate::books::ralfp::reviewed(def.id),
    label: "Robert Levy's 1971 study of 548 NYSE stocks (1964-69), whose own result is no edge after costs; indices, ETFs and US/India stocks are a declared application.",
    basis: "Robert A. Levy, The Predictive Significance of Five-Point Chart Patterns, Journal of Business 44(3), 1971, p.316-323 (OCR)",
};
