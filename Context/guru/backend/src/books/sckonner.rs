//! Peter Konner, "Combining RSI With RSI" (the Riviera model), Technical Analysis of Stocks & Commodities, January 2011,
//! pp.16-21 (research share Trading Knowledge I/Stocks & Commodities 2010 -2011 (traders.com)/SC2011-01.pdf; OCR of PDF
//! p.17-21, p.18-21 re-read at 200-220 dpi). Book id `sckonner`. Lane K19 (titles loop, 2 Nov 2026), to-code row 300;
//! independent check authors/titles-1030/K19w/k19_ref.py.
use crate::connors::{rsi, sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::scwhite::complete_weeks;

pub const REVIEWED: &[&str] = &["sckonner_riviera"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly bars = calendar weeks of the daily bars; the slow RSI is a 17-week and the quick RSI a 5-week Wilder RSI of the weekly closes; the price averages are 40- and 10-week SMAs of the weekly closes (the to-code summary's '4-week' was an OCR misread: the 200-dpi re-read gives '40 and 10 weeks' and Figure 3 shows 'SMA (40)'). Levels 61 / 39, which the author 'normally' uses and draws in Figures 3-4 (the rule text says 60 / 40, option up = 60, dn = 40). 'Rises above' = above the level after being at or below it the week before; 'falls below' mirrored. The slow trend is down from a slow sell signal until the next slow buy signal (kept over the whole data history). Position: a slow buy signal buys, a slow sell signal sells; while the slow trend is down, a quick buy signal buys and a quick sell signal sells. Sell = exit (long or cash); option short = 1 goes short on sell signals ('We sell at this point or go short'). The exit uses AND as the text states; option or = 1 uses the code's OR ('it gives a quicker exit. You can use either'). Fills at the next week's first open. The printed Avanza Vikingen code is only partly legible in the scan, so the rule text is followed. Article: Carlsberg weekly examples; parameters optimised on one period and 20 stocks (the author asks for more testing). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sckonner_riviera", name: "Konner Riviera model: weekly slow RSI(17) 61/39 with the 40-week SMA, and the quick RSI(5) with the 10-week SMA while the slow trend is down", group: "S&C: Combining RSI With RSI (Konner, Jan 2011)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sckonner", rules: &[
            "E|\"Buy when the slow RSI (17w) rises above its 60-level and the price is above its 40-week simple moving average.\" (PDF p.20; levels 61 / 39: 'I normally use the 61 and 39 levels to confirm that the change in trend really has happened', p.18)",
            "X|\"Sell when the slow RSI (17w) falls below its 40-level and the price is below its 40-week simple moving average.\" (p.20)",
            "E|\"If the slow RSI (17w) signals a downtrend: Buy when the quick RSI (5w) rises above its 60-level and the price is above its 10-week simple moving average ... Sell when the quick RSI (5w) falls below its 40-level and the price is below its 10-week simple moving average.\" (p.20-21)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("sckonner_riviera", "Konner Riviera RSI model (S&C)", "RSI 17 / 5, 61 / 39"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("up", "Upper level (author 61; rule text 60)", 61., 50., 80., 1.), p("dn", "Lower level (author 39; rule text 40)", 39., 20., 50., 1.),
        p("or", "1 = exit on the RSI cross OR the price below the average (the code)", 0., 0., 1., 1.), p("short", "1 = sell signals go short", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (up, dn, or) = (g("up", 61.), g("dn", 39.), g("or", 0.) > 0.5);
    let sell_to = if g("short", 0.) > 0.5 { -1.0 } else { 0.0 };
    let w = complete_weeks(x);
    let wc: Vec<f64> = w.iter().map(|&(_, e)| x.c[e]).collect();
    let (rs, rq, ms, mq) = (rsi(&wc, 17), rsi(&wc, 5), sma(&wc, 40), sma(&wc, 10));
    let buy = |r: &[f64], m: &[f64], k: usize| r[k - 1] <= up && r[k] > up && wc[k] > m[k];
    let sell = |r: &[f64], m: &[f64], k: usize| { let c = r[k - 1] >= dn && r[k] < dn; let b = wc[k] < m[k]; if or { c || b } else { c && b } };
    let mut sig = vec![None::<f64>; x.r.len()];
    let mut down = false;
    for k in 1..w.len() {
        if ![rs[k - 1], rs[k], ms[k]].iter().all(|v| v.is_finite()) { continue; }
        let s = if buy(&rs, &ms, k) { down = false; Some(1.0) } else if sell(&rs, &ms, k) { down = true; Some(sell_to) }
            else if down && [rq[k - 1], rq[k], mq[k]].iter().all(|v| v.is_finite()) { if buy(&rq, &mq, k) { Some(1.0) } else if sell(&rq, &mq, k) { Some(sell_to) } else { None } }
            else { None };
        sig[w[k].1] = s;
    }
    Some(super::sctraub::next_open(x, from, to, "sell signal", |t, side| sig[t].unwrap_or(side)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekly_rsi_model_trades_long_only() {
        let r = super::super::sctraub::tests::bars(6000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sckonner_riviera", &x, 0, r.len() - 1, &[("up".to_string(), 55.), ("dn".to_string(), 45.)].into_iter().collect()).unwrap();
        assert!(!t.is_empty() && t.iter().all(|z| z.dir > 0.));
        assert!(family_of("sckonner_riviera").is_some());
    }
}

// Instrument admission: S&C Konner Riviera model 2026-11-02 (titles lane K19).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sckonner" && reviewed(def.id),
    label: "Konner optimised the weekly model on 20 stocks over one period and shows Carlsberg examples. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities, January 2011, pp.16-21 (Combining RSI With RSI; OCR of PDF p.17-21); independent check authors/titles-1030/K19w/k19_ref.py",
};
