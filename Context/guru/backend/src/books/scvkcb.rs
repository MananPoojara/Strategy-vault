//! Mark Vakkur, "The Channel Breakout System", Technical Analysis of Stocks & Commodities V.17:4 (April 1999) pp.165-174
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C04/027CHAN.pdf).
//! Book id `scvkcb`. Lane PA1 (13 Nov 2026, pool A of authors/a1/A1.tsv; R4 #297). Independent check authors/pa1/pa1_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::sctraub::{ema0, week_groups};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scvkcb_wk"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 1 unstated detail, R4 #297; unverified] Weekly bars = calendar weeks of the vault's daily bars (high/low = the week's extremes, close = its last close). Long only (the article: 'I will consider only long trades'). At each week's close while flat: a buy stop at the highest high of the last 3 weeks (this week included) + 1 point works through the next week; at each week's close while long: a sell stop at the lowest low of the last 3 weeks - 1 point works through the next week (EasyLanguage 'Buy Highest(High,buy_len) + 1 point stop', 'LongExit Lowest(Low,sell_len) - 1 point stop', buy_len = sell_len = 3 weeks). Stops fill at their level, or at the open when the day opens beyond it. As on TradeStation weekly bars, the exit stop starts the week after the entry week and there is no re-entry in the week of an exit. Unstated in the article: (1) the size of TradeStation's '1 point' on the vault's instruments; vault reading: 0.01 (one cent / one index hundredth; option tick). Option ma = the article's filter 'the weekly close must be greater than the 12-week moving average', exponential as the author chose ('I would like to use the exponential moving average'), seeded with the first weekly close; 0 = off (the base system, default). Article (Value Line Arithmetic index, weekly, 01/19/90-12/04/98, TradeStation): 37 trades, 21 winners (57%), +$4,930.10 per index point x $1, profit factor 3.53 (Figure 1). The Value Line Arithmetic index is not held; runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scvkcb_wk", name: "Vakkur channel breakout: weekly buy stop above the 3-week high, sell stop below the 3-week low (long only)", group: "S&C: The Channel Breakout System (Vakkur, V.17:4)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scvkcb", rules: &[
            "E|\"Long entry: Buy Highest(High,buy_len) + 1 point stop\" (V.17:4 p.166); \"violation of a three-week high defines an upside breakout\" (p.167)",
            "X|\"Long exit: LongExit Lowest(Low,sell_len) - 1 point stop\" (p.166); \"let's keep the three-week/three-week lookback combination\" (p.169)",
            "F|\"If close > average(close, len_ma), then Buy at highest(h, hi_per) stop\" with \"the weekly close must be greater than the 12-week moving average ... I would like to use the exponential moving average\" (p.170; option ma)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scvkcb_wk", "Vakkur channel breakout (S&C)", "3/3 weeks"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("hi", "Entry look-back, weeks (article 3)", 3., 1., 20., 1.), p("lo", "Exit look-back, weeks (article 3)", 3., 1., 20., 1.),
        p("ma", "Filter: weekly close above its EMA of this many weeks (article 12; 0 = off)", 0., 0., 52., 1.), p("tick", "One point (vault reading 0.01)", 0.01, 0., 1., 0.01)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (nh, nl) = (g("hi", 3.).round().max(1.) as usize, g("lo", 3.).round().max(1.) as usize);
    let (ma, tick) = (g("ma", 0.).round().max(0.) as usize, g("tick", 0.01).max(0.));
    let w = week_groups(x);
    let wc: Vec<f64> = w.iter().map(|&(_, e)| x.c[e]).collect();
    let wh: Vec<f64> = w.iter().map(|&(s, e)| hi(&x.h, s, e)).collect();
    let wl: Vec<f64> = w.iter().map(|&(s, e)| lo(&x.l, s, e)).collect();
    let em = if ma > 0 { ema0(&wc, ma) } else { vec![f64::NAN; wc.len()] };
    let mut pos = Book::new();
    let (mut buy, mut sell): (Option<f64>, Option<f64>) = (None, None);
    for (k, &(s, e)) in w.iter().enumerate() {
        if s > to { break; }
        for i in s.max(from + 1)..=e.min(to) {
            if pos.side() == 0. {
                if let Some(lv) = buy { if x.h[i] >= lv { pos.enter(1., i, x.o[i].max(lv), "stop"); buy = None; } }
            } else if let Some(lv) = sell { if x.l[i] <= lv { pos.exit(i, x.o[i].min(lv), "channel low stop"); sell = None; } }
        }
        buy = None; sell = None;
        if e < from || e >= to { continue; }
        // week k is complete: orders for week k+1
        if pos.side() == 0. {
            if k + 1 >= nh && (ma == 0 || (k + 1 >= ma && wc[k] > em[k])) { buy = Some(hi(&wh, k + 1 - nh, k) + tick); }
        } else if k + 1 >= nl { sell = Some(lo(&wl, k + 1 - nl, k) - tick); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekly_channel_long_only() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scvkcb_wk", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().all(|z| z.dir > 0.));
        let w = week_groups(&x);
        let wk = |i: usize| w.iter().position(|&(s, e)| s <= i && i <= e).unwrap();
        for z in t.iter().filter(|z| !z.open) {
            let (ke, kx) = (wk(z.legs[0].i), wk(z.exit_i));
            assert!(kx > ke); // never out in the entry week
            let lv = lo(&x.l, w[kx - 3].0, w[kx - 1].1) - 0.01;
            assert!((z.exit_px - x.o[z.exit_i].min(lv)).abs() < 1e-9);
            let bv = hi(&x.h, w[ke - 3].0, w[ke - 1].1) + 0.01;
            assert!((z.legs[0].px - x.o[z.legs[0].i].max(bv)).abs() < 1e-9);
        }
        let mut p = Params::new(); p.insert("ma".into(), 12.);
        assert!(run("scvkcb_wk", &x, 0, r.len() - 1, &p).unwrap().len() <= t.len());
    }
}

// Instrument admission: S&C Vakkur channel breakout 2026-11-13 (lane PA1, R4 #297).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scvkcb" && reviewed(def.id),
    label: "Mark Vakkur tested the weekly channel breakout on the Value Line Arithmetic index, IBM and a Fidelity fund; daily bars of indices, ETFs and stocks, US and India, grouped into weeks, are a declared application with one labelled reading (R4 #297).",
    basis: "Technical Analysis of Stocks & Commodities V.17:4 pp.165-174 (The Channel Breakout System); R4 #297; independent check authors/pa1/pa1_ref.py",
};
