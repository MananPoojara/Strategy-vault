//! Lane PC (13 Nov 2026; authors/pc/PC.tsv; R4 #297 "up to two unstated details"; pool C row of authors/a1/A1.tsv):
//! Phillip C. Holt, "Enhancing On-Balance And Negative Volume", Technical Analysis of Stocks & Commodities V.14:6 (1996)
//! pp.265-269 (research share 'Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C06/
//! ENHANCI.pdf'). Book id `scholt`.
//! Coded: the OBVDI and NVDI weekly systems (rules and MetaStock formulas PDF p.4, Excel sidebar PDF p.5).
//! Independent reference: authors/pc/w/pc_ref.py (written from the article, not from this file).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scholt_obvdi", "scholt_nvdi"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { " Weekly bars: each week's last trading day closes the week; weekly volume = the sum of that week's daily volumes. %B(x) = (x - lower) / (upper - lower) with a 33-week simple average and bands 2 population standard deviations (STDEVP) away; the disparity = (1 + %B of the close) / (1 + %B of the volume line) (sidebar: 'The integer 1 is added ... to avoid a [division by zero]'); a 4-week simple average of it gives the signals. Long only; out of the market after a sell. Holt tested the NYSE Composite 1977-96 (weekly; OBVDI 27 trades, 22 winners; NVDI 15 trades, 11 winners); here each instrument's own close and volume are used (declared application; the article's NYSE check is in the lane reference, Norgate $NYA 1977-96). Unstated in the article: when the trade is filled; vault reading: at the close of the signal week." } }
const OBV_NOTE: &str = concat!("[Adapted: 2 unstated details, R4 #297; unverified] OBVDI: on-balance volume (week's volume added on an up close, subtracted on a down close, unchanged on an equal close; starts at the first week's volume; a week without volume restarts the line). Buy when the 4-week average of OBVDI rises through 0.85 or through 0.95; sell when it falls through 0.95. Unstated: the printed 'rises through the 0.85 or 0.95 level' names two levels; vault reading: a rise through either level buys (options: 0.85 only, 0.95 only).", common!());
const NVI_NOTE: &str = concat!("[Adapted: 2 unstated details, R4 #297; unverified] NVDI: the negative volume line changes only in a week whose volume is below the previous week's. Holt's MetaStock formulas use MetaStock's NVI() ('substitute NVI() where OBV() appears'), which moves by the close's percentage change of its own value (multiplicative); the editor's Excel sidebar adds the percentage change in points (=IF(C3<C2,H2+((B3-B2)/B2)*100,H2)). Unstated: which form Holt's tests used; vault reading: the author's MetaStock NVI() (option: the Excel additive form). %B does not depend on the line's starting value (100 here). A week without volume (any day with no volume), and the week after it, leave the line unchanged (R4 #320). Buy when the 4-week average of NVDI rises through 0.95; sell when it falls through 1.1.", common!());

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], note: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "S&C: Enhancing On-Balance And Negative Volume (Holt, V.14:6)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: "scholt", rules, notes: note, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def("scholt_obvdi", "Holt OBVDI: weekly %B of price over %B of on-balance volume; buy as its 4-week average rises through 0.85 or 0.95, sell as it falls through 0.95",
            &["F|\"Formula A for %B of price: 1+((c-(mov(c,33,s)-(2*(std(c,33)))))/(mov(c,33,s) +(2*(std(c,33)))-(mov(c,33,s)-(2*(std(c,33)))))) ... Formula C for on-balance volume disparity indicator: Formula A/Formula B\" (PDF p.4)",
              "E|\"Buy when the four-week simple moving average of OBVDI rises through the 0.85 or 0.95 level.\" (PDF p.4)",
              "X|\"Sell when the moving average falls through the 0.95 level.\" (PDF p.4)"], OBV_NOTE),
        def("scholt_nvdi", "Holt NVDI: weekly %B of price over %B of the negative volume line; buy as its 4-week average rises through 0.95, sell as it falls through 1.1",
            &["F|\"To create a negative volume disparity indicator, substitute NVI() where OBV() appears in the formulas.\" (PDF p.4); editor's Excel sidebar: \"=IF(C3<C2,H2+((B3-B2)/B2)*100,H2)\" (PDF p.5)",
              "E|\"Buy when the four-week simple moving average of NVDI rises through the 0.95 level.\" (PDF p.4)",
              "X|\"Sell when the four-week simple moving average of NVDI falls through the 1.1 level.\" (PDF p.4)"], NVI_NOTE),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "scholt_obvdi" => Some(("scholt_obvdi", "Holt OBVDI (S&C)", "33-week %B, 4-week average")),
        "scholt_nvdi" => Some(("scholt_nvdi", "Holt NVDI (S&C)", "33-week %B, 4-week average")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "scholt_obvdi" => Some(vec![p("levels", "Buy levels: 0 = 0.85 or 0.95 (reading), 1 = 0.85 only, 2 = 0.95 only", 0., 0., 2., 1.)]),
        "scholt_nvdi" => Some(vec![p("nvi", "Negative volume line: 0 = MetaStock NVI(), multiplicative (reading), 1 = Excel sidebar, additive", 0., 0., 1., 1.)]),
        _ => None,
    }
}

/// %B of `v` over `n` values: (x - (m - 2 sd)) / (4 sd), sd = population standard deviation (STDEVP); NaN while warming up or sd = 0.
pub fn pct_b(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|k| if k + 1 < n { f64::NAN } else {
        let w = &v[k + 1 - n..=k];
        let m = w.iter().sum::<f64>() / n as f64;
        let sd = (w.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n as f64).sqrt();
        if sd > 0. { (v[k] - (m - 2. * sd)) / (4. * sd) } else { f64::NAN } }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let we = super::swsp::week_ends(x);
    let (mut wc, mut wv) = (vec![], vec![]);
    let mut start = 0;
    for &e in &we {
        wc.push(x.c[e]);
        wv.push((start..=e).map(|i| x.r[i].volume.unwrap_or(f64::NAN)).sum::<f64>());
        start = e + 1;
    }
    let n = wc.len();
    let mut line = vec![f64::NAN; n];
    let excel = p.get("nvi").copied().unwrap_or(0.).round() as i32 == 1;
    let seed = |k: usize| if id == "scholt_obvdi" { wv[k] } else if wv[k].is_finite() { 100. } else { f64::NAN };
    if n > 0 { line[0] = seed(0); }
    for k in 1..n {
        line[k] = if !line[k - 1].is_finite() { seed(k) } else if id == "scholt_obvdi" {
            if wc[k] > wc[k - 1] { line[k - 1] + wv[k] } else if wc[k] < wc[k - 1] { line[k - 1] - wv[k] } else { line[k - 1] }
        } else if wv[k] < wv[k - 1] {
            let ch = (wc[k] - wc[k - 1]) / wc[k - 1];
            if excel { line[k - 1] + ch * 100. } else { line[k - 1] * (1. + ch) }
        } else { line[k - 1] };
    }
    let (bp, bv) = (pct_b(&wc, 33), pct_b(&line, 33));
    let di: Vec<f64> = (0..n).map(|k| (1. + bp[k]) / (1. + bv[k])).collect();
    let ma = super::msws::sma(&di, 4);
    let up = |k: usize, l: f64| ma[k - 1] < l && ma[k] >= l;
    let down = |k: usize, l: f64| ma[k - 1] >= l && ma[k] < l;
    let lv = p.get("levels").copied().unwrap_or(0.).round() as i32;
    let mut pos = Book::new();
    for k in 1..n {
        let t = we[k];
        if t < from || t > to || !ma[k].is_finite() || !ma[k - 1].is_finite() { continue; }
        let (buy, sell) = if id == "scholt_obvdi" {
            (match lv { 1 => up(k, 0.85), 2 => up(k, 0.95), _ => up(k, 0.85) || up(k, 0.95) }, down(k, 0.95))
        } else { (up(k, 0.95), down(k, 1.1)) };
        if pos.side() > 0. && sell { pos.exit(t, x.c[t], "sell level"); } else if pos.side() == 0. && buy { pos.enter(1., t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn holt_weekly_disparity_systems() {
        assert!((pct_b(&[1., 2., 3.], 3)[2] - (3. - (2. - 2. * (2f64 / 3.).sqrt())) / (4. * (2f64 / 3.).sqrt())).abs() < 1e-12);
        let r = super::super::sctraub::tests::bars(4000);
        let x = Ctx::new(&r, &r, &[]);
        let we = super::super::swsp::week_ends(&x);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 2, "{id}");
            for z in &t { assert!(z.dir > 0. && we.contains(&z.legs[0].i) && (z.open || we.contains(&z.exit_i))); }
            for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i); }
        }
        let mut p = Params::new(); p.insert("levels".into(), 2.);
        assert!(run("scholt_obvdi", &x, 0, r.len() - 1, &p).unwrap().len() <= run("scholt_obvdi", &x, 0, r.len() - 1, &Params::new()).unwrap().len());
    }
}

// Instrument admission: Holt OBVDI/NVDI weekly application 2026-11-13 (lane PC, R4 #297).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scholt" && reviewed(def.id),
    label: "Phillip Holt tested OBVDI and NVDI on the weekly NYSE Composite (1977-96). Weekly bars of indices, ETFs and stocks with volume, US and India, are a declared application with their own volume; the unstated fill is read as labelled on the card (R4 #297).",
    basis: "Technical Analysis of Stocks & Commodities V.14:6 pp.265-269; R4 #297; independent check authors/pc/w/pc_ref.py",
};
