//! Steve Palmquist, "Success Amid Risk: Trading Moving Average Pullbacks" (MAPS), Technical Analysis of Stocks & Commodities
//! V.23:4 (April 2005) pp.16-20 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/
//! VOLUMES/V23/C04/078PALM.pdf). Book id `scpalmap`. Lane K16 (titles loop, 2 Nov 2026), to-code row 274; independent check
//! authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::msws::sma;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scpalmap_maps"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed AIQ EDS code: RisingCloses = LoVal([close],5) > HiVal([close],20,20) (the lowest of the last 5 closes above the highest of the 20 closes ending 20 days ago); NotBelowPullbackMA = no close below the 30-day SMA in the last 30 sessions; CloseToPullbackMA = close - SMA30 < 1.5% of the close; MApullback = all three. The article's backtest settings: 'Entry Price: [open] Exit Price: [open] ... Hold for 3 Periods' = buy at the next open after a MApullback day and sell at the open three bars after entry. Option trig = 1 uses the text's trigger ('triggers if the stock moves above the previous day's high'): a buy stop at the signal day's high for the next day only (fill at the high or the open above it), same three-bar exit. Long only, one position, no stops; the price and volume filters the article discusses are not applied. Article (stocks; 3/14/2003-1/23/2004 bullish period): about 59% profitable, average loss 1.73% (Figure 3). The author tested stocks; runs on indices and ETFs, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scpalmap_maps", name: "Palmquist MAPS: uptrend never below the 30-day SMA pulls back within 1.5%, buy next open, out at the open 3 bars later", group: "S&C: Trading Moving Average Pullbacks (Palmquist, V.23:4)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scpalmap", rules: &[
            "F|\"RisingCloses if LoVal([close],5) > HiVal([close],20,20). PullbackMA is simpleavg([close],30). CloseBelowPullbackMA if [close] < PullbackMA. NotBelowPullbackMA if CountOf(CloseBelowPullbackMA,30) <1. CloseToPullbackMA if ( [close] - PullbackMA ) < 0.015 * [close].\" (V.23:4, PDF p.4 sidebar)",
            "E|\"MApullback if RisingCloses and NotBelowPullbackMA and CloseToPullbackMA.\"; backtest \"Entry Price: [open]\" (PDF p.3-4)",
            "X|\"Exit Price: [open] Exit summary Hold for 3 Periods\" (PDF p.3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scpalmap_maps", "Palmquist MAPS (S&C)", "30-day SMA, 1.5%, 3-bar hold")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("trig", "1 = enter only above the signal day's high (text trigger)", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let trig = p.get("trig").copied().unwrap_or(0.) > 0.5;
    let m = sma(&x.c, 30);
    let sig = |t: usize| t >= 39 && m[t - 29].is_finite() && lo(&x.c, t - 4, t) > hi(&x.c, t - 39, t - 20)
        && (t - 29..=t).all(|j| !(x.c[j] < m[j])) && x.c[t] - m[t] < 0.015 * x.c[t];
    let mut pos = Book::new();
    let mut pend: Option<f64> = None; // NaN = buy at the open; a level = buy stop
    for t in from..=to {
        if let Some(e) = pos.entry_bar() { if t - e == 3 { pos.exit(t, x.o[t], "3-bar hold"); } }
        if let Some(lv) = pend.take() {
            if pos.side() == 0.0 {
                if lv.is_nan() { pos.enter(1.0, t, x.o[t], "open"); } else if x.h[t] >= lv { pos.enter(1.0, t, x.o[t].max(lv), "stop"); }
            }
        }
        if t < to && pos.side() == 0.0 && sig(t) { pend = Some(if trig { x.h[t] } else { f64::NAN }); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_three_bar_hold() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("scpalmap_maps", &x, 0, r.len() - 1, &Params::new()).unwrap().iter().all(|z| z.open || z.exit_i - z.legs[0].i == 3));
    }
}

// Instrument admission: S&C Palmquist MAPS 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scpalmap" && reviewed(def.id),
    label: "Steve Palmquist tested MAPS on US stocks with AIQ (2001-2004 periods). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.23:4 pp.16-20 (Trading Moving Average Pullbacks), AIQ EDS code; independent check authors/titles-1030/K16w/k16_ref.py",
};
