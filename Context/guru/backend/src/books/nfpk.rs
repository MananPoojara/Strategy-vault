//! Nelson Freeburg, "Trading the Pankin Strategy for 30% Annual Gains and Low Risk" (TAG XIX,
//! Las Vegas 1997, INO workbook, 33 pp. scan). Source and checks: library/nfpk/.
//! Live: the two signal-confirmation illustrations (Fig. 35 S&P vs 20-week MA, unfiltered and +/-1%;
//! Fig. 36 NASDAQ vs 15-week WMA, crossover and 0.5% MA swing). The Pankin rules themselves are not
//! printed (outline only) and need Fidelity Select fund prices.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "nfpk";

const PANKIN: &str = "The workbook prints only the outline 'Pankin I / Pankin II Trading Rules' (pp.1-3), never the rules; it also needs Fidelity Select sector-fund prices (not in the vault).";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long", status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    macro_rules! wk { ($a:literal) => { concat!($a, " Weekly close = close of the last trading day of each Monday-Sunday week (crate swsp::week_ends). Signals and fills at that weekly close (the chart marks the signal week; Freeburg's 'Action on Day of Signal', Fig. 33). Long / money market; cash earns nothing. Illustration only: the workbook prints no results for it. India: run on NIFTY indices (instrument swap).") }; }
    vec![
        d("nfpk_ma20w", "S&P 500 vs 20-week MA crossover (unfiltered)", "Confirming Pankin signals · Fig. 35", Live,
          &["F|\"Unfiltered Signals: S&P 500 & 20-Week MA\" (Fig. 35, p.25)", "E|\"Buy: S&P Crosses Above 20-Week MA\" (Fig. 35)", "X|\"Sell: S&P Crosses Below 20-Week MA\" (Fig. 35)"],
          wk!("MA = simple 20-week mean of weekly closes (the workbook names WMA explicitly where it means weighted, Fig. 38). Run on SPX (book) or any index.")),
        d("nfpk_ma20w_f1", "S&P 500 vs 20-week MA +/- 1% filtered crossover", "Confirming Pankin signals · Fig. 35", Live,
          &["F|\"Filtered Signals: S&P 500 & 20-WK MA +/- 1%\" (Fig. 35, p.25)", "E|\"Buy: S&P Crosses 1% Above 20-Week MA\" (Fig. 35)", "X|\"Sell: S&P Crosses 1% Below 20-Week MA\" (Fig. 35)"],
          wk!("Filtered MA crossover (outline IV.C.3.a). MA = simple 20-week mean.")),
        d("nfpk_wma15w", "NASDAQ vs 15-week WMA crossover (unfiltered)", "Confirming Pankin signals · Fig. 36", Live,
          &["F|\"NASDAQ Composite and 15-WMA\" (Fig. 36, p.25)", "E|\"Buy: OTC Crosses Above 15-WMA\" (Fig. 36)", "X|\"Sell: OTC Crosses Below 15-WMA\" (Fig. 36)"],
          wk!("WMA = linearly weighted 15-week mean (weights 1..15, newest heaviest). The vault has no NASDAQ Composite: run on NDX (Nasdaq-100) or any index (instrument swap).")),
        d("nfpk_wma15w_swing", "NASDAQ 15-week WMA 0.5% swing (filtered)", "Confirming Pankin signals · Fig. 36", Live,
          &["F|\"Filtered Signals\" on the 15-week weighted MA (Fig. 36, p.25); 'MA Percent Swing' confirmation (outline IV.C.3.b)",
            "E|\"Buy: 15-WMA Rises 0.5%\" (Fig. 36) - from its lowest value since the last sell",
            "X|\"Sell: 15-WMA Falls 0.5%\" (Fig. 36) - from its highest value since the last buy"],
          wk!("Swing reference = the MA's trough/peak since the opposite signal, as the author defines the same signal for PENTAD ('27-Wk Simple MA of Dow Jones Utility Average Drops by 3% from a Peak', Timing Models p.E-13; 'Rises by Any Amount' = from its low). Rise/fall measured inclusive (>= 0.5%). Starts flat; the first trough is the first valid WMA.")),
        d("nfpk_pankin", "Pankin I / Pankin II / Pan-Mod I / Pankin FR sector-fund switching", "The Pankin Strategy", NotTestable(PANKIN),
          &["E|Switch among Fidelity Select sector funds (rules not printed)", "X|Low-risk variant: 4-6% initial stop, 7-9% retracement stop (Fig. 41) with Vega / PENTAD timing filters"],
          "Book: Pankin I 1986-97 CAR 25.7%, 83 trades, 67% (Fig. 4); Pankin II 37.3%, 49 trades (Fig. 5); 1970-96 simulation 19.4%, 172 trades, 63% (Fig. 10); Pan-Mod I 23.9%, 191 trades (Fig. 15); Pankin FR 21.4% (Fig. 40)."),
        d("nfpk_timing_models", "52-week MA crossover, 248, Fabian, SVI, KST, VEGA timing models", "Six timing models tested", NotTestable("Result tables only (Figs 19-32); the rules of these models are not printed in the workbook."),
          &["E|No rules printed"],
          "Book 1970-97 vs S&P 12.8%: 52-wk MA 14.1% (16 trades); 248 12.7%; Fabian 14.2%; PENTAD 15.6%; SVI 15.6%; KST 16.9%; VEGA 16.5%. PENTAD rules are in nftm_pentad (Fig. 38 parameter table)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "nfpk_ma20w" => ("nf_mafilter", "Freeburg MA signal confirmation", "S&P 20-week MA · unfiltered"),
        "nfpk_ma20w_f1" => ("nf_mafilter", "Freeburg MA signal confirmation", "S&P 20-week MA · 1% filter"),
        "nfpk_wma15w" => ("nf_mafilter", "Freeburg MA signal confirmation", "NASDAQ 15-week WMA · unfiltered"),
        "nfpk_wma15w_swing" => ("nf_mafilter", "Freeburg MA signal confirmation", "NASDAQ 15-week WMA · 0.5% swing"),
        "nfpk_pankin" => ("nf_pankin", "Pankin sector-fund strategy", "not testable"),
        "nfpk_timing_models" => ("nf_results", "Freeburg results-only examples", "1997"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "nfpk_ma20w" => vec![p("weeks", "MA length (weeks)", 20., 5., 60., 1.), p("band", "Crossover filter (%)", 0., 0., 5., 0.5)],
        "nfpk_ma20w_f1" => vec![p("weeks", "MA length (weeks)", 20., 5., 60., 1.), p("band", "Crossover filter (%)", 1., 0., 5., 0.5)],
        "nfpk_wma15w" => vec![p("weeks", "WMA length (weeks)", 15., 5., 60., 1.)],
        "nfpk_wma15w_swing" => vec![p("weeks", "WMA length (weeks)", 15., 5., 60., 1.), p("swing", "MA swing (%)", 0.5, 0.1, 5., 0.1)],
        _ => return None,
    })
}

fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}
pub fn wma(v: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1) / 2) as f64;
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { (0..n).map(|j| v[i + 1 - n + j] * (j + 1) as f64).sum::<f64>() / den }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let n = g("weeks").round().max(2.) as usize;
    let we: Vec<usize> = crate::books::swsp::week_ends(x).into_iter().filter(|&i| i <= to).collect();
    let wc: Vec<f64> = we.iter().map(|&i| x.c[i]).collect();
    let ma = if id.starts_with("nfpk_wma") { wma(&wc, n) } else { sma(&wc, n) };
    let swing = id == "nfpk_wma15w_swing";
    let f = if swing { g("swing") / 100. } else { g("band").max(0.) / 100. };
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    // Swing mode keeps a virtual long/flat state from the first valid MA (signals depend on it); only
    // entries at or after `from` are recorded (flat at the start of the selected window).
    let mut ext = f64::NAN; // MA trough since the last sell (flat) / peak since the last buy (long)
    let mut long_v = false;
    for k in 1..we.len() {
        let i = we[k];
        if !ma[k].is_finite() { continue; }
        let (buy, sell) = if swing {
            if !ext.is_finite() { ext = ma[k]; }
            if !long_v { ext = ext.min(ma[k]); (ma[k] >= ext * (1. + f), false) } else { ext = ext.max(ma[k]); (false, ma[k] <= ext * (1. - f)) }
        } else {
            let pm = ma[k - 1];
            (pm.is_finite() && wc[k] > ma[k] * (1. + f) && wc[k - 1] <= pm * (1. + f),
             pm.is_finite() && wc[k] < ma[k] * (1. - f) && wc[k - 1] >= pm * (1. - f))
        };
        if buy && !long_v { long_v = true; ext = ma[k]; } else if sell && long_v { long_v = false; ext = ma[k]; }
        match cur.as_mut() {
            None if buy && i >= from => {
                cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
            }
            Some(t) if sell => {
                t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = if swing { "Weekly MA fell from its peak".into() } else { "Weekly close crossed below the MA band".into() };
                out.push(cur.take().unwrap());
            }
            _ => {}
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wma_by_hand() {
        let w = wma(&[1., 2., 3.], 3);
        assert!((w[2] - (1. + 4. + 9.) / 6.).abs() < 1e-12);
        assert!(w[1].is_nan());
    }
}
