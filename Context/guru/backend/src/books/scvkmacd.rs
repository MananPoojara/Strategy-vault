//! Mark Vakkur, "The Moving Average Convergence/Divergence", Technical Analysis of Stocks & Commodities V.15:4 (April 1997)
//! pp.145-153 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V15/C04/
//! THEMOVI.pdf). Book id `scvkmacd`. Lane K16 (titles loop, 2 Nov 2026), to-code row 243; independent check
//! authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::msws::sma;
use super::sctraub::week_groups;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scvkmacd_wk"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly bars = calendar weeks of the vault's daily bars (close = the week's last close, high/low = its extremes). MACD = (9-week average of closes - 12-week average) / close x 100, simple averages (the type is not stated). Buy: when a week's MACD is above the previous week's, a buy stop at that week's high works through the next week ('The MACD of last week was higher than that of the MACD two weeks ago, and the price of the asset this week exceeds the weekly high of the previous week'). Exit: in a week when the MACD turns down (below the previous week's), a sell stop is placed at that week's low for the next week; it stays in place and is moved up to the low of each further week in which the MACD declines ('move this sell-stop up as the MACD declined'). Stops fill at their level or at the open when the open is beyond it; long only (the author: short signals 'seem less reliable'). The 100 contracts in the article are an illustration (no sizing). Article (Value Line index, March 1980 - December 1995, Figure 2): 60 trades, 48% profitable, profit factor 3.23, average win / average loss 3.45. The Value Line index is not held; runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scvkmacd_wk", name: "Vakkur weekly MACD: after a rising 9/12-week MACD buy above last week's high, sell below the low of a week the MACD fell", group: "S&C: The Moving Average Convergence/Divergence (Vakkur, V.15:4)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scvkmacd", rules: &[
            "F|\"MACD = (9-week moving average of close - 12-week moving average) / close *100\" (V.15:4, PDF p.2)",
            "E|\"only going long when both of the following conditions are true: 1 The MACD of last week was higher than that of the MACD two weeks ago, and 2 The price of the asset this week exceeds the weekly high of the previous week.\" (PDF p.3)",
            "X|\"The exit rule is when the MACDHis turns down, place a sell-stop at the bar's low for the next week\" (PDF p.1, Figure 1); \"You would have continued to move this sell-stop up as the MACD declined.\" (PDF p.3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scvkmacd_wk", "Vakkur weekly MACD (S&C)", "9/12 weeks"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = week_groups(x);
    let wc: Vec<f64> = g.iter().map(|&(_, e)| x.c[e]).collect();
    let (s9, s12) = (sma(&wc, 9), sma(&wc, 12));
    let m: Vec<f64> = (0..g.len()).map(|k| (s9[k] - s12[k]) / wc[k] * 100.0).collect();
    let mut pos = Book::new();
    let (mut buy, mut sell): (Option<f64>, Option<f64>) = (None, None); // stops working through the current week
    for (k, &(s, e)) in g.iter().enumerate() {
        if s > to { break; }
        for i in s.max(from + 1)..=e.min(to) {
            if pos.side() == 0.0 {
                if let Some(lv) = buy { if x.h[i] >= lv { pos.enter(1.0, i, x.o[i].max(lv), "stop"); buy = None; } }
            } else if let Some(lv) = sell { if x.l[i] <= lv { pos.exit(i, x.o[i].min(lv), "sell stop"); sell = None; } }
        }
        if e < from || e >= to || k == 0 || !m[k - 1].is_finite() { buy = None; continue; }
        // week k is complete: orders for week k+1
        buy = if pos.side() == 0.0 && m[k] > m[k - 1] { Some(hi(&x.h, s, e)) } else { None };
        if pos.side() > 0.0 && m[k] < m[k - 1] { let l = lo(&x.l, s, e); sell = Some(sell.map_or(l, |v| v.max(l))); }
        if pos.side() == 0.0 { sell = None; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekly_macd_long_only() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scvkmacd_wk", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().all(|z| z.dir > 0.0 && (z.exit_i > z.legs[0].i || z.open)));
    }
}

// Instrument admission: S&C Vakkur weekly MACD 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scvkmacd" && reviewed(def.id),
    label: "Mark Vakkur tested the weekly MACD on the Value Line index (1980-1995). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.15:4 pp.145-153 (The Moving Average Convergence/Divergence); independent check authors/titles-1030/K16w/k16_ref.py",
};
