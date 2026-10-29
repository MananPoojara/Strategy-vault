//! Gunter Meissner, Albin Alex and Kai Nolte, "A Refined MACD Indicator - Evidence against the Random Walk Hypothesis?"
//! (article; book id `gmrm`; PDF pages). Reading: claude-continuation-20261020/authors/meissner (admission/rm_ref.py).
//! Live: model MACDR1 (PDF 5) - trade at the close of the third day after a MACD / signal-line crossing when no new
//! crossing came on days 2 and 3, close the trade at a 3% (or 5%) gain or at the next crossing; MACDR2 (PDF 5-6, the
//! day-3 gap between MACD and signal line at least x% of the price) is the `gap` option. Long and short.
//! Book: NASDAQ-100 stocks (rules derived on the DJIA, 1989-1999); indices, ETFs and US/India stocks here are a declared
//! application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use super::tilma::ema;
/// Admitted ids (authors/meissner/admission/rm_ref.py (header)).
pub const REVIEWED: &[&str] = &["gmrm_macdr"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] MACD = EMA(12) - EMA(26) of closes, signal = EMA(9) of MACD, with the article's EMA recurrence (eq. 1), each started at the first value the vault holds. Crossing up = MACD above the signal today after at or below yesterday (down: the mirror); the crossing day is day 1. The trade opens at the close of day 3 if neither day 2 nor day 3 has a crossing (MACDR2: and |MACD - signal| on day 3 is at least gap% of the close). It closes at the first later close that is 3% (target) beyond the entry, or at the close of the next crossing, whichever comes first; that crossing is day 1 of the next signal. One position at a time. The study uses daily closes only, so targets are checked on closes. A position still open on the last bar is marked at its close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "gmrm_macdr", name: "Refined MACD (MACDR1): enter on day 3 of an unbroken crossing, 3% target or next crossing", group: "Meissner refined MACD",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "gmrm",
        rules: &["F|\"EMAt = (Pt K - EMAt-1 K) + EMAt-1 ... K = 2 / (number of periods + 1)\" (PDF 4); MACD(12,26) with a 9-day signal line",
            "E|\"the trading signal is given three days after the actual crossing if the trend is still intact. Thus, the position is opened at the closing price of the third day after the crossing, if no crossing has appeared on day 2 and 3.\" (MACDR1, PDF 5)",
            "X|\"the models gives a signal to close an open position when a 3% or 5% target gain has been reached or if another crossing occurs before the target is reached.\" (PDF 5)",
            "F|MACDR2: \"The difference between the averages is 1, which is 1% of the stock price. This would generate a trading signal for crossing-levels bigger or equal than 1%.\" (PDF 5-6): option gap"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "gmrm_macdr" => Some(("gmrm_macdr", "Meissner refined MACD", "MACDR1, 3% target")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "gmrm_macdr" => Some(vec![p("side", "Sides: 0 = long and short (article), 1 = long only, 2 = short only", 0., 0., 2., 1.),
            p("target", "Target gain % (article 3 or 5)", 3., 0.5, 20., 0.5),
            p("gap", "MACDR2 minimum day-3 gap, % of price (0 = MACDR1)", 0., 0., 5., 0.25)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (side, tg, gap) = (g("side").round() as i64, g("target") / 100., g("gap") / 100.);
    let c = &x.c;
    let (e12, e26) = (ema(c, 12), ema(c, 26));
    let m: Vec<f64> = e12.iter().zip(&e26).map(|(a, b)| a - b).collect();
    let s = ema(&m, 9);
    let dd: Vec<f64> = m.iter().zip(&s).map(|(a, b)| a - b).collect();
    let cross = |t: usize| -> f64 { if t == 0 { 0. } else if dd[t] > 0. && dd[t - 1] <= 0. { 1. } else if dd[t] < 0. && dd[t - 1] >= 0. { -1. } else { 0. } };
    let mk = |d: f64, ei: usize, xi: usize, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: ei, px: c[ei], w: 1. }], exit_i: xi, exit_px: c[xi],
        reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<(f64, usize)>);
    for t in from.max(3)..=to {
        if let Some((d, ei)) = cur {
            if d * (c[t] / c[ei] - 1.) >= tg { out.push(mk(d, ei, t, "target", false)); cur = None; }
            else if cross(t) != 0. { out.push(mk(d, ei, t, "next crossing", false)); cur = None; }
        }
        let d = cross(t - 2);
        if cur.is_none() && t < to && d != 0. && cross(t - 1) == 0. && cross(t) == 0. && dd[t].abs() >= gap * c[t]
            && ((d > 0. && side != 2) || (d < 0. && side != 1)) { cur = Some((d, t)); }
    }
    if let Some((d, ei)) = cur { out.push(mk(d, ei, to, "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hand_checked_day3_entry_and_target() {
        // A long fall then a steady rise: MACD crosses above its signal line once (an early short from the start is ignored here); the long opens two bars after that
        // crossing and closes at the first close 3% above the entry.
        let mut cl: Vec<f64> = (0..60).map(|i| 200. - i as f64).collect();
        cl.extend((1..30).map(|i| 141. + i as f64));
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2020-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: c, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("gmrm_macdr", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let (a, b) = (ema(&x.c, 12), ema(&x.c, 26));
        let m: Vec<f64> = a.iter().zip(&b).map(|(p, q)| p - q).collect();
        let dd: Vec<f64> = m.iter().zip(&ema(&m, 9)).map(|(p, q)| p - q).collect();
        let k = (1..dd.len()).find(|&i| dd[i] > 0. && dd[i - 1] <= 0.).unwrap();
        let l = t.iter().find(|z| z.dir > 0.).unwrap();
        assert_eq!((l.legs[0].i, l.reason.as_str()), (k + 2, "target"));
        let e = l.legs[0].px;
        assert!(cl[l.exit_i] >= e * 1.03 && cl[l.exit_i - 1] < e * 1.03);
        assert!(run("gmrm_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}
