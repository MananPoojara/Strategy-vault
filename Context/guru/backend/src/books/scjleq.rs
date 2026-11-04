//! Joe Luisi, "Trading The Equity Curve", Technical Analysis of Stocks & Commodities V.10:10 (1992) pp.444-446 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V10/C10/TRADING.pdf). Book id `scjleq`:
//! an 8-day, 20% volatility stop-and-reverse system traded only while its own equity is above the equity's 10-day average.
//! Lane K41 (batch T7a, 4 Nov 2026); independent check authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scjleq_vol"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Volatility system (Figure 1): ATR = simple 8-day average of the true range; after each close a buy stop at close + 0.20 x ATR and a sell stop at close - 0.20 x ATR for the next day; always in (stop and reverse): while long only the sell stop is live, while short only the buy stop; one reversal a day; the first position is the first stop hit (a day that hits both before any position is skipped); a stop the open gaps through fills at the open. This shadow system runs from the first bar of the data; its equity = the sum of its trades in price points, marked at each close (the article's $266 a point scales the line without moving its crossings; its $61 one-way commission is left out: a dollar amount on OEX has no meaning on the vault's instruments (decision 166), and a per-trade cost would step the equity down at each trade, which can move some crossings). Real trades (the published ones): a shadow reversal is taken only if at the previous close the equity was not below its 10-day simple average; when the equity closes below the average, the real position is closed at the next open and the system is traded hypothetically; after the equity is back above, the next shadow signal is taken ('you will miss the profitable trade that brought the equity above the moving average'). Option filter = 0 publishes the unfiltered system. Article (OEX, 31 Dec 1991 - 20 May 1992): unfiltered 68 trades, 52.94% winners; filtered 37 trades, 56.57% winners. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scjleq_vol", name: "8-day 20% volatility stop and reverse, traded only while its equity is above the equity's 10-day average", group: "S&C: trading the equity curve (Luisi, V.10:10)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scjleq", rules: &[
            "F|\"A volatility system uses an average of the true range over recent days. This particular case uses an eight-day average true range. Multiply the average true range by 20% and add it to the close for today's buy signal. Subtract the eight-day average true range multiplied by 20% from the close for today's sell signal.\" (V.10:10, PDF p.3, Figure 1)",
            "E|\"The volatility system is a stop and reverse system (SAR) that is always in the market.\" (PDF p.2)",
            "F|\"We can apply a simple crossover technique to the 10-day moving average of the equity line to stop trading the system during those times when the equity is below the moving average. When the equity line crosses above the moving average, we resume trading. One drawback is you will miss the profitable trade that brought the equity above the moving average in the first place.\" (PDF p.2)",
            "X|\"the trader should stop using real money to trade and trade hypothetically, still tracking the trades and updating the equity line to know when to resume trading\" (PDF p.2)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scjleq_vol", "Equity-curve filter (S&C)", "Volatility SAR")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "atr", label: "Average true range days", def: 8., min: 2., max: 30., step: 1. },
        ParamDef { key: "k", label: "Volatility breakout, % of the ATR", def: 20., min: 5., max: 100., step: 5. },
        ParamDef { key: "ma", label: "Equity moving average days", def: 10., min: 2., max: 50., step: 1. },
        ParamDef { key: "filter", label: "1 = trade only while the equity is not below its average; 0 = the unfiltered system", def: 1., min: 0., max: 1., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (na, k, nm, filt) = (g("atr", 8.).round().max(1.) as usize, g("k", 20.) / 100.0, g("ma", 10.).round().max(1.) as usize, g("filter", 1.) > 0.5);
    let n = x.c.len();
    let tr: Vec<f64> = (0..n).map(|i| if i == 0 { f64::NAN } else { (x.h[i] - x.l[i]).max((x.h[i] - x.c[i - 1]).abs()).max((x.l[i] - x.c[i - 1]).abs()) }).collect();
    let atr = sma(&tr, na);
    // shadow system over the whole file: side and entry price after each bar, equity in points at each close
    let (mut side, mut ep, mut closed) = (0.0f64, 0.0f64, 0.0f64);
    let mut eq = vec![0.0; n];
    let mut flip: Vec<Option<(f64, f64)>> = vec![None; n]; // (new side, fill) of a shadow reversal on that bar
    for i in 1..n {
        if atr[i - 1].is_finite() {
            let (b, s) = (x.c[i - 1] + k * atr[i - 1], x.c[i - 1] - k * atr[i - 1]);
            let (up, dn) = (x.h[i] >= b, x.l[i] <= s);
            let hit = if side > 0.0 { dn.then(|| (-1.0, x.o[i].min(s))) } else if side < 0.0 { up.then(|| (1.0, x.o[i].max(b))) }
                else if up && !dn { Some((1.0, x.o[i].max(b))) } else if dn && !up { Some((-1.0, x.o[i].min(s))) } else { None };
            if let Some((d, px)) = hit {
                if side != 0.0 { closed += side * (px - ep); }
                side = d; ep = px; flip[i] = Some((d, px));
            }
        }
        eq[i] = closed + if side != 0.0 { side * (x.c[i] - ep) } else { 0.0 };
    }
    let ma = sma(&eq, nm);
    let on = |i: usize| !filt || !(eq[i] < ma[i]);
    let mut pos = Book::new();
    let mut pend_exit = false;
    for i in from..=to {
        if pend_exit { pos.exit(i, x.o[i], "Equity below its average · next open"); pend_exit = false; }
        if let Some((d, px)) = flip[i] {
            if i >= 1 && on(i - 1) { pos.enter(d, i, px, "stop"); } else if pos.side() != 0.0 { pos.exit(i, px, "Equity filter off"); }
        }
        if pos.side() != 0.0 && !on(i) && i < to { pend_exit = true; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filter_cuts_trades() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let all = { let mut p = Params::new(); p.insert("filter".into(), 0.); run("scjleq_vol", &x, 0, r.len() - 1, &p).unwrap() };
        let f = run("scjleq_vol", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(all.len() > 10 && f.len() < all.len() && !f.is_empty());
        assert!(all.windows(2).all(|w| w[0].dir != w[1].dir));
    }
}
