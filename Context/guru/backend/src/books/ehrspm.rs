//! John Ehlers, "Rocket Science for Traders" (Wiley 2001), Chapter 20 "Optimum Predictive Filters", Predictive Moving Averages (pp.212-213, Figures 20.4-20.5).
//! Research share Trading Books/John Ehlers - Rocket Science for Traders (scan, OCR of PDF pp.108-113). Book id `ehrspm`. Lane K3 (25 Nov 2026, R4 #388 OCR rows, #400).
//! Independent check authors/conv-1123/k3w/ehrspm_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

pub const BOOK: &str = "ehrspm";
pub const REVIEWED: &[&str] = &["ehrspm_wma7"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 4 unstated details, R4 #400] Figure 20.4 prints the indicator: Price = (H+L)/2, WMA1 = 7-bar weighted average (weights 7..1 / 28), WMA2 = the same 7-bar WMA of WMA1, Predict = 2 WMA1 - WMA2, Trigger = (4 Predict + 3 Predict[1] + 2 Predict[2] + Predict[3]) / 10; p.213: 'this lagging line becomes a signal when the lines cross'. The book prints no trading rule or result for it. Unstated, one per item: (1) Predict crossing above the Trigger = buy and crossing below = sell and reverse to short (always in the market after the first cross); (2) the order fills at the close of the crossing bar; (3) a cross = Predict at or below the Trigger on the previous bar and above it today (mirror for below); (4) the first 16 bars (the 7+6+3 bars the lines look back) are warm-up with no signal. Declared application: the book names no instrument; run on the S&P 500, SPY and the NSE indices NIFTY 50 and NIFTY BANK. No book figure exists to compare.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "ehrspm_wma7", name: "Ehlers predictive moving average crossover (Predict vs Trigger), stop and reverse", group: "Ehlers · Rocket Science for Traders (Ch.20)", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|WMA1 = (7 P + 6 P[1] + 5 P[2] + 4 P[3] + 3 P[4] + 2 P[5] + P[6]) / 28 with P = (H+L)/2; WMA2 the same weights on WMA1; Predict = 2 WMA1 - WMA2; Trigger = (4 Predict + 3 Predict[1] + 2 Predict[2] + Predict[3]) / 10 (Fig. 20.4, p.213)",
            "E|\"If we then take a 4-bar WMA of the smoothed line to create a 1-bar lag, this lagging line becomes a signal when the lines cross\" (p.213): Predict crosses above the Trigger = long",
            "X|Predict crosses below the Trigger = sell and reverse to short (the mirror of the cross)",
        ], notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("ehrspm", "Ehlers predictive moving average", "7-bar WMA")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

fn wma7(v: &[f64]) -> Vec<f64> {
    (0..v.len()).map(|i| if i < 6 { f64::NAN } else { (7.0 * v[i] + 6.0 * v[i - 1] + 5.0 * v[i - 2] + 4.0 * v[i - 3] + 3.0 * v[i - 4] + 2.0 * v[i - 5] + v[i - 6]) / 28.0 }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let price: Vec<f64> = (0..=to).map(|i| (x.h[i] + x.l[i]) / 2.0).collect();
    let w1 = wma7(&price);
    let w2 = wma7(&w1);
    let pr: Vec<f64> = w1.iter().zip(&w2).map(|(a, b)| 2.0 * a - b).collect();
    let tr: Vec<f64> = (0..pr.len()).map(|i| if i < 3 { f64::NAN } else { (4.0 * pr[i] + 3.0 * pr[i - 1] + 2.0 * pr[i - 2] + pr[i - 3]) / 10.0 }).collect();
    let mut tgt = vec![0i8; to + 1];
    let mut pos = 0i8;
    for i in 1..=to {
        if pr[i].is_finite() && tr[i].is_finite() && pr[i - 1].is_finite() && tr[i - 1].is_finite() {
            if pr[i - 1] <= tr[i - 1] && pr[i] > tr[i] { pos = 1; } else if pr[i - 1] >= tr[i - 1] && pr[i] < tr[i] { pos = -1; }
        }
        tgt[i] = pos;
    }
    Some(target_trades(x, &tgt, from, to, false, "Predict/Trigger cross"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "John Ehlers prints the predictive moving average without naming an instrument; the crossover runs on the S&P 500 and SPY and on NIFTY 50 and NIFTY BANK as a declared application, with four unstated details labelled on the card.",
    basis: "John Ehlers, Rocket Science for Traders, Chapter 20 Optimum Predictive Filters, Figure 20.4 (pp.212-213)",
};
