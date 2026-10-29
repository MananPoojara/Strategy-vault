//! Timothy W. Hayes, "The Research Driven Investor" (McGraw-Hill 2001, html e-book; book id `thrdi`; printed pages).
//! Reading: claude-continuation-20261020/authors/thayes (admission/rdi_ref.py). Live: the reverse-bracket rule of
//! Fig 4-9 (p.72-73, 75): the 30-week rate of change rises back above 3% after falling below it = buy; it falls back
//! below 15% after rising above it = sell into cash (commercial paper). Held: Fig 5-2 (p.81-82, the kind of 42-week
//! 'smoothing' is not stated and it is half of an Industrials / Transports confirmation), Fig 5-27 (p.115-116, text
//! 103 / 97 against the chart's 103.5 / 96.5, fixed-horizon returns with repeats inside 60 days dropped, no exit).
//! Book: the Value Line Composite's ROC times the S&P 500 (weekly 1968-2000); here the ROC of the traded instrument
//! itself, on indices, ETFs and US/India stocks, is a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use super::swsp::week_ends;
/// Admitted ids (authors/thayes/admission/rdi_ref.py (header)).
pub const REVIEWED: &[&str] = &["thrdi_roc30_bracket"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly closes (a week's close is its last trading day). ROC = this week's close / the close 30 weeks earlier - 1, in percent. The lower bracket is armed when the ROC is below 3% and the buy is the first later week it is above 3%; the upper bracket is armed when the ROC is above 15% and the sell is the first later week it is below 15% (p.72-73); each signal disarms its bracket, and a signal while already in that state is ignored. Long only: a sell signal switches to cash, as the book's chart does ('S = Switch into Commercial Paper', p.75); no interest is credited on cash. Trades at the signal week's close. The book reads the Value Line Composite to time the S&P 500; the vault does not hold the Value Line index, so the ROC is taken on the traded instrument (declared application). Signals are tracked from the first week the vault holds; trades open only inside the test window; a position still open on the last bar is marked at its close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "thrdi_roc30_bracket", name: "30-week rate of change reverse brackets: buy back above 3%, sell back below 15%", group: "Hayes Research Driven Investor",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "thrdi",
        rules: &["F|\"indicators can also be developed based on reversals in which the data moves through a fixed band from one direction, reverses, and moves back through the level in the other direction, producing the signal. Called reverse bracket indicators by NDR\" (p.72)",
            "E|\"30-week rate of change of the Value Line Composite ... signals a reversal from an oversold condition when the rate of change drops below 3 percent and rises back above that level\" (p.73); chart: \"Rises Above Lower Bracket = Buy\" (Fig 4-9)",
            "X|\"signals a reversal from an overbought condition when the 30-week rate of change of the Value Line Composite rises above 15 percent and then drops below that level\" (p.72-73); \"Falls Below Upper Bracket = Sell\", \"S = Switch into Commercial Paper\" (Fig 4-9); \"instead of shorting, you could have the indicator respond to sell signals by switching into cash ... the approach taken in Figure 4-9\" (p.75)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "thrdi_roc30_bracket" => Some(("thrdi_roc30_bracket", "Hayes 30-week ROC reverse brackets", "3% / 15%")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "thrdi_roc30_bracket" => Some(vec![p("weeks", "Rate of change, weeks (book 30)", 30., 2., 104., 1.),
            p("lo", "Lower bracket, % (book 3)", 3., -30., 30., 0.5),
            p("hi", "Upper bracket, % (book 15)", 15., -30., 60., 0.5)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (n, lo, hi) = (g("weeks").round().max(1.) as usize, g("lo"), g("hi"));
    let we: Vec<usize> = week_ends(x).into_iter().filter(|&i| i <= to).collect();
    let c = &x.c;
    let mk = |ei: usize, xi: usize, why: &str, open: bool| Trade { dir: 1., legs: vec![Leg { i: ei, px: c[ei], w: 1. }],
        exit_i: xi, exit_px: c[xi], reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur, mut arm_lo, mut arm_hi) = (vec![], None::<usize>, false, false);
    for k in n..we.len() {
        let i = we[k];
        let roc = (c[i] / c[we[k - n]] - 1.) * 100.;
        if arm_lo && roc > lo { arm_lo = false; if cur.is_none() && i >= from && i < to { cur = Some(i); } }
        if arm_hi && roc < hi { arm_hi = false; if let Some(e) = cur.take() { out.push(mk(e, i, "ROC back below the upper bracket", false)); } }
        if roc < lo { arm_lo = true; }
        if roc > hi { arm_hi = true; }
    }
    if let Some(e) = cur { out.push(mk(e, to, "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hand_checked_brackets() {
        // one bar per Friday; ROC over 2 weeks with brackets 3 / 15 (params)
        let cl = [100., 100., 100., 101., 104., 118., 125., 125., 126., 120.];
        // ROC(2) from week 2: 0, 1, 4 (buy: armed at 0 and 1), 16.8 (arm hi), 20.2, 5.9 (sell), 0.8, -4
        let base = super::super::rgcts::days("2001-01-05");
        let r: Vec<Row> = cl.iter().enumerate().map(|(k, &c)| {
            let z = base + 7 * k as i64; // 2001-01-05 + k weeks, all inside 2001
            let (m, d) = { let doy = z - super::super::rgcts::days("2001-01-01"); let ml = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
                let (mut m, mut d) = (0, doy); while d >= ml[m] { d -= ml[m]; m += 1; } (m + 1, d + 1) };
            Row { ticker: "T".into(), date: format!("2001-{m:02}-{d:02}"), open: c, high: c, low: c, close: c, volume: None }
        }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let mut pr = Params::new(); pr.insert("weeks".into(), 2.);
        let t = run("thrdi_roc30_bracket", &x, 0, r.len() - 1, &pr).unwrap();
        let got: Vec<_> = t.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px, z.open)).collect();
        assert_eq!(got, vec![(4, 104., 7, 125., false)]);
        assert!(run("thrdi_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}
